---
status: working
---
# Make a pattern's checks gate your merges

Run a pattern's checker in `nix flake check`, run that in CI on every pull request, and make your forge
require it before a merge.

## Before you begin

- The repository grafts the pattern, and `nix flake check` passes ([Graft your first pattern](graft-a-repository.md)).
  The example grafts `page-rules`, a provider with one rule (each page in `docs/` gives its status in front
  matter) and a checker. Its `graft.nix` contributes a check, `pages`, that runs the checker on your `docs/`.
- CI must be able to fetch the provider. For a private provider, see
  [Graft a private repository](private-providers.md).
- `flake.nix` is at the repository's top. A flake in a subdirectory sees only that directory, so a provider's
  check there cannot read your pages: see [A flake in a subdirectory](#a-flake-in-a-subdirectory).

## Steps

1. List the checks that your providers contribute:

   ```
   nix eval .#checks.x86_64-linux --apply builtins.attrNames
   ```

   ```
   [ "page-rules-pages" ]
   ```

   Each is named `SCION-CHECK`. Use your machine's system for `x86_64-linux`. If a provider has no check, its
   README says how to run its tool; add a check of your own ([A check of your own](#a-check-of-your-own)).

2. If the provider reads a settings file, write it. Its README names the file and its fields. A check skips,
   and passes, when the file is absent.

3. Stage your pages:

   ```
   git add docs/
   ```

   A flake sees only what git tracks, so the check cannot see a new page until you add it.

4. Run the checks:

   ```
   nix flake check
   ```

   The check fails, and names each page that breaks the rule:

   ```
   error: builder for '/nix/store/…-pages.drv' failed with exit code 1;
          last 2 log lines:
          > docs/outage-2026-11-03.md: no status in front matter (rule: page-rules/README.md)
          > docs/retry-idea.md: status 'idea' is not one of draft, working, stable, deprecated
   ```

   Fix each page that it names, stage it, and run the check again until it exits 0.

5. Commit the pages, and any settings file:

   ```
   git add docs/
   git commit -m "Pages pass page-rules"
   ```

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

   graft's `graft-check.yaml` stays as it is: it checks that the scions are clean and at release tags. Your
   checks are a status of their own, from this workflow, which you own. graft never changes a workflow that does
   not carry its marker line.

   Give this workflow no secrets. On graft's upgrade pull request, `nix flake check` runs the new graft
   version and the new provider's checker before anyone has reviewed them ([CI security](../ci-security.md)).

   On GitHub, write `.github/workflows/flake-check.yaml` with the same `name:`, triggers and job key, and
   these steps: `uses: actions/checkout@v7`, `uses: cachix/install-nix-action@v31` (Nix 2.27 or later), then
   `run: nix flake check`. A top-level git flake does not need `submodules: true`: Nix fetches the scions.

7. On a new branch, commit:

   ```
   git switch -c check-pages
   git add .forgejo/workflows/flake-check.yaml
   git commit -m "Run nix flake check on pull requests"
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
- **The check names no page, and passes, though a page breaks the rule.** The provider's check reads a
  settings file that is absent, and skips. Write it (step 2).
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

- **Where the check comes from.** The provider's `graft.nix` returns `checks.pages`, a Nix build that reads
  your repository through `consumer`. mkFlake adds it as `checks.<system>.page-rules-pages`. It uses the tools the
  provider names, at the versions that the scion's commit pins, so CI uses the same checker as you. It is part
  of `nix flake check`, which graft's weekly upgrade also runs, so a new version of the pattern is tested on
  your pages before you merge it.
- **Which pages it sees.** A git flake's source is what is committed, plus uncommitted edits to tracked files.
  CI checks only committed pages, so the gate holds.

### A check of your own

When a provider ships a tool but no check, write one with `mkFlake`'s `perSystem`. Its `scions` argument has
each provider's tools:

```nix
outputs = { self, ... }@inputs: inputs.graft.lib.mkFlake {
  inherit inputs;
  perSystem = { pkgs, scions, ... }: {
    checks.pages = pkgs.runCommand "pages" { nativeBuildInputs = [ scions.page-rules.tools.pagecheck ]; } ''
      cd ${self}
      pagecheck docs
      touch $out
    '';
  };
};
```

It appears as `checks.<system>.pages`: your own checks are not prefixed.

### A flake in a subdirectory

A `path:` flake's checks see only its directory, so a provider's check cannot read your pages. Run the
provider's tool on the working tree in your workflow instead, in place of `nix flake check`:

```
git submodule update --init --recursive
nix shell 'path:nix/agent-tools#tools' -c pagecheck docs
```

Keep `graft-check.yaml` beside it: it runs graft's check.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): prefixed checks from each `graft.nix`, `consumer`, `perSystem`, and the merge of a
  repository's own `outputs` (`lib.recursiveUpdate`).
- [lib/ci.sh](../../lib/ci.sh): `check`, which evaluates `tools` but builds no check; `upgrade`, which runs the
  whole `nix flake check` and pushes; `pr`, which titles a failing upgrade.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/): `graft-check.yaml`, whose runner label
  and checkout step 6 copies, and `graft-upgrade.yaml`, which pushes with the job token and opens the pull
  request with `GRAFT_PR_TOKEN`.
- [tests/run.sh](../../tests/run.sh): the `gate:` checks, a provider check reading a settings file from `consumer`,
  and a consumer's own `outputs` overriding graft's.
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
