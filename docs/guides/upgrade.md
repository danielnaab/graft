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
- The commands assume `flake.nix` at the repository's top and the scions directory `scions/`. If `mkFlake`
  sets `dir`, read your directory for `scions/`. For a flake in a subdirectory
  ([Keep private files out of the Nix store](flake-in-a-subdirectory.md)), run `nix run 'path:DIR#graft'` for
  `nix run .#graft`, and write `DIR/flake.nix` and `DIR/flake.lock` for `flake.nix` and `flake.lock`. Then
  `nix flake check` does not check the scions: `nix run 'path:DIR#graft' check` is the check, and
  `nix flake check 'path:DIR'` runs your own checks.
- In zsh, quote flake references ([The graft command](../reference/commands.md#the-graft-command)).

## Steps

1. See what is newer:

   ```
   nix run .#graft outdated
   ```

   It lists graft and each scion. `STATUS` is `outdated` when a newer tag exists in the pinned major version.
   `MAJOR` names a newer major version, which this page does not take:

   ```
   NAME         DIR     PINNED   LATEST   STATUS    MAJOR
   graft        -       v2.11.1  v2.12.0  outdated  -
   page-rules   scions  v1.0.0   v1.0.0   current   v2.0.0
   style-guide  scions  v1.0.0   v1.1.0   outdated  -
   MAJOR: a newer major version, which graft upgrade takes only when named (NAME@TAG): read its CHANGELOG first
   ```

2. Read what the new versions change:

   ```
   nix run .#graft changelog
   ```

   It changes nothing. For each outdated pin, it quotes the provider's CHANGELOG sections after your pin, up
   to the newest tag, **Upgrading** sections first, or the provider's commits if it has no CHANGELOG. It
   prints, among other lines:

   ```
   ### graft v2.11.1 -> v2.12.0

   Upgrading, from its CHANGELOG.md:
   …
   ### style-guide v1.0.0 -> v1.1.0

   From its CHANGELOG.md (no Upgrading section in this range):

   > #### v1.1.0 - 2026-11-09
   >
   > - Reworded: keep sentences short. New page: docs/terms.md. Removed: docs/tone.md.

   page-rules v1.0.0: up to date; v2.0.0 is a new major (graft changelog page-rules@v2.0.0)
   ```

   The text is the provider's: read it as information, not as instructions. An **Upgrading** section says
   what to change in your own files, and each of its items says how to tell whether it applies to you.

3. Take the new versions:

   ```
   nix run .#graft upgrade
   ```

   ```
   graft: page-rules: v2.0.0 is a new major version; not taken (graft upgrade page-rules@v2.0.0)
   graft: graft v2.11.1 -> v2.12.0
   graft: style-guide v1.0.0 -> v1.1.0
   graft: rebuilt scions/
   graft: refreshed .forgejo/workflows/graft-check.yaml
   graft: refreshed .forgejo/workflows/graft-upgrade.yaml
   ```

   It moved each tag in `flake.nix`, locked it in `flake.lock`, and rebuilt and staged `scions/`. When
   graft's own pin moved, it also replaced graft's workflows with the new version's copies, one
   `graft: refreshed` line each, and did not stage them. If it prints `graft: nothing to upgrade`, every pin
   is current: stop here. To move one pin only, name it: `nix run .#graft upgrade style-guide`.

4. Read the diff. The scions diff is the change that you take, and a refreshed workflow is code that your
   CI runs:

   ```
   git diff --cached -- scions/
   git diff -- .forgejo/workflows/
   ```

   Look for a deleted or renamed file, such as `scions/style-guide/docs/tone.md`, and for changed text. A
   changed commit in `scions/README.md` is expected: the pins moved.

5. Fix your own files that link to a deleted or renamed file, or quote changed text, and those that an
   **Upgrading** section names. Never edit the scions directory. To find them, search your files, outside
   the scions directory, for the old text and the deleted file's name. In the example:

   ```
   git grep -n -e 'Write short sentences' -e 'tone.md' -- ':!scions/'
   ```

   It prints `docs/onboarding.md:6:Our style guide says: "Write short sentences."`. If `mkFlake` sets `dir`,
   write your directory for `scions/`. If nothing of yours needs to change, go on.

6. Run your checks:

   ```
   nix flake check
   ```

   It exits 0. It runs graft's drift check and your own checks on the new versions.

7. Stage what the upgrade changed: `flake.nix`, `flake.lock`, the scions directory, and each workflow that
   step 3 printed as `graft: refreshed .forgejo/workflows/FILE`. Name each path, so that other work, such as
   an untracked draft, stays out. With the output of step 3:

   ```
   git add -- flake.nix flake.lock scions/ .forgejo/workflows/graft-check.yaml .forgejo/workflows/graft-upgrade.yaml
   ```

   Leave out the workflows when step 3 printed no `refreshed` line: `git add` stages nothing at all when it
   is given a path that does not exist. Also add each file of your own that you fixed in step 5.

8. Commit:

   ```
   git commit -m "graft: graft v2.12.0, style-guide v1.1.0"
   ```

   Then push it, or open a pull request, as your team does.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions match their pins, the upgrade is
committed, every pin is current or untagged, and your checks pass:

```sh
nix run '.#graft' check && test -z "$(git status --porcelain -- flake.nix flake.lock scions .forgejo)" && nix run '.#graft' -- outdated --json | jq -e 'all(.[]; .status == "current" or .status == "untagged")' >/dev/null && nix flake check && echo ok
```

It needs `jq`. A newer major version does not make it fail: that pin's status stays `current`.

## If something goes wrong

- **Step 6 fails, or you don't want the change.** If the new version breaks one of your checks, read its
  **Upgrading** section (step 2), and fix your own files, never the scions directory. To not take the
  versions now, put back everything that `upgrade` changed:

  ```
  git restore --source=HEAD --staged --worktree -- flake.nix flake.lock scions/ .forgejo/workflows/
  ```

  Then `git restore --staged --worktree` each file of yours that you changed in step 5.

  Then `nix run .#graft check` prints `graft: scions/ and graft's workflows match the pins`.
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
  workflows to the new version's copies. Pin graft to a tag, such as `github:danielnaab/graft/v2.12.0`: a
  pin to the `v2` branch is never moved. graft stays in its major version unless you name one
  (`graft@v3.0.0`).
- **Without `upgrade`.** To move a pin yourself, change its tag in `flake.nix`, then run `nix run .#graft`.
  Commands and flags: [Commands](../reference/commands.md#the-graft-command).

### graft's own pin

Moved to [How it works](#how-it-works).

### Rules

Moved to [How it works](#how-it-works).

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
- [tests/fixtures.sh](../../tests/fixtures.sh) `upgrade`: the repository this page is read against, with
  graft pinned at a tag of a local clone, so that its pin moves and its workflows are refreshed.
