---
status: draft
---
# Publish a pattern

Release a version of your repository that others can graft: choose what it exports, say what changed, tag
it, and take it in a consumer.

## Before you begin

- The terms provider, export rules, export and scion are in [Concepts](../concepts.md). Any git repository
  with semver tags can be a provider.
- The example releases `note-kit` v1.2.0. Its v1.1.0 shipped its tests, its own scions, its CI workflow and
  its `AGENTS.md` to every consumer. v1.2.0 fixes its tool, `note-lint`, and its export. `team-notes`
  grafts it.
- Choose the version: a patch version for a fix, a minor version for an addition, and a major version for a
  change that can break a consumer ([Make an incompatible change](#make-an-incompatible-change)).
- Only committed files are exported. An untracked draft stays yours.
- Nix with flakes, for step 9 only.

## Steps

1. In `.gitattributes`, mark `export-ignore` each path that consumers must not get. What to exclude is in
   [Decide what you export](#decide-what-you-export). For note-kit, replace its one rule with these:

   ```
   /tests     export-ignore
   /scions    export-ignore
   /.forgejo  export-ignore
   /AGENTS.md export-ignore
   ```

   Write each rule without a trailing slash. Then this prints nothing:

   ```
   grep -nE '/[[:space:]]+export-ignore' .gitattributes
   ```

2. At the top of `CHANGELOG.md`, under its title, add a section for the release. Say what changed, for a
   reader who knows only their own repository:

   ```markdown
   ## v1.2.0 - 2026-11-12

   - note-lint checks notes in subfolders too.
   - Consumers no longer get note-kit's tests, scions, CI or AGENTS.md.
   ```

   graft reads sections headed `## vX.Y.Z` or `## X.Y.Z`. Anything can follow the version, such as the date.

3. Stage the files of the release:

   ```
   git add .gitattributes CHANGELOG.md bin/note-lint
   ```

   `git status --short` then shows only what stays out of the release, such as `?? notes/`.

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

   This is the export that GitHub serves to `github:` inputs. Nix applies the same rules for
   `git+…&exportIgnore=1` inputs when no rule has a trailing slash (step 1). If the list holds a path that
   consumers must not get, add a rule, and do steps 3 to 5 again.

6. Tag the commit with a lightweight tag:

   ```
   git tag v1.2.0
   ```

   It prints nothing. Do not use `git tag -a` or `git tag -s`: see
   [If something goes wrong](#if-something-goes-wrong). Never move a released tag. Consumers' locks pin
   its commit and hash, so release a new version instead.

7. Push the branch:

   ```
   git push origin main
   ```

8. Push the tag:

   ```
   git push origin v1.2.0
   ```

   Git prints `* [new tag]         v1.2.0 -> v1.2.0`. graft reads the tags of the repository that a
   consumer's input names, so consumers see the release only after this push.

9. Take the release in a repository that grafts yours. For team-notes:

   ```
   nix run .#graft upgrade
   ```

   It prints `graft: note-kit v1.1.0 -> v1.2.0` and `graft: rebuilt scions/`. `git status --short scions/`
   shows `D` for each file that left the export, such as `scions/note-kit/AGENTS.md`. Then
   `nix flake check` passes. To make the first consumer, follow
   [Graft your first pattern](graft-a-repository.md).

   graft takes the newest release in the major version that a consumer pins. In CI, the weekly upgrade
   takes it as a pull request ([Upgrade](upgrade.md)).

## Check it worked

In your repository, set `t` to the new tag, and the pattern after `grep -qE` to the paths that you exclude.
Then paste this. It prints `ok` only if the tag is lightweight and pushed, `CHANGELOG.md` at the tag has
its section, the export at the tag holds no excluded path, and no rule has a trailing slash:

```sh
t=v1.2.0 && test "$(git cat-file -t "$t")" = commit && git ls-remote --exit-code origin "refs/tags/$t" >/dev/null && git show "$t:CHANGELOG.md" | grep -q "^## $t" && ! git archive "$t" | tar -t | grep -qE '^(tests|scions|\.forgejo)/|^AGENTS\.md$' && ! grep -qE '/[[:space:]]+export-ignore' .gitattributes && echo ok
```

## If something goes wrong

- **`git tag v1.2.0` fails with `fatal: no tag message?`.** Your git config sets `tag.gpgSign`, which makes
  every tag an annotated, signed tag. Make a lightweight one: `git -c tag.gpgSign=false tag v1.2.0`.
- **A consumer's first `nix run .#graft` stops with `graft: flake.lock held annotated tag v1.1.0's object
  …; re-pinned its commit …` and `graft: flake.lock re-pinned; nothing synced yet: run nix run .#graft
  again`.** The tag is annotated, and the consumer takes you through a `git+` input. Nix 2.24 locks such a
  tag at the tag object, and then ignores your export rules. graft repairs the lock, and the consumer's
  second run grafts the right export
  ([Nix behaviours](../reference/nix-behaviours.md#annotated-tags-and-exportignore)). `github:` inputs are
  not affected. Leave the tag as it is, and tag your next release with a lightweight tag.
- **A consumer's scion holds a path that `git archive` leaves out, such as `tests/`.** A rule has a trailing
  slash (`tests/ export-ignore`), and the consumer takes you through a `git+` input, which ignores such a
  rule ([Nix behaviours](../reference/nix-behaviours.md#trailing-slash-rules)). Remove the slash, and release
  a patch version.

  Not yet: no graft check warns about a rule with a trailing slash.
- **A consumer's check fails with `…/bin/note-lint: cannot execute: required file not found`.** The check
  runs your script directly, and its `#!/usr/bin/env bash` line names a path that Nix's build sandbox does
  not have. Ship a Nix expression that builds the tool ([Ship a tool](#ship-a-tool)), or tell consumers to
  run it through an interpreter (`bash ${inputs.note-kit}/bin/note-lint`).

## Decide what you export

Your export is your tree at a tag, minus the paths that your export rules exclude. Exclude what is for work
on your repository, not for its consumers:

- **Your tests and fixtures.**
- **Your own scions directory.** Consumers take your content, never your dependencies. A consumer that
  needs one of your dependencies grafts it directly ([One level](../decisions.md#one-level)). graft does
  not check this.
- **Your CI files**, such as `.forgejo/` or `.github/`.
- **Your agents' instructions**, such as `AGENTS.md` or `CLAUDE.md`. A consumer's agent would read them as
  its own. Put instructions for consumers in a file that you export, and that they point their agents at.
- **Your own upkeep**, such as notes, and the configuration of tools that check only your repository.

Keep `README.md`, the files that consumers use, and `CHANGELOG.md`. graft reads `CHANGELOG.md` from your
repository with git, not from the export, so `graft changelog` works either way. Exported, it is also in
each consumer's scion.

- Write directory rules without a trailing slash, and with a leading `/` for a top-level path. `tests/`
  works for `github:` inputs, and matches nothing for `git+…&exportIgnore=1` inputs. See
  [Nix behaviours](../reference/nix-behaviours.md#trailing-slash-rules).
- Exported files must link only to exported files. A link into an excluded path breaks in every
  consumer's copy. Link to it by URL instead.
- graft's own [`.gitattributes`](../../.gitattributes) is a real example: it excludes its scions and its
  upkeep files.

## Ship a tool or seed files

### Ship a tool

A tool's source is part of your export. Consumers build and run it from the same input, so they need no
second input for your repository.

- Ship a Nix expression that builds the tool with what it needs, such as note-kit's `package.nix`:

  ```nix
  { writeShellApplication, gnused, gnugrep, findutils }:
  writeShellApplication {
    name = "note-lint";
    runtimeInputs = [ gnused gnugrep findutils ];
    text = builtins.readFile ./bin/note-lint;
  }
  ```

- Say in your README how to call it. A consumer builds it from the fetched export, `${inputs.note-kit}`,
  in a check of its own:

  ```nix
  note-lint = pkgs.callPackage "${inputs.note-kit}/package.nix" { };
  ```

  [Make a pattern's checks gate your merges](gate-merges.md) shows a whole check, and says why it reads
  `${inputs.<name>}` and not `scions/<name>`.
- Consumers graft you with `flake = false`, so your own flake's `packages` and `apps` do not reach them
  through that input.
- If your tool reads a setting from the consumer's repository, name the file and the setting in your
  README ([Change a grafted file](change-a-grafted-file.md)).

### Ship seed files

A seed file is a file that a consumer copies once and then owns, such as a starter configuration. It is not
a scion: graft replaces a scion, and would remove the consumer's edits.

- **One or two files:** export them, and tell consumers to copy them out of the scion once, for example
  `cp scions/note-kit/seed/note-kit.conf .`. The scion keeps your current version, to compare with at each
  upgrade.
- **A starter that `nix flake init` writes:** give your `flake.nix` a template, as note-kit does, and keep
  `flake.nix` and the template's directory in your export:

  ```nix
  {
    outputs = { self }: {
      templates.default = { path = ./seed; description = "note-kit's settings, to copy once"; };
    };
  }
  ```

  A consumer runs `nix flake init -t <your repository's URL at a tag>`, for example
  `nix flake init -t github:OWNER/note-kit/v1.2.0`. It writes the files and stages them.
- **Files that consumers edit, and that should still take your later changes:** use a template tool such as
  Copier, which merges your changes into their edits. See the README's
  [comparison](../../README.md#compared-with-copying-submodules-templates-and-bots).

### A step after each upgrade

If consumers must run a step after each upgrade, such as regenerating files from your templates, give them
a recipe that runs both. For example, for their `justfile`:

```just
# Take new releases, then regenerate what note-kit renders
upgrade:
    nix run .#graft upgrade
    nix run .#render
```

graft has no post-upgrade hook ([Scope](../decisions.md#scope-nix-a-copy-a-diff-and-a-tag-comparison)),
and the weekly upgrade in CI does not run the recipe. Also give consumers a check that fails when the
regenerated files differ from what the new version renders. The upgrade pull request then fails that check,
and a person runs the recipe on its branch.

### Links to another provider's files

If your exported files link to another provider's files by URL at a tag (because you do not export your
scions), change that tag when you upgrade that provider. graft does not. To find the links, search for the
old tag, for example `git grep -n 'knowledge-base-kit/blob/v2.1.0'`.

## Make an incompatible change

Anything that a consumer can rely on is your interface: an exported path, a heading that a consumer links
to, a file's shape.

- **An incompatible change needs a new major version.** graft never crosses a major version by itself.
  The weekly upgrade continues to take your fixes, and a person decides when to take the break.
- **Write Upgrading steps.** In the major version's CHANGELOG section, add an `### Upgrading` subsection.
  Write steps that a person or agent can follow with knowledge of the consumer only: what moved, what to
  search for, what to replace it with, and how to know it is done (`nix flake check` passes, for
  example). graft quotes these sections in upgrade pull requests and migration briefs.
- **Leave a redirect for a moved file.** For one major version, a stub at the old path can name the new
  path, so links into the scion still lead somewhere. Remove the stub at the next major version.

## Name the repository

The name ends with what the content is, not the role that the repository plays:

| Suffix | Content | A consumer should |
|---|---|---|
| `-kit` | Something to adopt and keep upgrading: tools, skills, structure, often with the rules that they enforce | run or adopt it |
| `-conventions` | Rules only | comply with it |
| `-knowledge` | A knowledge base about a topic | read it |

- A repository that defines a named method takes that name, such as `living-specifications`.
- Do not use role words (`-provider`, `-scion`), words that other tools own (`-pack`, `-bundle`,
  `-template`), or words that suggest a one-time copy (`-seed`, `-starter`).
- Give the repository the topic `graft-provider` on its forge (Forgejo or GitHub).

Related: [Upgrade](upgrade.md#cross-a-major-version), [Files](../reference/files.md#in-a-provider).

## Sources

- [lib/common.sh](../../lib/common.sh): `is_semver` (which tags are versions), "Annotated tags",
  `tag_shas` and `peel_locks` (the repair of a lock at a tag object), `upstream_range` and
  `changelog_between` (`CHANGELOG.md` read from the provider with git, and its section headings).
- [lib/export-ignored.sh](../../lib/export-ignored.sh): the check that a fetched export holds nothing that
  its export rules exclude.
- [lib/assemble.sh](../../lib/assemble.sh): the scions directory, one folder for each export.
- [.gitattributes](../../.gitattributes): graft's own export rules.
- [tests/run.sh](../../tests/run.sh): the annotated-tag tests, and the providers that the whole cycle runs
  against.
- [tests/fixtures.sh](../../tests/fixtures.sh) `publish`: the repositories this page was read against.
- [Files](../reference/files.md#in-a-provider), [Nix behaviours](../reference/nix-behaviours.md).
