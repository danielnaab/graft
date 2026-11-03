#!/usr/bin/env bash
# End-to-end test: a provider repo with export-ignore rules, a consumer flake that grafts it, and the
# cycle graft -> check -> drift -> check fails -> upgrade -> graft. Needs nix and git; fetches
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
expect "outdated --json" sh -c 'nix run .#graft-outdated -- --json 2>/dev/null | jq -e ".[0] | .status==\"outdated\" and .latest==\"v2.1.0\"" >/dev/null'
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
