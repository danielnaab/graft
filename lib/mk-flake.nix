# mkFlake: a repo's flake outputs, wired from its scions.
#
# A scion is a git submodule in the scions directory (`dir`, beside flake.nix): another repository,
# checked out at a release tag. The gitlink (the commit the repo records) is the pin and .gitmodules
# the source; flake.lock pins graft alone. Nix reads the scions through `inputs.self.submodules =
# true` (a git flake: Nix fetches each at its gitlink) or from the working tree (a `path:` flake).
#
# A scion whose repository has a `graft.nix` at its top is a provider: graft.nix is a function
#
#   { pkgs, lib, system, scion, scions, consumer, ... }:
#   { requires ? {}; tools ? {}; apps ? {}; packages ? {}; checks ? {}; kit ? {}; ... }
#
# called once per system, with the repo's own `inputs.nixpkgs` (else graft's) as pkgs; scion is this
# scion alone, copied to the store without its .git file; scions.<name> is every scion (its store
# copy as outPath, merged with that provider's result); consumer is the repo's flake source; and,
# when the function names it, kitArgs: the consumer's perSystem.kitArgs.<scion> (default { }). The
# repo's own `perSystem` ({ pkgs, lib, system, scions, ... }: { tools; apps; packages; checks;
# devShells; kit; kitArgs; }) is merged the same way, named `self` in messages. mkFlake merges:
#
#   tools      one pkgs.buildEnv, packages.<system>.tools; a name two give is an error, unless both
#              are the same store path
#   apps packages
#              by name; a duplicate is an error, and so are apps.graft, packages.tools and
#              packages.graft-kits (graft's), and the reserved names sandbox and pot
#   checks     checks.<system>.<scion>-<name>; perSystem's as checks.<system>.<name>
#   devShells  perSystem's only; devShells.<system>.default, when none is defined: a shell with the
#              tools
#   requires   { name = "url"; }: siblings the provider needs; a missing one is an error naming
#              `graft add url`
#   kit        each provider's (a scion kit) and perSystem's (the repo kit), as data (lib/pot/kit.nix):
#              packages.<system>.graft-kits, which pot builds with path: and reads: kit.json (the
#              repo kit and each provider's), bin/ (the tools, graft's CLI, the repo kit's packages)
#              and closure. graft v2.14.0's `sandbox` is an error naming `kit`.
#
# plus apps.<system>.graft, the graft command (lib/graft.sh). The repo's own `outputs` are merged
# last (per system, per name), so the repo can add or override; apps.<system>.graft and
# packages.<system>.graft-kits are graft's, and a repo's own is an error raised by graft's own value;
# sandbox and pot are reserved names in apps and packages.
#
# Every check on the scions (the marker README, .gitmodules against what the source holds, names,
# requires, collisions) is an error inside the values it guards, never on an attribute set's names,
# and the names a provider gives are read so that a failing provider gives none (builtins.tryEval):
# apps.<system>.graft (`graft sync` fills empty scions) and the repo's own outputs evaluate
# whatever state the scions are in; graft-kits, like the tools, fails on a failing provider. An
# empty scion (a path: flake's fresh clone; a private one left uninitialised) fails only what uses
# it (scions.<name>); the tools leave its own out, with a warning.
{ nixpkgs, systems, cli, potSrc }:

{ inputs, dir ? "scions", perSystem ? null, outputs ? { } }:

let
  lib = nixpkgs.lib;
  src = inputs.self;
  # the first line of the scions README (lib/graft.sh writes it); also lib/common.sh's MARKER
  marker = "<!-- managed by graft: https://github.com/danielnaab/graft -->";

  validDir = builtins.isString dir && builtins.match "[A-Za-z0-9._-]+" dir != null
    && dir != "." && dir != ".." && dir != ".git";
  validName = n: builtins.match "[A-Za-z0-9][A-Za-z0-9._-]*" n != null && n != "self";
  root = "${src}/${dir}";
  hasRoot = validDir && builtins.pathExists root;
  readme = "${root}/README.md";
  readmeText = if hasRoot && builtins.pathExists readme then builtins.readFile readme else "";
  hasMarker = lib.hasInfix marker readmeText;
  # what graft v2.12 and earlier wrote: a table of copied scions
  copyMode = hasRoot && (lib.hasInfix "| Scion | Commit |" readmeText || lib.hasInfix "<!-- graft flake: " readmeText);

  entries = if hasRoot && !copyMode then builtins.readDir root else { };
  dirs = lib.attrNames (lib.filterAttrs (_: t: t == "directory") entries);
  names = lib.filter validName dirs;
  badNames = lib.filter (n: !validName n) dirs;
  contents = n: builtins.readDir "${root}/${n}";
  # a submodule Nix didn't fetch (a path: flake's fresh clone) is an empty directory in the source
  empty = lib.filter (n: contents n == { }) names;
  isEmpty = n: lib.elem n empty;

  # .gitmodules, when the source holds it (a flake at the repo's top): the submodules registered
  # in the scions directory, one level down
  gitmodules = "${src}/.gitmodules";
  hasGitmodules = builtins.pathExists gitmodules;
  registered = if !(validDir && hasGitmodules) then [ ] else
    lib.filter (p: p != null) (map (l:
      let m = builtins.match "[[:space:]]*path[[:space:]]*=[[:space:]]*(.*[^[:space:]])[[:space:]]*" l;
          p = if m == null then "" else lib.head m;
      in if lib.hasPrefix "${dir}/" p && !(lib.hasInfix "/" (lib.removePrefix "${dir}/" p))
         then lib.removePrefix "${dir}/" p else null)
      (lib.splitString "\n" (builtins.readFile gitmodules)));
  absent = lib.sort lib.lessThan (lib.filter (n: !(lib.elem n dirs)) registered);
  # a folder that isn't a submodule: not registered (at the top), or (in a subdirectory flake, whose
  # source has no .gitmodules) holding files but no .git, which every checked-out submodule has
  unregistered = if hasGitmodules then lib.filter (n: !(lib.elem n registered)) names
    else lib.filter (n: !(isEmpty n) && !((contents n) ? ".git")) names;

  # folders at the top of the flake's source holding graft's README: one that isn't dir means dir is wrong
  markerDirs = if !validDir then [ ] else lib.filter (d:
      let f = "${src}/${d}/README.md"; in builtins.pathExists f && lib.hasInfix marker (builtins.readFile f))
    (lib.attrNames (lib.filterAttrs (_: t: t == "directory") (builtins.readDir src)));
  wrongDir = markerDirs != [ ] && !(lib.elem dir markerDirs);

  paths = ns: lib.concatMapStringsSep ", " (n: "${dir}/${n}") ns;
  github = ". Fetched with github:? use git+https://…, which keeps submodules.";
  problems =
    if !validDir then [ "graft: mkFlake's dir must be one folder name (letters, digits, . _ -; not ., .. or .git), got \"${toString dir}\"" ]
    else if copyMode then [ "graft: ${dir}/README.md is graft's copy-mode README (v2.12 or earlier): graft v2.13.0 pins scions with git submodules; follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md" ]
    else if wrongDir then [ "graft: graft's README is in ${lib.head markerDirs}/ but mkFlake's dir is \"${dir}\": set dir = \"${lib.head markerDirs}\"; (or move the scions)" ]
    else lib.optional (hasRoot && !hasMarker)
        "graft: ${dir}/README.md lacks graft's marker: is dir right? (mkFlake's dir = \"${dir}\")"
      ++ lib.optional (absent != [ ])
        "graft: .gitmodules registers ${paths absent} but the flake's source lacks ${if lib.length absent == 1 then "it" else "them"}: add inputs.self.submodules = true; to flake.nix (Nix >= 2.27)${github}"
      ++ map (n: "graft: ${dir}/${n} is not a submodule: graft add URL, or remove it") unregistered
      ++ map (n: "graft: ${dir}/${n}: a scion's folder name must be letters, digits, . _ and - (starting with a letter or digit), and not self") badNames;
  # every value that reads the scions goes through this
  ok = x: if problems == [ ] then x else throw (lib.concatStringsSep "\n" problems);
  emptyMsg = ns: why: "graft: ${paths ns} ${if lib.length ns == 1 then "is" else "are"} empty in the flake's source${why}: run git submodule update --init --recursive (or graft sync)${lib.optionalString hasGitmodules github}";

  providers = lib.filter (n: !(isEmpty n) && builtins.pathExists "${root}/${n}/graft.nix") names;
  # the units merged: each provider, then the repo's own perSystem as `self`
  units = providers ++ lib.optional (perSystem != null) "self";
  # each scion alone, without its .git file: a tool built from it doesn't change when the repo
  # does (interpolating "${src}/${dir}/x" would make every repo commit a new tool)
  copies = lib.genAttrs names (n: builtins.path {
    path = "${root}/${n}";
    name = "scion-${n}";
    filter = p: _: baseNameOf p != ".git";
  });

  forSystem = system:
    let
      pkgs = (inputs.nixpkgs or nixpkgs).legacyPackages.${system};
      args = { inherit pkgs system; lib = pkgs.lib; scions = siblings; };
      # an error inside a provider names it
      ctx = n: x: if n == "self" then x
        else builtins.addErrorContext "while evaluating ${dir}/${n}/graft.nix (graft calls it for the scion ${n})" x;
      # kitArgs: the consumer's perSystem.kitArgs.<scion>, for a provider whose function names it
      kitArgsOf = n: (raw.self.kitArgs or { }).${n} or { };
      raw = lib.genAttrs providers (n: ctx n (
        let f = import "${copies.${n}}/graft.nix"; in
        f (args // { scion = copies.${n}; consumer = inputs.self; }
          // lib.optionalAttrs ((builtins.functionArgs f) ? kitArgs) { kitArgs = kitArgsOf n; })))
        // lib.optionalAttrs (perSystem != null) { self = perSystem args; };
      # a provider's result, its requires asserted before anything else of it is used
      results = lib.mapAttrs (n: r:
        let
          req = if n == "self" then { } else r.requires or { };
          missing = lib.filter (m: !(lib.elem m names)) (lib.attrNames req);
        in
        if missing == [ ] then r else throw (lib.concatMapStringsSep "\n" (m:
          "graft: ${n} needs ${m}, which this repository does not graft: graft add ${toString req.${m}}") missing))
        raw;
      siblings = lib.genAttrs names (n:
        if isEmpty n then throw (emptyMsg [ n ] "")
        else { outPath = copies.${n}; } // (results.${n} or { }));

      # the names a unit gives for KIND, read from its raw result: a provider that fails fails them,
      # loudly (never dropped). keysOf, for apps and packages alone, gives none instead, so that
      # apps.graft (the repair path) and the repo's own outputs still evaluate; the tools and the
      # checks read every provider with keysStrict, so a failing one fails them, naming it
      keysStrict = n: kind: ctx n (lib.attrNames (raw.${n}.${kind} or { }));
      keysOf = n: kind:
        let t = builtins.tryEval (let ks = keysStrict n kind; in builtins.deepSeq ks ks);
        in if t.success then t.value else [ ];
      # KIND's names from every unit, each with the units that give it
      gather = keys: kind: lib.foldl' (acc: n:
        acc // lib.genAttrs (keys n kind) (k: (acc.${k} or [ ]) ++ [ n ])) { } units;
      who = n: if n == "self" then "the consumer's perSystem" else "scion ${n}";
      merge = keys: kind: lib.mapAttrs (k: from:
        let
          v = n: ctx n results.${n}.${kind}.${k};
          # the same store path from two units (both pass on one tool) is one tool
          same = kind == "tools" && lib.length (lib.unique (map (n: toString (v n)) from)) == 1;
        in
        if lib.length from > 1 && !same
        then throw (if lib.elem "self" from
          then "graft: ${kind} ${k} (from ${lib.concatMapStringsSep " and " who from}): both provide the same name; rename the consumer's own"
          else "graft: ${kind} ${k} (from ${lib.concatStringsSep " and " from}): two scions provide the same name")
        else ok (v (lib.head from))) (gather keys kind);

      tools = merge keysStrict "tools";
      # graft's own names, taken by a unit: raised by the tools, which every repo builds
      taken = kind: k: map (n: "graft: ${kind} ${k} (from ${n}): that name is graft's; ${if n == "self" then "rename the repo's own" else "the provider must rename it"}")
        (lib.filter (n: lib.elem k (keysOf n kind)) units);
      reserved = taken "apps" "graft" ++ taken "packages" "tools"
        ++ lib.concatMap (k: taken "apps" k ++ taken "packages" k) [ "sandbox" "pot" "graft-kits" ];
      warnEmpty = x: if empty == [ ] then x
        else lib.warn (emptyMsg empty ", so the tools leave out what ${if lib.length empty == 1 then "it gives" else "they give"}") x;
      toolsEnv = ok (if reserved != [ ] then throw (lib.concatStringsSep "\n" reserved)
        else warnEmpty (pkgs.buildEnv { name = "tools"; paths = lib.attrValues tools; }));
      # graft-kits: the repo kit (perSystem's `kit`) and each provider's, for pot (lib/pot). v2.14.0's
      # `sandbox` is an error that names its replacement
      kitLib = import ./pot/kit.nix { inherit lib; };
      kitOf = n: let r = results.${n}; in
        if r ? sandbox then throw "graft: ${if n == "self" then "the consumer's perSystem" else "${dir}/${n}/graft.nix"} gives `sandbox`: graft v2.15.0 replaced it with `kit` (pot; see graft's CHANGELOG)"
        else r.kit or null;
      kitFile = n: if n == "self" then "flake.nix (perSystem)" else "${dir}/${n}/graft.nix";
      kitData = lib.filterAttrs (_: v: v != null) (lib.genAttrs units (n:
        let k = ctx n (kitOf n); in if k == null then null else kitLib.manifest { kit = k; file = kitFile n; }));
      kitRoots = lib.concatMap (n: let k = ctx n (kitOf n); in if k == null then [ ] else kitLib.roots k) units;
      graftForRepo = cli pkgs { GRAFT_DIR = dir; GRAFT_IN_REPO = "1"; };
      selfKit = if perSystem != null then kitOf "self" else null;
      kitTools = pkgs.buildEnv { name = "graft-kits-tools"; paths = lib.attrValues tools ++ [ graftForRepo ]
        ++ (if selfKit == null then [ ] else selfKit.packages or [ ]); };
      graftKits = ok (pkgs.runCommand "graft-kits" {
        nativeBuildInputs = [ pkgs.python3 ];
        kitJson = builtins.toJSON {
          version = 1; inherit dir;
          self = kitData.self or null;
          file = "flake.nix (perSystem)";
          scions = removeAttrs kitData [ "self" ];
        };
        passAsFile = [ "kitJson" ];
      } ''
        mkdir -p $out
        cp "$kitJsonPath" $out/kit.json
        python3 -I ${potSrc}/compose.py validate $out/kit.json
        ln -s ${kitTools}/bin $out/bin
        cp ${pkgs.closureInfo { rootPaths = [ kitTools ] ++ kitRoots; }}/store-paths $out/closure
      '');
      devShells = lib.mapAttrs (_: ok) (results.self.devShells or { });
      ownDefault = devShells ? default || outputs ? devShells.${system}.default;
    in
    {
      # packages' names by keysOf too: `nix build .#NAME` and `nix eval` look in packages.<system>
      # first, so its names must evaluate for the repo's own outputs (and apps.graft) to; the tools
      # (strict) fail on the provider instead
      packages = removeAttrs (merge keysOf "packages") [ "tools" "graft-kits" "sandbox" "pot" ]
        // { tools = toolsEnv; graft-kits = graftKits; };
      apps = removeAttrs (merge keysOf "apps") [ "graft" "sandbox" "pot" "graft-kits" ];
      checks = lib.foldl' (acc: n:
        acc // lib.listToAttrs (map (k: lib.nameValuePair (if n == "self" then k else "${n}-${k}")
          (ok (ctx n results.${n}.checks.${k}))) (keysStrict n "checks"))) { } units;
      devShells = devShells // lib.optionalAttrs (!ownDefault) {
        default = ok (pkgs.mkShell { packages = [ toolsEnv ]; });
      };
    };

  bySystem = lib.genAttrs systems forSystem;
  graftOut = lib.genAttrs [ "packages" "apps" "checks" "devShells" ]
    (kind: lib.mapAttrs (_: o: o.${kind}) bySystem);

  # the repo's own last, per system and name (a derivation is replaced whole, never merged into)
  merged = lib.recursiveUpdateUntil
    (path: l: r: lib.length path >= 3 || !(builtins.isAttrs l && builtins.isAttrs r)) graftOut outputs;

  # graft's own outputs: the repo's own `outputs` may not replace them; sandbox and pot are reserved
  own = system: kind: k: v: if outputs ? ${kind}.${system}.${k}
    then throw "graft: mkFlake's outputs has ${kind}.${system}.${k}: that name is graft's; rename the repo's own"
    else v;
  reservedOut = lib.concatMap (kind: lib.concatMap (s: map (k: "graft: mkFlake's outputs has ${kind}.${s}.${k}: that name is reserved (pot); rename the repo's own")
    (lib.filter (k: lib.elem k [ "sandbox" "pot" ]) (lib.attrNames (outputs.${kind}.${s} or { }))))
    (lib.attrNames (outputs.${kind} or { }))) [ "apps" "packages" ];
  noReserved = v: if reservedOut == [ ] then v else throw (lib.concatStringsSep "\n" reservedOut);
  graftApps = system:
    let pkgs = nixpkgs.legacyPackages.${system}; in
    lib.mapAttrs (own system "apps") {
      graft = {
        type = "app";
        program = lib.getExe (cli pkgs { GRAFT_DIR = dir; GRAFT_IN_REPO = "1"; });
        meta.description = "graft: add, sync, check, outdated, upgrade, changelog, migrate, ci";
      };
    };
in
merged // {
  apps = (merged.apps or { }) // lib.genAttrs systems (s: lib.mapAttrs (_: noReserved) (merged.apps.${s} or { }) // graftApps s);
  packages = (merged.packages or { }) // lib.genAttrs systems (s: lib.mapAttrs (_: noReserved) (merged.packages.${s} or { }) // {
    graft-kits = own s "packages" "graft-kits" bySystem.${s}.packages.graft-kits;
  });
}
