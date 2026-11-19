# pot.lib.kitsFor (contract pot-lib/1): a repository's kits as the bundle pot reads at run start,
# packages.<system>.pot-kits (contract pot-kits/1). The one Nix edge between pot and a producer: graft's
# mkFlake calls it once per system and knows nothing of a kit's fields.
#
#   kitsFor {
#     pkgs;              # the system's nixpkgs
#     self ? null;       # the repo kit: { kit; file; } (file: where it is written, for pot's messages)
#     scions ? { };      # scions.<name> = { kit; file; }: each scion's kit
#     tools ? [ ];       # packages for bin/ (the repo kit's own `tools` are added here)
#     files ? [ ];       # what the producer has pot protect in every run: paths relative to the flake
#                        # directory, or "**/NAME" anywhere in the repository
#     recipes ? { };     # the producer's own recipes (graft's outdated, upgrade, graft-check), in kit/1's
#                        # recipe form, for kits.just (shape fixed in v2.17.0; generated from v2.18.0)
#   }
#
# The result's passthru.wrappers: the declared commands' wrappers, one package (v2.18.0); passthru.kitsJust: the
# kits.just text (a string, so graft check can compare it with nix eval, without a build).
#
# The bundle: kit.json { version = 1; self; scions.<name>; protect; } (each kit a manifest that keeps its
# `file`), bin/ (the tools) and closure (the store paths a run needs). compose.py validates it as it
# builds, so a kit with an unknown field fails the producer's build, naming its file.
{ potSrc }:

{ pkgs, self ? null, scions ? { }, tools ? [ ], files ? [ ], recipes ? { } }:

let
  lib = pkgs.lib;
  kitLib = import ./kit.nix { inherit lib; };
  unit = where: u:
    if !(builtins.isAttrs u && u ? kit && u ? file && builtins.isString u.file)
    then throw "pot: kitsFor: ${where} is { kit; file; } (file: a string naming where the kit is written)"
    else u;
  # kit.packages, tools' old name: gone at the source in v2.17.0 (pot still reads it in a bundle graft v2.16.x
  # built)
  noPackages = u: if builtins.isAttrs u.kit && u.kit ? packages
    then throw "pot: ${u.file}: packages is tools' old name, gone in v2.17.0: give tools" else u;
  selfU = if self == null then null else noPackages (unit "self" self);
  scionUs = lib.mapAttrs (n: s: noPackages (unit "scions.${n}" s)) scions;
  units = lib.optional (selfU != null) selfU ++ lib.attrValues scionUs;
  manifest = u: kitLib.manifest { inherit (u) kit file; };
  recipeOk = r: builtins.isString r || (builtins.isAttrs r && r ? run && builtins.isString r.run
    && builtins.attrNames (removeAttrs r [ "run" "description" ]) == [ ]);
  badRecipes = lib.filter (n: !(recipeOk recipes.${n})) (lib.attrNames recipes);
  # ---- D4 (v2.18.0): a wrapper for every declared command of every kit here, and kits.just from the recipes
  okName = n: builtins.match "[A-Za-z0-9][A-Za-z0-9._-]*" n != null;
  progOf = c: let p = c.program or null; in
    if p != null && (builtins.isString p || lib.isDerivation p || builtins.isPath p) then "${p}" else "";
  cmdsOf = slot: u: lib.mapAttrsToList (n: c: { inherit slot n; prog = progOf c; }) (u.kit.commands or { });
  allCmds = lib.optionals (selfU != null) (cmdsOf "self" selfU)
    ++ lib.concatLists (lib.mapAttrsToList (n: u: cmdsOf "scions.${n}" u) scionUs);
  cmdSlots = lib.foldl' (acc: c: acc // { ${c.n} = (acc.${c.n} or [ ]) ++ [ c.slot ]; }) { } allCmds;
  twice = lib.filterAttrs (_: v: lib.length v > 1) cmdSlots;
  badCmd = lib.filter (c: !(okName c.n)) allCmds;
  sh = "${pkgs.bash}/bin/sh";
  cu = "${pkgs.coreutils}/bin";
  wrapperText = c: ''
    #!${sh}
    # graft (pot-kits/1): ${c.n}, a command of ${c.slot}, run by pot under its own policy
    m=@out@/commands/${c.n}.json
    prog=${lib.escapeShellArg c.prog}
    u=$(${cu}/id -un)
    case $u in *[!A-Za-z0-9._-]*|"") h= ;; *) eval "h=~$u" ;; esac
    l=$h/.config/pot/launcher
    # inside a run: /run/pot/policy.json (bubblewrap), or POT_POLICY's (Seatbelt: the run's folder is its /run/pot)
    pol=/run/pot/policy.json
    [ -r "$pol" ] || pol=''${POT_POLICY:-/nonexistent}
    if [ -r "$pol" ]; then
      # a run a v2.18.0+ pot started names its launcher (beside its policy.json, read-only): only that pot, or 126
      if ${pkgs.gnugrep}/bin/grep -q '"launcher"' "$pol"; then
        [ ! -x "''${pol%/*}/launcher" ] || exec "''${pol%/*}/launcher" run --command ${c.n} --manifest "$m" -- "$@"
        echo "pot: ${c.n}: this run's launcher (''${pol%/*}/launcher) is missing, so its children table can't be reached: run ${c.n} on the host" >&2
        exit 126
      fi
      # a run an older pot started: the program, under that run's policy
      [ -n "$prog" ] || { echo "pot: ${c.n}: its program is on the host: run it there" >&2; exit 126; }
      exec "$prog" "$@"
    fi
    why=
    if [ -x "$l" ]; then
      r=$(${cu}/realpath "$l")
      [ ! -e "''${r%/bin/pot}/share/pot/run-1" ] || exec "$l" run --command ${c.n} --manifest "$m" -- "$@"
      why="your root's pot predates wrapped commands: take graft v2.18.0 in your root (home-manager switch)"
    fi
    [ -n "$prog" ] || { echo "pot: ${c.n} needs pot (''${why:-no root here: ~/.config/pot/launcher is missing})" >&2; exit 126; }
    echo "pot: ${c.n} runs without pot (''${why:-no root here: ~/.config/pot/launcher is missing})" >&2
    exec "$prog" "$@"
  '';
  bundleFiles = lib.listToAttrs (lib.concatMap (c: [
    { name = "wrappers/bin/${c.n}"; value = wrapperText c; }
    { name = "commands/${c.n}.json"; value = builtins.toJSON { version = 1; command = c.n; kit = c.slot; }; }
  ]) allCmds);
  # kits.just: graft's fixed header, then one recipe per kit recipe and producer recipe
  recipeUnits = lib.optional (selfU != null) selfU ++ lib.attrValues scionUs;
  # F1 (v2.18.0): every recipe is checked here, inside the text graft writes (nix eval reads kitsJust without a
  # build, so the build's own check would come too late): a name, then words of a safe set, one space apart, the first
  # one of the kit's own commands (a tool, for the producer's). compose.py checks the same at build and at run start.
  recipeOf = from: cmds: n: r: let
    run = if builtins.isString r then r else if builtins.isAttrs r && r ? run && builtins.isString r.run then r.run else "";
    desc = if builtins.isAttrs r && builtins.isString (r.description or "") then r.description or "" else "";
    words = lib.splitString " " run;
    where = "pot: ${from}: recipes.${n}";
    cmd = lib.head words;
  in
    if builtins.match "[A-Za-z][A-Za-z0-9_-]*" n == null then throw "${where}: a recipe's name is letters, digits, _ and - (starting with a letter)"
    else if run == "" || builtins.match ".*\n.*" desc != null then throw "${where}: { run; description; } (kit 1.2), or one line"
    else if !(lib.all (w: builtins.match "[A-Za-z0-9._/=:,@+%-]+" w != null) words)
    then throw "${where}: literal words only (letters, digits and . _ / = : , @ + % -, one space apart)"
    else if cmds != null && !(lib.elem cmd cmds)
    then throw "${where}: runs ${cmd}, which is not one of this kit's commands (${lib.concatStringsSep ", " cmds})"
    else { inherit n from desc cmd; args = lib.drop 1 words; };
  kitRecipes = lib.concatMap (u: lib.mapAttrsToList (recipeOf u.file (lib.attrNames (u.kit.commands or { }))) (u.kit.recipes or { })) recipeUnits;
  ownRecipes = lib.mapAttrsToList (recipeOf "kitsFor" null) recipes;
  allRecipes = kitRecipes ++ ownRecipes;
  recipeNames = lib.foldl' (acc: r: acc // { ${r.n} = (acc.${r.n} or [ ]) ++ [ r.from ]; }) { } allRecipes;
  recipeTwice = lib.filterAttrs (_: v: lib.length v > 1) recipeNames;
  recipeText = r: "\n" + lib.optionalString (r.desc != "") "# ${lib.replaceStrings [ "\n" ] [ " " ] r.desc}\n"
    + "[positional-arguments]\n${r.n} *args:\n    exec \"{{tools}}/bin/${r.cmd}\"${lib.concatMapStrings (a: " '" + a + "'") r.args} \"$@\"\n";
  kitsJust = if recipeTwice != { }
    then throw "pot: kitsFor: two recipes are named ${lib.head (lib.attrNames recipeTwice)} (${lib.concatStringsSep " and " recipeTwice.${lib.head (lib.attrNames recipeTwice)}})"
    else "# Generated by graft from the kits' recipes (pot-kits/1). graft sync writes it and graft check compares it:\n"
    + "# do not edit. The importing justfile defines `tools`, the tools package (its root, not its bin/).\n"
    + lib.concatMapStrings recipeText (lib.sort (a: b: a.n < b.n) allRecipes);
  env = pkgs.buildEnv {
    name = "pot-kits-tools";
    paths = tools ++ (if selfU == null then [ ] else selfU.kit.tools or [ ]);
  };
  potKits = pkgs.runCommand "pot-kits" {
  passthru = {
    # the wrappers alone, for the producer's tools (mkFlake puts them over the tools of the same name)
    wrappers = pkgs.runCommand "pot-kits-wrappers" { } ''
      mkdir -p $out/bin
      for f in ${potKits}/wrappers/bin/*; do [ ! -e "$f" ] || ln -s "$f" $out/bin/; done
    '';
    inherit kitsJust;
  };
  nativeBuildInputs = [ pkgs.python3 ];
  filesJson = builtins.toJSON bundleFiles;
  inherit kitsJust;
  kitJson = builtins.toJSON {
    version = 1;
    self = if selfU == null then null else manifest selfU;
    scions = lib.mapAttrs (_: manifest) scionUs;
    protect = files;
  };
  passAsFile = [ "kitJson" "filesJson" "kitsJust" ];
} ''
  mkdir -p $out/bin
  cp "$kitJsonPath" $out/kit.json
  python3 -I ${potSrc}/compose.py validate $out/kit.json
  cp "$kitsJustPath" $out/kits.just
  python3 -I - "$filesJsonPath" "$out" <<'PY'
  import json, os, sys
  out = sys.argv[2]
  for rel, text in json.load(open(sys.argv[1])).items():
      p = os.path.join(out, rel)
      os.makedirs(os.path.dirname(p), exist_ok=True)
      with open(p, "w") as f:
          f.write(text.replace("@out@", out))
      if rel.startswith("wrappers/"):
          os.chmod(p, 0o555)
  PY
  # bin/: the tools, each declared command's wrapper in place of the program of that name (v2.18.0)
  for f in ${env}/bin/*; do [ ! -e "$f" ] || ln -s "$f" $out/bin/; done
  if [ -d $out/wrappers/bin ]; then for w in $out/wrappers/bin/*; do ln -sfn "$w" $out/bin/; done; fi
  cp ${pkgs.closureInfo { rootPaths = [ env ] ++ lib.concatMap (u: kitLib.roots u.kit) units; }}/store-paths $out/closure
'';
in
if !builtins.isAttrs recipes || badRecipes != [ ]
then throw "pot: kitsFor: recipes.${lib.head (badRecipes ++ [ "?" ])} is \"<command> <args>\" or { run; description; }"
else if twice != { }
then throw "pot: kitsFor: two kits ship a command named ${lib.head (lib.attrNames twice)} (${lib.concatStringsSep " and " twice.${lib.head (lib.attrNames twice)}}): a wrapper has one name"
else if recipeTwice != { }
then throw "pot: kitsFor: two recipes are named ${lib.head (lib.attrNames recipeTwice)} (${lib.concatStringsSep " and " recipeTwice.${lib.head (lib.attrNames recipeTwice)}})"
else if badCmd != [ ]
then throw "pot: kitsFor: a command name is letters, digits, . _ and - (${(lib.head badCmd).n})"
else potKits
