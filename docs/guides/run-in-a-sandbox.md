---
status: working
---
# Run commands in pot

Run Claude Code, or any command, with pot, graft's sandbox, so that it can change your repository but cannot read your keys,
reach hosts that you did not grant, or change what decides the sandbox. `pot` is experimental in v2.16.1. The terms
are in [pot](../reference/pot.md#the-model).

## Before you begin

- Linux, with unprivileged user namespaces (the default on NixOS, Debian, Ubuntu and Fedora). On macOS, `pot`
  refuses to start in v2.16.1: stay on graft v2.14.0's sandbox there ([On macOS](../reference/pot.md#on-macos)).
- A root, the place where your grants live ([Set up a root](#set-up-a-root)).
- Claude Code installed on the host.
- tmux, if you want to see denials while the agent runs. Without it, you see them when the run ends.
- The steps use `team-kb`, a repository in `~/src`, a home-manager root in `~/.workstation`, and the account
  `personal`.

## Steps

1. Make a login token for the account, once per account:

   ```
   claude setup-token
   ```

   It prints a long-lived token. Save it in the file that your root's `credential` binding names, readable only by
   you, such as `~/.config/pot/claude-token-personal` (`chmod 600`). The file holds the token alone, or a line
   `CLAUDE_CODE_OAUTH_TOKEN=…`. Do not paste it on a command line. `pot` hides the file from every run.

2. Start Claude in the repository:

   ```
   cd ~/src/team-kb
   pot claude
   ```

3. The first time, `pot` shows the grant prompt before the run. It lists what the claude-code kit asks for,
   ordinary items together and each powerful item on its own:

   ```
   claude-code@personal   root kit (root:claude-code@personal), for every run that uses it   not granted
     ordinary:
       + capability status.relay  bound: workmux relay
       + network api.anthropic.com:443
       + network claude.ai:443
       + read ~/.local/share/claude
       + state claude
     grant these? [y/N]
   ```

   Type `y` for each set that you accept. Each powerful item, such as the login `credential`, asks on its own:
   `grant? [y/N]`. The repo kit and any scion kits that need a grant come next, in the same prompt. The prompt
   appears only at a terminal; elsewhere the run stops with exit 125 and `pot grant KIT`.

4. Claude starts. The startup line names what differs from the usual shape, and `pot explain`:

   ```
   pot: work · team-kb · pot explain
   ```

5. `pot` prints `wrote ~/.workstation/pot/grants.json. Commit it with your root's next change.` Commit it:

   ```
   git -C ~/.workstation add pot/grants.json
   git -C ~/.workstation commit -m "pot: grant claude-code@personal"
   ```

   From now on, `pot claude` starts with no prompt, until a kit asks for more.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the run is in `work`, can write the repository,
cannot write `justfile`, and cannot see `~/.ssh`:

```sh
pot bash -c 'test "$POT" = work && touch .pot-check && ! touch justfile 2>/dev/null && ! test -e ~/.ssh' &&
  test -e .pot-check && rm -f .pot-check && echo ok
```

If the repository has no `justfile`, write another [protected path](../reference/pot.md#protected-paths),
such as `flake.nix`.

## Choose a context

| You want to | Run | The run |
|---|---|---|
| Work on this repository | `pot claude` | writes the repository in place, reaches only what is granted |
| Diagnose the machine, change nothing | `pot inspect claude` | reads the machine except secrets and hidden paths, writes nothing, reaches only the model API |
| Hand a task off, or change a root repository | `pot dispatch claude` | works in a clone in pot's state; you review and fetch its commits ([Work in a clone](dispatch.md)) |
| Do what only the host may do | `pot host CMD`, or plain `claude` | has no sandbox |

`inspect` diagnoses; the host repairs. `sudo`, `systemctl restart`, `docker run` and `home-manager switch` are
host-only. To carry a diagnosis from a run to the host, as quoted data: `pot host --resume RUN claude`.

In workmux, set the agent command to `pot claude`, and turn off workmux's own Docker sandbox
(`workmux.sandbox.enable = false`; the home-manager module asserts it). The `status.relay` capability keeps the
pane's status working.

## When something is denied

A denial reaches you in three ways. Under tmux, a message shows on the agent's pane:

```
pot (work): the run tried pypi.org:443 (not in this run) · pot why pypi.org
```

The agent reads it from the denials file, through its hook, and tells you in its own words. When the run ends,
the exit line lists it:

```
pot (work): the run tried pypi.org:443 (not in this run).
  one run:   pot --net pypi.org …
  permanent: requests.network in the repo kit (flake.nix, perSystem)
             or programs.pot.kits.standing.requests.network, for every repository (home-manager switch)
```

1. Ask `pot` where the item would come from:

   ```
   pot why pypi.org
   ```

2. To allow it for one run, give the flag that `pot why` names:

   ```
   pot --net pypi.org claude --continue
   ```

   The exit line names the permanent fix for each flag that you used.

3. To allow it in every run of this repository, add it to the repo kit (see [Add a repo kit](#add-a-repo-kit)):

   ```nix
   kit.requests.network = [ "pypi.org" ];
   ```

   To allow it in every repository, add it to `standing`, the context's included kit
   (`programs.pot.kits.standing`, below), then `home-manager switch`.

4. Run `pot grant`, and accept the new item. Commit the grants file.

## When a kit asks for more

When an upgrade gives a kit new requests, or a new source for a powerful one, the next run shows the difference.
At a terminal, it is the grant prompt, inline: accept or refuse each item, and the run goes on. Elsewhere (an
agent, a script), the run stops with exit 125 and the difference:

1. Read the difference that `pot` printed, or run `pot grant --diff`. A stopped run is listed by `pot runs` as
   `stopped`, and a workmux pane that started it shows `waiting`.
2. If you accept it, run `pot grant` at a terminal: with no argument it offers every kit here that is stale, and
   every kit a stopped run waits for, showing only what changed. Or name one, `pot grant KIT`, with the short name
   that `pot` printed (`claude-code` names every instance). Add `--commit` to commit the grants file at once (only
   that file; it never pushes), or commit it with your root's next change. Otherwise, take back the upgrade.

Runs that the kit's requests do not apply to still start, with a note on the startup line:
`meeting-kit asks for more: pot grant`.

## Root repositories

Your root's own repository (`~/.workstation` here) decides what every run may do, so `pot` never writes it in
place. `pot claude` refuses there:

```
pot (work): ~/.workstation is a root: pot dispatch CMD (a reviewed branch), or pot inspect CMD
```
 <!-- TBD: sample -->

- To read it, use `pot inspect claude`.
- To change it, use `pot dispatch claude`: the change comes back as a branch for you to review, fetch and merge
  before `home-manager switch` ([Work in a clone](dispatch.md#change-a-root-repository)).
- In its worktrees, set workmux's agent command in `~/.workstation/.workmux.yaml` to anything but `pot claude`,
  because the project file overrides the global `agent`:

  ```yaml
  agent: pot dispatch claude
  ```

## Add a repo kit

A repository's own needs are its repo kit: the packages its work needs, paths to protect, and requests.

1. In a graft consumer, add `kit` to mkFlake's `perSystem`:

   ```nix
   outputs = inputs: inputs.graft.lib.mkFlake {
     inherit inputs;
     perSystem = { pkgs, ... }: {
       kit = {
         tools = [ pkgs.nodejs_22 ];
         protect = [ "AGENTS.md" ];
         requests.network = [ "registry.npmjs.org" ];
         requests.capabilities = [ { name = "nix"; } ];   # graft check evaluates graft.nix
       };
     };
   };
   ```

   A repository that runs `graft check` inside a run needs the `nix` capability: `graft check` evaluates each
   scion's `graft.nix`. The capability needs your root to bind it (`nix."*" = { via = "daemon-untrusted"; };`),
   which graft's default root and `pot init`'s root do.

   pot finds this flake from `.gitmodules`: the nearest `flake.nix` at or above the folder that holds the scions.
   A repository with no submodules has no repo kit from its flake, and pot never evaluates it: give it one with
   the root's `repoKits` (below), or graft a scion first.

2. Read what a run would get: `pot explain claude`. The repo kit's items are labelled `(repo)`, and so are the
   paths that the bundle protects (each `graft.nix` and the scions folder).
3. Run `pot grant`, and accept the repo kit's requests. Commit the grants file in your root.

A powerful grant of a repo kit is fixed to its flake directory as committed and staged. When a commit or a staged
change under it moves that tree, the next run stops with exit 125, lists the changed paths, and `pot grant` asks
again for each powerful item.

For a repository that is not a graft consumer, the root declares its repo kit, keyed by its normalised origin:

```nix
programs.pot.repoKits."github.com/example/site".requests.network = [ "registry.npmjs.org" ];
```

If the repository's `ensure.sh` or `justfile` builds tools with `nix`, make it use the tools that `pot` built
([Move from v2.14.0's sandbox](move-from-the-v2-14-sandbox.md#run-the-repositorys-tools-inside)).

## Set up a root

| Root | Use it when | Start it with |
|---|---|---|
| A home-manager root | You manage your home with home-manager. | `programs.pot` (below) |
| graft's default root | You have no root and want to try `pot`. | `nix run github:danielnaab/graft/v2.16.1#pot -- claude` |
| A minimal root of your own | You want a root without home-manager. | `pot init ~/src/pot-root`, then the steps that it prints |

graft's default root and the root that `pot init` writes work without home-manager. Each has a
`claude-code@personal` instance (the `claude` on your `PATH`), its token binding at
`~/.config/pot/claude-token-personal` (step 1), and the `nix` relay. graft's default root keeps its grants in
`~/.config/pot/grants.json`. `pot init DIR` writes `DIR/flake.nix`, with its grants file beside
it: track `DIR` in git. A home-manager root keeps its grants file in its own repository, where you review and
commit it.

A home-manager root:

```nix
{ inputs, pkgs, ... }:
{
  imports = [ inputs.graft.homeManagerModules.pot ];
  programs.pot = {
    enable = true;
    grantsFile = "~/.workstation/pot/grants.json";
    rootRepos = [ "~/.workstation" ];
    kits."claude-code@personal" = inputs.graft.kits.claude-code { inherit pkgs; };
    kits.standing.requests.network = [ "pypi.org" "registry.npmjs.org" ];
    tools = [ inputs.graft.packages.${pkgs.stdenv.hostPlatform.system}.default ];   # graft in runs of any repository
    contexts.work.include = [ "standing" ];
    bindings = {
      "status.relay"."*" = { workmux = true; };
      nix."*" = { via = "daemon-untrusted"; };
      credential."api.anthropic.com kit=claude-code@personal" = {
        via = "file"; from.file = "~/.config/pot/claude-token-personal"; };
    };
  };
}
```

For a second Claude account, or Bedrock, add an instance. Each instance is granted on its own:

```nix
kits."claude-code@work" = inputs.graft.kits.claude-code {
  inherit pkgs; configDir = ".claude-accounts/work";
  api = inputs.graft.apis.bedrock { region = "us-east-1"; };
};
defaultInstance.claude-code = "personal";
```

`pot claude@work` runs that instance, and so does `pot --instance work claude`. With neither, pot picks the
instance whose variables the shell matches (`CLAUDE_CONFIG_DIR=~/.claude-accounts/work` and
`CLAUDE_CODE_USE_BEDROCK=1` pick `@work`), and otherwise the default. The startup line names an instance that is
not the default: `pot: work · team-kb · instance work · pot explain`.

What the claude-code kit binds from your host into each run, read-only:

- your skills folder: links into the store or into a checkout under `skillSources`, and the real folders that
  `skillDirs` names (default `[ "synced" ]`; `skillDirs = [ ]` binds none);
- your plugins folder, whole (`plugins = false` leaves it out);
- your project memory for the workspace. `MEMORY.md` is a writable copy; the other files are read-only. New
  memory files, and a changed `MEMORY.md`, are listed on the exit line, with a copy in the run's directory, for
  you to read and copy by hand. Nothing is written back to your host memory. Claude is told this when the session
  starts.

Claude's synced claude.ai skills do not load in a run, even though `skills/synced` is bound: Claude loads them
only when it is signed in to claude.ai, and a run signs in with a token. Install the skills you need as ordinary
skill folders: a link into a checkout under `skillSources`, or a real folder that `skillDirs` names.

```nix
kits."claude-code@personal" = inputs.graft.kits.claude-code {
  inherit pkgs;
  skillSources = [ "~/src/agent-skills" ];   # your checked-out skills
  skillDirs = [ ];                           # no synced or hand-placed real folders
  plugins = false;                           # no plugins folder in the run
};
```

graft's command is not in a run's standard tools. A run in a graft consumer has that repository's own `graft`
(from its `pot-kits` bundle); `tools` gives every other run one too.

Run `home-manager switch`. `pot` is then on `PATH`, and `~/.config/pot/launcher` points at it. The options are
in [Home-manager options](../reference/pot.md#home-manager-options).

## If something goes wrong

- **Start with `pot doctor`.** It checks the backend, user systemd, the grants file, each binding's file (by path,
  never its contents), the instances and which one your shell selects ([pot doctor](../reference/pot.md#pot-doctor)).
- **`pot` fails before the command starts, in Docker,** because bubblewrap cannot create a user namespace.
  Docker's default seccomp profile blocks them. Run `pot` on the host or in a VM. `pot` never runs the command
  without the sandbox.
- **The run stops with exit 125 and a list of requests.** A kit asks for more than its grant, or was never
  granted where no prompt can appear (an agent, a script). Run `pot grant KIT` at a terminal
  ([When a kit asks for more](#when-a-kit-asks-for-more)).
- **A tool prints `CONNECT tunnel failed, response 403`, `tunnel error` or `Could not resolve host`.** The
  sandbox refused the host. Read `/run/pot/denials` inside, or run `pot why HOST`
  ([When something is denied](#when-something-is-denied)).
- **A test fails because `/run/current-system/sw/bin/…` is missing.** In `work`, `/run/current-system` is a
  link whose target the run does not see: without the `nix` capability, a run sees only its closure in the store.
  Make the test use the `PATH` it was given.
- **The startup line says `meeting-kit: gitlink is not a release tag at its URL; its kit is unverified and its grant
  does not apply`.** That scion is not at a release tag, so the run goes on without its reach. Move it to a release
  (`graft upgrade NAME`), or try it on the host with `pot host CMD`. <!-- TBD: sample -->
- **A claude.ai skill is missing inside a run.** Synced claude.ai skills do not load in a run (above). Install the
  skill as an ordinary skill folder.
- **`pot: no backend for darwin in v2.16.1`.** You are on macOS. Use graft v2.14.0's sandbox there.

## How it works

At start, `pot` finds the kits of the run: the command's, the repository's, its scions', and the root kits its
context includes. It reads your grants file, and composes the run's policy: what the run sees, its limits, and
the reach that you granted. Then bubblewrap starts the command with only that. Network goes through a proxy on
the host that allows only granted hosts. Nothing that the run writes can widen a later run: the grants file and
`pot`'s state are hidden, and your root's repository is never written in place
([Decisions](../decisions.md#roots-grant-reach-kits-only-ask)). `pot dispatch` runs in a clone instead of your
tree, and its commits come back only when you fetch them ([Work in a clone](dispatch.md)).

## Sources

- [pot](../reference/pot.md): contexts, kits, requests, grants, flags, messages
- [Move from v2.14.0's sandbox](move-from-the-v2-14-sandbox.md)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `kit` in `perSystem`
- [pot-kits/1](../../pot/contracts/pot-kits-1.md): how pot finds a repository's flake
- [pot/core/pot.py](../../pot/core/pot.py): the grant prompt, the startup and exit lines
- [pot/kits/claude-code.nix](../../pot/kits/claude-code.nix): the claude-code kit
- [pot/nix/hm.nix](../../pot/nix/hm.nix): `programs.pot`
- [tests/run.sh](../../tests/run.sh): the `pot` section
