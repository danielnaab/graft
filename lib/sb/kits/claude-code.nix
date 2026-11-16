# graft.kits.claude-code: Claude Code as an agent kit (design §6.3, §13.2). A root calls it once per
# account, as the root kit claude-code@<account>:
#
#   graft.kits.claude-code {
#     pkgs;                        # the root's nixpkgs
#     account ? "default";         # ai-profiles slot (the instance name is the root's kits key)
#     binary ? "native";           # "native": ~/.local/bin/claude, resolved on the host; or a package
#     files ? { };                 # home-manager's config.home.file: fixed files come from it
#     configDir ? ".claude";       # this account's folder in ~ (".claude-accounts/flexion")
#     skillDirs ? [ ];             # real directories in the skills folder that are bound (by name)
#     skillSources ? [ ];          # checkouts the skill links point into (read requests)
#     provider ? "anthropic";      # or "bedrock", with region
#     region ? null;
#     mcp ? [ ];                   # MCP server names, each a `mcp` request
#   }
#
# The run gets its own agent home: ~/.claude is the run's state (per instance, workspace and
# context), with the account's home-manager files fixed in it, its skills, agents, commands, hooks
# and plugins read-only, and onboarding and folder trust reasserted in ~/.claude/.claude.json. The
# `--settings` overlay turns Claude's own sandbox off (Claude treats it as managed) and adds the
# denial hook (PostToolUse on Bash and WebFetch): new lines of /run/graft-sandbox/denials reach the
# model as additionalContext. Overlay hooks are added to the user's own; `sb explain` lists them.
{ pkgs, account ? "default", binary ? "native", files ? { }, configDir ? ".claude", skillDirs ? [ ]
, skillSources ? [ ], provider ? "anthropic", region ? null, mcp ? [ ] }:

let
  lib = pkgs.lib;
  native = binary == "native";
  host = if provider == "bedrock" then "bedrock-runtime.${region}.amazonaws.com" else "api.anthropic.com";
  # the loader a foreign binary names (FHS), and glibc's own, which nix-ld hands it to (NIX_LD)
  fhsLinker = if pkgs.stdenv.hostPlatform.isAarch64 then "/lib/ld-linux-aarch64.so.1" else "/lib64/ld-linux-x86-64.so.2";
  linker = pkgs.stdenv.cc.bintools.dynamicLinker;
  # the account's home-manager files in its config folder, by target (store sources)
  prefix = "${configDir}/";
  fixedDirNames = [ "skills" "agents" "commands" "hooks" "plugins" ];
  hmFiles = lib.filterAttrs (t: f: lib.hasPrefix prefix t && f ? source
      && !(lib.elem (lib.head (lib.splitString "/" (lib.removePrefix prefix t))) fixedDirNames))
    (lib.mapAttrs' (_: f: lib.nameValuePair (f.target or "") f) files);
  denialHook = pkgs.writeShellScript "sb-denial-hook" ''
    # graft sb's PostToolUse hook: new sandbox denials since the last call, as context for the model
    d=/run/graft-sandbox/denials o=''${TMPDIR:-/tmp}/.sb-denials-offset
    [ -s "$d" ] || exit 0
    size=$(${pkgs.coreutils}/bin/stat -c %s "$d"); seen=$(${pkgs.coreutils}/bin/cat "$o" 2>/dev/null || echo 0)
    [ "$size" -gt "$seen" ] || exit 0
    echo "$size" > "$o"
    ${pkgs.coreutils}/bin/tail -c +"$((seen + 1))" "$d" | ${pkgs.jq}/bin/jq -Rs '{hookSpecificOutput: {hookEventName: "PostToolUse",
      additionalContext: ("The sandbox (graft sb) refused this during the command; read /run/graft-sandbox/README.md:\n" + .)}}'
  '';
  overlay = pkgs.writeText "sb-claude-overlay.json" (builtins.toJSON {
    sandbox.enabled = false;
    hooks.PostToolUse = [ { matcher = "Bash|WebFetch"; hooks = [ { type = "command"; command = "${denialHook}"; } ]; } ];
  });
  onboarding = pkgs.writeText "sb-claude-onboarding.json" (builtins.toJSON {
    hasCompletedOnboarding = true;
    projects."*".hasTrustDialogAccepted = true;
  });
in
{
  packages = lib.optional (lib.isDerivation binary) binary ++ lib.optionals native [ pkgs.glibc ];
  files = lib.mapAttrs' (t: f: lib.nameValuePair "~/.claude/${lib.removePrefix prefix t}" { source = f.source; mode = "fixed"; }) hmFiles
    // { "~/.claude/.claude.json" = { source = onboarding; mode = "reassert"; keys = [ "hasCompletedOnboarding" "projects.*.hasTrustDialogAccepted" ]; }; }
    // lib.optionalAttrs native { ${fhsLinker} = { source = "${pkgs.nix-ld}/libexec/nix-ld"; mode = "fixed"; }; };
  env.set = { CLAUDE_CONFIG_DIR = "~/.claude"; DISABLE_AUTOUPDATER = "1"; }
    // lib.optionalAttrs native {
      NIX_LD = linker;
      NIX_LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib ];
    };
  protect = [ "**/.claude" "**/.mcp.json" ];
  commands.claude = {
    program = if native then { host = "~/.local/bin/claude"; } else "${binary}/bin/claude";
    args = [ "--settings" overlay ]
      ++ lib.optionals (mcp != [ ]) [ "--mcp-config" "/run/graft-sandbox/mcp.json" "--strict-mcp-config" ];
    fixedDirs = lib.genAttrs (map (d: "~/.claude/${d}") fixedDirNames) (d: "~/${configDir}/${baseNameOf d}");
    inherit skillDirs;
    requests = {
      read = lib.optional native "~/.local/share/claude" ++ skillSources;
      network = [ host ] ++ lib.optional (provider == "anthropic") "claude.ai";
      env = lib.optionals (provider == "bedrock") [ "CLAUDE_CODE_USE_BEDROCK" "ANTHROPIC_MODEL" "AWS_REGION" ];
      state = [ { name = "claude"; at = "~/.claude"; } ];
      capabilities = [ { name = "status.relay"; }
        { name = "credential"; inherit host;
          env = if provider == "bedrock" then [ "AWS_ACCESS_KEY_ID" "AWS_SECRET_ACCESS_KEY" "AWS_SESSION_TOKEN" ] else "CLAUDE_CODE_OAUTH_TOKEN"; } ]
        ++ map (s: { name = "mcp"; server = s; }) mcp;
    };
  };
}
