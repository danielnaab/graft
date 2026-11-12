---
status: working
---
# CI security

Why graft's upgrade workflow is two jobs, and what it still trusts. Read this before you change the
workflows or give them more secrets.

## The risk

A weekly upgrade can move graft's own pin. The new graft version is code that nobody has reviewed yet,
and the upgrade runs it: its `assemble.sh`, its check, and the repository's own checks over new provider
content. The workflows also run with `sandbox = false`, so that code is not in Nix's sandbox.

Opening a pull request is also a risk. It runs the repository's `pull_request` workflows from the
branch, with the repository's secrets.

## The two jobs

1. **upgrade** runs `graft upgrade` and the checks, then pushes the branch `graft/upgrade`. It holds only
   the job token. Protect the default branch, so that this token cannot push to it.
2. **pr** is checked out at the default branch. It runs the graft version that the default branch pins,
   and never evaluates the upgraded flake. It opens the pull request with `GRAFT_PR_TOKEN`.

The **pr** job opens a pull request only if the branch changes nothing but `flake.nix`, `flake.lock` and
the scions directories. A branch that changes anything else (a new graft version's workflow files, for
example) could run its own code with the repository's secrets, so the job fails with a link to the diff.
Read the diff, then open the pull request yourself.

The **pr** job writes the pull request body from data only:

- It reads the two `flake.lock` files as data.
- For each input that moved, it fetches the provider with plain `git`, from the URL that the
  **default branch's** lock names, at the commit that the branch's lock pins. Nothing is checked out or
  run, and `GRAFT_PR_TOKEN` is not in git's environment.
- It passes the provider's text through `jq`, never through a shell command line. The text is quoted as
  the provider's, not as instructions.

Building the `graft` app in this job fetches the default branch's scions through Nix, with the token in
the environment. Those are the reviewed lock's URLs, nothing is sent to them, and the build only copies.

## Limits

- **Each job needs a new environment**, such as a container per job (`docker://nixos/nix`). The upgrade
  job runs unreviewed code outside Nix's sandbox. On a runner that shares `$HOME` or a writable
  `/nix/store` between jobs, it can leave code for the **pr** job to run.
- **Logs are public.** A failing check's log goes into the commit message and the pull request, which are
  as public as the repository. A check that prints secrets publishes them there.
- **`flake.nix` and `flake.lock` are code.** A moved graft pin is the new version. A `pull_request`
  workflow of your own that evaluates or builds the flake with secrets runs that version before review.
  graft's workflows do not. Keep yours the same, or do not let graft upgrade itself in CI.
- **graft moves itself only from `graft ci`.** `graft upgrade` refuses to move graft's pin from another
  CI job, because that job could run the new version beside the token.
- **Private scions are upgraded by hand.** The upgrade job runs unreviewed code, so it must not hold an
  ssh key. Upgrade a private scion with `nix run .#graft upgrade`, or from a runner that has the key.

## Sources

- [lib/ci.sh](../lib/ci.sh): `check`, `upgrade`, `pr`
- [template/.forgejo/workflows/](../template/.forgejo/workflows/graft-upgrade.yaml)
- [lib/tools.sh](../lib/tools.sh): the refusal to move graft outside `graft ci`
