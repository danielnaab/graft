# The sandbox policy: graft's base, each provider's `sandbox` and the consumer's, merged as modules.
#
# A provider may add to what the run can see, never to what it can reach. Each provider's module is
# evaluated alone first; what it sees (packages, env.set, protect) joins the policy as plain values,
# and what it requests (paths, network, env.pass) joins only when the consumer's grant names it.
# Plain values carry no priority, so nothing a provider writes (a nested mkForce included) can
# override the consumer; an mkForce on a whole option is an error all the same, to say so.
#
# The result is the contract with renderers (bwrap.nix): `config` holds what a run gets, with no
# meaning left to add (network.allow as hosts and ports); `explain` says where each value came from.
{ lib, base, providers, consumer, scions, consumerFile ? "flake.nix (perSystem)" }:

let
  options = ./options.nix;
  see = [ [ "packages" ] [ "env" "set" ] [ "protect" ] ];
  reach = [ [ "paths" "readOnly" ] [ "paths" "writable" ] [ "network" "allow" ] [ "env" "pass" ] ];
  own = [ [ "repository" ] [ "grant" ] ];
  name = lib.concatStringsSep ".";
  get = p: lib.attrByPath p null;
  # config's values for the options ps, as one plain definition
  values = config: ps: lib.mkMerge (map (p: lib.setAttrByPath p (get p config)) ps);

  alone = lib.mapAttrs (n: { file, module }:
    let
      e = lib.evalModules { modules = [ options { _file = file; imports = [ module ]; } ]; };
      prio = p: (get p e.options).highestPrio;
      forced = lib.filter (p: prio p < 100) (see ++ reach);
      taken = lib.filter (p: prio p < 1500) own;
    in
    if forced != [ ] then throw "graft: sandbox: ${file} uses mkForce (or mkOverride below 100) on ${lib.concatMapStringsSep ", " name forced}: a provider adds to the policy, never overrides it"
    else if taken != [ ] then throw "graft: sandbox: ${file} sets ${lib.concatMapStringsSep ", " name taken}: only the consumer's perSystem may"
    else { inherit file; inherit (e) config; }) providers;

  consumerModule = { _file = consumerFile; imports = lib.optional (consumer != null) consumer; };
  # the grant alone, so the merged policy can depend on it
  granted = (lib.evalModules { modules = [ options consumerModule ]; }).config.grant;
  unknown = lib.filter (g: !(lib.elem g scions)) granted;

  eval = lib.evalModules {
    modules = [ options base consumerModule ] ++ lib.mapAttrsToList (n: p: {
      _file = p.file;
      config = values p.config (see ++ lib.optionals (lib.elem n granted) reach);
    }) alone;
  };
  c = eval.config;

  notGranted = lib.concatLists (lib.mapAttrsToList (n: p:
    lib.optionals (!(lib.elem n granted)) (lib.concatMap (o:
      map (v: { option = name o; value = v; inherit (p) file; scion = n; }) (get o p.config)) reach))
    alone);

  # "host" is ports 443 and 80; "host:port" that port
  hostPorts = e:
    let m = builtins.match "(.*):([0-9]+)" e; in
    if m == null then { host = e; ports = [ 443 80 ]; }
    else { host = lib.elemAt m 0; ports = [ (lib.toInt (lib.elemAt m 1)) ]; };

  # every value with the file that gives it
  show = v: if lib.isDerivation v then v.name else toString v;
  defs = p: lib.concatMap (d:
    if builtins.isList d.value
    then map (x: { value = show x; inherit (d) file; } // lib.optionalAttrs (lib.isDerivation x) { path = x.outPath; }) d.value
    else lib.mapAttrsToList (k: x: { value = "${k}=${x}"; inherit (d) file; }) d.value)
    (lib.filter (d: d.value != [ ] && d.value != { }) (get p eval.options).definitionsWithLocations);
  # (the checked value first: its type error names the option and the file)
  readOnlyDefs = let ds = lib.filter (d: d.value ? readOnly) eval.options.repository.definitionsWithLocations; in
    builtins.seq c.repository.readOnly (if ds == [ ] then [ { value = "false"; file = "graft"; } ]
    else map (d: { value = lib.boolToString d.value.readOnly; inherit (d) file; }) ds);
  all = see ++ [ [ "repository" "readOnly" ] ] ++ reach ++ [ [ "grant" ] ];
  explain = {
    options = lib.listToAttrs (map (p: lib.nameValuePair (name p)
      (if p == [ "repository" "readOnly" ] then readOnlyDefs else defs p)) all);
    inherit notGranted;
  };

  pad = s: s + lib.concatStrings (lib.genList (_: " ") (lib.max 2 (40 - lib.stringLength s)));
  line = d: "  ${pad d.value}${d.file}";
  explainText = ''
    sandbox policy: each value with the file that sets it (graft is graft's base)
    ${lib.concatMapStrings (p: let ds = explain.options.${name p}; in ''

      ${name p}
      ${if ds != [ ] then lib.concatMapStringsSep "\n" line ds
        else if p == [ "network" "allow" ] then "  (none: no network)" else "  (none)"}
    '') all}${lib.optionalString (notGranted != [ ]) ''

      requested, not granted (inert until perSystem's sandbox.grant names the scion)
      ${lib.concatMapStringsSep "\n" (r: line (r // { value = "${r.option} ${r.value}"; })) notGranted}
    ''}'';
in
if unknown != [ ] then throw "graft: sandbox: grant names ${lib.concatStringsSep ", " unknown}, which this repository does not graft"
else {
  # what a run gets: a renderer's whole input
  config = {
    packages = lib.unique c.packages;
    env = { inherit (c.env) set; pass = lib.unique c.env.pass; };
    protect = lib.unique c.protect;
    repository.readOnly = c.repository.readOnly;
    paths = { readOnly = lib.unique c.paths.readOnly; writable = lib.unique c.paths.writable; };
    network.allow = map hostPorts (lib.unique c.network.allow);
  };
  inherit explain explainText notGranted;
}
