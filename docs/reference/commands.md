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
| `flakeDir` | `"."` | Where `flake.nix` is, from the repository root: `.`, or a relative path of letters, digits, `.`, `_`, `-` and `/`, with no `.` or `..` segments, not in `.git`. It goes into `path:` references and the scions README, so it has no spaces. Another value fails to evaluate with `graft: flakeDir must be …, got "VALUE"`. A value that names no `flake.nix` fails with `graft: no DIR/flake.nix in TOP: check mkFlake's flakeDir, …`. See [Keep private files out of the Nix store](../guides/flake-in-a-subdirectory.md). |
| `outputs` | `{ }` | The repository's own outputs, merged with graft's at every level. |

graft adds two outputs:

- `apps.<system>.graft`: the `graft` command, with `sync` and `check` built for this repository.
- `checks.<system>.graft`: the drift check. Only when `flakeDir` is `.`. In a subdirectory flake, graft
  drops a check of the repository's own with this name.

Both names are graft's, also when `flakeDir` is not `.`. If `outputs` has either one, graft's app and check
fail to evaluate with `graft: mkFlake's outputs has checks.SYSTEM.graft: that name is graft's; rename the repo's own`
(or `apps.SYSTEM.graft`). The repository's other outputs still evaluate.

Scion names must match `[A-Za-z0-9][A-Za-z0-9._-]*`.

## The graft command

Run it as `nix run .#graft COMMAND`. A flag needs `--` before it, because Nix otherwise reads it:
`nix run .#graft -- outdated --json`. For a flake in a subdirectory, use `nix run 'path:DIR#graft'`.

In zsh with `extendedglob` set, `#` is a glob character, so a flake reference that holds one fails with
`zsh: no matches found`. Quote it: `'.#graft'`, `'path:DIR#graft'`. The docs quote every `path:` form, and
write `.#graft` unquoted.

| Command | What it does |
|---|---|
| `sync` (the default) | Writes the scions directory from `flake.lock`, stages it with `git add`, and restores graft's workflows. If `flake.lock` pins an annotated tag's object, it changes the pin to the commit and stops: run it again. |
| `check` | Compares the working tree's scions directory and graft's workflows with the pins. Reads files as git would commit them. On drift, names each file from the repository root (`scions/NAME/FILE`): `edited`, with a unified diff; `only in your copy, not in the pin`; or `only in the pin, not in your copy`. Then it says what each kind of file usually is. `checks.<system>.graft` prints the same report for the files that git tracks. `check` fails when git ignores `flake.lock` (`FLAKEDIR/flake.lock` for a flake in a subdirectory), and `checks.<system>.graft` when git does not track it. A lock that is only uncommitted passes both: `ci check` catches it. |
| `outdated [-C REPO] [--json]` | For graft and each scion: the pinned tag, the newest tag in its major version, the status, and the newest later major version. Status is `current`, `outdated`, `untagged` (not pinned to a semver tag) or `unknown` (the provider's repository cannot be read). |
| `changelog [-C REPO] [--json] [NAME[@TAG]...]` | Each provider's CHANGELOG sections between the pin and the target, **Upgrading** first, or its commits. Changes nothing. Fails after it prints if a provider's repository cannot be read. |
| `upgrade [-C REPO] [NAME[@TAG]...]` | Moves pins, locks them again and rebuilds the scions directory. With no names: every outdated pin, in its major version. `NAME@TAG` is the only way to cross a major version. Needs `flake.nix` in UTF-8. Moves graft's own pin only to 2.3.0 or later, and in CI only from `graft ci`. |
| `migrate [-C REPO] [--base REF] [--no-check] [--json]` | Prints a migration brief for an upgraded branch. Exits 0 when it prints a brief, also when checks fail. Exits non-zero only when it cannot make a brief. |
| `ci check\|upgrade\|pr` | The steps of graft's workflows. See [Run graft in CI and review its upgrade pull requests](../guides/ci.md). `ci check` first fails, with `graft: flake.lock is not committed, so nothing is pinned: …`, when `flake.lock` is not in `HEAD`. |
| `help` | Lists the commands. |

- The target of `changelog` and `upgrade` is the newest tag in the pinned major version, unless a `@TAG`
  names one.
- `NAME` without a tag stays in the pinned major version, and reports any newer major version that it did
  not take.
- `outdated`, `changelog` and `upgrade`: an argument that the command does not take fails with exit 2, a
  `graft:` line that names it and the command's usage line: `-C` without a repository
  (`graft: -C needs a repository`), a name given to `outdated`
  (`graft: outdated takes no names: it lists graft and every scion`), `--json` given to `upgrade`
  (`graft: upgrade has no --json`), or an unknown option (`graft: unknown option OPTION`). A bare `--`
  among the arguments is skipped.
- `migrate`: an argument that it does not take, a bare `--` included, fails with exit 1 and
  `graft: usage: graft migrate [-C REPO] [--base REF] [--no-check] [--json]`. `-C` without a directory fails
  with `graft: -C needs a directory`.
- `sync` and `check` take no arguments: `graft: sync takes no arguments`, exit 2.
- `ci` reads only its step, and ignores the arguments after it. Another step fails with exit 1 and
  `graft: usage: graft ci check|upgrade|pr`.
- An unknown command prints the usage on stderr, and exits 2.
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
- [lib/drift.sh](../../lib/drift.sh): the drift report of `check` and `checks.<system>.graft`
- [lib/tools.sh](../../lib/tools.sh): `outdated`, `changelog`, `upgrade`
- [lib/migrate.sh](../../lib/migrate.sh), [lib/ci.sh](../../lib/ci.sh)
