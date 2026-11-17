{
  description = "graft's development checks: inputs that graft's own flake keeps out of its consumers' locks";

  # The docs check reads the whole repository (../), so run it from graft's checkout as a git flake,
  # never path: (that would copy dev/ alone):
  #   nix flake check ./dev        (tests/run.sh `own` runs it)
  # knowledge-base-kit is read from this input, at the scion's gitlink (tests/run.sh `own` checks they
  # match). To check against an unreleased edit in the scion:
  #   nix flake check ./dev --override-input knowledge-base-kit path:$PWD/scions/knowledge-base-kit
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/ac62194c3917d5f474c1a844b6fd6da2db95077d";   # graft's nixos-25.05
  inputs.knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };

  outputs = { nixpkgs, knowledge-base-kit, ... }:
    let
      lib = nixpkgs.lib;
      forAll = f: lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ]
        (system: f nixpkgs.legacyPackages.${system});
      repo = builtins.path { path = ./..; name = "graft-source"; };
    in
    {
      checks = forAll (pkgs: {
        # graft's docs against knowledge-base-kit's conventions: links and anchors, lifecycle status,
        # provenance (scopes in knowledge-base.yaml). The checker is knowledge-base-kit's src/.
        docs = pkgs.runCommand "graft-docs" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          cd ${repo}
          export PYTHONPATH=${knowledge-base-kit}/src
          # one command per line: under set -e, a failure left of `&&` would not stop the build
          python3 -m link_validator .
          python3 -m kb_linter .
          touch $out
        '';
      });
    };
}
