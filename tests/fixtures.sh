#!/usr/bin/env bash
# fixtures.sh SLICE OUT: build the fixture that a guide is read against, in OUT (which must not exist).
# A guide's reading protocol: a reader follows the page literally in this fixture (docs/development.md).
#
#   check-fails     a consumer that grafts a local provider with this checkout's graft, then drifts three
#                   ways: a committed edit to a scion, an uncommitted formatter run, an untracked __pycache__
#   first-pattern   three starting repositories, no graft yet: new/ (git init, no commits), existing/ (its
#                   own flake with a check, and a .gitignore that drops flake.lock), locked/ (its own
#                   flake, with flake.lock committed)
#   override        a consumer that grafts two local providers (policy-kit at v1.0.0 with a typo, v1.1.0 tagged,
#                   with a CHANGELOG and an untracked draft; glossary at v1.0.0) with this checkout's graft
#                   and its two workflows (graft-check.yaml with a kept paths-ignore line). Its own checks
#                   run policy-kit's checker (which reads policy-kit.conf) and read the glossary's export.
#                   Committed and passing, then the team's change made in the scion: a staged edit to the
#                   status policy, and an untracked new page
#
# Needs nix, git 2.28 or later and GNU sed. Network: check-fails fetches python3 and ruff from the nixpkgs
# registry (unpinned, so ruff's output can vary); first-pattern's flakes pin a nixpkgs that the reader fetches.
set -euo pipefail

graft=$(cd "$(dirname "$0")/.." && pwd)
slice=${1:?usage: fixtures.sh check-fails|first-pattern|override OUT}
out=${2:?usage: fixtures.sh check-fails|first-pattern|override OUT}
case $slice in check-fails|first-pattern|override) ;; *) echo "fixtures.sh: no fixture $slice (check-fails, first-pattern, override)" >&2; exit 2 ;; esac
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

case $slice in
  check-fails) check_fails ;;
  first-pattern) first_pattern ;;
  override) override ;;
esac
