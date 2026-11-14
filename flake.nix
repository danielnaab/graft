{
  description = "graft: other repositories as git submodules pinned to release tags, wired into your flake";

  # graft is other flakes' input (github:, and path: or tarballs in its tests), so this flake.nix must not
  # declare inputs.self.submodules: Nix applies it to that fetch too, and those schemes reject it
  # ("input attribute 'submodules' not supported by scheme 'github'"). # graft: no self.submodules
  # So graft can't mkFlake itself: its scion is absent from its source. The docs check reads
  # knowledge-base-kit from this input instead, at the scion's gitlink (tests/run.sh `own` checks they match).
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  inputs.knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };

  outputs = { self, nixpkgs, ... }@inputs:
    let
      lib = nixpkgs.lib;
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # The graft command (lib/graft.sh), with this version's workflows (template/). ENV is exported
      # first: a repo's own app (mkFlake) sets GRAFT_IN_REPO and GRAFT_DIR, its scions directory's name;
      # graft's own package sets nothing, and reads any checkout with -C.
      cli = pkgs: env: pkgs.writeShellApplication {
        name = "graft";
        runtimeInputs = with pkgs; [ bash coreutils diffutils findutils gawk gnugrep gnused jq git util-linux curl ];
        text = ''
          ${lib.concatStrings (lib.mapAttrsToList (k: v: "export ${k}=${lib.escapeShellArg v}\n") env)}
          export GRAFT_TEMPLATE=${./template}
          exec bash ${./lib}/graft.sh "$@"
        '';
        meta.description = "Graft other repositories as git submodules pinned to release tags";
      };
      mkFlake = import ./lib/mk-flake.nix { inherit nixpkgs systems cli; };
      kbKit = inputs.knowledge-base-kit;
    in
    {
      lib.mkFlake = mkFlake;

      # Run against any checkout without evaluating its flake:
      #   nix run github:danielnaab/graft -- outdated -C path/to/repo
      # tools: empty (graft grafts only docs conventions); `graft check` from the repo's app evaluates it
      packages = forAll (pkgs: {
        default = cli pkgs { };
        tools = pkgs.buildEnv { name = "tools"; paths = [ ]; };
      });

      # graft's own app, as mkFlake gives a consumer one: nix run .#graft check, sync, ci …
      apps = forAll (pkgs: {
        graft = { type = "app"; program = lib.getExe (cli pkgs { GRAFT_DIR = "scions"; GRAFT_IN_REPO = "1"; }); meta.description = "graft, for this repository"; };
      });

      devShells = forAll (pkgs: { default = pkgs.mkShell { packages = [ (cli pkgs { }) pkgs.shellcheck pkgs.jq ]; }; });

      # A new repo's flake.nix and graft's workflows: nix flake init -t github:danielnaab/graft
      templates.default = {
        path = ./template;
        description = "A repo that grafts other repositories, with graft's Forgejo workflows";
        welcomeText = ''
          Graft a first repository (a git submodule at its newest release tag), then commit:
            nix run github:danielnaab/graft/v2.13.1 -- add -C . URL
          See https://github.com/danielnaab/graft.
        '';
      };

      checks = forAll (pkgs: {
        # bash -n first: bash's own parser, so a script bash can't run fails even where shellcheck's
        # parser would differ
        shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          for f in ${./tests}/*.sh ${./lib}/*.sh; do bash -n "$f" || exit 1; done
          shellcheck -x ${./tests/run.sh} ${./tests/fixtures.sh} ${./lib}/*.sh
          touch $out
        '';
        # graft's docs against knowledge-base-kit's conventions: links and anchors, lifecycle status,
        # provenance (scopes in knowledge-base.yaml). The checker is knowledge-base-kit's src/.
        docs = pkgs.runCommand "graft-docs" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          cd ${self}
          export PYTHONPATH=${kbKit}/src
          # one command per line: under set -e, a failure left of `&&` would not stop the build
          python3 -m link_validator .
          python3 -m kb_linter .
          touch $out
        '';
      });
    };
}
