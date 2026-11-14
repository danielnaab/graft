# shellcheck shell=bash
# The bubblewrap backend (bwrap.nix): a run as bwrap, with its closure read-only (one --ro-bind per
# store path), the repository bound writable with each pin bound onto itself (pinDirs a mount point,
# which can't be renamed or removed: EBUSY; pinRO read-only), HOME and /tmp empty tmpfs, its own
# namespaces (no network but loopback; the proxy's socket bound in, served as 127.0.0.1:3128 by
# socat), a seccomp filter against TIOCSTI, and --die-with-parent. launcher.sh calls these.
# shellcheck disable=SC2154,SC2034 # policy.sh's and launcher.sh's variables; launcher.sh reads these

homeNote="a run gets an empty HOME, at the same path"
# the proxy listens on a unix socket in the run directory (AF_UNIX allows 108 bytes: relative)
proxyListen=proxy.sock

backend_check() { :; }
proxy_ready() { [ -S "$run/proxy.sock" ]; }
backend_cleanup() { :; }

backend_prepare() {
  runHome=$HOME lang=C.UTF-8 certFile=/etc/ssl/certs/ca-certificates.crt
  proxyUrl=http://127.0.0.1:3128 backendEnv=()
  printf '%s:x:%s:%s::%s:%s\n' "$user" "$UID" "$(id -g)" "$HOME" "$shell" >"$run/passwd"
  printf '%s:x:%s:\n' "$(id -gn)" "$(id -g)" >"$run/group"
  printf '127.0.0.1 localhost\n::1 localhost\n' >"$run/hosts"
}

# bwrap's arguments: the closure (3), the relay's closure (7) when there is network, then the rest
# and the environment (6), all read from descriptors, so that /proc/1/cmdline in the run names no
# host path, and ps no passed value
bwrap_args() {
  local p
  args=()
  for p in "${withPaths[@]}"; do args+=(--ro-bind "$p" "$p"); done
  args+=(--unshare-all --die-with-parent --proc /proc --dev /dev --tmpfs /tmp
    --ro-bind "$run/passwd" /etc/passwd --ro-bind "$run/group" /etc/group --ro-bind "$run/hosts" /etc/hosts
    --ro-bind "$cacert" /etc/ssl/certs/ca-certificates.crt
    --ro-bind "$sh" /bin/sh --ro-bind "$envbin" /usr/bin/env
    --tmpfs "$HOME")
  for p in "${readOnly[@]}"; do p=$(home "$p"); args+=(--ro-bind-try "$p" "$p"); done
  for p in "${writable[@]}"; do p=$(home "$p"); args+=(--bind-try "$p" "$p"); done
  if [ "$repoReadOnly" = 0 ]; then
    args+=(--bind "$top" "$top")
    for p in "${pinDirs[@]}"; do args+=(--bind "$p" "$p"); done
    for p in "${pinRO[@]}"; do args+=(--ro-bind "$p" "$p"); done
  else
    args+=(--ro-bind "$top" "$top")
    [ "$common" = "$top/.git" ] || args+=(--ro-bind "$common" "$common")
  fi
  [ ${#allow[@]} -eq 0 ] || args+=(--ro-bind "$run/proxy.sock" /run/graft-sandbox/proxy.sock)
  args+=(--chdir "$here" --seccomp 4 --json-status-fd 5 --clearenv)
  # with network, the relay's closure, and the relay before the command
  local net=() relay=()
  [ ${#allow[@]} -eq 0 ] || { net=(--args 7); relay=("$netwrap"); }
  cmd=("$bwrap" --args 3 "${net[@]}" --args 6 -- "$envbin" --default-signal=INT "${relay[@]}" "$@")
}

backend_dry() {
  bwrap_args "$@"
  printf '%q ' env -i "${cmd[@]}"
  printf '\n# --args 3: --ro-bind for each of %s store paths (%s)\n' "$(($(tr -cd '\0' <"$binds" | wc -c) / 3))" "$binds"
  [ ${#allow[@]} -eq 0 ] || printf '# --args 7: --ro-bind for the network relay'"'"'s closure (%s)\n' "$netBinds"
  printf '# --args 6:'
  printf ' %q' "${args[@]}"
  echo
  local v n
  for v in "${envv[@]}"; do
    n=${v%%=*}
    if [[ " ${passed[*]} " == *" $n "* ]]; then
      printf '#   --setenv %s "$%s" (env.pass)\n' "$n" "$n"
    else
      printf '#   --setenv %s %q\n' "$n" "${v#*=}"
    fi
  done
}

backend_run() {
  local argfd v
  bwrap_args "$@"
  exec {argfd}< <(
    printf '%s\0' "${args[@]}"
    for v in "${envv[@]}"; do printf -- '--setenv\0%s\0%s\0' "${v%%=*}" "${v#*=}"; done
  )
  # env -i: bwrap is PID 1 in the run, which can read /proc/1/environ. {argfd}<&- closes the pipe's
  # other descriptor.
  env -i "${cmd[@]}" 3<"$binds" 4<"$bpf" 5>"$run/status" 6<&"$argfd" 7<"$netBinds" {argfd}<&-
}

backend_after() {
  if [ "$status" -ne 0 ] && [[ $(<"$run/status") != *child-pid* ]] && ! "$bwrap" --unshare-user --ro-bind / / true 2>/dev/null; then
    echo "graft: sandbox: bubblewrap can't create a user namespace here (above). In Docker, run the container with --security-opt seccomp=unconfined --security-opt systempaths=unconfined, or run on the host or a VM; elsewhere, allow unprivileged user namespaces (sysctl user.max_user_namespaces). graft never runs it unsandboxed." >&2
  fi
}
