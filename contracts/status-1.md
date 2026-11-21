---
status: draft
---
# status/1: a repository's scions, for other programs

What `graft status --json` prints: for one checkout, each scion's pin and its state, and graft's own pin, read from
the checkout and the scions' upstreams. A program that looks across many checkouts, such as grove, reads this
contract instead of `.gitmodules`, gitlinks and `graft check`'s messages.

| | |
|---|---|
| Version | 1: the `version` of `graft status --json` |
| Producer | graft, from v2.22.0 |
| Reader | grove; any program that shows scions across checkouts |
| Owner | graft |
| Schema | [status-1.schema.json](status-1.schema.json) (JSON Schema 2020-12) |
| Fixtures | [fixtures/status-1/](fixtures/status-1/): `clean.json`, `problems.json`, `offline.json`, `no-scions.json` |

## Shape

```
graft status [-C REPO] [--json]
```

`graft status` reads what `graft check` and `graft outdated` read, and joins them: one entry for each scion, with
check's facts and messages, and outdated's newest tag. It changes nothing, evaluates nothing (no Nix), and reads each
upstream's tags once, with `git ls-remote`. It does not compare graft's workflows or evaluate a provider's
`graft.nix`: `graft check` does both.

```json
{ "version": 1, "repo": "/home/ada/src/team-kb", "scions_dir": "scions",
  "graft": { "url": "https://github.com/danielnaab/graft.git", "commit": "928a188…", "tag": "v2.21.0",
             "latest": { "status": "outdated", "tag": "v2.22.0", "major": null, "reason": null } },
  "scions": [
    { "name": "knowledge-base-kit", "path": "scions/knowledge-base-kit",
      "url": "https://github.com/danielnaab/knowledge-base-kit", "registered": true,
      "commit": "348eff0…", "tag": "v2.1.0", "tag_from": "upstream",
      "filled": true, "head": "348eff0…", "clean": true,
      "latest": { "status": "current", "tag": "v2.1.0", "major": "v3.0.0", "reason": null },
      "problems": [ ] } ],
  "problems": [ ] }
```

### The report

| Field | Type | Meaning |
|---|---|---|
| `version` | number | `1` |
| `repo` | string | the checkout's top, an absolute path |
| `scions_dir` | string or `null` | the scions directory, from the top; `null` when the repository has none yet (no README with graft's marker) |
| `graft` | object or `null` | graft's own pin, the flake input named `graft` in the flake's `flake.lock`; `null` when there is none, or when a scion is named `graft` |
| `scions` | list | one entry for each scion, sorted by name: each registered scion, then each folder or gitlink in the scions directory that `.gitmodules` does not register |
| `problems` | list of strings | `graft check`'s messages about the repository rather than one scion: `graft try` left on, a top-level flake without `inputs.self.submodules` |

### A scion

| Field | Type | Meaning |
|---|---|---|
| `name` | string | the scion's folder name |
| `path` | string | its path from the top |
| `url` | string or `null` | its URL in `.gitmodules`; `null` when it has none, or is not registered |
| `registered` | boolean | `.gitmodules` registers the path |
| `commit` | string or `null` | the pin: the gitlink the index records (40 hex digits; 64 in a SHA-256 repository); `null` when the path is not a submodule in the index |
| `tag` | string or `null` | the highest semver tag at `commit`; `null` when none is, or when it cannot be read |
| `tag_from` | `"upstream"`, `"local"` or `null` | where `tag` was read: the upstream's tags; the scion's own clone, when the upstream cannot be read (as `graft check` does); `null` when neither could be read. `tag` is `null` whenever `tag_from` is |
| `filled` | boolean | the folder holds a clone (`<path>/.git` exists) |
| `head` | string or `null` | the commit the clone has checked out; `null` when not filled |
| `clean` | boolean or `null` | no uncommitted edits to tracked files (untracked files do not count: Nix ignores them); `null` when not filled |
| `latest` | object | the newest release, as `graft outdated` reports it ([below](#latest)) |
| `problems` | list of strings | `graft check`'s messages for this scion, without the `graft: ` prefix; empty when check passes it |

A scion that `graft check` passes has `registered`, `filled` and `clean` true, `head` equal to `commit`, a `tag`, and
no `problems`. A reader that wants one answer reads `problems`: the fields say why.

### latest

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `current`: the pin is the newest release of its major version. `outdated`: a newer release of that major exists. `untagged`: the pin is not a release. `unknown`: the upstream's tags could not be read |
| `tag` | string or `null` | the newest release of the pinned major (for `untagged`, of the major of the nearest release before the pin); `null` when there is none, or for `unknown` |
| `major` | string or `null` | the newest release of a later major version, which `graft upgrade` takes only when named; `null` when there is none, or for `unknown` |
| `reason` | string or `null` | for `unknown`, why: git's error from listing the tags, `URL is not reachable from this pot run`, or what is missing (`no URL in .gitmodules`); `null` otherwise |

A value that needs the network is never left out: when the upstream cannot be read, `latest` is `unknown` with its
`reason`, and `tag` comes from the scion's clone (`tag_from: "local"`) or is `null`.

### Exit status

| Status | When |
|---|---|
| 0 | the report was written, whatever state the scions are in. `graft check` is the command that fails on them. |
| 1 | the repository cannot be read: not a git work tree, two scions directories, or graft v2.12's copy-mode README. The message, on stderr, names the fix. |
| 2 | a usage error: an unknown option or an argument. |

Without `--json`, `graft status` prints a table, one row for graft's pin and one for each scion, then each problem
as a `graft: ` line.

## Schema and fixtures

[status-1.schema.json](status-1.schema.json) describes what this release emits, and its fields are closed
(`additionalProperties: false`): a new field is a new version. graft's tests validate every report that
`tests/run.sh status` produces against it, and `nix flake check` validates the fixtures.

The fixtures in [fixtures/status-1/](fixtures/status-1/) are for readers: test against them, not against a live
`graft status`. `clean.json` is a repository whose scions are at release tags, with one not yet filled.
`problems.json` has a dirty scion away from its gitlink at no release, a stray folder, and `graft try` on.
`offline.json` has upstreams that cannot be read. `no-scions.json` is a repository with none.

## Compatibility

- **Values are added, never fields,** within version 1: a new `latest.status` or `tag_from` value, a new problem
  message. A reader ignores what it does not know, and treats a status it does not know as one it does not act on.
  Do not use the schema to reject a newer graft's report.
- **Messages are text.** A problem's wording may change in any release; a reader shows it, and decides on the
  fields.
- **A new field, or a changed meaning, is status/2.** graft then prints `version: 2`, and a reader that knows only 1
  says so rather than guessing.

## Sources

- [lib/scions.sh](../lib/scions.sh): `graft status` and `graft check`, and the facts that both read of a scion
- [lib/common.sh](../lib/common.sh): the scions directory, `.gitmodules`, gitlinks, upstream tags, and the newest
  release (`graft outdated`'s rows)
- [lib/graft.sh](../lib/graft.sh): the command
