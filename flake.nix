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
      # graft sb's programs (python), alone in the store: a change elsewhere in graft changes no root
      sbSrc = builtins.path { path = ./lib/sb; name = "graft-sb"; filter = p: _: lib.hasSuffix ".py" p; };
      mkFlake = import ./lib/mk-flake.nix { inherit nixpkgs systems cli sbSrc; };
      mkRoot = import ./lib/sb/root.nix { inherit cli sbSrc; };
      # graft's default root: for someone without one (sb init writes a minimal one to edit)
      defaultRoot = pkgs: mkRoot { inherit pkgs; };
      kbKit = inputs.knowledge-base-kit;
    in
    {
      lib.mkFlake = mkFlake;
      lib.mkRoot = mkRoot;
      # agent kits, for a root to call: graft.kits.claude-code { pkgs; account; … } (lib/sb/kits)
      kits.claude-code = import ./lib/sb/kits/claude-code.nix;
      # a personal root in home-manager: programs.sb (lib/sb/hm.nix)
      homeManagerModules.sb = import ./lib/sb/hm.nix { inherit mkRoot; };

      # Run against any checkout without evaluating its flake:
      #   nix run github:danielnaab/graft -- outdated -C path/to/repo
      # tools: empty (graft grafts only docs conventions); `graft check` from the repo's app evaluates it
      packages = forAll (pkgs: {
        default = cli pkgs { };
        tools = pkgs.buildEnv { name = "tools"; paths = [ ]; };
        # graft's default root's sb (experimental, v2.15.0): nix run github:danielnaab/graft#sb -- CMD
        sb = defaultRoot pkgs;
      });

      # graft's own app, as mkFlake gives a consumer one: nix run .#graft check, sync, ci …
      apps = forAll (pkgs: {
        graft = { type = "app"; program = lib.getExe (cli pkgs { GRAFT_DIR = "scions"; GRAFT_IN_REPO = "1"; }); meta.description = "graft, for this repository"; };
        sb = { type = "app"; program = lib.getExe (defaultRoot pkgs); meta.description = "graft sb with graft's default root (experimental)"; };
      });

      devShells = forAll (pkgs: { default = pkgs.mkShell { packages = [ (cli pkgs { }) pkgs.shellcheck pkgs.jq ]; }; });

      # A new repo's flake.nix and graft's workflows: nix flake init -t github:danielnaab/graft
      templates.default = {
        path = ./template;
        description = "A repo that grafts other repositories, with graft's Forgejo workflows";
        welcomeText = ''
          Graft a first repository (a git submodule at its newest release tag), then commit:
            nix run github:danielnaab/graft/v2.14.0 -- add -C . URL
          See https://github.com/danielnaab/graft.
        '';
      };

      checks = forAll (pkgs: {
        # bash -n first: bash's own parser, so a script bash can't run fails even where shellcheck's
        # parser would differ
        shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          for f in ${./tests}/*.sh ${./lib}/*.sh ${./lib}/sandbox/*.sh; do bash -n "$f" || exit 1; done
          shellcheck -x ${./tests/run.sh} ${./tests/fixtures.sh} ${./tests/macos-smoke.sh} ${./lib}/*.sh ${./lib}/sandbox/*.sh
          touch $out
        '';
        # graft sb's unit tests: composition, identity, normalisation, digests, merge rules, grants
        sb-unit = pkgs.runCommand "graft-sb-unit" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          cp -r ${sbSrc} sb && chmod -R u+w sb && cp ${./tests/sb}/*.py sb/
          cd sb && python3 -I -m unittest discover -s . -p 'test_*.py' -v
          touch $out
        '';
        # the default root builds (on Darwin: an sb that refuses, D21)
        sb-root = defaultRoot pkgs;
        # a root with the claude-code kit (two accounts, MCP, Bedrock) builds: its manifests validate
        sb-root-claude = mkRoot {
          inherit pkgs;
          name = "check";
          kits."claude-code@personal" = self.kits.claude-code { inherit pkgs; account = "personal"; mcp = [ "metamcp" ]; };
          kits."claude-code@work" = self.kits.claude-code { inherit pkgs; account = "work"; configDir = ".claude-accounts/work";
            provider = "bedrock"; region = "us-east-1"; binary = pkgs.hello; };
          defaultAccount = "personal";
          bindings.mcp.metamcp = { url = "https://mcp.example/mcp"; via = "env"; env = "METAMCP_API_KEY"; from.file = "~/.config/x/key.env"; };
        };
        # homeManagerModules.sb evaluates (with home-manager's few options stubbed), and asserts
        # workmux's sandbox off
        sb-hm = let
          ev = workmux: lib.evalModules { modules = [ self.homeManagerModules.sb ({ lib, ... }: {
            options.home.packages = lib.mkOption { type = lib.types.listOf lib.types.package; default = [ ]; };
            options.home.file = lib.mkOption { type = lib.types.attrsOf lib.types.attrs; default = { }; };
            options.assertions = lib.mkOption { type = lib.types.listOf lib.types.attrs; default = [ ]; };
            options.workmux.sandbox.enable = lib.mkOption { type = lib.types.bool; default = false; };
            config = { _module.args.pkgs = pkgs; programs.sb.enable = true; workmux.sandbox.enable = workmux; };
          }) ]; };
          ok = ev false; bad = ev true;
        in assert lib.all (a: a.assertion) ok.config.assertions;
          assert !(lib.all (a: a.assertion) bad.config.assertions);
          pkgs.runCommand "graft-sb-hm" { } "test -x ${ok.config.programs.sb.package}/bin/sb && touch $out";
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
