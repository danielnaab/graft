{
  description = "knowledge-base-kit: knowledge base conventions, and the checker that enforces them (link validation, linting, backlink scanning)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    graft = { url = "github:danielnaab/graft/v2.9.0"; inputs.nixpkgs.follows = "nixpkgs"; };
    living-specifications = { url = "github:danielnaab/living-specifications/v1.0.2"; flake = false; };
  };

  outputs = { nixpkgs, graft, ... }@inputs:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      # `nix run .#<name>` from a knowledge base's root; a repo that takes this flake as an input
      # re-exports the ones it wants.
      tool = pkgs: name: description: text: {
        type = "app";
        meta = { inherit description; };
        program = nixpkgs.lib.getExe (pkgs.writeShellApplication {
          inherit name;
          runtimeInputs = [ pkgs.python3 ];
          text = ''
            export PYTHONPATH=${./src} PYTHONDONTWRITEBYTECODE=1
            ${text}
          '';
        });
      };

      tools = pkgs: {
        lint = tool pkgs "lint" "Check frontmatter status and provenance" "python3 -m kb_linter .";
        check-links = tool pkgs "check-links" "Find broken internal markdown links" "python3 -m link_validator .";
        scan-backlinks = tool pkgs "scan-backlinks" "Trace specs to their implementations" "python3 -m backlink_scanner .";
        validate = tool pkgs "validate" "Check links, then report lint findings"
          "python3 -m link_validator . && python3 -m kb_linter --report-only .";
      };
    in
    graft.lib.mkFlake {
      inherit inputs;
      outputs.apps = nixpkgs.lib.genAttrs systems (system: tools nixpkgs.legacyPackages.${system});
    };
}
