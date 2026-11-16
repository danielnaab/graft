# graft.lib.mkRoot: a root as data (design §1, §13.1), and its `sb`.
#
#   graft.lib.mkRoot {
#     pkgs;                         # the root's nixpkgs (--package builds from it alone)
#     grantsFile ? "~/.config/graft/sb/grants.json";   # host-only, hidden from every run (D7)
#     stateDir ? "~/.local/state/graft/sb";             # runs, logs, caches, state: hidden
#     contexts ? { };               # added to, or changing, host inspect work dispatch (§5)
#     defaultContext ? "work";
#     kits ? { };                   # root kits, by name ("claude-code@personal", "daniel")
#     defaultAccount ? null;        # the account instance chosen with no --account
#     hidden ? [ ];                 # beyond graft's defaults (Appendix B)
#     tools ? [ ];                  # packages on every run's PATH, after the standard set
#     bindings ? { };               # bindings.<capability>."<selector>" (§7.2; carried out from milestone 2)
#     rootRepos ? [ ];              # the root's source repositories: work refuses them (D13, D23)
#     repoKits ? { };               # repo kits for repositories that are not graft consumers, by origin pattern
#     inspect ? { };                # inspect.home: what inspect reads in ~ (replaces [ "~/src" ])
#     caBundle ? null;              # a CA bundle (a path), at /etc/ssl/certs/ca-certificates.crt in runs
#   }
#
# The result is the root's `sb` (passthru.root: the manifest; passthru.backend: the bubblewrap
# backend). Kit manifests are checked when it builds (compose.py validate). On Darwin, sb refuses:
# Seatbelt returns in 1b (D21).
{ cli, sbSrc }:

{ pkgs
, name ? "default"
, grantsFile ? "~/.config/graft/sb/grants.json"
, stateDir ? "~/.local/state/graft/sb"
, contexts ? { }
, defaultContext ? "work"
, kits ? { }
, defaultAccount ? null
, hidden ? [ ]
, tools ? [ ]
, bindings ? { }
, rootRepos ? [ ]
, repoKits ? { }
, inspect ? { }
, caBundle ? null
}:

let
  lib = pkgs.lib;
  kitLib = import ./kit.nix { inherit lib; };
  darwin = pkgs.stdenv.hostPlatform.isDarwin;
  python = pkgs.python3;
  graft = cli pkgs { };
  reservedContexts = [ "explain" "why" "grant" "review" "init" "run" "serve" ];
  badContexts = lib.filter (n: lib.elem n reservedContexts) (lib.attrNames contexts);

  # §8: the standard set, first on every run's PATH
  standard = with pkgs; [ bashInteractive coreutils findutils gnugrep gnused gawk diffutils less gitMinimal which file
    tree ripgrep fd jq curl gnutar gzip unzip xz gnupatch python3 just gh graft ];
  inspectTools = with pkgs; lib.optionals (!darwin) [ procps iproute2 lsof systemdMinimal ];

  # sb inside a run: explain and why, from /run/graft-sandbox/policy.json
  inner = pkgs.writeText "sb-inner.json" (builtins.toJSON { inner = true; });
  sbInner = pkgs.writeShellScriptBin "sb" ''exec ${python}/bin/python3 -I ${sbSrc}/sb.py ${inner} "$@"'';

  netwrap = pkgs.writeShellScript "sb-net" ''
    # inside a run with network: the proxy's socket as 127.0.0.1:3128, listening before the command
    # starts (0100007F:0C38 0A is 127.0.0.1:3128, LISTEN, in /proc/net/tcp)
    ${pkgs.socat}/bin/socat -lf /dev/null TCP-LISTEN:3128,bind=127.0.0.1,fork,reuseaddr UNIX-CONNECT:/run/graft-sandbox/proxy.sock &
    listening() {
      local _ local state
      while read -r _ local _ state _; do [ "$local $state" != "0100007F:0C38 0A" ] || return 0; done </proc/net/tcp
      return 1
    }
    for _ in {1..300}; do
      ! listening || exec "$@"
      ${pkgs.coreutils}/bin/sleep 0.01
    done
    echo "graft sandbox: the network relay (socat) did not start" >&2
    exit 1
  '';
  bpf = pkgs.runCommand "sb-tiocsti.bpf" { nativeBuildInputs = [ (pkgs.python3.withPackages (p: [ p.seccomp ])) ]; } ''
    python3 ${sbSrc}/tiocsti.py $out
  '';
  closureOf = n: roots: pkgs.runCommand n { } "cp ${pkgs.closureInfo { rootPaths = roots; }}/store-paths $out";
  backendConfig = pkgs.writeText "sb-bwrap.json" (builtins.toJSON {
    bwrap = "${pkgs.bubblewrap}/bin/bwrap";
    python = "${python}/bin/python3";
    proxy = "${sbSrc}/proxy.py";
    inherit bpf netwrap;
    sh = "${pkgs.bashInteractive}/bin/sh";
    shell = "${pkgs.bashInteractive}/bin/bash";
    env = "${pkgs.coreutils}/bin/env";
    cacert = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
    terminfo = "${pkgs.ncurses}/share/terminfo";
    tzdir = "${pkgs.tzdata}/share/zoneinfo";
    fd = "${pkgs.fd}/bin/fd";
    find = "${pkgs.findutils}/bin/find";
    git = "${pkgs.gitMinimal}/bin/git";
    baseClosure = closureOf "sb-base-closure" [ pkgs.bashInteractive pkgs.coreutils pkgs.cacert pkgs.ncurses pkgs.tzdata ];
    netClosure = closureOf "sb-net-closure" [ netwrap ];
  });
  backend = pkgs.writeShellScriptBin "sb-backend-bwrap" ''exec ${python}/bin/python3 -I ${sbSrc}/bwrap.py --config ${backendConfig} "$@"'';

  kitManifests = lib.mapAttrs (n: k: kitLib.manifest { kit = k; file = "the root (programs.sb.kits.${n})"; }) kits;
  repoKitManifests = lib.mapAttrs (n: k: kitLib.manifest { kit = k; file = "the root (programs.sb.repoKits.\"${n}\")"; }) repoKits;
  kitRoots = lib.concatLists (lib.mapAttrsToList (_: k: kitLib.roots k) kits);
  closure = closureOf "sb-root-closure" (standard ++ tools ++ inspectTools ++ kitRoots ++ [ sbInner ]
    ++ lib.optional (caBundle != null) caBundle);

  manifest = {
    version = 1;
    inherit name grantsFile stateDir defaultContext defaultAccount rootRepos bindings darwin;
    contexts = contexts;
    kits = kitManifests;
    repoKits = repoKitManifests;
    hidden = hidden;
    inspect = inspect;
    standard = map toString standard;
    standardNames = lib.unique (map (p: p.meta.mainProgram or (lib.getName p)) standard);
    tools = map toString tools;
    inspectTools = map toString inspectTools;
    caBundle = if caBundle == null then null else "${caBundle}";
    system = pkgs.stdenv.hostPlatform.system;
    git = "${pkgs.gitMinimal}/bin/git";
    nixpkgs = builtins.unsafeDiscardStringContext (toString pkgs.path);
    closure = "${closure}";
    sbBin = "${sbInner}/bin";
    shell = "bash";
    backends = lib.optionalAttrs (!darwin) { bubblewrap = { kind = "bwrap"; program = "${backend}/bin/sb-backend-bwrap"; }; };
    defaultBackend = if darwin then null else "bubblewrap";
  };
  rootJson = pkgs.runCommand "sb-root-${name}.json" {
    nativeBuildInputs = [ python ];
    manifest = builtins.toJSON manifest;
    passAsFile = [ "manifest" ];
  } ''
    python3 -I ${sbSrc}/compose.py validate-root "$manifestPath"
    cp "$manifestPath" $out
  '';
in
if badContexts != [ ] then throw "graft: mkRoot: a context may not be named like an sb verb: ${lib.concatStringsSep ", " badContexts}"
else pkgs.writeShellApplication {
  name = "sb";
  text = ''exec ${python}/bin/python3 -I ${sbSrc}/sb.py ${rootJson} "$@"'';
  passthru = { root = manifest; inherit rootJson; } // lib.optionalAttrs (!darwin) { inherit backend; };
  meta.description = "graft sb: run a command under the root's policy (${name})";
}
