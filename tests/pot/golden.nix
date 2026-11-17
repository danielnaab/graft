# The golden grant test's root (check pot-golden, tests/pot/golden.py): instances built with the
# arguments of a real personal root (Daniel's ~/.workstation, modules/pot.nix, at graft v2.15.0),
# its standing kit `daniel`, and flexion-assistant's repo kit (nix/agent-tools/flake.nix). The
# canonical grant items of these kits, and their digests, are fixed in golden.json: a change to the
# claude-code kit or to the kit schema that moves one item re-asks every grant on a live root.
#
# Change the arguments here only to say the same thing in a new way (a renamed argument): the items
# must stay byte-identical. Never regenerate golden.json to make the check pass.
{ pkgs, mkRoot, claudeCode }:

let
  # what Daniel's root passes to every instance (files: home-manager's, which only fix files)
  common = {
    inherit pkgs;
    binary = "native";
    skillSources = [ "~/src/agent-skills" "~/src/agent-skills-private" ];
    mcp = [ "sessions" ];
  };
in
mkRoot {
  inherit pkgs;
  name = "golden";
  kits."claude-code@personal" = claudeCode (common // {
    account = "personal"; configDir = ".claude"; provider = "anthropic"; skillDirs = [ "synced" "address-pr-comments" ];
  });
  kits."claude-code@flexion" = claudeCode (common // {
    account = "flexion"; configDir = ".claude-accounts/flexion"; provider = "anthropic"; skillDirs = [ "synced" ];
  });
  kits."claude-code@flexion-bedrock" = claudeCode (common // {
    account = "flexion-bedrock"; configDir = ".claude-accounts/flexion"; provider = "bedrock"; region = "us-east-2";
    skillDirs = [ "synced" ];
  });
  kits.daniel.requests = {
    network = [ "pypi.org" "files.pythonhosted.org" "registry.npmjs.org" ];
    capabilities = [ { name = "git.fetch"; remote = "ssh://forgejo@platform-vm:2222/*"; } ];
  };
  defaultAccount = "personal";
  # flexion-assistant's perSystem.kit, as a root's repo kit (the same manifest, the same items)
  repoKits."platform-vm:2222/daniel/flexion-assistant" = {
    protect = [ "nix/agent-tools/ensure.sh" ".local" ".githooks" "AGENTS.md" "docs/boundaries.md" ];
    requests.capabilities = [ { name = "nix"; } ];
  };
}
