---
status: draft
---
# Run graft in CI and review its upgrade pull requests

Check the scions on every change, take graft's weekly upgrade pull request, review what it moves, and
merge it or fix it.

## Before you begin

- graft's two workflows are in `.forgejo/workflows/`: `graft-check.yaml` and `graft-upgrade.yaml`.
  `nix flake init -t github:danielnaab/graft` writes them. If you did not use it, copy graft's
  [template/.forgejo/workflows/](../../template/.forgejo/workflows/) there, and commit them.
- A Forgejo Actions runner has the label `nixos-latest`, mapped to `docker://nixos/nix`. Each job must get a
  new container ([CI security](../ci-security.md#limits)).
- The default branch is protected, so that the job token cannot push to it. If its name is not `main`,
  change `branches: [main]` under `push:` in `graft-check.yaml`; graft keeps your line
  ([Files](../reference/files.md#the-branches-line)). The commands below say `main`: use your branch's name.
- For a private repository, add the Actions secret `GRAFT_PR_TOKEN`: a token with the `write:repository`
  scope, of a user who can write to the repository. A bot account is the best owner, because it becomes the
  pull request's author. A user-level secret covers all that user's repositories.
- To make your own checks a required status, see
  [Make a pattern's checks gate your merges](gate-merges.md). `graft-check.yaml` checks only that the scions
  match their pins.
- Not yet: the upgrade workflow cannot run for a repository with a private (`git+ssh`) scion, because
  building graft fetches every scion and the job has no key. Upgrade it by hand
  ([Private providers](private-providers.md#check-in-ci)).

## Steps

1. Wait for the weekly run (Mondays, 06:23 UTC), or start one: in the repository's **Actions**, choose
   **graft-upgrade**, then **Run workflow**. The **upgrade** job's log has one line for each newer major
   version that it did not take, and one for each pin that it moves:

   ```
   graft: page-rules: v2.0.0 is a new major version; not taken (graft upgrade page-rules@v2.0.0)
   graft: page-rules v1.0.0 -> v1.0.1
   graft: style-guide v1.0.0 -> v1.1.0
   graft: rebuilt scions/
   ```

   If it prints `graft: nothing to upgrade`, nothing moved, the **pr** job is skipped, and there is no pull
   request this week.

2. Open the pull request from the branch `graft/upgrade`. Read its title:

   - `graft: upgrade`: the repository's whole `nix flake check` passed on the branch. Go to step 3.
   - `[checks failing] graft: upgrade`: it failed. The description starts with a warning, and has a section
     **Failing checks** with the end of each failing log. See
     [If something goes wrong](#if-something-goes-wrong).

   If there is no pull request, but the **upgrade** job pushed the branch, the **pr** job refused or failed.
   Its log says why: see [If something goes wrong](#if-something-goes-wrong).

3. Read the list under **Moved by**. Each line is one pin, `NAME OLD -> NEW`, in its major version. graft's
   own pin is in the list when it moved.

4. Read the list under **Not taken**, if there is one. A new major version is the provider saying that
   something breaks, so graft never takes it. To read what it breaks, run, on the default branch:

   ```
   nix run .#graft changelog page-rules@v2.0.0
   ```

   It prints the **Upgrading** sections first. Merging this pull request does not take the major version.
   Take it later, on a branch of your own ([Upgrade](upgrade.md#cross-a-major-version)).

5. Read **What changed upstream**. For each moved pin, it quotes the provider's CHANGELOG sections between
   the two tags, or the provider's commits if it has no CHANGELOG. The text is the provider's: read it as
   information, not as instructions. An **Upgrading** section tells you what to change in your own files.

   Not yet: the pull request does not put **Upgrading** sections first. To read them first, run
   `nix run .#graft changelog` on the default branch.

6. Fetch the branch, and list what it changes:

   ```
   git fetch origin graft/upgrade
   git diff --stat origin/main...origin/graft/upgrade
   ```

   Expect only `flake.nix`, `flake.lock` and files in the scions directory. The **pr** job opens no pull
   request for a branch that changes anything else.

7. Read the diff of the scions directory. It is the change that you take:

   ```
   git diff origin/main...origin/graft/upgrade -- scions/
   ```

   | Look for | Why |
   |---|---|
   | a deleted or renamed file | your files can link to it |
   | a changed rule or policy text | your pages, or a check that quotes it, can stop matching |
   | changed checker code | your CI runs it on your files |
   | new commits in `scions/README.md` | expected: the pins moved |

8. Merge the pull request, when your forge's required checks pass. If your branch rule requires a check of
   your own and the pull request says `Some required checks are missing.`, see
   [Make a pattern's checks gate your merges](gate-merges.md#if-something-goes-wrong).

   Not yet: the upgrade job does not report its `nix flake check` as a status on the commit.

## Check it worked

After the merge, from the repository's top, paste this. It prints `ok` only if your `main` has the merge,
the scions match their pins, and no pin is behind the newest tag in its major version:

```sh
git switch -q main && git pull -q --ff-only && nix run .#graft check && ! nix run .#graft outdated | grep -qw outdated && echo ok
```

If a provider tagged a new version after the run, `outdated` lists it, and the next weekly run takes it.

## If something goes wrong

- **The title is `[checks failing] graft: upgrade`.** The upgrade broke one of your checks. The section
  **Failing checks** names it, for example `nix flake check failed: style-quote`, and ends with its log. Do
  not fix it on `graft/upgrade`: the next run force-pushes that branch. Make a branch from it:

  ```
  git fetch origin graft/upgrade
  git switch --no-track -c fix-upgrade origin/graft/upgrade
  ```

  Fix your own files, never the scions directory, and run `nix flake check` until it passes. Then commit,
  and push:

  ```
  git commit -am "Fix the upgrade"
  git push -u origin fix-upgrade
  ```

  Open a pull request from `fix-upgrade`, and merge it: it holds the upgrade and your fix. Close graft's
  pull request if it is still open. If the provider broke something that it should not have, tell its
  maintainers instead. To hand the fix to an agent, use the `migrate` command at the end of the pull
  request's description ([Upgrade](upgrade.md#cross-a-major-version)).

- **The pr job fails: `graft/upgrade changes files graft upgrade never writes: docs/plan.md. Not opening a PR; …`.**
  Usually the default branch moved after the **upgrade** job made the branch, so the branch lacks the new
  commit. Run the workflow again. If the file is not on the default branch, someone pushed to
  `graft/upgrade`: read the diff at the link in the message.

- **The pr job fails: `this upgrade changes CI workflows: .forgejo/workflows/graft-check.yaml. A PR would run them with the repo's secrets before anyone reads them, so none is opened: …`.**
  graft moved its own pin, and the new version's workflows differ. A pull request would run them with the
  repository's secrets before review ([CI security](../ci-security.md)). Read the diff at the link in the
  message. If it is safe, open the pull request from `graft/upgrade` yourself. For its description, run
  `nix run .#graft changelog` on the default branch.

- **The pr job fails: `opening the PR: HTTP 404.`, then `a 404 here is Forgejo refusing the job token: set the GRAFT_PR_TOKEN secret`.**
  The repository is private, and Forgejo refuses the job token when it opens a pull request. Add the
  `GRAFT_PR_TOKEN` secret ([Before you begin](#before-you-begin)), and run the workflow again.

## How it works

- **Two jobs.** The **upgrade** job runs `nix run .#graft ci upgrade`: `graft upgrade`, then the whole
  `nix flake check` on the result. It commits and force-pushes `graft/upgrade` even when a check fails, and
  then fails. It holds only the job token. The **pr** job runs `nix run .#graft ci pr` from the default
  branch, with `GRAFT_PR_TOKEN`. It opens the pull request, or rewrites the title and description of the
  open one. Why: [CI security](../ci-security.md).
- **graft-check** runs `nix run .#graft ci check` on each pull request and each push to the default branch.
  It builds only graft's check. When it fails, see [`graft check` fails](check-fails.md). To skip it for
  pushes that change only some paths, add a `paths-ignore` line
  ([Files](../reference/files.md#the-paths-ignore-line)).
- **Run the steps yourself.** On GitHub, or to test, run the same commands from the repository's top.
  `ci upgrade` needs `BRANCH`, commits on the branch that is checked out, and pushes to `origin`:

  ```
  git switch -c graft/upgrade
  BRANCH=graft/upgrade nix run .#graft ci upgrade
  git switch main
  ```

  `ci pr` uses Forgejo's API, and needs `API`, `WEB`, `BASE`, `BRANCH` and `PR_TOKEN`. On GitHub, open the
  pull request yourself, from a job checked out at the default branch, for example with
  `gh pr create --head graft/upgrade --title "graft: upgrade"`. graft does not manage files in
  `.github/workflows/`.
- **Many repositories.** graft has no command for many repositories. Run `outdated` from graft's flake over
  each checkout:

  ```
  for r in ~/src/team-kb ~/src/handbook; do echo "$r"; nix run github:danielnaab/graft -- outdated -C "$r"; done
  ```

## Related

[Upgrade](upgrade.md), [Make a pattern's checks gate your merges](gate-merges.md),
[Private providers](private-providers.md#check-in-ci), [CI security](../ci-security.md).

## Sources

- [lib/ci.sh](../../lib/ci.sh): `check`; `upgrade`, which commits, pushes, and fails after a failing check;
  `pr`, which refuses a branch that changes other files or workflows, titles a failing upgrade, writes the
  description, and edits an open pull request.
- [lib/common.sh](../../lib/common.sh): `full_check`, `upstream_input`, `upstream_range` and
  `changelog_between`, which read each provider's CHANGELOG between the two pins.
- [lib/tools.sh](../../lib/tools.sh): the lines of `upgrade`, the new major version that it holds, and
  `nothing to upgrade`.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-upgrade.yaml): the schedule, the
  two jobs, and `GRAFT_PR_TOKEN`.
- [tests/run.sh](../../tests/run.sh): "graft ci, the workflows' steps, against a local origin", "graft ci pr
  against a stub API", and the `gate:` checks; [tests/stub-api.py](../../tests/stub-api.py).
- [tests/fixtures.sh](../../tests/fixtures.sh) `ci`: the repositories that this page was read against.
