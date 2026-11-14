---
status: working
---
# Graft your first pattern

Graft one provider into a new or existing repository, get its tools on your `PATH`, point your agents at it,
and end with `graft check` passing and everything committed.

## Before you begin

- The example grafts [knowledge-base-kit](https://github.com/danielnaab/knowledge-base-kit). The terms
  provider, scion and pin are in [Concepts](../concepts.md).
- Nix 2.27 or later, with flakes: `nix --version` prints the version, and
  `nix config show experimental-features` must list `flakes` and `nix-command`. If it does not, add
  `experimental-features = nix-command flakes` to `~/.config/nix/nix.conf`.
- A git repository. For a new one: `mkdir team-kb && cd team-kb && git init`.
- `git check-ignore -v flake.lock` must print nothing. If it prints a rule, remove that rule: `flake.lock`
  pins graft.
- Commit or stash other work, so that `git status` shows only what graft adds.
- In zsh with `extendedglob` set, `.#graft` is a glob (`no matches found`): quote it, `nix run '.#graft'`
  ([The graft command](../reference/commands.md#the-graft-command)).
- If your repository has private files that must not enter the world-readable Nix store, read
  [Keep private files out of the Nix store](flake-in-a-subdirectory.md) instead.

## Steps

1. At the top of the repository, write graft's template:

   ```
   nix flake init -t github:danielnaab/graft/v2.13.0
   ```

   It writes `flake.nix` and graft's two CI workflows in `.forgejo/workflows/`, and marks them for git
   (`git status --short` shows ` A`). The `flake.nix` it writes is the whole flake:

   ```nix
   {
     description = "What this repo is";
     inputs.self.submodules = true;
     inputs.graft.url = "github:danielnaab/graft/v2.13.0";
     outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
   }
   ```

   If the repository already has a `flake.nix`, it writes the workflows but not `flake.nix`
   (`refusing to overwrite existing file`), and exits 1. Then make three changes to your own flake, and keep
   everything else:

   - Add `inputs.self.submodules = true;` and `inputs.graft.url = "github:danielnaab/graft/v2.13.0";`
     (inside an `inputs = { … };` block, drop the `inputs.` prefix).
   - Name the outputs function's argument `inputs`, with `...`: `outputs = { self, nixpkgs, ... }@inputs:`.
   - Pass what the function returned as `mkFlake`'s `outputs`:

     ```nix
     outputs = { self, nixpkgs, ... }@inputs: inputs.graft.lib.mkFlake {
       inherit inputs;
       outputs = YOUR_OLD_RESULT;   # an attribute set, `let … in { … }`, or flake-utils' eachDefaultSystem (…)
     };
     ```

     If you have an `inputs.nixpkgs`, every provider builds its tools with it.

2. In a new repository, set `description`.

3. Graft the provider:

   ```
   nix run .#graft add https://github.com/danielnaab/knowledge-base-kit
   ```

   The first run fetches graft, which can take a minute. Nix warns that the git tree is dirty (expected until
   step 8) and prints `warning: creating lock file`. graft adds a submodule at the newest release, and
   prints:

   ```
   graft: scions/knowledge-base-kit <- v2.2.0 (abc1234)
   ```

   It wrote `scions/README.md`, and staged `.gitmodules`, the submodule and the README. To graft a version
   other than the newest, add it: `…/knowledge-base-kit@v2.1.0`.

4. Look at what arrived:

   ```
   cat .gitmodules
   ls scions/knowledge-base-kit
   ```

   `.gitmodules` names the path and the URL. `scions/knowledge-base-kit/` is the provider's whole repository,
   checked out at the tag. `scions/README.md` says what the folder is.

5. Use what the provider ships. If it has a `graft.nix`, its tools are in `packages.<system>.tools`:

   ```
   nix shell .#tools
   kb-lint --help
   ```

   `nix develop` gives a shell with the same tools, and direnv's `use flake` does it on `cd`. Its checks
   run in `nix flake check` (step 9). The provider's README says what each tool does, and which settings file
   it reads, such as `knowledge-base.yaml` at your repository's top.

6. Point your agents at the pattern. Add this section to `AGENTS.md`, or create it:

   ```markdown
   ## Scions (graft)
   `scions/` holds git submodules: other repositories at release tags. Read them; they are the rules and tools
   this repository follows. If one is empty, run `git submodule update --init`.
   - Fix a provider bug here only as a provider change: in `scions/NAME`, `git switch -c fix-…`, edit, run the
     provider's own tests there, push the branch to the provider and open its pull request. This repository takes
     the fix with `nix run .#graft upgrade NAME` once it is released.
   - Nix builds of this repository see a scion edit only while another tracked file here is modified, and only
     files that are `git add`-ed inside the scion. A scion checked out away from its gitlink (`git submodule
     status` shows `+`) changes what Nix builds the same way. Test a provider fix with the provider's own tests.
   - `nix run .#graft check` fails while a scion has uncommitted edits, is away from its gitlink, or sits on an
     unreleased commit. Don't commit a moved scion unless it is at a release tag.
   ```

   For a flake in a subdirectory, write its command, `nix run 'path:DIR#graft'`, for `nix run .#graft`.

   Then add a line that says what to follow, for example
   `Follow the policies in scions/knowledge-base-kit/docs/policies/.` If your agent tool denies edits to
   `scions/`, change that to ask: a scion edit is a provider change that a person should see
   ([Fix a provider from a consumer](fix-a-provider.md)).

7. If you use `just`, import a provider's recipes with `import?`, so that a clone with empty scions still
   lists your own recipes:

   ```just
   import? "scions/notebook-kit/tools/kit.just"
   ```

8. On GitHub, delete `.forgejo/`: those workflows are for Forgejo Actions
   ([Run graft in CI and review its upgrade pull requests](ci.md)). On Forgejo, keep it. Then commit:

   ```
   git add -A
   git commit -m "Graft knowledge-base-kit v2.2.0"
   ```

9. Check it:

   ```
   nix run .#graft check
   nix flake check
   ```

   `graft check` prints `graft: 1 scions clean and at release tags`, and exits 0. `nix flake check` builds
   the provider's checks, named `checks.<system>.knowledge-base-kit-NAME`, beside your own. To require them
   before a merge, see [Make a pattern's checks gate your merges](gate-merges.md).

Clones of the repository get empty `scions/` folders until someone runs `git submodule update --init`. Builds
still work while the clone's tracked files are unmodified. To fill them on every clone and worktree, see
[Clones, worktrees, sandboxes and CI](clones-and-worktrees.md).

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scion is registered, clean and at a release,
everything is committed, and `nix flake check` passes:

```sh
nix run '.#graft' check && git ls-files --error-unmatch flake.nix flake.lock .gitmodules scions/README.md >/dev/null && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## If something goes wrong

- **`error: path '…/flake.nix' does not exist` or `… is not tracked by Git`.** Nix reads only files that git
  knows. Run `git add -N flake.nix`, then step 3 again.
- **`graft: .gitmodules registers scions/knowledge-base-kit but the flake's source lacks it: add inputs.self.submodules = true; …`.**
  Your `flake.nix` lacks the line, or Nix is older than 2.27. Add it as in step 1.
- **`graft: scions/knowledge-base-kit exists`.** The repository already has a folder of that name. Give
  the scion another name, as the last argument of `add`, or another directory: `--scions .scions` (and
  `dir = ".scions";` in `mkFlake`).
- **`graft: https://… has no semver tags`.** The repository has no release yet. Ask its maintainers to tag
  one ([Publish a pattern](publish.md)).

## How it works

- `add` runs `git submodule add`, then checks out the tag's commit. The gitlink is the pin; there is no other
  lock.
- mkFlake finds scions by folder: every submodule in `scions/` is one, and one with a `graft.nix` adds tools,
  apps and checks. Your `flake.nix` never names a provider.
- To graft more providers, run `add` again. To stop grafting one: `git rm scions/NAME`, then commit; git
  removes its `.gitmodules` entry. `nix flake check` then fails if another provider `requires` it.
- To add tools of your own to `tools` (a compiler your scripts need, say), use `mkFlake`'s `perSystem`
  ([Commands](../reference/commands.md#mkflake)).
- Commands and flags: [Commands](../reference/commands.md).

## Try a provider's unreleased change

Moved to [Fix a provider from a consumer](fix-a-provider.md).

## Sources

- [template/](../../template/): the `flake.nix` and workflows that step 1 writes.
- [lib/tools.sh](../../lib/tools.sh): `add`.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): discovery, `tools`, prefixed checks, and the error without
  `inputs.self.submodules`.
- [tests/run.sh](../../tests/run.sh): `add` and the first-pattern checks.
- [tests/fixtures.sh](../../tests/fixtures.sh) `first-pattern`: the repositories this page was read against.
- [Commands](../reference/commands.md), [Files](../reference/files.md).
