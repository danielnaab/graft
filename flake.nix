{
  description = "graft: copy pinned, provider-defined exports of other repos into scions/";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # graft-outdated and graft-upgrade: lib/tools.sh, which reads a repo as data (see its header);
      # graft-ci: lib/ci.sh, the steps of the Forgejo workflows in examples/forgejo/
      tools = pkgs:
        let
          tool = name: script: pkgs.writeShellApplication {
            inherit name;
            runtimeInputs = with pkgs; [ bash coreutils diffutils gnugrep gnused jq git util-linux curl ];
            text = ''exec bash ${./lib}/${script} "$@"'';
          };
        in {
          outdated = tool "graft-outdated" "tools.sh outdated";
          upgrade = tool "graft-upgrade" "tools.sh upgrade";
          ci = tool "graft-ci" "ci.sh";
        };
      app = drv: { type = "app"; program = nixpkgs.lib.getExe drv; };
    in
    {
      lib.mkGraft = import ./lib/mk-graft.nix { inherit nixpkgs systems tools; };

      # Run against any checkout without evaluating its flake:
      #   nix run github:danielnaab/graft#outdated -- -C path/to/repo
      packages = forAll (pkgs: tools pkgs);
      apps = forAll (pkgs: builtins.mapAttrs (_: app) (tools pkgs));

      checks = forAll (pkgs: {
        shellcheck = pkgs.runCommand "graft-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          shellcheck -x ${./tests/run.sh} ${./lib/assemble.sh} ${./lib/tools.sh} ${./lib/ci.sh} ${./lib/workflows.sh}
          touch $out
        '';
      });
    };
}
