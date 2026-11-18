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
# The result's passthru.wrappers: the declared commands' wrappers, one package (empty until v2.18.0).
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
  wrappers = pkgs.buildEnv { name = "pot-kits-wrappers"; paths = [ ]; };
  env = pkgs.buildEnv {
    name = "pot-kits-tools";
    paths = tools ++ (if selfU == null then [ ] else selfU.kit.tools or [ ]);
  };
in
if !builtins.isAttrs recipes || badRecipes != [ ]
then throw "pot: kitsFor: recipes.${lib.head (badRecipes ++ [ "?" ])} is \"<command> <args>\" or { run; description; }"
else pkgs.runCommand "pot-kits" {
  passthru = { inherit wrappers; };
  nativeBuildInputs = [ pkgs.python3 ];
  kitJson = builtins.toJSON {
    version = 1;
    self = if selfU == null then null else manifest selfU;
    scions = lib.mapAttrs (_: manifest) scionUs;
    protect = files;
  };
  passAsFile = [ "kitJson" ];
} ''
  mkdir -p $out
  cp "$kitJsonPath" $out/kit.json
  python3 -I ${potSrc}/compose.py validate $out/kit.json
  ln -s ${env}/bin $out/bin
  cp ${pkgs.closureInfo { rootPaths = [ env ] ++ lib.concatMap (u: kitLib.roots u.kit) units; }}/store-paths $out/closure
''
