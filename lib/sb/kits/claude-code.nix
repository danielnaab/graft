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
#     provider ? "anthropic";      # or "bedrock", with region
#     region ? null;
#     mcp ? [ ];                   # MCP server names, each a `mcp` request
#   }
#
# The run gets its own agent home: ~/.claude is the run's state (per instance, workspace and
# context), with the account's home-manager files fixed in it, its skills, agents, commands, hooks
# and plugins read-only, the host's project memory for the workspace read-only (memory the run
# writes stays in its state, and the exit line names it), and onboarding and folder trust
# reasserted in ~/.claude/.claude.json. The
# `--settings` overlay turns Claude's own sandbox off (Claude treats it as managed) and adds the
# denial hook (PostToolUse on Bash and WebFetch): new lines of /run/graft-sandbox/denials reach the
# model as additionalContext. Overlay hooks are added to the user's own; `sb explain` lists them.
{ pkgs, account ? "default", binary ? "native", files ? { }, configDir ? ".claude", skillDirs ? [ "synced" ]
, plugins ? true, skillSources ? [ ], provider ? "anthropic", region ? null, mcp ? [ ] }:

let
  lib = pkgs.lib;
  native = binary == "native";
  hostBinary = binary == "native" || binary == "host";
  # a host binary built for an FHS system gets nix-ld at the FHS loader's path (Linux)
  fhs = hostBinary && pkgs.stdenv.hostPlatform.isLinux;
  bedrock = provider == "bedrock";
  host = if bedrock then "bedrock-runtime.${region}.amazonaws.com" else "api.anthropic.com";
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
  mark = pkgs.writeShellScript "sb-denial-mark" ''
    d=/run/graft-sandbox/denials
    ${pkgs.coreutils}/bin/stat -c %s "$d" > "''${TMPDIR:-/tmp}/.sb-denials-offset" 2>/dev/null || true
  '';
  denialHook = pkgs.writeShellScript "sb-denial-hook" ''
    # graft sb's PostToolUse hook: sandbox denials during this tool call, as context for the model
    d=/run/graft-sandbox/denials o=''${TMPDIR:-/tmp}/.sb-denials-offset
    [ -s "$d" ] && [ -f "$o" ] || exit 0
    size=$(${pkgs.coreutils}/bin/stat -c %s "$d"); seen=$(${pkgs.coreutils}/bin/cat "$o")
    [ "$size" -gt "$seen" ] || exit 0
    echo "$size" > "$o"
    ${pkgs.coreutils}/bin/tail -c +"$((seen + 1))" "$d" | ${pkgs.jq}/bin/jq -Rs '{hookSpecificOutput: {hookEventName: "PostToolUse",
      additionalContext: ("The sandbox (graft sb) refused this during the command; read /run/graft-sandbox/README.md:\n" + .)}}'
  '';
  hookFor = c: [ { matcher = "Bash|WebFetch"; hooks = [ { type = "command"; command = "${c}"; } ]; } ];
  overlay = pkgs.writeText "sb-claude-overlay.json" (builtins.toJSON ({
    sandbox.enabled = false;
    hooks = { PreToolUse = hookFor mark; PostToolUse = hookFor denialHook; };
  } // lib.optionalAttrs bedrock {
    # nothing inside can refresh AWS credentials (no aws CLI, no SSO): the host refreshes them
    awsAuthRefresh = "";
  }));
  onboarding = pkgs.writeText "sb-claude-onboarding.json" (builtins.toJSON {
    hasCompletedOnboarding = true;
    projects."*".hasTrustDialogAccepted = true;
  });
in
{
  packages = lib.optional (lib.isDerivation binary) binary ++ lib.optionals fhs [ pkgs.glibc ];
  files = lib.mapAttrs' (t: f: lib.nameValuePair "~/.claude/${lib.removePrefix prefix t}" { source = f.source; mode = "fixed"; }) hmFiles
    // { "~/.claude/.claude.json" = { source = onboarding; mode = "reassert"; keys = [ "hasCompletedOnboarding" "projects.*.hasTrustDialogAccepted" ]; }; }
    // lib.optionalAttrs fhs { ${fhsLinker} = { source = "${pkgs.nix-ld}/libexec/nix-ld"; mode = "fixed"; }; };
  env.set = { CLAUDE_CONFIG_DIR = "~/.claude"; DISABLE_AUTOUPDATER = "1"; CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"; }
    // lib.optionalAttrs fhs {
      NIX_LD = linker;
      NIX_LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ];
    };
  protect = [ "**/.claude" "**/.mcp.json" ];
  commands.claude = {
    program = if native then { host = "~/.local/bin/claude"; } else if binary == "host" then { host = "claude"; } else "${binary}/bin/claude";
    args = [ "--settings" overlay ]
      ++ lib.optionals (mcp != [ ]) [ "--mcp-config" "/run/graft-sandbox/mcp.json" "--strict-mcp-config" ];
    fixedDirs = lib.genAttrs (map (d: "~/.claude/${d}") (lib.filter (d: plugins || d != "plugins") fixedDirNames))
      (d: "~/${configDir}/${baseNameOf d}");
    realDirs = lib.optional plugins "~/.claude/plugins";
    memory = { at = "~/.claude/projects"; host = "~/${configDir}/projects"; };
    inherit skillDirs;
    requests = {
      read = lib.optional hostBinary "~/.local/share/claude" ++ skillSources;
      network = [ host ] ++ lib.optional (provider == "anthropic") "claude.ai"
        ++ lib.optionals bedrock [ "bedrock.${region}.amazonaws.com" "sts.${region}.amazonaws.com" ];
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
