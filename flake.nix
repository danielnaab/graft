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
          credential."api.anthropic.com kit=claude-code@personal" = { via = "file"; from.file = "~/.config/pot/claude-token-personal"; refresh = "claude setup-token"; };
          nix."*" = { via = "daemon-untrusted"; };
        };
      };
    in
    {
      lib.mkFlake = mkFlake;
      lib.mkRoot = mkRoot;
      # pot's library for a producer of kits (contract pot-lib/1): kitsFor { pkgs; self; scions; tools; files; }
      lib.pot.kitsFor = kitsFor;
      # agent kits, for a root to call: graft.kits.claude-code { pkgs; configDir; api; … } and graft.kits.pi (pot/kits)
      kits.claude-code = import ./pot/kits/claude-code.nix;
      # graft.kits.pi { pkgs; providers; settings; files; endpoints; … } (v2.19.0)
      kits.pi = import ./pot/kits/pi.nix;
      # the APIs an agent kit can use: graft.apis.anthropic, graft.apis.bedrock { region; }, graft.apis.gateway { url; } (pot/kits/apis.nix)
      apis = import ./pot/kits/apis.nix;
      # a personal root in home-manager: programs.pot (pot/nix/hm.nix)
      homeManagerModules.pot = import ./pot/nix/hm.nix { inherit mkRoot; };

      # Run against any checkout without evaluating its flake:
      #   nix run github:danielnaab/graft -- outdated -C path/to/repo
      # tools: empty (graft grafts only docs conventions); `graft check` from the repo's app evaluates it
      packages = forAll (pkgs: {
        default = cli pkgs { };
        tools = pkgs.buildEnv { name = "tools"; paths = [ ]; };
        # graft's default root's pot (experimental, v2.20.1): nix run github:danielnaab/graft#pot -- CMD
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
            nix run github:danielnaab/graft/v2.20.1 -- add -C . URL
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
        # a root with the claude-code kit (three instances, MCP, Bedrock, a gateway) builds: its manifests validate
        pot-root-claude = mkRoot {
          inherit pkgs;
          name = "check";
          kits."claude-code@personal" = self.kits.claude-code { inherit pkgs; mcp = [ "metamcp" ]; };
          kits."claude-code@work" = self.kits.claude-code { inherit pkgs; configDir = ".claude-accounts/work";
            api = self.apis.bedrock { region = "us-east-1"; }; binary = pkgs.hello; };
          kits."claude-code@gateway" = self.kits.claude-code { inherit pkgs; configDir = ".claude-accounts/work";
            api = self.apis.gateway { url = "https://llm.example.test"; }; };
          defaultInstance.claude-code = "personal";
          bindings.mcp.metamcp = { url = "https://mcp.example/mcp"; via = "env"; env = "METAMCP_API_KEY"; from.file = "~/.config/x/key.env"; };
        };
        # v2.17.1: the claude-code kit pointed at a gateway (graft.apis.gateway): its host on 443 and no claude.ai,
        # its base URL and model discovery (nonessential traffic left on; telemetry, error reports and /bug off),
        # the credential as ANTHROPIC_AUTH_TOKEN, and the base URL in agent.select.env; a url that is not https on
        # 443 fails evaluation (tests/run.sh kits checks the message)
        pot-gateway = let
          fails = x: !(builtins.tryEval (builtins.deepSeq x true)).success;
          gw = url: self.kits.claude-code { inherit pkgs; configDir = ".claude-accounts/flexion"; api = self.apis.gateway { inherit url; }; };
          k = gw "https://llm.example.test/";
          rq = k.commands.claude.requests;
          base = "https://llm.example.test/anthropic";
        in assert rq.network == [ "llm.example.test" ];
          assert rq.env == [ ];
          assert k.env.set.ANTHROPIC_BASE_URL == base;
          assert k.env.set.CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY == "1";
          assert !(k.env.set ? CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC);
          assert lib.all (v: k.env.set.${v} == "1") [ "DISABLE_AUTOUPDATER" "DISABLE_TELEMETRY" "DISABLE_ERROR_REPORTING" "DISABLE_BUG_COMMAND" ];
          assert lib.elem { name = "credential"; host = "llm.example.test"; env = "ANTHROPIC_AUTH_TOKEN"; } rq.capabilities;
          assert k.agent.select.env == { CLAUDE_CONFIG_DIR = "~/.claude-accounts/flexion"; ANTHROPIC_BASE_URL = base; };
          assert (gw "https://llm.example.test:443/v1/").env.set.ANTHROPIC_BASE_URL == "https://llm.example.test/v1/anthropic";
          assert (gw "https://LLM.Example.test").commands.claude.requests.network == [ "llm.example.test" ];
          assert fails (self.apis.gateway { url = "http://llm.example.test"; });
          assert fails (self.apis.gateway { url = "https://llm.example.test:8443"; });
          assert fails (self.apis.gateway { url = "llm.example.test"; });
          assert fails (self.apis.gateway { url = "https://user@llm.example.test"; });
          assert fails (self.apis.gateway { url = "https://llm.example.test/?x=1"; });
          pkgs.runCommand "graft-pot-gateway" { } "touch $out";
        # v2.19.0: the pi kit. Its reach is computed from the providers: each baseUrl (provider and model) and
        # endpoint is a network request (https HOST on 443, http HOST:PORT), each "$NAME"/"${NAME}" key a credential
        # for its host, and nothing else; pi's configuration is fixed (settings.json, models.json, auth.json and
        # the rest); its folders are read-only; a loopback url, a provider with no baseUrl and a command key fail
        pot-pi = let
          fails = x: !(builtins.tryEval (builtins.deepSeq x true)).success;
          pi = args: self.kits.pi ({ inherit pkgs; } // args);
          agents = pkgs.writeText "agents" "x";
          k = pi {
            files.a = { target = ".pi/agent/AGENTS.md"; source = agents; };
            files.b = { target = ".pi/agent/extensions/x.ts"; source = agents; };
            files.c = { target = ".claude/CLAUDE.md"; source = agents; };
            settings.defaultProvider = "local";
            providers = {
              local = { baseUrl = "http://192.168.1.52:11434/v1"; apiKey = "llama-cpp"; models = [ { id = "q"; } ]; };
              openrouter = { baseUrl = "https://openrouter.ai/api/v1"; apiKey = "\${OPENROUTER_API_KEY}"; };
              gw = { baseUrl = "https://LLM.example.test/openai"; apiKey = "$GW_KEY";
                     models = [ { id = "c"; baseUrl = "https://llm.example.test:443/anthropic"; } { id = "o"; baseUrl = "https://other.example.test:8443/v1"; } ]; };
            };
            endpoints = [ { url = "https://ext.example.test"; key = "EXT_KEY"; } { url = "https://nokey.example.test"; } ];
          };
          rq = k.commands.pi.requests;
          f = k.files;
          fixed = lib.all (v: v.mode == "fixed") (lib.attrValues f);
          src = n: f."~/.pi/agent/${n}".source;
          one = p: pi { providers.p = p; };
        in assert lib.sort lib.lessThan rq.network == lib.sort lib.lessThan [ "192.168.1.52:11434" "openrouter.ai" "llm.example.test"
            "other.example.test:8443" "ext.example.test" "nokey.example.test" ];
          assert lib.sort (a: b: a.host < b.host) (lib.filter (c: c.name == "credential") rq.capabilities) == [
            { name = "credential"; host = "ext.example.test"; env = "EXT_KEY"; }
            { name = "credential"; host = "llm.example.test"; env = "GW_KEY"; }
            { name = "credential"; host = "openrouter.ai"; env = "OPENROUTER_API_KEY"; }
            { name = "credential"; host = "other.example.test"; env = "GW_KEY"; } ];
          assert lib.elem { name = "status.relay"; } rq.capabilities;
          assert rq.read == [ "~/.pi/agent/bin" "~/.pi/agent/install" ];
          assert rq.state == [ { name = "pi"; at = "~/.pi/agent"; } ];
          assert !(rq ? env) && !(rq ? write) && !(rq ? socket) && !(rq ? unsandboxed);
          assert k.commands.pi.program == { host = "~/.pi/agent/bin/pi"; };
          assert k.env.set == { PI_CODING_AGENT_DIR = "~/.pi/agent"; PI_OFFLINE = "1"; PI_SKIP_VERSION_CHECK = "1"; PI_TELEMETRY = "0"; };
          assert fixed;
          assert lib.attrNames f == map (n: "~/.pi/agent/${n}") [ "AGENTS.md" "AGENTS.override.md" "APPEND_SYSTEM.md" "SYSTEM.md"
            "auth.json" "keybindings.json" "mcp-auth.json" "mcp.json" "models.json" "settings.json" ];
          assert src "AGENTS.md" == agents && src "AGENTS.override.md" == agents;
          assert lib.all (d: k.commands.pi.dirs."~/.pi/agent/${d}".from == "~/.pi/agent/${d}") [ "extensions" "skills" "prompts" "themes" "npm" "git" ];
          assert k.commands.pi.dirs."~/.pi/agent/npm".real == true && k.commands.pi.dirs."~/.pi/agent/extensions".real == [ ];
          assert k.agent.markers == [ "PI_CODING_AGENT" ] && !(k.agent ? select);
          assert k.agent.resume.args == [ "--continue" ];
          # a provider at the host's loopback (host-loopback is a later release), with no baseUrl, or a command key
          assert fails (one { baseUrl = "http://localhost:8000/api/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "http://127.0.0.1:8000/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "http://[::1]:8000/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://ok.example.test"; models = [ { id = "m"; baseUrl = "http://localhost:1/v1"; } ]; }).commands.pi.requests;
          assert fails (one { models = [ ]; }).commands.pi.requests;
          assert fails (one { baseUrl = "ftp://x.example.test"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test"; apiKey = "!pass show key"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test"; apiKey = "Bearer $X"; }).commands.pi.requests;
          assert fails (pi { endpoints = [ { url = "https://x.example.test"; header = "h"; } ]; }).commands.pi.requests;
          # F1 (fix round 1): a trailing dot names the same host (localhost. is loopback; Example.test. is example.test)
          assert fails (one { baseUrl = "http://localhost.:8000/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "http://LOCALHOST..:8000/v1"; }).commands.pi.requests;
          assert (one { baseUrl = "https://Example.test./v1"; }).commands.pi.requests.network == [ "example.test" ];
          # F2: a key named by a variable travels only over https; a literal key on http stays allowed
          assert fails (one { baseUrl = "http://x.example.test/v1"; apiKey = "\${K}"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test/v1"; apiKey = "$K"; models = [ { id = "m"; baseUrl = "http://y.example.test:8080/v1"; } ]; }).commands.pi.requests;
          assert fails (pi { endpoints = [ { url = "http://x.example.test"; key = "K"; } ]; }).commands.pi.requests;
          assert (one { baseUrl = "http://x.example.test:8080/v1"; apiKey = "literal"; }).commands.pi.requests.network == [ "x.example.test:8080" ];
          # F3: every header value follows apiKey's rules: no command; "''${NAME}" is a credential for the provider's hosts
          assert fails (one { baseUrl = "https://x.example.test"; headers.Authorization = "!pass key"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test"; models = [ { id = "m"; headers.X = "!cmd"; } ]; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test"; modelOverrides.m.headers.X = "!cmd"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test"; headers.X = "Bearer $X"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test"; headers.X = 1; }).commands.pi.requests;
          assert fails (one { baseUrl = "http://x.example.test"; headers.X = "\${H}"; }).commands.pi.requests;
          assert lib.filter (c: c.name == "credential") (one { baseUrl = "https://h.example.test"; apiKey = "\${A_KEY}";
              headers = { X-Key = "\${H_KEY}"; X-Lit = "plain"; }; modelOverrides.m.headers.X-O = "$O_KEY";
              models = [ { id = "n"; headers.X-M = "\${A_KEY}"; } ]; }).commands.pi.requests.capabilities
            == [ { name = "credential"; host = "h.example.test"; env = [ "A_KEY" "H_KEY" "O_KEY" ]; } ];
          # I3: ports 1 to 65535 with no leading zero, and no IPv6 literal (pot's network requests name hosts)
          assert fails (one { baseUrl = "https://x.example.test:0/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test:99999/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://x.example.test:0443/v1"; }).commands.pi.requests;
          assert fails (one { baseUrl = "https://[2001:db8::1]/v1"; }).commands.pi.requests;
          assert fails (pi { endpoints = [ { url = "https://x.example.test"; key = "not a name"; } ]; }).commands.pi.requests;
          assert fails (pi { binary = "host"; }).version;
          assert (pi { binary = pkgs.hello; }).commands.pi.program == "${pkgs.hello}/bin/pi";
          assert (pi { binary = pkgs.hello; }).commands.pi.requests.read == [ ];
          # the files' contents, read at build (no import from derivation)
          pkgs.runCommand "graft-pot-pi" { nativeBuildInputs = [ pkgs.jq ]; } ''
            test "$(jq -c . ${src "auth.json"})" = '{}' && test "$(jq -c . ${src "mcp-auth.json"})" = '{}'
            test "$(jq -c . ${src "mcp.json"})" = '{"mcpServers":{}}' && test "$(jq -c . ${src "keybindings.json"})" = '{}'
            test ! -s ${src "SYSTEM.md"} && test ! -s ${src "APPEND_SYSTEM.md"}
            test "$(jq -r .providers.local.apiKey ${src "models.json"})" = llama-cpp
            test "$(jq -r .providers.openrouter.apiKey ${src "models.json"})" = '${"$"}{OPENROUTER_API_KEY}'
            test "$(jq -c . ${src "settings.json"})" = '{"defaultProvider":"local"}'
            touch $out
          '';
        # a root with the pi kit builds: its manifest validates (kit 1.1, no new field)
        pot-root-pi = mkRoot {
          inherit pkgs;
          name = "check-pi";
          kits.pi = self.kits.pi { inherit pkgs;
            providers.gw = { baseUrl = "https://llm.example.test/v1"; apiKey = "\${GW_KEY}"; models = [ { id = "m"; } ]; }; };
          bindings.credential."llm.example.test kit=pi" = { via = "file"; from.file = "~/.config/x/key"; };
        };
        # v2.16.0's deprecated forms are gone in v2.17.0: each fails evaluation (tests/run.sh kits checks the
        # message, which names its replacement); defaultInstance is checked at build (F4)
        pot-removed = let
          fails = x: !(builtins.tryEval (builtins.deepSeq x true)).success;
          cc = args: self.kits.claude-code ({ inherit pkgs; } // args);
          root = args: (mkRoot ({ inherit pkgs; kits."claude-code@w" = cc { }; } // args)).drvPath;
        in assert fails (cc { provider = "bedrock"; region = "us-east-1"; }).version;
          assert fails (cc { account = "w"; }).version;
          assert fails (root { defaultAccount = "w"; });
          assert fails (mkRoot { inherit pkgs; kits.x.packages = [ pkgs.hello ]; }).drvPath;
          assert fails (kitsFor { inherit pkgs; self = { kit.packages = [ pkgs.hello ]; file = "f"; }; }).drvPath;
          assert !fails (root { defaultInstance.claude-code = "w"; });
          assert fails (root { defaultInstance.claude-code = "nope"; });
          assert fails (root { defaultInstance.claud-code = "w"; });
          pkgs.runCommand "graft-pot-removed" { } "touch $out";
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
