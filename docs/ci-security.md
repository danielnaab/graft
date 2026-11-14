---
status: working
---
# CI security

Why graft's upgrade workflow is two jobs, and what it still trusts. Read this before you change the
workflows or give them more secrets.

## The risk

A weekly upgrade moves scions, and can move graft's own pin. The new versions are code that nobody has
reviewed yet, and the upgrade runs it: each provider's `graft.nix`, tools and checks, the new graft version's
commands, and the repository's own checks over them. The workflows run with `sandbox = false`, so that code is
not in Nix's sandbox.

Opening a pull request is also a risk. It runs the repository's `pull_request` workflows from the branch,
with the repository's secrets.

## The two jobs

1. **upgrade** runs `graft upgrade`, commits, and runs `nix flake check` on the commit, then pushes the branch
   `graft/upgrade`. It holds the job token, and the `# graft ssh:` key if the repository has a private scion.
   Protect the default branch, so that the job token cannot push to it.
2. **pr** is checked out at the default branch. It runs the graft version that the default branch pins, and
   never evaluates the upgraded flake. It opens the pull request with `GRAFT_PR_TOKEN`.

The **pr** job opens a pull request only if the branch changes nothing but gitlinks of existing scions, and,
when graft's own pin moved, graft's lines in `flake.nix` and `flake.lock`, the marker README and graft's
marked workflows. A `.gitmodules` change (a provider swapped for another URL) and any other file are refused,
with a link to the diff. Read the diff, then open the pull request yourself
([If something goes wrong](guides/ci.md#if-something-goes-wrong)).

When graft's pin moves, its refreshed workflows are in the pull request, and the pull request's own
`pull_request` workflows run from the branch: the new graft workflows run with your secrets before you merge.
They are graft's release, which you chose to trust by pinning graft; read their diff in the pull request
first ([Run graft in CI and review its upgrade pull requests](guides/ci.md#steps), step 7).

The **pr** job writes the pull request body from data only:

- It reads the gitlinks of the default branch and of `graft/upgrade` as data.
- For each moved scion, it fetches the provider with plain `git`, from the URL in the **default branch's**
  `.gitmodules`, and reads its `CHANGELOG.md` between the two commits. Nothing is checked out or run, and
  `GRAFT_PR_TOKEN` is not in git's environment.
- It passes the provider's text through `jq`, never through a shell command line. The text is quoted as the
  provider's, not as instructions.

## Limits

- **Each job needs a new environment**, such as a container per job (`docker://nixos/nix`). The upgrade job
  runs unreviewed code outside Nix's sandbox. On a runner that shares `$HOME` or a writable `/nix/store`
  between jobs, it can leave code for the **pr** job to run.
- **Logs are public.** A failing check's log goes into the commit message and the pull request, which are as
  public as the repository. A check that prints secrets publishes them there.
- **`flake.nix`, `flake.lock` and the gitlinks are code.** A moved pin is the new version. A `pull_request`
  workflow of your own that evaluates or builds the flake with secrets runs that version before review.
  graft's workflows do not hold secrets in the jobs that build. Keep yours the same.
- **graft moves itself only from `graft ci`.** `graft upgrade` refuses to move graft's pin from another CI
  job, because that job could run the new version beside the token.
- **A private scion's key is in the upgrade job.** Its `nix flake check` must fetch the private scion at its
  new tag. Use a read-only deploy key for that one provider: the unreviewed code that the job runs is that
  provider's own new version, which the key can already read. Without the key, upgrade private scions by
  hand.

## Sources

- [lib/ci.sh](../lib/ci.sh): `check`, `upgrade`, `pr`
- [template/.forgejo/workflows/](../template/.forgejo/workflows/graft-upgrade.yaml)
- [lib/tools.sh](../lib/tools.sh): the refusal to move graft outside `graft ci`
