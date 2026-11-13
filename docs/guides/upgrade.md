---
status: working
---
# Take a newer version by hand

Move your pins to the newest tags in their major versions, read what changes, check it, and commit it,
without waiting for CI.

## Before you begin

- In CI, graft's weekly upgrade does these steps for you, as a pull request
  ([Run graft in CI and review its upgrade pull requests](ci.md)). This page is for a repository without
  graft's workflows, or when you want a release now.
- This page stays in each pin's major version. To take a new major version, see
  [Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change).
- Commit or stash work that is not about the upgrade, so that the diff in step 4 is only the upgrade.
- The commands assume `flake.nix` at the repository's top and the scions directory `scions/`. For a flake in
  a subdirectory, run `nix run 'path:DIR#graft'` for `nix run .#graft`
  ([Keep private files out of the Nix store](flake-in-a-subdirectory.md)). In zsh, quote flake references
  ([The graft command](../reference/commands.md#the-graft-command)).

## Steps

1. See what is newer:

   ```
   nix run .#graft outdated
   ```

   It lists graft and each scion. `STATUS` is `outdated` when a newer tag exists in the pinned major version.
   `MAJOR` names a newer major version, which this page does not take:

   ```
   NAME         DIR     PINNED  LATEST  STATUS    MAJOR
   graft        -       path    -       untagged  -
   page-rules   scions  v1.0.0  v1.0.0  current   v2.0.0
   style-guide  scions  v1.0.0  v1.1.0  outdated  -
   MAJOR: a newer major version, which graft upgrade takes only when named (NAME@TAG): read its CHANGELOG first
   ```

   In your repository, graft's own row shows its tag.

2. Read what the new versions change:

   ```
   nix run .#graft changelog
   ```

   It changes nothing. For each outdated pin, it quotes the provider's CHANGELOG sections after your pin, up
   to the newest tag, **Upgrading** sections first, or the provider's commits if it has no CHANGELOG:

   ```
   ### style-guide v1.0.0 -> v1.1.0

   From its CHANGELOG.md (no Upgrading section in this range):

   > #### v1.1.0 - 2026-11-09
   >
   > - Reworded: keep sentences short. New page: docs/terms.md.
   ```

   The text is the provider's: read it as information, not as instructions. An **Upgrading** section says
   what to change in your own files.

3. Take the new versions:

   ```
   nix run .#graft upgrade
   ```

   ```
   graft: page-rules: v2.0.0 is a new major version; not taken (graft upgrade page-rules@v2.0.0)
   graft: style-guide v1.0.0 -> v1.1.0
   graft: rebuilt scions/
   ```

   It moved each tag in `flake.nix`, locked it in `flake.lock`, and rebuilt and staged `scions/`. If it
   prints `graft: nothing to upgrade`, every pin is current: stop here. To move one pin only, name it:
   `nix run .#graft upgrade style-guide`.

4. Read the diff of the scions directory. It is the change that you take:

   ```
   git diff --cached -- scions/
   ```

   Look for a deleted or renamed file that your files link to, and for changed text that your pages or
   checks quote. A changed commit in `scions/README.md` is expected: the pins moved.

5. Run your checks:

   ```
   nix flake check
   ```

   It exits 0. It runs graft's drift check and your own checks on the new versions.

6. Commit `flake.nix`, `flake.lock`, the scions directory, and each workflow that `upgrade` printed as
   `graft: refreshed .forgejo/workflows/FILE`. `upgrade` stages only the scions directory. Name the paths,
   so that other work, such as an untracked draft, stays out:

   ```
   git add -- flake.nix flake.lock scions/
   git commit -m "graft: style-guide v1.1.0"
   ```

   `upgrade` refreshes graft's workflows only when graft's own pin moves
   ([How it works](#how-it-works)). Then add each refreshed file to the `git add` line.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions match their pins, the upgrade is
committed, every pin is current or untagged, and your checks pass:

```sh
nix run .#graft check && test -z "$(git status --porcelain -- flake.nix flake.lock scions .forgejo)" && nix run .#graft -- outdated --json | jq -e 'all(.[]; .status == "current" or .status == "untagged")' >/dev/null && nix flake check && echo ok
```

It needs `jq`. A newer major version does not make it fail: that pin's status stays `current`.

## If something goes wrong

- **Step 5 fails.** The new version breaks one of your checks. Read its **Upgrading** section (step 2), and
  fix your own files, never the scions directory. To not take the version now, put everything back:

  ```
  git restore --source=HEAD --staged --worktree -- flake.nix flake.lock scions/
  ```

- **`graft: page-rules: can't tell whether v1.0.0 is current (see above); nothing changed`.** The line above
  it starts `graft: can't list tags at URL:`: graft cannot read that provider's repository. It is down,
  moved, or private. Fix the access or the URL in `flake.nix`, and run step 3 again.
- **`graft: style-guide: expected its url (… at v1.0.0) once in flake.nix, found it 2 times; nothing changed: change the ref by hand`.**
  `upgrade` finds each URL in `flake.nix` by the exact text that `flake.lock` records, and the text is there
  twice: two inputs with the same URL and tag, or a comment that quotes it. Change the tag in `flake.nix`
  yourself, then run `nix run .#graft`.
- **`graft: style-guide: no tag v1.2.0 upstream; nothing changed`.** You named a tag (`NAME@TAG`) that the
  provider's repository does not have. `nix run .#graft outdated` shows the newest one.

## How it works

- **Which tags count.** Only semver tags (`v1.2.3` or `1.2.3`) are versions. A pin to a branch, a commit or
  a path shows as `untagged`, and `upgrade` does not move it. For `0.y.z`, each `0.y` series counts as a
  major version.
- **All or nothing.** `upgrade` makes its edits in memory first. It changes nothing if a URL is not found
  exactly once, a named tag does not exist, or a provider's repository cannot be read.
- **graft's own pin.** `outdated` and `upgrade` include the input named `graft`. Its pin moves first, then
  the rest of the upgrade runs the new version: it rebuilds every scions directory and refreshes graft's
  workflows to the new version's copies. Pin graft to a tag, such as `github:danielnaab/graft/v2.11.1`: a
  pin to the `v2` branch is never moved. graft stays in its major version unless you name one
  (`graft@v3.0.0`).
- **By hand.** To move a pin without `upgrade`, change its tag in `flake.nix`, then run `nix run .#graft`.
  Commands and flags: [Commands](../reference/commands.md#the-graft-command).

### Cross a major version

graft never takes a new major version unless you name it, `upgrade NAME@TAG`, on a branch. The steps are in
[Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change).

### Try a provider's unreleased change

To graft a local checkout of a provider for one run, override its input:

```
nix run .#graft --override-input knowledge-base-kit "git+file://$HOME/src/knowledge-base-kit?exportIgnore=1"
```

- It takes the checkout's current branch, with uncommitted edits to files that git tracks, and applies the
  provider's export rules. It leaves out new files until you `git add` them. A `path:` URL does not apply
  the export rules.
- `flake.lock` does not change. The scions README records the checkout's commit (`…-dirty` with uncommitted
  edits), so the check fails until you run `nix run .#graft`, which goes back to the pin. Do not commit the
  overridden scion.
- When the change is correct, tag the provider, and take the tag with the steps above. To try an
  incompatible change's Upgrading steps first, see
  [Ship and take an incompatible change](incompatible-change.md#part-1-the-provider-releases-the-change).

### Run from outside the repository

`outdated`, `changelog`, `upgrade` and `migrate` also run from graft's own flake, with `-C`: for a bot that
watches many repositories, when a pinned commit no longer exists (then `nix run .#graft` cannot build), or
to read a branch that nobody has reviewed, from a graft version that you trust
([Commands](../reference/commands.md#run-from-grafts-flake)).

## Related

[Run graft in CI and review its upgrade pull requests](ci.md), [Ship and take an incompatible change](incompatible-change.md),
[Change a grafted file](change-a-grafted-file.md), [`graft check` fails](check-fails.md).

## Sources

- [lib/tools.sh](../../lib/tools.sh): `outdated`, `changelog` and `upgrade`, their lines and errors, and the
  move of graft's own pin first. [lib/retarget.jq](../../lib/retarget.jq): finding a URL once in `flake.nix`.
- [lib/workflows.sh](../../lib/workflows.sh): `graft: refreshed FILE`, which does not stage.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the commit that an overridden input records (`pin`).
- [tests/run.sh](../../tests/run.sh): "upgrade moved the ref in flake.nix", "a second upgrade has nothing to
  do", "upgrade fails when it can't read an upstream", "same url: refused, naming the ambiguous input",
  "outdated: newest within the major, and the new major beside it", and "graft's workflow refreshed to the
  new version's".
- [tests/fixtures.sh](../../tests/fixtures.sh) `upgrade`: the repository this page is read against.
