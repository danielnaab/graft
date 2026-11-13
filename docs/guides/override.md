---
status: draft
---
# Change a file that a scion provides

A grafted file does not fit your repository, or has a mistake: change what your repository uses, and never
edit the scion.

## When to use this

graft replaces a scion on every `nix run .#graft`, and its check fails while a scion differs from its pin
([`graft check` fails](check-fails.md)). So an edit to a scion never lasts. Choose one of these instead:

| You want | Do this | Later versions of the file |
|---|---|---|
| A fix that every consumer needs: a mistake, a missing case | [Fix it in the provider](#fix-it-in-the-provider) | You get them, your fix included |
| A different rule for your repository only | [Keep your own version](#keep-your-own-version) | You get them in the scion, and carry over what you want |
| The file, and none of the provider's later changes to it | [Stop grafting it](#stop-grafting-it) | You do not get them |
| Your own edits to one of graft's workflows | [Keep your own workflow](#keep-your-own-workflow) | You do not get them for that workflow |

A provider cannot leave out one file for one consumer: its export rules are the same for every consumer. If
a file never fits, ask the provider to make it optional or to read a setting from your repository.

## Before you begin

- The commands assume the scions directory is `scions/` and `flake.nix` is at the repository's top, as in
  [`graft check` fails](check-fails.md#before-you-begin). The examples use a provider named `policy-kit`:
  use your scion's name.
- Commit or stash other work, so that `git status` shows only this change.

## Steps

### Fix it in the provider

You need a checkout of the provider in which you can commit, here `~/src/policy-kit`. If you do not maintain
the provider, send it the change instead of step 5, and wait for its release.

1. If you already edited the scion, save the change outside the repository, then put the scion back:

   ```
   git diff -- scions/policy-kit/ > /tmp/scion-change.patch
   nix run .#graft
   ```

   `nix run .#graft` prints `graft: scions/NAME <- COMMIT` for each scion. For an edit that is already
   committed, follow [`graft check` fails](check-fails.md) steps 3 to 6 instead.

2. Make the change in the provider's checkout. To apply a saved change there:

   ```
   git apply -p3 /tmp/scion-change.patch
   ```

   Add one to `-p` for each extra segment in your scions directory's path.

3. In your repository, graft the provider's checkout for one run:

   ```
   nix run .#graft --override-input policy-kit "git+file://$HOME/src/policy-kit?exportIgnore=1"
   ```

   Nix warns that the provider's tree is dirty, prints `warning: not writing modified lock file` and
   `• Updated input 'policy-kit'`. graft then prints `graft: scions/policy-kit <- COMMIT-dirty`. The scion
   now holds the checkout's current branch with your uncommitted edits, so it can also hold changes that the
   provider made after your pin.

4. Read what changed, and try it with your agents and tools:

   ```
   git diff --cached -- scions/policy-kit/
   ```

   Do not commit this. `flake.lock` did not change, so the check fails while the scions README records the
   commit as `…-dirty`.

5. In the provider's checkout, commit the change, add a section for the new version to its `CHANGELOG.md`,
   and tag it:

   ```
   git tag v1.2.0
   ```

6. In your repository, take the release:

   ```
   nix run .#graft upgrade policy-kit
   ```

   Expected:

   ```
   graft: policy-kit v1.0.0 -> v1.2.0
   graft: rebuilt scions/
   ```

   It changes the tag in `flake.nix`, locks it in `flake.lock`, and replaces the override's scion with the
   release's. If it prints `graft: nothing to upgrade`, the provider has no newer tag yet.

7. Check, then commit:

   ```
   nix run .#graft check
   git add -A
   git commit -m "Take policy-kit v1.2.0"
   ```

   The check prints `graft: scions/ and graft's workflows match the pins`.

### Keep your own version

Your version is an ordinary file in your repository. graft does not read it, and the scion stays as it is.

1. Write your version outside the scions directory. To start from the provider's text:

   ```
   mkdir -p docs/policies
   cp scions/policy-kit/docs/status.md docs/policies/status.md
   ```

   Edit it, and say in it which file it replaces: `This page replaces scions/policy-kit/docs/status.md.`

2. Find what points at the scion's file, outside the scions directory:

   ```
   git grep -n 'policy-kit/docs' -- ':!scions'
   ```

3. Point each of them at your version. For an agent, in `AGENTS.md`:

   ```
   Follow the policies in scions/policy-kit/docs/, except status.md: follow docs/policies/status.md instead.
   ```

   When a provider's tool reads its settings from your repository, change the setting there, not the
   tool. For example, knowledge-base-kit's checker reads the statuses that a page may have from
   `rules.lifecycle.statuses` in your `knowledge-base.yaml`.

4. Commit:

   ```
   git add -A
   git commit -m "Use our own status policy"
   ```

When you upgrade, read what the provider changed in the file that you replaced
(`nix run .#graft changelog`, and the diff of the scion), and carry over what you want.

### Stop grafting it

1. Copy the file into your own tree. From then on it is yours, and it gets none of the provider's changes:

   ```
   cp scions/glossary/terms.md docs/terms.md
   ```

   Then point what reads it at your copy, as in [Keep your own version](#keep-your-own-version) steps 2 and
   3.

2. If you need nothing else from the provider, delete its input from `flake.nix`. If your flake's own code
   still reads it (`${inputs.glossary}`), keep the input, and name the inputs that graft copies instead:

   ```nix
   outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; scions = [ "policy-kit" ]; };
   ```

   With `scions`, graft copies only the inputs that it names: add each new provider to the list.

3. Graft:

   ```
   nix run .#graft
   ```

   If you deleted the input, Nix prints `• Removed input 'glossary'`. graft prints a line for each scion that
   remains, and deletes and stages `scions/glossary/`.

4. Commit:

   ```
   git add -A
   git commit -m "Stop grafting glossary"
   ```

To own a set of files and still merge in a provider's later changes, use a template tool, such as Copier,
instead of a scion ([README](../../README.md#compared-with-copying-submodules-templates-and-bots)).

### Keep your own workflow

graft's workflows in `.forgejo/workflows/` are graft's while their first line is the marker
`# managed by graft: …`. To change only the `branches:`, `paths-ignore:` or `# graft ssh:` line, keep the
marker: graft keeps those lines ([Files](../reference/files.md#grafts-workflows)).

1. Delete the marker line, the first line of the file, before you run `nix run .#graft`. While the marker is
   there, `nix run .#graft` puts graft's copy back, and your edits are lost.
2. Make your edits.
3. Check:

   ```
   nix run .#graft check
   ```

   It prints `graft: scions/ and graft's workflows match the pins`. graft no longer reads the file.

4. Commit the workflow.

Without the marker, `nix run .#graft upgrade` no longer replaces the file, so you do not get graft's changes
to it. To take graft's copy again, put the marker line back and run `nix run .#graft`. It prints
`graft: refreshed .forgejo/workflows/FILE`, and your edits are gone.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions and graft's workflows match their
pins, nothing is left to commit, and the check that CI runs passes:

```sh
nix run .#graft check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## If something goes wrong

- **The check prints `graft: .forgejo/workflows/FILE differs from the copy this graft version ships`.** The
  workflow still has its marker. Delete the marker line to keep your edits, or run `nix run .#graft` to put
  graft's copy back.
- **Nix warns `the flag '--override-input NAME …' does not match any input`.** The name is not one of your
  inputs, so graft grafted the pin, and its line shows the pinned commit, not `…-dirty`. Use the input's
  name from `flake.nix`.
- **The check fails after an override, and the scions README shows `…-dirty`.** The override's scion is
  still there. Run `nix run .#graft` to go back to the pin, or take the release as in step 6.
- **`error: graft: no scions: no input has flake = false, and scions names none`.** You removed the last
  scion. graft needs at least one: if you need none, remove `graft` from the flake too.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the `scions` argument (`names`), the `sync` case, which
  replaces the whole scions directory, and the `-dirty` commit of an overridden input (`pin`).
- [lib/workflows.sh](../../lib/workflows.sh): the marker, and the lines graft keeps.
- [lib/tools.sh](../../lib/tools.sh): `upgrade`.
- [tests/run.sh](../../tests/run.sh): "check catches an edited graft workflow", "a workflow without the marker
  is the repo's: check passes", "a workflow without the marker untouched", and the `scions = [ "a" "b" ]`
  consumers.
- [tests/fixtures.sh](../../tests/fixtures.sh) `override`: the repositories this page was read against.
- [Upgrade](upgrade.md#try-a-providers-unreleased-change), [Commands](../reference/commands.md#mkflake),
  [Files](../reference/files.md#grafts-workflows).
