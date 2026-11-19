# graft.lib.mkRoot: a root as data (design §1, §13.1), and its `pot`.
#
#   graft.lib.mkRoot {
#     pkgs;                         # the root's nixpkgs (--package builds from it alone)
#     grantsFile ? "~/.config/pot/grants.json";   # host-only, hidden from every run (D7)
#     stateDir ? "~/.local/state/pot";             # runs, logs, caches, state: hidden
#     contexts ? { };               # added to, or changing, host inspect work dispatch (§5)
#     defaultContext ? "work";
#     kits ? { };                   # root kits, by name ("claude-code@personal", "daniel")
#     defaultInstance ? { };        # per kit, the instance chosen with no @NAME, --instance or caller match
#                                   # ({ claude-code = "personal"; })
#     agentMarkers ? [ ];           # variables that mean an agent is the caller, beyond the floor
#                                   # (CODEX_SANDBOX, CLAUDECODE) and the kits' own agent.markers
#     hidden ? [ ];                 # beyond pot's defaults (Appendix B)
#     tools ? [ ];                  # packages on every run's PATH, after the standard set
#     bindings ? { };               # bindings.<capability>."<selector>" (§7.2; carried out from milestone 2)
#     rootRepos ? [ ];              # the root's source repositories: work refuses them (D13, D23)
#     repoKits ? { };               # repo kits for repositories with no pot-kits of their own, by origin pattern
#     inspect ? { };                # inspect.home: what inspect reads in ~ (replaces [ "~/src" ])
#     caBundle ? null;              # a CA bundle (a path), at /etc/ssl/certs/ca-certificates.crt in runs
#     limits ? { };                 # the resource scope of runs without a terminal: { memoryMax = "50%"; tasksMax = 2048; }
#     clones ? { };                 # dispatch clones kept: { keepDays = 14; maxSize = "20G"; } (never unfetched commits)
#   }
#
# The result is the root's `pot` (passthru.root: the manifest; passthru.backend: the bubblewrap
# backend: bubblewrap on Linux, Seatbelt on Darwin). Kit manifests are checked when it builds (compose.py
# validate).
{ potSrc }:
let potSrc0 = potSrc; in

{ pkgs
, name ? "default"
, grantsFile ? "~/.config/pot/grants.json"
, stateDir ? "~/.local/state/pot"
, contexts ? { }
, defaultContext ? "work"
, kits ? { }
, defaultInstance ? { }
, defaultAccount ? null
, agentMarkers ? [ ]
, hidden ? [ ]
, tools ? [ ]
, bindings ? { }
, rootRepos ? [ ]
, repoKits ? { }
, inspect ? { }
, caBundle ? null
, limits ? { }
, clones ? { }
}:

let
  lib = pkgs.lib;
  # pot's programs, byte-compiled for this root's python (v2.18.0): each start (pot, its backend steps, a wrapped
  # command's) skips compiling them again; unchecked-hash, as a store path's mtime says nothing
  potSrc = pkgs.runCommand "graft-pot-compiled" { } ''
    cp -r ${potSrc0} $out && chmod -R u+w $out
    ${pkgs.python3}/bin/python3 -m compileall -q --invalidation-mode unchecked-hash $out
  '';
  kitLib = import ../lib/kit.nix { inherit lib; };
  darwin = pkgs.stdenv.hostPlatform.isDarwin;
  python = pkgs.python3;
  # defaultAccount (v2.15.0's): deprecated in v2.16.0, gone in v2.17.0; the argument stays only to say so
  defaultInstance' = if defaultAccount == null then defaultInstance
    else throw "graft.lib.mkRoot: defaultAccount is gone (v2.17.0): defaultInstance.<kit> = \"NAME\"";
  # the floor every run gets, which a root's hidden adds to and cannot remove: agents' folders and
  # credential stores, and their configuration in a workspace
  baseHidden = [ "~/.claude" "~/.claude.json" "~/.pi/agent/auth.json" "~/.pi/agent/mcp-auth.json" "~/.codex/auth.json" "~/.local/share/opencode/auth.json" ];
  baseProtect = [ "**/.claude" "**/.pi" "**/.codex" ];
  # pot grant refuses under an agent whatever kits the root has: the floor of agent markers
  baseAgentMarkers = [ "CODEX_SANDBOX" "CLAUDECODE" ];
  # every defaultInstance.<kit> names a kit the root applies under instances, and one of them
  instancesOf = base: map (n: lib.removePrefix "${base}@" n) (lib.filter (lib.hasPrefix "${base}@") (lib.attrNames kits));
  badDefaults = lib.concatLists (lib.mapAttrsToList (base: inst:
    let have = instancesOf base; in
    lib.optional (!(lib.elem inst have))
      (if have == [ ] then "defaultInstance.${base}: the root has no kit ${base}@NAME (kits: ${lib.concatStringsSep ", " (lib.attrNames kits)})"
       else "defaultInstance.${base} = \"${inst}\": the root declares ${lib.concatMapStringsSep ", " (i: "${base}@${i}") have}"))
    defaultInstance');
  limits' = { memoryMax = "50%"; tasksMax = 2048; } // limits;
  clones' = { keepDays = 14; maxSize = "20G"; } // clones;
  badLimits = lib.attrNames (removeAttrs limits [ "memoryMax" "tasksMax" ]) ++ lib.attrNames (removeAttrs clones [ "keepDays" "maxSize" ]);
  reservedContexts = [ "explain" "why" "grant" "review" "init" "run" "serve" "runs" "doctor" ];
  badContexts = lib.filter (n: lib.elem n reservedContexts) (lib.attrNames contexts);

  # §8: the standard set, first on every run's PATH (a root adds graft, or any other command, to `tools`;
  # in a graft consumer, the repo's pot-kits bin/ carries its own graft)
  standard = with pkgs; [ bashInteractive coreutils findutils gnugrep gnused gawk diffutils less gitMinimal which file
    tree ripgrep fd jq curl gnutar gzip unzip xz gnupatch python3 just gh ];
  inspectTools = with pkgs; lib.optionals (!darwin) [ procps iproute2 lsof systemdMinimal ];


  netwrap = pkgs.writeShellScript "pot-net" ''
    # inside a run with network: the proxy's socket as 127.0.0.1:3128, listening before the command
    # starts (0100007F:0C38 0A is 127.0.0.1:3128, LISTEN, in /proc/net/tcp)
    ${pkgs.socat}/bin/socat -lf /dev/null TCP-LISTEN:3128,bind=127.0.0.1,fork,reuseaddr UNIX-CONNECT:/run/pot/proxy.sock &
    listening() {
      local _ local state
      while read -r _ local _ state _; do [ "$local $state" != "0100007F:0C38 0A" ] || return 0; done </proc/net/tcp
      return 1
    }
    for _ in {1..300}; do
      ! listening || exec "$@"
      ${pkgs.coreutils}/bin/sleep 0.01
    done
    echo "pot: the network relay (socat) did not start" >&2
    exit 1
  '';
  bpf = pkgs.runCommand "pot-tiocsti.bpf" { nativeBuildInputs = [ (pkgs.python3.withPackages (p: [ p.seccomp ])) ]; } ''
    python3 ${potSrc}/tiocsti.py $out
  '';
  closureOf = n: roots: pkgs.runCommand n { } "cp ${pkgs.closureInfo { rootPaths = roots; }}/store-paths $out";
  backendConfig = pkgs.writeText "pot-bwrap.json" (builtins.toJSON {
    bwrap = "${pkgs.bubblewrap}/bin/bwrap";
    python = "${python}/bin/python3";
    proxy = "${potSrc}/proxy.py";
    broker = "${potSrc}/broker.py";
    shim = "${potSrc}/shim.py";
    socat = "${pkgs.socat}/bin/socat";
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
    baseClosure = closureOf "pot-base-closure" [ pkgs.bashInteractive pkgs.coreutils pkgs.cacert pkgs.ncurses pkgs.tzdata ];
    netClosure = closureOf "pot-net-closure" [ netwrap ];
    # fetch-back's upload-pack, sandboxed: git's closure only
    gitClosure = closureOf "pot-git-closure" [ pkgs.gitMinimal ];
  });
  backend = pkgs.writeShellScriptBin "pot-backend-bwrap" ''exec ${python}/bin/python3 -I -c 'import sys; sys.path.insert(0, "${potSrc}"); import bwrap; sys.exit(bwrap.main(sys.argv[1:]))' --config ${backendConfig} "$@"'';

  # pot inside a run: explain and why from /run/pot/policy.json, and (v2.18.0) the wrappers' entry, which nests a
  # child with the root's backend (so the backend is in every run's closure)
  inner = pkgs.writeText "pot-inner.json" (builtins.toJSON ({ inner = true; }
    // lib.optionalAttrs (!darwin) { backend = "${backend}/bin/pot-backend-bwrap"; }));
  potInner = pkgs.writeShellScriptBin "pot" ''exec ${python}/bin/python3 -I -c 'import sys; sys.path.insert(0, "${potSrc}"); import pot; pot.entry(sys.argv[1:])' ${inner} "$@"'';

  # macOS: the Seatbelt backend (seatbelt.py, /usr/bin/sandbox-exec). Its profile is written at start, from
  # policy.json; what a run reads of the store is the closure, as with bubblewrap. No netwrap (the run's
  # network is the proxy's port on the loopback) and no seccomp filter (the profile denies TIOCSTI).
  seatbeltConfig = pkgs.writeText "pot-seatbelt.json" (builtins.toJSON {
    python = "${python}/bin/python3";
    proxy = "${potSrc}/proxy.py";
    broker = "${potSrc}/broker.py";
    shim = "${potSrc}/shim.py";
    socat = "${pkgs.socat}/bin/socat";
    sh = "${pkgs.bashInteractive}/bin/sh";
    shell = "${pkgs.bashInteractive}/bin/bash";
    env = "${pkgs.coreutils}/bin/env";
    cacert = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
    terminfo = "${pkgs.ncurses}/share/terminfo";
    tzdir = "${pkgs.tzdata}/share/zoneinfo";
    fd = "${pkgs.fd}/bin/fd";
    find = "${pkgs.findutils}/bin/find";
    git = "${pkgs.gitMinimal}/bin/git";
    # the shims run pot's python, so the base closure holds it
    baseClosure = closureOf "pot-base-closure" [ pkgs.bashInteractive pkgs.coreutils pkgs.cacert pkgs.ncurses pkgs.tzdata python ];
    gitClosure = closureOf "pot-git-closure" [ pkgs.gitMinimal pkgs.coreutils ];
  });
  seatbelt = pkgs.writeShellScriptBin "pot-backend-seatbelt" ''exec ${python}/bin/python3 -I -c 'import sys; sys.path.insert(0, "${potSrc}"); import seatbelt; sys.exit(seatbelt.main(sys.argv[1:]))' --config ${seatbeltConfig} "$@"'';

  # kit.packages, tools' old name: gone at the source in v2.17.0
  noPackages = file: k: if builtins.isAttrs k && k ? packages
    then throw "pot: ${file}: packages is tools' old name, gone in v2.17.0: give tools" else k;
  kitManifests = lib.mapAttrs (n: k: let file = "the root (programs.pot.kits.${n})"; in
    kitLib.manifest { kit = noPackages file k; inherit file; }) kits;
  repoKitManifests = lib.mapAttrs (n: k: let file = "the root (programs.pot.repoKits.\"${n}\")"; in
    kitLib.manifest { kit = noPackages file k; inherit file; }) repoKits;
  kitRoots = lib.concatLists (lib.mapAttrsToList (_: k: kitLib.roots k) kits);
  closure = closureOf "pot-root-closure" (standard ++ tools ++ inspectTools ++ kitRoots ++ [ potInner ]
    ++ lib.optional (caBundle != null) caBundle);

  manifest = {
    version = 1;
    inherit name grantsFile stateDir defaultContext rootRepos bindings darwin agentMarkers baseHidden baseProtect baseAgentMarkers;
    defaultInstance = defaultInstance';
    limits = { memoryMax = toString limits'.memoryMax; tasksMax = toString limits'.tasksMax; };
    clones = { keepDays = clones'.keepDays; maxSize = toString clones'.maxSize; };
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
    potBin = "${potInner}/bin";
    shell = "bash";
    backends = if darwin then { seatbelt = { kind = "seatbelt"; program = "${seatbelt}/bin/pot-backend-seatbelt"; }; }
      else { bubblewrap = { kind = "bwrap"; program = "${backend}/bin/pot-backend-bwrap"; }; };
    defaultBackend = if darwin then "seatbelt" else "bubblewrap";
  } // lib.optionalAttrs darwin {
    # the startup line names what a Seatbelt run lacks that a Linux one has (design §12.2); explain lists every lack
    usualGuarantees = [ "own-netns" "own-pidns" ];
  };
  rootJson = pkgs.runCommand "pot-root-${name}.json" {
    nativeBuildInputs = [ python ];
    manifest = builtins.toJSON manifest;
    passAsFile = [ "manifest" ];
  } ''
    python3 -I ${potSrc}/compose.py validate-root "$manifestPath"
    cp "$manifestPath" $out
  '';
in
if badContexts != [ ] then throw "pot: mkRoot: a context may not be named like a pot verb: ${lib.concatStringsSep ", " badContexts}"
else if badLimits != [ ] then throw "graft.lib.mkRoot: limits is { memoryMax; tasksMax; } and clones { keepDays; maxSize; } (not ${lib.concatStringsSep ", " badLimits})"
else if badDefaults != [ ] then throw "graft.lib.mkRoot: ${lib.concatStringsSep "; " badDefaults}"
# share/pot/run-1 (v2.18.0): this pot takes `pot run`, the wrappers' entry; a wrapper looks for it beside the
# launcher's bin/pot, and runs its command unwrapped (with one line) under an older root's pot
else pkgs.runCommand "pot" {
  passthru = { root = manifest; inherit rootJson; backend = if darwin then seatbelt else backend; };
  meta = { description = "pot: run a command under the root's policy (${name})"; mainProgram = "pot"; };
} ''
  mkdir -p $out/bin $out/share/pot
  cat > $out/bin/pot <<'EOF'
  #!${pkgs.runtimeShell}
  exec ${python}/bin/python3 -I -c 'import sys; sys.path.insert(0, "${potSrc}"); import pot; pot.entry(sys.argv[1:])' ${rootJson} "$@"
  EOF
  chmod +x $out/bin/pot
  echo "pot run --command NAME --manifest PATH -- ARGS" > $out/share/pot/run-1

''
