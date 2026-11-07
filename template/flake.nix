{
  description = "Say what this repo is";

  inputs = {
    graft.url = "github:danielnaab/graft/v2.7.1";

    # Each `flake = false` input is a scion, grafted into scions/<name>/.
    meta-knowledge-base = { url = "github:danielnaab/meta-knowledge-base/v1.0.1"; flake = false; };
  };

  # A repo with content that must stay out of the Nix store keeps this file in a subdirectory,
  # passes flakeDir = "nix/workspace"; and runs nix run path:nix/workspace#graft (graft's README).
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
