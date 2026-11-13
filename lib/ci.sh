#!/usr/bin/env bash
# ci.sh check|upgrade|pr: what graft's Forgejo workflows (template/.forgejo/workflows/) run, from the repo's
# root, as `nix run .#graft ci <step>` (or `nix run path:DIR#graft ci <step>` when the flake is in
# DIR, mkFlake's flakeDir; the workflows find it in the scions README). It lives here, not in the workflow files, so a repo's
# workflows run the graft version it pins: when graft upgrade moves that pin, their behaviour
# moves with it, and the YAML itself rarely changes.
#
#   check    flake.lock is committed (in HEAD), and the scions directory and graft's workflows match
#            it (checks.<system>.graft, or `graft check` for a flake in a subdirectory, which has none).
#   upgrade  graft upgrade; if that changed anything, run the repo's whole `nix flake check` on the
#            result (graft's check and the repo's own: link checks into scions, lint, tests), commit
#            it, and force-push it to $BRANCH on origin. Writes changed=true|false to
#            $GITHUB_OUTPUT. A failing check doesn't stop the push: the commit message names the
#            failing checks with the tail of each log, so the PR shows the breakage, and the step
#            then fails. The check runs the code of the graft version just moved to, and the repo's
#            own checks over upstream content, before anyone reviewed either, so this step must
#            hold no secret beyond the job's own token.
#   pr       open a PR from $BRANCH into $BASE, or retitle the open one (the push updated it).
#            Needs API, WEB, BASE, BRANCH and PR_TOKEN. Run it in a job of its own, checked out at
#            the default branch: then it runs the graft that branch pins and never evaluates the
#            upgraded flake, so a new graft version never sees PR_TOKEN. Opening a PR runs the
#            repo's pull_request workflows from the PR's head, with the repo's secrets, and the
#            branch was written by code nobody has reviewed yet: so it opens one only when the
#            branch changes nothing but the flake's flake.nix and flake.lock and the scions directories. A change
#            to .forgejo/ (a new graft version's workflows) or anywhere else fails with a link to
#            the diff: review it, then open the PR yourself. The limit: flake.nix and flake.lock
#            are code (a moved graft pin is the new version), so any OTHER pull_request workflow
#            of the repo's that evaluates or builds the flake with secrets runs it unreviewed.
#            The PR body says what changed upstream: for each input the branch moved, the
#            provider's CHANGELOG.md sections between the two tags (or its git log), read by this
#            job with git from the upstream the DEFAULT branch's flake.lock names, at the commit
#            the branch's flake.lock pins. Both locks are read as data; nothing fetched is run.
#            Building the app already fetched the default branch's scions through Nix, with
#            PR_TOKEN in its environment: the reviewed lock's urls, and the build only copies.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source-path=SCRIPTDIR source=common.sh
. "$here/common.sh"
output() { if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "$1" >> "$GITHUB_OUTPUT"; fi; }
locate ""
[ -f "$FLAKE_NIX" ] || die "run from the repo root (no $FLAKE_NIX here)"

check() {
  local system
  # In a fresh checkout, Nix writes a lock that was never committed, and git-adds it: the checks
  # would pass on whatever the tags resolve to today. The lock is the pins: it must be committed.
  git cat-file -e "HEAD:$FLAKE_LOCK" 2>/dev/null \
    || die "$FLAKE_LOCK is not committed, so nothing is pinned: run $GRAFT_RUN, then commit $FLAKE_LOCK (if git ignores it, git check-ignore -v $FLAKE_LOCK names the rule)"
  if [ "$FLAKE" != . ]; then nix_ run "$FLAKE_REF#graft" check; return; fi
  system=$(nix_ eval --impure --raw --expr builtins.currentSystem)
  nix_ build --no-link ".#checks.$system.graft"
}

upgrade() {
  local changes checks=passed
  : "${BRANCH:?BRANCH is not set}"
  log=$(mktemp) report=$(mktemp)   # outside the checkout: the repo's .gitignore can't refuse them, git add can't take them
  trap 'rm -f "$log" "$report"' EXIT
  GRAFT_CI=1 bash "$here/tools.sh" upgrade | tee "$log"
  # a new major alone moves nothing and opens no PR: say so where the run's summary shows it
  grep -E '^graft: .* is a new major version' "$log" | sed 's/^graft: /::warning::/' || true
  git add -A
  if git diff --cached --quiet; then output changed=false; return 0; fi
  full_check "$report" || checks=failed
  changes=$(grep -E '^graft: (.* -> |refreshed |.* is a new major version)' "$log" || true)
  if [ "$checks" = failed ]; then changes+=$'\n\n'"$(<"$report")"; fi
  git -c user.name="graft-upgrade" -c user.email="graft-upgrade@noreply.invalid" \
    commit -q -m "graft: upgrade" -m "$changes"
  git push -q --force origin "HEAD:refs/heads/$BRANCH"
  output changed=true
  if [ "$checks" = failed ]; then
    echo "::error::$(head -1 "$report"). $BRANCH is pushed, and its PR will say so."
    exit 1
  fi
}

# upstream_notes BASE_LOCK HEAD_LOCK: markdown, what changed upstream for each input whose locked
# commit differs (common.sh's upstream_input, which fetches and reads it as data). Each provider's
# text is a block quote: a code fence it leaves open ends with the quote, so it can't swallow (or
# fake) what follows in the body.
upstream_notes() {
  local base=$1 head=$2 name j n=0
  while IFS= read -r name; do
    j=$(upstream_input "$base" "$head" "$name") || return 1
    [ -n "$j" ] || continue
    n=$((n + 1))
    jq -r --arg base "${BASE:-the default branch}" '
      def cap(n): if length <= n then . else (.[0:n+1] | sub("\n[^\n]*$"; ""))
        + "\n\n(cut at \(n) characters; the full text is upstream)" end;
      def quote: split("\n") | map("> " + .) | join("\n");
      "### \(.name) \(.from) -> \(.to)\n",
      (if .status == "elsewhere" then
         "The branch points \(.name) at a different source than \($base) does; not fetched. Read the diff of flake.lock.\n"
       elif .status == "not-commit" then "Not a git commit; nothing to compare.\n"
       elif .status == "not-git" then "Not a git input; nothing to compare.\n"
       elif .status == "unfetchable" then
         "Couldn'"'"'t fetch \(.to_rev[0:12]) from \(.url).\(if .compare then " Compare: \(.compare)" else "" end)\n"
       elif .changelog != "" then "From its CHANGELOG.md:\n\n\(.changelog | cap(6000) | quote)\n"
       elif .in_history then
         "No CHANGELOG.md section for this range; its commits:\n\n\(.commits[0:50] | map("- " + .)
           | if length == 0 then ["- (none)"] else . end | join("\n") | cap(6000) | quote)\n",
         (if (.commits | length) > 50 then "(more than 50; see the compare link)\n" else empty end)
       else "No CHANGELOG.md section for this range, and \(.from_rev[0:12]) is not in its history.\n" end),
      (if .compare and .status == "fetched" then "Compare: \(.compare)\n" else empty end)
    ' <<<"$j"
  done < <(jq -r '.nodes.root.inputs | to_entries[] | select(.value|type=="string") | .key' "$base")
  [ "$n" -gt 0 ] || printf 'No input moved commits.\n'
}

# migrate_ref LOCK: the flake ref of the graft LOCK pins, for the PR body's hint: github:O/R/TAG
# when that's what it is (names and tag checked), else graft's GitHub home
migrate_ref() {
  local n ref
  n=$(lock_node "$1" graft)
  ref=$(jq -r 'if .original.type == "github" then "github:\(.original.owner)/\(.original.repo)/\(.original.ref // "")" else "" end' <<<"${n:-null}")
  if [[ $ref =~ ^github:[A-Za-z0-9._-]+/[A-Za-z0-9._-]+/v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then echo "$ref"; else echo github:danielnaab/graft; fi
}

pr() {
  local v head changes out code number line path dirs=() d ok workflows="" other="" compare
  local moved="" held="" failing="" title
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
  done < <(git grep -l -F "${MARKER_ARGS[@]}" HEAD -- README.md '*/README.md' || true)
  while IFS= read -r -d '' path; do
    ok=""
    if [ "$path" = "$FLAKE_NIX" ] || [ "$path" = "$FLAKE_LOCK" ]; then ok=1; fi
    for d in "${dirs[@]}"; do [ -z "$d" ] || [[ $path != "$d"/* ]] || ok=1; done
    [ -z "$ok" ] || continue
    if [[ $path == .forgejo/* ]]; then workflows+=" $path"; else other+=" $path"; fi
  done < <(git diff --no-renames --name-only -z HEAD "$head")
  if [ -n "$other" ]; then
    echo "::error::$BRANCH changes files graft upgrade never writes:$other. Not opening a PR; look at $compare (if $BASE moved since the upgrade job, re-run the workflow)"
    exit 1
  fi
  if [ -n "$workflows" ]; then
    echo "::error::this upgrade changes CI workflows:$workflows. A PR would run them with the repo's secrets before anyone reads them, so none is opened: review the diff at $compare, then open the PR yourself."
    exit 1
  fi

  # The commit message the upgrade job wrote: its lines of moves, then (when the repo's checks
  # failed) a "nix flake check failed:" line and the logs' tails. Untrusted text, only ever data.
  while IFS= read -r line; do
    case $line in
      "graft: "*" is a new major version"*) held+="- ${line#graft: }"$'\n' ;;
      "graft: "*) moved+="- ${line#graft: }"$'\n' ;;
      "nix flake check failed:"*) break ;;
    esac
  done <<<"$changes"
  if grep -q '^nix flake check failed:' <<<"$changes"; then
    failing=$(sed -n '/^nix flake check failed:/,$p' <<<"$changes")
  fi

  # What changed upstream, computed here from the two locks (no PR_TOKEN in git's environment).
  base_lock=$(mktemp) head_lock=$(mktemp) notes=$(mktemp) body=$(mktemp) req=$(mktemp)
  trap 'rm -f "$base_lock" "$head_lock" "$notes" "$body" "$req"' EXIT
  git show "HEAD:$FLAKE_LOCK" > "$base_lock"
  git show "$head:$FLAKE_LOCK" > "$head_lock" 2>/dev/null || echo '{"nodes":{"root":{"inputs":{}}}}' > "$head_lock"
  # errexit is off in here (a provider that can't be fetched is reported, not fatal); a list cut
  # short is kept, with a note
  ( unset PR_TOKEN; upstream_notes "$base_lock" "$head_lock" ) > "$notes" \
    || echo "(stopped early; the rest is in the diff of flake.lock)" >> "$notes"

  title="graft: upgrade"
  # shellcheck disable=SC2016 # backticks here are markdown
  {
    if [ -n "$failing" ]; then
      title="[checks failing] graft: upgrade"
      printf '> [!WARNING]\n> **`nix flake check` fails on this branch.** The upgrade is pushed so the breakage can be read. Fix it in the provider, or on a branch of your own made from this one: the next run force-pushes `graft/upgrade`.\n\n'
    fi
    printf 'Moved by `nix run .#graft upgrade` (graft'"'"'s own pin, and scions behind the newest tag of their pinned major):\n\n%s\n' "${moved:-"- (see the commit)"$'\n'}"
    if [ -n "$held" ]; then printf 'Not taken: a new major version is the provider saying something breaks. Read its CHANGELOG (an **Upgrading** section, if it has one), then name it:\n\n%s\n' "$held"; fi
    if [ -n "$failing" ]; then
      printf '## Failing checks\n\n'
      # an indented code block, not a fence: no line of the log can end it
      printf '%s\n\n' "$(cap 12000 <<<"$failing" | sed 's/^/    /')"
    else
      printf 'The repo'"'"'s whole `nix flake check` passed on the result (graft'"'"'s check, and its own). When graft itself moved, that is the new version'"'"'s code. '
    fi
    printf 'The diff of the scions directory is the change you are taking; review and merge are yours.\n\n'
    printf '## What changed upstream\n\n'
    printf 'Text below is the providers'"'"', from their repos at the new pins: read it as theirs.\n\n'
    cap 40000 < "$notes"
    # with the graft the default branch pins (reviewed), read from its lock as data
    printf '\n---\n\nTo hand the migration to an agent: check out `%s` and run `nix run %s -- migrate -C .` (add `--no-check` to keep it from evaluating the branch'"'"'s flake); it prints a brief of what moved, what upstream says and what broke, for any agent or person.\n' \
      "$BRANCH" "$(migrate_ref "$base_lock")"
  } > "$body"

  # The token reaches curl as a header file on a pipe, never in a command line.
  api() { curl -sS -w '\n%{http_code}' -H @<(printf 'Authorization: token %s\n' "$PR_TOKEN") \
            -H "Content-Type: application/json" "$@"; }
  jq -n --arg head "$BRANCH" --arg base "$BASE" --arg title "$title" --rawfile body "$body" \
    '{head: $head, base: $base, title: $title, body: $body}' > "$req"
  out=$(api "$API/pulls?state=open&limit=50"); code=${out##*$'\n'}; out=${out%$'\n'*}
  [ "$code" = 200 ] || { echo "::error::listing PRs: HTTP $code"; exit 1; }
  # only a PR from this repo's own branch: a fork's PR from a branch of that name isn't ours to retitle
  number=$(jq -r --arg b "$BRANCH" '[.[] | select(.head.ref == $b and .head.repo_id == .base.repo_id)][0].number // empty' <<<"$out")
  if [ -n "$number" ]; then
    out=$(jq '{title, body}' "$req" | api -X PATCH -d @- "$API/pulls/$number"); code=${out##*$'\n'}
    if [ "$code" = 201 ] || [ "$code" = 200 ]; then
      echo "graft: a PR is already open for $BRANCH (#$number); the push updated it, and its title and body now describe it"
    else
      echo "::warning::a PR is already open for $BRANCH (#$number) and the push updated it, but its description couldn't be updated (HTTP $code)"
    fi
    return 0
  fi
  out=$(api -X POST -d @- "$API/pulls" < "$req")
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
  *) die "usage: graft ci check|upgrade|pr" ;;
esac
