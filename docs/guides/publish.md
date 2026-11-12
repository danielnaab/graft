---
status: working
---
# Publish

Make a repository that others can graft: decide what it exports, tag releases, and say what changes.

## Decide what you export

Your export is your tree at a tag, minus the paths that `.gitattributes` marks `export-ignore`:

```
/tests   export-ignore
/notes   export-ignore
/scions  export-ignore
```

- Always exclude your own scions directory. Consumers take your content, never your dependencies. A
  consumer that needs one of your dependencies grafts it directly. graft does not check this.
- Write directory rules without a trailing slash, as above. `tests/` works for `github:` inputs, but
  matches nothing for `git+…&exportIgnore=1` inputs. See
  [Nix behaviours](../reference/nix-behaviours.md#trailing-slash-rules).
- Exported files must link only to exported files. A link into an excluded path breaks in every
  consumer's copy. Link to it by URL instead.
- If consumers also run your code (a tool's source, for example), export it. They read it from the same
  input. Do not ask them to add a second input for your repository.

## Release

1. Add a section to the top of `CHANGELOG.md`: `## vX.Y.Z - YYYY-MM-DD`. Say what changed.
2. Commit.
3. Tag the commit with a semver tag, such as `v1.2.0`.
4. Push the branch and the tag.

Consumers take the release on their next `graft upgrade`, if it is in the major version that they pin.

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
