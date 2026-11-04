#!/usr/bin/env bash
# ci.sh check|upgrade|pr: what graft's Forgejo workflows (examples/forgejo/) run, from the repo's
# root, as `nix run .#graft-ci -- <step>`. It lives here, not in the workflow files, so a repo's
# workflows run the graft version it pins: when graft-upgrade moves that pin, their behaviour
# moves with it, and the YAML itself rarely changes.
#
#   check    the scions directory and graft's workflows match flake.lock (checks.<system>.graft).
#   upgrade  graft-upgrade; if that changed anything, check the result, commit it, and force-push
#            it to $BRANCH on origin. Writes changed=true|false to $GITHUB_OUTPUT. The check runs
#            the code of the graft version just moved to, before anyone reviewed it, so this step
#            must hold no secret beyond the job's own token.
#   pr       open a PR from $BRANCH into $BASE, unless one is open already (the push updated it).
#            Needs API, WEB, BASE, BRANCH and PR_TOKEN. Run it in a job of its own, checked out at
#            the default branch: then it runs the graft that branch pins and never evaluates the
#            upgraded flake, so a new graft version never sees PR_TOKEN. Opening a PR runs the
#            repo's pull_request workflows from the PR's head, with the repo's secrets, and the
#            branch was written by code nobody has reviewed yet: so it opens one only when the
#            branch changes nothing but flake.nix, flake.lock and the scions directories. A change
#            to .forgejo/ (a new graft version's workflows) or anywhere else fails with a link to
#            the diff: review it, then open the PR yourself. The limit: flake.nix and flake.lock
#            are code (a moved graft pin is the new version), so any OTHER pull_request workflow
#            of the repo's that evaluates or builds the flake with secrets runs it unreviewed.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
nix_() { nix --extra-experimental-features 'nix-command flakes' "$@"; }
die() { echo "graft: $*" >&2; exit 1; }
output() { if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "$1" >> "$GITHUB_OUTPUT"; fi; }
[ -f flake.nix ] || die "run from the repo root (no flake.nix here)"

check() {
  local system
  system=$(nix_ eval --impure --raw --expr builtins.currentSystem)
  nix_ build --no-link ".#checks.$system.graft"
}

upgrade() {
  local changes
  : "${BRANCH:?BRANCH is not set}"
  log=$(mktemp)   # outside the checkout: the repo's .gitignore can't refuse it, git add can't take it
  trap 'rm -f "$log"' EXIT
  GRAFT_CI=1 bash "$here/tools.sh" upgrade | tee "$log"
  git add -A
  if git diff --cached --quiet; then output changed=false; return 0; fi
  check
  changes=$(grep -E '^graft: (.* -> |refreshed )' "$log" || true)
  git -c user.name="graft-upgrade" -c user.email="graft-upgrade@noreply.invalid" \
    commit -q -m "graft: upgrade" -m "$changes"
  git push -q --force origin "HEAD:refs/heads/$BRANCH"
  output changed=true
}

pr() {
  local v head changes out code list line body jq_ path dirs=() d ok workflows="" other="" compare
  for v in API WEB BASE BRANCH PR_TOKEN; do [ -n "${!v:-}" ] || die "$v is not set"; done
  git fetch -q --no-tags origin "refs/heads/$BRANCH" || die "can't fetch $BRANCH"
  head=$(git rev-parse FETCH_HEAD)
  if git merge-base --is-ancestor "$head" HEAD; then
    echo "graft: $BRANCH is already in $BASE; no PR to open"; return 0
  fi
  changes=$(git log -1 --format=%b "$head")
  compare="$WEB/compare/$BASE...$BRANCH"

  # What the branch may change without review: the lock, its refs, and graft's scions directories
  # as the default branch has them (a README.md carrying graft's marker).
  while IFS= read -r path; do
    path=${path#HEAD:}; path=${path%README.md}; dirs+=("${path%/}")
  done < <(git grep -l -F -e '<!-- managed by graft: https://github.com/danielnaab/graft -->' \
             -e 'Managed by [graft]' HEAD -- README.md '*/README.md' || true)
  while IFS= read -r -d '' path; do
    ok=""
    case $path in flake.nix|flake.lock) ok=1 ;; esac
    for d in "${dirs[@]}"; do [ -z "$d" ] || [[ $path != "$d"/* ]] || ok=1; done
    [ -z "$ok" ] || continue
    if [[ $path == .forgejo/* ]]; then workflows+=" $path"; else other+=" $path"; fi
  done < <(git diff --no-renames --name-only -z HEAD "$head")
  if [ -n "$other" ]; then
    echo "::error::$BRANCH changes files graft-upgrade never writes:$other. Not opening a PR; look at $compare (if $BASE moved since the upgrade job, re-run the workflow)"
    exit 1
  fi
  if [ -n "$workflows" ]; then
    echo "::error::this upgrade changes CI workflows:$workflows. A PR would run them with the repo's secrets before anyone reads them, so none is opened: review the diff at $compare, then open the PR yourself."
    exit 1
  fi

  # The token reaches curl as a header file on a pipe, never in a command line.
  api() { curl -sS -w '\n%{http_code}' -H @<(printf 'Authorization: token %s\n' "$PR_TOKEN") \
            -H "Content-Type: application/json" "$@"; }
  out=$(api "$API/pulls?state=open&limit=50"); code=${out##*$'\n'}; out=${out%$'\n'*}
  [ "$code" = 200 ] || { echo "::error::listing PRs: HTTP $code"; exit 1; }
  if [ "$(jq --arg b "$BRANCH" '[.[] | select(.head.ref == $b)] | length' <<<"$out")" != 0 ]; then
    echo "graft: a PR is already open for $BRANCH; the push updated it"; return 0
  fi
  list=""
  while IFS= read -r line; do [ -z "$line" ] || list+="- ${line#graft: }"$'\n'; done <<<"$changes"
  body="Moved by \`nix run .#graft-upgrade\` (graft's own pin, and scions behind their upstream's newest tag):"$'\n\n'"$list"$'\n'
  body+="graft's check (\`checks.<system>.graft\`) passed on the result: when graft itself moved, that is the new version's own check. The diff of the scions directory is the change you are taking; review and merge are yours."
  jq_=$(jq -n --arg head "$BRANCH" --arg base "$BASE" --arg body "$body" \
          '{head: $head, base: $base, title: "graft: upgrade", body: $body}')
  out=$(api -X POST -d @- "$API/pulls" <<<"$jq_")
  code=${out##*$'\n'}; out=${out%$'\n'*}
  if [ "$code" = 201 ]; then jq -r .html_url <<<"$out"; return 0; fi
  # 409: a PR for this branch is open already (one the listing above didn't reach)
  if [ "$code" = 409 ]; then echo "graft: a PR is already open for $BRANCH; the push updated it"; return 0; fi
  echo "::error::opening the PR: HTTP $code. $BRANCH is pushed; open it at $compare"
  [ "$code" != 404 ] || echo "::error::a 404 here is Forgejo refusing the job token: set the GRAFT_PR_TOKEN secret"
  exit 1
}

case ${1:-} in
  check) check ;;
  upgrade) upgrade ;;
  pr) pr ;;
  *) die "usage: graft-ci check|upgrade|pr" ;;
esac
