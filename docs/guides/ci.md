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

## Keep your own edits to a workflow

graft's workflows in `.forgejo/workflows/` are graft's while they carry the marker line
`# managed by graft: …`, their first line. graft keeps the `branches:`, `paths-ignore:` and
`# graft ssh:` lines with the marker ([Files](../reference/files.md#grafts-workflows)). For any other edit:

1. Delete the marker line. Do this before you run `nix run .#graft`: while the marker is there, it puts
   graft's copy back.
2. Make your edits. The comment below the old marker still says that graft replaces the file: change it
   too.
3. Run `nix run .#graft check`. It prints `graft: scions/ and graft's workflows match the pins`, because
   graft no longer reads the file.
4. Commit the workflow.

Without the marker, `nix run .#graft upgrade` no longer replaces the file, so you get none of graft's later
changes to it. To take graft's copy again, put the marker line back as the first line and run
`nix run .#graft`. It prints `graft: refreshed .forgejo/workflows/FILE`, keeps the `branches:`,
`paths-ignore:` and `# graft ssh:` lines, and drops your other edits.

If `nix run .#graft` printed `graft: refreshed .forgejo/workflows/FILE` over an edit that you had committed
or staged, get it back with `git restore -- .forgejo/workflows/FILE`, then delete the marker line. graft
does not stage the workflows, so git still has the edit. An edit that git did not have is lost.

Related: [Private providers](private-providers.md#check-in-ci), [Upgrade](upgrade.md).
