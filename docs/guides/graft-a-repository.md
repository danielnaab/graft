---
status: working
---
# Graft your first pattern

Graft one provider's pattern into a new or existing repository, and end with its check passing and everything
committed.

## Before you begin

- The example grafts [knowledge-base-kit](https://github.com/danielnaab/knowledge-base-kit). The terms
  provider, pin and scion are in [Concepts](../concepts.md).
- Nix with flakes: `nix config show experimental-features` must list `flakes` and `nix-command`. If it does
  not, add `experimental-features = nix-command flakes` to `~/.config/nix/nix.conf`.
- A git repository. For a new one: `mkdir team-kb && cd team-kb && git init`.
- `git check-ignore -v flake.lock` must print nothing. If it prints a rule, remove that rule and commit
  `.gitignore`: `flake.lock` holds the pins, and git would leave it out without a word.
- Commit or stash other work, so that `git status` shows only what graft adds.
- In zsh with `extendedglob` set, `.#graft` is a glob (`no matches found`): quote it, `nix run '.#graft'`.

## Steps

1. At the top of the repository, write graft's template:

   ```
   nix flake init -t github:danielnaab/graft
   ```

   It writes `flake.nix` and graft's two CI workflows in `.forgejo/workflows/`, marks them for git
   (`git add --intent-to-add`: `git status --short` shows ` A`), and prints
   `Edit description and the inputs in flake.nix …`.

   If the repository already has a `flake.nix`, it writes the workflows but not `flake.nix`
   (`refusing to overwrite existing file`), ends with `error: Encountered 1 conflicts - see above`, and exits
   1. That is expected: step 2 edits your own `flake.nix`.

2. Edit `flake.nix`. In a new repository, set `description`. The template already pins knowledge-base-kit
   and graft. To graft another provider, add an input pinned to a tag, with `flake = false`:

   ```nix
   knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
   ```

   In an existing flake, make three changes, and keep everything else:

   - Add graft and the provider to your inputs, in the form your flake already uses:

     ```nix
     inputs.graft.url = "github:danielnaab/graft/v2.11.1";
     inputs.knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
     ```

     Inside an `inputs = { … };` block, drop the `inputs.` prefix.
   - Add `graft` and `...` to your outputs function's arguments, and name it `inputs`. Keep its other
     arguments as they are: naming an input that the flake does not have makes Nix add one.
   - Pass what the function returned as `mkFlake`'s `outputs`. Any expression that gives an attribute set
     works:

   ```nix
   outputs = { self, graft, ... }@inputs: graft.lib.mkFlake {
     inherit inputs;
     outputs = YOUR_OLD_RESULT;   # the attribute set, `let … in { … }`, or flake-utils' eachDefaultSystem (…)
   };
   ```

   If the flake already has `flake = false` inputs that are not patterns (source code that it builds, say),
   add `scions = [ "knowledge-base-kit" ];` beside `inherit inputs;`: otherwise graft copies those too.
   `apps.<system>.graft` and `checks.<system>.graft` are graft's names: rename any of your own.

3. Graft:

   ```
   nix run .#graft
   ```

   The first run fetches graft and the provider, which can take a minute. Nix warns that the git tree is
   dirty (it has uncommitted changes, which is expected until step 8), prints `warning: creating lock file`
   (or `updating lock file`, if there was one), and an `• Added input` line for each new input, graft's own
   included. graft prints one line for each scion, then exits 0:

   ```
   graft: scions/knowledge-base-kit <- COMMIT
   ```

   It wrote `flake.lock`, and wrote and staged `scions/`.

4. Look at what arrived:

   ```
   cat scions/README.md
   ```

   It names each scion and its pinned commit, and says not to edit them. `scions/knowledge-base-kit/` holds
   the provider's export: its files, minus what the provider keeps to itself.

5. Check it:

   ```
   nix run .#graft check
   ```

   Expected, with exit 0:

   ```
   graft: scions/ and graft's workflows match the pins
   ```

   If it says `scions/ differs from the pins` instead, a tool in your repository changed the scions: see
   [`graft check` fails](check-fails.md).

6. Point your agents at the pattern. For example, add this line to `AGENTS.md`, or create it:

   ```
   Follow the policies in scions/knowledge-base-kit/docs/policies/. Never edit scions/: graft replaces it.
   ```

   If a grafted file does not fit your repository, see [Change a grafted file](change-a-grafted-file.md).

7. On GitHub, delete `.forgejo/`. Those workflows are for Forgejo Actions, and graft never adds a workflow
   that you did not keep ([Run graft in CI and review its upgrade pull requests](ci.md)). On Forgejo, keep it.

8. Commit:

   ```
   git add -A
   git commit -m "Graft knowledge-base-kit v2.1.0"
   ```

   Then `git ls-files flake.lock` must print `flake.lock`. If it prints nothing, see
   [If something goes wrong](#if-something-goes-wrong).

9. Run the check that CI runs:

   ```
   nix flake check
   ```

   It exits 0. It runs graft's check, `checks.<system>.graft`, beside your own checks. To run the pattern's
   checks here too, and require them before a merge, see [Make a pattern's checks gate your merges](gate-merges.md).

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions match their pins, the pins are
committed, nothing is left to commit, and `nix flake check` passes:

```sh
nix run .#graft check && git ls-files --error-unmatch flake.nix flake.lock scions/README.md >/dev/null && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## If something goes wrong

- **`experimental Nix feature 'flakes' is disabled`.** Enable flakes, as in
  [Before you begin](#before-you-begin).
- **`error: path '/nix/store/…-source/flake.nix' does not exist`.** A provider's input lacks `flake = false`
  in its attribute set, so Nix tried to read it as a flake. Nix ignores `?flake=false` in a URL. Write it as
  in step 2. (If no input at all is a scion, graft says `graft: no scions: no input has flake = false`.)
- **`git ls-files flake.lock` prints nothing, or Nix prints `• Added input` on every run.** `flake.lock` is
  not committed, usually because `.gitignore` drops it (`git check-ignore -v flake.lock` names the rule).
  Then nothing is pinned: each run resolves the tags again, and both checks still pass. Remove the rule,
  then do step 8 again.

  Not yet: no check fails when `flake.lock` is not committed.
- **`graft: scions/ exists and graft didn't write it`.** The repository already has a `scions/` of its
  own. graft never replaces it: put the scions elsewhere with `dir`, for example
  `graft.lib.mkFlake { inherit inputs; dir = ".scions"; }`, and run step 3 again. Read `.scions/` for
  `scions/` in the later steps.

## How it works

- Write `inputs` as a literal attribute set: Nix reads it without evaluation, and so does graft.
- Take one repository through one input only. If two inputs name one repository at one commit, Nix
  fetches it once, and both get the tree of the first fetch.
- To keep one copy of an input that graft also has, add a `follows`. graft uses its own
  `knowledge-base-kit` input only for its own docs, so the second line only keeps `flake.lock` to one entry:

  ```nix
  graft.inputs.nixpkgs.follows = "nixpkgs";                       # if your repository has nixpkgs
  graft.inputs.knowledge-base-kit.follows = "knowledge-base-kit"; # if it grafts knowledge-base-kit
  ```

- `mkFlake` also takes `dir`, `scions` and `flakeDir`: see [Commands](../reference/commands.md#mkflake). See also
  [Keep private files out of the Nix store](flake-in-a-subdirectory.md) and
  [Graft a private repository](private-providers.md). To take newer versions, see
  [Upgrade](upgrade.md).

## Sources

- [template/](../../template/): the `flake.nix` and workflows that step 1 writes.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `valid` (the argument checks), `names` (which inputs are
  scions), `expected`, the `sync` and `check` cases, and the merge of a repository's own `outputs`.
- [lib/assemble.sh](../../lib/assemble.sh): the scions directory and its README.
- [tests/run.sh](../../tests/run.sh): the first graft ("export grafted", "check passes on a fresh graft"),
  "graft won't replace a directory it didn't write", and the template ("template: grafts and passes the check").
- [tests/fixtures.sh](../../tests/fixtures.sh) `first-pattern`: the repositories this page was read against.
- [Commands](../reference/commands.md), [Files](../reference/files.md).
