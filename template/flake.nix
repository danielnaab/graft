{
  inputs.self.submodules = true;
  inputs.graft.url = "github:danielnaab/graft/v2.17.1";
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
