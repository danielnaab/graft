{
  description = "graft: other repositories as git submodules pinned to release tags, wired into your flake";

  # graft's own scion (scions/knowledge-base-kit, its docs checker) is a git submodule, but this flake
  # doesn't declare inputs.self.submodules: Nix refuses that attribute on a path: input, and repos
  # and tests use graft as one (--override-input graft path:...). Check graft itself with the
  # submodule: nix flake check '.?submodules=1' (or 'path:.').
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

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
      # the docs checker, from the knowledge-base-kit scion alone (not the whole checkout)
      kbKit = builtins.path { path = "${self}/scions/knowledge-base-kit"; name = "scion-knowledge-base-kit"; filter = p: _: baseNameOf p != ".git"; };
    in
    # graft grafts its own scions with this checkout's mkFlake
    mkFlake {
      inherit inputs;
      outputs = {
        lib.mkFlake = mkFlake;

        # Run against any checkout without evaluating its flake:
        #   nix run github:danielnaab/graft -- outdated -C path/to/repo
        packages = forAll (pkgs: { default = cli pkgs { }; });

        # A new repo's flake.nix and graft's workflows: nix flake init -t github:danielnaab/graft
        templates.default = {
          path = ./template;
          description = "A repo that grafts other repositories, with graft's Forgejo workflows";
          welcomeText = ''
            Edit `description` in flake.nix, then graft a first repository:
              nix run github:danielnaab/graft/v2.13.0 -- add URL
            and commit. See https://github.com/danielnaab/graft.
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
          # provenance (scopes in knowledge-base.yaml). The checker is the scion's src/.
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
    };
}
