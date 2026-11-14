#!/usr/bin/env bash
# shellcheck disable=SC2016 # backticks in CHANGELOG text are markdown
# fixtures.sh SLICE OUT: build the fixture that a guide is read against, in OUT (which must not exist).
# A guide's reading protocol: a reader follows the page literally in this fixture (docs/development.md).
#
# Every provider is a local repository with a bare origin, OUT/NAME.git, grafted by its file:// URL.
# git refuses file:// submodules by default (git 2.38+): `. OUT/env.sh` first, which allows them for
# this shell (GIT_CONFIG_COUNT; nothing global changes). Consumers use this checkout's graft as a
# path: input (OUT/graft, a copy without the inputs.self.submodules line, which Nix refuses on path:).
#
#   first-pattern   page-rules.git (a provider with graft.nix: a tool, page-lint, and a check of the
#                   consumer's docs/), and three starting repositories, no graft yet: new/ (git init, no
#                   commits), existing/ (its own flake with a check, committed), notes/ (pages only)
#   check-fails     page-rules.git at v1.0.0, v1.1.0; consumer/ grafts it and passes `graft check`; then a
#                   clone per failing rule: edited/ (an uncommitted edit in the scion), moved/ (the scion
#                   checked out at v1.1.0, gitlink not recorded), unreleased/ (a local commit in the scion,
#                   recorded), empty/ (a fresh clone, scions not filled), stray/ (a folder that isn't a
#                   submodule), relative/ (a relative URL in .gitmodules), noself/ (flake.nix without
#                   inputs.self.submodules)
#   fix-a-provider  policy-kit/ (the provider's own clone; origin policy-kit.git) at v1.0.0 with a typo in
#                   docs/status.md and a test (tests/run.sh); team-kb/ grafts it at v1.0.0, committed, with
#                   graft's workflows. The reader fixes the typo inside team-kb/scions/policy-kit.
#   gate-merges     page-rules.git (graft.nix: checks.pages, which reads the consumer's docs/); team-kb/
#                   grafts it with graft's workflows and has one committed page that breaks the rule, and an
#                   untracked one; existing/ has a check of its own (perSystem) as well
#   private         provider.git (public) and notes-provider.git (private: a file:// URL stands in for
#                   ssh), both v1.0.0 and v1.1.0; moving/: a repo with a committed notebook/ (an uncommitted
#                   edit and an untracked draft), no flake yet; new/: notebook/ only; consumer/: a top-level
#                   flake grafting provider, with graft's workflows; server-host-key.pub (a made-up host key)
#   ci              style-guide.git (v1.0.0, v1.1.0 with a CHANGELOG) and page-rules.git (v1.0.0, v1.0.1,
#                   v2.0.0 with an Upgrading section); team-kb/ and handbook/ graft both at v1.0.0, with
#                   graft's workflows, each pushed to a bare origin (NAME.git); handbook/'s own check fails
#                   on style-guide v1.1.0. pr.sh REPO [open] runs `graft ci pr` in REPO against
#                   stub-api.py and prints the pull request graft asked it to open
#   breaking-change page-rules/ (the provider's clone): v1.0.0, v1.1.0, and on branch `lifecycle` an
#                   untagged v2.0.0 that renames the front-matter field status: to lifecycle:, and
#                   docs/status.md to docs/lifecycle.md, with Upgrading steps; team-kb/ at v1.1.0, whose
#                   pages (and a template outside docs/, which the check misses) say status:
#   upgrade         team-kb/ grafts style-guide and page-rules at v1.0.0, committed; then style-guide v1.1.0
#                   (a reworded page, a new one, a deleted one) and page-rules v2.0.0 (Upgrading steps); an
#                   untracked docs/retro.md that is not about the upgrade
#   publish         note-kit/ (a provider to release from: rules, bin/note-lint, graft.nix building it, a
#                   seed file, tests/, a CHANGELOG; v1.0.0 lightweight, v1.1.0 annotated; then an
#                   uncommitted fix and an untracked draft) with origin note-kit.git; team-notes/ grafts
#                   v1.1.0 and runs note-lint in its own check (perSystem), committed and passing
#   worktrees       team-kb/ grafting page-rules and style-guide, committed, with one more branch `draft`;
#                   clone-only/ (git clone without --recurse-submodules: the scions are empty)
#   copy-mode       old/: a graft v2.12 consumer to move (flake.nix with a `flake = false` provider input,
#                   scions/ a committed copy with the v2.12 README table; no flake.lock), and page-rules.git,
#                   its provider, at v1.0.0
#
# Needs nix, git 2.38 or later and jq. No network for the providers; checks use the nixpkgs graft's
# flake.lock pins (likely cached). copy-mode's old/ names github:danielnaab/graft/v2.12.0.
set -euo pipefail

src=$(cd "$(dirname "$0")/.." && pwd)
slices="first-pattern check-fails fix-a-provider gate-merges private ci breaking-change upgrade publish worktrees copy-mode"
usage="usage: fixtures.sh ${slices// /|} OUT"
slice=${1:?$usage}
out=${2:?$usage}
case " $slices " in *" $slice "*) ;; *) echo "fixtures.sh: no fixture $slice ($slices)" >&2; exit 2 ;; esac
[ ! -e "$out" ] || { echo "fixtures.sh: $out exists; name a new directory" >&2; exit 1; }
mkdir -p "$out"
out=$(cd "$out" && pwd)
nixpkgs="github:NixOS/nixpkgs/$(jq -r '.nodes.nixpkgs.locked.rev' "$src/flake.lock")"

cat > "$out/env.sh" <<'EOF'
# . env.sh: let git clone this fixture's file:// providers as submodules, in this shell only
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=protocol.file.allow GIT_CONFIG_VALUE_0=always
EOF
# shellcheck source=/dev/null
. "$out/env.sh"
g() { git -c user.name=fixture -c user.email=fixture@example.invalid -c commit.gpgsign=false -c tag.gpgsign=false "$@"; }

# graft: a copy of this checkout, as a path: input
graft=$out/graft
mkdir -p "$graft" && (cd "$src" && tar --exclude=./.git --exclude=./result -cf - .) | tar -C "$graft" -xf -
sed -i '/^  inputs.self.submodules = true;$/d' "$graft/flake.nix"

# repo DIR: git init, with an identity and no signing, as a reader's would have
repo() {
  mkdir -p "$1"
  g -C "$1" init -q -b main
  git -C "$1" config user.name fixture
  git -C "$1" config user.email fixture@example.invalid
  git -C "$1" config commit.gpgsign false
  git -C "$1" config tag.gpgsign false
}
url() { echo "file://$out/$1.git"; }
# prov NAME [DIR]: a provider's clone (default OUT/src/NAME) with its bare origin OUT/NAME.git
prov() {
  local d=${2:-$out/src/$1}
  repo "$d"
  g init -q --bare "$out/$1.git"
  g -C "$d" remote add origin "$(url "$1")"
}
# release DIR TAG [-a]: commit everything, tag (annotated with -a), push
release() {
  g -C "$1" add -A && g -C "$1" commit -q --allow-empty -m "$2"
  if [ "${3:-}" = -a ]; then g -C "$1" tag -a -m "$2" "$2"; else g -C "$1" tag "$2"; fi
  g -C "$1" push -q origin HEAD:refs/heads/main "refs/tags/$2"
}
# flake DIR [PERSYSTEM]: the four-line flake (graft by path:), with the repo's own perSystem if given
flake() {
  if [ -z "${2:-}" ]; then
    printf '{\n  inputs.self.submodules = true;\n  inputs.graft.url = "path:%s";\n  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };\n}\n' "$graft" > "$1/flake.nix"
  else
    printf '{\n  inputs.self.submodules = true;\n  inputs.graft.url = "path:%s";\n  outputs = inputs: inputs.graft.lib.mkFlake {\n    inherit inputs;\n    perSystem = %s;\n  };\n}\n' "$graft" "$2" > "$1/flake.nix"
  fi
}
# consumer DIR: a repo with the four-line flake and its lock, committed
consumer() {
  repo "$1"
  flake "$1"
  g -C "$1" add -A && g -C "$1" commit -qm "flake"
  (cd "$1" && nix flake lock 2>/dev/null) && g -C "$1" add -A && g -C "$1" commit -qm "lock"
}
# graft_in DIR URL[@TAG]...: graft each, committed
graft_in() {
  local d=$1 u
  shift
  for u in "$@"; do (cd "$d" && nix run .#graft -- add "$u" >/dev/null); done
  g -C "$d" add -A && g -C "$d" commit -qm "graft $*"
}
workflows() { mkdir -p "$1/.forgejo/workflows" && cp "$graft"/template/.forgejo/workflows/*.yaml "$1/.forgejo/workflows/"; }

# page-rules: pages must say `status:`; a tool (page-lint DIR) and a check of the consumer's docs/
page_rules() {
  local d=$out/src/page-rules
  prov page-rules
  mkdir -p "$d/docs" "$d/bin"
  printf '# Page rules\n\nEvery page under docs/ starts with front matter naming its status:.\n' > "$d/docs/status.md"
  cat > "$d/bin/page-lint" <<'EOF'
# page-lint DIR: every .md page under DIR has a status: line
bad=0
while IFS= read -r f; do grep -q '^status:' "$f" || { echo "page-lint: $f: no status:"; bad=1; }; done < <(find "${1:-docs}" -name '*.md')
exit "$bad"
EOF
  cat > "$d/graft.nix" <<'EOF'
# page-rules wires itself into the consumer's flake (graft's mkFlake calls this once per system)
{ pkgs, scion, consumer, ... }:
let
  page-lint = pkgs.writeShellApplication {
    name = "page-lint";
    runtimeInputs = [ pkgs.findutils pkgs.gnugrep ];
    text = builtins.readFile "${scion}/bin/page-lint";
  };
in
{
  tools.page-lint = page-lint;
  # the consumer's pages, when it has a docs/
  checks.pages = pkgs.runCommand "pages" { } (
    if builtins.pathExists "${consumer}/docs" then "${page-lint}/bin/page-lint ${consumer}/docs && touch $out" else "touch $out");
}
EOF
  printf '# Changelog\n\n## v1.0.0\n- First release.\n' > "$d/CHANGELOG.md"
  release "$d" v1.0.0
}
style_guide() {
  local d=$out/src/style-guide
  prov style-guide
  mkdir -p "$d/docs"
  printf '# Writing style\n\nShort sentences.\n' > "$d/docs/writing.md"
  printf '# Old advice\n' > "$d/docs/old.md"
  printf '# Changelog\n\n## v1.0.0\n- First release.\n' > "$d/CHANGELOG.md"
  release "$d" v1.0.0
}

first_pattern() {
  page_rules
  repo "$out/new"
  repo "$out/existing"
  cat > "$out/existing/flake.nix" <<EOF
{
  description = "Pantry";
  inputs.nixpkgs.url = "$nixpkgs";
  outputs = { self, nixpkgs }: {
    checks.x86_64-linux.notes = nixpkgs.legacyPackages.x86_64-linux.runCommand "notes" { } "test -f \${self}/notes.md && touch \$out";
  };
}
EOF
  echo "# Notes" > "$out/existing/notes.md"
  g -C "$out/existing" add -A && g -C "$out/existing" commit -qm "own flake"
  repo "$out/notes" && mkdir -p "$out/notes/docs" && printf -- '---\nstatus: working\n---\n# Notes\n' > "$out/notes/docs/a.md"
  g -C "$out/notes" add -A && g -C "$out/notes" commit -qm notes
}

check_fails() {
  local c=$out/consumer d
  page_rules
  sed -i 's/naming its/naming its own/' "$out/src/page-rules/docs/status.md" && release "$out/src/page-rules" v1.1.0
  consumer "$c"
  graft_in "$c" "$(url page-rules)@v1.0.0"
  workflows "$c" && g -C "$c" add -A && g -C "$c" commit -qm workflows
  (cd "$c" && nix run .#graft check >/dev/null)
  for d in edited moved unreleased empty stray relative noself; do g clone -q "$c" "$out/$d"; done
  for d in edited moved unreleased stray relative noself; do g -C "$out/$d" submodule update -q --init; done
  echo "Every page starts with front matter." > "$out/edited/scions/page-rules/docs/status.md"
  g -C "$out/moved/scions/page-rules" -c advice.detachedHead=false checkout -q v1.1.0
  echo "a fix" >> "$out/unreleased/scions/page-rules/docs/status.md"
  g -C "$out/unreleased/scions/page-rules" commit -qam "a local fix"
  g -C "$out/unreleased" add scions/page-rules && g -C "$out/unreleased" commit -qm "record the fix"
  mkdir -p "$out/stray/scions/notes" && echo "# mine" > "$out/stray/scions/notes/a.md"
  g -C "$out/stray" add -A && g -C "$out/stray" commit -qm "a folder of my own"
  git -C "$out/relative" config -f .gitmodules submodule.scions/page-rules.url ../page-rules.git
  g -C "$out/relative" commit -qam "relative URL"
  sed -i '/inputs.self.submodules/d' "$out/noself/flake.nix" && g -C "$out/noself" commit -qam "no self.submodules"
}

fix_a_provider() {
  local p=$out/policy-kit c=$out/team-kb
  prov policy-kit "$p"
  mkdir -p "$p/docs" "$p/tests"
  printf '# Status\n\nEvery page has a status: draft, working, stabel or deprecated.\n' > "$p/docs/status.md"
  printf '#!/bin/sh\n# the policy names each status correctly\ngrep -q "stable" docs/status.md || { echo "status.md: no stable"; exit 1; }\n' > "$p/tests/run.sh"
  chmod +x "$p/tests/run.sh"
  printf '# Changelog\n\n## v1.0.0\n- First release.\n' > "$p/CHANGELOG.md"
  release "$p" v1.0.0
  consumer "$c"
  graft_in "$c" "$(url policy-kit)"
  workflows "$c" && g -C "$c" add -A && g -C "$c" commit -qm workflows
}

gate_merges() {
  local c=$out/team-kb e=$out/existing
  page_rules
  consumer "$c"
  graft_in "$c" "$(url page-rules)"
  workflows "$c"
  mkdir -p "$c/docs" && printf -- '---\nstatus: working\n---\n# Good\n' > "$c/docs/good.md" && printf '# Missing its status\n' > "$c/docs/bad.md"
  g -C "$c" add -A && g -C "$c" commit -qm "pages"
  printf '# Draft, untracked\n' > "$c/docs/draft.md"
  consumer "$e"
  # shellcheck disable=SC2016 # Nix's own expressions
  flake "$e" '{ pkgs, ... }: { checks.notes = pkgs.runCommand "notes" { } "test -f ${inputs.self}/notes.md && touch $out"; }'
  echo "# Notes" > "$e/notes.md" && mkdir -p "$e/docs" && printf '# No status\n' > "$e/docs/x.md"
  g -C "$e" add -A && g -C "$e" commit -qm "own check"
  graft_in "$e" "$(url page-rules)"
}

private() {
  local p=$out/src/provider n=$out/src/notes-provider
  prov provider && mkdir -p "$p/docs" && echo "# Shared" > "$p/docs/shared.md"
  printf '# Changelog\n\n## v1.0.0\n- First.\n' > "$p/CHANGELOG.md" && release "$p" v1.0.0
  printf '\n## v1.1.0\n- Second.\n' >> "$p/CHANGELOG.md" && release "$p" v1.1.0
  prov notes-provider && mkdir -p "$n/docs" && echo "# Private notes rules" > "$n/docs/rules.md"
  printf '# Changelog\n\n## v1.0.0\n- First.\n' > "$n/CHANGELOG.md" && release "$n" v1.0.0
  printf '\n## v1.1.0\n- Second.\n' >> "$n/CHANGELOG.md" && release "$n" v1.1.0
  repo "$out/moving" && mkdir -p "$out/moving/notebook"
  echo "# Journal" > "$out/moving/notebook/journal.md" && echo "secret: x" > "$out/moving/notebook/private.md"
  g -C "$out/moving" add -A && g -C "$out/moving" commit -qm notebook
  echo "an uncommitted edit" >> "$out/moving/notebook/journal.md" && echo "# Draft" > "$out/moving/notebook/draft.md"
  repo "$out/new" && mkdir -p "$out/new/notebook" && echo "# Notes" > "$out/new/notebook/a.md"
  consumer "$out/consumer"
  graft_in "$out/consumer" "$(url provider)@v1.0.0"
  workflows "$out/consumer" && g -C "$out/consumer" add -A && g -C "$out/consumer" commit -qm workflows
  echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGraftTestHostKeyNotARealOne+/0123456789abcdefgh made-up" > "$out/server-host-key.pub"
}

ci_fixture() {
  local d c
  page_rules
  style_guide
  for c in team-kb handbook; do
    d=$out/$c
    consumer "$d"
    graft_in "$d" "$(url style-guide)@v1.0.0" "$(url page-rules)@v1.0.0"
    workflows "$d" && g -C "$d" add -A && g -C "$d" commit -qm workflows
  done
  # handbook's own check: its style is v1.0.0's (style-guide v1.1.0 breaks it)
  # shellcheck disable=SC2016 # Nix's own expressions
  flake "$out/handbook" '{ pkgs, scions, ... }: {
      checks.style = pkgs.runCommand "style" { } (builtins.concatStringsSep " " [
        "grep -qx \"Short sentences.\" ${scions.style-guide}/docs/writing.md"
        "|| { echo \"style: handbook follows Short sentences.; style-guide changed it\" >&2; exit 1; };"
        "touch $out" ]);
    }'
  g -C "$out/handbook" add -A && g -C "$out/handbook" commit -qm "own check"
  for c in team-kb handbook; do
    g init -q --bare "$out/$c.git" && g -C "$out/$c" remote add origin "$out/$c.git" && g -C "$out/$c" push -q origin main
  done
  d=$out/src/style-guide
  printf '# Writing style\n\nShort sentences. Plain words.\n' > "$d/docs/writing.md"
  printf '\n## v1.1.0\n- Plain words.\n' >> "$d/CHANGELOG.md" && release "$d" v1.1.0
  d=$out/src/page-rules
  printf '\n## v1.0.1\n- A fix.\n' >> "$d/CHANGELOG.md" && release "$d" v1.0.1
  printf '\n## v2.0.0\n### Upgrading\n- Pages need `lifecycle:` instead of `status:`.\n' >> "$d/CHANGELOG.md" && release "$d" v2.0.0
  cp "$src/tests/stub-api.py" "$out/stub-api.py"
  cat > "$out/pr.sh" <<EOF
#!/usr/bin/env bash
# pr.sh REPO [open]: run graft ci pr in REPO against stub-api.py; print the PR graft asked it to open
# (with \`open\`: a PR is already open, #7, and graft edits it)
set -euo pipefail
cd "\$1"
t=\$(mktemp -d)
listing=/nonexistent
if [ "\${2:-}" = open ]; then echo '[{"number": 7, "head": {"ref": "graft/upgrade", "repo_id": 1}, "base": {"repo_id": 1}}]' > "\$t/listing"; listing=\$t/listing; fi
nix run "$nixpkgs#python3" -- "$out/stub-api.py" "\$t/port" "\$t/auth" "\$t/req" "\$listing" & stub=\$!
trap 'kill \$stub 2>/dev/null; rm -rf "\$t"' EXIT
for _ in \$(seq 100); do [ -s "\$t/port" ] && break; sleep 0.1; done
API="http://127.0.0.1:\$(cat "\$t/port")" WEB="http://forgejo.example/team/\$(basename "\$PWD")" BASE=main BRANCH=graft/upgrade PR_TOKEN=x \\
  nix run .#graft -- ci pr || true
jq -r 'select(.method != "GET") | "\\(.method) \\(.path)\\n\\nTitle: \\(.body.title)\\n\\n\\(.body.body)"' "\$t/req"
EOF
  chmod +x "$out/pr.sh"
}

breaking_change() {
  local p=$out/page-rules c=$out/team-kb
  page_rules
  g clone -q "$out/page-rules.git" "$p"
  git -C "$p" config user.name fixture && git -C "$p" config user.email fixture@example.invalid
  git -C "$p" config commit.gpgsign false && git -C "$p" config tag.gpgsign false
  sed -i 's/^## v1.0.0/## v1.1.0\n- Clearer wording.\n\n## v1.0.0/' "$p/CHANGELOG.md"
  g -C "$p" commit -qam v1.1.0 && g -C "$p" tag v1.1.0 && g -C "$p" push -q origin main v1.1.0
  consumer "$c"
  graft_in "$c" "$(url page-rules)@v1.1.0"
  workflows "$c"
  mkdir -p "$c/docs" "$c/templates"
  printf -- '---\nstatus: working\n---\n# Team page\n' > "$c/docs/team.md"
  printf -- '---\nstatus: draft\n---\n# New page template\n' > "$c/templates/page.md"
  g -C "$c" add -A && g -C "$c" commit -qm pages
  g -C "$p" checkout -q -b lifecycle
  sed -i 's/status:/lifecycle:/g' "$p/bin/page-lint"
  g -C "$p" mv docs/status.md docs/lifecycle.md
  sed -i 's/naming its status:/naming its lifecycle:/' "$p/docs/lifecycle.md"
  sed -i 's/^## v1.1.0/## v2.0.0\n### Upgrading\n- Rename the front-matter field `status:` to `lifecycle:` in every page.\n- docs\/status.md is now docs\/lifecycle.md: update links to it.\n\n## v1.1.0/' "$p/CHANGELOG.md"
  g -C "$p" commit -qam "v2.0.0: status: is lifecycle:"
}

upgrade_fixture() {
  local c=$out/team-kb d
  page_rules
  style_guide
  consumer "$c"
  graft_in "$c" "$(url style-guide)@v1.0.0" "$(url page-rules)@v1.0.0"
  workflows "$c" && g -C "$c" add -A && g -C "$c" commit -qm workflows
  d=$out/src/style-guide
  printf '# Writing style\n\nShort sentences. Plain words.\n' > "$d/docs/writing.md"
  echo "# Headings" > "$d/docs/headings.md" && g -C "$d" rm -q docs/old.md
  printf '\n## v1.1.0\n- Plain words; a headings page; the old advice removed.\n' >> "$d/CHANGELOG.md" && release "$d" v1.1.0
  d=$out/src/page-rules
  printf '\n## v2.0.0\n### Upgrading\n- Pages need `lifecycle:` instead of `status:`.\n' >> "$d/CHANGELOG.md" && release "$d" v2.0.0
  mkdir -p "$c/docs" && echo "# Retro" > "$c/docs/retro.md"
}

publish() {
  local p=$out/note-kit c=$out/team-notes
  prov note-kit "$p"
  mkdir -p "$p/docs" "$p/bin" "$p/seed" "$p/tests"
  echo "# Note rules: every note has a title line." > "$p/docs/rules.md"
  # shellcheck disable=SC2016 # the tool's own text
  printf '# note-lint DIR: every note starts with a # title\nfor f in "${1:-.}"/*.md; do head -1 "$f" | grep -q "^# " || { echo "note-lint: $f: no title"; exit 1; }; done\n' > "$p/bin/note-lint"
  echo "titles = required" > "$p/seed/note-kit.conf"
  printf '#!/bin/sh\nbash bin/note-lint docs\n' > "$p/tests/run.sh" && chmod +x "$p/tests/run.sh"
  cat > "$p/graft.nix" <<'EOF'
# note-kit wires itself into a consumer's flake: graft's mkFlake calls this once per system
{ pkgs, scion, ... }:
{
  tools.note-lint = pkgs.writeShellApplication {
    name = "note-lint";
    runtimeInputs = [ pkgs.coreutils pkgs.gnugrep ];
    text = builtins.readFile "${scion}/bin/note-lint";
  };
}
EOF
  printf '# Changelog\n\n## v1.0.0\n- First release.\n' > "$p/CHANGELOG.md"
  release "$p" v1.0.0
  printf '\n## v1.1.0\n- graft.nix: note-lint on PATH.\n' >> "$p/CHANGELOG.md"
  release "$p" v1.1.0 -a
  consumer "$c"
  graft_in "$c" "$(url note-kit)@v1.1.0"
  # shellcheck disable=SC2016 # Nix's own expressions
  flake "$c" '{ pkgs, scions, ... }: { checks.notes = pkgs.runCommand "notes" { } "${scions.note-kit.tools.note-lint}/bin/note-lint ${inputs.self}/notes && touch $out"; }'
  mkdir -p "$c/notes" && echo "# Monday" > "$c/notes/monday.md" && cp "$p/seed/note-kit.conf" "$c/"
  g -C "$c" add -A && g -C "$c" commit -qm "notes, checked"
  (cd "$c" && nix flake check >/dev/null 2>&1) || { echo "fixtures.sh: publish: team-notes' check fails" >&2; exit 1; }
  # the next release's work, not committed
  sed -i 's/no title/no title (start it with "# ")/' "$p/bin/note-lint"
  echo "# Draft of a new rule" > "$p/docs/draft.md"
}

worktrees() {
  local c=$out/team-kb
  page_rules
  style_guide
  consumer "$c"
  graft_in "$c" "$(url page-rules)" "$(url style-guide)"
  g -C "$c" branch draft
  g clone -q "$c" "$out/clone-only"
}

copy_mode() {
  local c=$out/old r
  page_rules
  r=$(g -C "$out/src/page-rules" rev-parse v1.0.0)
  repo "$c"
  mkdir -p "$c/scions/page-rules"
  cat > "$c/flake.nix" <<EOF
{
  inputs = {
    graft.url = "github:danielnaab/graft/v2.12.0";
    page-rules = { url = "git+$(url page-rules)?ref=v1.0.0&exportIgnore=1"; flake = false; };
  };
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
EOF
  g -C "$out/src/page-rules" archive v1.0.0 | tar -C "$c/scions/page-rules" -xf -
  cat > "$c/scions/README.md" <<EOF
<!-- managed by graft: https://github.com/danielnaab/graft -->
# scions/

Scions: each folder is another repository's export, grafted here by
[graft](https://github.com/danielnaab/graft) at the commit pinned in \`flake.lock\`. Do not edit
them. To upgrade: \`nix run .#graft upgrade\` (or change the tag in \`flake.nix\`, then
\`nix run .#graft\`), then commit. \`nix flake check\` fails if this directory drifts from the pins.

| Scion | Commit |
|---|---|
| [page-rules](page-rules/) | \`$r\` |
EOF
  g -C "$c" add -A && g -C "$c" commit -qm "graft v2.12, copy mode"
}

case $slice in
  first-pattern) first_pattern ;;
  check-fails) check_fails ;;
  fix-a-provider) fix_a_provider ;;
  gate-merges) gate_merges ;;
  private) private ;;
  ci) ci_fixture ;;
  breaking-change) breaking_change ;;
  upgrade) upgrade_fixture ;;
  publish) publish ;;
  worktrees) worktrees ;;
  copy-mode) copy_mode ;;
esac
echo "fixtures.sh: $slice in $out (first, in your shell: . $out/env.sh)"
