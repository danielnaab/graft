#!/usr/bin/env bash
# fixtures.sh SLICE OUT: build the fixture that a guide is read against, in OUT (which must not exist).
# A guide's reading protocol: a reader follows the page literally in this fixture (docs/development.md).
#
#   check-fails     a consumer that grafts a local provider with this checkout's graft, then drifts three
#                   ways: a committed edit to a scion, an uncommitted formatter run, an untracked __pycache__
#   first-pattern   three starting repositories, no graft yet: new/ (git init, no commits), existing/ (its
#                   own flake with a check, and a .gitignore that drops flake.lock), locked/ (its own
#                   flake, with flake.lock committed)
#   gate-merges     two consumers that graft a local provider (page-rules: a rule and a Python checker), with
#                   graft's workflows and a nixpkgs input; in each, one committed page breaks the rule.
#                   team-kb/ is in graft's template form, and has an untracked page that breaks it too;
#                   existing/ already has checks of its own
#   override        a consumer that grafts two local providers (policy-kit at v1.0.0 with a typo, v1.1.0 tagged,
#                   with a CHANGELOG and an untracked draft; glossary at v1.0.0) with this checkout's graft
#                   and its two workflows (graft-check.yaml with a kept paths-ignore line). Its own checks
#                   run policy-kit's checker (which reads policy-kit.conf) and read the glossary's export.
#                   Committed and passing, then the team's change made in the scion: a staged edit to the
#                   status policy, and an untracked new page
#   publish         a provider to release from, and a consumer of it. note-kit/: rules, a tool (bin/note-lint,
#                   which package.nix builds), a seed file (seed/note-kit.conf, also its flake's template),
#                   tests/, a stand-in for its own scions/, a CI workflow and AGENTS.md. v1.0.0 (a lightweight
#                   tag) and v1.1.0 (an annotated tag) shipped with one rule, `tests/ export-ignore`, so its
#                   scions/, CI and AGENTS.md reach consumers. Then the next release's work, not committed: a
#                   fix to the tool, and an untracked draft. note-kit.git/: its origin, a bare repository.
#                   team-notes/: grafts note-kit v1.1.0 from note-kit.git with this checkout's graft (so its
#                   scion has tests/: Nix ignores `tests/` from a bare clone; its lock re-pinned from the
#                   annotated tag's object to the commit), runs note-lint in its check, and has its own copy
#                   of the seed file; committed and passing
#
# Needs nix, git 2.28 or later and GNU sed. Network: check-fails fetches python3 and ruff from the nixpkgs
# registry (unpinned, so ruff's output can vary); first-pattern's flakes pin a nixpkgs that the reader fetches;
# gate-merges pins the nixpkgs that this checkout's graft pins, so its python3 is likely cached; publish builds
# with graft's nixpkgs too (shellcheck, for writeShellApplication).
set -euo pipefail

graft=$(cd "$(dirname "$0")/.." && pwd)
slice=${1:?usage: fixtures.sh check-fails|first-pattern|gate-merges|override|publish OUT}
out=${2:?usage: fixtures.sh check-fails|first-pattern|gate-merges|override|publish OUT}
case $slice in check-fails|first-pattern|gate-merges|override|publish) ;; *) echo "fixtures.sh: no fixture $slice (check-fails, first-pattern, gate-merges, override, publish)" >&2; exit 2 ;; esac
[ ! -e "$out" ] || { echo "fixtures.sh: $out exists; name a new directory" >&2; exit 1; }
mkdir -p "$out"
out=$(cd "$out" && pwd)
g() { git -c user.name=fixture -c user.email=fixture@example.invalid -c commit.gpgsign=false -c tag.gpgsign=false "$@"; }
# a repo whose commits need no global git config, as a reader's would
repo() {
  g -C "$1" init -q -b main
  git -C "$1" config user.name fixture
  git -C "$1" config user.email fixture@example.invalid
  git -C "$1" config commit.gpgsign false
  git -C "$1" config tag.gpgsign false
}

check_fails() {
  local p=$out/provider c=$out/consumer
  mkdir -p "$p/docs" "$p/checker" "$p/tests"
  printf '# Provenance policy\n\nEvery page that drives a decision cites its sources.\nPages have a status: draft, working, stable or deprecated.\n' > "$p/docs/policy.md"
  # not in ruff's style: single quotes, cramped spacing
  cat > "$p/checker/__init__.py" <<'EOF'
import sys,pathlib
def check(root):
    bad=[p for p in pathlib.Path(root).rglob('*.md') if 'status:' not in p.read_text()]
    for p in bad: print('no status: %s'%p)
    return 1 if bad else 0
EOF
  cat > "$p/checker/__main__.py" <<'EOF'
import sys
from checker import check
sys.exit(check(sys.argv[1] if len(sys.argv)>1 else '.'))
EOF
  echo "echo test" > "$p/tests/t.sh"
  echo "tests/ export-ignore" > "$p/.gitattributes"
  g -C "$p" init -q -b main
  g -C "$p" add -A
  g -C "$p" commit -qm v1.0.0
  g -C "$p" tag v1.0.0

  mkdir -p "$c/docs"
  cat > "$c/flake.nix" <<EOF
{
  description = "a consumer that grafts one provider";
  inputs = {
    graft.url = "path:$graft";
    provider = { url = "git+file://$p?ref=v1.0.0&exportIgnore=1"; flake = false; };
  };
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
EOF
  printf -- '---\nstatus: working\n---\n# Team notes\n' > "$c/docs/notes.md"
  printf '[project]\nname = "team-kb"\nversion = "0.1.0"\n\n[tool.ruff]\nline-length = 100\n' > "$c/pyproject.toml"
  printf 'Never edit scions/.\n' > "$c/AGENTS.md"
  repo "$c"
  g -C "$c" add -A
  (cd "$c" && nix run .#graft >/dev/null)
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft provider v1.0.0"
  (cd "$c" && nix run .#graft check >/dev/null)

  # 1, committed: someone "tightened" a sentence in a grafted page
  sed -i 's/Every page that drives a decision cites its sources./Every page cites its sources./' "$c/scions/provider/docs/policy.md"
  g -C "$c" commit -qam "Tighten provenance wording"
  # 2, uncommitted: a formatter run over the whole repo
  (cd "$c" && nix run nixpkgs#ruff -- format . >/dev/null)
  # 3, untracked: running the grafted checker writes bytecode into the scion
  (cd "$c" && PYTHONPATH=scions/provider nix run nixpkgs#python3 -- -m checker docs)
  echo "$c"
}

# existing_flake DIR: a repository's own flake, with one check of its own
existing_flake() {
  cat > "$1/flake.nix" <<'EOF'
{
  description = "Pantry";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/ac62194c3917d5f474c1a844b6fd6da2db95077d";
  outputs = { self, nixpkgs }: {
    checks.x86_64-linux.notes = nixpkgs.legacyPackages.x86_64-linux.runCommand "notes" { } ''
      test -f ${self}/docs/notes.md
      touch $out
    '';
  };
}
EOF
}

first_pattern() {
  local n=$out/new e=$out/existing l=$out/locked d
  mkdir -p "$n" "$e/docs" "$l/docs"
  repo "$n"
  for d in "$e" "$l"; do
    echo "# Pantry" > "$d/README.md"
    printf -- '---\nstatus: working\n---\n# Notes\n' > "$d/docs/notes.md"
    existing_flake "$d"
    repo "$d"
  done
  # existing/ drops flake.lock, as some repositories did before they grafted (pantry); locked/ commits it
  printf 'result\nflake.lock\n' > "$e/.gitignore"
  echo "result" > "$l/.gitignore"
  g -C "$e" add -A
  g -C "$e" commit -qm "Start"
  g -C "$l" add -A
  (cd "$l" && nix flake lock 2>/dev/null)
  g -C "$l" add -A
  g -C "$l" commit -qm "Start"
  echo "$n"
  echo "$e"
  echo "$l"
}

gate_merges() {
  local p=$out/page-rules c=$out/team-kb
  mkdir -p "$p/pagecheck" "$p/tests"
  cat > "$p/README.md" <<'EOF'
# page-rules

One rule for a knowledge base's pages, and a checker for it.

**The rule.** Every Markdown page starts with front matter that gives its status:

    ---
    status: working
    ---

The status is one of `draft`, `working`, `stable` or `deprecated`.

**The checker.** `python3 -m pagecheck DIR` checks every `.md` file under DIR. It prints one line for each
page that breaks the rule, and exits 1 if there is one, or if DIR is not a directory. It needs Python 3 and
nothing else. Put this directory on `PYTHONPATH` to run it.
EOF
  cat > "$p/pagecheck/__init__.py" <<'EOF'
"""page-rules' checker: every Markdown page starts with front matter that gives its status."""
import pathlib
import re

STATUSES = ("draft", "working", "stable", "deprecated")
FRONT = re.compile(r"\A---\n(.*?)\n---\n", re.S)


def problems(root):
    for page in sorted(pathlib.Path(root).rglob("*.md")):
        m = FRONT.match(page.read_text(encoding="utf-8"))
        status = re.search(r"^status:\s*(\S+)\s*$", m.group(1), re.M) if m else None
        if not status:
            yield f"{page}: no status in front matter (rule: page-rules/README.md)"
        elif status.group(1) not in STATUSES:
            yield f"{page}: status {status.group(1)!r} is not one of {', '.join(STATUSES)}"
EOF
  cat > "$p/pagecheck/__main__.py" <<'EOF'
import pathlib
import sys

from pagecheck import problems

root = sys.argv[1] if len(sys.argv) > 1 else "."
if not pathlib.Path(root).is_dir():
    sys.exit(f"pagecheck: {root} is not a directory")
found = list(problems(root))
for line in found:
    print(line)
sys.exit(1 if found else 0)
EOF
  echo 'PYTHONPATH=. python3 -m pagecheck tests' > "$p/tests/run.sh"
  echo "tests/ export-ignore" > "$p/.gitattributes"
  printf '# Changelog\n\n## v1.0.0 - 2026-11-01\n\n- The status rule, and pagecheck.\n' > "$p/CHANGELOG.md"
  g -C "$p" init -q -b main
  g -C "$p" add -A
  g -C "$p" commit -qm v1.0.0
  g -C "$p" tag v1.0.0

  # team-kb/: graft's template form, with a nixpkgs input of its own (graft's pin, so it is cached)
  kb_consumer "$c" '  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };'
  # untracked, and breaks the rule: a draft that no flake can see yet
  printf -- '---\nstatus: idea\n---\n# Retry the queue\n' > "$c/docs/retry-idea.md"
  # existing/: its outputs already have checks of their own, one set for each system
  kb_consumer "$out/existing" "$(cat <<'EOF'
  outputs = { self, nixpkgs, graft, ... }@inputs: graft.lib.mkFlake {
    inherit inputs;
    outputs.checks = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        onboarding = pkgs.runCommand "onboarding" { } ''
          test -f ${self}/docs/onboarding.md
          touch $out
        '';
      });
  };
EOF
)"
  echo "$c"
  echo "$out/existing"
}

# kb_consumer DIR OUTPUTS: a consumer that grafts page-rules and graft's workflows, with OUTPUTS as its
# flake's outputs line(s); one committed page breaks the rule. graft's own messages go to stderr.
kb_consumer() {
  local c=$1 p=$out/page-rules
  mkdir -p "$c/docs" "$c/.forgejo/workflows"
  cat > "$c/flake.nix" <<EOF
{
  description = "Team knowledge base";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/ac62194c3917d5f474c1a844b6fd6da2db95077d";
    graft.url = "path:$graft";
    graft.inputs.nixpkgs.follows = "nixpkgs";
    page-rules = { url = "git+file://$p?ref=v1.0.0&exportIgnore=1"; flake = false; };
  };

$2
}
EOF
  command cp -f "$graft"/template/.forgejo/workflows/*.yaml "$c/.forgejo/workflows/"
  printf 'Follow the rule in scions/page-rules/README.md. Never edit scions/: graft replaces it.\n' > "$c/AGENTS.md"
  printf -- '---\nstatus: stable\n---\n# Onboarding\n\nRead AGENTS.md first.\n' > "$c/docs/onboarding.md"
  repo "$c"
  g -C "$c" add -A
  (cd "$c" && nix run .#graft >&2)
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft page-rules v1.0.0"
  # committed, and breaks the rule: an agent's write-up with no front matter
  printf '# Outage, 3 November\n\nThe queue filled; we drained it by hand.\n' > "$c/docs/outage-2026-11-03.md"
  g -C "$c" add -A
  g -C "$c" commit -qm "Write up the 3 November outage"
  (cd "$c" && nix run .#graft check >&2)
}

override() {
  local p=$out/policy-kit q=$out/glossary c=$out/consumer
  mkdir -p "$p/docs" "$p/checker" "$p/tests" "$q"
  printf '# Status policy\n\nEvery page has a status: draft, working, stable or deprecated.\nA deprecated page links to the page that replaces it.\n' > "$p/docs/status.md"
  # a typo that every consumer gets, for "Fix it in the provider"
  printf '# Provenance policy\n\nEvery page that drives a decision cites it sources.\n' > "$p/docs/provenance.md"
  # the checker reads one setting from the consumer: the statuses a page may have
  cat > "$p/checker/check.sh" <<'EOF'
#!/usr/bin/env bash
# check.sh DIR: fail for each Markdown page under DIR whose status line is missing, or names a status
# that policy-kit.conf (`statuses = a b c`, in the current directory) does not list
statuses="draft working stable deprecated"
if [ -f policy-kit.conf ]; then statuses=$(sed -n 's/^statuses *= *//p' policy-kit.conf); fi
rc=0
while IFS= read -r f; do
  s=$(sed -n 's/^status: *//p' "$f" | head -n 1)
  if [ -z "$s" ]; then echo "no status: $f"; rc=1
  elif [[ " $statuses " != *" $s "* ]]; then echo "status $s not in: $statuses: $f"; rc=1; fi
done < <(find "$1" -name '*.md' | sort)
exit "$rc"
EOF
  echo "echo test" > "$p/tests/t.sh"
  echo "tests/ export-ignore" > "$p/.gitattributes"
  printf '# Changelog\n\n## v1.0.0\n\n- First release: the status and provenance policies, and the checker.\n' > "$p/CHANGELOG.md"
  repo "$p"
  g -C "$p" add -A
  g -C "$p" commit -qm v1.0.0
  g -C "$p" tag v1.0.0
  printf 'A page that is not decided yet may say so in its first line.\n' >> "$p/docs/provenance.md"
  printf '# Changelog\n\n## v1.1.0\n\n- The provenance policy allows an undecided page to say so.\n\n## v1.0.0\n\n- First release: the status and provenance policies, and the checker.\n' > "$p/CHANGELOG.md"
  g -C "$p" commit -qam v1.1.0
  g -C "$p" tag v1.1.0
  # the maintainer's own draft, never added to git
  printf '# Review policy (draft)\n' > "$p/docs/review.md"

  printf '# Glossary\n\n- **Pantry:** the team knowledge base.\n' > "$q/terms.md"
  repo "$q"
  g -C "$q" add -A
  g -C "$q" commit -qm v1.0.0
  g -C "$q" tag v1.0.0

  mkdir -p "$c/docs" "$c/.forgejo/workflows"
  # its own checks read the fetched exports: policy-kit's checker, and the glossary's terms
  cat > "$c/flake.nix" <<EOF
{
  description = "Pantry: a team knowledge base";
  inputs = {
    graft.url = "path:$graft";
    policy-kit = { url = "git+file://$p?ref=v1.0.0&exportIgnore=1"; flake = false; };
    glossary = { url = "git+file://$q?ref=v1.0.0&exportIgnore=1"; flake = false; };
  };
  outputs = inputs: inputs.graft.lib.mkFlake {
    inherit inputs;
    outputs.checks.x86_64-linux = let pkgs = inputs.graft.inputs.nixpkgs.legacyPackages.x86_64-linux; in {
      statuses = pkgs.runCommand "statuses" { } ''
        cd \${inputs.self} && bash \${inputs.policy-kit}/checker/check.sh docs && touch \$out
      '';
      terms = pkgs.runCommand "terms" { } ''
        grep -q Pantry \${inputs.glossary}/terms.md && touch \$out
      '';
    };
  };
}
EOF
  printf -- '---\nstatus: working\n---\n# Team notes\n' > "$c/docs/notes.md"
  printf 'statuses = draft working stable deprecated\n' > "$c/policy-kit.conf"
  printf 'Follow the policies in scions/policy-kit/docs/. Never edit scions/: graft replaces it.\n' > "$c/AGENTS.md"
  cp "$graft/template/.forgejo/workflows/"*.yaml "$c/.forgejo/workflows/"
  # the one edit to a graft workflow that graft keeps: the bot's notes don't run the check
  sed -i 's|^  pull_request:$|&\n    paths-ignore: ["notes/**"]|' "$c/.forgejo/workflows/graft-check.yaml"
  repo "$c"
  g -C "$c" add -A
  (cd "$c" && nix run .#graft >/dev/null)
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft policy-kit v1.0.0 and glossary v1.0.0"
  (cd "$c" && nix run .#graft check >/dev/null && nix flake check 2>/dev/null)

  # the team's change, made in the scion: a staged edit to the status policy, and a new page, untracked
  sed -i 's/draft, working, stable or deprecated./proposed, draft, working, stable or deprecated./' "$c/scions/policy-kit/docs/status.md"
  g -C "$c" add -- scions/policy-kit/docs/status.md
  printf '# Proposals\n\nA proposed page names who decides it.\n' > "$c/scions/policy-kit/docs/proposals.md"
  echo "$p"
  echo "$q"
  echo "$c"
}

publish() {
  local p=$out/note-kit c=$out/team-notes
  mkdir -p "$p/docs" "$p/bin" "$p/tests/no-owner" "$p/scions/glossary" "$p/.forgejo/workflows"
  cat > "$p/README.md" <<'EOF'
# note-kit

Rules for a team's notes, and a tool that checks them.

- `docs/notes-policy.md`: the rule.
- `note-lint`: checks the notes in the folders that `note-kit.conf` lists. Build it from `package.nix`:
  `pkgs.callPackage "${inputs.note-kit}/package.nix" { }`.
EOF
  cat > "$p/docs/notes-policy.md" <<'EOF'
# Notes policy

Every note names its owner, on a line `owner: NAME`.
EOF
  # the tool, with a bug that the next release fixes: it misses notes in subfolders
  cat > "$p/bin/note-lint" <<'EOF'
#!/usr/bin/env bash
# note-lint: fail for each Markdown note with no `owner:` line, in the folders that note-kit.conf
# (in the current directory) lists, as `dirs = a b`; default docs
dirs=docs
if [ -f note-kit.conf ]; then dirs=$(sed -n 's/^dirs *= *//p' note-kit.conf); fi
read -ra folders <<<"$dirs"
rc=0
for d in "${folders[@]}"; do
  for f in "$d"/*.md; do
    [ -e "$f" ] || continue
    grep -q '^owner: ' "$f" || { echo "note-lint: no owner: $f"; rc=1; }
  done
done
exit "$rc"
EOF
  chmod +x "$p/bin/note-lint"
  cat > "$p/package.nix" <<'EOF'
# note-lint, with what it needs: pkgs.callPackage "${inputs.note-kit}/package.nix" { }
{ writeShellApplication, gnused, gnugrep, findutils }:
writeShellApplication {
  name = "note-lint";
  runtimeInputs = [ gnused gnugrep findutils ];
  text = builtins.readFile ./bin/note-lint;
}
EOF
  printf '# A note\n' > "$p/tests/no-owner/a.md"
  printf '#!/usr/bin/env bash\ncd tests/no-owner && ! bash ../../bin/note-lint\n' > "$p/tests/run.sh"
  # a stand-in for the scions that note-kit grafts for its own use
  printf '# Scions\n\nGrafted by graft. Never edit.\n' > "$p/scions/README.md"
  printf '# Glossary\n\n- **Owner:** the person who keeps a note current.\n' > "$p/scions/glossary/terms.md"
  printf 'on: [push]\njobs:\n  test:\n    runs-on: nix\n    steps:\n      - uses: actions/checkout@v4\n      - run: bash tests/run.sh\n' > "$p/.forgejo/workflows/test.yaml"
  printf 'Run tests/run.sh before a release. Never edit scions/.\n' > "$p/AGENTS.md"
  echo "tests/ export-ignore" > "$p/.gitattributes"
  printf '# Changelog\n\n## v1.0.0 - 2026-10-20\n\n- The notes policy, and note-lint.\n' > "$p/CHANGELOG.md"
  repo "$p"
  g -C "$p" add -A
  g -C "$p" commit -qm v1.0.0
  g -C "$p" tag v1.0.0
  # v1.1.0: a seed file, which consumers copy once, and a flake whose template is the same file
  mkdir -p "$p/seed"
  printf '# note-kit settings. This file is yours: copy it once, then edit it.\ndirs = docs\n' > "$p/seed/note-kit.conf"
  cat > "$p/flake.nix" <<'EOF'
{
  description = "note-kit";
  outputs = { self }: {
    templates.default = { path = ./seed; description = "note-kit's settings, to copy once"; };
  };
}
EOF
  cat >> "$p/README.md" <<'EOF'
- `seed/note-kit.conf`: settings for note-lint. Copy it once to the top of your repository; then it is
  yours. Or: `nix flake init -t <note-kit's URL at a tag>`.
EOF
  cat > "$p/CHANGELOG.md" <<'EOF'
# Changelog

## v1.1.0 - 2026-11-03

- `seed/note-kit.conf`: settings for note-lint, to copy once.

## v1.0.0 - 2026-10-20

- The notes policy, and note-lint.
EOF
  g -C "$p" add -A
  g -C "$p" commit -qm v1.1.0
  g -C "$p" tag -a -m "note-kit v1.1.0" v1.1.0
  # where releases are pushed: a bare repository, as a forge would hold it
  g clone -q --bare "$p" "$out/note-kit.git"
  g -C "$p" remote add origin "$out/note-kit.git"
  g -C "$p" fetch -q origin
  g -C "$p" branch -q -u origin/main
  # the next release's work: a fix to the tool, not committed, and the maintainer's untracked draft
  sed -i 's|^  for f in "$d"/\*.md; do$|  while IFS= read -r f; do|; s|^    \[ -e "$f" \] \|\| continue$||; s|^  done$|  done < <(find "$d" -name "*.md" \| sort)|' "$p/bin/note-lint"
  sed -i '/^$/d' "$p/bin/note-lint"
  mkdir -p "$p/notes"
  printf '# Next: check the owner against a list of people\n' > "$p/notes/next.md"

  mkdir -p "$c/docs/retros"
  cat > "$c/flake.nix" <<EOF
{
  description = "Team notes";
  inputs = {
    graft.url = "path:$graft";
    note-kit = { url = "git+file://$out/note-kit.git?ref=v1.1.0&exportIgnore=1"; flake = false; };
  };
  outputs = inputs: inputs.graft.lib.mkFlake {
    inherit inputs;
    # note-kit's tool, built from its fetched export, checks the notes
    outputs.checks.x86_64-linux = let
      pkgs = inputs.graft.inputs.nixpkgs.legacyPackages.x86_64-linux;
      note-lint = pkgs.callPackage "\${inputs.note-kit}/package.nix" { };
    in {
      notes = pkgs.runCommand "notes" { } ''
        cd \${inputs.self} && \${note-lint}/bin/note-lint && touch \$out
      '';
    };
  };
}
EOF
  printf '# On-call\n\nowner: sam\n\nPage the on-call engineer through the rota tool.\n' > "$c/docs/on-call.md"
  printf '# Retro, 2 November\n\nowner: ana\n\nThe queue filled; we drained it by hand.\n' > "$c/docs/retros/2026-11-02.md"
  command cp -f "$p/seed/note-kit.conf" "$c/note-kit.conf"
  printf 'Follow scions/note-kit/docs/notes-policy.md. Never edit scions/: graft replaces it.\n' > "$c/AGENTS.md"
  repo "$c"
  g -C "$c" add -A
  # it takes note-kit from the bare origin, as from a forge. Nix 2.24 can't lock `ref=v1.1.0` there
  # from a cold cache (it asks for refs/heads/v1.1.0): fetch the tag by its full name first. Then the
  # first run re-pins the annotated tag's object to its commit, and stops; the second grafts
  nix flake prefetch "git+file://$out/note-kit.git?ref=refs/tags/v1.1.0&exportIgnore=1" >/dev/null 2>&1
  (cd "$c" && { nix run .#graft >/dev/null 2>&1 || nix run .#graft >/dev/null; })
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft note-kit v1.1.0"
  (cd "$c" && nix run .#graft check >/dev/null && nix flake check 2>/dev/null)
  echo "$p"
  echo "$out/note-kit.git"
  echo "$c"
}

case $slice in
  check-fails) check_fails ;;
  first-pattern) first_pattern ;;
  gate-merges) gate_merges ;;
  override) override ;;
  publish) publish ;;
esac
