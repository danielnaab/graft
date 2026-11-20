{
  inputs.self.submodules = true;
  inputs.graft.url = "github:danielnaab/graft/v2.20.1";
  outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
}
