# graft.kits.claude-code: Claude Code as an agent kit (design §6.3, §13.2). A root calls it once per
# account, as the root kit claude-code@<account>:
#
#   graft.kits.claude-code {
#     pkgs;                        # the root's nixpkgs
#     account ? "default";         # ai-profiles slot (the instance name is the root's kits key)
#     binary ? "native";           # "native": ~/.local/bin/claude; "host": the claude on the host's PATH;
#                                  # either resolved on the host at start; or a package
#     files ? { };                 # home-manager's config.home.file: fixed files come from it
#     configDir ? ".claude";       # this account's folder in ~ (".claude-accounts/flexion")
#     skillDirs ? [ "synced" ];    # real directories in the skills folder that are bound (by name; [ ] opts out)
#     plugins ? true;              # bind the account's plugins folder read-only (false opts out)
#     skillSources ? [ ];          # checkouts the skill links point into (read requests)
#     api ? graft.apis.anthropic;  # or graft.apis.bedrock { region = "us-east-1"; }
#                                  # (provider and region: the old form, until v2.17.0)
#     mcp ? [ ];                   # MCP server names, each a `mcp` request
#   }
#
# The run gets its own agent home: ~/.claude is the run's state (per instance, workspace and
# context), with the account's home-manager files fixed in it, its skills, agents, commands, hooks
# and plugins read-only, the host's project memory for the workspace read-only (memory the run
# writes stays in its state, and the exit line names it), and onboarding and folder trust
# reasserted in ~/.claude/.claude.json. The
# `--settings` overlay turns Claude's own sandbox off (Claude treats it as managed) and adds the
# denial hook (PostToolUse on Bash and WebFetch): new lines of /run/pot/denials reach the
# model as additionalContext. Overlay hooks are added to the user's own; `pot explain` lists them.
{ pkgs, account ? "default", binary ? "native", files ? { }, configDir ? ".claude", skillDirs ? [ "synced" ]
, plugins ? true, skillSources ? [ ], api ? null, provider ? null, region ? null, mcp ? [ ] }:

let
  lib = pkgs.lib;
  apis = import ./apis.nix;
  # api, or v2.15.0's provider and region (deprecated in v2.16.0, removed in v2.17.0)
  old = provider != null || region != null;
  api' =
    if api != null && old then throw "graft.kits.claude-code: give api, or provider and region (the old form), not both"
    else if api != null then api
    else if !old then apis.anthropic
    else lib.warn "graft.kits.claude-code: provider and region are api = graft.apis.bedrock { region = …; } (or graft.apis.anthropic); they go in v2.17.0"
      (if provider == "bedrock" then apis.bedrock { inherit region; } else apis.anthropic);
  provider' = api'.name;
  region' = api'.region or null;
  native = binary == "native";
  hostBinary = binary == "native" || binary == "host";
  # a host binary built for an FHS system gets nix-ld at the FHS loader's path (Linux)
  fhs = hostBinary && pkgs.stdenv.hostPlatform.isLinux;
  bedrock = provider' == "bedrock";
  host = if bedrock then "bedrock-runtime.${region'}.amazonaws.com" else "api.anthropic.com";
  # the loader a foreign binary names (FHS), and glibc's own, which nix-ld hands it to (NIX_LD)
  fhsLinker = if pkgs.stdenv.hostPlatform.isAarch64 then "/lib/ld-linux-aarch64.so.1" else "/lib64/ld-linux-x86-64.so.2";
  linker = pkgs.stdenv.cc.bintools.dynamicLinker;
  # the account's home-manager files in its config folder, by target (store sources)
  prefix = "${configDir}/";
  fixedDirNames = [ "skills" "agents" "commands" "hooks" "plugins" ];
  hmFiles = lib.filterAttrs (t: f: lib.hasPrefix prefix t && f ? source
      && !(lib.elem (lib.head (lib.splitString "/" (lib.removePrefix prefix t))) fixedDirNames))
    (lib.mapAttrs' (_: f: lib.nameValuePair (f.target or "") f) files);
  # PreToolUse records where the denials file ends; PostToolUse passes on what the tool added since,
  # so refusals from before the first tool (Claude's own startup traffic) go only to the exit line
  mark = pkgs.writeShellScript "pot-denial-mark" ''
    d=/run/pot/denials
    ${pkgs.coreutils}/bin/stat -c %s "$d" > "''${TMPDIR:-/tmp}/.pot-denials-offset" 2>/dev/null || true
  '';
  denialHook = pkgs.writeShellScript "pot-denial-hook" ''
    # pot's PostToolUse and PostToolUseFailure hook (Claude runs the second for a failed call):
    # sandbox denials during this tool call, as context for the model, under the event Claude names
    ev=$(${pkgs.jq}/bin/jq -r '.hook_event_name // "PostToolUse"' 2>/dev/null) || ev=PostToolUse
    d=/run/pot/denials o=''${TMPDIR:-/tmp}/.pot-denials-offset
    [ -s "$d" ] && [ -f "$o" ] || exit 0
    size=$(${pkgs.coreutils}/bin/stat -c %s "$d"); seen=$(${pkgs.coreutils}/bin/cat "$o")
    [ "$size" -gt "$seen" ] || exit 0
    echo "$size" > "$o"
    ${pkgs.coreutils}/bin/tail -c +"$((seen + 1))" "$d" | ${pkgs.jq}/bin/jq -Rs --arg ev "$ev" '{hookSpecificOutput: {hookEventName: $ev,
      additionalContext: ("pot refused this during the command; read /run/pot/README.md:\n" + .)}}'
  '';
  # at start: what the model should know about its memory here
  sessionHook = pkgs.writeShellScript "pot-session-start" ''
    echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"In this pot run, the host memory files are read-only and MEMORY.md is a writable copy: write new memory as new files in the usual memory folder; pot copies new files and a changed MEMORY.md out for the person to review when the run ends."}}'
  '';
  hookFor = c: [ { matcher = "Bash|WebFetch"; hooks = [ { type = "command"; command = "${c}"; } ]; } ];
  overlayHooks = { PreToolUse = hookFor mark; PostToolUse = hookFor denialHook; PostToolUseFailure = hookFor denialHook;
                   SessionStart = [ { hooks = [ { type = "command"; command = "${sessionHook}"; } ]; } ]; };
  overlay = pkgs.writeText "pot-claude-overlay.json" (builtins.toJSON ({
    sandbox.enabled = false;
    hooks = overlayHooks;
  } // lib.optionalAttrs bedrock {
    # nothing inside can refresh AWS credentials (no aws CLI, no SSO): the host refreshes them
    awsAuthRefresh = "";
  }));
  onboarding = pkgs.writeText "pot-claude-onboarding.json" (builtins.toJSON {
    hasCompletedOnboarding = true;
    projects."*".hasTrustDialogAccepted = true;
  });
  # pot host --resume: a transcript (Claude's JSONL) as text: each user and assistant message's text
  render = pkgs.writeShellScript "pot-claude-transcript" ''
    exec ${pkgs.jq}/bin/jq -R -s -r '
      split("\n") | map(fromjson? // empty | objects
        | ((.message | objects) // {}) as $m | ($m.role // .type) as $role
        | ($m.content | if type == "array" then map(objects | select(.type == "text") | (.text // "") | strings) | join("\n") else . end) as $c
        | select(($role == "user" or $role == "assistant") and ($c | type) == "string")
        | ($c | gsub("^\\s+|\\s+$"; "")) as $t | select($t != "") | "[\($role)] \($t)")
      | join("\n\n")' "$1"
  '';
  fixedDirs = lib.filter (d: plugins || d != "plugins") fixedDirNames;
in
{
  version = "1.1";
  tools = lib.optional (lib.isDerivation binary) binary ++ lib.optionals fhs [ pkgs.glibc ];
  files = lib.mapAttrs' (t: f: lib.nameValuePair "~/.claude/${lib.removePrefix prefix t}" { source = f.source; mode = "fixed"; }) hmFiles
    // { "~/.claude/.claude.json" = { source = onboarding; mode = "reassert"; keys = [ "hasCompletedOnboarding" "projects.*.hasTrustDialogAccepted" ]; }; }
    // lib.optionalAttrs fhs { ${fhsLinker} = { source = "${pkgs.nix-ld}/libexec/nix-ld"; mode = "fixed"; }; };
  env.set = { CLAUDE_CONFIG_DIR = "~/.claude"; DISABLE_AUTOUPDATER = "1"; CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"; }
    // lib.optionalAttrs fhs {
      NIX_LD = linker;
      NIX_LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ];
    };
  protect = [ "**/.claude" "**/.mcp.json" ];
  # what pot's core knows of Claude Code, as data (kit 1.1): how a caller in Claude is recognised, which
  # caller variables select this instance (Bedrock's add CLAUDE_CODE_USE_BEDROCK, so they are the more
  # specific match), where its transcripts are and how a session resumes, and the overlay's hooks
  agent = {
    markers = [ "CLAUDECODE" ];
    select.env = { CLAUDE_CONFIG_DIR = "~/${configDir}"; } // lib.optionalAttrs bedrock { CLAUDE_CODE_USE_BEDROCK = "1"; };
    resume = { transcripts = "~/.claude/projects/**/*.jsonl"; args = [ "--resume" "{id}" ]; render = "${render}"; };
    hooks = lib.concatLists (lib.mapAttrsToList (ev: rules:
      map (r: "${ev} ${r.matcher or "*"} (pot's --settings overlay; /hooks does not list it)") rules) overlayHooks);
  };
  commands.claude = {
    program = if native then { host = "~/.local/bin/claude"; } else if binary == "host" then { host = "claude"; } else "${binary}/bin/claude";
    args = [ "--settings" overlay ]
      ++ lib.optionals (mcp != [ ]) [ "--mcp-config" "/run/pot/mcp.json" "--strict-mcp-config" ];
    # the account's skills, agents, commands, hooks and plugins, read-only; the host's project memory for
    # each workspace mirrored (MEMORY.md a writable copy; new files copied out at exit)
    dirs = lib.listToAttrs (map (d: lib.nameValuePair "~/.claude/${d}" {
        from = "~/${configDir}/${d}"; real = if d == "plugins" then true else skillDirs; }) fixedDirs)
      // { "~/.claude/projects/{workspace}/memory" = { mode = "mirror"; from = "~/${configDir}/projects/{workspace}/memory";
             copy = [ "MEMORY.md" ]; label = "memory"; }; };
    requests = {
      read = lib.optional hostBinary "~/.local/share/claude" ++ skillSources;
      network = [ host ] ++ lib.optional (provider' == "anthropic") "claude.ai"
        ++ lib.optionals bedrock [ "bedrock.${region'}.amazonaws.com" "sts.${region'}.amazonaws.com" ];
      env = lib.optionals bedrock [ "CLAUDE_CODE_USE_BEDROCK" "ANTHROPIC_MODEL" "AWS_REGION" ];
      state = [ { name = "claude"; at = "~/.claude"; } ];
      capabilities = [ { name = "status.relay"; }
        { name = "credential"; inherit host;
          env = if bedrock then [ "AWS_ACCESS_KEY_ID" "AWS_SECRET_ACCESS_KEY" "AWS_SESSION_TOKEN" "AWS_CREDENTIAL_EXPIRATION" ]
                else "CLAUDE_CODE_OAUTH_TOKEN"; } ]
        ++ map (s: { name = "mcp"; server = s; }) mcp;
    };
  };
}
