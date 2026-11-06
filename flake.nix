{
  description = "graft: copy pinned, provider-defined exports of other repos into scions/";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # The graft command (lib/graft.sh). A repo's copy carries its `sync` (lib/mk-flake.nix);
      # graft's own has none, and reads any checkout with -C.
      cli = pkgs: sync: pkgs.writeShellApplication {
        name = "graft";
        runtimeInputs = with pkgs; [ bash coreutils diffutils findutils gawk gnugrep gnused jq git util-linux curl ];
        text = ''
          export GRAFT_SYNC=${sync}
          exec bash ${./lib}/graft.sh "$@"
        '';
        meta.description = "Copy pinned, provider-defined exports of other repos into this one";
      };
    in
    {
      lib.mkFlake = import ./lib/mk-flake.nix { inherit nixpkgs systems cli; };

      # Run against any checkout without evaluating its flake:
      #   nix run github:danielnaab/graft -- outdated -C path/to/repo
      packages = forAll (pkgs: { default = cli pkgs ""; });

      # A new repo's flake.nix and graft's workflows: nix flake init -t github:danielnaab/graft
      templates.default = {
        path = ./template;
        description = "A repo that grafts other repos' exports, with graft's Forgejo workflows";
        welcomeText = ''
          Edit `description` and the inputs in flake.nix (each `flake = false` input is a scion),
          then `nix run .#graft` and commit. See https://github.com/danielnaab/graft.
        '';
      };

      checks = forAll (pkgs: {
        shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          shellcheck -x ${./tests/run.sh} ${./lib}/*.sh
          touch $out
        '';
      });
    };
}
