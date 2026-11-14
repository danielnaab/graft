# shellcheck shell=bash
# The sandbox launcher (lib/sandbox/bwrap.nix sources it after the policy it renders, policy.sh):
# runs a command in bubblewrap with its closure read-only, the repository around the current
# directory writable, HOME empty, and the network through an allowlisting proxy, or none.
#
# The sandbox never writes what decides the sandbox or what the host runs. Host git trusts the
# repository's git metadata (a planted core.fsmonitor runs on the next host `git status`), so each
# git directory, its config and hooks, and the files that say where a repository is, are pinned:
# bound onto themselves, a mount point, which can't be renamed or removed (EBUSY). A directory that
# only holds a mount point can still be renamed (and a fresh one made in its place), so every
# directory between the repository's top and a pinned path is pinned too. The protect list is
# pinned the same way, read-only. A symbolic link can't be pinned (bubblewrap would bind what it
# points to), so one anywhere on those paths stops the launcher before the run. What a run can
# still create (a new nested repository in any writable directory) the launcher reports after it,
# from the host.
# shellcheck disable=SC2154 # policy.sh's variables

usage="usage: sandbox [--with PKG]... [--allow HOST[:PORT]]... [--explain [--json]] [--dry-run] [--] [CMD [ARGS...]]

Runs CMD (default: bash) in a sandbox, in the git repository around the current directory: the
closure of the policy's packages read-only, the repository writable (but for what decides the
sandbox or what the host runs), HOME empty, the network only to the hosts the policy allows.

  --with PKG           add the nixpkgs package PKG (or an installable, FLAKE#ATTR), for this run
  --allow HOST[:PORT]  let this run reach HOST (ports 443 and 80), or HOST:PORT
  --explain            print the policy, each value with the file that sets it; --json: as JSON
  --dry-run            print the bubblewrap command, and run nothing
  GRAFT_SANDBOX_QUIET=1 silences the startup line."

# visible: stdin to stdout with each control byte (C0, ESC, DEL, C1) shown as \xNN. Text a run
# chose (paths in its repository, hosts it asked the proxy for) reaches the host's terminal only
# through it: an escape sequence could otherwise erase the warning, or worse.
visible() {
  LC_ALL=C awk 'BEGIN { for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i }
    { o = ""; for (i = 1; i <= length($0); i++) { c = substr($0, i, 1)
        o = o (c ~ /[\001-\037\177-\237]/ ? sprintf("\\x%02x", ord[c]) : c) }
      print o }'
}
say() { [ -n "${GRAFT_SANDBOX_QUIET:-}" ] || echo "graft: sandbox: $*" | visible >&2; }
die() { echo "graft: sandbox: $*" | visible >&2; exit 1; }
bad_arg() { echo "graft: sandbox: $*" >&2; echo "${usage%%$'\n'*}" >&2; exit 2; }
# join A B...: "A, B"
join() { local s; s=$(printf '%s, ' "$@"); echo "${s%, }"; }
home() { echo "${1/#\~\//$HOME/}"; }
tilde() { echo "${1/#"$HOME"/\~}"; }

with=() cliAllow=() explain='' json='' dry=''
while [ $# -gt 0 ]; do
  case $1 in
    --with) [ $# -ge 2 ] || bad_arg "--with needs a package name"; with+=("$2"); shift 2 ;;
    --allow) [ $# -ge 2 ] || bad_arg "--allow needs a host"; cliAllow+=("$2"); shift 2 ;;
    --explain) explain=1; shift ;;
    --json) json=1; shift ;;
    --dry-run) dry=1; shift ;;
    -h | --help) echo "$usage"; exit 0 ;;
    --) shift; break ;;
    -*) bad_arg "unknown option $1" ;;
    *) break ;;
  esac
done
[ -z "$json" ] || [ -n "$explain" ] || bad_arg "--json goes with --explain"
# --allow: the policy's network.allow syntax; "host" is ports 443 and 80
for h in "${cliAllow[@]}"; do
  if ! [[ $h =~ ^[A-Za-z0-9*?_][A-Za-z0-9*?._-]*(:([0-9]{1,5}))?$ ]] \
    || { [ -n "${BASH_REMATCH[2]}" ] && ! ((10#${BASH_REMATCH[2]} >= 1 && 10#${BASH_REMATCH[2]} <= 65535)); }; then
    bad_arg "--allow $h: give \"host\" or \"host:port\" (the host may use * and ?, as in \"*.example.com\")"
  fi
  allowShown+=("$h")
  if [ -n "${BASH_REMATCH[2]}" ]; then allow+=("$h"); else allow+=("$h:443" "$h:80"); fi
done

# paths.readOnly and paths.writable that this host lacks: skipped
absent=()
for p in "${readOnly[@]}" "${writable[@]}"; do [ -e "$(home "$p")" ] || absent+=("$p"); done

if [ -n "$explain" ]; then
  if [ -n "$json" ]; then
    list() { jq -cn '$ARGS.positional' --args "$@"; }
    jq --argjson allow "$(list "${cliAllow[@]}")" --argjson with "$(list "${with[@]}")" --argjson absent "$(list "${absent[@]}")" '
      .options["network.allow"] += [$allow[] | { value: ., file: "command line" }]
      | .options.packages += [$with[] | { value: ., file: "command line" }]
      | .absent = $absent' "$explainJson"
  else
    cat "$explainText"
    if [ $((${#cliAllow[@]} + ${#with[@]})) -gt 0 ]; then
      printf '\ncommand line (this run only)\n'
      for h in "${cliAllow[@]}"; do printf '  %-38s  command line\n' "network.allow $h"; done
      for w in "${with[@]}"; do printf '  %-38s  command line\n' "packages $w"; done
    fi
    [ ${#absent[@]} -eq 0 ] || printf '\nabsent on this host (skipped)\n%s\n' "$(printf '  %s\n' "${absent[@]}")"
  fi
  exit 0
fi
[ $# -gt 0 ] || set -- "$shell"
[ -n "${HOME:-}" ] || die "HOME is not set: a run gets an empty HOME, at the same path"

# the repository: the outermost one around the current directory (a scion's belongs to it)
here=$(pwd -P)
top=$(git rev-parse --show-toplevel 2>/dev/null) || die "$here is not in a git repository: a run writes to the repository around the current directory"
while sup=$(git -C "$top" rev-parse --show-superproject-working-tree) && [ -n "$sup" ]; do top=$sup; done
gitpaths=$(git -C "$top" rev-parse --path-format=absolute --git-common-dir --git-path hooks) \
  || die "git rev-parse --path-format=absolute failed in $top (above): the launcher needs git 2.31 or later"
{ read -r common; read -r hooks; } <<<"$gitpaths"

# --with: a package's closure and bin/, for this run alone, from the flake's nixpkgs (else graft's,
# else the registry's); NIXPKGS_ALLOW_UNFREE=1 is honoured (--impure)
withPath='' withBinds=() impure=()
[ "${NIXPKGS_ALLOW_UNFREE:-}" != 1 ] || impure=(--impure)
for p in "${with[@]}"; do
  command -v nix >/dev/null || die "--with $p needs nix on PATH"
  if [[ $p == *'#'* ]]; then ref=$p from="from ${p%%#*}"
  elif [ -e "$nixpkgs" ]; then ref="path:$nixpkgs#$p" from=$nixpkgsFrom
  else ref="nixpkgs#$p" from="the registry's nixpkgs"
  fi
  outs=$(nix build "${impure[@]}" --no-link --print-out-paths "$ref") || die "--with $p: nix build $ref failed (above)"
  # shellcheck disable=SC2016 # Nix's
  what=$(nix eval "${impure[@]}" --raw "$ref" --apply 'p: "${p.pname or (builtins.parseDrvName p.name).name} ${p.version or ""}"') || what=$p
  for o in $outs; do [ ! -d "$o/bin" ] || withPath+="$o/bin:"; done
  # shellcheck disable=SC2086 # one store path per word
  while read -r c; do withBinds+=(--ro-bind "$c" "$c"); done < <(nix path-info -r $outs)
  say "with ${what% } ($from), for this run"
done

# the command must be in the run: a name, on the run's PATH
if [[ $1 != */* ]]; then
  IFS=: read -ra dirs <<<"$withPath$path"
  found=''
  for d in "${dirs[@]}"; do [ ! -x "$d/$1" ] || { found=1; break; }; done
  [ -n "$found" ] || { echo "graft: sandbox: $1 is not on the sandbox's PATH: add its package to sandbox.packages, or run with --with PKG" >&2; exit 127; }
fi

run=$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/graft-sandbox.XXXXXX")
proxyPid=''
# shellcheck disable=SC2317 # the trap runs it
cleanup() {
  [ -z "$proxyPid" ] || kill "$proxyPid" 2>/dev/null || true
  rm -f -- "${run:?}"/* && rmdir -- "${run:?}"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP

# the proxy starts first: it is the slowest part, and the rest doesn't need it
if [ ${#allow[@]} -gt 0 ] && [ -z "$dry" ]; then
  # a relative socket path: AF_UNIX allows 108 bytes
  (cd "$run" && exec "${proxy[@]}" proxy.sock "${allow[@]}") >"$run/proxy.log" 2>&1 &
  proxyPid=$!
fi

shopt -s nullglob dotglob
# Pins. mounts: directories bound onto themselves (still writable, but mount points), outer ones
# first; ro: paths bound read-only onto themselves, after them (so read-only wins)
declare -A seen=()
mounts=() ro=()
rel() { case $1 in "$top"/*) echo "${1#"$top"/}" ;; *) echo "$1" ;; esac; }
# nolink P ROOT: stop when P, or a directory between it and ROOT, is a symbolic link
nolink() {
  local p=$1 d=$1 what
  while [ ${#d} -gt ${#2} ]; do
    if [ -L "$d" ]; then
      if [ "$d" = "$p" ]; then what="make it read-only"; else what="pin $(rel "$p"), below it"; fi
      die "$(rel "$d") is a symbolic link, so the sandbox can't $what (a run must not write what decides the sandbox or what the host runs): replace the link with what it points to, or remove it, then run again"
    fi
    d=${d%/*}
  done
}
# pin rw|ro PATH: PATH and every directory between it and the repository's top (or the git common
# directory, outside a worktree's top) a mount point; with ro, PATH read-only
pin() {
  local how=$1 p=$2 root d up=()
  case $p in "$top"/*) root=$top ;; *) root=$common ;; esac
  nolink "$p" "$root"
  d=$p
  while d=${d%/*} && [ ${#d} -gt ${#root} ]; do up=("$d" "${up[@]}"); done
  [ "$how" = ro ] || up+=("$p")
  for d in "${up[@]}"; do [ -n "${seen[$d]:-}" ] || { seen[$d]=1; mounts+=(--bind "$d" "$d"); }; done
  [ "$how" = rw ] || [ -n "${seen[ro:$p]:-}" ] || { seen[ro:$p]=1; ro+=(--ro-bind "$p" "$p"); }
}
# gitdirs G: G, a git directory, and those below it: worktrees' and submodules' (modules/NAME, where
# NAME may hold slashes)
gitdirs() {
  local d
  echo "$1"
  for d in "$1"/worktrees/*/; do [ ! -f "$d/HEAD" ] || gitdirs "${d%/}"; done
  [ ! -d "$1/modules" ] || below "$1/modules"
}
below() {
  local d
  for d in "$1"/*/; do
    d=${d%/}
    if [ -f "$d/HEAD" ]; then gitdirs "$d"; else below "$d"; fi
  done
}
# a real directory, not a link to one
isdir() { [ -d "$1" ] && [ ! -L "$1" ]; }
# every git directory: below each .git directory in the working tree, and the common one
allgitdirs() {
  local e
  for e in "${dotgits[@]}"; do ! isdir "$e" || gitdirs "$e"; done
  [ "$common" = "$top/.git" ] || gitdirs "$common"
}
# what in a git directory says what host git runs (config, hooks) or where a repository is
gitfiles=(config config.worktree hooks info commondir gitdir)
# protect's "**/NAME" entries, for find
names=()
for p in "${protect[@]}"; do
  [[ $p == '**/'* ]] || continue
  [ ${#names[@]} -eq 0 ] || names+=(-o)
  names+=(-name "${p#'**/'}")
done
# The walk of the working tree: every .git entry (the repository's, each scion's, a nested
# repository's), not entered, and what the "**/NAME" entries name
scan() {
  local e expr=(-name .git -prune -print0)
  [ ${#names[@]} -eq 0 ] || expr+=(-o "(" "${names[@]}" ")" -print0)
  dotgits=() named=()
  while IFS= read -r -d '' e; do
    if [ "${e##*/}" = .git ]; then dotgits+=("$e"); else named+=("$e"); fi
  done < <(find "$top" "${expr[@]}")
}
# What host git, a shell prompt, an editor or direnv would read and could run code from: a line per
# path, and one per gitlink in the index. Before the run and after it; what is new is reported.
fingerprint() {
  local g f p
  printf '%s\n' "${dotgits[@]}" "${named[@]}"
  while read -r g; do
    echo "$g"
    for f in "${gitfiles[@]}"; do [ ! -e "$g/$f" ] || echo "$g/$f"; done
  done < <(allgitdirs)
  git -C "$top" ls-files -s | awk '$1 == "160000" { print "gitlink " $4 }'
  for p in "${protect[@]}"; do [[ $p == '**/'* ]] || [ ! -e "$top/${p%/}" ] || echo "$top/${p%/}"; done
}

if [ "$repoReadOnly" = 0 ]; then
  scan
  while read -r g; do
    pin rw "$g"
    # a hooks directory that isn't there is one the run could create
    [ -n "$dry" ] || [ ! -f "$g/config" ] || [ -d "$g/hooks" ] || mkdir "$g/hooks"
    for f in "${gitfiles[@]}"; do [ ! -e "$g/$f" ] || pin ro "$g/$f"; done
  done < <(allgitdirs)
  # a .git file (a scion's, a worktree's) says where its repository is
  for e in "${dotgits[@]}"; do isdir "$e" || pin ro "$e"; done
  for e in "${named[@]}"; do pin ro "$e"; done
  for p in "${protect[@]}"; do
    [[ $p != '**/'* ]] || continue
    p=$top/${p%/}
    nolink "$p" "$top"
    [ ! -e "$p" ] || pin ro "$p"
  done
  # core.hooksPath, when it names a directory in the repository (.githooks)
  case $hooks in
    "$top"/* | "$common"/*)
      [ -n "$dry" ] || mkdir -p "$hooks"
      pin ro "$hooks"
      ;;
  esac
  # a pinned file with another hard link can be written through that one, from anywhere the run writes
  pinned=()
  for ((i = 1; i < ${#ro[@]}; i += 3)); do pinned+=("${ro[i]}"); done
  while IFS= read -r -d '' f; do
    die "$(rel "$f") has another hard link, so a run could write it through that link (a run must not write what decides the sandbox or what the host runs): replace it with a copy of its own (cp -p F F.copy && mv F.copy F), then run again"
  done < <(find "${pinned[@]}" -maxdepth 0 -type f -links +1 -print0)
  fingerprint | sort -u >"$run/before"
fi

# the run's identity: the caller's, and git's from the host's config (the run sees no global config)
user=${USER:-$(id -un)}
printf '%s:x:%s:%s::%s:%s\n' "$user" "$UID" "$(id -g)" "$HOME" "$shell" >"$run/passwd"
printf '%s:x:%s:\n' "$(id -gn)" "$(id -g)" >"$run/group"
printf '127.0.0.1 localhost\n::1 localhost\n' >"$run/hosts"
name='' email='' sign=''
while IFS= read -r -d '' kv; do
  case ${kv%%$'\n'*} in
    user.name) name=${kv#*$'\n'} ;;
    user.email) email=${kv#*$'\n'} ;;
    commit.gpgsign) sign=${kv#*$'\n'} ;;
  esac
done < <(git -C "$top" config -z --get-regexp '^(user\.(name|email)|commit\.gpgsign)$' || true)

# The run's environment, whole: what follows, and of the caller's only TERM, COLORTERM and env.pass.
envv=(HOME="$HOME" USER="$user" LOGNAME="$user" SHELL="$shell" PATH="$withPath$path" LANG=C.UTF-8
  TERMINFO_DIRS="$terminfo" SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
  NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt GRAFT_SANDBOX=1 PS1="$ps1"
  # no signing key inside: a repository that signs would refuse every commit
  GIT_CONFIG_COUNT=2 GIT_CONFIG_KEY_0=commit.gpgsign GIT_CONFIG_VALUE_0=false
  GIT_CONFIG_KEY_1=tag.gpgsign GIT_CONFIG_VALUE_1=false)
[ -z "$name" ] || envv+=(GIT_AUTHOR_NAME="$name" GIT_COMMITTER_NAME="$name")
[ -z "$email" ] || envv+=(GIT_AUTHOR_EMAIL="$email" GIT_COMMITTER_EMAIL="$email")
for v in TERM COLORTERM; do [ -z "${!v+x}" ] || envv+=("$v=${!v}"); done
if [ ${#allow[@]} -gt 0 ]; then
  for v in HTTPS_PROXY https_proxy HTTP_PROXY http_proxy ALL_PROXY all_proxy; do envv+=("$v=http://127.0.0.1:3128"); done
  envv+=('NO_PROXY=localhost,127.0.0.1,::1' 'no_proxy=localhost,127.0.0.1,::1')
fi
envv+=("${envSet[@]}")
passed=()
for v in "${pass[@]}"; do [ -z "${!v+x}" ] || { envv+=("$v=${!v}"); passed+=("$v"); }; done

args=("${withBinds[@]}" --unshare-all --die-with-parent --proc /proc --dev /dev --tmpfs /tmp
  --ro-bind "$run/passwd" /etc/passwd --ro-bind "$run/group" /etc/group --ro-bind "$run/hosts" /etc/hosts
  --ro-bind "$cacert" /etc/ssl/certs/ca-certificates.crt
  --ro-bind "$sh" /bin/sh --ro-bind "$envbin" /usr/bin/env
  --tmpfs "$HOME")
for p in "${readOnly[@]}"; do p=$(home "$p"); args+=(--ro-bind-try "$p" "$p"); done
for p in "${writable[@]}"; do p=$(home "$p"); args+=(--bind-try "$p" "$p"); done
if [ "$repoReadOnly" = 0 ]; then
  args+=(--bind "$top" "$top" "${mounts[@]}" "${ro[@]}")
else
  args+=(--ro-bind "$top" "$top")
  [ "$common" = "$top/.git" ] || args+=(--ro-bind "$common" "$common")
fi
[ ${#allow[@]} -eq 0 ] || args+=(--ro-bind "$run/proxy.sock" /run/graft-sandbox/proxy.sock)
args+=(--chdir "$here" --seccomp 4 --json-status-fd 5 --clearenv)
# bwrap's arguments: the closure (3), the relay's closure (7) when there is network, then the rest
# and the environment (6), all read from descriptors, so that /proc/1/cmdline in the run names no
# host path, and ps no passed value. The command's INT is the default again (--default-signal):
# the launcher and bwrap ignore it, so that ^C reaches the command alone, and its status and the
# checks after the run survive it.
net=()
if [ ${#allow[@]} -gt 0 ]; then net=(--args 7); set -- "$netwrap" "$@"; fi
cmd=("$bwrap" --args 3 "${net[@]}" --args 6 -- "$envbin" --default-signal=INT "$@")

line=$(tilde "$top")
if [ "$repoReadOnly" = 1 ]; then line+=" read-only"; else line+=", $((${#ro[@]} / 3)) paths protected"; fi
present=(); for p in "${writable[@]}"; do if [ -e "$(home "$p")" ]; then present+=("$p"); fi; done
[ ${#present[@]} -eq 0 ] || line+="; also writable: $(join "${present[@]}")"
if [ ${#allowShown[@]} -gt 0 ]; then line+="; network: $(join "${allowShown[@]}")"; else line+="; network: none"; fi
[ ${#passed[@]} -eq 0 ] || line+="; env: $(join "${passed[@]}")"
[ ${#absent[@]} -eq 0 ] || line+="; absent, skipped: $(join "${absent[@]}")"
case $sign in true | yes | on | 1) [ "$repoReadOnly" = 1 ] || line+="; commits unsigned" ;; esac
case $notGranted in 0) ;; 1) line+="; 1 request not granted: sandbox --explain" ;; *) line+="; $notGranted requests not granted: sandbox --explain" ;; esac
say "$line"

if [ -n "$dry" ]; then
  printf '%q ' env -i "${cmd[@]}"
  printf '\n# --args 3: --ro-bind for each of %s store paths (%s)\n' "$(($(tr -cd '\0' <"$binds" | wc -c) / 3))" "$binds"
  [ ${#allow[@]} -eq 0 ] || printf '# --args 7: --ro-bind for the network relay'"'"'s closure (%s)\n' "$netBinds"
  printf '# --args 6:'
  printf ' %q' "${args[@]}"
  echo
  for v in "${envv[@]}"; do
    n=${v%%=*}
    if [[ " ${passed[*]} " == *" $n "* ]]; then
      printf '#   --setenv %s "$%s" (env.pass)\n' "$n" "$n"
    else
      printf '#   --setenv %s %q\n' "$n" "${v#*=}"
    fi
  done
  exit 0
fi

if [ -n "$proxyPid" ]; then
  for _ in {1..300}; do
    { [ ! -S "$run/proxy.sock" ] && kill -0 "$proxyPid" 2>/dev/null; } || break
    sleep 0.01
  done
  [ -S "$run/proxy.sock" ] || die "the network proxy did not start: $(cat "$run/proxy.log")"
fi

status=0
exec {argfd}< <(
  printf '%s\0' "${args[@]}"
  for v in "${envv[@]}"; do printf -- '--setenv\0%s\0%s\0' "${v%%=*}" "${v#*=}"; done
)
trap '' INT
# env -i: bwrap is PID 1 in the run, which can read /proc/1/environ. {argfd}<&- closes the pipe's
# other descriptor.
env -i "${cmd[@]}" 3<"$binds" 4<"$bpf" 5>"$run/status" 6<&"$argfd" 7<"$netBinds" {argfd}<&- || status=$?

if [ "$status" -ne 0 ] && [[ $(<"$run/status") != *child-pid* ]] && ! "$bwrap" --unshare-user --ro-bind / / true 2>/dev/null; then
  echo "graft: sandbox: bubblewrap can't create a user namespace here (above). In Docker, run the container with --security-opt seccomp=unconfined --security-opt systempaths=unconfined, or run on the host or a VM; elsewhere, allow unprivileged user namespaces (sysctl user.max_user_namespaces). graft never runs it unsandboxed." >&2
fi
if [ -s "$run/proxy.log" ]; then
  denied=$(awk '$1 == "DENY" { print $2 }' "$run/proxy.log" | sort | uniq -c | awk '{ printf "%s%s (%d)", s, $2, $1; s = ", " }')
  [ -z "$denied" ] || echo "graft: sandbox: the proxy refused: $denied; network.allow lists the hosts a run may reach" | visible >&2
fi
if [ "$repoReadOnly" = 0 ]; then
  scan
  new=$(fingerprint | sort -u | comm -13 "$run/before" -)
  if [ -n "$new" ]; then
    new=${new//"$top/"/}
    {
      echo "graft: sandbox: WARNING: the run created what host git, a shell prompt, an editor or direnv reads, and may run code from (in $(tilde "$top")):"
      echo "  ${new//$'\n'/$'\n'  }"
      echo "graft: sandbox: read a new repository's config before anything runs git in it: git --no-pager config --file DIR/.git/config --list | cat -v (git -C DIR would obey that config); remove one you didn't expect: git clean -ffd DIR (plain -fd keeps a nested repository)"
    } | visible >&2
  fi
fi
exit "$status"
