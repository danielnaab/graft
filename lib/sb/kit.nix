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
in
{
  manifest = { kit, file }:
    if !builtins.isAttrs kit then throw "graft: ${file}: a kit is an attribute set"
    else kit // { inherit file; }
      // lib.optionalAttrs (kit ? packages) { packages = map str kit.packages; }
      // lib.optionalAttrs (kit ? files) { files = lib.mapAttrs (_: f: f // lib.optionalAttrs (f ? source) { source = str f.source; }) kit.files; }
      // lib.optionalAttrs (kit ? commands) {
        commands = lib.mapAttrs (_: c: c // lib.optionalAttrs (c ? program) { program = program c.program; }) kit.commands;
      };

  # what a run of this kit needs in its closure
  roots = kit: (kit.packages or [ ])
    ++ lib.concatMap (f: lib.optional (f ? source && (lib.isDerivation f.source || builtins.isPath f.source || lib.hasPrefix "/nix/store/" (toString f.source))) f.source)
      (lib.attrValues (kit.files or { }));
}
