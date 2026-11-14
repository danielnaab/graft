# The sandbox: a run sees its closure, read-only, and the impure things it declares (the
# repository, host paths, hosts, variables), nothing else. mkFlake calls this per system with each
# provider's `sandbox` and the consumer's; graft's base comes first. The policy (policy.nix) is a
# module evaluation; a renderer turns it into the launcher: bubblewrap (bwrap.nix), on Linux.
# nixpkgs is the one --with builds from; nixpkgsFrom says whose it is.
{ pkgs, dir, nixpkgs, nixpkgsFrom, tools, scions, providers, consumer }:

let
  lib = pkgs.lib;
  base = {
    _file = "graft";
    packages = with pkgs; [ bashInteractive coreutils findutils gnugrep gnused diffutils less gitMinimal tools ];
    # what decides the sandbox (the flake, its lock, the providers' graft.nix, the submodules) or
    # what the host runs (direnv, just, git's pre-commit framework)
    protect = [ "**/flake.nix" "**/flake.lock" "**/graft.nix" "**/.gitmodules" "**/.envrc" "**/justfile"
      "**/.pre-commit-config.yaml" ];
  };
  policy = import ./policy.nix {
    inherit lib base consumer scions;
    providers = lib.mapAttrs (n: module: { file = "${dir}/${n}/graft.nix"; inherit module; }) providers;
  };
  passthru = { policy = policy.config; inherit (policy) explain; };
in
if pkgs.stdenv.hostPlatform.isLinux then import ./bwrap.nix { inherit pkgs lib policy nixpkgs nixpkgsFrom passthru; }
else pkgs.writeShellApplication {
  name = "sandbox";
  text = ''echo "graft: sandbox: not yet on macOS (Linux bubblewrap only)" >&2; exit 1'';
  inherit passthru;
}
