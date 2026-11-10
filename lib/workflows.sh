#!/usr/bin/env bash
# workflows.sh check|refresh REPO EXAMPLES [RUN [FLAKEDIR [DIR...]]]
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
# A glob that could match a file graft-check reads is refused, since a push that changes only those
# must be checked: one that starts with `*`, `/` or `.forgejo`, and one that could match a file in
# .forgejo/, in a scions directory DIR (default `scions`), or the flake: FLAKEDIR's flake.nix and
# flake.lock at the top (FLAKEDIR `.`, the default), or anything in FLAKEDIR otherwise. Each glob
# is read as a shell pattern, where `*` and `?` also match `/`, so it errs toward refusing.
#
# One more line is yours, in graft-check.yaml only: `# graft ssh: ...`, for a repo whose scions
# include a private git+ssh one. Its job's container has no key, so with this line graft-check gets
# a step, before the check, that writes an ssh key from an Actions secret, a known_hosts entry and
# an ssh config for the host. The line (a comment, so it can sit anywhere; graft keeps it above
# `name:`) names the host as the scion's url spells it, optionally the address it resolves to, the
# host's public key, and optionally the secret (default GRAFT_SSH_KEY):
#
#     # graft ssh: host=[platform-vm]:2222 address=192.168.1.51 hostkey=ssh-rsa AAAA...
#     # graft ssh: secret=NOTEBOOK_APP_DEPLOY_KEY host=git.example.com hostkey=ssh-ed25519 AAAA...
#
# Without it, the file is EXAMPLES' copy (with any paths-ignore) byte for byte.
#
# mkFlake's check and `nix run .#graft` call this with their own examples; graft upgrade calls the
# copy in the version it moves to, with that version's examples. RUN is how the repo runs graft, for
# the message (default `nix run .#graft`). Bash, grep, cmp and coreutils only: mkFlake's app has
# nothing else.
set -euo pipefail

mode=$1 repo=$2 examples=$3 run=${4:-nix run .#graft} flakedir=${5:-.}
shift $(($# < 5 ? $# : 5))
marker='# managed by graft: '
rc=0
case $mode in check|refresh) ;; *) echo "usage: workflows.sh check|refresh REPO EXAMPLES [RUN [FLAKEDIR [DIR...]]]" >&2; exit 2 ;; esac

# What graft-check reads, which no paths-ignore glob may match: a path ending in / is a directory
# (any file in it), any other a file.
flakedir=${flakedir%/}
if [ -z "$flakedir" ] || [ "$flakedir" = . ]; then protected=(flake.nix flake.lock); else protected=("$flakedir/"); fi
protected+=(.forgejo/)
if [ $# -eq 0 ]; then set -- scions; fi
for d in "$@"; do protected+=("${d%/}/"); done

# could_match GLOB PATH: whether GLOB, read as a shell pattern, matches PATH, or (PATH ending in /)
# the directory's own name or any path under it: some prefix of GLOB matches PATH.
# shellcheck disable=SC2053 # the glob is the pattern
could_match() {
  local g=$1 p=$2 i
  if [[ ${p%/} == $g ]]; then return 0; fi
  if [[ $p != */ ]]; then return 1; fi
  for ((i = 1; i <= ${#g}; i++)); do
    if [[ $p == ${g:0:i} ]]; then return 0; fi
  done
  return 1
}

# A symlink could point anywhere: graft only writes regular files inside the repo.
for d in .forgejo .forgejo/workflows; do
  [ ! -L "$repo/$d" ] || { echo "graft: $d is a symlink; not reading workflows through it" >&2; exit 1; }
done

# paths_ignore FILE WF: FILE's paths-ignore line as graft writes it (`    paths-ignore: ["a", "b"]`),
# or nothing when it has none. Fails, saying why, on a line graft can't read, or two that differ.
paths_ignore() {
  local file=$1 wf=$2 line want="" got item parts globs p
  local form='one line under push: and pull_request:, a list of quoted globs and no comment after it, like paths-ignore: ["notebook/**"]'
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
      for p in "${protected[@]}"; do
        ! could_match "$item" "$p" \
          || { echo "graft: $wf: paths-ignore glob \"$item\" could match $p, which graft-check reads; name another directory, like \"notebook/**\"" >&2; return 1; }
      done
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

# ssh_parse LINE: the fields of a `# graft ssh:` line into SSH_SECRET SSH_HOST SSH_ALIAS SSH_ADDR
# SSH_HOSTKEY, each checked to be plain enough to sit in YAML and single-quoted shell; fails,
# saying why, otherwise. ssh_form is the line in graft's form.
ssh_parse() {
  local rest=${1#*graft ssh:} toks tok key="" val
  SSH_SECRET=GRAFT_SSH_KEY SSH_HOST="" SSH_ALIAS="" SSH_ADDR="" SSH_HOSTKEY=""
  read -ra toks <<<"$rest"   # read, not $rest unquoted: [host]:port is a glob
  for tok in "${toks[@]}"; do
    if [[ $tok =~ ^(secret|host|address|hostkey)= ]]; then   # (a key's base64 may end in =)
      key=${tok%%=*} val=${tok#*=}
      case $key in
        secret) SSH_SECRET=$val ;; host) SSH_HOST=$val ;; address) SSH_ADDR=$val ;; hostkey) SSH_HOSTKEY=$val ;;
      esac
    elif [ "$key" = hostkey ] && [[ -n $SSH_HOSTKEY && $SSH_HOSTKEY != *" "* ]]; then
      SSH_HOSTKEY+=" $tok"
    else
      echo "can't read \"$tok\""; return 1
    fi
  done
  [[ $SSH_SECRET =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { echo "secret= must be an Actions secret's name (letters, digits, _)"; return 1; }
  if [[ $SSH_HOST =~ ^\[([A-Za-z0-9.-]+)\]:[0-9]{1,5}$ ]]; then SSH_ALIAS=${BASH_REMATCH[1]}
  elif [[ $SSH_HOST =~ ^[A-Za-z0-9.-]+$ ]]; then SSH_ALIAS=$SSH_HOST
  else echo "host= must be the host as known_hosts names it: host, or [host]:port"; return 1; fi
  [[ -z $SSH_ADDR || $SSH_ADDR =~ ^[A-Za-z0-9.:-]+$ ]] || { echo "address= must be a host name or an IP address"; return 1; }
  [[ $SSH_HOSTKEY =~ ^(ssh-ed25519|ssh-rsa|ecdsa-sha2-nistp(256|384|521))\ [A-Za-z0-9+/]+={0,2}$ ]] \
    || { echo "hostkey= must be the host's public key, TYPE KEY (as in its .pub file, no comment)"; return 1; }
}
ssh_form() {
  local f="# graft ssh:"
  [ "$SSH_SECRET" = GRAFT_SSH_KEY ] || f+=" secret=$SSH_SECRET"
  f+=" host=$SSH_HOST"
  [ -z "$SSH_ADDR" ] || f+=" address=$SSH_ADDR"
  printf '%s hostkey=%s' "$f" "$SSH_HOSTKEY"
}

# ssh_line FILE WF: FILE's `# graft ssh:` line in graft's form, or nothing when it has none. Fails,
# saying why, on a line graft can't read, or two that differ.
ssh_line() {
  local file=$1 wf=$2 line want="" got why
  local form='# graft ssh: host=[HOST]:PORT address=ADDRESS hostkey=TYPE KEY (address= and secret=NAME optional)'
  while IFS= read -r line || [ -n "$line" ]; do
    [[ $line =~ ^[[:space:]]*#[[:space:]]*graft\ ssh: ]] || continue
    why=$(ssh_parse "$line") || { echo "graft: $wf: its \"# graft ssh:\" line: $why; write it as $form" >&2; return 1; }
    ssh_parse "$line" >/dev/null
    got=$(ssh_form)
    if [ -z "$want" ]; then want=$got
    elif [ "$want" != "$got" ]; then
      echo "graft: $wf: its \"# graft ssh:\" lines differ; keep one" >&2; return 1
    fi
  done < "$file"
  printf '%s' "$want"
}

# ssh_step: the step a `# graft ssh:` line asks for (ssh_parse's fields), and a blank line after it
ssh_step() {
  local cfg="'Host $SSH_ALIAS'" names=$SSH_HOST
  # ssh looks the key up under the name it connects to: with an address, that's the address
  if [ -n "$SSH_ADDR" ]; then cfg+=" '  HostName $SSH_ADDR'" names+=",${SSH_HOST/"$SSH_ALIAS"/"$SSH_ADDR"}"; fi
  cat <<EOF
      - name: SSH key for private scions
        # graft writes this step from the "# graft ssh:" line at the top of this file
        env:
          GRAFT_SSH_KEY: \${{ secrets.$SSH_SECRET }}
        run: |
          set -euo pipefail
          if [ -z "\$GRAFT_SSH_KEY" ]; then
            echo "::error::the $SSH_SECRET secret is empty or not set"; exit 1
          fi
          # nixos/nix has an ssh client; another image may not
          if ! command -v ssh >/dev/null; then
            ssh_pkg=\$(nix build --no-link --print-out-paths nixpkgs#openssh)
            echo "\$ssh_pkg/bin" >> "\$GITHUB_PATH"
          fi
          mkdir -p ~/.ssh && chmod 700 ~/.ssh
          printf '%s\n' "\$GRAFT_SSH_KEY" > ~/.ssh/graft_scions && chmod 600 ~/.ssh/graft_scions
          echo '$names $SSH_HOSTKEY' >> ~/.ssh/known_hosts
          printf '%s\n' $cfg '  IdentityFile ~/.ssh/graft_scions' '  IdentitiesOnly yes' \\
            '  StrictHostKeyChecking yes' '  CheckHostIP no' '  BatchMode yes' >> ~/.ssh/config

EOF
}

# render_ssh FILE LINE: FILE with LINE (a `# graft ssh:` line, graft's form) above its first line
# that isn't a comment, and ssh_step before its check step. Fails, saying why, when FILE has no
# check step (a workflow other than graft-check.yaml).
render_ssh() {
  local file=$1 line top="" step="" l
  ssh_parse "$2" >/dev/null || return 1
  while IFS= read -r l || [ -n "$l" ]; do
    if [ -z "$top" ] && [[ ! $l =~ ^# ]]; then printf '%s\n' "$2"; top=1; fi
    if [ "$l" = "      - name: Scions match flake.lock" ]; then ssh_step; step=1; fi
    printf '%s\n' "$l"
  done < "$file"
  [ -n "$step" ]
}

# render SRC LINE: SRC with LINE as the last child of each `push:` and `pull_request:` trigger in
# its top-level `on:` block. With no LINE, SRC itself.
render() {
  local src=$1 pi=$2 line inon="" pending="" held=()
  if [ -z "$pi" ]; then cat "$src"; return; fi
  while IFS= read -r line || [ -n "$line" ]; do
    if [ -n "$inon" ]; then
      # blank lines and comments wait: the line goes after a trigger's last child, before them
      if [[ $line =~ ^[[:space:]]*(#.*)?$ ]]; then held+=("$line"); continue; fi
      if [[ ! $line =~ ^\ \ \  ]]; then   # not a trigger's child: the trigger (or `on:`) ends here
        if [ -n "$pending" ]; then printf '%s\n' "$pi"; pending=""; fi
        if [[ ! $line =~ ^\  ]]; then inon=""; fi
        if [[ $line =~ ^\ \ (push|pull_request):[[:space:]]*$ ]]; then pending=1; fi
      fi
      if [ ${#held[@]} -gt 0 ]; then printf '%s\n' "${held[@]}"; held=(); fi
    elif [[ $line =~ ^on:[[:space:]]*$ ]]; then
      inon=1
    fi
    printf '%s\n' "$line"
  done < "$src"
  if [ -n "$pending" ]; then printf '%s\n' "$pi"; fi
  if [ ${#held[@]} -gt 0 ]; then printf '%s\n' "${held[@]}"; fi
}

want=$(mktemp)
trap 'rm -f "$want" "$want.ssh"' EXIT
for src in "$examples"/*; do
  [ -f "$src" ] || continue
  wf=.forgejo/workflows/${src##*/}
  if [ ! -f "$repo/$wf" ] || [ -L "$repo/$wf" ] || ! grep -q "^$marker" "$repo/$wf"; then continue; fi
  if ! pi=$(paths_ignore "$repo/$wf" "$wf"); then
    echo "graft: (that line is the only edit graft keeps in $wf; to keep others, delete its \"$marker\" line)" >&2
    rc=1; continue
  fi
  if ! sl=$(ssh_line "$repo/$wf" "$wf"); then
    echo "graft: (that line, and paths-ignore, are the only edits graft keeps in $wf; to keep others, delete its \"$marker\" line)" >&2
    rc=1; continue
  fi
  render "$src" "$pi" > "$want"
  if [ -n "$pi" ] && ! grep -qxF -- "$pi" "$want"; then
    echo "graft: $wf: its paths-ignore is kept only under push: and pull_request:, and this graft version's copy has neither; delete the line" >&2
    rc=1; continue
  fi
  if [ -n "$sl" ]; then
    if ! render_ssh "$want" "$sl" > "$want.ssh"; then
      echo "graft: $wf: its \"# graft ssh:\" line is kept only in graft-check.yaml, before its check step, and this graft version's copy has none; delete the line" >&2
      rc=1; continue
    fi
    mv "$want.ssh" "$want"
  fi
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
