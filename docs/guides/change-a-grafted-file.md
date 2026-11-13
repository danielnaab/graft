---
status: working
---
# Change a grafted file

A grafted file does not fit your repository: keep your own version of it outside the scions directory, and
point your repository at yours.

## When to use this

Never edit a scion: `nix run .#graft` replaces it, and the check fails until it does
([`graft check` fails](check-fails.md)). Choose the way that fits your change:

| You want | Do this |
|---|---|
| A different value for something that a provider's tool reads from your repository | Change the setting in your file, not the tool. The provider's README, or its tool's usage text, names the settings and the file that holds them. For example, knowledge-base-kit's checker reads the statuses that a page may have from `rules.lifecycle.statuses` in your `knowledge-base.yaml`. |
| A different rule or page for your repository only | The [Steps](#steps) below. |
| To fix a mistake that every consumer has | [Fix it in the provider](#fix-it-in-the-provider). |
| None of a provider's files | [Stop grafting a provider](#stop-grafting-a-provider). |
| Your own edits to one of graft's workflows | [Keep your own edits to a workflow](../reference/files.md#keep-your-own-edits-to-a-workflow). |

A provider cannot leave out one file for one consumer: its export rules are the same for every consumer. If
one of its files never fits, ask the provider to make it optional, or to read a setting from your
repository.

## Before you begin

- The commands assume the scions directory is `scions/` and `flake.nix` is at the repository's top, as in
  [`graft check` fails](check-fails.md#before-you-begin). The examples use a provider named `policy-kit`
  and its file `docs/status.md`: use your own names.
- Commit or stash your work outside `scions/`, so that `git status` shows only this change.

## Steps

1. Copy the file out of the scions directory, into your own tree:

   ```
   mkdir -p docs/policies
   cp scions/policy-kit/docs/status.md docs/policies/status.md
   ```

   If someone already edited the scion, this copies their edits too. Run `git status --short -- scions/`,
   and copy every other file that it lists and that you want to keep, new files included. Step 2 deletes
   them.

2. Put the scions back to their pins:

   ```
   nix run .#graft
   ```

   It prints `graft: scions/NAME <- COMMIT` for each scion, and replaces the whole scions directory.

3. Edit your copy. Say in it which file it replaces, for example
   `This page replaces scions/policy-kit/docs/status.md.` Give each file that you copied what your checks,
   and the provider's tools that read your files, require of a page in that folder, such as a `status:`
   line in its front matter.

4. Find what points at the scion's file, outside the scions directory:

   ```
   git grep -n 'policy-kit/docs' -- ':!scions'
   ```

5. Point each of them at your copy. For an agent, in `AGENTS.md`:

   ```
   Follow the policies in scions/policy-kit/docs/, except status.md: follow docs/policies/status.md instead.
   ```

   If your version changes something that a provider's tool checks, change the tool's setting in your
   repository too.

6. Check:

   ```
   git add -A
   nix run .#graft check
   nix flake check
   ```

   `graft check` prints `graft: scions/ and graft's workflows match the pins`. `nix flake check` runs
   your own checks on your copy: a failure names your file.

7. Commit:

   ```
   git commit -m "Use our own status policy"
   ```

When you upgrade, the scion's copy of the file changes and yours does not. Read what the provider changed
(`nix run .#graft changelog`, and the diff of the scion), and carry over what you want.

## Fix it in the provider

When every consumer needs the change, make it in the provider and take its next version. You need a checkout
of the provider in which you can commit, or a way to send the change to its maintainers.

1. Save an uncommitted change to a scion, new files included, outside the repository. Then put the scions
   back:

   ```
   git add -A -- scions/policy-kit/
   git diff --cached -- scions/policy-kit/ > /tmp/scion-change.patch
   nix run .#graft
   ```

   For a change that is already committed, follow [`graft check` fails](check-fails.md) steps 3 to 6
   instead.

2. In the provider's checkout, apply it with `git apply -p3 /tmp/scion-change.patch`, and `git add` each
   file of the change. Try it in your repository with
   [a provider's unreleased change](upgrade.md#try-a-providers-unreleased-change). The override takes the
   checkout's current branch, so the scion also holds the provider's other unreleased changes. graft stages
   the overridden scion: do not commit it. Step 4, or `nix run .#graft`, puts the pinned version back.
3. Commit it in the provider, add a section for the new version to its `CHANGELOG.md`, and tag it, for
   example `git tag v1.2.0`. If the provider is a remote repository, push the tag: graft reads the tags
   that the input's URL has.
4. In your repository, take the release, then review, check and commit it as in
   [Take a newer version by hand](upgrade.md#steps), steps 4 to 6:

   ```
   nix run .#graft upgrade policy-kit
   ```

   Expected:

   ```
   graft: policy-kit v1.0.0 -> v1.2.0
   graft: rebuilt scions/
   ```

### Check the release is taken

From the repository's top, paste this. It prints `ok` only if `policy-kit` is pinned to its newest tag, the
scions match their pins, nothing is left to commit, and your checks pass. It needs `jq`:

```sh
nix run .#graft -- outdated --json | jq -e '.[] | select(.name == "policy-kit") | .status == "current"' >/dev/null && nix run .#graft check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## Stop grafting a provider

graft grafts a provider whole, or not at all. To keep some of its files, copy them into your own tree first,
as in [Steps](#steps) 1 and 3 to 5: from then on they are yours, and get none of the provider's changes.

1. Delete the provider's input from `flake.nix`. If your flake's own code still reads it
   (`${inputs.glossary}`), keep the input, and name the inputs that graft copies instead. Add this line
   beside `inherit inputs;` in your `mkFlake` call:

   ```nix
   scions = [ "policy-kit" ];
   ```

   With `scions`, graft copies only the inputs that it names: add each new provider to the list.

2. Run `nix run .#graft`. If you deleted the input, Nix prints `• Removed input 'glossary'`. graft prints a
   line for each scion that remains, and deletes and stages `scions/glossary/`.
3. Run `git add -A && nix flake check`, then commit.

### Check the provider is gone

From the repository's top, paste this. It prints `ok` only if `glossary` has no scion and no row in the
scions README, the other scions match their pins, nothing is left to commit, and your checks pass:

```sh
test ! -e scions/glossary && ! grep -qF '[glossary]' scions/README.md && nix run .#graft check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

To own files and still merge in a provider's later changes, use a template tool, such as Copier, instead of
a scion ([README](../../README.md#compared-with-copying-submodules-templates-and-bots)).

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions and graft's workflows match their
pins, nothing is left to commit, and your checks pass:

```sh
nix run .#graft check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## If something goes wrong

- **`nix flake check` fails on your copy, for example `no status: docs/policies/status.md`.** A provider's
  checker reads your files too. Give your copy what the checker asks for (step 3), or change the checker's
  setting in your repository (step 5).
- **`nix flake check` fails with `error: attribute 'glossary' missing`.** You deleted an input that your
  flake's own code still reads. Put the input back, and use `scions = [ … ]` instead.
- **The scions README shows a commit that is not the pin, often ending in `-dirty`, and the check fails.**
  A run with `--override-input` left its scion. Run `nix run .#graft` to go back to the pin.
- **`error: graft: no scions: no input has flake = false, and scions names none`.** You stopped grafting the
  last provider. If you need no scion at all, remove graft from the repository: its input and the
  `mkFlake` call in `flake.nix`, `scions/`, and the workflows in `.forgejo/workflows/` that carry graft's
  marker line.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the `scions` argument (`names`), the `sync` case, which
  replaces the whole scions directory, and the commit that an overridden input records (`pin`).
- [lib/tools.sh](../../lib/tools.sh): `upgrade`.
- [tests/run.sh](../../tests/run.sh): the `scions = [ "a" "b" ]` consumers.
- [tests/fixtures.sh](../../tests/fixtures.sh) `override`: the repositories this page was read against.
- [Try a provider's unreleased change](upgrade.md#try-a-providers-unreleased-change), [Commands](../reference/commands.md#mkflake).
