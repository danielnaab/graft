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
| `perSystem` | none | The repository's own tools, apps, packages, checks and dev shells: a function `{ pkgs, lib, system, scions, ... }: { tools ? { }; apps; packages; checks; devShells; }`, called once per system and merged like a provider's `graft.nix`, under the name `self` (a collision says `from scion A and the consumer's perSystem`). Its checks are not prefixed: `checks.<system>.NAME`. Its `devShells` are output as they are. Use it to put a tool of your own, such as `bun`, into `tools`. |
| `outputs` | `{ }` | Plain extra outputs, merged last with `recursiveUpdate`, so they add to and override graft's. |

Removed in v2.13.0: `scions` (there is no input list) and `flakeDir` (graft finds the flake as the parent of
the scions directory).

A top-level git flake must declare `inputs.self.submodules = true;` (Nix 2.27 or later); `graft check` fails
without it, unless `flake.nix` carries the comment `# graft: no self.submodules`. A `path:` flake does not
need it.

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
evaluate when a scion is empty or broken, so `nix run .#graft …` can still repair it. A provider whose
`graft.nix` throws is never dropped: `packages.<system>.tools` and `checks.<system>` fail with its error,
under `… while evaluating scions/NAME/graft.nix (graft calls it for the scion NAME)`, and so do
`nix flake check` and `graft check`. Only the names of `apps` and `packages` skip a failing provider, so that
`nix run .#graft` and your own outputs still evaluate.

One failure Nix cannot contain: a `graft.nix` that does not **parse** breaks every output of the flake,
`apps.<system>.graft` too. Run graft from outside to see it and move the scion back:
`nix run github:danielnaab/graft/v2.13.0 -- check -C .` names it
([`graft check` fails](../guides/check-fails.md)).

### Evaluation errors

mkFlake fails evaluation of the outputs that use a broken scion with one of these. Each is thrown inside a
value, so other outputs still evaluate. The fix for each is in
[`graft check` fails](../guides/check-fails.md#nix-evaluation-errors).

- `graft: tools nb-index (from mk and nk): two scions provide the same name`. Also for `apps` and `packages`.
  Providers are named in alphabetical order. Two tools with the same name and the same store path are one
  tool, not a collision. When one side is your `perSystem`:
  `graft: tools lk (from scion lk and the consumer's perSystem): both provide the same name; rename the consumer's own`.
- `graft: nk needs kb, which this repository does not graft: graft add URL`
- `graft: .gitmodules registers scions/a, scions/b but the flake's source lacks them: add inputs.self.submodules = true; to flake.nix (Nix >= 2.27). Fetched with github:? use git+https://…, which keeps submodules.`
  (`lacks it` for one scion.)
- `graft: graft's README is in scions/ but mkFlake's dir is "DIR": set dir = "scions"; (or move the scions)`,
  when a folder at the top of the flake's source holds graft's README and `dir` names another (missing or not).
- `graft: DIR/README.md lacks graft's marker: is dir right? (mkFlake's dir = "DIR")`
- `graft: DIR/README.md is graft's copy-mode README (v2.12 or earlier): graft v2.13.0 pins scions with git submodules; follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md`
- `graft: DIR/NAME is not a submodule: graft add URL, or remove it`
- `graft: DIR/NAME: a scion's folder name must be letters, digits, . _ and - (starting with a letter or digit), and not self`
- `graft: mkFlake's dir must be one folder name (letters, digits, . _ -; not ., .. or .git), got "DIR"`
- `graft: scions/NAME is empty in the flake's source: run git submodule update --init --recursive (or graft sync)`, when a
  provider uses an empty sibling (`scions.NAME`).
- `graft: apps graft (from NAME): that name is graft's; the provider must rename it`, and the same for
  `packages tools`; for `perSystem`, `…; rename the repo's own`.
- `graft: mkFlake's outputs has apps.<system>.graft: that name is graft's; rename the repo's own`

An empty scion is not an error by itself. `packages.<system>.tools` builds without it, and Nix prints a
warning:

```
warning: graft: scions/a, scions/b are empty in the flake's source, so the tools leave out what they give: run git submodule update --init --recursive (or graft sync)
```

followed by `. Fetched with github:? use git+https://…, which keeps submodules.` for a flake at the top. A
`github:` tarball has the scion folders empty, so it gets this warning.

A repository with no scions yet is allowed: `tools` is then an empty environment, and `graft check` warns.

## The graft command

Run it as `nix run .#graft COMMAND`. A flag needs `--` before it, because Nix otherwise reads it:
`nix run .#graft -- outdated --json`. For a flake in a subdirectory, run `nix run 'path:DIR#graft'` from
the repository's top.

Every command except `sync` also runs from graft's own flake, against any checkout:
`nix run github:danielnaab/graft/v2.13.0 -- COMMAND -C REPO`. graft then reads the repository as data, and
never evaluates its flake outputs.

`nix run .#graft` evaluates the repository's flake first, and for a git flake with
`inputs.self.submodules = true` Nix fetches **every** submodule of the repository (not only the scions) at
its gitlink, from its URL, while the tree is clean. A gitlink at a commit the provider's remote lacks (an
unpushed fix) then stops `nix run .#graft` before graft runs (`Cannot find Git revision …`). Use
`nix run github:danielnaab/graft/v2.13.0 -- COMMAND -C .` in that state.

In zsh with `extendedglob` set, `#` is a glob character, so a flake reference that holds one fails with
`zsh: no matches found`. Quote it: `'.#graft'`, `'path:DIR#graft'`. The docs quote every `path:` form, and
write `.#graft` unquoted.

graft finds the scions directory by its marker README, with
`git grep -l -F '<!-- managed by graft: https://github.com/danielnaab/graft -->' -- '*/README.md'`, in the
files that git tracks. `git grep` does not look inside submodules, so a provider's own scions README never
counts, and a top-level `README.md` with the marker is ignored. A repository has at most one:
`graft: two scions directories: A, B; graft supports one per repository`. The flake's directory is the scions
directory's parent; with none, the flake is at the top.

Every command first refuses a copy-mode scions README (one with a `| Scion | Commit |` table or a
`<!-- graft flake:` line):
`graft: scions/README.md is graft's copy-mode README (v2.12 or earlier): graft v2.13.0 pins scions with git submodules; follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md (https://github.com/danielnaab/graft/blob/v2/CHANGELOG.md)`.

| Command | What it does |
|---|---|
| `add [-C REPO] [--scions PATH] URL[@TAG] [NAME]` | Adds a scion at `TAG`, default the newest version upstream. `NAME` defaults to the URL's last segment without `.git`. Runs `git submodule add`, checks out the tag's commit, writes the marker README if it is missing, and stages `.gitmodules`, the gitlink and the README. |
| `sync` (the default) | Fills every empty scion and its own submodules, writes the marker README, and refreshes graft's workflows. Never moves a gitlink, and never moves a filled scion. In the repository's own flake only. |
| `check [-C REPO] [--committed]` | Fails unless every rule in [Check](#check) holds. |
| `outdated [-C REPO] [--json]` | For graft and each scion: the pinned version, the newest version in its major, the status, and the newest later major. |
| `changelog [-C REPO] [--json] [NAME[@TAG]...]` | Each provider's CHANGELOG sections between the pin and the target, **Upgrading** first, or its commits. Changes nothing. |
| `upgrade [-C REPO] [NAME[@TAG]...]` | Moves pins. With no names: graft and every outdated scion, each in its major version. `NAME@TAG` is the only way to cross a major. |
| `migrate [-C REPO] [--base REF] [--no-check] [--json]` | Prints a migration brief for an upgraded branch. |
| `ci check\|upgrade\|pr` | The steps of graft's workflows. See [CI](#ci). |
| `help` | Prints the usage, one line per command. |

There is no `remove`: `git rm scions/NAME` removes a scion, and git edits `.gitmodules`.

### add

- `@TAG` is split off only when the part after the last `@` is a version tag, so `git@host:team/x.git` is one
  URL.
- `NAME` defaults to the URL's last path segment, without `.git`. It must match `[A-Za-z0-9][A-Za-z0-9._-]*`,
  and not be `self`.
- `TAG` defaults to the highest version tag upstream; an annotated tag is taken at its commit.
- The scions directory: the existing one; else `--scions PATH`, from the repository's top, whose parent holds
  a `flake.nix` (`nix/agent-tools/.scions`); else the repository's own app's `dir`, beside its flake; else
  `scions` at the top.
- It runs `git submodule add`, checks out the tag's commit detached, fills the scion's own submodules
  recursively, writes the marker README if it is missing, and stages `.gitmodules`, the gitlink and the
  README. A scion removed with `git rm` can be added again.
- Prints `graft: scions/NAME <- v1.1.0 (abc1234)`, then
  `graft: note: scions/NAME has its own submodules (scions/docs); Nix fetches them when evaluating` when it has
  some, then `graft: next: commit`, or, for a flake at the top that lacks it,
  `graft: next: commit; a git flake also needs inputs.self.submodules = true;`.
- When the scion's `graft.nix` requires a sibling this repository does not graft (read from its text,
  `requires.X = "URL";` or `requires = { X = "URL"; };`), it prints
  `graft: note: NAME requires X, which this repository does not graft: graft add URL`.
- Errors: `graft: scions/NAME is still tracked as files (a copied scion?): git rm -r -q --cached scions/NAME, delete the folder, then graft add again`
  (a path the index still holds as files: `git add` would record the clone as files, not as a submodule);
  `graft: scions/NAME was not recorded as a submodule (git ls-files -s scions/NAME); remove it (git rm -r -q --cached scions/NAME) and try again`;
  `graft: scions/NAME exists`; `graft: a relative URL (../x.git) won't resolve for Nix; give an absolute URL`;
  `graft: URL has no semver tags`; `graft: NAME: no tag TAG upstream`; `graft: can't list tags at URL: …`
  (git's last line); `graft: not inside a git work tree: DIR`;
  `graft: --scions P: no flake.nix in D; the scions directory goes beside the flake that grafts them`;
  `graft: this repository's scions are in X; graft supports one scions directory per repository`.

### sync

Only in the repository's own flake. From graft's flake it says
`graft: sync runs from the repo's own flake (it writes that graft version's files): nix run .#graft sync, or nix run 'path:DIR#graft' sync`.

For each scion: when it is empty, `git submodule update --init`, then its own empty submodules
(recursively). A scion the index holds as files stops it, with the message `check` gives. A filled scion
is never moved, also one on a fix branch or away from its gitlink. One line per scion; the version comes from
the scion's local tags:

```
graft: scions/pa at v1.1.0 (abc1234)
graft: scions/pa at abc1234 (untagged)
graft: scions/pa at v1.0.0 (abc1234), not the recorded def5678: left as it is (git add scions/pa records it; git submodule update --recursive -- scions/pa goes back)
```

Then it writes the marker README (`graft: wrote scions/README.md` when it changed) and refreshes graft's
workflows (`graft: refreshed .forgejo/workflows/FILE`). With no scions:
`graft: no scions here yet: graft add URL`.

The marker README's commands name this flake's run command: `nix run .#graft` for a flake at the top,
`nix run 'path:DIR#graft'` for a flake in a subdirectory.

### check

`check [-C REPO] [--committed]`. Each failure is one `graft:` line on stderr, naming the scion; then it exits 1.

| Rule | Message |
|---|---|
| The repository's own app's `dir` is the scions directory | `graft: graft's README is in scions/ but mkFlake's dir is ".scions": set dir = "scions"; in flake.nix (or move the scions)` (only from `nix run .#graft`) |
| Every folder in the scions directory is a submodule | `graft: scions/foo is not a submodule: graft add URL, or remove it` |
| No `.gitmodules` path is tracked as files (a copied scion left in the index, then `git add`) | `graft: scions/x is tracked as files, not as a submodule: git rm -r -q --cached scions/x && git add scions/x (git then records the submodule's commit)` |
| Every submodule there is registered in `.gitmodules` | `graft: scions/x is a submodule .gitmodules doesn't register: git rm --cached scions/x, then graft add its URL` |
| Every submodule has its folder | `graft: scions/x is a submodule with no folder: git submodule update --init -- scions/x, or git rm scions/x` |
| Every `.gitmodules` entry under the scions directory is a submodule | `graft: .gitmodules registers scions/x, which is not a submodule here: graft add its URL again, or remove its entry (git config -f .gitmodules --remove-section submodule.scions/x)` |
| Every URL is absolute | `graft: scions/x: .gitmodules gives a relative URL (../x.git), which Nix can't resolve: give an absolute one (git config -f .gitmodules submodule.scions/x.url URL)` |
| A top-level flake with scions declares `inputs.self.submodules = true;` (unless it carries the comment `# graft: no self.submodules`) | `graft: flake.nix doesn't declare inputs.self.submodules = true;, so Nix sees no scions: add it (Nix 2.27 or later)` |
| Every scion is filled | `graft: scions/pa is empty: git submodule update --init --recursive -- scions/pa (or graft sync)` |
| Every scion is clean: no modified or staged tracked file (untracked files do not count) | `graft: uncommitted edits in scions/pa: docs/status.md (Nix builds ignore them while the rest of the tree is clean)` |
| Every scion is checked out at its gitlink | `graft: scions/pa is checked out at e3f04b8 but the repository records 9147b3c: git add scions/pa to record it, or git submodule update --recursive -- scions/pa to go back` |
| Every scion's own submodules are filled | `graft: scions/nest's own submodules are empty (scions/docs): git -C scions/nest submodule update --init --recursive (or graft sync); Nix fails on them while the repo's tree has a modified file` |
| Every `graft.nix` parses | `graft: scions/x/graft.nix doesn't evaluate (above): every output of this flake fails until it does, graft's own app too; move the scion back (git submodule update --recursive -- scions/x), or from outside: nix run github:danielnaab/graft/v2.13.0 -- upgrade -C . x@TAG` |
| Every gitlink is a version tag's commit | `graft: scions/pa at ec99ed1 is not a release tag of URL (nearest: v1.0.0-1-gec99ed1): release it, then graft upgrade pa@TAG (or graft upgrade pa, the newest release of its major)` (`nearest` only for a filled scion) |
| … and that tag is upstream | `graft: scions/pa: tag v1.0.1 exists locally but not at URL: push it (git -C scions/pa push origin v1.0.1)` |
| The tools and checks evaluate (from `nix run .#graft` only, after every other rule passed) | `graft: the flake's tools or checks don't evaluate (above): a provider's graft.nix fails` |
| graft's workflows match this version's | as in [Files](files.md#grafts-workflows) |

- `--committed` checks only what is committed: it skips the filled, clean, at-gitlink, nested, parse and
  evaluation rules.
  `graft ci check` uses it.
- If `git ls-remote` cannot read a provider, it says
  `graft: scions/pa: can't list tags at URL: … (upstream unreadable: checked local tags)`, and the scion's
  local tags decide.
- A scion with submodules of its own prints `graft: note: scions/nest has its own submodules (scions/docs); Nix fetches them when evaluating`
  on stdout.
- Success: `graft: N scions clean and at release tags` (`1 scion`); with `--committed`,
  `graft: N scions registered and at release tags`.
- No scions directory: `graft: no scions here (no README.md carrying graft's marker); graft add URL grafts one`,
  exit 0 (unless a workflow fails).

`check` does not compare the marker README's text: the marker line is what finds the scions directory. Each
message and its fix: [`graft check` fails](../guides/check-fails.md).

### outdated

Columns `NAME DIR PINNED LATEST STATUS MAJOR`, then a line explaining `MAJOR` when a row has one:

```
NAME        DIR     PINNED  LATEST  STATUS    MAJOR
graft       -       path    -       untagged  -
page-rules  scions  v1.0.0  v1.1.0  outdated  -
```

`PINNED` is the highest version tag upstream at the gitlink; with none, `STATUS` is `untagged` and `PINNED`
the gitlink's first 12 hex digits, and, for a filled scion with a release behind its gitlink (`git describe`),
`LATEST` is the newest version in that release's major: what `graft upgrade NAME` takes. When the provider cannot be read, `STATUS` is `unknown` and `PINNED` the
scion's local tag, or the commit. graft's own row comes from `flake.lock` (a `path:` graft shows `path`,
`untagged`). In `--json`, `major` is `null` when there is no newer major. With nothing to list:
`graft: no scions here (no README.md carrying graft's marker), and no graft input`.

### upgrade

- The target is the newest tag in the pinned major, unless `@TAG` names one (which may also go down).
  `NAME` without a tag reports a newer major that it did not take:
  `graft: pa: v2.0.0 is a new major version; not taken (graft upgrade pa@v2.0.0)`.
- graft's own pin moves first (`graft: graft v2.13.0 -> v2.13.1`): graft rewrites the tag in `flake.nix`,
  runs `nix flake update graft`, then the new version's `lib/refresh.sh`, which writes the marker README and
  refreshes graft's workflows. It moves graft only to v2.13.0 or later
  (`graft: graft upgrade moves graft only to v2.13.0 or later (submodule scions); move it by hand`), and in
  CI only from `graft ci` (`graft: not moving graft itself from a CI job that isn't graft ci (…)`).
- A filled scion is fetched at the tag from its `.gitmodules` URL, checked out detached, its own submodules
  filled, and staged. An empty one (in CI) gets its gitlink set directly (`git update-index --cacheinfo`),
  with no clone. Prints `graft: pa v1.1.0 -> v1.2.0`. It never changes `.gitmodules`, and never commits.
- An untagged scion (a fix recorded before its release) moves when named: `graft upgrade pa@v1.0.1`, or
  `graft upgrade pa` for the newest version in the major of its nearest release. With no names it is left
  alone. A scion checked out on a fix branch moves too; the branch stays.
- It refuses, before changing anything:
  - `graft: pa: uncommitted edits in scions/pa (FILE); nothing changed: commit them in the provider, or git -C scions/pa checkout -- .`
  - `graft: pa is at <12 hex>, not a release tag, and graft can't tell its major: name the tag (graft upgrade pa@TAG)`
  - `graft: pa: no tag v1.9.0 upstream; nothing changed`
  - `graft: pa: can't tell whether v1.0.0 is current (see above); nothing changed`
  - `graft: no scion (or graft input) named X`
- A git flake build ignores a staged gitlink while the rest of the tree is clean: commit the upgrade on a
  branch before `nix flake check` ([Take a newer version by hand](../guides/upgrade.md)).

### changelog

The same output as before v2.13.0: per provider, `### NAME vOLD -> vNEW`, the **Upgrading** sections first,
then the rest, quoted. It reads the provider's `CHANGELOG.md` at the target tag from the URL in `.gitmodules`.
For a GitHub `https` URL it adds a compare link.
It fails after it prints if a provider cannot be read.

### migrate

What moved is graft (from `flake.lock`) and every scion whose gitlink at the base differs from now (the
scion's checkout when it is filled, else the index); new and removed scions are listed as `(new)` and
`(removed)`. The brief tells the reader not to edit the scions or graft's workflows, nor move the pins, and
that a scion that needs a fix is a provider change. The brief has: what
upstream says (each provider's CHANGELOG, quoted); what moved (`git diff --name-status` between the two
commits of each provider, renames and deletions first); references here (lines of your own files, outside
the scions directory and graft's workflows, that name a renamed or deleted `<dir>/<name>/<path>`); what broke
(`nix flake check`, unless `--no-check`). The base is the merge base of `HEAD` with `origin/HEAD`, else with
the first of `origin/main`, `origin/master`, `main`, `master`; `--base REF` names another. Exits 0 when it
prints a brief, also when checks fail.

### ci

- `ci check`: `flake.lock` committed and matching `flake.nix` for graft's input; then `graft check
  --committed`, which needs no filled scions; then `nix flake check --no-build`, so that the whole flake
  evaluates. Success:
  `graft: ci check passed: flake.lock is committed, every scion is at a release tag, graft's workflows are current, and the flake evaluates`.
  An evaluation failure ends `graft: the flake doesn't evaluate (above)`.
- `ci upgrade`: needs `BRANCH`, and refuses a working tree with changes of its own and a branch other than
  `BRANCH`. Runs `upgrade`, `git add -A`, commits, then runs the whole `nix flake check` on the clean committed
  tree (so Nix fetches the new scions). On a failure it amends the commit with `nix flake check failed: …` and
  the logs' ends. It force-pushes `BRANCH` either way, and then fails if a check failed.
- `ci pr`: opens or updates the pull request only when the branch changes nothing but `flake.nix`,
  `flake.lock`, gitlinks of scions that the default branch's `.gitmodules` registers, and the marker README
  when graft's own pin moved. A change to `.forgejo/` is refused (`… changes CI workflows …`), also when
  graft moved; so is `.gitmodules` and any other file (`… changes files graft upgrade never writes: …`).

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
