{
  description = "graft: copy pinned, provider-defined exports of other repos into scions/";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

    # graft's own scions, grafted by this checkout's mkFlake (below), not by a pinned graft.
    # A repo that uses graft never evaluates this input; `graft.inputs.knowledge-base-kit.follows`
    # can point it at the repo's own copy to keep its flake.lock to one entry.
    knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
  };

  outputs = { self, nixpkgs, ... }@inputs:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # The graft command (lib/graft.sh). A repo's copy carries its `sync` (lib/mk-flake.nix) and
      # where its flake sits under the repo's top (mkFlake's flakeDir); graft's own has neither, and
      # reads any checkout with -C.
      cli = pkgs: sync: flakeDir: pkgs.writeShellApplication {
        name = "graft";
        runtimeInputs = with pkgs; [ bash coreutils diffutils findutils gawk gnugrep gnused jq git util-linux curl ];
        text = ''
          export GRAFT_SYNC=${nixpkgs.lib.escapeShellArg sync} GRAFT_FLAKE_DIR=${nixpkgs.lib.escapeShellArg flakeDir}
          exec bash ${./lib}/graft.sh "$@"
        '';
        meta.description = "Copy pinned, provider-defined exports of other repos into this one";
      };
      mkFlake = import ./lib/mk-flake.nix { inherit nixpkgs systems cli; };
    in
    # graft grafts its own scions with this checkout's mkFlake: `nix run .#graft` writes scions/,
    # `nix flake check` checks them, and `nix run .#graft upgrade` moves knowledge-base-kit.
    mkFlake {
      inherit inputs;
      outputs = {
        lib.mkFlake = mkFlake;

        # Run against any checkout without evaluating its flake:
        #   nix run github:danielnaab/graft -- outdated -C path/to/repo
        packages = forAll (pkgs: { default = cli pkgs "" ""; });

        # A new repo's flake.nix and graft's workflows: nix flake init -t github:danielnaab/graft
        templates.default = {
          path = ./template;
          description = "A repo that grafts other repos' exports, with graft's Forgejo workflows";
          welcomeText = ''
            Edit `description` and the inputs in flake.nix (each `flake = false` input is a scion),
            then `nix run .#graft` and commit. See https://github.com/danielnaab/graft.
            In a subdirectory (to keep private files out of the Nix store), set `flakeDir` and run
            `nix run 'path:DIR#graft'` from the repository's top: `nix run .#graft` would copy the whole repository.
          '';
        };

        checks = forAll (pkgs: {
          shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
            shellcheck -x ${./tests/run.sh} ${./tests/fixtures.sh} ${./lib}/*.sh
            touch $out
          '';
          # graft's docs against knowledge-base-kit's conventions: links and anchors, lifecycle status,
          # provenance (scopes in knowledge-base.yaml). The checker is the fetched export's src/.
          docs = pkgs.runCommand "graft-docs" { nativeBuildInputs = [ pkgs.python3 ]; } ''
            cd ${self}
            export PYTHONPATH=${inputs.knowledge-base-kit}/src
            # one command per line: under set -e, a failure left of `&&` would not stop the build
            python3 -m link_validator .
            python3 -m kb_linter .
            touch $out
          '';
        });
      };
    };
}
