{
  inputs.self.submodules = true;
  inputs.graft.url = "github:danielnaab/graft/v2.22.0";
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
