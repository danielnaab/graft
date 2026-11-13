---
status: draft
---
# Make a pattern's checks gate your merges

Run a pattern's checker in `nix flake check`, run that in CI on every pull request, and make your forge
require it before a merge.

## Before you begin

- The repository grafts the pattern, and `nix flake check` passes ([Graft your first pattern](graft-a-repository.md)).
  The example grafts `page-rules`, a provider with one rule (each page in `docs/` gives its status in front
  matter) and a Python checker, `pagecheck`. Read your provider's own name, checker and folder for these.
- `flake.nix` is at the repository's top. A flake in a subdirectory sees only that directory, so its checks
  cannot read your pages ([A flake in a subdirectory](flake-in-a-subdirectory.md)).

  Not yet: a way to gate merges from a flake in a subdirectory.

## Steps

1. Find how the provider runs its checker. It is usually in the scion's README:

   ```
   cat scions/page-rules/README.md
   ```

   Here it says: `python3 -m pagecheck DIR`, with the provider's directory on `PYTHONPATH`. It exits 1 when a
   page breaks the rule.

2. In `flake.nix`, pass a check to `mkFlake` as `outputs.checks`. It runs the checker from the fetched
   export, `${inputs.page-rules}`, over your pages, `${inputs.self}`:

   ```nix
   outputs = inputs: inputs.graft.lib.mkFlake {
     inherit inputs;
     outputs.checks = builtins.mapAttrs (system: pkgs: {
       pages = pkgs.runCommand "pages" { nativeBuildInputs = [ pkgs.python3 ]; } ''
         cd ${inputs.self}
         PYTHONPATH=${inputs.page-rules} python3 -m pagecheck docs
         touch $out
       '';
     }) inputs.graft.inputs.nixpkgs.legacyPackages;
   };
   ```

   `pages` is your name for the check: any name but `graft`. The example takes Python from the nixpkgs that
   graft pins. If your flake has a `nixpkgs` input, use `nixpkgs.legacyPackages` instead. If your outputs
   already have `checks`, add `pages` to them.

3. Stage your pages:

   ```
   git add docs/
   ```

   A flake sees only what git tracks, so the check cannot see a new page until you add it.

4. Run the check:

   ```
   nix flake check
   ```

   Nix warns `Git tree '…' is dirty` until you commit. The check fails, and names each page that breaks
   the rule:

   ```
   error: builder for '/nix/store/…-pages.drv' failed with exit code 1;
          last 2 log lines:
          > docs/outage-2026-11-03.md: no status in front matter (rule: page-rules/README.md)
          > docs/retry-idea.md: status 'idea' is not one of draft, working, stable, deprecated
   ```

5. Fix each page that the check names. Here, add front matter to the top of `docs/outage-2026-11-03.md`:

   ```
   ---
   status: working
   ---
   ```

   In `docs/retry-idea.md`, change `status: idea` to `status: draft`.

6. Run the check again:

   ```
   nix flake check
   ```

   It exits 0. Nix may warn `The check omitted these incompatible systems: …`: it builds the checks of your
   machine's system only.

7. Commit:

   ```
   git add flake.nix docs/
   git commit -m "Check pages against page-rules"
   ```

8. Add a workflow that runs `nix flake check` on each pull request. graft's `graft-check.yaml` runs only
   graft's own check, `checks.<system>.graft`. On Forgejo, write `.forgejo/workflows/flake-check.yaml`:

   ```yaml
   name: flake-check
   on:
     pull_request:
     push:
       branches: [main]

   jobs:
     checks:
       runs-on: nixos-latest
       timeout-minutes: 30
       env:
         NIX_CONFIG: |
           experimental-features = nix-command flakes
           sandbox = false
       steps:
         - name: Checkout
           env:
             REPO_URL: ${{ github.server_url }}/${{ github.repository }}
             SHA: ${{ github.sha }}
             TOKEN: ${{ github.token }}
           run: |
             set -euo pipefail
             git init -q .
             git remote add origin "$REPO_URL.git"
             git -c "http.extraheader=Authorization: Basic $(printf 'x-access-token:%s' "$TOKEN" | base64 -w0)" \
               fetch -q --no-tags --depth=1 origin "$SHA"
             git checkout -q --force --detach "$SHA"
         - name: nix flake check
           run: nix flake check
   ```

   It uses the runner label of graft's workflows ([CI](ci.md)). Change `main` to your default branch.

   Not yet: graft's `graft-check.yaml` does not run your `nix flake check`, so you keep this workflow
   yourself.

9. Commit the workflow, push a branch, and open a pull request. Its checks list
   `flake-check / checks (pull_request)`. Forgejo names a status `WORKFLOW / JOB (EVENT)`, from the
   workflow's `name:` and the job's key.

10. Protect the default branch. In the repository's **Settings**, open **Branches**, and choose
    **Add new rule**:

    - **Protected branch name pattern:** `main`
    - **Enable status check:** on
    - **Status check patterns:** `flake-check / checks (pull_request)`

    Then choose **Save rule**. A pull request whose check fails, or has not run, cannot merge. Forgejo still
    lets an administrator merge it.

    On GitHub, run the same check from `.github/workflows/flake-check.yaml`: steps `actions/checkout@v4`,
    `cachix/install-nix-action@v31`, then `nix flake check`. In the branch rule, turn on
    **Require status checks to pass before merging**, and add the job's name, `checks`.

## Check it worked

From the repository's top, paste this. It adds a page that breaks the rule and expects the check to fail
on it, removes the page, and expects the check to pass. It prints `ok` only if both happen:

```sh
printf '# Gate test\n' > docs/gate-test.md && git add docs/gate-test.md && nix flake check 2>&1 | grep -q 'docs/gate-test.md: no status' && git rm -qf docs/gate-test.md && nix flake check && echo ok
```

If it stops without `ok`, run `git rm -f docs/gate-test.md` to remove the test page.

To see the forge refuse it too, push the same page on a branch and open a pull request: Forgejo says
`Some required checks were not successful.`, and the merge stays blocked until you remove the page.

## If something goes wrong

- **A page that breaks the rule passes `nix flake check`.** git does not track it yet, and a flake sees only
  what git tracks: `git status --short` lists it with `??`. Do step 3 again, then check again. CI checks
  only committed pages, so the gate still holds.
- **`error: attribute 'page_rules' missing`, and `Did you mean page-rules?`.** The name in
  `${inputs.…}` is not the input's name in `flake.nix`. Use the input's name exactly.
- **The pull request says `Some required checks are missing.`** No status matches the pattern of step 10.
  Compare the pattern with the names in the pull request's checks: a changed workflow `name:` or job key
  changes the status name. A workflow that skips the pull request (`paths-ignore`, say) sends no status at all.
- **`graft check` fails with `Only in …/scions/page-rules/pagecheck: __pycache__`.** Someone ran the
  checker from the scion, not from the fetched export (see [How it works](#how-it-works)). Remove the
  directory with `nix run .#graft`, and see [`graft check` fails](check-fails.md) step 7.

## How it works

- **Why `${inputs.page-rules}` and not `scions/page-rules`.** The fetched export is the pin itself, in the
  read-only Nix store ([Concepts](../concepts.md#the-nix-binding)). The check runs as a Nix build, with only
  the tools it names, so it gives the same result on your machine and in CI. It is part of `nix flake check`,
  which graft's weekly upgrade also runs, so a new version of the pattern is tested on your pages before
  you merge it ([Upgrade](upgrade.md)).
- **The other way.** You can run the checker from the scion in the working tree, for example
  `PYTHONPATH=scions/page-rules python3 -m pagecheck docs` in a script or a pre-commit hook. It is quick
  while you write, but it uses whatever Python is on `PATH`, and it is not in `nix flake check`, so CI and
  upgrades do not run it. Python writes `__pycache__/` into the scion, which fails `graft check`: set
  `PYTHONDONTWRITEBYTECODE=1`, or ignore `__pycache__/` in `.gitignore`.
- `mkFlake` merges `outputs` with its own at every level, so your `checks` sit beside `checks.<system>.graft`
  ([Commands](../reference/commands.md#mkflake)).

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the merge of a repository's own `outputs` (`own`, and
  `lib.recursiveUpdate`).
- [flake.nix](../../flake.nix): graft's own `checks.docs`, which runs knowledge-base-kit's checkers from
  `${inputs.knowledge-base-kit}` the same way.
- [lib/ci.sh](../../lib/ci.sh): `check`, which builds only `checks.<system>.graft`; `upgrade`, which runs the
  whole `nix flake check`.
- [template/.forgejo/workflows/graft-check.yaml](../../template/.forgejo/workflows/graft-check.yaml): the
  runner label and checkout that step 7 copies.
- [tests/run.sh](../../tests/run.sh): "gate: ci upgrade fails when the repo's own check fails", and "own
  outputs evaluate without graft's argument checks".
- [tests/fixtures.sh](../../tests/fixtures.sh) `gate-merges`: the repositories this page was read against.
- Forgejo's [commit status names](https://codeberg.org/forgejo/forgejo/src/branch/forgejo/services/actions/commit_status.go)
  (`createCommitStatus`).
