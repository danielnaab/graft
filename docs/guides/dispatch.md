---
status: working
---
# Work in a clone

`pot dispatch CMD` runs a command in a throwaway clone of your repository, kept in pot's state, not in your working
tree. The run can commit there. It cannot reach your repository's history, hooks or config. Its commits reach your
repository only when you fetch them, and you merge what you choose. Use it for unattended work, and to change a
root repository such as `~/.workstation`. The terms are in [pot](../reference/pot.md#the-model).

## Before you begin

- `pot` set up, with your agent's kit granted ([Run commands in pot](run-in-a-sandbox.md)).
- Linux. On macOS, `pot` refuses to start ([On macOS](../reference/pot.md#on-macos)).
- Commit what the run should start from. The clone starts at a commit, never at your working tree, and your
  uncommitted changes stay behind.
- The steps use `~/src/flexion-assistant` (its flake is `nix/agent-tools/flake.nix`) and the instance
  `claude-code@flexion`.

## Steps

1. In the repository, start the run in a clone:

   ```
   cd ~/src/flexion-assistant
   pot dispatch claude@flexion
   ```

   The startup line names the base:

   ```
   pot: dispatch · flexion-assistant · from main (716424f) · pot explain
   ```

   Only the first level of submodules is cloned; `pot runs ID` and `pot review ID` note each scion whose own
   submodules are left out.

   When it ends, the exit line counts its commits on each branch it moved, and in each scion:
   `pot: run 1ac1 · 1 commit on main: pot review 1ac1, then pot review 1ac1 --fetch`.

   To start from another commit or branch, add `--from REF`. To leave it running in the background, add `--detach`
   and give the agent its task:

   ```
   id=$(pot dispatch --detach claude@flexion -p "migrate the tasks to schema v3")
   ```

   pot prints the run's id on stdout, and on stderr how to follow it:
   `pot: run 1ac1 in the background · dispatch · flexion-assistant · pot runs 1ac1 · pot review 1ac1`.

2. Follow it:

   ```
   pot runs              # every run and its state
   pot runs 1ac1 --follow
   ```

   `--follow` prints the run's events until it ends (`20:22:43 start dispatch …`, `20:22:45 exit 0`, `after 1
   commit`). A run is `running`, then `review` when it ended with commits, or `exited` when it made none (and once
   its commits are fetched or discarded). A run that needs a
   grant does not start: it is `stopped`, `pot grant` (no argument) offers what it needs, and a workmux pane that
   started it shows `waiting`.

3. When it ends, see what it did:

   ```
   pot review            # the runs that need you
   pot review 1ac1       # one run: its commits, diffstat and the host's findings
   ```

   The ATTENTION column lists what to read with care: unsigned commits (when your git config signs), changed
   host-run files (`justfile`, `.envrc`, CI workflows), new gitlinks or git metadata, changes to the repo kit,
   root-repository changes, and denials. A run whose commits need nothing more than a read shows none.

4. Fetch its commits into your repository:

   ```
   pot review 1ac1 --fetch
   ```

   pot copies the run's branches into `refs/remotes/pot-1ac1/*`, and prints the next steps:

   ```
   pot: fetched run 1ac1 into ~/src/flexion-assistant: pot-1ac1/main (1 commit on 716424f)
     review:  git log --stat 716424f..pot-1ac1/main
     take it: git merge pot-1ac1/main
   ```

   If your git config signs commits, a `sign:` line makes a branch of the run's commits and signs them:
   `git switch -c pot-1ac1 pot-1ac1/main && git rebase --exec 'git commit --amend --no-edit -S' 716424f`.
   If the run committed in a scion, a line names that scion: its commits are in the scion's own
   `refs/remotes/pot-1ac1/*`, and a fix there goes to its provider.

5. Read the commits as you would a stranger's pull request, then merge what you want:

   ```
   git log --stat 716424f..pot-1ac1/main
   git merge pot-1ac1/main
   ```


6. When you are done, remove the clone:

   ```
   pot review 1ac1 --discard
   ```

   The fetched refs stay: `git branch -r -d pot-1ac1/main` removes them.

## Check it worked

- `git log -1 pot-1ac1/main` shows the run's last commit.
- `pot runs 1ac1` says `exited` and `fetched`.

## Change a root repository

`pot claude` refuses in your root's repository (`~/.workstation`), because it decides what every run may do. Use
a clone instead:

```
cd ~/.workstation
pot dispatch claude
pot review            # attention: root: modules/pot.nix (programs.pot), root: pot/grants.json
pot review 1ac1 --fetch
git log --stat main..pot-1ac1/main
git merge pot-1ac1/main
home-manager switch
```

Review the branch before the switch: it changes what every later run may do.

## Give dispatch runs your standing reach

`dispatch` is its own context: it does not include the root kit that your `work` context includes (`daniel` in
these examples), so a dispatch run has no package registries or `git.fetch` from it. To give dispatch runs the
same standing reach, include that kit in the context, in your root:

```nix
programs.pot.contexts.dispatch.include = [ "daniel" ];
```

With mkRoot, the same goes in `contexts`. Then `home-manager switch` (or rebuild the root). The kit's grant is the
one you already made: including it asks for nothing new.

## What the clone has, and what it doesn't

- Your commit (the base) and its branch, and no other branch of yours; its scions (the first level of submodules) at their gitlinks. The
  scions folder is read-only, as in place; a commit made in a scion's git directory comes back into that scion's
  `refs/remotes/pot-<id>/*`, and one on its detached `HEAD` as `pot-<id>/detached`.
- Your `user.name`, `user.email` and the origin URL. None of your other git config, and none of your hooks.
- No LFS content: pot's git runs no filters, so LFS files are pointer files.
- Your repository's kits, as committed at the base. A kit change that you have not committed does not apply.
- One repository: `--repo` is refused in clone mode.

## Keeping clones

A clone stays until you discard it. pot removes, on its own, only clones that were fetched, discarded or have no
commits, after `clones.keepDays` (14 days) or when the clones pass `clones.maxSize` (20G). It never removes
unfetched commits: over the cap, the startup line and `pot doctor` name the oldest runs waiting for review, with
`pot review ID --discard`. Both values are root options ([pot](../reference/pot.md#home-manager-options)).

## If something goes wrong

- **`your 3 uncommitted changes stay here, outside it`.** The clone started at your last commit. Commit first, or
  pass `--from` a branch that has the work.
- **The run is `stopped`.** It needs a grant: run `pot grant` at a terminal, then start it again.
- **`pot: run 1ac1 has no commits to fetch`.** The run made no commit. Its uncommitted work stays in the clone,
  which pot never copies to your repository.
- **`ended      unknown: pot ended without an exit (killed or crashed)`.** pot was killed or crashed. The next
  `pot runs` or `pot review` copies the commits it made out of the clone, so the run is in `review` and
  `pot review 1ac1` reads them as usual.
- **Anything else:** `pot doctor` checks the backend, user systemd, the grants file and the bindings.

## How it works

pot clones your repository into its state, with no hard links, and mounts the clone at your repository's own path
inside the run. When the run ends, the host reads the clone's branches through a sandboxed `git upload-pack` with
fsck into staging repositories it owns, and computes what to flag from there; it never opens the clone with host
git. `--fetch` copies from staging. See [pot: dispatch](../reference/pot.md#dispatch) and
[runs/1](../../pot/contracts/runs-1.md).

## Sources

- [pot's reference: dispatch](../reference/pot.md#dispatch), [pot review](../reference/pot.md#pot-review),
  [pot runs](../reference/pot.md#pot-runs)
- [pot/core/pot.py](../../pot/core/pot.py): dispatch, review and the run log
