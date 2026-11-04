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

# majors: a new major is reported, and crossed only when named. v3.0.0 also brings a CHANGELOG.md
# (with text a shell would expand, to show the PR body never lets one)
cat > "$p/CHANGELOG.md" <<'EOF'
# Changelog

## Unreleased
- not yet released

## v3.0.0 - 2026-11-04
### Upgrading
- docs/policy.md now says v3: read it again
- odd text: $(touch pwned) `touch pwned` "quoted" \n

## 2.1.0
- policy v2.1

## v2.0.0
- policy v2
EOF
echo "policy v3" > "$p/docs/policy.md" && git_ -C "$p" add -A && git_ -C "$p" commit -qm v3 && git_ -C "$p" tag v3.0.0
expect "outdated: newest within the major, and the new major beside it" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "provider +scions +v2.0.0 +v2.1.0 +outdated +v3.0.0"'
expect "outdated says what MAJOR means" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -q "^MAJOR: "'
expect "outdated --json: a major field" sh -c 'nix run .#graft-outdated -- --json 2>/dev/null | jq -e ".[] | select(.name==\"provider\") | .latest==\"v2.1.0\" and .major==\"v3.0.0\"" >/dev/null'
expect "outdated --json: major is null when there's none" sh -c 'nix run .#graft-outdated -- --json 2>/dev/null | jq -e ".[] | select(.name==\"graft\") | .major==null" >/dev/null'
out=$(nix run .#graft-upgrade 2>/dev/null) || fail "graft-upgrade (within the major)"
git_ add -A
expect "upgrade stays within the pinned major" grep -q 'ref=v2.1.0' flake.nix
expect "and says the new major wasn't taken" grep -q "provider: v3.0.0 is a new major version; not taken (graft-upgrade provider@v3.0.0)" <<<"$out"
expect "check passes" quiet nix flake check
expect "a named scion without a tag stays within its major too" sh -c 'nix run .#graft-upgrade -- provider 2>/dev/null | grep -q "nothing to upgrade"'
quiet nix run .#graft-upgrade -- provider@v3.0.0 || fail "graft-upgrade provider@v3.0.0"
git_ add -A
expect "NAME@TAG crosses a major" grep -q 'ref=v3.0.0' flake.nix
expect "and grafts it" grep -q "policy v3" scions/provider/docs/policy.md
expect "check passes after crossing" quiet nix flake check
expect "outdated: current, no newer major" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "provider +scions +v3.0.0 +v3.0.0 +current +-"'
quiet nix run .#graft-upgrade -- provider@v2.0.0 && git_ add -A

# 0.x: a 0.y series is its own major (semver: anything may change before 1.0)
z=$work/zero
mkdir -p "$z" && echo "zero" > "$z/z.md"
git_ -C "$z" init -q -b main && git_ -C "$z" add -A && git_ -C "$z" commit -qm z
for t in v0.3.0 v0.3.1 v0.4.0 v1.0.0; do git_ -C "$z" commit -q --allow-empty -m "$t" && git_ -C "$z" tag "$t"; done
zc=$work/zero-consumer
mkdir -p "$zc"
cat > "$zc/flake.nix" <<EOF
{
  inputs.graft.url = "path:$graft";
  inputs.zero = { url = "git+file://$z?ref=v0.3.0&exportIgnore=1"; flake = false; };
  outputs = inputs: inputs.graft.lib.mkGraft { inherit inputs; deps = [ "zero" ]; };
}
EOF
git_ -C "$zc" init -q -b main && git_ -C "$zc" add -A
cd "$zc" && quiet nix run .#graft && git_ add -A
expect "0.x: 0.3.0's newest is 0.3.1; 1.0.0 is the next major" sh -c 'nix run .#graft-outdated 2>/dev/null | grep -qE "zero +scions +v0.3.0 +v0.3.1 +outdated +v1.0.0"'
quiet nix run .#graft-upgrade && git_ add -A
expect "0.x: upgrade takes 0.3.1, not 0.4.0" grep -q 'ref=v0.3.1' flake.nix
cd "$c"

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
git_ -C "$g" tag v10.0.0   # a new major: never taken unless named
# v9.0.1: a graft without lib/workflows.sh, like 2.2, which graft-upgrade won't move graft to
git_ -C "$g" checkout -q -b old v9.0.0 && git_ -C "$g" rm -q lib/workflows.sh && git_ -C "$g" commit -qm v9.0.1 \
  && git_ -C "$g" tag v9.0.1 && git_ -C "$g" checkout -q main
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
refuse "upgrade won't move graft to a version without workflows.sh" quiet nix run .#graft-upgrade -- graft@v9.0.1
expect "and changes nothing" git_ diff --quiet
CI=true quiet nix run .#graft-upgrade || fail "graft-upgrade under CI"
expect "a CI job other than graft-ci doesn't move graft" grep -q "ref=v9.0.0" flake.nix
git_ checkout -q -- . && git_ clean -qfd
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
pr_env=(API=http://127.0.0.1:9 WEB=x BASE=main BRANCH=graft/upgrade PR_TOKEN=x)
expect "ci pr: no PR when the branch is already in the base" sh -c "env ${pr_env[*]} nix run .#graft-ci -- pr 2>/dev/null | grep -q 'already in main'"
refuse "ci pr: refuses without a token" quiet env "${pr_env[@]}" PR_TOKEN= nix run .#graft-ci -- pr
git_ reset -q --hard HEAD~1   # back to the default branch before the upgrade
refuse "ci pr: no PR for a branch that changes workflows" quiet env "${pr_env[@]}" nix run .#graft-ci -- pr
expect "and it says why" sh -c "env ${pr_env[*]} nix run .#graft-ci -- pr 2>/dev/null | grep -q 'changes CI workflows'"
git_ checkout -q -b elsewhere "$(git_ -C "$work/origin.git" rev-parse graft/upgrade)"
echo "sneaky" > sneaky.sh && git_ add -A && git_ commit -qm sneaky && git_ push -q --force origin HEAD:graft/upgrade
git_ checkout -q main
refuse "ci pr: no PR for a branch that changes other files" quiet env "${pr_env[@]}" nix run .#graft-ci -- pr

# graft-ci pr against a stub API. stub_pr NAME [LISTING]: run `graft-ci pr` in the current repo
# against a fresh stub (GET answers LISTING), with output in $work/NAME.out, the request log in
# $work/NAME.auth and the requests in $work/NAME.req
stub_pr() {
  local port=$work/$1.port
  rm -f "$port"
  nix run nixpkgs#python3 -- "$graft/tests/stub-api.py" "$port" "$work/$1.auth" "$work/$1.req" "${2:-/nonexistent}" & stub=$!
  for _ in $(seq 100); do [ -s "$port" ] && break; sleep 0.1; done
  env API="http://127.0.0.1:$(cat "$port")" WEB=x BASE=main BRANCH=graft/upgrade PR_TOKEN=s3cret \
    nix run .#graft-ci -- pr > "$work/$1.out" 2>&1 || true
  kill "$stub" 2>/dev/null || true
}
# post_body NAME: the body of the PR the stub was asked to open
post_body() { jq -r 'select(.method=="POST") | .body.body' "$work/$1.req"; }

# a branch that only moves a scion gets its PR, token in a header; the provider has no CHANGELOG.md
# at the new tag, so the body lists its commits
selfc self-pr v9.1.0
git_ commit -qm init
git_ init -q --bare "$work/origin-pr.git" && git_ remote add origin "$work/origin-pr.git" && git_ push -q origin main
BRANCH=graft/upgrade quiet nix run .#graft-ci -- upgrade || fail "graft-ci upgrade (scion only)"
git_ reset -q --hard origin/main
stub_pr pr1
expect "ci pr: opens the PR for a scion-only branch" grep -q "http://stub/pulls/1" "$work/pr1.out"
expect "with the token in the Authorization header" grep -qx "POST token s3cret" "$work/pr1.auth"
body=$(post_body pr1)
expect "titled graft: upgrade" quiet jq -e 'select(.method=="POST") | .body.title == "graft: upgrade"' "$work/pr1.req"
expect "the body lists the move" grep -q "^- provider v2.0.0 -> v2.1.0" <<<"$body"
expect "and the new majors not taken" grep -q "^- provider: v3.0.0 is a new major version" <<<"$body"
expect "and that the whole check passed" grep -q "whole \`nix flake check\` passed" <<<"$body"
expect "no CHANGELOG.md: the provider's commits instead" grep -q "^- [0-9a-f]* v2.1$" <<<"$body"
refuse "and only those in the range" grep -q "^- [0-9a-f]* v2$" <<<"$body"

# a branch crossing to v3.0.0, whose CHANGELOG.md has sections for it: those between the tags
quiet nix run .#graft-upgrade -- provider@v3.0.0 && git_ add -A
git_ commit -qm "graft: upgrade" -m "graft: provider v2.0.0 -> v3.0.0" && git_ push -q --force origin HEAD:graft/upgrade
git_ reset -q --hard origin/main
stub_pr pr2
body=$(post_body pr2)
expect "changelog: the sections between the tags, headings moved down" grep -q "^#### v3.0.0 - 2026-11-04" <<<"$body"
expect "changelog: subheadings too" grep -q "^##### Upgrading" <<<"$body"
expect "changelog: a heading without the v" grep -q "^#### 2.1.0" <<<"$body"
refuse "changelog: not the old tag's own section" grep -q "^#### v2.0.0" <<<"$body"
refuse "changelog: not Unreleased" grep -q "not yet released" <<<"$body"
# shellcheck disable=SC2016 # the literal text
expect "changelog text arrives as text" grep -qF 'odd text: $(touch pwned) `touch pwned` "quoted" \n' <<<"$body"
expect "and nothing in it ran" test ! -e pwned
refuse "changelog: no commit list when there's a CHANGELOG section" grep -q "its commits:" <<<"$body"

# graft-ci upgrade runs the repo's whole flake check: here a check of its own (a link into scions)
# that the upgrade breaks. The branch is still pushed, the step fails, and the PR says why.
gc=$work/gate
mkdir -p "$gc/.forgejo/workflows"
cat > "$gc/flake.nix" <<EOF
{
  inputs.graft.url = "path:$graft";
  inputs.provider = { url = "git+file://$p?ref=v2.0.0&exportIgnore=1"; flake = false; };
  outputs = inputs:
    let g = inputs.graft.lib.mkGraft { inherit inputs; deps = [ "provider" ]; };
    in g // {
      checks = builtins.mapAttrs (system: checks: checks // {
        policy-link = inputs.graft.inputs.nixpkgs.legacyPackages.\${system}.runCommand "policy-link" { } ''
          grep -qx "policy v2" \${inputs.self}/scions/provider/docs/policy.md || { echo "policy-link: expected policy v2" >&2; exit 1; }
          touch \$out
        '';
        fine = inputs.graft.inputs.nixpkgs.legacyPackages.\${system}.runCommand "fine" { } "touch \$out";
      }) g.checks;
    };
}
EOF
git_ -C "$gc" init -q -b main && git_ -C "$gc" add -A
cd "$gc" && quiet nix run .#graft && git_ add -A
expect "gate: the repo's own check passes before the upgrade" quiet nix flake check
git_ commit -qm init
git_ init -q --bare "$work/origin-gate.git" && git_ remote add origin "$work/origin-gate.git" && git_ push -q origin main
: > "$work/gate-out"
refuse "gate: ci upgrade fails when the repo's own check fails" quiet env GITHUB_OUTPUT="$work/gate-out" BRANCH=graft/upgrade nix run .#graft-ci -- upgrade
expect "but says it changed" grep -qx changed=true "$work/gate-out"
expect "and pushed the branch" quiet git_ -C "$work/origin-gate.git" rev-parse --verify refs/heads/graft/upgrade
body=$(git_ -C "$work/origin-gate.git" log -1 --format=%b graft/upgrade)
expect "its commit names the failing check, only" grep -qx "nix flake check failed: policy-link" <<<"$body"
expect "with its log's tail" grep -q "policy-link: expected policy v2" <<<"$body"
git_ reset -q --hard origin/main
stub_pr gate1
expect "gate: the PR is opened" grep -q "http://stub/pulls/1" "$work/gate1.out"
expect "titled as failing" quiet jq -e 'select(.method=="POST") | .body.title == "[checks failing] graft: upgrade"' "$work/gate1.req"
body=$(post_body gate1)
expect "its body says the check fails" grep -q "fails on this branch" <<<"$body"
expect "and which, in a section of its own" grep -qz $'## Failing checks\n\n    nix flake check failed: policy-link' <<<"$body"
expect "and still says what changed upstream" grep -q "^### provider v2.0.0 -> v2.1.0" <<<"$body"
echo '[{"number": 7, "head": {"ref": "graft/upgrade"}}]' > "$work/listing.json"
stub_pr gate2 "$work/listing.json"
expect "an open PR: retitled and described, not opened again" sh -c "jq -e 'select(.method==\"PATCH\") | .path == \"/pulls/7\" and .body.title == \"[checks failing] graft: upgrade\"' '$work/gate2.req' >/dev/null && ! grep -q '^POST' '$work/gate2.auth'"

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
