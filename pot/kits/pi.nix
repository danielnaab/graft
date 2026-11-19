# graft.kits.pi: the pi coding agent (pi.dev) as an agent kit (design §6.3). A root calls it once, as the
# root kit pi, or once per instance, as pi@<instance> (the root's kits key names it):
#
#   graft.kits.pi {
#     pkgs;                        # the root's nixpkgs
#     binary ? "managed";          # "managed": ~/.pi/agent/bin/pi, pi's managed install, resolved on the host
#                                  # at each start; or a package with bin/pi
#     node ? pkgs.nodejs_22;       # on PATH: the managed launcher runs `env node`
#     agentDir ? ".pi/agent";      # the host's pi folder in ~ (its PI_CODING_AGENT_DIR): the managed install
#                                  # and the folders bound read-only come from it
#     files ? { };                 # home-manager's config.home.file: targets under .pi/agent/ are fixed
#     settings ? { };              # settings.json, fixed
#     providers ? { };             # models.json's providers, fixed: their reach and credentials come from them
#     endpoints ? [ ];             # [ { url; key ? "NAME"; } ]: APIs an extension reaches that models.json lacks
#     skillSources ? [ ];          # checkouts the skill links point into (read requests)
#     skillDirs ? [ ];             # real directories in the skills folder that are bound (by name)
#   }
#
# The run gets its own pi home: ~/.pi/agent is the run's state (per instance, workspace and context:
# sessions, project trust, caches), with pi's configuration fixed in it by the root: settings.json and
# models.json from the arguments, the home-manager files, and pi's other config files in a neutral form
# (auth.json and mcp-auth.json {}, no MCP servers, empty SYSTEM.md and APPEND_SYSTEM.md, AGENTS.override.md
# as the root's AGENTS.md), so a run leaves no configuration for a later one. extensions, skills, prompts,
# themes and the installed packages (npm, git) are read-only, from the host's folders. Reach is computed:
# each provider's and model's baseUrl, and each endpoint, is a network request (https HOST on 443, or
# HOST:PORT); each "$NAME" or "${NAME}" apiKey, and each endpoint's key, a credential for its host. pi runs
# offline at start (PI_OFFLINE: no npm installs, version checks or catalog fetches).
{ pkgs, binary ? "managed", node ? pkgs.nodejs_22, agentDir ? ".pi/agent", files ? { }, settings ? { }, providers ? { }, endpoints ? [ ]
, skillSources ? [ ], skillDirs ? [ ] }:

let
  lib = pkgs.lib;
  managed = binary == "managed";
  err = msg: throw "graft.kits.pi: ${msg}";
  upper = lib.stringToCharacters "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  lower = lib.stringToCharacters "abcdefghijklmnopqrstuvwxyz";
  # a url's network request: https HOST (443) or HOST:PORT; http HOST:PORT (80). The host's loopback is
  # the run's own, and host-loopback:PORT is reserved for a later release, so a loopback url fails.
  reachOf = what: url:
    let
      m = builtins.match "(https?)://([A-Za-z0-9.-]+|[[][0-9A-Fa-f:.]+[]])(:([0-9]+))?(/[^?#]*)?" url;
      scheme = builtins.elemAt m 0;
      host = builtins.replaceStrings upper lower (builtins.elemAt m 1);
      port = let p = builtins.elemAt m 3; in if p != null then lib.toInt p else if scheme == "https" then 443 else 80;
      loopback = host == "localhost" || lib.hasSuffix ".localhost" host || lib.hasPrefix "127." host
        || host == "[::1]" || host == "0.0.0.0" || host == "[::]";
    in
    if !(builtins.isString url) || m == null then err "${what}: url ${builtins.toJSON url}: give http(s)://HOST[:PORT][/PATH]"
    else if loopback then err "${what}: ${url} is the host's loopback, which a run cannot reach (host-loopback is reserved for a later release): leave it out"
    else { inherit host; request = if port == 443 then host else "${host}:${toString port}"; };
  # an apiKey (or an endpoint's key) as the variable it names: "$NAME" or "${NAME}"; a literal is no secret
  keyOf = what: k:
    let m = builtins.match "\\$\\{?([A-Za-z_][A-Za-z0-9_]*)}?" k; in
    if k == null then null
    else if !(builtins.isString k) then err "${what}: apiKey must be a string"
    else if lib.hasPrefix "!" k then err "${what}: a command apiKey (\"!…\") runs a program in the run: give \"\${NAME}\" and bind NAME"
    else if m != null && (lib.hasPrefix "\${" k) == (lib.hasSuffix "}" k) then builtins.head m
    else if lib.hasInfix "$" k then err "${what}: apiKey ${builtins.toJSON k}: give \"\${NAME}\" or a literal"
    else null;
  # every API the run reaches: { what; host; request; env; }
  providerApis = lib.concatLists (lib.mapAttrsToList (name: p:
    let
      what = "provider ${name}";
      urls = lib.optional (p ? baseUrl) p.baseUrl ++ map (m: m.baseUrl) (lib.filter (m: m ? baseUrl) (p.models or [ ]));
      env = keyOf what (p.apiKey or null);
    in
    if !(p ? baseUrl) then err "${what} has no baseUrl (pi's built-in providers are not in this kit): leave it out"
    else map (u: reachOf what u // { inherit env; }) (lib.unique urls)) providers);
  endpointApis = lib.imap1 (i: e:
    let what = "endpoints #${toString i}"; in
    if !(builtins.isAttrs e) || !(e ? url) || builtins.removeAttrs e [ "url" "key" ] != { }
    then err "${what}: give { url; key ? \"NAME\"; }"
    else reachOf what e.url // { env = keyOf what (if e ? key && e.key != null then "\${${e.key}}" else null); }) endpoints;
  apis = providerApis ++ endpointApis;
  network = lib.unique (map (a: a.request) apis);
  # one credential per host: the variables that name its keys
  credentialHosts = lib.unique (map (a: a.host) (lib.filter (a: a.env != null) apis));
  credentials = map (h: let envs = lib.unique (map (a: a.env) (lib.filter (a: a.host == h && a.env != null) apis)); in
    { name = "credential"; host = h; env = if lib.length envs == 1 then lib.head envs else envs; }) credentialHosts;

  # home-manager's files in pi's home, by target (store sources), except the folders bound below
  prefix = "${agentDir}/";
  fixedDirNames = [ "extensions" "skills" "prompts" "themes" "npm" "git" ];
  hmFiles = lib.filterAttrs (t: f: lib.hasPrefix prefix t && f ? source
      && !(lib.elem (lib.head (lib.splitString "/" (lib.removePrefix prefix t))) fixedDirNames))
    (lib.mapAttrs' (_: f: lib.nameValuePair (f.target or "") f) files);
  hmSource = n: (hmFiles."${prefix}${n}" or { }).source or null;
  text = n: s: pkgs.writeText "pot-pi-${lib.replaceStrings [ "." ] [ "-" ] n}" s;
  json = n: v: text n (builtins.toJSON v);
  # pi's other config files: neutral unless the root gives them. Empty SYSTEM.md and APPEND_SYSTEM.md are
  # pi's defaults; AGENTS.override.md, the first context file pi reads, is the root's AGENTS.md.
  neutral = {
    "keybindings.json" = json "keybindings.json" { };
    "mcp.json" = json "mcp.json" { mcpServers = { }; };
    "SYSTEM.md" = text "SYSTEM.md" "";
    "APPEND_SYSTEM.md" = text "APPEND_SYSTEM.md" "";
    "AGENTS.override.md" = let a = hmSource "AGENTS.md"; in if a != null then a else text "AGENTS.override.md" "";
  };
  generated = {
    "settings.json" = json "settings.json" settings;
    "models.json" = json "models.json" { inherit providers; };
    # credential files: a /login in a run cannot persist (log in on the host)
    "auth.json" = json "auth.json" { };
    "mcp-auth.json" = json "mcp-auth.json" { };
  };
  fixedFiles = neutral // lib.mapAttrs' (t: f: lib.nameValuePair (lib.removePrefix prefix t) f.source) hmFiles // generated;

  # pot host --resume: a session (pi's JSONL) as text: each user and assistant message's text
  render = pkgs.writeShellScript "pot-pi-transcript" ''
    exec ${pkgs.jq}/bin/jq -R -s -r '
      split("\n") | map(fromjson? // empty | objects | select(.type == "message")
        | ((.message | objects) // {}) as $m | $m.role as $role
        | ($m.content | if type == "array" then map(objects | select(.type == "text") | (.text // "") | strings) | join("\n") else . end) as $c
        | select(($role == "user" or $role == "assistant") and ($c | type) == "string")
        | ($c | gsub("^\\s+|\\s+$"; "")) as $t | select($t != "") | "[\($role)] \($t)")
      | join("\n\n")' "$1"
  '';
in
assert lib.isDerivation binary || managed || err "binary = \"managed\" or a package";
{
  version = "1.1";
  tools = [ node ] ++ lib.optional (lib.isDerivation binary) binary;
  files = lib.mapAttrs' (n: s: lib.nameValuePair "~/.pi/agent/${n}" { source = s; mode = "fixed"; }) fixedFiles;
  env.set = { PI_CODING_AGENT_DIR = "~/.pi/agent"; PI_OFFLINE = "1"; PI_SKIP_VERSION_CHECK = "1"; PI_TELEMETRY = "0"; };
  # what pot's core knows of pi, as data (kit 1.1): pi marks its children, keeps its sessions per working
  # directory, and resumes the newest with --continue (a session file's name is not an id pi matches)
  agent = {
    markers = [ "PI_CODING_AGENT" ];
    resume = { transcripts = "~/.pi/agent/sessions/**/*.jsonl"; args = [ "--continue" ]; render = "${render}"; };
  };
  commands.pi = {
    program = if managed then { host = "~/${agentDir}/bin/pi"; } else "${binary}/bin/pi";
    # the host's extensions, skills, prompts and themes (links bound by target), and its installed packages
    dirs = lib.listToAttrs (map (d: lib.nameValuePair "~/.pi/agent/${d}" {
      from = "~/${agentDir}/${d}"; real = if d == "skills" then skillDirs else if d == "npm" || d == "git" then true else [ ]; })
      fixedDirNames);
    requests = {
      read = lib.optionals managed [ "~/${agentDir}/bin" "~/${agentDir}/install" ] ++ skillSources;
      inherit network;
      state = [ { name = "pi"; at = "~/.pi/agent"; } ];
      capabilities = [ { name = "status.relay"; } ] ++ credentials;
    };
  };
}
