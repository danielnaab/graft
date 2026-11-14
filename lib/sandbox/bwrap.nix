# The bubblewrap renderer: a policy (policy.nix) as the launcher `sandbox` (launcher.sh). What is
# known before a run is fixed here, when the launcher builds: the closure's binds (from closureInfo:
# the run sees these store paths and no others), PATH, and the store paths of the CA bundle,
# terminfo, the proxy and the TIOCSTI filter. The launcher reads the rest when it runs: the
# repository, its git metadata, the caller's identity.
{ pkgs, lib, policy, nixpkgs, nixpkgsFrom, passthru }:

let
  inherit (policy) config;
  shell = pkgs.bashInteractive;
  # the prompt of a shell in a run
  ps1 = "(sandbox) \\w \\$ ";

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
    export PS1=${lib.escapeShellArg ps1}
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

  # what this renderer guarantees, after the policy in --explain
  explainText = pkgs.writeText "sandbox-explain.txt" (policy.explainText + ''

    Always (bubblewrap): the closure read-only, and no other store path; the repository around the
    current directory (read-only with repository.readOnly = true); the git metadata that says where
    the repository is and what host git runs, read-only; HOME empty; /tmp empty; no Nix daemon; the
    caller's terminal, but no typing into it (TIOCSTI); the environment, only what the policy names.
  '');
  arr = name: xs: "${name}=(${lib.concatMapStringsSep " " lib.escapeShellArg xs})";
  allow = config.network.allow;
  policySh = pkgs.writeText "sandbox-policy.sh" ''
    # shellcheck shell=bash disable=SC2034
    bwrap=${pkgs.bubblewrap}/bin/bwrap
    binds=${binds}
    netBinds=${netBinds}
    bpf=${bpf}
    shell=${shell}/bin/bash
    sh=${shell}/bin/sh
    envbin=${pkgs.coreutils}/bin/env
    cacert=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
    terminfo=${pkgs.ncurses}/share/terminfo
    path=${lib.escapeShellArg (lib.makeBinPath config.packages)}
    netwrap=${netwrap}
    ps1=${lib.escapeShellArg ps1}
    # the proxy runs on the host, isolated (-I -S): python3Minimal has no bytecode, and takes 0.2 s
    ${arr "proxy" [ "${pkgs.python3}/bin/python3" "-I" "-S" "${./proxy.py}" ]}
    # nixpkgs for --with, while the store holds it (no dependency: a deployed launcher needn't carry it)
    nixpkgs=${lib.escapeShellArg (builtins.unsafeDiscardStringContext "${nixpkgs}")}
    nixpkgsFrom=${lib.escapeShellArg nixpkgsFrom}
    explainText=${explainText}
    explainJson=${pkgs.writeText "sandbox-explain.json" (builtins.toJSON policy.explain)}
    repoReadOnly=${if config.repository.readOnly then "1" else "0"}
    ${arr "protect" config.protect}
    ${arr "readOnly" config.paths.readOnly}
    ${arr "writable" config.paths.writable}
    # host:port pairs, for the proxy; and as the policy names them, for people
    ${arr "allow" (lib.concatMap (a: map (p: "${a.host}:${toString p}") a.ports) allow)}
    ${arr "allowShown" (map (a: if a.ports == [ 443 80 ] then a.host else "${a.host}:${toString (lib.head a.ports)}") allow)}
    ${arr "pass" config.env.pass}
    ${arr "envSet" (lib.mapAttrsToList (k: v: "${k}=${v}") config.env.set)}
    notGranted=${toString (lib.length policy.notGranted)}
  '';
in
pkgs.writeShellApplication {
  name = "sandbox";
  runtimeInputs = with pkgs; [ coreutils findutils gawk git jq ];
  text = ''
    # shellcheck source=/dev/null
    source ${policySh}
    # shellcheck source=/dev/null
    source ${./launcher.sh}
  '';
  inherit passthru;
  meta.description = "Run a command in the repository's sandbox (bubblewrap)";
}
