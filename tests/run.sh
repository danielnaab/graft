#!/usr/bin/env bash
# End-to-end test: a provider repo with export-ignore rules, a consumer flake that grafts it, and the
# cycle graft -> check -> drift -> check fails -> upgrade -> graft, then graft upgrading its own
# pin and workflows, and the CI steps against a local origin. Needs nix and git; fetches
# nixpkgs on the first run. Usage: tests/run.sh (from anywhere).
set -euo pipefail

graft=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
git_() { git -c user.name=test -c user.email=test@example.com -c commit.gpgsign=false -c tag.gpgsign=false "$@"; }
pass() { echo "ok   $*"; }
fail() { echo "FAIL $*" >&2; exit 1; }
# expect <description> <command...>: the command must succeed
expect() { local d=$1; shift; if "$@"; then pass "$d"; else fail "$d"; fi; }
# refuse <description> <command...>: the command must fail
refuse() { local d=$1; shift; if "$@"; then fail "$d"; else pass "$d"; fi; }
quiet() { "$@" >/dev/null 2>&1; }

# provider: exports docs, keeps tests/ and its own scions/ to itself
p=$work/provider
mkdir -p "$p/docs" "$p/tests" "$p/scions/upstream"
echo "policy v1" > "$p/docs/policy.md"
echo "readme" > "$p/README.md"
echo "test" > "$p/tests/t.sh"
echo "nested dependency" > "$p/scions/upstream/x.md"
printf 'tests/ export-ignore\nscions/ export-ignore\n' > "$p/.gitattributes"
git_ -C "$p" init -q -b main && git_ -C "$p" add -A && git_ -C "$p" commit -qm v1 && git_ -C "$p" tag v1.0.0

# consumer
c=$work/consumer
mkdir -p "$c"
cat > "$c/flake.nix" <<EOF
{
  inputs.graft.url = "path:$graft";
  inputs.provider = { url = "git+file://$p?ref=v1.0.0&exportIgnore=1"; flake = false; };
  outputs = inputs: inputs.graft.lib.mkGraft { inherit inputs; deps = [ "provider" ]; };
}
EOF
git_ -C "$c" init -q -b main && git_ -C "$c" add -A
cd "$c"

quiet nix run .#graft || fail "nix run .#graft"
expect "export grafted" test -f scions/provider/docs/policy.md
expect "provider's export-ignore applied (tests/)" test ! -e scions/provider/tests
expect "provider's own scions not nested" test ! -e scions/provider/scions
rev1=$(git_ -C "$p" rev-parse v1.0.0)
expect "README names the pinned commit" grep -q "$rev1" scions/README.md

git_ add -A
expect "check passes on a fresh graft" quiet nix flake check

echo "local edit" >> scions/provider/docs/policy.md && git_ add -A
refuse "check catches an edited scion" quiet nix flake check
quiet nix run .#graft && git_ add -A
expect "graft restores the pinned export" quiet nix flake check

# upgrade: provider tags v2; the consumer moves the ref and updates the lock
echo "policy v2" > "$p/docs/policy.md" && git_ -C "$p" commit -qam v2 && git_ -C "$p" tag v2.0.0
sed -i.bak 's/ref=v1.0.0/ref=v2.0.0/' flake.nix && rm flake.nix.bak
quiet nix flake update provider && git_ add -A
refuse "check catches a pin moved without grafting" quiet nix flake check
quiet nix run .#graft && git_ add -A
expect "upgrade grafts v2" grep -q "policy v2" scions/provider/docs/policy.md
expect "check passes after the upgrade" quiet nix flake check

# graft-outdated / graft-upgrade: CI mode (the repo's own flake) and from outside (graft's flake)
echo "policy v2.1" > "$p/docs/policy.md" && git_ -C "$p" commit -qam v2.1 && git_ -C "$p" tag v2.1.0
git_ -C "$p" tag not-a-version
expect "outdated: names the newer tag" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "provider +scions +v2.0.0 +v2.1.0 +outdated"'
expect "outdated --json" sh -c 'nix run .#graft-outdated -- --json 2>/dev/null | jq -e ".[] | select(.name==\"provider\") | .status==\"outdated\" and .latest==\"v2.1.0\"" >/dev/null'
expect "outdated from outside, without the repo's flake" sh -c "nix run '$graft'#outdated -- -C '$c' 2>/dev/null | grep -q outdated"
quiet nix run .#graft-upgrade || fail "graft-upgrade"
git_ add -A
expect "upgrade moved the ref in flake.nix" grep -q 'ref=v2.1.0' flake.nix
expect "upgrade grafted v2.1" grep -q "policy v2.1" scions/provider/docs/policy.md
expect "upgrade kept the export rules" test ! -e scions/provider/tests
expect "check passes after graft-upgrade" quiet nix flake check
expect "a second upgrade has nothing to do" sh -c 'nix run .#graft-upgrade 2>/dev/null | grep -q "nothing to upgrade"'
quiet nix run .#graft-upgrade -- provider@v2.0.0 && git_ add -A
expect "upgrade to a named tag (here back to v2.0.0)" grep -q 'ref=v2.0.0' flake.nix
expect "check passes after a named-tag upgrade" quiet nix flake check
refuse "upgrade refuses an unknown scion" quiet nix run .#graft-upgrade -- nonesuch
# graft pinned by path (here) or branch is untagged: listed, and never moved
expect "outdated: a path-pinned graft is untagged" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "^graft +- +path +- +untagged"'
refuse "upgrade refuses to move an untagged graft" quiet nix run .#graft-upgrade -- graft
expect "and graft's pin is left alone" grep -q "path:$graft" flake.nix
# an upstream that can't be read is an error, never "nothing to upgrade"
mv "$p" "$p.away"
expect "outdated: an unreachable upstream is unknown" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -q " unknown"'
refuse "upgrade fails when it can't read an upstream" quiet nix run .#graft-upgrade
expect "and changes nothing" git_ diff --quiet
mv "$p.away" "$p"

# graft's own pin. A graft repo with this tree at v9.0.0, and at v9.1.0 an assemble.sh that writes
# one more line and a graft-check.yaml with one more comment; consumers pin it by tag.
g=$work/graft-repo
mkdir -p "$g"
git -C "$graft" ls-files -coz --exclude-standard \
  | while IFS= read -r -d '' f; do if [ -e "$graft/$f" ]; then printf '%s\0' "$f"; fi; done \
  | tar -C "$graft" --null -T - -cf - | tar -C "$g" -xf -
git_ -C "$g" init -q -b main && git_ -C "$g" add -A && git_ -C "$g" commit -qm v9.0.0 && git_ -C "$g" tag v9.0.0
# shellcheck disable=SC2016 # a line of assemble.sh, expanded when it runs
printf 'echo "assembled by graft v9.1.0" >> "$out/README.md"\n' >> "$g/lib/assemble.sh"
echo "# changed in v9.1.0" >> "$g/examples/forgejo/graft-check.yaml"
git_ -C "$g" commit -qam v9.1.0 && git_ -C "$g" tag v9.1.0
v9=$work/graft-v9.0.0
git_ clone -q --branch v9.0.0 "$g" "$v9" 2>/dev/null

# selfc <name> <ref>: a consumer of provider@v2 and of graft at <ref>, carrying graft's workflows
# from v9.0.0: graft-check.yaml as shipped (marked), graft-upgrade.yaml without the marker (its own)
selfc() {
  local d=$work/$1
  mkdir -p "$d/.forgejo/workflows"
  cat > "$d/flake.nix" <<EOF
{
  inputs.graft.url = "git+file://$g?ref=$2";
  inputs.provider = { url = "git+file://$p?ref=v2.0.0&exportIgnore=1"; flake = false; };
  outputs = inputs: inputs.graft.lib.mkGraft { inherit inputs; deps = [ "provider" ]; };
}
EOF
  cp "$v9/examples/forgejo/graft-check.yaml" "$d/.forgejo/workflows/"
  tail -n +2 "$v9/examples/forgejo/graft-upgrade.yaml" > "$d/.forgejo/workflows/graft-upgrade.yaml"
  git_ -C "$d" init -q -b main && git_ -C "$d" add -A
  cd "$d"
  quiet nix run .#graft || fail "nix run .#graft in $1"
  git_ add -A
}

selfc self v9.0.0
expect "graft pinned by tag: check passes" quiet nix flake check
expect "outdated lists graft's own pin" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "^graft +- +v9.0.0 +v9.1.0 +outdated"'
echo "# my edit" >> .forgejo/workflows/graft-check.yaml && git_ add -A
refuse "check catches an edited graft workflow" quiet nix flake check
quiet nix run .#graft && git_ add -A
expect "graft restores its workflow" cmp -s .forgejo/workflows/graft-check.yaml "$v9/examples/forgejo/graft-check.yaml"
echo "# my edit" >> .forgejo/workflows/graft-upgrade.yaml && git_ add -A
cp .forgejo/workflows/graft-upgrade.yaml "$work/mine.yaml"
expect "a workflow without the marker is the repo's: check passes" quiet nix flake check
quiet nix run .#graft-upgrade || fail "graft-upgrade (graft itself)"
git_ add -A
expect "upgrade moved graft's ref" grep -q "ref=v9.1.0" flake.nix
# shellcheck disable=SC2016 # jq's $r
expect "upgrade re-locked graft" quiet jq -e --arg r "$(git_ -C "$g" rev-parse v9.1.0)" '.nodes.graft.locked.rev == $r' flake.lock
expect "scions rebuilt by the new version's assemble.sh" grep -q "assembled by graft v9.1.0" scions/README.md
expect "graft's workflow refreshed to the new version's" cmp -s .forgejo/workflows/graft-check.yaml "$g/examples/forgejo/graft-check.yaml"
expect "a workflow without the marker untouched" cmp -s .forgejo/workflows/graft-upgrade.yaml "$work/mine.yaml"
expect "check passes under the new pin" quiet nix flake check
quiet nix run .#graft-upgrade -- graft@v9.0.0 && git_ add -A
expect "graft@TAG: graft moved to a named tag" grep -q "ref=v9.0.0" flake.nix
refuse "and the scions rebuilt by that version" grep -q "assembled by graft v9.1.0" scions/README.md
expect "check passes after graft@TAG" quiet nix flake check

selfc self-outside v9.0.0
cd "$work"
quiet nix run "$graft"#upgrade -- -C "$work/self-outside" graft@v9.1.0 || fail "graft-upgrade -C (graft itself)"
cd "$work/self-outside" && git_ add -A
expect "-C: graft moved" grep -q "ref=v9.1.0" flake.nix
expect "-C: scions rebuilt by the new version" grep -q "assembled by graft v9.1.0" scions/README.md
expect "-C: workflow refreshed" cmp -s .forgejo/workflows/graft-check.yaml "$g/examples/forgejo/graft-check.yaml"
expect "-C: check passes under the new pin" quiet nix flake check

selfc self-branch main
expect "outdated: a branch-pinned graft is untagged" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "^graft +- +main +- +untagged"'
quiet nix run .#graft-upgrade && git_ add -A
expect "upgrade moves the scion" grep -q "ref=v2.1.0" flake.nix
expect "but leaves graft's branch pin alone" grep -q "graft-repo?ref=main" flake.nix
expect "check passes" quiet nix flake check

# graft-ci, the workflows' steps, against a local origin
selfc self-ci v9.0.0
git_ commit -qm init
git_ init -q --bare "$work/origin.git" && git_ remote add origin "$work/origin.git" && git_ push -q origin main
expect "ci check passes" quiet nix run .#graft-ci -- check
GITHUB_OUTPUT=$work/ci-out BRANCH=graft/upgrade quiet nix run .#graft-ci -- upgrade || fail "graft-ci upgrade"
expect "ci upgrade: changed=true" grep -qx changed=true "$work/ci-out"
expect "ci upgrade pushed graft/upgrade" quiet git_ -C "$work/origin.git" rev-parse --verify refs/heads/graft/upgrade
body=$(git_ -C "$work/origin.git" log -1 --format=%b graft/upgrade)
expect "its commit names graft's move" grep -q "graft v9.0.0 -> v9.1.0" <<<"$body"
expect "and the refreshed workflow" grep -q "refreshed .forgejo/workflows/graft-check.yaml" <<<"$body"
: > "$work/ci-out"
GITHUB_OUTPUT=$work/ci-out BRANCH=graft/upgrade quiet nix run .#graft-ci -- upgrade || fail "graft-ci upgrade, again"
expect "ci upgrade again: changed=false" grep -qx changed=false "$work/ci-out"
expect "ci pr: no PR when the branch is already in the base" sh -c 'API=http://127.0.0.1:9 WEB=x BASE=main BRANCH=graft/upgrade PR_TOKEN=x nix run .#graft-ci -- pr 2>/dev/null | grep -q "already in main"'
refuse "ci pr: refuses without a token" quiet env API=x WEB=x BASE=main BRANCH=graft/upgrade PR_TOKEN= nix run .#graft-ci -- pr

# dir: any relative path, here nested with a space and an apostrophe, and a hidden one
consumer() {  # consumer <name> <dir-attr>: a fresh consumer of provider@v2 with mkGraft's dir set
  local d=$work/$1
  mkdir -p "$d"
  cat > "$d/flake.nix" <<EOF
{
  inputs.graft.url = "path:$graft";
  inputs.provider = { url = "git+file://$p?ref=v2.0.0&exportIgnore=1"; flake = false; };
  outputs = inputs: inputs.graft.lib.mkGraft { inherit inputs; deps = [ "provider" ]; $2 };
}
EOF
  git_ -C "$d" init -q -b main && git_ -C "$d" add -A
  cd "$d"
}
consumer custom 'dir = "docs/Dan'"'"'s Scions/";'
quiet nix run .#graft && git_ add -A
expect "custom dir: nested, a space, an apostrophe" test -f "docs/Dan's Scions/provider/docs/policy.md"
expect "custom dir: check passes" quiet nix flake check
echo "edit" >> "docs/Dan's Scions/provider/README.md" && git_ add -A
refuse "custom dir: check catches drift" quiet nix flake check

consumer hidden 'dir = ".scions";'
quiet nix run .#graft && git_ add -A
expect "hidden dir: .scions grafted and checked" quiet nix flake check

consumer guarded 'dir = "src";'
mkdir -p src && echo "my code" > src/main.py && git_ add -A
refuse "graft won't replace a directory it didn't write" quiet nix run .#graft
expect "and the directory is untouched" grep -q "my code" src/main.py

consumer escape 'dir = "../outside";'
refuse "a dir outside the repo is rejected" quiet nix flake check
expect "and nothing is written outside" test ! -e "$work/outside"

echo "all passed"
