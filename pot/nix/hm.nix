# homeManagerModules.pot: a personal root in home-manager (design §13.1, D1). programs.pot's options are
# graft.lib.mkRoot's arguments; the module builds the root's pot, puts it on PATH, and points
# ~/.config/pot/launcher at it (the pointer pot's wrappers read from 1b). It asserts that
# workmux's Docker sandbox is off: pot replaces it for agent panes, never stacks with it.
{ mkRoot }:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.pot;
  t = lib.types;
  opt = type: default: description: lib.mkOption { inherit type default description; };
  # defaultAccount (v2.15.0's): every kit's default instance; deprecated in v2.16.0, removed in v2.17.0
  defaultInstance =
    if cfg.defaultAccount == null then cfg.defaultInstance
    else if cfg.defaultInstance != { } then throw "programs.pot: give defaultInstance, or defaultAccount (its old form), not both"
    else lib.warn "programs.pot.defaultAccount is programs.pot.defaultInstance.<kit>; defaultAccount goes in v2.17.0"
      (lib.listToAttrs (map (n: lib.nameValuePair (lib.head (lib.splitString "@" n)) cfg.defaultAccount)
        (lib.filter (lib.hasInfix "@") (lib.attrNames cfg.kits))));
  pot = mkRoot {
    inherit pkgs defaultInstance;
    name = "home-manager";
    inherit (cfg) grantsFile stateDir contexts defaultContext kits hidden tools bindings rootRepos agentMarkers
      repoKits inspect caBundle;
  };
in
{
  options.programs.pot = {
    enable = lib.mkEnableOption "pot, a sandbox for agents and other commands (experimental)";
    grantsFile = opt t.str "~/.config/pot/grants.json" "The grants file (hidden from every run). Track it in your root's repository.";
    stateDir = opt t.str "~/.local/state/pot" "pot's runs, logs, caches and state (hidden from every run).";
    contexts = opt (t.attrsOf (t.attrsOf t.anything)) { } "Contexts added to, or changing, host inspect work dispatch.";
    defaultContext = opt t.str "work" "The context of `pot CMD`.";
    kits = opt (t.attrsOf (t.attrsOf t.anything)) { } "Root kits, by name (NAME, or NAME@INSTANCE): graft.kits.claude-code { … }, or a kit's data.";
    defaultInstance = opt (t.attrsOf t.str) { } "Per kit, the instance a command with several gets with no @NAME, --instance or matching caller variables: { claude-code = \"personal\"; }.";
    defaultAccount = opt (t.nullOr t.str) null "Deprecated (goes in v2.17.0): defaultInstance.<kit>.";
    agentMarkers = opt (t.listOf t.str) [ ] "Variables that mean an agent is the caller (pot grant refuses; no inline prompt), beyond the floor (CODEX_SANDBOX, CLAUDECODE, POT) and each kit's agent.markers.";
    hidden = opt (t.listOf t.str) [ ] "Paths hidden from every run, beyond pot's defaults.";
    tools = opt (t.listOf t.package) [ ] "Packages on every run's PATH, after the standard set.";
    bindings = opt (t.attrsOf (t.attrsOf (t.attrsOf t.anything))) { } "bindings.<capability>.\"<selector>\": how each capability is carried out.";
    rootRepos = opt (t.listOf t.str) [ "~/.workstation" ] "This root's source repositories: pot work refuses them. The module can't see which repository holds the home-manager flake, so the default is ~/.workstation.";
    repoKits = opt (t.attrsOf (t.attrsOf t.anything)) { } "Repo kits for repositories with no pot-kits of their own, by origin pattern.";
    inspect = opt (t.attrsOf t.anything) { } "inspect.home: what pot inspect reads in ~ (replaces [ \"~/src\" ]).";
    caBundle = opt (t.nullOr t.path) null "A CA bundle, at /etc/ssl/certs/ca-certificates.crt in every run.";
    package = lib.mkOption { type = t.package; readOnly = true; description = "The root's pot."; };
  };

  config = lib.mkIf cfg.enable {
    programs.pot.package = pot;
    home.packages = [ pot ];
    home.file.".config/pot/launcher".source = "${pot}/bin/pot";
    assertions = [ {
      assertion = !(lib.attrByPath [ "workmux" "sandbox" "enable" ] false config);
      message = "programs.pot.enable: set workmux.sandbox.enable = false (pot replaces workmux's Docker sandbox for agent panes; it never stacks with it)";
    } ];
  };
}
