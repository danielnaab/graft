# homeManagerModules.sb: a personal root in home-manager (design §13.1, D1). programs.sb's options are
# graft.lib.mkRoot's arguments; the module builds the root's sb, puts it on PATH, and points
# ~/.config/graft/sb/launcher at it (the pointer graft's wrappers read from 1b). It asserts that
# workmux's Docker sandbox is off: sb replaces it for agent panes, never stacks with it.
{ mkRoot }:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.sb;
  t = lib.types;
  opt = type: default: description: lib.mkOption { inherit type default description; };
  sb = mkRoot {
    inherit pkgs;
    name = "home-manager";
    inherit (cfg) grantsFile stateDir contexts defaultContext kits defaultAccount hidden tools bindings rootRepos
      repoKits inspect caBundle;
  };
in
{
  options.programs.sb = {
    enable = lib.mkEnableOption "graft sb, a sandbox for agents and other commands (experimental)";
    grantsFile = opt t.str "~/.config/graft/sb/grants.json" "The grants file (hidden from every run). Track it in your root's repository.";
    stateDir = opt t.str "~/.local/state/graft/sb" "sb's runs, logs, caches and state (hidden from every run).";
    contexts = opt (t.attrsOf (t.attrsOf t.anything)) { } "Contexts added to, or changing, host inspect work dispatch.";
    defaultContext = opt t.str "work" "The context of `sb CMD`.";
    kits = opt (t.attrsOf (t.attrsOf t.anything)) { } "Root kits, by name (NAME or NAME@ACCOUNT): graft.kits.claude-code { … }, or a kit's data.";
    defaultAccount = opt (t.nullOr t.str) null "The account instance a command with several gets with no --account.";
    hidden = opt (t.listOf t.str) [ ] "Paths hidden from every run, beyond graft's defaults.";
    tools = opt (t.listOf t.package) [ ] "Packages on every run's PATH, after the standard set.";
    bindings = opt (t.attrsOf (t.attrsOf (t.attrsOf t.anything))) { } "bindings.<capability>.\"<selector>\": how each capability is carried out.";
    rootRepos = opt (t.listOf t.str) [ "~/.workstation" ] "This root's source repositories: sb work refuses them. The module can't see which repository holds the home-manager flake, so the default is ~/.workstation.";
    repoKits = opt (t.attrsOf (t.attrsOf t.anything)) { } "Repo kits for repositories that are not graft consumers, by origin pattern.";
    inspect = opt (t.attrsOf t.anything) { } "inspect.home: what sb inspect reads in ~ (replaces [ \"~/src\" ]).";
    caBundle = opt (t.nullOr t.path) null "A CA bundle, at /etc/ssl/certs/ca-certificates.crt in every run.";
    package = lib.mkOption { type = t.package; readOnly = true; description = "The root's sb."; };
  };

  config = lib.mkIf cfg.enable {
    programs.sb.package = sb;
    home.packages = [ sb ];
    home.file.".config/graft/sb/launcher".source = "${sb}/bin/sb";
    assertions = [ {
      assertion = !(lib.attrByPath [ "workmux" "sandbox" "enable" ] false config);
      message = "programs.sb.enable: set workmux.sandbox.enable = false (sb replaces workmux's Docker sandbox for agent panes; it never stacks with it)";
    } ];
  };
}
