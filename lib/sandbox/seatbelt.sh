# shellcheck shell=bash
# The Seatbelt backend (seatbelt.nix), for macOS: a run as /usr/bin/sandbox-exec with a profile
# that denies by default. The profile is three parts: seatbelt.sb (what a process needs from macOS),
# the closure (sb_store, when the launcher builds), and the run (sb_profile, at each run). Seatbelt
# has no mount namespace, so the same pins are path rules on resolved paths: pinRO, file-write*
# denied; pinDirs, unlink and create denied (rename needs both), so none can be renamed or replaced.
# There is no network namespace either: with network, the proxy listens on 127.0.0.1 at a free port,
# the one address the run may connect to. HOME and TMPDIR are new empty folders in the run
# directory; `~/` entries of paths.* appear in that HOME as symbolic links to the real ones.
#
# This file holds functions and settings only: seatbelt.nix sources it to build the closure's
# rules, launcher.sh calls its backend_ functions, and tests/run.sh tests sb_store and sb_profile.
# Every path that a run's repository or its caller names reaches the profile as a parameter
# (sandbox-exec -DPn=PATH, (param "Pn")), never as text in it, so no name can add a rule.
# shellcheck disable=SC2154,SC2034 # policy.sh's and launcher.sh's variables; launcher.sh reads these

homeNote="the launcher needs it for paths.* entries that start with ~/"
proxyListen=tcp:127.0.0.1:0 proxyPort=''
# TIOCSTI, _IOW('t', 114, char)
sbTiocsti=2147579026

# sb_rule "allow|deny OPERATIONS" FILTER...: one rule; for no filter, nothing (a rule without a
# filter matches everything)
sb_rule() {
  [ $# -gt 1 ] || return 0
  printf '(%s' "$1"
  shift
  printf '\n  %s' "$@"
  printf ')\n'
}

# sb_store < STORE-PATHS: read, and run, these store paths (a closure's, from closureInfo or nix
# path-info). They are written in the profile: a store path holds no character to escape.
sb_store() {
  local p fs=()
  while read -r p; do
    [[ $p =~ ^/nix/store/[0-9a-z]{32}-[A-Za-z0-9+._?=-]+$ ]] || { echo "graft: sandbox: not a store path: $p" >&2; return 1; }
    fs+=("(subpath \"$p\")")
  done
  sb_rule "allow file-read* process-exec" "${fs[@]}"
}

# sb_param ARRAY KIND PATH: add to ARRAY the filter (KIND (param "Pn")), and PATH to sbParams
# (once: sb_profile's sbKey holds each path's parameter)
sb_param() {
  local -n into=$1
  [ -n "${sbKey[$3]:-}" ] || { sbKey[$3]=P${#sbParams[@]}; sbParams+=("-D${sbKey[$3]}=$3"); }
  into+=("($2 (param \"${sbKey[$3]}\"))")
}

# sb_profile > FILE: the run's part of the profile, from
#   sbRead   paths read (and run): paths.readOnly; the repository when it is read-only
#   sbWrite  paths read and written: the repository, paths.writable, the run's HOME and TMPDIR
#   sbTty    the caller's terminal devices
#   withPaths  --with's store paths
#   pinRO, pinDirs  launcher.sh's pins
#   proxyPort  the proxy's port, or empty: no network
# and sbParams, its parameters (it runs in the current shell: call it with a redirection).
sb_profile() {
  local p d read=() write=() bind=() connect=() meta=() tty=() ro=() keep=()
  local -A up=() sbKey=()
  sbParams=()
  echo "; the run (graft's sandbox launcher)"
  [ ${#withPaths[@]} -eq 0 ] || printf '%s\n' "${withPaths[@]}" | sb_store
  for p in "${sbRead[@]}"; do sb_param read subpath "$p"; done
  for p in "${sbWrite[@]}"; do
    sb_param write subpath "$p"
    # a socket below a writable path: the run's own (git's fsmonitor daemon, a test server, ssh's
    # ControlMaster); bound into a run, a host's socket there is reachable on Linux too
    bind+=("(local unix-socket ${write[-1]})")
    connect+=("(remote unix-socket ${write[-1]})")
    # each writable path can't be renamed or replaced, as each is a mount point with bubblewrap
    sb_param keep literal "$p"
  done
  # their parents: stat, for realpath(3) and getcwd(3), but no listing
  for p in "${sbRead[@]}" "${sbWrite[@]}"; do
    d=$p
    while d=${d%/*} && [ -n "$d" ]; do up[$d]=1; done
  done
  for d in "${!up[@]}"; do sb_param meta literal "$d"; done
  for p in "${sbTty[@]}"; do sb_param tty literal "$p"; done
  sb_rule "allow file-read* process-exec" "${read[@]}"
  sb_rule "allow file-read* file-write* process-exec" "${write[@]}"
  sb_rule "allow network-bind" "${bind[@]}"
  sb_rule "allow network-outbound" "${connect[@]}"
  sb_rule "allow file-read-metadata" "${meta[@]}"
  sb_rule "allow file-read* file-write-data file-ioctl" "${tty[@]}"
  [ -z "$proxyPort" ] || sb_rule "allow network-outbound" "(remote ip \"localhost:$proxyPort\")"
  echo "; last: what no rule above may reopen"
  for p in "${pinRO[@]}"; do sb_param ro subpath "$p"; done
  for p in "${pinDirs[@]}"; do sb_param keep literal "$p"; done
  sb_rule "deny file-write*" "${ro[@]}"
  sb_rule "deny file-write-unlink file-write-create" "${keep[@]}"
  # F_MAKECOMPRESSED and F_TRANSFEREXTENTS change a file through a read-only descriptor (codex's policy)
  sb_rule "deny system-fcntl" "(fcntl-command 80 110)"
  sb_rule "deny file-ioctl" "(ioctl-command $sbTiocsti)"
}

backend_check() {
  [ -x /usr/bin/sandbox-exec ] || die "/usr/bin/sandbox-exec is missing (macOS's Seatbelt), so the sandbox can't start. graft never runs it unsandboxed."
}

# the proxy's first line: LISTENING PORT
proxy_ready() {
  local w p
  [ -s "$run/proxy.log" ] || return 1
  read -r w p <"$run/proxy.log" || true
  [ "$w" = LISTENING ] && [[ $p =~ ^[0-9]+$ ]] && proxyPort=$p
}

# sb_host ARRAY PATH...: each host path of paths.* that exists, resolved, into ARRAY; a ~/ entry
# also where a program looks for it, in the run's HOME, as a symbolic link to it
sb_host() {
  local -n to=$1
  local p r l
  for p in "${@:2}"; do
    r=$(home "$p")
    [ -e "$r" ] || continue
    l=$runHome/${p#\~/}
    # shellcheck disable=SC2088 # the policy's ~/, not the shell's
    if [[ $p == '~/'* ]] && ! [ -e "$l" ] && ! [ -L "$l" ]; then
      mkdir -p "$(dirname "$l")"
      ln -s "$r" "$l"
    fi
    to+=("$(realpath -- "$r")")
  done
}

backend_prepare() {
  local p t fd
  runHome=$run/home lang=en_US.UTF-8 certFile=$cacert
  backendEnv=(TMPDIR="$run/tmp")
  # the proxy's port (proxy_ready); none without network; a placeholder for --dry-run
  if [ ${#allow[@]} -eq 0 ]; then proxyPort=''; elif [ -n "$dry" ]; then proxyPort=PORT; fi
  proxyUrl=http://127.0.0.1:$proxyPort
  mkdir "$run/home" "$run/tmp"
  sbRead=() sbWrite=("$run/home" "$run/tmp") sbTty=()
  sb_host sbRead "${readOnly[@]}"
  sb_host sbWrite "${writable[@]}"
  if [ "$repoReadOnly" = 0 ]; then
    sbWrite+=("$top")
    [[ $common == "$top"/* ]] || sbWrite+=("$common")
    # a file in a pinned folder with another hard link: as for a pinned file (launcher.sh)
    while IFS= read -r -d '' p; do
      die "$(rel "$p") has another hard link, so a run could write it through that link (a run must not write what decides the sandbox or what the host runs): replace it with a copy of its own (cp -p F F.copy && mv F.copy F), then run again"
    done < <([ ${#pinRO[@]} -eq 0 ] || find "${pinRO[@]}" -mindepth 1 -type f -links +1 -print0)
  else
    sbRead+=("$top")
    [[ $common == "$top"/* ]] || sbRead+=("$common")
  fi
  # the caller's terminal, by its device's name (/dev/tty is the controlling one)
  for fd in 0 1 2; do
    [ -t "$fd" ] || continue
    { t=$(tty 0<&3); } 3<&"$fd" || continue
    [[ " ${sbTty[*]} " == *" $t "* ]] || sbTty+=("$t")
  done
  { cat "$profile"; sb_profile; } >"$run/profile.sb"
}

backend_dry() {
  local v n
  printf '%s' 'env -i'
  for v in "${envv[@]}"; do
    n=${v%%=*}
    if [[ " ${passed[*]} " == *" $n "* ]]; then printf ' %s="$%s"' "$n" "$n"; else printf ' %s=%q' "$n" "${v#*=}"; fi
  done
  printf ' %q' /usr/bin/sandbox-exec -f "$run/profile.sb" "${sbParams[@]}" "$envbin" --default-signal=INT "$@"
  printf '\n# the profile: %s (macOS and the closure), then:\n' "$profile"
  sed -n '/^; the run/,$p' "$run/profile.sb" | sed 's/^/#   /'
}

# env -i, then the run's environment: the launcher's own isn't the run's. (sandbox-exec and env see
# the values in their arguments until they exec; on macOS only the caller's user can read them.)
backend_run() {
  ( cd "$here" && exec env -i "${envv[@]}" /usr/bin/sandbox-exec -f "$run/profile.sb" "${sbParams[@]}" "$envbin" --default-signal=INT "$@" )
}

# processes of the run still there (a command's background job, a daemon): stopped, as the end of
# bubblewrap's PID namespace stops them. Found by the run's TMPDIR in their environment (ps -E):
# a process that drops it, and leaves its process group, outlives the run.
sb_leftovers() {
  /bin/ps -A -E -ww -o pid= -o command= 2>/dev/null | r=$run/tmp awk 'index($0, " TMPDIR=" ENVIRON["r"]) { print $1 }'
}
sb_stop() {
  local pids
  pids=$(sb_leftovers) || return 0
  [ -n "$pids" ] || return 0
  # shellcheck disable=SC2086 # one PID per word
  kill -TERM $pids 2>/dev/null || true
  sleep 0.2
  pids=$(sb_leftovers) || return 0
  # shellcheck disable=SC2086
  [ -z "$pids" ] || kill -KILL $pids 2>/dev/null || true
}

backend_after() {
  local f
  sb_stop
  sbStopped=1
  [ "$repoReadOnly" = 0 ] || return 0
  # Seatbelt matches the path a write names; a hard link the run made to a protected file would be
  # another path to it
  while IFS= read -r -d '' f; do
    echo "graft: sandbox: WARNING: $(rel "$f") is protected and now has another hard link: the run may have written it through that link; read it, then replace it with a copy of its own (cp -p F F.copy && mv F.copy F)" | visible >&2
  done < <([ ${#pinRO[@]} -eq 0 ] || find "${pinRO[@]}" -type f -links +1 -print0)
}

# (the run can't replace HOME or TMPDIR, only what they hold: a folder it made unwritable too)
backend_cleanup() {
  [ -d "${run:?}/home" ] || return 0
  [ -n "${sbStopped:-}" ] || sb_stop
  chmod -R u+rwx -- "${run:?}/home" "${run:?}/tmp" 2>/dev/null || true
  rm -rf -- "${run:?}/home" "${run:?}/tmp" || true
}
