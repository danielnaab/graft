#!/usr/bin/env bash
# ci.sh check|upgrade|pr: what graft's Forgejo workflows (template/.forgejo/workflows/) run, from the
# repo's root, as `nix run .#graft ci <step>` (or `nix run 'path:DIR#graft' ci <step>` when the flake
# is in DIR, beside the scions directory; the workflows find it by graft's README). It lives here,
# not in the workflow files, so a repo's workflows run the graft version it pins.
#
#   check    flake.lock is committed (in HEAD) and matches flake.nix; the committed state of the
#            scions (`graft check --committed`: registered, absolute URLs, inputs.self.submodules,
#            each gitlink a release tag upstream; graft's workflows current); then the flake
#            evaluates (`nix flake check --no-build`), so mkFlake's errors stop it.
#   upgrade  graft upgrade; if that changed anything, commit it, then run the repo's whole `nix flake
#            check` on the committed tree (a git flake then fetches each scion at its new gitlink),
#            and force-push it to $BRANCH on origin. Writes changed=true|false to $GITHUB_OUTPUT. A
#            failing check doesn't stop the push: the commit message is amended with the failing
#            checks and the tail of each log, so the PR shows the breakage, and the step then
#            fails. Refuses a working tree with changes of its own (but a re-locked flake.lock),
#            and a branch other than $BRANCH checked out (a detached HEAD is fine). The check runs
#            the code of the versions just moved to before anyone reviewed them, so this step
#            must hold no secret beyond the job's own token (and a read-only key for private scions).
#   pr       open a PR from $BRANCH into $BASE, or retitle the open one (the push updated it).
#            Needs API, WEB, BASE, BRANCH and PR_TOKEN. Run it in a job of its own, checked out at
#            the default branch: then it runs the graft that branch pins and never evaluates the
#            upgraded flake. Opening a PR runs the repo's pull_request workflows from the PR's head
#            with the repo's secrets, and the branch was written by code nobody has reviewed yet: so
#            it opens one only when the branch changes nothing but the flake's flake.nix and
#            flake.lock and the gitlinks of scions the default branch already has, and (only when
#            graft's own pin moved) the scions README. A workflow change, or anything else,
#            fails with a link to the diff. The PR body says what changed upstream: for graft and
#            each moved scion, the provider's CHANGELOG.md sections between the two tags (or its
#            git log), fetched with git from the upstream the DEFAULT branch names (flake.lock,
#            .gitmodules). Both are read as data; nothing fetched is run.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source-path=SCRIPTDIR source=common.sh
. "$here/common.sh"
output() { if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "$1" >> "$GITHUB_OUTPUT"; fi; }
locate ""
[ -f "$FLAKE_NIX" ] || die "run from the repo root (no $FLAKE_NIX here)"

check() {
  # In a fresh checkout, Nix writes a lock that was never committed, and git-adds it: the checks
  # would pass on whatever the tags resolve to today. The lock is the pins: it must be committed.
  git cat-file -e "HEAD:$FLAKE_LOCK" 2>/dev/null \
    || die "$FLAKE_LOCK is not committed, so nothing is pinned: run $GRAFT_RUN, then commit $FLAKE_LOCK (if git ignores it, git check-ignore -v $FLAKE_LOCK names the rule)"
  # A lock that doesn't cover flake.nix (an input added, its url changed, without re-locking): Nix
  # re-locked it while evaluating graft, before this ran, so the committed one pins nothing for it
  if ! git diff --quiet HEAD -- "$FLAKE_LOCK"; then
    git --no-pager diff --stat HEAD -- "$FLAKE_LOCK" >&2 || true
    die "$FLAKE_LOCK in HEAD does not match $FLAKE_NIX: Nix re-locked it to evaluate graft (git diff -- $FLAKE_LOCK shows how). Run $GRAFT_RUN, then commit $FLAKE_LOCK"
  fi
  bash "$here/scions.sh" check --committed
  nix_ flake check --no-build "$FLAKE_REF" || die "the flake doesn't evaluate (above)"
  echo "graft: ci check passed: $FLAKE_LOCK is committed, every scion is at a release tag, graft's workflows are current, and the flake evaluates"
}

upgrade() {
  local changes checks=passed cur dirty
  [ -n "${BRANCH:-}" ] || die "BRANCH is not set: name the branch to push the upgrade to (graft's workflow sets BRANCH=graft/upgrade)"
  # It commits on the branch checked out and stages everything: never the default branch, and
  # nothing of anyone's but graft's (untracked files would be pushed with it)
  cur=$(git symbolic-ref -q --short HEAD) || cur=""
  if [ -n "$cur" ] && [ "$cur" != "$BRANCH" ]; then
    die "ci upgrade commits on the branch checked out, $cur, and pushes it to $BRANCH: switch to a new branch $BRANCH first (git switch -c $BRANCH)"
  fi
  # Only the lock may differ: Nix re-locks (and git-adds) a lock that doesn't cover flake.nix while
  # evaluating graft, before this runs; the upgrade rewrites the lock anyway, and commits that too
  dirty=$(git status --porcelain --untracked-files=all | cut -c4-)
  if [ "$dirty" = "$FLAKE_LOCK" ]; then
    echo "graft: $FLAKE_LOCK differs from HEAD (Nix re-locks a lock that doesn't cover $FLAKE_NIX): the upgrade commit includes it"
  elif [ -n "$dirty" ]; then
    git status --short --untracked-files=all >&2
    die "ci upgrade commits every change in the working tree, and this one has the changes of its own above: commit, stash or remove them first"
  fi
  log=$(mktemp -p "$GRAFT_TMP") report=$(mktemp -p "$GRAFT_TMP")   # outside the checkout: git add can't take them
  GRAFT_CI=1 bash "$here/tools.sh" upgrade | tee "$log"
  # a new major alone moves nothing and opens no PR: say so where the run's summary shows it
  grep -E '^graft: .* is a new major version' "$log" | sed 's/^graft: /::warning::/' || true
  git add -A
  if git diff --cached --quiet; then output changed=false; return 0; fi
  changes=$(grep -E '^graft: (.* -> |refreshed |wrote |.* is a new major version)' "$log" || true)
  # committed first: a git flake reads the scions at the gitlinks HEAD records only while the tree
  # is clean (a staged gitlink alone is still the old version to Nix)
  git -c user.name="graft-upgrade" -c user.email="graft-upgrade@noreply.invalid" \
    commit -q -m "graft: upgrade" -m "$changes"
  full_check "$report" || checks=failed
  if [ "$checks" = failed ]; then
    git -c user.name="graft-upgrade" -c user.email="graft-upgrade@noreply.invalid" \
      commit -q --amend -m "graft: upgrade" -m "$changes"$'\n\n'"$(<"$report")"
  fi
  git push -q --force origin "HEAD:refs/heads/$BRANCH"
  output changed=true
  if [ "$checks" = failed ]; then
    echo "::error::$(head -1 "$report"). $BRANCH is pushed, and its PR will say so."
    exit 1
  fi
}

# pr_notes BASE_LOCK HEAD_LOCK: markdown, what changed upstream for each input whose locked commit
# differs (common.sh's upstream_input, which fetches and reads it as data), as graft changelog and
# graft migrate print it (upstream.jq's upstream_notes: Upgrading sections first, provider text
# cleaned and block-quoted, so a code fence it leaves open ends with the quote).
pr_notes() {
  local base=$1 head=$2 commit=$3 name path url ov nv ot nt j n=0
  notes() { jq -r -L "$GRAFT_LIB" --arg base "${BASE:-the default branch}" 'include "upstream"; upstream_notes($base)' <<<"$1"; }
  j=$(upstream_input "$base" "$head" graft) || return 1
  if [ -n "$j" ]; then n=$((n + 1)); notes "$j"; fi
  # the scions the default branch has (its .gitmodules: reviewed), moved by the branch
  while IFS=$'\t' read -r name path url; do
    [ -n "$name" ] || continue
    ov=$(gitlink "$path" HEAD); nv=$(gitlink "$path" "$commit")
    if [ -z "$ov" ] || [ -z "$nv" ] || [ "$ov" = "$nv" ]; then continue; fi
    ot=$(version_at "$url" "$ov" 2>/dev/null) || ot=""
    nt=$(version_at "$url" "$nv" 2>/dev/null) || nt=""
    j=$(upstream_range "$name" "$(scion_node "$url" "$ot")" "$ov" "$nv" "$ot" "$nt") || return 1
    n=$((n + 1)); notes "$j"
  done < <(scions_registered "$GRAFT_TMP/base.gitmodules")
  [ "$n" -gt 0 ] || printf 'Nothing moved commits.\n'
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
  local v head changes out code number line path d ok workflows="" other="" compare meta om nm base_lock head_lock graft_moved notes body req
  local moved="" held="" failing="" title
  for v in API WEB BASE BRANCH PR_TOKEN; do [ -n "${!v:-}" ] || die "$v is not set"; done
  git fetch -q --no-tags origin "refs/heads/$BRANCH" || die "can't fetch $BRANCH"
  head=$(git rev-parse FETCH_HEAD)
  if git merge-base --is-ancestor "$head" HEAD; then
    echo "graft: $BRANCH is already in $BASE; no PR to open"; return 0
  fi
  changes=$(git log -1 --format=%b "$head")
  compare="$WEB/compare/$BASE...$BRANCH"

  # What the branch may change without review: flake.nix and flake.lock, the gitlinks of the scions
  # the default branch has (a moved submodule stays a submodule), and, when graft's own pin moved,
  # the scions README (the new version's text). A new version's workflows are never opened
  # unreviewed: they would run with the repo's secrets.
  git show HEAD:.gitmodules > "$GRAFT_TMP/base.gitmodules" 2>/dev/null || : > "$GRAFT_TMP/base.gitmodules"
  base_lock=$GRAFT_TMP/base.lock head_lock=$GRAFT_TMP/head.lock
  git show "HEAD:$FLAKE_LOCK" > "$base_lock"
  git show "$head:$FLAKE_LOCK" > "$head_lock" 2>/dev/null || echo '{"nodes":{"root":{"inputs":{}}}}' > "$head_lock"
  graft_moved=""
  if [ "$(jq -r '.locked.rev // .locked.narHash // ""' <<<"$(lock_node "$base_lock" graft)")" \
       != "$(jq -r '.locked.rev // .locked.narHash // ""' <<<"$(lock_node "$head_lock" graft)")" ]; then
    graft_moved=1
  fi
  declare -A scion=()
  while IFS=$'\t' read -r _ d _; do [ -z "$d" ] || scion[$d]=1; done < <(scions_registered "$GRAFT_TMP/base.gitmodules")
  while IFS= read -r -d '' meta && IFS= read -r -d '' path; do
    ok=""
    read -r om nm _ _ _ <<<"${meta#:}"
    if [ "$path" = "$FLAKE_NIX" ] || [ "$path" = "$FLAKE_LOCK" ]; then ok=1; fi
    if [ -n "${scion[$path]:-}" ] && [ "$om" = 160000 ] && [ "$nm" = 160000 ]; then ok=1; fi
    if [ -n "$graft_moved" ] && [ "$path" = "$SCIONS/README.md" ]; then ok=1; fi
    [ -z "$ok" ] || continue
    if [[ $path == .forgejo/* ]]; then workflows+=" $path"; else other+=" $path"; fi
  done < <(git diff --raw --no-abbrev --no-renames -z HEAD "$head")
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

  # What changed upstream, computed here (no PR_TOKEN in git's environment).
  notes=$GRAFT_TMP/notes body=$GRAFT_TMP/body req=$GRAFT_TMP/req
  # errexit is off in here (a provider that can't be fetched is reported, not fatal); a list cut
  # short is kept, with a note
  ( unset PR_TOKEN; pr_notes "$base_lock" "$head_lock" "$head" ) > "$notes" \
    || echo "(stopped early; the rest is in the diff of flake.lock)" >> "$notes"

  title="graft: upgrade"
  # shellcheck disable=SC2016 # backticks here are markdown
  {
    if [ -n "$failing" ]; then
      title="[checks failing] graft: upgrade"
      printf '> [!WARNING]\n> **`nix flake check` fails on this branch.** The upgrade is pushed so the breakage can be read. Fix your own files on a branch made from this one: the next run force-pushes `%s`. If the provider broke something, tell its maintainers.\n\n' "$BRANCH"
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
    printf 'The moved submodule commits are the change you are taking; review and merge are yours.\n\n'
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
