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
   pot: dispatch · flexion-assistant · from main (c9996c5) · pot explain
   ```
   <!-- TBD: sample -->

   To start from another commit or branch, add `--from REF`. To leave it running in the background, add `--detach`
   and give the agent its task:

   ```
   id=$(pot dispatch --detach claude@flexion -p "migrate the tasks to schema v3")
   ```

   pot prints the run's id on stdout, and on stderr how to follow it:
   `pot: run 9b0e in the background · dispatch · flexion-assistant · pot runs 9b0e · pot review 9b0e`.
   <!-- TBD: sample -->

2. Follow it:

   ```
   pot runs              # every run and its state
   pot runs 9b0e --follow
   ```

   A run is `running`, then `review` when it ended with commits, or `exited` when it made none. A run that needs a
   grant does not start: it is `stopped`, `pot grant` (no argument) offers what it needs, and a workmux pane that
   started it shows `waiting`.

3. When it ends, see what it did:

   ```
   pot review            # the runs that need you
   pot review 9b0e       # one run: its commits, diffstat and the host's findings
   ```

   The ATTENTION column lists what to read with care: unsigned commits, changed host-run files (`justfile`,
   `.envrc`, CI workflows), new gitlinks or nested repositories, changes to the repo kit, and denials.

4. Fetch its commits into your repository:

   ```
   pot review 9b0e --fetch
   ```

   pot copies the run's branches into `refs/remotes/pot-9b0e/*`, and prints the next steps:

   ```
   pot: fetched run 9b0e into ~/src/flexion-assistant: pot-9b0e/main (3 commits on c9996c5)
     review:  git log --stat c9996c5..pot-9b0e/main
     take it: git merge pot-9b0e/main
   ```
   <!-- TBD: sample -->

5. Read the commits as you would a stranger's pull request, then merge what you want:

   ```
   git log --stat c9996c5..pot-9b0e/main
   git merge pot-9b0e/main
   ```

   If your git config signs commits, `--fetch` also prints the `git rebase --exec … -S` line that signs them.

6. When you are done, remove the clone:

   ```
   pot review 9b0e --discard
   ```

   The fetched refs stay: `git branch -r -d pot-9b0e/main` removes them.

## Check it worked

- `git log -1 pot-9b0e/main` shows the run's last commit.
- `pot runs 9b0e` says `exited` and `fetched`.

## Change a root repository

`pot claude` refuses in your root's repository (`~/.workstation`), because it decides what every run may do. Use
a clone instead:

```
cd ~/.workstation
pot dispatch claude
pot review            # commits that touch pot/grants.json or programs.pot are shown in red
pot review 9b0e --fetch
git log --stat main..pot-9b0e/main
git merge pot-9b0e/main
home-manager switch
```

Review the branch before the switch: it changes what every later run may do.

## What the clone has, and what it doesn't

- Your commit (the base), its branch, and its scions at their gitlinks.
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
- **`pot: run 9b0e has no commits to fetch`.** The run made no commit. Its uncommitted work stays in the clone,
  which pot never copies to your repository.
- **The run is `exited`, `pot ended without an exit`.** pot was killed or crashed. `pot review 9b0e` still reads
  the commits it made.
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
