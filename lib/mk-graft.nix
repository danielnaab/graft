# mkGraft: a repo's scions, from its flake inputs.
#
# A scion is another repository's export grafted onto this one. Each dep is a flake input
# (flake = false) whose fetched tree is the provider's export: Nix applies the provider's
# `export-ignore` rules for `github:` inputs and for `git+…?exportIgnore=1` inputs, and pins the
# commit and a content hash in flake.lock. mkGraft builds `expected`, the directory those inputs
# make (one folder per dep, plus a README naming each pin), and gives the repo two outputs over it:
#
#   apps.<system>.graft          replace <dir> with `expected`   (nix run .#graft)
#   checks.<system>.graft        fail unless <dir> is `expected` (nix flake check)
#   apps.<system>.graft-outdated graft's and each scion's pinned tag against the newest upstream
#   apps.<system>.graft-upgrade  move graft and scions to newer tags, re-lock, rebuild <dir>
#   apps.<system>.graft-ci       the steps of graft's Forgejo workflows (lib/ci.sh)
#   apps.<system>.graft-migrate  a migration brief for an upgraded branch, for an agent (lib/migrate.sh)
#
# `expected` is built by lib/assemble.sh, which graft-upgrade also uses, so an upgrade writes
# exactly what the check expects.
#
# graft's workflows: a file in .forgejo/workflows/ named like one in examples/forgejo/ and carrying
# graft's marker line is graft's (lib/workflows.sh). The app replaces it with this version's copy
# and the check fails if it differs; a workflow without the marker is never read.
#
# <dir> defaults to `scions` and may be any relative path inside the repo (`.scions`,
# `docs/Grafted Docs`, …). graft owns it entirely and replaces it on every run, so the app refuses
# to delete an existing directory that graft didn't write: a mistyped `dir = "src"` stops there.
#
# Nothing else: no config file, no lock of its own, no network access of its own. flake.lock is the
# lock and `nix flake update <dep>` is the upgrade.
{ nixpkgs, systems, tools }:

{ inputs, deps, dir ? "scions" }:

let
  lib = nixpkgs.lib;
  forAll = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

  # the directory as given, minus trailing slashes
  path = lib.removeSuffix "/" (lib.removeSuffix "/" dir);
  segments = lib.splitString "/" path;
  validDir = path != ""
    && !(lib.hasPrefix "/" path)
    && !(lib.hasInfix "\n" path)
    && lib.all (s: s != "" && s != "." && s != "..") segments
    && lib.head segments != ".git";
  q = lib.escapeShellArg path;

  validName = n: builtins.match "[A-Za-z0-9][A-Za-z0-9._-]*" n != null;
  bad = lib.filter (n: !validName n) deps;
  missing = lib.filter (n: !(inputs ? ${n})) deps;

  pin = n: inputs.${n}.rev or inputs.${n}.dirtyRev or "unpinned (no rev)";

  # The marker README.md's first line carries (lib/assemble.sh writes it): the app only ever
  # deletes a directory carrying it (or the sentence graft 2.0.0 wrote, so a 2.0.0 directory can be
  # replaced).
  marker = "<!-- managed by graft: https://github.com/danielnaab/graft -->";

  # this graft version's workflows, which the repo's marked copies must match
  examples = ../examples/forgejo;

  expected = pkgs: pkgs.runCommand "graft-scions" { } ''
    ${pkgs.bash}/bin/bash ${./assemble.sh} $out ${q} ${lib.concatMapStringsSep " " (n: lib.escapeShellArg "${n}=${inputs.${n}}=${pin n}") deps}
  '';

  sync = pkgs: pkgs.writeShellApplication {
    name = "graft";
    runtimeInputs = [ pkgs.bash pkgs.coreutils pkgs.diffutils pkgs.gnugrep ];
    text = ''
      if [ ! -f flake.nix ]; then
        echo "graft: run from the repo root (no flake.nix here)" >&2
        exit 1
      fi
      if [ -e ${q} ]; then
        if ! grep -qsF -e ${lib.escapeShellArg marker} -e 'Managed by [graft]' ${q}/README.md; then
          echo ${lib.escapeShellArg "graft: ${path}/ exists and graft didn't write it (no marker in its README.md); not replacing it"} >&2
          exit 1
        fi
      fi
      rm -rf ${q}
      mkdir -p "$(dirname ${q})"
      cp -R --no-preserve=mode ${expected pkgs} ${q}
      ${lib.concatMapStrings (n: ''
        echo ${lib.escapeShellArg "graft: ${path}/${n} <- ${pin n}"}
      '') deps}
      bash ${./workflows.sh} refresh . ${examples}
    '';
  };

  check = pkgs: pkgs.runCommand "graft-check" { } ''
    actual=${inputs.self}/${q}
    if [ ! -d "$actual" ]; then
      echo ${lib.escapeShellArg "graft: ${path}/ is missing; run: nix run .#graft"} >&2
      exit 1
    fi
    if ! diff -r ${expected pkgs} "$actual" >&2; then
      echo ${lib.escapeShellArg "graft: ${path}/ differs from the pins in flake.lock; run: nix run .#graft"} >&2
      echo "graft: (a file listed only on the left may be one your .gitignore drops)" >&2
      exit 1
    fi
    ${pkgs.bash}/bin/bash ${./workflows.sh} check ${inputs.self} ${examples}
    touch $out
  '';
in
assert lib.assertMsg (deps != [ ]) "graft: deps is empty";
assert lib.assertMsg (bad == [ ]) "graft: dep names must be plain folder names: ${toString bad}";
assert lib.assertMsg (missing == [ ]) "graft: deps not among the flake's inputs: ${toString missing}";
assert lib.assertMsg validDir
  "graft: dir must be a relative path inside the repo (no leading /, no . or .. segments, not .git), got \"${dir}\"";
{
  apps = forAll (pkgs: {
    graft = { type = "app"; program = lib.getExe (sync pkgs); };
    graft-outdated = { type = "app"; program = lib.getExe (tools pkgs).outdated; };
    graft-upgrade = { type = "app"; program = lib.getExe (tools pkgs).upgrade; };
    graft-ci = { type = "app"; program = lib.getExe (tools pkgs).ci; };
    graft-migrate = { type = "app"; program = lib.getExe (tools pkgs).migrate; };
  });
  checks = forAll (pkgs: { graft = check pkgs; });
}
