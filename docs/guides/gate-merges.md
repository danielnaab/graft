---
status: working
---
# Make a pattern's checks gate your merges

Run a pattern's checker in `nix flake check`, run that in CI on every pull request, and make your forge
require it before a merge.

## Before you begin

- The repository grafts the pattern, and `nix flake check` passes ([Graft your first pattern](graft-a-repository.md)).
  The example grafts `page-rules`, a provider with one rule (each page in `docs/` gives its status in front
  matter) and a Python checker. Its README, `scions/page-rules/README.md`, says how to run the checker:
  `python3 -m pagecheck DIR`, with the provider's directory on `PYTHONPATH`. Read your provider's README
  for its own command.
- CI must be able to fetch the provider. For a private provider, see
  [Private providers](private-providers.md#check-in-ci).
- `flake.nix` is at the repository's top. A flake in a subdirectory sees only that directory, so a check in it
  cannot read your pages. Instead, skip steps 1 to 5, and in step 6's workflow run the checker from the
  scion in place of `nix flake check`:

  ```
  PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scions/page-rules nix run nixpkgs#python3 -- -m pagecheck docs
  ```

  It copies none of your files into the Nix store. `PYTHONDONTWRITEBYTECODE=1` keeps Python from writing
  `__pycache__/` into the scion. Keep `graft-check.yaml` beside it: it runs graft's check.

## Steps

1. In `flake.nix`, give `mkFlake` a check that runs the checker. If the flake has no `nixpkgs` input, add
   one, and make graft use it:

   ```nix
   inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
   inputs.graft.inputs.nixpkgs.follows = "nixpkgs";
   ```

   Then pass the check as `outputs.checks`:

   ```nix
   outputs = { self, nixpkgs, graft, ... }@inputs: graft.lib.mkFlake {
     inherit inputs;
     outputs.checks = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] (system:
       let pkgs = nixpkgs.legacyPackages.${system}; in {
         pages = pkgs.runCommand "pages" { nativeBuildInputs = [ pkgs.python3 ]; } ''
           cd ${self}
           PYTHONPATH=${inputs.page-rules} python3 -m pagecheck docs
           touch $out
         '';
       });
   };
   ```

   `${inputs.page-rules}` is the provider's fetched export, and `${self}` is your repository. `pages` is your
   name for the check: any name but `graft`. If your outputs already have checks for each system, add
   `pages = …;` beside them, in the same set.

2. Stage your pages:

   ```
   git add docs/
   ```

   A flake sees only what git tracks, so the check cannot see a new page until you add it.

3. Run the check:

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

4. Fix each page that the check names. In the example, add front matter to the top of
   `docs/outage-2026-11-03.md`, and, if your repository has `docs/retry-idea.md`, change its `status: idea`
   to `status: draft`:

   ```
   ---
   status: working
   ---
   ```

5. Run the check again:

   ```
   nix flake check
   ```

   It exits 0. Nix warns `The check omitted these incompatible systems: …`: it builds the checks of your
   machine's system only.

6. Write a workflow that runs `nix flake check` on each pull request. On Forgejo, write
   `.forgejo/workflows/flake-check.yaml`:

   ```yaml
   name: flake-check
   on:
     pull_request:
     push:
       branches: [main]

   concurrency:
     group: flake-check-${{ github.ref }}
     cancel-in-progress: true

   jobs:
     flake-check:
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

   It uses the runner label of graft's workflows
   ([Run graft in CI and review its upgrade pull requests](ci.md)). Like `graft-check.yaml`, it cancels a run
   when a newer push to the same branch starts one. Change `main` to your default branch.

   graft's `graft-check.yaml` stays as it is: it checks only that the scions match their pins. Your checks
   are a status of their own, from this workflow, which you own. graft never changes a workflow that does
   not carry its marker line.

   Give this workflow no secrets. On graft's upgrade pull request, `nix flake check` runs the new graft
   version and the new provider's checker before anyone has reviewed them ([CI security](../ci-security.md)).

   On GitHub, write `.github/workflows/flake-check.yaml` with the same `name:`, triggers and job key, and
   these steps: `uses: actions/checkout@v7`, `uses: cachix/install-nix-action@v31`, then
   `run: nix flake check`.

7. On a new branch, commit:

   ```
   git switch -c check-pages
   git add flake.nix docs/ .forgejo/workflows/flake-check.yaml
   git commit -m "Check pages against page-rules"
   ```

8. Push the branch, and open a pull request. When the workflow runs, the pull request's checks list
   `flake-check / flake-check (pull_request)`. Forgejo names a status `WORKFLOW / JOB (EVENT)`, from the
   workflow's `name:` and the job's key. Merge the pull request when it passes.

9. Protect the default branch. In the repository's **Settings**, open **Branches**, and choose
   **Add new rule**:

   - **Protected branch name pattern:** `main`
   - **Enable status check:** on
   - **Status check patterns:** `flake-check / flake-check (pull_request)`
   - **Enforce this rule for repository admins:** on, unless an administrator must be able to merge
     without the check.

   Then choose **Save rule**. A pull request whose check failed, or did not run, cannot merge.

   On GitHub, add a branch rule with **Require status checks to pass before merging**, and add the job,
   `flake-check`.

   The rule also holds graft's weekly upgrade pull request until a person starts the check on it: Forgejo
   starts no workflow for what a job token did, and graft's upgrade job pushes with one. See
   [If something goes wrong](#if-something-goes-wrong).

10. Test the gate in the forge. On a new branch from the default branch, add a page with no front matter,
    push it, and open a pull request. Forgejo says `Some required checks were not successful.`, and you
    cannot merge. Close the pull request, and delete the branch.

## Check it worked

From the repository's top, paste this. It adds a page that breaks the rule, expects the check to fail on
it, removes the page, and expects the check to pass. It prints `ok` only if both happen:

```sh
printf '# Gate test\n' > docs/gate-test.md && git add docs/gate-test.md && nix flake check 2>&1 | grep -q 'docs/gate-test.md: no status' && git rm -qf docs/gate-test.md && nix flake check && echo ok
```

If it stops without `ok`, run `git rm -f docs/gate-test.md` to remove the test page.

That proves the check. To prove that Forgejo requires it, you need `curl` and `jq`, and a token with the
`read:repository` scope of a user who administers the repository: Forgejo shows branch protection only to
repository administrators. Set `FORGEJO_TOKEN` to it, put your server, owner and repository in the URL, and
paste this. It prints `ok` only if the
rule for `main` requires the status:

```sh
curl -fsS -H "Authorization: token $FORGEJO_TOKEN" "https://forgejo.example/api/v1/repos/OWNER/REPO/branch_protections/main" | jq -e '.enable_status_check and (.status_check_contexts | index("flake-check / flake-check (pull_request)"))' >/dev/null && echo ok
```

## If something goes wrong

- **A page that breaks the rule passes `nix flake check`.** git does not track it yet, and a flake sees only
  what git tracks: `git status --short` lists it with `??`. Do step 2 again, then check again. CI checks
  only committed pages, so the gate still holds.
- **`error: attribute 'page_rules' missing`, and `Did you mean page-rules?`.** The name in
  `${inputs.…}` is not the input's name in `flake.nix`. Use the input's name exactly.
- **graft's upgrade pull request says `Some required checks are missing.`** No workflow ran on its newest
  commit. graft's upgrade job pushes `graft/upgrade` with the job token, and Forgejo starts no workflow for
  that push. Your workflow runs only when graft opens the pull request with `GRAFT_PR_TOKEN`, a user's
  token. It does not run when graft opens the pull request with the job token, or when a later week's
  upgrade updates it. The upgrade job has already run your whole `nix flake check` on the branch: if it
  failed, the title starts `[checks failing]`. To get the status, close the pull request and reopen it:
  that starts the workflow as you.

  Not yet: graft's upgrade job does not report its `nix flake check` as a status on the commit.
- **Another pull request says `Some required checks are missing.`** No status matches the pattern of
  step 9. Compare the pattern with the names in the pull request's checks: a changed workflow `name:` or
  job key changes the status name. A workflow that skips the pull request (`paths-ignore`, say) sends no
  status at all.

## How it works

- **Why `${inputs.page-rules}` and not `scions/page-rules`.** The fetched export is the pin itself, in the
  read-only Nix store ([Concepts](../concepts.md#the-nix-binding)). The check is a Nix build that uses only
  the tools it names, at the versions that `flake.lock` pins, so CI uses the same Python as you. It is part
  of `nix flake check`, which graft's weekly upgrade also runs, so a new version of the pattern is tested on
  your pages before you merge it ([Upgrade](upgrade.md)).
- **The other way.** You can run the checker from the scion in the working tree, as in
  [Before you begin](#before-you-begin), in a script or a pre-commit hook. It uses an unpinned Python, and
  it is not part of `nix flake check`, so graft's upgrade does not run it. Without
  `PYTHONDONTWRITEBYTECODE=1`, Python writes `__pycache__/` into the scion, and `graft check` fails
  ([`graft check` fails](check-fails.md), step 7).
- `mkFlake` merges `outputs` with its own at every level, so your `checks` sit beside `checks.<system>.graft`
  ([Commands](../reference/commands.md#mkflake)).

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the merge of a repository's own `outputs` (`own`, and
  `lib.recursiveUpdate`).
- [flake.nix](../../flake.nix): graft's own `checks.docs`, which runs knowledge-base-kit's checkers from
  `${inputs.knowledge-base-kit}` the same way.
- [lib/ci.sh](../../lib/ci.sh): `check`, which builds only `checks.<system>.graft`; `upgrade`, which runs the
  whole `nix flake check` and pushes; `pr`, which titles a failing upgrade.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/): `graft-check.yaml`, whose runner label
  and checkout step 6 copies, and `graft-upgrade.yaml`, which pushes with the job token and opens the pull
  request with `GRAFT_PR_TOKEN`.
- [tests/run.sh](../../tests/run.sh): "gate: ci upgrade fails when the repo's own check fails", and "own
  outputs evaluate without graft's argument checks".
- [tests/fixtures.sh](../../tests/fixtures.sh) `gate-merges`: the repositories this page was read against.
- Forgejo's source, read but not run against a forge for this page: the status name
  ([services/actions/commit_status.go](https://codeberg.org/forgejo/forgejo/src/branch/forgejo/services/actions/commit_status.go)),
  no workflow for what the Actions user did
  ([services/actions/notifier_helper.go](https://codeberg.org/forgejo/forgejo/src/branch/forgejo/services/actions/notifier_helper.go)),
  the default `pull_request` types
  ([modules/actions/workflows.go](https://codeberg.org/forgejo/forgejo/src/branch/forgejo/modules/actions/workflows.go)),
  who may read branch protection
  ([routers/api/v1/api.go](https://codeberg.org/forgejo/forgejo/src/branch/forgejo/routers/api/v1/api.go)),
  and the settings' labels and messages
  ([locale_en-US.ini](https://codeberg.org/forgejo/forgejo/src/branch/forgejo/options/locale/locale_en-US.ini)).
