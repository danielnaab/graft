{
  description = "Say what this repo is";

  inputs = {
    graft.url = "github:danielnaab/graft/v2.6.0";

    # Each `flake = false` input is a scion, grafted into scions/<name>/.
    meta-knowledge-base = { url = "github:danielnaab/meta-knowledge-base/v1.0.1"; flake = false; };
  };

  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
