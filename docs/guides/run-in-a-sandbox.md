---
status: draft
---
# Run an agent or a command in a sandbox

Run Claude Code, or any command, with `sb`, so that it can change your repository but cannot read your keys,
reach hosts that you did not grant, or change what decides the sandbox. `sb` is experimental in v2.15.0. The terms
are in [Sandbox](../reference/sandbox.md#the-model).

## Before you begin

- Linux, with unprivileged user namespaces (the default on NixOS, Debian, Ubuntu and Fedora). On macOS, `sb`
  refuses to start in v2.15.0: stay on graft v2.14.0's sandbox there ([On macOS](../reference/sandbox.md#on-macos)).
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
   you, such as `~/.config/graft/sb/claude-token-personal` (`chmod 600`). The file holds the token alone, or a line
   `CLAUDE_CODE_OAUTH_TOKEN=…`. Do not paste it on a command line. `sb` hides the file from every run.

2. Start Claude in the repository:

   ```
   cd ~/src/team-kb
   sb claude
   ```

3. The first time, `sb` shows the grant prompt before the run. It lists what the claude-code kit asks for,
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
   `grant? [y/N]`. The repo kit and any scion kits that were never granted come next, in the same prompt. The
   prompt appears only at a terminal; elsewhere the run stops with exit 125 and `sb grant KIT`.

4. Claude starts. The startup line names what differs from the usual shape, and `sb explain`:

   ```
   sb: work · team-kb · sb explain
   ```

5. `sb` prints `wrote ~/.workstation/sandbox/grants.json. Commit it with your root's next change.` Commit it:

   ```
   git -C ~/.workstation add sandbox/grants.json
   git -C ~/.workstation commit -m "sb: grant claude-code@personal"
   ```

   From now on, `sb claude` starts with no prompt, until a kit asks for more.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the run is in `work`, can write the repository,
cannot write `justfile`, and cannot see `~/.ssh`:

```sh
sb bash -c 'test "$GRAFT_SANDBOX" = work && touch .sb-check && ! touch justfile 2>/dev/null && ! test -e ~/.ssh' &&
  test -e .sb-check && rm -f .sb-check && echo ok
```

If the repository has no `justfile`, write another [protected path](../reference/sandbox.md#protected-paths),
such as `flake.nix`.

## Choose a context

| You want to | Run | The run |
|---|---|---|
| Work on this repository | `sb claude` | writes the repository in place, reaches only what is granted |
| Diagnose the machine, change nothing | `sb inspect claude` | reads the machine except secrets and hidden paths, writes nothing, reaches only the model API |
| Do what only the host may do | `sb host CMD`, or plain `claude` | has no sandbox |

`inspect` diagnoses; the host repairs. `sudo`, `systemctl restart`, `docker run` and `home-manager switch` are
host-only. To carry a diagnosis from a run to the host, as quoted data: `sb host --resume RUN claude`.

In workmux, set the agent command to `sb claude`. The `status.relay` capability keeps the pane's status working.

## When something is denied

A denial reaches you in three ways. Under tmux, a message shows on the agent's pane:

```
graft sandbox (work): the run tried pypi.org:443 (not in this run) · sb why pypi.org
```

The agent reads it from the denials file, through its hook, and tells you in its own words. When the run ends,
the exit line lists it:

```
graft sandbox (work): the run tried pypi.org:443 (not in this run). one run: sb --net pypi.org …  permanent: the repo kit's requests.network (perSystem.kit), or programs.sb.kits.standing.requests.network for every repository (home-manager switch)
```

1. Ask `sb` where the item would come from:

   ```
   sb why pypi.org
   ```

2. To allow it for one run, give the flag that `sb why` names:

   ```
   sb --net pypi.org claude --continue
   ```

   The exit line names the permanent fix for each flag that you used.

3. To allow it in every run of this repository, add it to the repo kit (see [Add a repo kit](#add-a-repo-kit)):

   ```nix
   kit.requests.network = [ "pypi.org" ];
   ```

   To allow it in every repository, add it to `standing`, the context's included kit
   (`programs.sb.kits.standing`, below), then `home-manager switch`.

4. Run `sb grant`, and accept the new item. Commit the grants file.

## When a kit asks for more

When an upgrade gives a kit new requests, or a new source for a powerful one, its runs stop with exit 125 and the
difference:

1. Read the difference that `sb` printed, or run `sb grant --diff`.
2. If you accept it, run `sb grant KIT`, and commit the grants file. Otherwise, take back the upgrade.

Runs that the kit's requests do not apply to still start, with a note on the startup line:
`meeting-kit asks for more: sb grant`.

## Root repositories

Your root's own repository (`~/.workstation` here) decides what every run may do, so `sb` never writes it in
place. In v2.15.0, `sb claude` refuses there:

```
graft sandbox (work): ~/.workstation is a root: sb inspect, or claude (unsandboxed) until sb dispatch (1b)
```

- To read it, use `sb inspect claude`.
- To change it, use plain `claude`, then review the diff before `home-manager switch`.
- In its worktrees, set workmux's agent command to `claude` in `~/.workstation/.workmux.yaml`:

  ```yaml
  agent: claude
  ```

<!-- unverified: the .workmux.yaml key for the agent command. -->

## Add a repo kit

A repository's own needs are its repo kit: the packages its work needs, paths to protect, and requests.

1. In a graft consumer, add `kit` to mkFlake's `perSystem`:

   ```nix
   outputs = inputs: inputs.graft.lib.mkFlake {
     inherit inputs;
     perSystem = { pkgs, ... }: {
       kit = {
         packages = [ pkgs.nodejs_22 ];
         protect = [ "AGENTS.md" ];
         requests.network = [ "registry.npmjs.org" ];
         requests.capabilities = [ { name = "nix"; } ];   # graft check evaluates graft.nix
       };
     };
   };
   ```

   A repository that runs `graft check` inside a run needs the `nix` capability: `graft check` evaluates each
   scion's `graft.nix`. The capability needs your root to bind it (`nix."*" = { via = "daemon-untrusted"; };`),
   which graft's default root and `sb init`'s root do.

2. Read what a run would get: `sb explain claude`. The repo kit's items are labelled `(repo)`.
3. Run `sb grant`, and accept the repo kit's requests. Commit the grants file in your root.

A powerful grant of a repo kit is fixed to its flake directory as committed and staged. When a commit or a staged
change under it moves that tree, the next run stops with exit 125, lists the changed paths, and `sb grant` asks
again for each powerful item.

For a repository that is not a graft consumer, the root declares its repo kit, keyed by its normalised origin:

```nix
programs.sb.repoKits."github.com/example/site".requests.network = [ "registry.npmjs.org" ];
```

If the repository's `ensure.sh` or `justfile` builds tools with `nix`, make it use the tools that `sb` built
([Move from v2.14.0's sandbox](move-from-the-v2-14-sandbox.md#run-the-repositorys-tools-inside)).

## Set up a root

| Root | Use it when | Start it with |
|---|---|---|
| A home-manager root | You manage your home with home-manager. | `programs.sb` (below) |
| graft's default root | You have no root and want to try `sb`. | `nix run github:danielnaab/graft/v2.15.0#sb -- claude` |
| A minimal root of your own | You want a root without home-manager. | `sb init ~/src/sb-root`, then the steps that it prints |

Both work without home-manager. Each has a `claude-code@personal` instance (the `claude` on your `PATH`), its
token binding at `~/.config/graft/sb/claude-token-personal` (step 1), and the `nix` relay. graft's default root keeps
its grants in `~/.config/graft/sb/grants.json`. `sb init DIR` writes `DIR/flake.nix`, with its grants file beside
it: track `DIR` in git. A home-manager root keeps its grants file in its own repository, where you review and
commit it.

A home-manager root:

```nix
{ inputs, pkgs, ... }:
{
  imports = [ inputs.graft.homeManagerModules.sb ];
  programs.sb = {
    enable = true;
    grantsFile = "~/.workstation/sandbox/grants.json";
    rootRepos = [ "~/.workstation" ];
    kits."claude-code@personal" = inputs.graft.kits.claude-code { inherit pkgs; account = "personal"; };
    kits.standing.requests.network = [ "pypi.org" "registry.npmjs.org" ];
    contexts.work.include = [ "standing" ];
    bindings = {
      "status.relay"."*" = { workmux = true; };
      credential."api.anthropic.com kit=claude-code@personal" = {
        via = "file"; from.file = "~/.config/graft/sb/claude-token-personal"; };
    };
  };
}
```

Run `home-manager switch`. `sb` is then on `PATH`, and `~/.config/graft/sb/launcher` points at it. The options are
in [Home-manager options](../reference/sandbox.md#home-manager-options).

## If something goes wrong

- **`sb` fails before the command starts, in Docker,** because bubblewrap cannot create a user namespace.
  Docker's default seccomp profile blocks them. Run `sb` on the host or in a VM. `sb` never runs the command
  without the sandbox.
- **The run stops with exit 125 and a list of requests.** A kit asks for more than its grant, or was never
  granted where no prompt can appear (an agent, a script). Run `sb grant KIT` at a terminal
  ([When a kit asks for more](#when-a-kit-asks-for-more)).
- **A tool prints `CONNECT tunnel failed, response 403`, `tunnel error` or `Could not resolve host`.** The
  sandbox refused the host. Read `/run/graft-sandbox/denials` inside, or run `sb why HOST`
  ([When something is denied](#when-something-is-denied)).
- **A test fails because `/run/current-system/sw/bin/…` is missing.** In `work`, `/run/current-system` is a
  link whose target the run does not see: a run sees only its closure. Make the test use the `PATH` it was given.
- **`sb: no backend for darwin in v2.15.0`.** You are on macOS. Use graft v2.14.0's sandbox there.

## How it works

At start, `sb` finds the kits of the run: the command's, the repository's, its scions', and the root kits its
context includes. It reads your grants file, and composes the run's policy: what the run sees, its limits, and
the reach that you granted. Then bubblewrap starts the command with only that. Network goes through a proxy on
the host that allows only granted hosts. Nothing that the run writes can widen a later run: the grants file and
`sb`'s state are hidden, and your root's repository is never written in place
([Decisions](../decisions.md#roots-grant-reach-kits-only-ask)).

## Sources

- [Sandbox](../reference/sandbox.md): contexts, kits, requests, grants, flags, messages
- [Move from v2.14.0's sandbox](move-from-the-v2-14-sandbox.md)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `kit` in `perSystem`
- [tests/run.sh](../../tests/run.sh)
