# mkFlake: a repo's flake outputs, with its scions, from its flake inputs.
#
# A scion is another repository's export grafted onto this one. Each is a flake input
# (flake = false) whose fetched tree is the provider's export: Nix applies the provider's
# `export-ignore` rules for `github:` inputs and for `git+…?exportIgnore=1` inputs, and pins the
# commit and a content hash in flake.lock. mkFlake builds `expected`, the directory those inputs
# make (one folder per scion, plus a README naming each pin), and gives the repo two outputs:
#
#   apps.<system>.graft    the graft command (lib/graft.sh); with no arguments, replace <dir> with
#                          `expected` (nix run .#graft); `graft check` compares the working tree's
#                          <dir> (and graft's workflows) with it
#   checks.<system>.graft  fail unless <dir> is `expected` (nix flake check); only when the flake is
#                          at the repo's top, the only place its source holds <dir>
#
# Both checks also fail when flake.lock, which holds the pins, isn't in git: a .gitignore that drops
# it would leave each run to resolve the tags again, with every check passing.
#
# Arguments:
#
#   inputs   the flake's inputs
#   scions   the input names to graft; default: every `flake = false` input. Name them when a
#            non-flake input isn't one (source a repo builds from, say).
#   dir      where they go; default `scions`, and may be any relative path inside the repo
#            (`.scions`, `docs/Grafted Docs`, …). graft owns it entirely and replaces it on every
#            run, so the app refuses to delete an existing directory that graft didn't write: a
#            mistyped `dir = "src"` stops there.
#   outputs  the repo's own flake outputs, merged with graft's at every level, so a repo's `checks`
#            sit beside graft's. `apps.<system>.graft` and `checks.<system>.graft` stay graft's.
#   flakeDir where this flake.nix sits, relative to the repo's top; default `.`, the top. A
#            subdirectory (`nix/workspace`) is for a repo with content that must never be copied
#            into the world-readable Nix store: run as a `path:` flake (nix run path:nix/workspace#graft)
#            Nix copies only that directory, never the repo's top, so the app finds the top with git
#            and `dir` stays relative to it. Such a flake has no `checks.<system>.graft` (its source
#            can't see <dir>): `graft check` is the check. The scions README records flakeDir, which
#            is how graft's other commands, and its workflows, find the flake.
#
# `expected` is built by lib/assemble.sh, which `graft upgrade` also uses, so an upgrade writes
# exactly what the check expects.
#
# graft's workflows: a file in .forgejo/workflows/ named like one in template/.forgejo/workflows/
# and carrying graft's marker line is graft's (lib/workflows.sh). The app replaces it with this
# version's copy and the check fails if it differs; a workflow without the marker is never read.
# The one line of graft's copy that is the repo's, `paths-ignore:` under its push and pull_request
# triggers, is read from the repo's copy and kept; `dir` and flakeDir are passed so a glob that
# could match them is refused.
#
# Nothing else: no config file, no lock of its own, no network access of its own. flake.lock is the
# lock. Everything that looks at the arguments is inside `apps` and `checks`, so a repo's own
# outputs (`nix develop`) evaluate without it.
#
# The app depends on `expected`, so `nix run .#graft COMMAND` needs every scion's pin fetchable. If
# one has vanished, move it from outside: nix run github:danielnaab/graft -- upgrade -C . NAME
{ nixpkgs, systems, cli }:

{ inputs, scions ? null, dir ? "scions", flakeDir ? ".", outputs ? { } }:

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

  # where this flake sits under the repo's top: `.`, or a plain relative path (it goes into `path:`
  # flake references and the scions README, so no spaces or URL punctuation)
  fd = lib.removeSuffix "/" flakeDir;
  root = fd == "." || fd == "";
  validFlakeDir = root || (builtins.match "[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*" fd != null
    && lib.all (s: s != "." && s != "..") (lib.splitString "/" fd)
    && lib.head (lib.splitString "/" fd) != ".git");
  flakeArg = if root then "." else fd;
  # the flake's lock, from the repo's top
  lockPath = if root then "flake.lock" else "${fd}/flake.lock";
  # how a person runs this flake's app, from the repo's top
  run = if root then "nix run .#graft" else "nix run path:${fd}#graft";

  # flake.nix's `inputs` is a literal, so it's read here as data: asking the fetched inputs instead
  # would fetch every one, and call every flake input's outputs.
  declared = (import "${inputs.self}/flake.nix").inputs or { };
  names = if scions != null then scions
    else lib.attrNames (lib.filterAttrs (_: v: (v.flake or true) == false) declared);

  validName = n: builtins.match "[A-Za-z0-9][A-Za-z0-9._-]*" n != null;
  bad = lib.filter (n: !validName n) names;
  missing = lib.filter (n: !(inputs ? ${n})) names;

  pin = n: inputs.${n}.rev or inputs.${n}.dirtyRev or "unpinned (no rev)";

  # The marker README.md's first line carries (lib/assemble.sh writes it): the app only ever
  # deletes a directory carrying it (or the sentence graft 2.0.0 wrote, so a 2.0.0 directory can be
  # replaced).
  marker = "<!-- managed by graft: https://github.com/danielnaab/graft -->";

  # The scions flake.lock fetches as git inputs with exportIgnore. If such a lock holds an annotated
  # tag's object, not its commit, Nix's exportIgnore ignores nothing (lib/common.sh, "Annotated
  # tags"), and `expected` is the provider's whole tree: the check and the app look for that
  # (lib/export-ignored.sh) and fail, naming it, where the drift diff alone would agree with it.
  # Read from the lock as data; nothing is fetched beyond the inputs themselves.
  lockFile = "${inputs.self}/flake.lock";
  lock = if builtins.pathExists lockFile then builtins.fromJSON (builtins.readFile lockFile) else { };
  lockedOf = n:
    let k = lock.nodes.root.inputs.${n} or null;
    in if builtins.isString k then lock.nodes.${k}.locked or { } else { };
  exportIgnored = lib.filter (n: (lockedOf n).type or "" == "git" && (lockedOf n).exportIgnore or false) names;
  # the command, or nothing when no scion needs it
  exportCheck = pkgs: lib.optionalString (exportIgnored != [ ]) ''
    PATH=${lib.makeBinPath [ pkgs.git pkgs.findutils pkgs.coreutils ]}:$PATH ${pkgs.bash}/bin/bash ${./export-ignored.sh} ${lib.escapeShellArg run} ${lib.concatMapStringsSep " " (n: lib.escapeShellArg "${n}=${inputs.${n}}") exportIgnored}'';

  # this graft version's workflows, which the repo's marked copies must match
  examples = ../template/.forgejo/workflows;

  expected = pkgs: pkgs.runCommand "graft-scions" { } ''
    GRAFT_FLAKE_DIR=${flakeArg} ${pkgs.bash}/bin/bash ${./assemble.sh} $out ${q} ${lib.concatMapStringsSep " " (n: lib.escapeShellArg "${n}=${inputs.${n}}=${pin n}") names}
  '';

  # The app's own work, `graft-sync [sync|check]`: sync replaces <dir> with `expected`; check compares
  # the working tree's <dir> with it, as git would commit it (tracked files as they stand, and new
  # ones git doesn't ignore), and graft's workflows with this version's.
  sync = pkgs: pkgs.writeShellApplication {
    name = "graft-sync";
    runtimeInputs = [ pkgs.bash pkgs.coreutils pkgs.diffutils pkgs.findutils pkgs.git pkgs.gnugrep ];
    text = ''
      mode=''${1:-sync}
      ${if root then ''
        if [ ! -f flake.nix ]; then
          echo "graft: run from the repo root (no flake.nix here)" >&2
          exit 1
        fi
      '' else ''
        # the flake's source is only ${fd}/: the repo is the git checkout this runs in
        top=$(git rev-parse --show-toplevel 2>/dev/null) \
          || { echo ${lib.escapeShellArg "graft: run inside the repo's git checkout (this flake is its ${fd}/)"} >&2; exit 1; }
        cd "$top"
        if [ ! -f ${fd}/flake.nix ]; then
          echo "graft: no ${fd}/flake.nix in $top: run inside the checkout this flake belongs to" >&2
          exit 1
        fi
      ''}
      case $mode in
        sync)
          ${lib.optionalString (exportIgnored != [ ]) ''
            ${exportCheck pkgs} || { echo ${lib.escapeShellArg "graft: not replacing ${path}/ with an export Nix didn't filter"} >&2; exit 1; }
          ''}
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
          '') names}
          # A flake sees only what git tracks: without this, a new scion's files fail the check.
          git add -- ${q} 2>/dev/null || true
          bash ${./workflows.sh} refresh . ${examples} ${lib.escapeShellArg run} ${flakeArg} ${q}
          ;;
        check)
          if [ ! -d ${q} ]; then
            echo ${lib.escapeShellArg "graft: ${path}/ is missing; run: ${run}"} >&2
            exit 1
          fi
          rc=0
          tmp=$(mktemp -d)
          trap 'rm -rf "$tmp"' EXIT
          mkdir -p "$tmp/actual/"${q}
          if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
            # the lock holds the pins: without it in git, every run resolves the tags again
            if ! git ls-files --error-unmatch --cached --others --exclude-standard -- ${lib.escapeShellArg ":(literal)${lockPath}"} >/dev/null 2>&1; then
              if git check-ignore -q -- ${lib.escapeShellArg lockPath}; then
                echo ${lib.escapeShellArg "graft: git ignores ${lockPath}, so the pins are not committed: git check-ignore -v ${lockPath} names the rule; remove the rule, then commit ${lockPath}"} >&2
              else
                echo ${lib.escapeShellArg "graft: ${lockPath} is missing; run: ${run}"} >&2
              fi
              rc=1
            fi
            git ls-files -z --cached --others --exclude-standard -- ${lib.escapeShellArg ":(literal)${path}"} \
              | while IFS= read -r -d ''' f; do
                  if [ -e "$f" ] || [ -L "$f" ]; then cp -P --parents --no-preserve=mode -- "$f" "$tmp/actual/"; fi
                done
          else
            cp -RP --no-preserve=mode ${q}/. "$tmp/actual/"${q}
          fi
          ${lib.optionalString (exportIgnored != [ ]) "${exportCheck pkgs} || rc=1"}
          bash ${./drift.sh} git ${expected pkgs} "$tmp/actual/"${q} ${q} ${lib.escapeShellArg lockPath} ${lib.escapeShellArg run} || rc=1
          bash ${./workflows.sh} check . ${examples} ${lib.escapeShellArg run} ${flakeArg} ${q} || rc=1
          if [ "$rc" = 0 ]; then echo ${lib.escapeShellArg "graft: ${path}/ and graft's workflows match the pins"}; fi
          exit "$rc"
          ;;
        *) echo "graft-sync: sync|check" >&2; exit 2 ;;
      esac
    '';
  };

  check = pkgs: pkgs.runCommand "graft-check" { } ''
    actual=${inputs.self}/${q}
    if [ ! -d "$actual" ]; then
      echo ${lib.escapeShellArg "graft: ${path}/ is missing; run: nix run .#graft"} >&2
      exit 1
    fi
    ${lib.optionalString (!builtins.pathExists lockFile) ''
      echo ${lib.escapeShellArg "graft: flake.lock is not in git, so the pins are not committed: if git ignores it, git check-ignore -v flake.lock names the rule; remove the rule, then commit flake.lock"} >&2
      exit 1
    ''}
    ${lib.optionalString (exportIgnored != [ ]) "${exportCheck pkgs} || exit 1"}
    ${pkgs.bash}/bin/bash ${./drift.sh} nix ${expected pkgs} "$actual" ${q} flake.lock ${lib.escapeShellArg run} || exit 1
    ${pkgs.bash}/bin/bash ${./workflows.sh} check ${inputs.self} ${examples} ${lib.escapeShellArg run} . ${q}
    touch $out
  '';

  # every check on the arguments, on the way to anything that uses them
  valid = x:
    assert lib.assertMsg (names != [ ]) "graft: no scions: no input has flake = false, and scions names none";
    assert lib.assertMsg (bad == [ ]) "graft: scion names must be plain folder names: ${toString bad}";
    assert lib.assertMsg (missing == [ ]) "graft: scions not among the flake's inputs: ${toString missing}";
    assert lib.assertMsg validDir
      "graft: dir must be a relative path inside the repo (no leading /, no . or .. segments, not .git), got \"${dir}\"";
    assert lib.assertMsg validFlakeDir
      "graft: flakeDir must be `.` or a relative path of letters, digits, `.`, `_`, `-` and `/` (no . or .. segments, not .git), got \"${flakeDir}\"";
    x;

  # without a check of graft's own, the name still isn't the repo's to take
  own = if root || !(outputs ? checks) then outputs
    else outputs // { checks = lib.mapAttrs (_: c: removeAttrs c [ "graft" ]) outputs.checks; };
in
# graft's side last: a repo's own `checks.<system>.graft` must not replace the drift check
lib.recursiveUpdate own
  {
    apps = forAll (pkgs: {
      graft = valid {
        type = "app";
        program = lib.getExe (cli pkgs (lib.getExe (sync pkgs)) flakeArg);
        meta.description = "graft: sync, check, outdated, upgrade, migrate, ci";
      };
    });
    # in a subdirectory, an empty set where a repo with no checks of its own would have none, so
    # `checks.<system>` always evaluates (graft ci and migrate list it)
    checks = forAll (pkgs: lib.optionalAttrs root { graft = valid (check pkgs); });
  }
