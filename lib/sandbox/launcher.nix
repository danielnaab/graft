# The launcher `sandbox`, for a renderer: what every renderer's launcher holds (the policy as shell
# variables, the explain files, the proxy, the shared runtime launcher.sh), and the renderer's own
# part: its backend script (bwrap.sh, seatbelt.sh), which turns a run into a command, and its
# variables. bwrap.nix and seatbelt.nix call it.
{ pkgs, lib, policy, nixpkgs, nixpkgsFrom, passthru }:
{ renderer, backend, vars, always, runtimeInputs ? [ ] }:

let
  inherit (policy) config;
  arr = name: xs: "${name}=(${lib.concatMapStringsSep " " lib.escapeShellArg xs})";
  allow = config.network.allow;
  # what this renderer guarantees, after the policy in --explain
  explainText = pkgs.writeText "sandbox-explain.txt" (policy.explainText + "\n" + always);
  policySh = pkgs.writeText "sandbox-policy.sh" ''
    # shellcheck shell=bash disable=SC2034
    renderer=${lib.escapeShellArg renderer}
    shell=${pkgs.bashInteractive}/bin/bash
    envbin=${pkgs.coreutils}/bin/env
    terminfo=${pkgs.ncurses}/share/terminfo
    path=${lib.escapeShellArg (lib.makeBinPath config.packages)}
    # the prompt of a shell in a run
    ps1=${lib.escapeShellArg "(sandbox) \\w \\$ "}
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
    # the renderer's
    ${vars}
  '';
in
pkgs.writeShellApplication {
  name = "sandbox";
  runtimeInputs = with pkgs; [ coreutils findutils gawk git jq ] ++ runtimeInputs;
  text = ''
    # shellcheck source=/dev/null
    source ${policySh}
    # shellcheck source=/dev/null
    source ${backend}
    # shellcheck source=/dev/null
    source ${./launcher.sh}
  '';
  inherit passthru;
  meta.description = "Run a command in the repository's sandbox (${renderer})";
}
