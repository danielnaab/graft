{
  description = "Say what this repo is";

  inputs = {
    graft.url = "github:danielnaab/graft/v2.11.0";

    # Each `flake = false` input is a scion, grafted into scions/<name>/.
    knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
  };

  # A repo with content that must stay out of the Nix store keeps this file in a subdirectory,
  # passes flakeDir = "nix/workspace"; and runs nix run path:nix/workspace#graft
  # (graft's docs/guides/flake-in-a-subdirectory.md).
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
