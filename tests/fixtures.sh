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
#   private         private files kept out of the Nix store, and a private provider: provider/ and
#                   notes-provider/ (tags v1.0.0 and v1.1.0, a CHANGELOG; notes-provider is the private one,
#                   git+file standing in for git+ssh), moving/ (a flake at the top grafts provider into
#                   .scions beside a committed notebook/, with an uncommitted edit and an untracked draft in
#                   it, and graft's workflows), new/ (a notebook/, no flake yet), consumer/ (a flake at the
#                   top that grafts provider, and graft's workflows), server-host-key.pub (a made-up host key)
#   ci              two consumers with graft's workflows, each pushed to a bare origin (NAME.git), that graft two
#                   local providers: style-guide (v1.0.0, then v1.1.0, with a CHANGELOG) and page-rules (v1.0.0,
#                   then v1.0.1 and v2.0.0, whose CHANGELOG has an Upgrading section). team-kb/ is in graft's
#                   template form, and its upgrade passes; handbook/ has a check of its own that style-guide
#                   v1.1.0 breaks. pr.sh REPO [open] runs `graft ci pr` in REPO against tests/stub-api.py
#                   (copied in), and prints the pull request that graft asked it to open, or with `open`, to edit
#   breaking-change a provider (page-rules: v1.0.0, v1.1.0, and on its checked-out branch `lifecycle` an
#                   untagged v2.0.0 that renames the required front-matter field status: to lifecycle:, and
#                   docs/status.md to docs/lifecycle.md, with Upgrading steps); two consumers at v1.1.0 with
#                   graft's workflows and a `pages` check that v2.0.0 breaks: team-kb/ (a clone of team-kb.git,
#                   a page template outside docs/ that the check misses) takes the change; handbook/ is for the
#                   provider's dry run
#   upgrade         team-kb/: a consumer with graft's workflows (graft pinned by path, so untagged) that grafts two local
#                   providers at v1.0.0, committed and pushed to team-kb.git; then style-guide v1.1.0 (a new minor:
#                   a reworded page and a new one) and page-rules v2.0.0 (a new major, with Upgrading steps); an
#                   untracked docs/retro.md in team-kb/ that is not about the upgrade
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
# Needs nix, git 2.28 or later and GNU sed; ci's pr.sh also needs jq. Network: check-fails fetches python3 and ruff from the nixpkgs
# registry (unpinned, so ruff's output can vary); first-pattern's flakes pin a nixpkgs that the reader fetches;
# gate-merges pins the nixpkgs that this checkout's graft pins, so its python3 is likely cached; so do ci's
# handbook/ check and pr.sh. breaking-change reads that nixpkgs from graft's flake.lock (needs jq). publish
# builds with graft's nixpkgs too (shellcheck, for writeShellApplication). Providers are local git+file repos.
set -euo pipefail

graft=$(cd "$(dirname "$0")/.." && pwd)
slice=${1:?usage: fixtures.sh check-fails|first-pattern|gate-merges|override|private|ci|breaking-change|upgrade|publish OUT}
out=${2:?usage: fixtures.sh check-fails|first-pattern|gate-merges|override|private|ci|breaking-change|upgrade|publish OUT}
case $slice in check-fails|first-pattern|gate-merges|override|private|ci|breaking-change|upgrade|publish) ;; *) echo "fixtures.sh: no fixture $slice (check-fails, first-pattern, gate-merges, override, private, ci, breaking-change, upgrade, publish)" >&2; exit 2 ;; esac
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
2. Rename the field in each of those files, and keep its value:
   `grep -rl --include='*.md' --exclude-dir=scions '^status:' . | xargs sed -i 's/^status:/lifecycle:/'`
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
  local c=$1 p=$out/page-rules nixpkgs
  nixpkgs=$(jq -r .nodes.nixpkgs.locked.rev "$graft/flake.lock")
  mkdir -p "$c/docs" "$c/.forgejo/workflows"
  cat > "$c/flake.nix" <<EOF
{
  description = "$2";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/$nixpkgs";
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

# tagged_provider NAME: a provider at OUT/NAME, tags v1.0.0 and v1.1.0, a CHANGELOG, tests it keeps to itself
tagged_provider() {
  local p=$out/$1
  mkdir -p "$p/docs" "$p/tests"
  echo "tests export-ignore" > "$p/.gitattributes"
  echo "echo test" > "$p/tests/t.sh"
  printf '# %s policy\n\nEvery page has a status.\n' "$1" > "$p/docs/policy.md"
  printf '# Changelog\n\n## v1.0.0\n\nFirst release.\n' > "$p/CHANGELOG.md"
  g -C "$p" init -q -b main
  g -C "$p" add -A
  g -C "$p" commit -qm v1.0.0
  g -C "$p" tag v1.0.0
  printf '\nEvery page names its owner.\n' >> "$p/docs/policy.md"
  printf '# Changelog\n\n## v1.1.0\n\nPages name their owner.\n\n## v1.0.0\n\nFirst release.\n' > "$p/CHANGELOG.md"
  g -C "$p" commit -qam v1.1.0
  g -C "$p" tag v1.1.0
}

# top_flake DIR: a flake at DIR's top that grafts provider v1.0.0 into DIR/.scions, with a check of its
# own, and graft's workflows
top_flake() {
  mkdir -p "$1"
  cat > "$1/flake.nix" <<EOF
{
  description = "Team workspace";
  inputs = {
    graft.url = "path:$graft";
    provider = { url = "git+file://$out/provider?ref=v1.0.0&exportIgnore=1"; flake = false; };
  };
  outputs = inputs: inputs.graft.lib.mkFlake {
    inherit inputs;
    dir = ".scions";
    outputs.checks = builtins.mapAttrs (system: _: {
      lint = inputs.graft.inputs.nixpkgs.legacyPackages.\${system}.runCommand "lint" { } "touch \$out";
    }) inputs.graft.packages;
  };
}
EOF
  mkdir -p "$1/.forgejo/workflows"
  cp "$graft"/template/.forgejo/workflows/*.yaml "$1/.forgejo/workflows/"
}

private() {
  local m=$out/moving n=$out/new c=$out/consumer
  tagged_provider provider
  tagged_provider notes-provider
  # moving/: private notes beside a flake at the top, which copies them to the store on every run
  mkdir -p "$m/notebook"
  printf '# Meeting transcript\n\n%s-%s: only this repository may hold this line.\n' private-sentinel moving > "$m/notebook/transcript.md"
  echo "# Team workspace" > "$m/README.md"
  top_flake "$m"
  repo "$m"
  g -C "$m" add -A
  (cd "$m" && nix run .#graft >/dev/null 2>&1)
  g -C "$m" add -A
  g -C "$m" commit -qm "Graft provider v1.0.0"
  echo "An uncommitted note." >> "$m/notebook/transcript.md"
  echo "# Draft, never committed" > "$m/notebook/draft.md"
  # new/: private notes, no flake yet
  mkdir -p "$n/notebook"
  printf '# Meeting transcript\n\n%s-%s: only this repository may hold this line.\n' private-sentinel new > "$n/notebook/transcript.md"
  echo "# Team notes" > "$n/README.md"
  repo "$n"
  g -C "$n" add -A
  g -C "$n" commit -qm "Start"
  # consumer/: grafts a public provider; a reader adds notes-provider as the private one
  top_flake "$c"
  repo "$c"
  g -C "$c" add -A
  (cd "$c" && nix run .#graft >/dev/null 2>&1)
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft provider v1.0.0"
  # a made-up host key, as ssh-keyscan prints one: test data, not a real key
  echo "[git.example]:2222 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGraftTestHostKeyNotARealOne+/0123456789abcdefgh" > "$out/server-host-key.pub"
  echo "$m"
  echo "$n"
  echo "$c"
}

# release DIR TAG DATE NOTE [UPGRADING]: tag a provider's next version, with a CHANGELOG section on top
release() {
  local d=$1 tag=$2 date=$3 note=$4 up=${5:-} old=""
  [ ! -f "$d/CHANGELOG.md" ] || old=$(sed '1,2d' "$d/CHANGELOG.md")
  {
    printf '# Changelog\n\n## %s - %s\n\n- %s\n' "$tag" "$date" "$note"
    if [ -n "$up" ]; then printf '\n### Upgrading\n\n- %s\n' "$up"; fi
    if [ -n "$old" ]; then printf '\n%s\n' "$old"; fi
  } > "$d/CHANGELOG.md"
  g -C "$d" add -A
  g -C "$d" commit -qm "$tag"
  g -C "$d" tag "$tag"
}

# ci_consumer DIR OUTPUTS: a consumer that grafts style-guide and page-rules v1.0.0, with graft's workflows
# and OUTPUTS as its flake's outputs line(s), committed and pushed to a bare DIR.git, its origin
ci_consumer() {
  local c=$1
  mkdir -p "$c/docs" "$c/.forgejo/workflows"
  cat > "$c/flake.nix" <<EOF
{
  description = "Team knowledge base";

  inputs = {
    graft.url = "path:$graft";
    style-guide = { url = "git+file://$out/style-guide?ref=v1.0.0&exportIgnore=1"; flake = false; };
    page-rules = { url = "git+file://$out/page-rules?ref=v1.0.0&exportIgnore=1"; flake = false; };
  };

$2
}
EOF
  command cp -f "$graft"/template/.forgejo/workflows/*.yaml "$c/.forgejo/workflows/"
  printf -- '---\nstatus: working\n---\n# Onboarding\n\nOur style guide says: "Write short sentences."\n' > "$c/docs/onboarding.md"
  repo "$c"
  g -C "$c" add -A
  (cd "$c" && nix run .#graft >&2)
  g -C "$c" add -A
  g -C "$c" commit -qm "Graft style-guide and page-rules v1.0.0"
  g init -q --bare "$c.git"
  g -C "$c" remote add origin "$c.git"
  g -C "$c" push -q -u origin main 2>/dev/null
}

# two_providers: style-guide and page-rules, local providers at v1.0.0, each with a CHANGELOG
two_providers() {
  local s=$out/style-guide p=$out/page-rules d
  mkdir -p "$s/docs" "$s/tests" "$p/docs" "$p/tests"
  printf '# Style\n\nWrite short sentences.\n' > "$s/docs/style.md"
  printf '# Page rules\n\nEvery page has a status.\n' > "$p/docs/rules.md"
  for d in "$s" "$p"; do
    echo "echo test" > "$d/tests/t.sh"
    echo "tests/ export-ignore" > "$d/.gitattributes"
    g -C "$d" init -q -b main
  done
  release "$s" v1.0.0 2026-10-01 "First rules."
  release "$p" v1.0.0 2026-10-01 "The status rule."
}

ci_fixture() {
  local s=$out/style-guide p=$out/page-rules
  two_providers

  ci_consumer "$out/team-kb" '  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };'
  # handbook/: a check of its own, that onboarding.md quotes style-guide's rule word for word
  ci_consumer "$out/handbook" "$(cat <<'EOF'
  outputs = inputs:
    let flake = inputs.graft.lib.mkFlake { inherit inputs; };
    in flake // {
      checks = builtins.mapAttrs (system: checks: checks // {
        style-quote = inputs.graft.inputs.nixpkgs.legacyPackages.${system}.runCommand "style-quote" { } ''
          q=$(sed -n 's/^Our style guide says: "\(.*\)"$/\1/p' ${inputs.self}/docs/onboarding.md)
          [ -n "$q" ] && grep -qF -- "$q" ${inputs.self}/scions/style-guide/docs/style.md \
            || { echo "style-quote: docs/onboarding.md quotes a rule that scions/style-guide/docs/style.md does not have" >&2; exit 1; }
          touch $out
        '';
      }) flake.checks;
    };
EOF
)"
  (cd "$out/handbook" && nix flake check >&2)

  # what is new upstream since the consumers grafted
  printf '# Style\n\nKeep sentences short. Use one term for one concept.\n' > "$s/docs/style.md"
  release "$s" v1.1.0 2026-11-09 "Reworded: keep sentences short. New rule: use one term for one concept."
  printf '# Page rules\n\nEvery page has a status: draft, working or stable.\n' > "$p/docs/rules.md"
  release "$p" v1.0.1 2026-11-02 "Fixed: the rule lists the allowed statuses."
  printf '# Page rules\n\nEvery page has a status and an owner.\n' > "$p/docs/rules.md"
  # shellcheck disable=SC2016 # Markdown backticks
  release "$p" v2.0.0 2026-11-10 "Every page names an owner." 'Add `owner:` to the front matter of each page.'

  command cp -f "$graft/tests/stub-api.py" "$out/stub-api.py"
  echo '[{"number": 7, "head": {"ref": "graft/upgrade", "repo_id": 1}, "base": {"repo_id": 1}}]' > "$out/open-pr.json"
  cat > "$out/pr.sh" <<'EOF'
#!/usr/bin/env bash
# pr.sh REPO [open]: run `graft ci pr` in REPO, at the commit it has checked out, against a stand-in for
# Forgejo's API; then print the pull request that graft asked it to open (with `open`, PR #7 is open already,
# and graft edits it)
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "${1:?usage: pr.sh REPO [open]}" && pwd)
listing=/nonexistent
if [ "${2:-}" = open ]; then listing=$here/open-pr.json; fi
tmp=$(mktemp -d)
nix run github:NixOS/nixpkgs/ac62194c3917d5f474c1a844b6fd6da2db95077d#python3 -- \
  "$here/stub-api.py" "$tmp/port" "$tmp/auth" "$tmp/req" "$listing" & stub=$!
trap 'kill "$stub" 2>/dev/null || true; rm -f "$tmp"/*; rmdir "$tmp"' EXIT
for _ in $(seq 600); do if [ -s "$tmp/port" ]; then break; fi; sleep 0.1; done
[ -s "$tmp/port" ] || { echo "pr.sh: the stub API did not start" >&2; exit 1; }
rc=0
(cd "$repo" && env API="http://127.0.0.1:$(cat "$tmp/port")" WEB="https://forgejo.example/team/$(basename "$repo")" \
  BASE=main BRANCH=graft/upgrade PR_TOKEN=fixture-token nix run .#graft ci pr) || rc=$?
echo "pr.sh: graft ci pr exited $rc"
if [ -s "$tmp/req" ]; then
  jq -r 'select(.method != "GET") | "pr.sh: \(.method) \(.path)", "title: \(.body.title)", "", .body.body' "$tmp/req"
fi
exit "$rc"
EOF
  chmod +x "$out/pr.sh"
  echo "$out/team-kb"
  echo "$out/handbook"
}

# upgrade: team-kb/ takes a new minor version by hand; page-rules' new major waits
upgrade_fixture() {
  local s=$out/style-guide p=$out/page-rules c=$out/team-kb
  two_providers
  ci_consumer "$c" '  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };'
  # new upstream: a minor version of style-guide (a reworded rule, a new page, a deleted one), a major of page-rules
  printf '# Style\n\nKeep sentences short. Use one term for one concept.\n' > "$s/docs/style.md"
  printf '# Terms\n\nUse one term for one concept.\n' > "$s/docs/terms.md"
  release "$s" v1.1.0 2026-11-09 "Reworded: keep sentences short. New page: docs/terms.md."
  printf '# Page rules\n\nEvery page has a status and an owner.\n' > "$p/docs/rules.md"
  # shellcheck disable=SC2016 # Markdown backticks
  release "$p" v2.0.0 2026-11-10 "Every page names an owner." 'Add `owner:` to the front matter of each page.'
  # the team's own work in progress, not about the upgrade
  printf -- '---\nstatus: draft\n---\n# Retro\n' > "$c/docs/retro.md"
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
  private) private ;;
  ci) ci_fixture ;;
  breaking-change) breaking_change ;;
  upgrade) upgrade_fixture ;;
  publish) publish ;;
esac
