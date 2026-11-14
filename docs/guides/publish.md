---
status: working
---
# Publish a pattern

Release a version of your repository that others can graft: say what changed, tag it, push it, and take it
in a consumer.

## Before you begin

- The terms provider, consumer and scion are in [Concepts](../concepts.md). Any git repository with semver
  tags can be a provider. How to name it is in [Files](../reference/files.md#naming).
- Consumers get your **whole repository** at the tag, as a submodule: tests, CI files and all. Keep instructions
  for your own agents (`AGENTS.md`) about work on the provider; tell consumers in your README what to point
  their agents at.
- The example releases `note-kit` v1.2.0, which fixes its tool, `note-lint`. `team-notes` grafts it.
- Choose the version: a patch version for a fix, a minor version for an addition (a new tool, a first
  `graft.nix`), and a major version for a change that can break a consumer
  ([Ship and take an incompatible change](incompatible-change.md#part-1-the-provider-releases-the-change)).
- If your pattern ships tools or checks, also read [Write a provider's graft.nix](write-graft-nix.md).

## Steps

1. At the top of `CHANGELOG.md`, under its title, add a section for the release. Say what changed, for a
   reader who knows only their own repository:

   ```markdown
   ## v1.2.0 - 2026-11-12

   - note-lint checks notes in subfolders too.
   ```

   graft reads sections headed `## vX.Y.Z` or `## X.Y.Z`. Anything can follow the version, such as the date.

2. If you ship a `graft.nix`, run `nix flake check` in your repository, with a check that evaluates it as a
   consumer would ([Check the contract before you tag](write-graft-nix.md#check-the-contract-before-you-tag)).
   An error in `graft.nix` fails every consumer's checks at their next upgrade; a parse error breaks their
   whole flake.

3. Commit the release on your default branch:

   ```
   git add CHANGELOG.md bin/note-lint
   git commit -m "v1.2.0"
   ```

4. Tag the commit:

   ```
   git tag v1.2.0
   ```

   A lightweight, annotated or signed tag all work. Tag a commit on your default branch: a consumer with
   `push.recurseSubmodules=check` cannot push a gitlink that only a tag reaches. Never move a released tag:
   consumers' gitlinks point at its commit.

5. Push the branch, then the tag:

   ```
   git push origin main
   git push origin v1.2.0
   ```

   Git prints `* [new tag]         v1.2.0 -> v1.2.0`. graft reads tags with `git ls-remote`, so consumers see
   the release only after this push.

6. Take the release in a repository that grafts yours. In team-notes:

   ```
   nix run .#graft upgrade note-kit
   ```

   It prints `graft: note-kit v1.1.0 -> v1.2.0`. Commit and check as
   [Take a newer version by hand](upgrade.md#steps) says. In CI, the weekly upgrade takes it as a pull
   request. To make the first consumer, follow [Graft your first pattern](graft-a-repository.md).

## Check it worked

After step 5, in your repository, set `t` to the new tag. Paste this. It prints `ok` only if the tag is
pushed, it is on `origin/main`, and `CHANGELOG.md` at the tag has its section. If the tag is not on `origin`,
it prints `not pushed: v1.2.0`:

```sh
t=v1.2.0 && { git ls-remote --exit-code origin "refs/tags/$t" >/dev/null || { echo "not pushed: $t"; false; }; } && git fetch -q origin main && git merge-base --is-ancestor "$t" origin/main && git show "$t:CHANGELOG.md" | grep -qE "^## \[?v?${t#v}( |\]|$)" && echo ok
```

## If something goes wrong

- **`git tag v1.2.0` opens an editor, or fails with `fatal: no tag message?`.** Your git config sets
  `tag.gpgSign`. Give it a message, `git tag -m v1.2.0 v1.2.0`, or make a lightweight one:
  `git -c tag.gpgSign=false tag v1.2.0`.
- **A consumer's `graft add` or `upgrade` says `graft: NAME: no tag v1.2.0 upstream`.** The tag is not pushed:
  step 5.
- **A consumer's build fails with `Cannot find Git revision '<sha>' in ref 'refs/heads/main' of repository '<url>'`.**
  The consumer records a commit that your remote does not have: you deleted or rewrote a pushed commit or tag.
  Push it again, or release a new version.
- **A consumer's evaluation fails fetching a repository it has never heard of.** Your repository has a
  submodule of its own that the consumer cannot read: Nix fetches nested submodules. Remove it, or make it
  readable ([One level](../decisions.md#one-level)).

## Release

Moved to [Steps](#steps).

## Decide what you export

Since v2.13.0 there are no export rules: a consumer gets the whole repository. `export-ignore` lines in
`.gitattributes` still apply to `git archive`, if you use it.

## Make an incompatible change

Release it as a new major version, with Upgrading steps that you have tried on a consumer:
[Ship and take an incompatible change](incompatible-change.md#part-1-the-provider-releases-the-change).

## Name the repository

Moved to [Files: Naming](../reference/files.md#naming).

## Sources

- [lib/common.sh](../../lib/common.sh): `is_semver` (which tags are versions), tags read with `git ls-remote`,
  `upstream_range` and `changelog_between` (`CHANGELOG.md` read from the provider, and its section headings).
- [tests/run.sh](../../tests/run.sh): the providers that the whole cycle runs against, with lightweight and
  annotated tags.
- [tests/fixtures.sh](../../tests/fixtures.sh) `publish`: the repositories this page was read against.
- [Files](../reference/files.md#in-a-provider).
