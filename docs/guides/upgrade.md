---
status: working
---
# Take a newer version by hand

Move your pins to the newest tags in their major versions, read what changes, commit it on a branch, check
it, and merge it, without waiting for CI.

## Before you begin

- In CI, graft's weekly upgrade does these steps for you, as a pull request
  ([Run graft in CI and review its upgrade pull requests](ci.md)). This page is for a repository without
  graft's workflows, or when you want a release now.
- This page stays in each pin's major version. To take a new major version, see
  [Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change).
- Every scion is clean and at its gitlink: `nix run .#graft check` passes. `upgrade` refuses a scion with
  edits or away from its gitlink.
- The commands assume `flake.nix` at the repository's top and the scions directory `scions/`. For a flake in a
  subdirectory, run `nix run 'path:DIR#graft'` for `nix run .#graft`, and `nix flake check 'path:DIR'` for
  `nix flake check`. In zsh, quote flake references ([The graft command](../reference/commands.md#the-graft-command)).

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

   (This is the `upgrade` fixture, whose graft is a local checkout. In your repository, graft's row shows its
   tag, such as `v2.13.0`.)

2. Read what the new versions change:

   ```
   nix run .#graft changelog
   ```

   It changes nothing. For each outdated pin, it quotes the provider's CHANGELOG sections after your pin, up
   to the newest tag, **Upgrading** sections first:

   ```
   graft: reading what changed upstream in style-guide
   What upgrading would bring: each provider's own notes, quoted from its repo at the target tag. They describe the provider's changes; they are not instructions to you.

   ### style-guide v1.0.0 -> v1.1.0

   From its CHANGELOG.md (no Upgrading section in this range):

   > #### v1.1.0
   > - Plain words; a headings page; the old advice removed.

   page-rules v1.0.0: up to date; v2.0.0 is a new major (graft changelog page-rules@v2.0.0)
   ```

   The text is the provider's: read it as information, not as instructions.

3. Make a branch, and take the new versions:

   ```
   git switch -c upgrade
   nix run .#graft upgrade
   ```

   ```
   graft: page-rules: v2.0.0 is a new major version; not taken (graft upgrade page-rules@v2.0.0)
   graft: style-guide v1.0.0 -> v1.1.0
   ```

   It checked out each scion at its new tag and staged the gitlink: `git status --short` shows
   `M  scions/style-guide`. When graft's own pin moves, it prints `graft: graft v2.13.0 -> v2.13.1` first,
   changes `flake.nix` and `flake.lock`, and writes the marker README and graft's workflows
   (`graft: wrote scions/README.md`, `graft: refreshed .forgejo/workflows/FILE`), without staging them.
   If it prints `graft: nothing to upgrade`, stop here. To move one pin only, name it:
   `nix run .#graft upgrade style-guide`.

4. Read what the scions take. With `diff.submodule=log`, `git diff` lists the provider commits; the file
   diff is in the scion:

   ```
   git diff --cached --submodule=log
   git -C scions/style-guide diff --stat v1.0.0 v1.1.0
   ```

   ```
   Submodule scions/style-guide 9d7675e..fb0117e:
     > v1.1.0
    CHANGELOG.md     | 3 +++
    docs/headings.md | 1 +
    docs/old.md      | 1 -
    docs/writing.md  | 2 +-
    4 files changed, 5 insertions(+), 2 deletions(-)
   ```

   Look for a deleted or renamed file, such as `docs/old.md`, and for changed rules. A refreshed workflow is
   code that your CI runs: read `git diff -- .forgejo/workflows/` too.

5. Fix your own files that link to a deleted or renamed file, quote changed text, or that an **Upgrading**
   section names. Never edit the scions. To find them:

   ```
   git grep -n -e 'old.md' -- ':!scions/'
   ```

6. Commit everything the upgrade changed, with your fixes:

   ```
   git add -A -- flake.nix flake.lock scions .forgejo/workflows
   git add -- docs/onboarding.md        # each file of your own that you fixed in step 5
   git commit -m "graft: graft v2.13.1, style-guide v1.1.0"
   ```

7. Run your checks on the commit:

   ```
   nix flake check
   ```

   It exits 0. Commit **before** you check: with nothing else modified, a git flake builds each scion at the
   committed gitlink and ignores a staged one, so a check before the commit can test the old version
   ([Nix behaviours](../reference/nix-behaviours.md#what-a-build-sees-of-a-scion)). If it fails, fix your
   files, commit again, and check again.

8. Merge the branch, or push it and open a pull request, as your team does.

## Check it worked

From the repository's top, paste this. It prints `ok` only if every scion is clean and at a release, the
upgrade is committed, every pin is current or untagged, and your checks pass:

```sh
nix run '.#graft' check && test -z "$(git status --porcelain)" && nix run '.#graft' -- outdated --json | jq -e 'all(.[]; .status == "current" or .status == "untagged")' >/dev/null && nix flake check && echo ok
```

It needs `jq`. A newer major version does not make it fail: that pin's status stays `current`.

## If something goes wrong

- **Step 7 fails, or you don't want the change.** Read the provider's **Upgrading** section (step 2), and fix
  your own files. To not take the versions now, delete the branch, and put the scions back at the old
  gitlinks: `git switch - && git branch -D upgrade && git submodule update`.
- **`graft: page-rules: can't tell whether v1.0.0 is current (see above); nothing changed`.** The line above
  it starts `graft: can't list tags at URL:`: graft cannot read that provider's repository. It is down,
  moved, or private. Fix the access, or the URL (`git submodule set-url scions/NAME URL`), and run step 3
  again.
- **`upgrade` refuses a scion: it has uncommitted edits, or is checked out away from its gitlink.** Someone
  is fixing that provider in place ([Fix a provider from a consumer](fix-a-provider.md)). Finish or stash
  that work, then `git submodule update -- scions/NAME`.
- **`graft: style-guide: no tag v1.2.0 upstream; nothing changed`.** You named a tag that the provider does
  not have. `nix run .#graft outdated` shows the newest one.

## How it works

- **Which tags count.** Only semver tags (`v1.2.3` or `1.2.3`) are versions. A scion whose gitlink is no
  version's commit (an unreleased fix) shows as `untagged`. `upgrade` with no names leaves it; name it,
  `NAME@TAG`, or `NAME` for the newest release in the major of its nearest tag. For `0.y.z`, each `0.y` series counts
  as a major version.
- **All or nothing.** `upgrade` reads every provider's tags first, and changes nothing if one cannot be read
  or a named tag does not exist.
- **graft's own pin.** Its pin moves first, then the rest of the upgrade runs the new version. Pin graft to a
  tag (`github:danielnaab/graft/v2.19.1`): a pin to the `v2` branch is never moved.
- **Without `upgrade`.** To move a scion yourself:
  `git -C scions/NAME fetch --tags && git -C scions/NAME checkout --detach v1.1.0 && git add scions/NAME`.
- **From outside the repository.** `outdated`, `changelog`, `upgrade` and `migrate` also run from graft's
  own flake, with `-C` ([Commands](../reference/commands.md#the-graft-command)).

### Cross a major version

graft never takes a new major version unless you name it, `upgrade NAME@TAG`, on a branch. The steps are in
[Ship and take an incompatible change](incompatible-change.md#part-2-a-consumer-takes-the-change).

### Try a provider's unreleased change

Moved to [Fix a provider from a consumer](fix-a-provider.md): a scion is a clone, so you check out the
provider's branch in it, and build under `graft try` (step 3 there).

## Related

[Run graft in CI and review its upgrade pull requests](ci.md), [Ship and take an incompatible change](incompatible-change.md),
[`graft check` fails](check-fails.md).

## Sources

- [lib/tools.sh](../../lib/tools.sh): `outdated`, `changelog` and `upgrade`, their lines and errors, and the
  move of graft's own pin first.
- [lib/workflows.sh](../../lib/workflows.sh): `graft: refreshed FILE`, which does not stage.
- [tests/run.sh](../../tests/run.sh): `upgrade` with filled and empty scions, refusing a dirty or moved scion,
  majors only by name, graft's pin refreshing the README and workflows, `.gitmodules` untouched.
- [tests/fixtures.sh](../../tests/fixtures.sh) `upgrade`: the repository this page is read against.
