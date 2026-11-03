{
  description = "graft: copy pinned, provider-defined exports of other repos into scions/";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      lib.mkGraft = import ./lib/mk-graft.nix { inherit nixpkgs systems; };

      checks = forAll (pkgs: {
        shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          shellcheck ${./tests/run.sh}
          touch $out
        '';
      });
    };
}
