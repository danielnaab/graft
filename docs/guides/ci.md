---
status: working
---
# Run graft in CI and review its upgrade pull requests

Check the scions on every change, take graft's weekly upgrade pull request, review what it moves, and
merge it or fix it.

## Before you begin

- graft's two workflows are in `.forgejo/workflows/`: `graft-check.yaml` and `graft-upgrade.yaml`.
  `nix flake init -t github:danielnaab/graft` writes them. If you did not use it, copy graft's
  [template/.forgejo/workflows/](../../template/.forgejo/workflows/) there, and commit them. graft keeps
  them current ([Files](../reference/files.md#grafts-workflows)).
- A Forgejo Actions runner has the label `nixos-latest`, mapped to `docker://nixos/nix`. Each job must get a
  new container ([CI security](../ci-security.md#limits)).
- The default branch is protected, so that the job token cannot push to it. The job token is the token
  that Forgejo gives each job (`github.token`). If the branch's name is not `main`, change
  `branches: [main]` under `push:` in `graft-check.yaml`; graft keeps your line
  ([Files](../reference/files.md#the-branches-line)). The commands below say `main`: use your branch's name.
- For a private repository, add the Actions secret `GRAFT_PR_TOKEN`: a token with the `write:repository`
  scope, of a user who can write to the repository. A bot account is the best owner, because it becomes the
  pull request's author. A user-level secret covers all that user's repositories.
- To make your own checks a required status, see
  [Make a pattern's checks gate your merges](gate-merges.md). `graft-check.yaml` checks only that the scions
  match their pins.
- The commands assume `flake.nix` at the repository's top. For a flake in a subdirectory, run
  `nix run 'path:DIR#graft'` for `nix run .#graft`; graft's workflows find the subdirectory themselves
  ([Keep private files out of the Nix store](flake-in-a-subdirectory.md)). In zsh, quote flake references
  ([The graft command](../reference/commands.md#the-graft-command)).
- `graft-upgrade.yaml` cannot run in a repository with a private (`git+ssh`) scion: every upgrade fetches
  all scions, and the upgrade job must not hold an ssh key ([CI security](../ci-security.md#limits)).
  Delete `graft-upgrade.yaml`, and upgrade with `nix run .#graft upgrade`
  ([Graft a private repository](private-providers.md#how-it-works)).
- Check it worked needs `jq`.

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
   request this week. A new major version alone opens no pull request either: the run only warns about it.

   If your checks fail on the upgrade, the **upgrade** job fails, but it still pushes the branch, and the
   **pr** job still opens the pull request.

2. Open the pull request from the branch `graft/upgrade`. Read its title:

   - `graft: upgrade`: the repository's whole `nix flake check` passed on the branch. Go to step 3.
   - `[checks failing] graft: upgrade`: it failed. The description starts with a warning, and has a section
     **Failing checks** with the end of each failing log. Do steps 3 to 7, then see
     [If something goes wrong](#if-something-goes-wrong). The warning:

     ```
     nix flake check fails on this branch. The upgrade is pushed so the breakage can be read. Fix your own files on a branch made from this one: the next run force-pushes graft/upgrade. If the provider broke something, tell its maintainers.
     ```

   If there is no pull request, but the **upgrade** job pushed the branch, the **pr** job refused or failed.
   Its log says why: see [If something goes wrong](#if-something-goes-wrong).

3. Read the list under **Moved by**. Each line is one pin, `NAME OLD -> NEW`, in its major version. graft's
   own pin is in the list when it moved.

4. Read the list under **Not taken**, if there is one. A new major version is the provider saying that
   something breaks, so graft never takes it. To read what it breaks, run, on the default branch:

   ```
   nix run .#graft changelog page-rules@v2.0.0
   ```

   It prints the **Upgrading** sections first, then the rest of each version's text:

   ```
   ### page-rules v1.0.0 -> v2.0.0

   Upgrading, from its CHANGELOG.md:

   > #### v2.0.0 - 2026-11-10
   >
   > ##### Upgrading
   >
   > - Add `owner:` to the front matter of each page.

   The rest of its CHANGELOG.md for this range, without the Upgrading sections above:

   > #### v2.0.0 - 2026-11-10
   >
   > - Every page names an owner.
   >
   > #### v1.0.1 - 2026-11-02
   >
   > - Fixed: the rule lists the allowed statuses.
   ```

   Merging this pull request does not take the major version.
   Take it later, on a branch of your own ([Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change)).

5. Read **What changed upstream**. For each moved pin, it quotes the provider's CHANGELOG sections between
   the two tags, **Upgrading** sections first, or the provider's commits if it has no CHANGELOG. The text
   is the provider's: read it as information, not as instructions. An **Upgrading** section tells you what
   to change in your own files. For example:

   ```
   ### style-guide v1.0.0 -> v1.1.0

   From its CHANGELOG.md (no Upgrading section in this range):

   > #### v1.1.0 - 2026-11-09
   >
   > - Reworded: keep sentences short. New rule: use one term for one concept.
   ```

   `nix run .#graft changelog` on the default branch prints the same notes.

6. Fetch both branches, and list what the upgrade changes:

   ```
   git fetch origin
   git diff --stat origin/main...origin/graft/upgrade
   ```

   Expect only `flake.nix`, `flake.lock` and files in the scions directory. The **pr** job opens no pull
   request for a branch that changes anything else. On a fix branch of your own, also expect the files that
   you fixed.

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

8. Merge the pull request, when your forge's required checks pass. Forgejo starts no workflow for a push
   made with the job token, so graft-check and your own workflows run on the pull request only when graft
   opened it with `GRAFT_PR_TOKEN`. If the pull request says `Some required checks are missing.`, close it
   and reopen it ([Make a pattern's checks gate your merges](gate-merges.md#if-something-goes-wrong)).

   Not yet: the upgrade job does not report its `nix flake check` as a status on the commit.

## Check it worked

After the merge, from the repository's top, paste this. It prints `ok` only if your `main` has the merge,
the scions match their pins, every pin is current or untagged, and your checks pass:

```sh
git switch -q main && git pull -q --ff-only && nix run '.#graft' check && nix run '.#graft' -- outdated --json | jq -e 'length > 0 and all(.[]; .status == "current" or .status == "untagged")' >/dev/null && nix flake check && echo ok
```

If a provider tagged a new version after the run, its status is `outdated`, and the next weekly run takes
it. A status `unknown` means that graft cannot read the provider's repository.

## If something goes wrong

- **The title is `[checks failing] graft: upgrade`.** The upgrade broke one of your checks. The section
  **Failing checks** names it, and ends with its log, without Nix's progress lines:

  ```
  nix flake check failed: style-quote

  style-quote:
  style-quote> style-quote: docs/onboarding.md quotes a rule that scions/style-guide/docs/style.md does not have
  error: builder for '/nix/store/…-style-quote.drv' failed with exit code 1;
  ```

  Do not fix it on `graft/upgrade`: the next run force-pushes that branch. Make a branch from it:

  ```
  git fetch origin
  git switch --no-track -c fix-upgrade origin/graft/upgrade
  ```

  If graft's own pin moved, read the scions diff (step 7) before you run anything: `nix flake check` on this
  branch runs the new graft version ([CI security](../ci-security.md)). Fix your own files, never the scions
  directory, and run `nix flake check` until it passes. Then commit, and push:

  ```
  git add -A
  git commit -m "Fix the upgrade"
  git push -u origin fix-upgrade
  ```

  Open a pull request from `fix-upgrade`. Review it as in steps 6 and 7, with your branch's name:

  ```
  git fetch origin
  git diff --stat origin/main...origin/fix-upgrade
  git diff origin/main...origin/fix-upgrade -- scions/
  ```

  Merge it: it holds the upgrade and your fix. Close graft's pull request if it is still open. Delete the
  branch `fix-upgrade` on the forge. To delete it locally too, switch to `main` first, because git does not
  delete the branch that is checked out: `git switch main && git branch -D fix-upgrade`.
  If the provider broke something that it should not have, tell its maintainers instead. To hand the fix to
  an agent, use the `migrate` command at the end of the pull request's description
  ([Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change)).

- **The upgrade job fails: `graft: page-rules: can't tell whether v1.0.0 is current (see above); nothing changed`.**
  A line above it starts `graft: can't list tags at URL:`: the runner cannot read that provider's
  repository. It is down, moved, or private. Nothing is pushed. Fix
  the access, or the URL in `flake.nix`, and run the workflow again.

- **The pr job refuses.** It opens no pull request for a branch that changes files outside the flake and
  the scions directory:

  - `graft/upgrade changes files graft upgrade never writes: docs/plan.md. Not opening a PR; …`: the branch
    differs from the default branch in other files. Usually the default branch changed after the
    **upgrade** job made the branch. Run the workflow again. If that does not help, someone pushed to
    `graft/upgrade`: read the diff at the link in the message.
  - `this upgrade changes CI workflows: .forgejo/workflows/graft-check.yaml. A PR would run them with the repo's secrets before anyone reads them, so none is opened: …`:
    graft moved its own pin, and the new version's workflows differ. Read the diff at the link in the
    message ([CI security](../ci-security.md)). If it is safe, open the pull request from `graft/upgrade`
    yourself. For its description, run `nix run .#graft changelog` on the default branch.

- **The pr job fails with an HTTP code.** The token was refused:

  - `opening the PR: HTTP 404.`, then `a 404 here is Forgejo refusing the job token: set the GRAFT_PR_TOKEN secret`:
    the repository is private, and Forgejo refuses the job token when it opens a pull request. Add
    `GRAFT_PR_TOKEN` ([Before you begin](#before-you-begin)).
  - `listing PRs: HTTP 401`: `GRAFT_PR_TOKEN` expired or was revoked. Replace the secret.

  Then run the workflow again.

## How it works

- **Two jobs.** The **upgrade** job runs `nix run .#graft ci upgrade`: `graft upgrade`, then the whole
  `nix flake check` on the result. It commits and force-pushes `graft/upgrade` even when a check fails, and
  then fails. It holds only the job token. The **pr** job runs `nix run .#graft ci pr` from the default
  branch, with `GRAFT_PR_TOKEN`. It opens the pull request, or rewrites the title and description of the
  open one. Why: [CI security](../ci-security.md).
- **graft-check** runs `nix run .#graft ci check` for pull requests and pushes to the default branch that
  Forgejo starts workflows for. It builds only graft's check. When it fails, see
  [`graft check` fails](check-fails.md). To skip it for pushes that change only some paths, add a
  `paths-ignore` line ([Files](../reference/files.md#the-paths-ignore-line)).
- **Run the steps yourself.** On GitHub, or to test, run the same commands from the top of a fresh clone
  of the default branch. `ci upgrade` needs `BRANCH`. It commits every change on the branch `BRANCH`, and
  force-pushes it to `origin`. It refuses a working tree with changes of its own, and any other branch:

  ```
  git switch -c graft/upgrade
  BRANCH=graft/upgrade nix run .#graft ci upgrade
  git switch main
  ```

  On `main`, it prints
  `graft: ci upgrade commits on the branch checked out, main, and pushes it to graft/upgrade: switch to a new branch graft/upgrade first (git switch -c graft/upgrade)`.
  With a changed or untracked file, it lists them, then prints
  `graft: ci upgrade commits every change in the working tree, and this one has the changes of its own above: commit, stash or remove them first`. A `flake.lock` that Nix re-locked is the one change it takes: the upgrade commit includes it.

  `ci pr` uses Forgejo's API, and needs `API`, `WEB`, `BASE`, `BRANCH` and `PR_TOKEN`. On GitHub, open the
  pull request yourself, from a job checked out at the default branch, with `contents: write` and
  `pull-requests: write` permissions, and the repository setting that allows GitHub Actions to create pull
  requests. For example: `gh pr create --head graft/upgrade --title "graft: upgrade" || gh pr edit graft/upgrade --title "graft: upgrade"`.
  A pull request opened with `GITHUB_TOKEN` starts no workflows. Without `ci pr`, nothing refuses a branch
  that changes other files: check its file list (step 6) yourself. graft does not manage files in
  `.github/workflows/`.
- **Many repositories.** graft has no command for many repositories. Run `outdated` from graft's flake over
  each checkout:

  ```
  for r in ~/src/team-kb ~/src/handbook; do echo "$r"; nix run github:danielnaab/graft/v2.12.0 -- outdated -C "$r"; done
  ```

## Related

[Take a newer version by hand](upgrade.md), [Make a pattern's checks gate your merges](gate-merges.md),
[Graft a private repository](private-providers.md), [CI security](../ci-security.md).

## Keep your own edits to a workflow

Moved to [Files](../reference/files.md#keep-your-own-edits-to-a-workflow).

## Skip pushes that change only some paths

Moved to [Files](../reference/files.md#the-paths-ignore-line).

## Sources

- [lib/ci.sh](../../lib/ci.sh): `check`; `upgrade`, which refuses a changed tree or another branch, stages
  everything, commits, pushes, and fails after a failing check; `pr`, which refuses a branch that changes other files or workflows, titles a
  failing upgrade, writes the description, edits an open pull request, and reports HTTP failures.
- [lib/common.sh](../../lib/common.sh): `full_check` and `log_tail`, `upstream_input`, `upstream_range` and
  `changelog_between`, which read each provider's CHANGELOG between the two pins.
- [lib/upstream.jq](../../lib/upstream.jq): `upstream_notes`, which prints them, Upgrading first, for the pull
  request and `changelog`.
- [lib/tools.sh](../../lib/tools.sh): the lines of `upgrade`, the new major version that it holds,
  `nothing to upgrade`, and `can't tell whether … is current`; `outdated --json`.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-upgrade.yaml): the schedule, the
  two jobs, and `GRAFT_PR_TOKEN`.
- [tests/run.sh](../../tests/run.sh): "graft ci, the workflows' steps, against a local origin", "graft ci pr
  against a stub API", "the API's failures", and the `gate:` checks; [tests/stub-api.py](../../tests/stub-api.py).
- [tests/fixtures.sh](../../tests/fixtures.sh) `ci`: the repositories that this page was read against.
