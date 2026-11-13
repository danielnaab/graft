---
status: working
---
# Commands

Every `mkFlake` argument, command and flag.

## mkFlake

`graft.lib.mkFlake { inputs; scions?; dir?; flakeDir?; outputs?; }` returns the repository's flake
outputs.

| Argument | Default | Meaning |
|---|---|---|
| `inputs` | (required) | The flake's inputs. |
| `scions` | every `flake = false` input | The input names to graft. |
| `dir` | `"scions"` | The scions directory: a relative path inside the repository, not `.git`, with no `.` or `..` segments. graft replaces it on every sync, and refuses to replace a directory that it did not write. |
| `flakeDir` | `"."` | Where `flake.nix` is, from the repository root. See [Keep private files out of the Nix store](../guides/flake-in-a-subdirectory.md). |
| `outputs` | `{ }` | The repository's own outputs, merged with graft's at every level. |

graft adds two outputs. A repository's own outputs cannot replace them:

- `apps.<system>.graft`: the `graft` command, with `sync` and `check` built for this repository.
- `checks.<system>.graft`: the drift check. Only when `flakeDir` is `.`. In a subdirectory flake, graft
  drops a check of the repository's own with this name.

Scion names must match `[A-Za-z0-9][A-Za-z0-9._-]*`.

## The graft command

Run it as `nix run .#graft COMMAND`. A flag needs `--` before it, because Nix otherwise reads it:
`nix run .#graft -- outdated --json`. For a flake in a subdirectory, use `nix run path:DIR#graft`.

| Command | What it does |
|---|---|
| `sync` (the default) | Writes the scions directory from `flake.lock`, stages it with `git add`, and restores graft's workflows. If `flake.lock` pins an annotated tag's object, it changes the pin to the commit and stops: run it again. |
| `check` | Compares the working tree's scions directory and graft's workflows with the pins. Reads files as git would commit them. |
| `outdated [-C REPO] [--json]` | For graft and each scion: the pinned tag, the newest tag in its major version, the status, and the newest later major version. Status is `current`, `outdated`, `untagged` (not pinned to a semver tag) or `unknown` (the provider's repository cannot be read). |
| `changelog [-C REPO] [--json] [NAME[@TAG]...]` | Each provider's CHANGELOG sections between the pin and the target, **Upgrading** first, or its commits. Changes nothing. Fails after it prints if a provider's repository cannot be read. |
| `upgrade [-C REPO] [NAME[@TAG]...]` | Moves pins, locks them again and rebuilds the scions directory. With no names: every outdated pin, in its major version. `NAME@TAG` is the only way to cross a major version. Needs `flake.nix` in UTF-8. Moves graft's own pin only to 2.3.0 or later, and in CI only from `graft ci`. |
| `migrate [-C REPO] [--base REF] [--no-check] [--json]` | Prints a migration brief for an upgraded branch. Exits 0 when it prints a brief, also when checks fail. Exits non-zero only when it cannot make a brief. |
| `ci check\|upgrade\|pr` | The steps of graft's workflows. See [Run graft in CI and review its upgrade pull requests](../guides/ci.md). |
| `help` | Lists the commands. |

- The target of `changelog` and `upgrade` is the newest tag in the pinned major version, unless a `@TAG`
  names one.
- `NAME` without a tag stays in the pinned major version, and reports any newer major version that it did
  not take.
- `--json` gives the same data as the text output. In `outdated --json`, `major` is `null` when there is
  no newer major version.

## Run from graft's flake

`outdated`, `changelog`, `upgrade` and `migrate` also run from graft's own flake, against any checkout:

```
nix run github:danielnaab/graft -- outdated -C path/to/repo [--json]
nix run github:danielnaab/graft/v2.11.1 -- migrate -C path/to/repo --no-check
```

- `-C` takes the repository root, also for a flake in a subdirectory.
- graft reads the repository as data, and never evaluates its flake outputs.
- `upgrade` locks a moved input with `nix flake update`, which reads the flake's `inputs` but not its
  `outputs`. It locks a `git+` input itself ([Nix behaviours](nix-behaviours.md#annotated-tags-and-exportignore)).
- `upgrade` rebuilds the scions directory with the `lib/assemble.sh` of the graft version that the
  repository pins, so it writes exactly what the repository's check expects.

`sync` and `check` run only from the repository's own flake.

## Sources

- [lib/graft.sh](../../lib/graft.sh): commands and usage
- [lib/mk-flake.nix](../../lib/mk-flake.nix): arguments and outputs
- [lib/tools.sh](../../lib/tools.sh): `outdated`, `changelog`, `upgrade`
- [lib/migrate.sh](../../lib/migrate.sh), [lib/ci.sh](../../lib/ci.sh)
