# The bubblewrap renderer (Linux): a policy (policy.nix) as the launcher `sandbox` (launcher.nix,
# with bwrap.sh). What is known before a run is fixed here, when the launcher builds: the closure's
# binds (from closureInfo: the run sees these store paths and no others), and the store paths of the
# CA bundle, the TIOCSTI filter and the network relay. The launcher reads the rest when it runs: the
# repository, its git metadata, the caller's identity.
{ pkgs, lib, policy, nixpkgs, nixpkgsFrom, passthru }:

let
  inherit (policy) config;
  shell = pkgs.bashInteractive;

  # inside a run with network: the proxy's socket as 127.0.0.1:3128 (socat), listening before the
  # command starts; 0100007F:0C38 0A is 127.0.0.1:3128, LISTEN, in /proc/net/tcp. Built always (an
  # --allow can ask for the network), bound into a run only when it has some.
  netwrap = pkgs.writeShellScript "sandbox-net" ''
    ${pkgs.socat}/bin/socat TCP-LISTEN:3128,bind=127.0.0.1,fork,reuseaddr UNIX-CONNECT:/run/graft-sandbox/proxy.sock &
    listening() {
      local _ local state
      while read -r _ local _ state _; do [ "$local $state" != "0100007F:0C38 0A" ] || return 0; done </proc/net/tcp
      return 1
    }
    # a non-interactive bash (this one) drops PS1
    export PS1=${lib.escapeShellArg "(sandbox) \\w \\$ "}
    for _ in {1..300}; do
      ! listening || exec "$@"
      ${pkgs.coreutils}/bin/sleep 0.01
    done
    echo "graft: sandbox: the network relay (socat) did not start" >&2
    exit 1
  '';

  # bwrap reads them with --args: one --ro-bind per store path, NUL-separated
  bindsOf = name: roots: pkgs.runCommand name { } ''
    while read -r p; do printf -- '--ro-bind\0%s\0%s\0' "$p" "$p"; done <${pkgs.closureInfo { rootPaths = roots; }}/store-paths >$out
  '';
  # the policy's packages, and what the launcher itself puts in a run: /bin/sh, /usr/bin/env (and
  # its --default-signal), the CA bundle, terminfo
  binds = bindsOf "sandbox-binds" (config.packages ++ [ shell pkgs.coreutils pkgs.cacert pkgs.ncurses ]);
  netBinds = bindsOf "sandbox-net-binds" [ netwrap ];
  bpf = pkgs.runCommand "sandbox-tiocsti.bpf" { nativeBuildInputs = [ (pkgs.python3.withPackages (p: [ p.seccomp ])) ]; } ''
    python3 ${./tiocsti.py} $out
  '';
in
import ./launcher.nix { inherit pkgs lib policy nixpkgs nixpkgsFrom passthru; } {
  renderer = "bubblewrap";
  backend = ./bwrap.sh;
  vars = ''
    bwrap=${pkgs.bubblewrap}/bin/bwrap
    binds=${binds}
    netBinds=${netBinds}
    bpf=${bpf}
    sh=${shell}/bin/sh
    cacert=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
    netwrap=${netwrap}
  '';
  always = ''
    Always (bubblewrap): the closure read-only, and no other store path; the repository around the
    current directory (read-only with repository.readOnly = true); the git metadata that says where
    the repository is and what host git runs, read-only; HOME empty; /tmp empty; no Nix daemon; the
    caller's terminal, but no typing into it (TIOCSTI); the environment, only what the policy names.
  '';
}
