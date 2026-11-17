{
  description = "graft: other repositories as git submodules pinned to release tags, wired into your flake";

  # graft is other flakes' input (github:, and path: or tarballs in its tests), so this flake.nix must not
  # declare inputs.self.submodules: Nix applies it to that fetch too, and those schemes reject it
  # ("input attribute 'submodules' not supported by scheme 'github'"). # graft: no self.submodules
  # So graft can't mkFlake itself: its scion is absent from its source. The docs check is in dev/flake.nix
  # (nix flake check ./dev), with its knowledge-base-kit input, so graft's consumers never lock it.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs, ... }@inputs:
    let
      lib = nixpkgs.lib;
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # graft's command is built from graft's own sources alone: lib/'s scripts, its jq files and the
      # scions README it writes, and template/. pot/ and lib/mk-flake.nix are not in it, so a pot change
      # leaves packages.<system>.default as it was (tests/run.sh `own` checks it).
      graftLib = builtins.path { path = ./lib; name = "graft-lib";
        filter = p: t: t == "regular" && (lib.hasSuffix ".sh" p || lib.hasSuffix ".jq" p || baseNameOf p == "scions-readme.md"); };
      # The graft command (lib/graft.sh), with this version's workflows (template/). ENV is exported
      # first: a repo's own app (mkFlake) sets GRAFT_IN_REPO and GRAFT_DIR, its scions directory's name;
      # graft's own package sets nothing, and reads any checkout with -C.
      cli = pkgs: env: pkgs.writeShellApplication {
        name = "graft";
        runtimeInputs = with pkgs; [ bash coreutils diffutils findutils gawk gnugrep gnused jq git util-linux curl ];
        text = ''
          ${lib.concatStrings (lib.mapAttrsToList (k: v: "export ${k}=${lib.escapeShellArg v}\n") env)}
          export GRAFT_TEMPLATE=${./template}
          exec bash ${graftLib}/graft.sh "$@"
        '';
        meta.description = "Graft other repositories as git submodules pinned to release tags";
      };
      # pot's programs (python, pot/core), alone in the store: a change elsewhere changes no root
      potSrc = builtins.path { path = ./pot/core; name = "graft-pot"; filter = p: _: lib.hasSuffix ".py" p; };
      # pot.lib.kitsFor (pot/lib/kits-for.nix, contract pot-lib/1): the one thing of pot's that mkFlake calls
      kitsFor = import ./pot/lib/kits-for.nix { inherit potSrc; };
      mkFlake = import ./lib/mk-flake.nix { inherit nixpkgs systems cli kitsFor; };
      mkRoot = import ./pot/nix/root.nix { inherit potSrc; };
      # graft's default root: for someone without one (pot init writes a minimal one to edit). It runs
      # the host's claude as claude-code@personal, with its token file (claude setup-token) and nix
      defaultRoot = pkgs: mkRoot {
        inherit pkgs;
        kits."claude-code@personal" = import ./pot/kits/claude-code.nix { inherit pkgs; binary = "host"; };
        defaultInstance.claude-code = "personal";
        bindings = {
          "status.relay"."*" = { workmux = true; };
          credential."api.anthropic.com kit=claude-code@personal" = { via = "file"; from.file = "~/.config/pot/claude-token-personal"; };
          nix."*" = { via = "daemon-untrusted"; };
        };
      };
    in
    {
      lib.mkFlake = mkFlake;
      lib.mkRoot = mkRoot;
      # pot's library for a producer of kits (contract pot-lib/1): kitsFor { pkgs; self; scions; tools; files; }
      lib.pot.kitsFor = kitsFor;
      # agent kits, for a root to call: graft.kits.claude-code { pkgs; configDir; api; … } (pot/kits)
      kits.claude-code = import ./pot/kits/claude-code.nix;
      # the APIs an agent kit can use: graft.apis.anthropic, graft.apis.bedrock { region; } (pot/kits/apis.nix)
      apis = import ./pot/kits/apis.nix;
      # a personal root in home-manager: programs.pot (pot/nix/hm.nix)
      homeManagerModules.pot = import ./pot/nix/hm.nix { inherit mkRoot; };

      # Run against any checkout without evaluating its flake:
      #   nix run github:danielnaab/graft -- outdated -C path/to/repo
      # tools: empty (graft grafts only docs conventions); `graft check` from the repo's app evaluates it
      packages = forAll (pkgs: {
        default = cli pkgs { };
        tools = pkgs.buildEnv { name = "tools"; paths = [ ]; };
        # graft's default root's pot (experimental, v2.16.1): nix run github:danielnaab/graft#pot -- CMD
        pot = defaultRoot pkgs;
      });

      # graft's own app, as mkFlake gives a consumer one: nix run .#graft check, sync, ci …
      apps = forAll (pkgs: {
        graft = { type = "app"; program = lib.getExe (cli pkgs { GRAFT_DIR = "scions"; GRAFT_IN_REPO = "1"; }); meta.description = "graft, for this repository"; };
        pot = { type = "app"; program = lib.getExe (defaultRoot pkgs); meta.description = "pot with graft's default root (experimental)"; };
      });

      devShells = forAll (pkgs: { default = pkgs.mkShell { packages = [ (cli pkgs { }) pkgs.shellcheck pkgs.jq ]; }; });

      # A new repo's flake.nix and graft's workflows: nix flake init -t github:danielnaab/graft
      templates.default = {
        path = ./template;
        description = "A repo that grafts other repositories, with graft's Forgejo workflows";
        welcomeText = ''
          Graft a first repository (a git submodule at its newest release tag), then commit:
            nix run github:danielnaab/graft/v2.16.1 -- add -C . URL
          See https://github.com/danielnaab/graft.
        '';
      };

      checks = forAll (pkgs: {
        # bash -n first: bash's own parser, so a script bash can't run fails even where shellcheck's
        # parser would differ
        shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          for f in ${./tests}/*.sh ${graftLib}/*.sh; do bash -n "$f" || exit 1; done
          shellcheck -x ${./tests/run.sh} ${./tests/fixtures.sh} ${graftLib}/*.sh
          touch $out
        '';
        # pot's unit tests: composition, identity, normalisation, digests, merge rules, grants
        pot-unit = pkgs.runCommand "graft-pot-unit" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          cp -r ${potSrc} pot && chmod -R u+w pot && cp ${./pot/tests}/test_*.py pot/ && cp -r ${./pot/tests/output} pot/output
          cd pot && python3 -I -m unittest discover -s . -p 'test_*.py' -v
          touch $out
        '';
        # the golden grant test: the canonical grant items and digests of the default root's
        # claude-code@personal, and of pot/tests/golden.nix's root (a real root's instances, its
        # standing kit and a repo kit), equal pot/tests/golden.json byte for byte
        pot-golden = let
          kitsOf = r: { inherit (r.passthru.root) kits repoKits; };
          kitsJson = builtins.unsafeDiscardStringContext (builtins.toJSON {
            default = kitsOf (defaultRoot pkgs);
            golden = kitsOf (import ./pot/tests/golden.nix { inherit pkgs mkRoot; claudeCode = self.kits.claude-code; inherit (self) apis; });
          });
        in pkgs.runCommand "graft-pot-golden" { nativeBuildInputs = [ pkgs.python3 ]; inherit kitsJson; passAsFile = [ "kitsJson" ];
          passthru = { inherit kitsJson; }; } ''
          python3 -I ${./pot/tests/golden.py} ${potSrc} "$kitsJsonPath" ${./pot/tests/golden.json}
          touch $out
        '';
        # the default root builds (on Darwin: a pot that refuses, D21)
        pot-root = defaultRoot pkgs;
        # a root with the claude-code kit (two instances, MCP, Bedrock) builds: its manifests validate
        pot-root-claude = mkRoot {
          inherit pkgs;
          name = "check";
          kits."claude-code@personal" = self.kits.claude-code { inherit pkgs; mcp = [ "metamcp" ]; };
          kits."claude-code@work" = self.kits.claude-code { inherit pkgs; configDir = ".claude-accounts/work";
            api = self.apis.bedrock { region = "us-east-1"; }; binary = pkgs.hello; };
          defaultInstance.claude-code = "personal";
          bindings.mcp.metamcp = { url = "https://mcp.example/mcp"; via = "env"; env = "METAMCP_API_KEY"; from.file = "~/.config/x/key.env"; };
        };
        # v2.15.0's forms, deprecated in v2.16.0 (they go in v2.17.0), give the same root: provider and region
        # (the claude-code kit), and defaultAccount (mkRoot)
        pot-aliases = let
          ck = args: builtins.unsafeDiscardStringContext (builtins.toJSON (self.kits.claude-code ({ inherit pkgs; } // args)));
          rootOf = args: (mkRoot ({ inherit pkgs; kits."claude-code@w" = self.kits.claude-code { inherit pkgs; }; } // args)).passthru.root.defaultInstance;
        in assert ck { provider = "bedrock"; region = "us-east-1"; } == ck { api = self.apis.bedrock { region = "us-east-1"; }; };
          assert ck { provider = "anthropic"; } == ck { };
          assert ck { account = "w"; } == ck { };
          # defaultInstance is checked at build (F4): an unknown kit or instance is an error naming the declared ones
          assert !(builtins.tryEval (builtins.deepSeq (mkRoot { inherit pkgs; kits."claude-code@w" = self.kits.claude-code { inherit pkgs; };
            defaultInstance.claude-code = "nope"; }).drvPath true)).success;
          assert !(builtins.tryEval (builtins.deepSeq (mkRoot { inherit pkgs; kits."claude-code@w" = self.kits.claude-code { inherit pkgs; };
            defaultInstance.claud-code = "w"; }).drvPath true)).success;
          assert rootOf { defaultAccount = "w"; } == rootOf { defaultInstance.claude-code = "w"; };
          pkgs.runCommand "graft-pot-aliases" { } "touch $out";
        # the agent-marker floor is root data (C3): a root with no agent kit still knows CLAUDECODE and CODEX_SANDBOX
        pot-markers = let r = (mkRoot { inherit pkgs; }).passthru.root; in
          assert lib.all (m: lib.elem m r.baseAgentMarkers) [ "CODEX_SANDBOX" "CLAUDECODE" ];
          assert r.agentMarkers == [ ];
          pkgs.runCommand "graft-pot-markers" { } "touch $out";
        # homeManagerModules.pot evaluates (with home-manager's few options stubbed), and asserts
        # workmux's sandbox off
        pot-hm = let
          ev = workmux: lib.evalModules { modules = [ self.homeManagerModules.pot ({ lib, ... }: {
            options.home.packages = lib.mkOption { type = lib.types.listOf lib.types.package; default = [ ]; };
            options.home.file = lib.mkOption { type = lib.types.attrsOf lib.types.attrs; default = { }; };
            options.assertions = lib.mkOption { type = lib.types.listOf lib.types.attrs; default = [ ]; };
            options.workmux.sandbox.enable = lib.mkOption { type = lib.types.bool; default = false; };
            config = { _module.args.pkgs = pkgs; programs.pot.enable = true; workmux.sandbox.enable = workmux; };
          }) ]; };
          ok = ev false; bad = ev true;
        in assert lib.all (a: a.assertion) ok.config.assertions;
          assert !(lib.all (a: a.assertion) bad.config.assertions);
          pkgs.runCommand "graft-pot-hm" { } "test -x ${ok.config.programs.pot.package}/bin/pot && touch $out";
      });
    };
}
