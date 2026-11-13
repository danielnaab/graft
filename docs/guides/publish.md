---
status: working
---
# Publish a pattern

Release a version of your repository that others can graft: choose what it exports, say what changed, tag
it, and take it in a consumer.

## Before you begin

- The terms provider, export rules, export and scion are in [Concepts](../concepts.md). Any git repository
  with semver tags can be a provider. What to export, and how to name the repository, are in
  [Files](../reference/files.md#in-a-provider).
- The example releases `note-kit` v1.2.0. Its v1.1.0 shipped its tests, its own scions, its CI workflow and
  its `AGENTS.md` to every consumer. v1.2.0 fixes its tool, `note-lint`, and its export. `team-notes`
  grafts it from note-kit's remote.
- Choose the version: a patch version for a fix, a minor version for an addition, and a major version for a
  change that can break a consumer ([Make an incompatible change](#make-an-incompatible-change)).
- Only committed files are exported. An untracked draft stays yours.
- If your pattern ships a tool, or files that consumers copy once, also read
  [Ship a tool or seed files with a pattern](ship-tools.md).
- Nix with flakes, for step 10 only.

## Steps

1. In `.gitattributes`, mark `export-ignore` each path that consumers must not get
   ([What to export](../reference/files.md#what-to-export)). Write each rule without a trailing slash. A
   leading `/` anchors the rule at the repository's top, so `/tests` does not match `docs/tests`. For
   note-kit, replace its one rule, `tests/ export-ignore`, with these:

   ```
   /tests     export-ignore
   /scions    export-ignore
   /.forgejo  export-ignore
   /AGENTS.md export-ignore
   ```

2. At the top of `CHANGELOG.md`, under its title, add a section for the release. Say what changed, for a
   reader who knows only their own repository:

   ```markdown
   ## v1.2.0 - 2026-11-12

   - note-lint checks notes in subfolders too.
   - Consumers no longer get note-kit's tests, scions, CI or AGENTS.md.
   ```

   graft reads sections headed `## vX.Y.Z` or `## X.Y.Z`. Anything can follow the version, such as the date.

3. Stage the files of the release: your export rules, the CHANGELOG, and any other change for this release
   (for note-kit, the fix to `bin/note-lint`):

   ```
   git add .gitattributes CHANGELOG.md bin/note-lint
   ```

   `git status --short` then shows them staged (`M  .gitattributes`, `M  CHANGELOG.md`, `M  bin/note-lint`),
   and what stays out of the release as untracked (`?? notes/`).

4. Commit:

   ```
   git commit -m "v1.2.0"
   ```

5. List what a consumer will get:

   ```
   git archive HEAD | tar -t
   ```

   For note-kit:

   ```
   .gitattributes
   CHANGELOG.md
   README.md
   bin/
   bin/note-lint
   docs/
   docs/notes-policy.md
   flake.nix
   package.nix
   seed/
   seed/note-kit.conf
   ```

   This is the export that GitHub serves to `github:` inputs. If the list holds a path that consumers must
   not get, add a rule, stage `.gitattributes`, run `git commit --amend --no-edit`, and list again. The list
   cannot show a rule with a trailing slash, because `git archive` applies it: step 6 catches that.

6. Check that no rule in the export has a trailing slash. This prints nothing:

   ```
   git archive HEAD | tar -t | grep -E '(^|/)\.gitattributes$' | while IFS= read -r f; do git show "HEAD:$f"; done | grep -E '^[[:space:]]*[^#[:space:]][^[:space:]]*/[[:space:]](.*[[:space:]])?export-ignore([[:space:]]|$)'
   ```

   `git archive` applies a rule such as `tests/`, but Nix ignores it when it fetches from a bare clone, as it
   does for `git+https` and `git+ssh` inputs ([Nix behaviours](../reference/nix-behaviours.md#trailing-slash-rules)).
   If it prints a rule, remove the slash, and amend as in step 5. Steps 5 and 6 are the checks before you
   tag. "Check it worked" confirms the release after the push.

7. Tag the commit:

   ```
   git tag v1.2.0
   ```

   It prints nothing. Prefer a lightweight tag, as here. A signed or annotated tag works too: on Nix 2.24,
   a consumer with a `git+` input sees one re-pin on its first run
   ([Nix behaviours](../reference/nix-behaviours.md#annotated-tags-and-exportignore)). Never move a released
   tag. Consumers' locks pin its commit and hash, so release a new version instead.

8. Push the branch:

   ```
   git push origin main
   ```

9. Push the tag:

   ```
   git push origin v1.2.0
   ```

   Git prints `* [new tag]         v1.2.0 -> v1.2.0`. graft reads the tags of the repository that a
   consumer's input names, so consumers see the release only after this push.

10. Take the release in a repository that grafts yours. For team-notes:

    ```
    nix run .#graft upgrade
    ```

    It prints `graft: note-kit v1.1.0 -> v1.2.0` and `graft: rebuilt scions/`. `git status --short scions/`
    shows `D` for each file that left the export, such as `scions/note-kit/AGENTS.md`,
    `scions/note-kit/.forgejo/workflows/test.yaml` and `scions/note-kit/tests/run.sh`. `upgrade` stages the
    scions directory, and leaves `flake.nix` and `flake.lock` changed but not staged. Review and check it as
    [Upgrade](upgrade.md) says, then commit all three. To make the first consumer, follow
    [Graft your first pattern](graft-a-repository.md).

    graft takes the newest release in the major version that a consumer pins. In CI, the weekly upgrade
    takes it as a pull request.

## Check it worked

After step 9, in your repository, set `t` to the new tag, and the pattern after `grep -qE '^(tests` to the
paths that you exclude. Then paste this. It prints `ok` only if the tag is pushed, `CHANGELOG.md` at the
tag has its section, the export at the tag holds no excluded path, and no rule in it has a trailing slash.
If the tag is not on `origin`, it prints `not pushed: v1.2.0`; any other failure prints nothing:

```sh
t=v1.2.0 && { git ls-remote --exit-code origin "refs/tags/$t" >/dev/null || { echo "not pushed: $t"; false; }; } && git show "$t:CHANGELOG.md" | grep -qE "^## \[?v?${t#v}( |\]|$)" && ! git archive "$t" | tar -t | grep -qE '^(tests|scions|\.forgejo)/|^AGENTS\.md$' && ! git archive "$t" | tar -t | grep -E '(^|/)\.gitattributes$' | while IFS= read -r f; do git show "$t:$f"; done | grep -qE '^[[:space:]]*[^#[:space:]][^[:space:]]*/[[:space:]](.*[[:space:]])?export-ignore([[:space:]]|$)' && echo ok
```

## If something goes wrong

- **`git tag v1.2.0` opens an editor, or fails with `fatal: no tag message?` where there is no editor.**
  Your git config sets `tag.gpgSign`, which makes every tag a signed tag. A signed tag works (step 7): give
  it a message, `git tag -m v1.2.0 v1.2.0`. Or make a lightweight one:
  `git -c tag.gpgSign=false tag v1.2.0`.
- **A consumer's first `nix run .#graft` stops with `graft: note-kit: flake.lock held annotated tag v1.1.0's
  object …; re-pinned its commit …` and `graft: flake.lock re-pinned; nothing synced yet: run nix run
  .#graft again`.** The tag is annotated or signed, and the consumer takes you through a `git+` input.
  This is expected: the consumer's second run grafts the right export
  ([Nix behaviours](../reference/nix-behaviours.md#annotated-tags-and-exportignore)).
- **A consumer's first run fails with `revspec 'v1.1.0' not found`.** The consumer's input is `git+https`,
  `git+ssh` or `git+file` to a bare repository, and Nix 2.24 looks for the tag as a branch until its git
  cache has seen the tag. The consumer fetches the tag once by its full name, as in
  [Graft a private repository](private-providers.md#steps) step 1, then runs graft again
  ([Nix behaviours](../reference/nix-behaviours.md)).
- **A consumer's scion holds a path that `git archive` leaves out, such as `tests/`.** A rule has a trailing
  slash (`tests/ export-ignore`), and the consumer's input fetches from a bare clone. Remove the slash
  (step 6), and release a patch version.

  Not yet: no graft check warns about a rule with a trailing slash.

## Decide what you export

Moved to [Files: What to export](../reference/files.md#what-to-export).

## Make an incompatible change

Release it as a new major version, with Upgrading steps that you have tried on a consumer:
[Ship and take an incompatible change](incompatible-change.md#part-1-the-provider-releases-the-change).

## Name the repository

Moved to [Files: Naming](../reference/files.md#naming).

Related: [Ship and take an incompatible change](incompatible-change.md), [Ship a tool or seed files with a pattern](ship-tools.md),
[Files](../reference/files.md#in-a-provider).

## Sources

- [lib/common.sh](../../lib/common.sh): `is_semver` (which tags are versions), "Annotated tags",
  `tag_shas` and `peel_locks` (the re-pin of a lock at a tag object), `upstream_range` and
  `changelog_between` (`CHANGELOG.md` read from the provider with git, and its section headings).
- [lib/export-ignored.sh](../../lib/export-ignored.sh): the check that a fetched export holds nothing that
  its export rules exclude.
- [lib/assemble.sh](../../lib/assemble.sh): the scions directory, one folder for each export.
- [tests/run.sh](../../tests/run.sh): the annotated-tag tests, and the providers that the whole cycle runs
  against.
- [tests/fixtures.sh](../../tests/fixtures.sh) `publish`: the repositories this page was read against.
- [Files](../reference/files.md#in-a-provider), [Nix behaviours](../reference/nix-behaviours.md).
