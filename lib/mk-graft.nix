# mkGraft: a repo's grafts, from its flake inputs.
#
# Each dep is a flake input (flake = false) whose fetched tree is the provider's export: Nix applies
# the provider's `export-ignore` rules for `github:` inputs and for `git+…?exportIgnore=1` inputs, and
# pins the commit and a content hash in flake.lock. mkGraft builds `expected`, the vendor directory
# those inputs make (one folder per dep, plus a README naming each pin), and gives the repo two
# outputs over it:
#
#   apps.<system>.graft    replace <dir> with `expected`   (nix run .#graft)
#   checks.<system>.graft  fail unless <dir> is `expected` (nix flake check)
#
# Nothing else: no config file, no lock of its own, no network access of its own. flake.lock is the
# lock and `nix flake update <dep>` is the upgrade.
{ nixpkgs, systems }:

{ inputs, deps, dir ? "vendor" }:

let
  lib = nixpkgs.lib;
  forAll = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

  validName = n: builtins.match "[A-Za-z0-9][A-Za-z0-9._-]*" n != null;
  validDir = builtins.match "[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*" dir != null
    && !(lib.any (s: s == "." || s == "..") (lib.splitString "/" dir));
  bad = lib.filter (n: !validName n) deps;
  missing = lib.filter (n: !(inputs ? ${n})) deps;

  pin = n: inputs.${n}.rev or inputs.${n}.dirtyRev or "unpinned (no rev)";

  readme = builtins.toFile "graft-README.md" ''
    # ${dir}/

    Managed by [graft](https://github.com/danielnaab/graft): do not edit. Each folder is the export
    of one flake input, pinned in `flake.lock`. To upgrade one: `nix flake update <name>`, then
    `nix run .#graft`, then commit. `nix flake check` fails if this folder drifts from the pins.

    | Folder | Commit |
    |---|---|
    ${lib.concatMapStrings (n: "| [${n}](${n}/) | `${pin n}` |\n") deps}'';

  expected = pkgs: pkgs.runCommand "graft-${baseNameOf dir}" { } ''
    mkdir -p $out
    cp ${readme} $out/README.md
    ${lib.concatMapStrings (n: ''
      cp -R --no-preserve=mode ${inputs.${n}} $out/${n}
    '') deps}
    # git can't hold an empty directory, so a checkout never has one: drop them here too
    find $out -mindepth 1 -type d -empty -delete
  '';

  sync = pkgs: pkgs.writeShellApplication {
    name = "graft";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      if [ ! -f flake.nix ]; then
        echo "graft: run from the repo root (no flake.nix here)" >&2
        exit 1
      fi
      rm -rf ${lib.escapeShellArg dir}
      mkdir -p "$(dirname ${lib.escapeShellArg dir})"
      cp -R --no-preserve=mode ${expected pkgs} ${lib.escapeShellArg dir}
      ${lib.concatMapStrings (n: ''
        echo "graft: ${dir}/${n} <- ${pin n}"
      '') deps}
    '';
  };

  check = pkgs: pkgs.runCommand "graft-check" { } ''
    if [ ! -d ${inputs.self}/${dir} ]; then
      echo "graft: ${dir}/ is missing; run: nix run .#graft" >&2
      exit 1
    fi
    if ! diff -r ${expected pkgs} ${inputs.self}/${dir} >&2; then
      echo "graft: ${dir}/ differs from the pins in flake.lock; run: nix run .#graft" >&2
      echo "graft: (a file listed only on the left may be one your .gitignore drops)" >&2
      exit 1
    fi
    touch $out
  '';
in
assert lib.assertMsg (deps != [ ]) "graft: deps is empty";
assert lib.assertMsg (bad == [ ]) "graft: dep names must be plain folder names: ${toString bad}";
assert lib.assertMsg (missing == [ ]) "graft: deps not among the flake's inputs: ${toString missing}";
assert lib.assertMsg validDir "graft: dir must be a relative path inside the repo, got \"${dir}\"";
{
  apps = forAll (pkgs: { graft = { type = "app"; program = lib.getExe (sync pkgs); }; });
  checks = forAll (pkgs: { graft = check pkgs; });
}
