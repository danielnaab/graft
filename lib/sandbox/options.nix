# The sandbox's options: what a run sees (its closure, fixed values, protected repository paths) and
# what it may reach (host paths, hosts, the caller's variables). A provider's `sandbox` may set the
# first kind; the second is a request, inert until the consumer grants the scion (policy.nix).
{ lib, ... }:

let
  inherit (lib) mkOption types;
  # a type that names the rule it breaks: the module system's error quotes the description, with
  # the file of the definition
  str' = description: check: types.addCheck types.str check // { inherit description; };
  hostPath = str' ''host path starting with "~/" or "/" (a path in the repository goes in protect)''
    (p: lib.hasPrefix "/" p || lib.hasPrefix "~/" p);
  # a path, or "**/NAME": what is named NAME anywhere in the repository (the launcher's one walk of
  # the working tree finds both, scions and untracked files too)
  repoPath = str' ''path in the repository, relative to its top (no leading "/" or "~", no "..", no * ? [ but in "**/NAME")''
    (p: p != "" && !(lib.hasPrefix "/" p) && !(lib.hasPrefix "~" p)
      && !(lib.elem ".." (lib.splitString "/" p))
      && (builtins.match "[^*?[]*" p != null || builtins.match "[*][*]/[^/]+" p != null));
  # host, or host:port; fnmatch on the host ("*.example.com"), ports 1-65535
  host = str' ''"host" or "host:port" (the host may use * and ?, as in "*.example.com")''
    (h: let m = builtins.match "([A-Za-z0-9*?_][A-Za-z0-9*?._-]*)(:([0-9]{1,5}))?" h;
            port = if m == null then null else lib.elemAt m 2;
        in m != null && (port == null || (lib.toInt port >= 1 && lib.toInt port <= 65535)));
  envName = str' "environment variable name" (n: builtins.match "[A-Za-z_][A-Za-z0-9_]*" n != null);
  list = type: description: mkOption { type = types.listOf type; default = [ ]; inherit description; };
in
{
  _file = "graft";

  options = {
    packages = list types.package "Closure roots: on PATH, and with their closures, all the store the run sees.";
    protect = list repoPath ''
      Paths in the repository that are read-only inside, relative to its top. "**/NAME" is every
      file or directory named NAME (a file name pattern) anywhere in it, its scions too.'';
    env.set = mkOption {
      type = types.addCheck (types.attrsOf types.str) (s: lib.all (n: envName.check n) (lib.attrNames s))
        // { description = "attribute set of strings, named like environment variables"; };
      default = { };
      description = "Fixed values. They land in the world-readable Nix store: never secrets (env.pass).";
    };
    repository = mkOption {
      type = types.submodule {
        options.readOnly = mkOption {
          type = types.bool;
          default = false;
          description = "The consumer's alone: true makes the whole repository read-only (a review).";
        };
      };
      default = { };
      description = "The repository around the current directory, writable but for what protect and graft pin.";
    };

    # requests, from a provider: they apply when the consumer grants it
    paths.readOnly = list hostPath "Host paths the run reads.";
    paths.writable = list hostPath "Host paths the run writes.";
    network.allow = list host ''
      Hosts the run may reach, through an allowlisting proxy: "host" (ports 443 and 80) or
      "host:port". Only proxy-aware HTTP(S) works; [ ] is no network at all.'';
    env.pass = list envName "Variables copied from the caller's environment, when it sets them.";
    grant = list types.str "The consumer's alone: scions whose requests (the options above) apply.";
  };
}
