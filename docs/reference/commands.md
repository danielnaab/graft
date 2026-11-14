---
status: working
---
# Commands

Every `mkFlake` argument, flake output, command and flag.

## mkFlake

`graft.lib.mkFlake { inputs; dir ? "scions"; perSystem ? null; outputs ? { }; }` returns the repository's
flake outputs.

| Argument | Default | Meaning |
|---|---|---|
| `inputs` | (required) | The flake's inputs. `inputs.self` is the repository. If there is an `inputs.nixpkgs`, it is the `pkgs` that every `graft.nix` gets; otherwise graft's own pinned nixpkgs. |
| `dir` | `"scions"` | The scions directory: one folder name (`[A-Za-z0-9._-]+`, not `.`, `..` or `.git`), in the flake's directory. For a [flake in a subdirectory](../guides/flake-in-a-subdirectory.md), `".scions"` by convention. |
| `perSystem` | none | The repository's own tools, apps, packages, checks and dev shells: a function `{ pkgs, lib, system, scions, ... }: { tools ? { }; apps; packages; checks; devShells; }`, called once per system and merged like a provider's `graft.nix`, under the name `self` (collisions say `from A and self`). Its checks are not prefixed: `checks.<system>.NAME`. Its `devShells` are output as they are. Use it to put a tool of your own, such as `bun`, into `tools`. |
| `outputs` | `{ }` | Plain extra outputs, merged last with `recursiveUpdate`, so they add to and override graft's. |

Removed in v2.13.0: `scions` (there is no input list) and `flakeDir` (graft finds the flake as the parent of
the scions directory).

A top-level git flake must declare `inputs.self.submodules = true;` (Nix 2.27 or later). A `path:` flake
does not need it.

### Discovery

Every directory in `${inputs.self}/<dir>` is a scion. Where the flake's source has a `.gitmodules` (a flake
at the repository's top), its submodule paths under `<dir>` must match those directories. Each scion is copied
alone into the store, without its `.git` file, as `scion-<name>`. A scion with a `graft.nix` at its top is
called once per system ([Files](files.md#graftnix)).

### Outputs

For `x86_64-linux`, `aarch64-linux`, `x86_64-darwin` and `aarch64-darwin`:

| Output | What |
|---|---|
| `packages.<system>.tools` | One environment (`buildEnv`) of every scion's `tools`. Empty when no scion has any. |
| `packages.<system>.<name>` | Each scion's `packages`. |
| `apps.<system>.<name>` | Each scion's `apps`. |
| `apps.<system>.graft` | The `graft` command for this repository. |
| `checks.<system>.<scion>-<name>` | Each scion's `checks`, prefixed with the scion's folder name. |
| `devShells.<system>.<name>` | `perSystem`'s `devShells`. If none defines `default`, `devShells.<system>.default` is a shell with `tools` on `PATH`. Providers ship no dev shells. |

Everything that reads scions is evaluated lazily. `apps.<system>.graft` and the repository's own `outputs`
evaluate when a scion is empty or broken, so `nix run .#graft …` can still repair it.

### Evaluation errors

mkFlake fails evaluation of the outputs that use a broken scion with one of these. Each is thrown inside a
value, so other outputs still evaluate. The fix for each is in
[`graft check` fails](../guides/check-fails.md#nix-evaluation-errors).

- `graft: tools nb-index (from mk and nk): two scions provide the same name`. Also for `apps` and `packages`.
  Providers are named in alphabetical order, `self` (your `perSystem`) last. Two tools with the same name and
  the same store path are one tool, not a collision.
- `graft: nk needs kb, which this repository does not graft: graft add URL`
- `graft: .gitmodules registers scions/a, scions/b but the flake's source lacks them: add inputs.self.submodules = true; to flake.nix (Nix >= 2.27). Fetched with github:? use git+https://…, which keeps submodules.`
  (`lacks it` for one scion.)
- `graft: DIR/README.md lacks graft's marker: is dir right? (mkFlake's dir = "DIR")`
- `graft: DIR/README.md is graft's copy-mode README (v2.12 or earlier): graft v2.13.0 pins scions with git submodules; follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md`
- `graft: DIR/NAME is not a submodule: graft add URL, or remove it`
- `graft: DIR/NAME: a scion's folder name must be letters, digits, . _ and - (starting with a letter or digit), and not self`
- `graft: mkFlake's dir must be one folder name (letters, digits, . _ -; not ., .. or .git), got "DIR"`
- `graft: scions/NAME is empty in the flake's source: run git submodule update --init (or graft sync)`, when a
  provider uses an empty sibling (`scions.NAME`).
- `graft: apps graft (from NAME): that name is graft's; the provider must rename it`, and the same for
  `packages tools`; for `perSystem`, `…; rename the repo's own`.
- `graft: mkFlake's outputs has apps.<system>.graft: that name is graft's; rename the repo's own`

An empty scion is not an error by itself. `packages.<system>.tools` builds without it, and Nix prints a
warning:

```
warning: graft: scions/a, scions/b are empty in the flake's source, so the tools leave out what they give: run git submodule update --init (or graft sync)
```

followed by `. Fetched with github:? use git+https://…, which keeps submodules.` for a flake at the top. A
`github:` tarball has the scion folders empty, so it gets this warning.

A repository with no scions yet is allowed: `tools` is then an empty environment, and `graft check` warns.

## The graft command

Run it as `nix run .#graft COMMAND`. A flag needs `--` before it, because Nix otherwise reads it:
`nix run .#graft -- outdated --json`. For a flake in a subdirectory, run `nix run 'path:DIR#graft'` from
the repository's top.

Every command except `sync` also runs from graft's own flake, against any checkout:
`nix run github:danielnaab/graft/v2.13.0 -- COMMAND -C REPO`. graft reads the repository as data, and never
evaluates its flake outputs.

In zsh with `extendedglob` set, `#` is a glob character, so a flake reference that holds one fails with
`zsh: no matches found`. Quote it: `'.#graft'`, `'path:DIR#graft'`. The docs quote every `path:` form, and
write `.#graft` unquoted.

graft finds the scions directory by its marker README, with
`git grep -l -F '<!-- managed by graft: https://github.com/danielnaab/graft -->' -- README.md '*/README.md'`.
`git grep` does not look inside submodules, so a provider's own scions README never counts. A repository has
at most one: `graft: two scions directories: A, B; graft supports one per repository`. The flake's directory
is the scions directory's parent.

| Command | What it does |
|---|---|
| `add [-C REPO] [--scions PATH] URL[@TAG] [NAME]` | Adds a scion at `TAG`, default the newest version upstream. `NAME` defaults to the URL's last segment without `.git`. Runs `git submodule add`, checks out the tag's commit, writes the marker README if it is missing, and stages `.gitmodules`, the gitlink and the README. |
| `sync` (the default) | Fills every empty scion (`git submodule update --init`, not `--recursive`), writes the marker README, and refreshes graft's workflows. Never moves a gitlink, and never moves a filled scion. In the repository's own flake only. |
| `check` | Fails unless every rule in [Check](#check) holds. |
| `outdated [-C REPO] [--json]` | For graft and each scion: the pinned version, the newest version in its major, the status, and the newest later major. |
| `changelog [-C REPO] [--json] [NAME[@TAG]...]` | Each provider's CHANGELOG sections between the pin and the target, **Upgrading** first, or its commits. Changes nothing. |
| `upgrade [-C REPO] [NAME[@TAG]...]` | Moves pins. With no names: graft and every outdated scion, each in its major version. `NAME@TAG` is the only way to cross a major. |
| `migrate [-C REPO] [--base REF] [--no-check] [--json]` | Prints a migration brief for an upgraded branch. |
| `ci check\|upgrade\|pr` | The steps of graft's workflows. See [CI](#ci). |
| `help` | Lists the commands. |

There is no `remove`: `git rm scions/NAME` removes a scion, and git edits `.gitmodules`.

### add

- `@TAG` is split off only when it is a version tag, so `git@host:team/x.git` is one URL.
- The scions directory: the existing one; else `--scions`, a path from the repository's top whose parent
  holds a `flake.nix` (`nix/agent-tools/.scions`); else `scions` at the top.
- Prints `graft: scions/NAME <- vX.Y.Z (abc1234)`. For a top-level flake without `inputs.self.submodules`, it
  then prints `graft: next: commit; a git flake also needs inputs.self.submodules = true;`.
- When the new scion has submodules of its own, it prints
  `note: scions/NAME has its own submodules (…); Nix fetches them when evaluating`.
- Errors: `graft: URL has no semver tags`, `graft: NAME: no tag TAG upstream`, `graft: scions/NAME exists`,
  `graft: a relative URL (../x.git) won't resolve for Nix; give an absolute URL`,
  `graft: not inside a git work tree`.

### sync

Prints one line per scion: `graft: scions/NAME at vX.Y.Z (abc1234)`, or `graft: scions/NAME at abc1234 (untagged)`.
A filled scion is left as it is, also one on a fix branch or away from its gitlink, and reported. To put a
filled scion back at its gitlink, use `git submodule update -- scions/NAME`. Nested submodules are not filled:
Nix fetches them itself when it needs them.

The marker README's commands name this flake's run command: `nix run .#graft` for a flake at the top,
`nix run 'path:DIR#graft'` for a flake in a subdirectory.

### check

| Rule | Message |
|---|---|
| One marker README, whose first line is the marker | `scions/README.md lacks graft's marker: run graft sync` |
| Every folder in the scions directory is a submodule, and every `.gitmodules` entry under it exists | `scions/foo is not a submodule: graft add URL, or remove it` |
| Every URL is absolute | `relative URL` |
| A top-level git flake declares `inputs.self.submodules = true;` | `add inputs.self.submodules = true;` |
| Every scion is filled | `scions/NAME is empty: graft sync` |
| Every scion is clean: no modified or staged tracked file (untracked files do not count) | `uncommitted edits in scions/NAME: … (Nix builds ignore them while the rest of the tree is clean)` |
| Every scion is checked out at its gitlink | `scions/NAME is checked out at X but the repository records Y: git add scions/NAME to record it, or graft sync to go back` |
| Every gitlink is a version tag's commit | `scions/NAME at abc1234 is not a release tag of URL (nearest: v1.0.0-1-g…): release it, or graft upgrade NAME` |
| graft's workflows match this version's | as in [Files](files.md#grafts-workflows) |

It names each failing scion, and exits 1. If `git ls-remote` cannot read a provider, it checks the
submodule's local tags instead and says `(upstream unreadable: checked local tags)`. On success it ends with
`graft: N scions clean and at release tags`. Each message and its fix:
[`graft check` fails](../guides/check-fails.md).

### outdated

Columns `NAME DIR PINNED LATEST STATUS MAJOR`. `PINNED` is the highest version tag at the gitlink. `STATUS`
is `current`, `outdated`, `untagged` (the gitlink is no version's commit) or `unknown` (the provider's
repository cannot be read). graft's own row comes from `flake.lock`. In `--json`, `major` is `null` when there
is no newer major.

### upgrade

- The target is the newest tag in the pinned major, unless `@TAG` names one. `NAME` without a tag reports any
  newer major that it did not take.
- graft's own pin moves first: graft rewrites the tag in `graft.url`, runs `nix flake update graft`, then the
  new version's `sync` steps for the README and workflows. It moves graft's pin in CI only from `graft ci`.
- A scion: refused when it has uncommitted edits or is checked out away from its gitlink. A filled scion is
  fetched, checked out at the tag's commit and staged. An empty one (in CI) gets its gitlink set directly
  (`git update-index --cacheinfo`), with no clone.
- Never changes `.gitmodules`.
- A `git+` flake build ignores a staged gitlink while the rest of the tree is clean: commit the upgrade on a
  branch before `nix flake check` ([Take a newer version by hand](../guides/upgrade.md)).

### changelog

The same output as before v2.13.0: per provider, `### NAME vOLD -> vNEW`, the **Upgrading** sections first,
then the rest, quoted. It reads the provider's `CHANGELOG.md` at the target tag from the URL in `.gitmodules`.
It fails after it prints if a provider cannot be read.

### migrate

What moved is every gitlink that differs between the merge base and the working tree. The brief has: what
upstream says (each provider's CHANGELOG, quoted); what moved (`git diff --name-status` between the two
commits of each provider, renames and deletions first); references here (lines of your own files, outside
the scions directory and graft's workflows, that name a renamed or deleted `<dir>/<name>/<path>`); what broke
(`nix flake check`, unless `--no-check`). The base is the merge base of `HEAD` with `origin/HEAD`, else with
the first of `origin/main`, `origin/master`, `main`, `master`; `--base REF` names another. Exits 0 when it
prints a brief, also when checks fail.

### ci

- `ci check`: `flake.lock` committed and matching `flake.nix` for graft's input; then the rules of
  [check](#check) that need only committed state (marker, registered, absolute URLs,
  `self.submodules`, gitlinks at version tags, workflows). It does not need filled scions. Then it evaluates
  `packages.<system>.tools`, so that mkFlake's evaluation errors fail CI.
- `ci upgrade`: needs `BRANCH`, and refuses a working tree with changes of its own and a branch other than
  `BRANCH`. Runs `upgrade`, commits, then runs the whole `nix flake check` on the clean committed tree (so Nix
  fetches the new scions), records the result in the commit, and force-pushes `BRANCH`. Fails after the push
  when a check fails.
- `ci pr`: opens or updates the pull request, only when the branch changes nothing but gitlinks of existing
  scions, and, when graft's own pin moved, graft's `flake.nix`/`flake.lock` lines, the marker README and
  graft's marked workflows. Any other change, such as `.gitmodules` (a provider's URL), is refused, with a
  link to the diff.

### Argument errors

- `outdated`, `changelog` and `upgrade`: an argument that the command does not take fails with exit 2, a
  `graft:` line that names it and the command's usage line (`graft: -C needs a repository`,
  `graft: outdated takes no names: it lists graft and every scion`, `graft: upgrade has no --json`,
  `graft: unknown option OPTION`). A bare `--` is skipped.
- `changelog` and `upgrade`: `graft: no scion (or graft input) named NAME`,
  `graft: NAME: TAG is not a version tag`, `graft: NAME: no tag TAG upstream` (exit 1).
- `migrate`: `graft: usage: graft migrate [-C REPO] [--base REF] [--no-check] [--json]` (exit 1).
- `sync` and `check` take no arguments: `graft: sync takes no arguments`, exit 2.
- `ci` with another step: `graft: usage: graft ci check|upgrade|pr`, exit 1.
- An unknown command prints the usage on stderr, and exits 2.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): arguments, discovery, outputs, and the evaluation errors
- [lib/graft.sh](../../lib/graft.sh): commands and usage
- [lib/tools.sh](../../lib/tools.sh): `add`, `outdated`, `changelog`, `upgrade`
- [lib/common.sh](../../lib/common.sh): finding the scions directory, tags with `git ls-remote`
- [lib/migrate.sh](../../lib/migrate.sh), [lib/ci.sh](../../lib/ci.sh)
- [tests/run.sh](../../tests/run.sh)
