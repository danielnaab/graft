---
status: working
---
# CI

Check scions on every change, and upgrade them every week, with graft's two Forgejo Actions workflows.

## Set up

1. If you did not use `nix flake init -t github:danielnaab/graft`, copy graft's
   `template/.forgejo/workflows/` to `.forgejo/workflows/`.
2. Make sure that a runner has the label `nixos-latest`, mapped to `docker://nixos/nix`.
3. If the repository is private, add the Actions secret `GRAFT_PR_TOKEN`: a token with
   `write:repository` for a user who can write to the repository.
4. Protect the default branch, so that the job token cannot push to it.

Notes:

- The job token is the token that Forgejo gives each job (`github.token`). Forgejo refuses it when it
  opens a pull request on a private repository. Without `GRAFT_PR_TOKEN`, the **pr** job then fails with
  HTTP 404.
- A bot account is the best owner of `GRAFT_PR_TOKEN`, because it becomes the pull request's author. A
  user-level secret covers all that user's repositories.
- graft-check runs on pushes to `main`. If your default branch has another name, change
  `branches: [main]` under `push:` to that name. graft keeps the line
  ([Files](../reference/files.md#the-branches-line)).
- The workflows are for Forgejo. On GitHub, you can run `nix run .#graft ci check` and `ci upgrade` from
  workflows of your own. graft does not manage those files, and `ci pr` uses Forgejo's API.

## The workflows

| Workflow | When | What |
|---|---|---|
| `graft-check.yaml` | Pull requests, and pushes to `main` (or the branches you name) | Checks that the scions and graft's workflows match `flake.lock`. |
| `graft-upgrade.yaml` | Weekly, and on demand | Runs `graft upgrade`. If a pin moved, runs the whole `nix flake check`, force-pushes the branch `graft/upgrade`, and opens a pull request or updates the open one. |

The upgrade pull request says what moved. It quotes each provider's CHANGELOG sections between the tags
(or its commits, if it has no CHANGELOG), and names newer major versions that it did not take. It ends
with a `graft migrate -C . --no-check` command at the graft version that the default branch pins: the
safe way to brief an agent on that branch.

If a check fails, graft still pushes the branch. It titles the pull request
`[checks failing] graft: upgrade`, adds the end of each failing log, and fails the run. It never pushes to
the default branch.

graft keeps the workflow files current: see [Files](../reference/files.md#grafts-workflows). The reasons
for the two-job design are in [CI security](../ci-security.md).

## Skip pushes that change only some paths

If something pushes often to files that graft's check never reads (a notes vault that a bot syncs, for
example), you can skip graft-check for those pushes.

1. Add one `paths-ignore` line under both triggers of `graft-check.yaml`:

   ```yaml
   on:
     pull_request:
       paths-ignore: ["notebook/**"]
     push:
       branches: [main]
       paths-ignore: ["notebook/**"]
   ```

2. Run `nix run .#graft check`.
3. Commit.

graft keeps the line through sync and upgrade. The rules are in
[Files](../reference/files.md#the-paths-ignore-line).

Notes:

- A pull request that changes only ignored files gets no graft-check status. Do not make that status
  required in branch protection.
- graft-check needs no schedule, because an ignored push cannot change what it checks. Check the ignored
  files with a hook or a workflow of your own, for example on a daily `schedule:`.

Related: [Make a pattern's checks gate your merges](gate-merges.md) (graft-check runs only graft's check), [Private providers](private-providers.md#check-in-ci), [Upgrade](upgrade.md).
