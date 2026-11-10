#!/usr/bin/env bash
# workflows.sh check|refresh REPO EXAMPLES [RUN]
#
# graft's CI workflows in REPO: each file in REPO/.forgejo/workflows/ named like one in EXAMPLES
# (graft's template/.forgejo/workflows/, at the version the repo pins) whose text has graft's marker line.
# graft owns those and nothing else: a workflow without the marker is yours and is never touched,
# and graft never adds one you didn't copy in.
#
#   check    fail, naming each of graft's workflows that differs from EXAMPLES
#   refresh  replace each of graft's workflows that differs from EXAMPLES with EXAMPLES' copy
#
# One line of graft's copy is yours: `paths-ignore:` under its `push:` and `pull_request:`
# triggers, the globs whose changes alone don't run it (a notes directory a bot pushes to often,
# say). It is read from the repo's copy and written under both triggers of EXAMPLES' copy before
# comparing, so it survives `nix run .#graft` and `graft upgrade`. Without one, the file is
# EXAMPLES' copy byte for byte. It is one line, a list of quoted globs:
#
#     paths-ignore: ["notebook/**", "drafts/**"]
#
# A glob that starts with `*`, `/` or `.forgejo` is refused: it could match graft's own files (the
# scions directory, flake.lock, these workflows), and a push that changes those must be checked.
#
# mkFlake's check and `nix run .#graft` call this with their own examples; graft upgrade calls the
# copy in the version it moves to, with that version's examples. RUN is how the repo runs graft, for
# the message (default `nix run .#graft`). Bash, grep, cmp and coreutils only: mkFlake's app has
# nothing else.
set -euo pipefail

mode=$1 repo=$2 examples=$3 run=${4:-nix run .#graft}
marker='# managed by graft: '
rc=0
case $mode in check|refresh) ;; *) echo "usage: workflows.sh check|refresh REPO EXAMPLES [RUN]" >&2; exit 2 ;; esac

# A symlink could point anywhere: graft only writes regular files inside the repo.
for d in .forgejo .forgejo/workflows; do
  [ ! -L "$repo/$d" ] || { echo "graft: $d is a symlink; not reading workflows through it" >&2; exit 1; }
done

# paths_ignore FILE WF: FILE's paths-ignore line as graft writes it (`    paths-ignore: ["a", "b"]`),
# or nothing when it has none. Fails, saying why, on a line graft can't read, or two that differ.
paths_ignore() {
  local file=$1 wf=$2 line want="" got item parts globs
  local form='one line under push: and pull_request:, a list of quoted globs, like paths-ignore: ["notebook/**"]'
  while IFS= read -r line || [ -n "$line" ]; do
    [[ $line =~ ^[[:space:]]*paths-ignore: ]] || continue
    [[ $line =~ ^\ \ \ \ paths-ignore:\ *\[(.*)\][[:space:]]*$ ]] \
      || { echo "graft: $wf: can't read its paths-ignore; write it as $form" >&2; return 1; }
    IFS=, read -ra parts <<<"${BASH_REMATCH[1]}"
    globs=()
    for item in "${parts[@]}"; do
      item=${item#"${item%%[![:space:]]*}"} item=${item%"${item##*[![:space:]]}"}
      if [[ $item =~ ^\"([^\"]*)\"$ || $item =~ ^\'([^\']*)\'$ ]]; then item=${BASH_REMATCH[1]}; else
        echo "graft: $wf: can't read its paths-ignore; write it as $form" >&2; return 1
      fi
      [[ $item =~ ^[A-Za-z0-9._/*?-]+$ ]] \
        || { echo "graft: $wf: paths-ignore glob \"$item\": use only letters, digits, . _ / * ? and -" >&2; return 1; }
      [[ ! $item =~ ^(\*|/|\.forgejo) ]] \
        || { echo "graft: $wf: paths-ignore glob \"$item\" could match graft's own files (it starts with *, / or .forgejo); name a directory, like \"notebook/**\"" >&2; return 1; }
      globs+=("\"$item\"")
    done
    [ ${#globs[@]} -gt 0 ] || { echo "graft: $wf: its paths-ignore is empty; delete the line instead" >&2; return 1; }
    got="    paths-ignore: ["
    for item in "${globs[@]}"; do got+="$item, "; done
    got="${got%, }]"
    if [ -z "$want" ]; then want=$got
    elif [ "$want" != "$got" ]; then
      echo "graft: $wf: its paths-ignore lines differ; make them the same" >&2; return 1
    fi
  done < "$file"
  printf '%s' "$want"
}

# render SRC LINE: SRC with LINE as the last child of each `push:` and `pull_request:` trigger in
# its top-level `on:` block. With no LINE, SRC itself.
render() {
  local src=$1 pi=$2 line inon="" pending="" held=0
  if [ -z "$pi" ]; then cat "$src"; return; fi
  while IFS= read -r line || [ -n "$line" ]; do
    if [ -n "$inon" ]; then
      # blank lines wait: the line goes after a trigger's last child, before any blank line
      if [ -z "$line" ]; then held=$((held + 1)); continue; fi
      if [[ ! $line =~ ^\ \ \  ]]; then   # not a trigger's child: the trigger (or `on:`) ends here
        if [ -n "$pending" ]; then printf '%s\n' "$pi"; pending=""; fi
        if [[ ! $line =~ ^\  ]]; then inon=""; fi
        if [[ $line =~ ^\ \ (push|pull_request):[[:space:]]*$ ]]; then pending=1; fi
      fi
      while [ "$held" -gt 0 ]; do echo; held=$((held - 1)); done
    elif [[ $line =~ ^on:[[:space:]]*$ ]]; then
      inon=1
    fi
    printf '%s\n' "$line"
  done < "$src"
  if [ -n "$pending" ]; then printf '%s\n' "$pi"; fi
  while [ "$held" -gt 0 ]; do echo; held=$((held - 1)); done
}

want=$(mktemp)
trap 'rm -f "$want"' EXIT
for src in "$examples"/*; do
  [ -f "$src" ] || continue
  wf=.forgejo/workflows/${src##*/}
  if [ ! -f "$repo/$wf" ] || [ -L "$repo/$wf" ] || ! grep -q "^$marker" "$repo/$wf"; then continue; fi
  if ! pi=$(paths_ignore "$repo/$wf" "$wf"); then
    echo "graft: (that line is the only edit graft keeps in $wf; to keep others, delete its \"$marker\" line)" >&2
    rc=1; continue
  fi
  render "$src" "$pi" > "$want"
  if cmp -s "$want" "$repo/$wf"; then continue; fi
  case $mode in
    check)
      echo "graft: $wf differs from the copy this graft version ships; run: $run" >&2
      echo "graft: (to keep your own edits, delete its \"$marker\" line; graft then leaves it alone)" >&2
      rc=1 ;;
    refresh)
      cp --no-preserve=mode "$want" "$repo/$wf"
      echo "graft: refreshed $wf" ;;
  esac
done
exit "$rc"
