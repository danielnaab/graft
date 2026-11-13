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
#   breaking-change a provider (page-rules: v1.0.0, v1.1.0, and on its checked-out branch `lifecycle` an
#                   untagged v2.0.0 that renames the required front-matter field status: to lifecycle:, and
#                   docs/status.md to docs/lifecycle.md, with Upgrading steps); two consumers at v1.1.0 with
#                   graft's workflows and a `pages` check that v2.0.0 breaks: team-kb/ (a clone of team-kb.git,
#                   a page template outside docs/ that the check misses) takes the change; handbook/ is for the
#                   provider's dry run
#
# Needs nix, git 2.28 or later and GNU sed. Network: check-fails fetches python3 and ruff from the nixpkgs
# registry (unpinned, so ruff's output can vary); first-pattern's flakes pin a nixpkgs that the reader fetches;
# gate-merges pins the nixpkgs that this checkout's graft pins, so its python3 is likely cached.
set -euo pipefail

graft=$(cd "$(dirname "$0")/.." && pwd)
slice=${1:?usage: fixtures.sh check-fails|first-pattern|gate-merges|breaking-change OUT}
out=${2:?usage: fixtures.sh check-fails|first-pattern|gate-merges|breaking-change OUT}
case $slice in check-fails|first-pattern|gate-merges|breaking-change) ;; *) echo "fixtures.sh: no fixture $slice (check-fails, first-pattern, gate-merges, breaking-change)" >&2; exit 2 ;; esac
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

# pagecheck_v FIELD STATUSES DIR: page-rules' checker, requiring FIELD in front matter, one of STATUSES
pagecheck_v() {
  mkdir -p "$3/pagecheck"
  cat > "$3/pagecheck/__init__.py" <<EOF
"""page-rules' checker: every Markdown page starts with front matter that gives its $1."""
import pathlib
import re

FIELD = "$1"
VALUES = ($2)
FRONT = re.compile(r"\A---\n(.*?)\n---\n", re.S)


def problems(root):
    for page in sorted(pathlib.Path(root).rglob("*.md")):
        m = FRONT.match(page.read_text(encoding="utf-8"))
        front = m.group(1) if m else ""
        value = re.search(rf"^{FIELD}:\s*(\S+)\s*$", front, re.M)
        if FIELD != "status" and re.search(r"^status:", front, re.M):
            yield f"{page}: 'status:' is now '{FIELD}:' (page-rules v2.0.0; see its CHANGELOG)"
        elif not value:
            yield f"{page}: no {FIELD} in front matter (rule: page-rules/docs/{FIELD}.md)"
        elif value.group(1) not in VALUES:
            yield f"{page}: {FIELD} {value.group(1)!r} is not one of {', '.join(VALUES)}"
EOF
  cat > "$3/pagecheck/__main__.py" <<'EOF'
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
}

# A provider (page-rules) whose v2.0.0 renames a required front-matter field, `status:` to `lifecycle:`,
# and its rule page with it; two consumers at v1.1.0 that use the field.
# shellcheck disable=SC2016 # backticks are Markdown, not commands
breaking_change() {
  local p=$out/page-rules t=$out/team-kb h=$out/handbook
  mkdir -p "$p/docs" "$p/tests"
  cat > "$p/README.md" <<'EOF'
# page-rules

One rule for a knowledge base's pages, and a checker for it. The rule is in `docs/`.

**The checker.** `python3 -m pagecheck DIR` checks every `.md` file under DIR. It prints one line for each
page that breaks the rule, and exits 1 if there is one. Put this directory on `PYTHONPATH` to run it.
EOF
  printf '# The status rule\n\nEvery page starts with front matter that gives its status:\n\n    ---\n    status: working\n    ---\n\nThe status is one of `draft`, `working`, `stable` or `deprecated`.\n' > "$p/docs/status.md"
  pagecheck_v status '"draft", "working", "stable", "deprecated"' "$p"
  echo 'PYTHONPATH=. python3 -m pagecheck tests' > "$p/tests/run.sh"
  echo "/tests export-ignore" > "$p/.gitattributes"
  printf '# Changelog\n\n## v1.0.0 - 2026-11-01\n\n- The status rule, and pagecheck.\n' > "$p/CHANGELOG.md"
  repo "$p"
  g -C "$p" add -A
  g -C "$p" commit -qm "The status rule"
  g -C "$p" tag v1.0.0
  # v1.1.0: compatible, a new value
  sed -i 's/`stable` or `deprecated`/`stable`, `living` or `deprecated`/' "$p/docs/status.md"
  pagecheck_v status '"draft", "working", "stable", "living", "deprecated"' "$p"
  printf '# Changelog\n\n## v1.1.0 - 2026-11-05\n\n### Added\n- `living`: a status for a page that is always current.\n\n## v1.0.0 - 2026-11-01\n\n- The status rule, and pagecheck.\n' > "$p/CHANGELOG.md"
  g -C "$p" commit -qam "Allow the living status"
  g -C "$p" tag v1.1.0

  # two consumers at v1.1.0: team-kb/ (a clone of team-kb.git) takes the break; handbook/ is a provider's dry run
  bc_consumer "$t" "Team knowledge base" onboarding stable "Read AGENTS.md first."
  printf -- '---\nstatus: working\n---\n# Outage, 3 November\n\nThe queue filled; we drained it by hand.\n' > "$t/docs/outage-2026-11-03.md"
  mkdir -p "$t/templates"
  printf -- '---\nstatus: draft\n---\n# Title\n' > "$t/templates/page.md"
  g -C "$t" add -A
  g -C "$t" commit -qm "Write up the 3 November outage; a page template"
  g -c init.defaultBranch=main clone -q --bare "$t" "$out/team-kb.git"
  g -C "$t" remote add origin "$out/team-kb.git"
  g -C "$t" fetch -q origin
  g -C "$t" remote set-head origin main >/dev/null
  g -C "$t" branch -q -u origin/main
  bc_consumer "$h" "Handbook" holidays living "Ask the office."

  # the provider's next major version, on a branch, not yet tagged: status: is now lifecycle:
  g -C "$p" switch -q -c lifecycle
  g -C "$p" mv docs/status.md docs/lifecycle.md
  printf '# The lifecycle rule\n\nEvery page starts with front matter that gives its lifecycle:\n\n    ---\n    lifecycle: working\n    ---\n\nThe lifecycle is one of `draft`, `working`, `stable`, `living` or `deprecated`.\n' > "$p/docs/lifecycle.md"
  pagecheck_v lifecycle '"draft", "working", "stable", "living", "deprecated"' "$p"
  cat > "$p/CHANGELOG.md" <<'EOF'
# Changelog

## v2.0.0 - 2026-11-12

The front-matter field `status:` is now `lifecycle:`, so it no longer clashes with tools that use `status:`
for something else.

### Changed
- pagecheck requires `lifecycle:`, and fails a page that still has `status:`.
- `docs/status.md` is now `docs/lifecycle.md`.

### Upgrading
1. Find your pages that still use the old field. Leave out the scions directory:
   `grep -rl --include='*.md' --exclude-dir=scions '^status:' .`
2. In each file it lists, rename the field and keep its value:
   `sed -i 's/^status:/lifecycle:/'` on those files.
3. Find links to the old rule page: `grep -rn --exclude-dir=scions 'page-rules/docs/status.md' .`
   Change each to `page-rules/docs/lifecycle.md`.
4. Done when `nix flake check` passes.

## v1.1.0 - 2026-11-05

### Added
- `living`: a status for a page that is always current.

## v1.0.0 - 2026-11-01

- The status rule, and pagecheck.
EOF
  g -C "$p" add -A
  g -C "$p" commit -qm "Rename status: to lifecycle:"
  echo "$p"
  echo "$t"
  echo "$h"
}

# bc_consumer DIR DESCRIPTION PAGE STATUS TEXT: a consumer at page-rules v1.1.0 with graft's workflows, a
# `pages` check that runs pagecheck on docs/, an AGENTS.md that links the rule page, and one page
bc_consumer() {
  local c=$1 p=$out/page-rules
  mkdir -p "$c/docs" "$c/.forgejo/workflows"
  cat > "$c/flake.nix" <<EOF
{
  description = "$2";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/ac62194c3917d5f474c1a844b6fd6da2db95077d";
    graft.url = "path:$graft";
    graft.inputs.nixpkgs.follows = "nixpkgs";
    page-rules = { url = "git+file://$p?ref=v1.1.0&exportIgnore=1"; flake = false; };
  };

  outputs = { self, nixpkgs, graft, ... }@inputs: graft.lib.mkFlake {
    inherit inputs;
    outputs.checks = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] (system:
      let pkgs = nixpkgs.legacyPackages.\${system}; in {
        pages = pkgs.runCommand "pages" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          cd \${self}
          PYTHONPATH=\${inputs.page-rules} python3 -m pagecheck docs
          touch \$out
        '';
      });
  };
}
EOF
  command cp -f "$graft"/template/.forgejo/workflows/*.yaml "$c/.forgejo/workflows/"
  printf 'Follow the rule in scions/page-rules/docs/status.md. Never edit scions/: graft replaces it.\n' > "$c/AGENTS.md"
  printf -- '---\nstatus: %s\n---\n# %s\n\n%s\n' "$4" "${3^}" "$5" > "$c/docs/$3.md"
  repo "$c"
  g -C "$c" add -A
  (cd "$c" && nix run .#graft >&2)
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft page-rules v1.1.0"
  (cd "$c" && nix run .#graft check >&2)
}

case $slice in
  check-fails) check_fails ;;
  first-pattern) first_pattern ;;
  gate-merges) gate_merges ;;
  breaking-change) breaking_change ;;
esac
