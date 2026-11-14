#!/usr/bin/env bash
# The sandbox on a Mac: builds the Seatbelt launcher for a throwaway consumer of GRAFT, runs it, and
# prints PASS or FAIL for each property, then a summary (exit 1 on a failure). Needs nix and git.
#
#   tests/macos-smoke.sh [GRAFT]
#
# GRAFT is a flake reference to graft: github:danielnaab/graft/BRANCH, or path:/ABSOLUTE/PATH/TO/graft
# (default: this checkout, as a path:). The network checks reach example.com and example.org. On a
# failure, it prints the sandbox's denials from the unified log. It writes only in its own temporary
# folder, and removes it (KEEP=1 keeps it). Runs on Linux too, against bubblewrap, where the checks
# that are about macOS are skipped. Written for macOS's /bin/bash (3.2).
# shellcheck disable=SC2016 # the sandbox's bash expands them
set -uo pipefail

graft=${1:-path:$(cd "$(dirname "$0")/.." && pwd -P)}
mac=''
[ "$(uname -s)" != Darwin ] || mac=1
export NIX_CONFIG="extra-experimental-features = nix-command flakes"
if ! command -v nix >/dev/null || ! command -v git >/dev/null; then echo "macos-smoke: needs nix and git on PATH" >&2; exit 2; fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/graft-smoke.XXXXXX") || exit 2
tmp=$(cd "$tmp" && pwd -P)
cleanup() {
  if [ -n "${KEEP:-}" ]; then echo "kept: $tmp"; return; fi
  case $tmp in
    */graft-smoke.??????) chmod -R u+rwx "$tmp" 2>/dev/null; rm -rf -- "$tmp" ;;
  esac
}
trap cleanup EXIT

pass=0 fail=0 failed=''
ok() { pass=$((pass + 1)); printf 'PASS  %s\n' "$1"; }
no() {
  fail=$((fail + 1)) failed="$failed
  $1"
  printf 'FAIL  %s\n' "$1"
  [ -z "${2:-}" ] || printf '%s\n' "$2" | head -20 | sed 's/^/        /'
}
# check NAME COMMAND...: PASS when COMMAND succeeds; on FAIL, the last run's output
check() { local n=$1; shift; if "$@"; then ok "$n"; else no "$n" "${out:-}"; fi; }
info() { printf 'INFO  %s\n' "$*"; }
has() { printf '%s\n' "$out" | grep -qF -- "$1"; }

started=$(date '+%Y-%m-%d %H:%M:%S')
info "graft: $graft"
info "host: $(uname -sm)${mac:+, macOS $(sw_vers -productVersion 2>/dev/null)}; $(nix --version)"

# the consumer: a repository with a flake that grafts nothing and protects AGENTS.md
repo=$tmp/repo home=$tmp/home realhome=$HOME
mkdir -p "$repo" "$home" "$tmp/outside"
echo "decoy-home-secret" >"$home/.decoy-secret"
cat >"$repo/flake.nix" <<EOF
{
  inputs.graft.url = "$graft";
  outputs = inputs: inputs.graft.lib.mkFlake {
    inherit inputs;
    perSystem = { pkgs, ... }: { sandbox.protect = [ "AGENTS.md" ]; };
  };
}
EOF
echo "# agents" >"$repo/AGENTS.md"
g() { git -C "$repo" -c commit.gpgsign=false "$@"; }
g init -q && g config user.name smoke && g config user.email smoke@example.com
g add -A && g commit -qm init
out=$(cd "$repo" && nix flake lock 2>&1) || { no "the consumer locks graft ($graft)" "$out"; exit 1; }
g add -A && g commit -qm lock

# the launcher, built once (as a deployed one is)
out=$(cd "$repo" && nix build -o "$tmp/sbl" .#sandbox 2>&1)
check "the launcher builds" test -x "$tmp/sbl/bin/sandbox"
[ "$fail" -eq 0 ] || exit 1
# sb ARGS: the launcher in the repository, with a HOME holding a decoy secret; out, st
sb() { out=$(cd "$repo" && HOME=$home GRAFT_SANDBOX_QUIET=1 "$tmp/sbl/bin/sandbox" "$@" 2>&1); st=$?; }
sbloud() { out=$(cd "$repo" && HOME=$home "$tmp/sbl/bin/sandbox" "$@" 2>&1); st=$?; }

# the shell, the startup line, the environment
sb bash -c 'echo started'
check "the shell starts" has started
if [ "$st" -ne 0 ] && [ -n "$mac" ]; then
  info "nothing runs: the rest would fail the same way; the denials follow"
fi
sbloud true
check "the startup line" has "graft: sandbox: "
check "says no network" has "; network: none"
sb bash -c 'echo "HOME=$HOME"; echo "TMPDIR=${TMPDIR:-}"; touch "${TMPDIR:-/tmp}/t" && echo tmp-writable'
[ -z "$mac" ] || check "HOME is a new folder, not the caller's" test "$(printf '%s\n' "$out" | grep '^HOME=')" != "HOME=$home"
check "the run's temporary folder is writable" has tmp-writable
SMOKE_DECOY=decoy-launcher-env sb bash -c 'env | grep -c decoy-launcher-env'
check "a variable env.pass doesn't name: not in the run" test "$out" = 0

# reading: the home that the launcher had, the real home, other homes
sb bash -c 'cat "$1"' _ "$home/.decoy-secret"
check "a decoy home secret is unreadable" test "$st" -ne 0 -a "$(printf '%s' "$out" | grep -c decoy-home-secret)" = 0
sb bash -c 'cat ~/.decoy-secret; ls -A ~'
check "and not in the run's HOME" test "$(printf '%s' "$out" | grep -c decoy-home-secret)" = 0
sb ls "$realhome"
check "the real home can't be listed ($realhome)" test "$st" -ne 0
f=$(find "$realhome" -maxdepth 1 -type f -name '.*' 2>/dev/null | head -1)
if [ -n "$f" ]; then
  sb head -c1 "$f"
  check "a file in it can't be read (${f##*/})" test "$st" -ne 0
fi
sb ls "${realhome%/*}"
check "other homes can't be listed (${realhome%/*})" test "$st" -ne 0
if [ -n "$mac" ]; then
  sb ls "$realhome/Library/Keychains"
  check "nor the keychains" test "$st" -ne 0
  sb ls /Volumes
  check "nor /Volumes" test "$st" -ne 0
fi
ln -s "$home/.decoy-secret" "$repo/link-out" && g add link-out && g commit -qm link
sb cat link-out
check "a symbolic link in the repository to a secret: unreadable" test "$st" -ne 0 -a "$(printf '%s' "$out" | grep -c decoy-home-secret)" = 0
g rm -q link-out && g commit -qm "no link"

# writing: the repository, a commit; nothing outside it
sb bash -c 'echo new > new.md && git add new.md && git commit -qm inside && git log -1 --format=%s'
check "the repository is writable, and a commit works" has inside
check "the commit is on the host" test "$(g log -1 --format=%s)" = inside
sb bash -c 'echo x > "$1/x"' _ "$tmp/outside"
check "no write outside the repository (a sibling folder)" test ! -e "$tmp/outside/x"
sb bash -c 'echo x > "$1"' _ "/tmp/graft-smoke-$$"
check "no write to /tmp outside the run's TMPDIR" test ! -e "/tmp/graft-smoke-$$" -a ! -e "/private/tmp/graft-smoke-$$"
if [ -d "$realhome/Desktop" ]; then
  sb bash -c 'echo x > "$1"' _ "$realhome/Desktop/graft-smoke-$$"
  if [ -e "$realhome/Desktop/graft-smoke-$$" ]; then
    no "no write to ~/Desktop" "the run wrote $realhome/Desktop/graft-smoke-$$: remove it"
  else ok "no write to ~/Desktop"; fi
fi

# what the sandbox never writes
for p in flake.nix flake.lock AGENTS.md .git/config; do
  before=$(cksum <"$repo/$p")
  sb bash -c 'echo x >> "$1"' _ "$p"
  check "protected: $p" test "$st" -ne 0 -a "$(cksum <"$repo/$p")" = "$before"
done
sb bash -c 'echo "#!/bin/sh" > .git/hooks/pre-commit'
check "protected: .git/hooks (a new hook)" test ! -e "$repo/.git/hooks/pre-commit"
sb mv .git .git2
check "renaming .git fails" test -d "$repo/.git" -a ! -e "$repo/.git2"
sb mv flake.nix flake.nix.2
check "renaming flake.nix fails" test -f "$repo/flake.nix" -a ! -e "$repo/flake.nix.2"
sb git config core.fsmonitor "touch $tmp/pwned"
g status >/dev/null 2>&1
check "a planted core.fsmonitor: refused, and host git runs nothing" test ! -e "$tmp/pwned"
sb bash -c 'ln -s .git/config c && echo x >> c'
check "writing .git/config through a new symbolic link fails" test "$st" -ne 0
rm -f "$repo/c"
before=$(cksum <"$repo/.git/config")
sb bash -c 'ln .git/config hl && echo "[core]fsmonitor = touch pwned" >> hl'
if [ "$(cksum <"$repo/.git/config")" != "$before" ]; then
  no "writing .git/config through a new hard link fails" "the run changed .git/config through a hard link: Seatbelt didn't stop it"
else ok "writing .git/config through a new hard link fails"; fi
check "nor can a hard link to it be made" test ! -e "$repo/hl"
rm -f "$repo/hl"
sb bash -c 'mkdir evil && git -C evil init -q'
check "a planted nested repository is reported" has "evil/.git"

# the network
sb bash -c 'exec 3<>/dev/tcp/1.1.1.1/443 && echo connected'
check "the network is denied by default" test "$st" -ne 0 -a "$(printf '%s' "$out" | grep -c connected)" = 0
sb --with curl curl -sS -o /dev/null -w '%{http_code}' https://example.com
check "without --allow, curl can't reach example.com" test "$st" -ne 0
sb --with curl --allow example.com curl -sS -o /dev/null -w '%{http_code}\n' https://example.com
check "with --allow example.com, curl reaches it" has 200
sb --with curl --allow example.com curl -sS -o /dev/null https://example.org
check "and another host gets the proxy's 403" has 403
check "which the launcher reports" has "the proxy refused: example.org:443"

# the commands, --explain, exit statuses
sb --with jq jq -n '1 + 1'
check "--with jq" test "$st" = 0 -a "$out" = 2
sb --explain
check "--explain" has "Always ($([ -n "$mac" ] && echo Seatbelt || echo bubblewrap))"
sb nosuch
check "a command not in the closure: exit 127" test "$st" = 127
sb --bogus
check "a usage error: exit 2" test "$st" = 2
sb bash -c 'kill -INT $$'
check "a command that dies of SIGINT: exit 130 (its INT is the default again)" test "$st" = 130
sb bash -c 'exit 7'
check "the command's exit status" test "$st" = 7

# macOS alone: processes (the launcher's, launchd's, the run's after it), sockets, TIOCSTI
if [ -n "$mac" ]; then
  # a process of the caller's, outside the run, with the decoy in its environment
  SMOKE_DECOY=decoy-launcher-env sleep 60 & other=$!
  SMOKE_DECOY=decoy-launcher-env sb --with python3 python3 -c '
import ctypes, os, sys
libc = ctypes.CDLL(None, use_errno=True)
for name, pid in (("launcher", os.getppid()), ("pid-1", 1), ("other", int(sys.argv[1]))):
    mib = (ctypes.c_int * 3)(1, 49, pid)  # CTL_KERN, KERN_PROCARGS2
    size = ctypes.c_size_t(1 << 20)
    buf = ctypes.create_string_buffer(size.value)
    if libc.sysctl(mib, 3, buf, ctypes.byref(size), None, 0) != 0:
        print(name, "denied", ctypes.get_errno())
    else:
        print(name, "read", b"decoy-launcher-env" in buf.raw[:size.value])
    try:
        os.kill(pid, 0); print(name, "signal sent")
    except OSError as e:
        print(name, "signal refused", e.errno)
' "$other"
  kill "$other" 2>/dev/null
  # a known limit, documented (reference/sandbox.md, On macOS): Seatbelt doesn't check KERN_PROCARGS2,
  # so the run reads the first environment of the user's processes; only the kernel's same-user check
  # applies. Reported, not failed; a FAIL if a macOS starts denying it, so the docs get updated.
  if has "launcher read True" && has "other read True"; then
    info "known limit: the run reads the environment of the user's processes (KERN_PROCARGS2), the launcher's and others'"
  else no "the known KERN_PROCARGS2 limit changed: update On macOS in docs/reference/sandbox.md" "$out"; fi
  check "another user's process (launchd) can't be read" has "pid-1 denied"
  check "no signal to the launcher" has "launcher signal refused"
  check "nor to another process of the user's" has "other signal refused"
  sb bash -c 'sleep 60 >/dev/null 2>&1 & echo $!'
  left=$(printf '%s\n' "$out" | tail -1)
  sleep 0.5
  if [ -n "$left" ] && kill -0 "$left" 2>/dev/null; then
    no "a background process doesn't outlive the run" "pid $left still runs"; kill "$left" 2>/dev/null
  else ok "a background process doesn't outlive the run"; fi
  sb --with python3 python3 -c '
import socket
a, b = socket.socketpair(); print("socketpair ok")
for p in ("/private/var/run/mDNSResponder", "/private/var/run/syslog"):
    s = socket.socket(socket.AF_UNIX)
    try:
        s.connect(p); print("connected", p)
    except OSError as e:
        print("refused", p, e.errno)
'
  check "socketpair works (Node's pipes)" has "socketpair ok"
  check "a host service's socket is unreachable (mDNSResponder)" has "refused /private/var/run/mDNSResponder"
  # an agent's tool call: stdin a socket, where bash reads the host's /etc/bashrc below SHLVL 2
  out=$(cd "$repo" && HOME=$home GRAFT_SANDBOX_QUIET=1 /usr/bin/perl -MSocket -e \
    'socketpair(my $a, my $b, AF_UNIX, SOCK_STREAM, 0) or die; open(STDIN, "<&", $a) or die; exec @ARGV' \
    "$tmp/sbl/bin/sandbox" bash -c 'echo ran' 2>&1)
  check "bash with a socket on stdin: runs, and no denial of the host's /etc/bashrc" test "$out" = ran
  if command -v script >/dev/null; then
    out=$(cd "$repo" && HOME=$home GRAFT_SANDBOX_QUIET=1 script -q /dev/null "$tmp/sbl/bin/sandbox" --with python3 python3 -c '
import fcntl, termios
try:
    fcntl.ioctl(0, termios.TIOCSTI, b"x"); print("typed")
except OSError as e:
    print("refused", e.errno)
' </dev/null 2>&1)
    check "TIOCSTI on the caller's terminal is refused" has refused
  fi
fi

# startup time
TIMEFORMAT=%3R
times=''
for _ in 1 2 3 4 5; do
  t=$( { time (cd "$repo" && HOME=$home GRAFT_SANDBOX_QUIET=1 "$tmp/sbl/bin/sandbox" true >/dev/null 2>&1); } 2>&1)
  times="$times $t"
done
info "startup, \`sandbox true\`, 5 runs (s):$times"

# probes for the lead: which SBPL operations this macOS knows
if [ -n "$mac" ]; then
  for op in file-link file-map-executable; do
    if /usr/bin/sandbox-exec -p "(version 1)(allow default)(deny $op (literal \"/graft-smoke-none\"))" /usr/bin/true 2>/dev/null; then
      info "SBPL knows $op"
    else info "SBPL doesn't know $op"; fi
  done
fi

echo
echo "$pass passed, $fail failed"
if [ "$fail" -gt 0 ]; then
  echo "failed:$failed"
  if [ -n "$mac" ]; then
    echo
    echo "the sandbox's denials since $started (log show):"
    log show --start "$started" --style compact --predicate 'sender == "Sandbox" OR eventMessage CONTAINS "deny("' 2>/dev/null \
      | grep -i 'deny' | sed 's/^/  /' | tail -80
  fi
  exit 1
fi
