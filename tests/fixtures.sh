#!/usr/bin/env bash
# fixtures.sh SLICE OUT: build the fixture that a guide is read against, in OUT (which must not exist).
# A guide's reading protocol: a reader follows the page literally in this fixture (docs/development.md).
#
#   check-fails     a consumer that grafts a local provider with this checkout's graft, then drifts three
#                   ways: a committed edit to a scion, an uncommitted formatter run, an untracked __pycache__
#   first-pattern   three starting repositories, no graft yet: new/ (git init, no commits), existing/ (its
#                   own flake with a check, and a .gitignore that drops flake.lock), locked/ (its own
#                   flake, with flake.lock committed)
#   private         private files kept out of the Nix store, and a private provider: provider/ and
#                   notes-provider/ (tags v1.0.0 and v1.1.0, a CHANGELOG; notes-provider is the private one,
#                   git+file standing in for git+ssh), moving/ (a flake at the top grafts provider into
#                   .scions beside a committed notebook/, with an uncommitted edit and an untracked draft in
#                   it, and graft's workflows), new/ (a notebook/, no flake yet), consumer/ (a flake at the
#                   top that grafts provider, and graft's workflows), server-host-key.pub (a made-up host key)
#
# Needs nix, git 2.28 or later and GNU sed. Network: check-fails fetches python3 and ruff from the nixpkgs
# registry (unpinned, so ruff's output can vary); first-pattern's flakes pin a nixpkgs that the reader fetches.
set -euo pipefail

graft=$(cd "$(dirname "$0")/.." && pwd)
slice=${1:?usage: fixtures.sh check-fails|first-pattern|private OUT}
out=${2:?usage: fixtures.sh check-fails|first-pattern|private OUT}
case $slice in check-fails|first-pattern|private) ;; *) echo "fixtures.sh: no fixture $slice (check-fails, first-pattern, private)" >&2; exit 2 ;; esac
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
  printf '# Meeting transcript\n\nprivate-sentinel-moving: only this repository may hold this line.\n' > "$m/notebook/transcript.md"
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
  printf '# Meeting transcript\n\nprivate-sentinel-new: only this repository may hold this line.\n' > "$n/notebook/transcript.md"
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

case $slice in
  check-fails) check_fails ;;
  first-pattern) first_pattern ;;
  private) private ;;
esac
