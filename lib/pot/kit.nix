# A kit (design §6.1) as a JSON manifest: packages as store paths, file sources as store paths, the
# rest literal. compose.py validates it (unknown fields are errors) when the root or graft-kits builds.
#
#   kit = {
#     packages = [ ];  env.set = { };  files."~/x" = { source; mode = "fixed"; };    # see
#     protect = [ ];                                                                  # limits
#     requests = { read; write; socket; network; env; state; capabilities; unsandboxed; };   # reach
#     commands.NAME = { program; access; requests; };  recipes.NAME = "COMMAND ARGS";
#   };
{ lib }:

let
  str = x: if lib.isDerivation x || builtins.isPath x then "${x}" else x;
  program = p: if builtins.isAttrs p && !(lib.isDerivation p) then p else str p;
  storeRoot = p: if lib.isDerivation p || builtins.isPath p then p else
    let s = toString p; m = builtins.match "(/nix/store/[^/]+).*" s; in
    if m == null || !(builtins.hasContext s) then null else builtins.appendContext (lib.head m) (builtins.getContext s);
in
{
  manifest = { kit, file }:
    if !builtins.isAttrs kit then throw "graft: ${file}: a kit is an attribute set"
    else kit // { inherit file; }
      // lib.optionalAttrs (kit ? packages) { packages = map str kit.packages; }
      // lib.optionalAttrs (kit ? files) { files = lib.mapAttrs (_: f: f // lib.optionalAttrs (f ? source) { source = str f.source; }) kit.files; }
      // lib.optionalAttrs (kit ? commands) {
        commands = lib.mapAttrs (_: c: c // lib.optionalAttrs (c ? program) { program = program c.program; }
          // lib.optionalAttrs (c ? args) { args = map str c.args; }) kit.commands;
      };

  # what a run of this kit needs in its closure (store roots: a subpath, "${pkg}/bin/x", as its package)
  # (a plain store-path string, with no context, is left to pot, which adds its closure at start)
  roots = kit: lib.filter (p: p != null) (map storeRoot ((kit.packages or [ ])
    ++ lib.concatMap (c: lib.filter (a: lib.isDerivation a || builtins.isPath a) (c.args or [ ])
      ++ lib.optional (c ? program && (lib.isDerivation c.program || builtins.isString c.program && lib.hasPrefix "/nix/store/" c.program)) c.program)
      (lib.attrValues (kit.commands or { }))
    ++ lib.concatMap (f: lib.optional (f ? source && (lib.isDerivation f.source || builtins.isPath f.source || lib.hasPrefix "/nix/store/" (toString f.source))) f.source)
      (lib.attrValues (kit.files or { }))));
}
