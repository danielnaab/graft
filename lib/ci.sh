#!/usr/bin/env bash
# ci.sh check|upgrade|pr: what graft's Forgejo workflows (examples/forgejo/) run, from the repo's
# root, as `nix run .#graft-ci -- <step>`. It lives here, not in the workflow files, so a repo's
# workflows run the graft version it pins: when graft-upgrade moves that pin, their behaviour
# moves with it, and the YAML itself rarely changes.
#
#   check    the scions directory and graft's workflows match flake.lock (checks.<system>.graft).
#   upgrade  graft-upgrade; if that changed anything, run the repo's whole `nix flake check` on the
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
#            branch changes nothing but flake.nix, flake.lock and the scions directories. A change
#            to .forgejo/ (a new graft version's workflows) or anywhere else fails with a link to
#            the diff: review it, then open the PR yourself. The limit: flake.nix and flake.lock
#            are code (a moved graft pin is the new version), so any OTHER pull_request workflow
#            of the repo's that evaluates or builds the flake with secrets runs it unreviewed.
#            The PR body says what changed upstream: for each input the branch moved, the
#            provider's CHANGELOG.md sections between the two tags (or its git log), read by this
#            job with git from the upstream the DEFAULT branch's flake.lock names, at the commit
#            the branch's flake.lock pins. Both locks are read as data; nothing fetched is run.
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

# The repo's whole check on the working tree: each of checks.<system> built on its own (so a
# failure has a name), then `nix flake check` for the rest (evaluation, other systems' outputs).
# Logs go to the job log as they run. Fails if anything did, after writing to $1 one line
# "nix flake check failed: NAME..." and the last lines of each failing log.
full_check() {
  local report=$1 system names n log failed=() tails=""
  system=$(nix_ eval --impure --raw --expr builtins.currentSystem)
  log=$(mktemp)
  if names=$(nix_ eval --json ".#checks.$system" --apply builtins.attrNames 2>"$log" | jq -r '.[]'); then
    while IFS= read -r n; do
      [ -n "$n" ] || continue
      echo "graft: checks.$system.$n"
      [[ $n =~ ^[A-Za-z0-9_][A-Za-z0-9_\'-]*$ ]] || { echo "graft: (an odd name: left to nix flake check)"; continue; }
      if ! nix_ build --no-link -L ".#checks.$system.$n" 2>&1 | tee "$log"; then
        failed+=("$n"); tails+="$n:"$'\n'"$(tail -n 30 "$log")"$'\n\n'
        echo "::error::check $n failed on the upgraded tree"
      fi
    done <<<"$names"
  else
    cat "$log" >&2
    failed+=("checks"); tails+="checks (evaluating checks.$system):"$'\n'"$(tail -n 30 "$log")"$'\n\n'
  fi
  if ! nix_ flake check -L 2>&1 | tee "$log"; then
    if [ ${#failed[@]} -eq 0 ]; then
      failed+=("flake-check"); tails+="nix flake check:"$'\n'"$(tail -n 30 "$log")"$'\n\n'
    fi
    echo "::error::nix flake check failed on the upgraded tree"
  fi
  rm -f "$log"
  [ ${#failed[@]} -gt 0 ] || return 0
  { echo "nix flake check failed: ${failed[*]}"; echo; printf '%s' "${tails:0:12000}"; } > "$report"
  return 1
}

upgrade() {
  local changes checks=passed
  : "${BRANCH:?BRANCH is not set}"
  log=$(mktemp) report=$(mktemp)   # outside the checkout: the repo's .gitignore can't refuse them, git add can't take them
  trap 'rm -f "$log" "$report"' EXIT
  GRAFT_CI=1 bash "$here/tools.sh" upgrade | tee "$log"
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

# One root input's node in the flake.lock at $1, as JSON, or nothing.
lock_node() { jq -c --arg n "$2" '.nodes.root.inputs[$n] as $k | if ($k|type)=="string" then .nodes[$k] else empty end' "$1"; }

# upstream_notes BASE_LOCK HEAD_LOCK: markdown, what changed upstream for each input whose locked
# commit differs. The upstream is the one BASE_LOCK (the default branch's, reviewed) names; an input
# the branch points somewhere else is listed, not fetched. HEAD_LOCK's revs and refs are untrusted:
# a rev must be a full commit hash and a ref a version tag before git sees it. Fetched text is
# data: it reaches the body through files and jq, never a command line or eval.
upstream_notes() {
  local base=$1 head=$2 name o nw url ov nv ot nt tmp text cmp n=0
  while IFS= read -r name; do
    o=$(lock_node "$base" "$name"); nw=$(lock_node "$head" "$name")
    if [ -z "$o" ] || [ -z "$nw" ]; then continue; fi
    ov=$(jq -r '.locked.rev // empty' <<<"$o"); nv=$(jq -r '.locked.rev // empty' <<<"$nw")
    [ "$ov" != "$nv" ] || continue
    ot=$(jq -r '.original.ref // empty' <<<"$o"); nt=$(jq -r '.original.ref // empty' <<<"$nw")
    is_semver "$ot" || ot=""; is_semver "$nt" || nt=""
    n=$((n + 1))
    printf '### %s %s -> %s\n\n' "$name" "${ot:-${ov:0:12}}" "${nt:-${nv:0:12}}"
    if [ "$(jq -n --argjson a "$o" --argjson b "$nw" '($a.original|del(.ref,.rev)) == ($b.original|del(.ref,.rev))')" != true ]; then
      printf 'The branch points %s at a different source than %s does; not fetched. Read the diff of flake.lock.\n\n' "$name" "${BASE:-the default branch}"
      continue
    fi
    if ! [[ $ov =~ ^[0-9a-f]{40}$ && $nv =~ ^[0-9a-f]{40}$ ]]; then printf 'Not a git commit; nothing to compare.\n\n'; continue; fi
    url=$(jq -r 'if .original.type=="github" then "https://github.com/\(.original.owner)/\(.original.repo).git"
                 elif .original.type=="git" then .original.url else empty end' <<<"$o")
    cmp=$(jq -r --arg a "${ot:-$ov}" --arg b "${nt:-$nv}" \
      'if .original.type=="github" then "https://github.com/\(.original.owner)/\(.original.repo)/compare/\($a)...\($b)" else empty end' <<<"$o")
    [ -n "$url" ] || { printf 'Not a git input; nothing to compare.\n\n'; continue; }
    tmp=$(mktemp -d)
    git init -q --bare "$tmp"
    # by commit (GitHub and Forgejo serve any reachable one), else by tag; either way the commit
    # checked against the lock. No checkout, so nothing fetched runs (no hooks, no filters).
    { git -C "$tmp" -c http.extraHeader= fetch -q --no-tags "$url" "$nv" \
        || { [ -n "$nt" ] && git -C "$tmp" -c http.extraHeader= fetch -q --no-tags "$url" "refs/tags/$nt"; }; } 2>/dev/null || true
    if ! git -C "$tmp" cat-file -e "$nv^{commit}" 2>/dev/null; then
      printf "Couldn't fetch %s from %s.%s\n\n" "${nv:0:12}" "$url" "${cmp:+ Compare: $cmp}"
      rm -rf "$tmp"; continue
    fi
    git -C "$tmp" cat-file -e "$ov^{commit}" 2>/dev/null \
      || git -C "$tmp" -c http.extraHeader= fetch -q --no-tags "$url" "$ov" 2>/dev/null || true
    text=""
    if [ -n "$ot" ] && [ -n "$nt" ]; then
      text=$(git -C "$tmp" show "$nv:CHANGELOG.md" 2>/dev/null | changelog_between "$ot" "$nt") || text=""
    fi
    if [ -n "$text" ]; then
      printf 'From its CHANGELOG.md:\n\n%s\n\n' "$(cap 6000 <<<"$text")"
    elif git -C "$tmp" cat-file -e "$ov^{commit}" 2>/dev/null; then
      text=$(git -C "$tmp" log --oneline --no-decorate -n 51 "$ov..$nv" 2>/dev/null | sed 's/^/- /') || text=""
      printf 'No CHANGELOG.md section for this range; its commits:\n\n%s\n\n' "$(cap 6000 < <(head -n 50 <<<"${text:-"- (none)"}"))"
      [ "$(wc -l <<<"$text")" -le 50 ] || printf '(more than 50; see the compare link)\n\n'
    else
      printf 'No CHANGELOG.md section for this range, and %s is not in its history.\n\n' "${ov:0:12}"
    fi
    [ -z "$cmp" ] || printf 'Compare: %s\n\n' "$cmp"
    rm -rf "$tmp"
  done < <(jq -r '.nodes.root.inputs | to_entries[] | select(.value|type=="string") | .key' "$base")
  [ "$n" -gt 0 ] || printf 'No input moved commits.\n'
}

is_semver() { [[ $1 =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# changelog_between OLD NEW: from a CHANGELOG.md on stdin, the sections headed `## vX.Y.Z` or
# `## X.Y.Z` (or `## [X.Y.Z]`, anything after) for OLD < version <= NEW, headings moved two
# levels down to sit under the PR body's own.
changelog_between() {
  awk -v old="${1#v}" -v new="${2#v}" '
    function cmp(a, b,   x, y, i) {
      split(a, x, "."); split(b, y, ".")
      for (i = 1; i <= 3; i++) { if (x[i] + 0 < y[i] + 0) return -1; if (x[i] + 0 > y[i] + 0) return 1 }
      return 0
    }
    /^(```|~~~)/ { fence = !fence }
    !fence && /^## / {
      v = ""
      if (match($0, /^## \[?v?[0-9]+\.[0-9]+\.[0-9]+/)) { v = substr($0, RSTART, RLENGTH); sub(/^## \[?v?/, "", v) }
      keep = (v != "" && cmp(v, old) > 0 && cmp(v, new) <= 0)
    }
    !fence && /^# / { keep = 0 }
    keep && !fence && /^#{1,4} / { print "##" $0; next }
    keep { print }
  '
}

# At most N characters of stdin, cut at a line, with a note when cut.
cap() {
  awk -v max="$1" '{ if (total + length($0) + 1 > max) { cut = 1; exit } total += length($0) + 1; print }
                   END { if (cut) print "\n(cut at " max " characters; the full text is upstream)" }'
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
  git show HEAD:flake.lock > "$base_lock"
  git show "$head:flake.lock" > "$head_lock" 2>/dev/null || echo '{"nodes":{"root":{"inputs":{}}}}' > "$head_lock"
  ( unset PR_TOKEN; upstream_notes "$base_lock" "$head_lock" ) > "$notes" \
    || echo "(couldn't read what changed upstream; see the diff of flake.lock)" > "$notes"

  title="graft: upgrade"
  # shellcheck disable=SC2016 # backticks here are markdown
  {
    if [ -n "$failing" ]; then
      title="[checks failing] graft: upgrade"
      printf '> [!WARNING]\n> **`nix flake check` fails on this branch.** The upgrade is pushed so the breakage can be read; fix it here (or in the provider) before merging.\n\n'
    fi
    printf 'Moved by `nix run .#graft-upgrade` (graft'"'"'s own pin, and scions behind the newest tag of their pinned major):\n\n%s\n' "${moved:-"- (see the commit)"$'\n'}"
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
  } > "$body"

  # The token reaches curl as a header file on a pipe, never in a command line.
  api() { curl -sS -w '\n%{http_code}' -H @<(printf 'Authorization: token %s\n' "$PR_TOKEN") \
            -H "Content-Type: application/json" "$@"; }
  jq -n --arg head "$BRANCH" --arg base "$BASE" --arg title "$title" --rawfile body "$body" \
    '{head: $head, base: $base, title: $title, body: $body}' > "$req"
  out=$(api "$API/pulls?state=open&limit=50"); code=${out##*$'\n'}; out=${out%$'\n'*}
  [ "$code" = 200 ] || { echo "::error::listing PRs: HTTP $code"; exit 1; }
  number=$(jq -r --arg b "$BRANCH" '[.[] | select(.head.ref == $b)][0].number // empty' <<<"$out")
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
  *) die "usage: graft-ci check|upgrade|pr" ;;
esac
