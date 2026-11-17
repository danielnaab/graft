---
status: working
---
# `graft check` fails

Find the message that `graft check`, `graft ci check` or a Nix build printed, and fix what it names. Each
heading below is one message.

## Before you begin

- Run the check from the repository's top: `nix run .#graft check`. For a flake in a subdirectory,
  `nix run 'path:DIR#graft' check`. In zsh, quote flake references
  ([The graft command](../reference/commands.md#the-graft-command)).
- It prints one `graft:` line per problem, naming each scion, and exits 1. Fix them in the order printed.
- Read your scions directory for `scions/` below.
- If a scion has edits that someone meant, keep them before you go back to the pin: they are a provider fix
  ([Fix a provider from a consumer](fix-a-provider.md)).

## Steps

1. Run the check, and find each message below.
2. Do the fix under it.
3. Run the check again. On success it prints `graft: N scions clean and at release tags` (`1 scion` for
   one), and exits 0.
4. Commit what the fixes changed: `git status --short` names it.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the check passes, nothing is left to commit,
and `nix flake check` passes on the committed tree:

```sh
nix run '.#graft' check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## `graft check` messages

### graft: graft try is on (.graft-try): Nix builds read the working tree, scions included: graft try --stop turns it off

`graft try` is on: someone (or an agent) is building the repository with a scion's uncommitted edits
([Fix a provider from a consumer](fix-a-provider.md#steps), step 3). Its marker, `.graft-try`, must not be
committed. When the trial is done, turn it off from the repository's top:

```
nix run .#graft -- try --stop
```

It removes `.graft-try`; the scions' edits stay. Then deal with them as the messages below say.
`graft upgrade` refuses while try is on, with `graft: graft try is on; nothing changed: graft try --stop first`,
and `git stash` fails (`error: Entry '.graft-try' not uptodate. Cannot merge.`).

### graft: .graft-try is committed (graft try's marker): graft try --stop, then commit its removal

A commit holds `.graft-try`: `git commit -a`, or `git add -A` (or `git add .`) then a commit, takes a file
marked with intent to add. In CI this is `graft ci check`'s failure. Run `nix run .#graft -- try --stop` (it stages the removal:
`graft: .graft-try was committed: its removal is staged; commit it`), then commit. If the commit also moved a
scion to an unreleased commit, the release-tag message below says so.

### graft: .graft-try is left from graft try but untracked, so Nix builds read the recorded scions: …

A `git reset` of the repository unmarked try's marker, so builds no longer read the scions' working trees.
`nix run .#graft -- try` marks it again; `nix run .#graft -- try --stop` removes it.

### graft: no scions here (no README.md carrying graft's marker); graft add URL grafts one

Not a failure (exit 0). graft found no `README.md` with its marker line: the repository grafts nothing yet,
or the scions README was deleted. If you have scions, run `nix run .#graft sync`, which writes it again, and
commit `scions/README.md`.

### graft: graft's README is in scions/ but mkFlake's dir is "DIR": set dir = "scions"; …

`mkFlake`'s `dir` in `flake.nix` names another folder than the one holding your scions, so Nix would build
without them. Set `dir` to the folder the message names (or leave `dir` out for `scions`), and commit. mkFlake
gives the same message when Nix evaluates the tools.

### graft: scions/x is tracked as files, not as a submodule: …

`.gitmodules` registers `scions/x`, but git records it as ordinary files: `git add` ran over a clone while the
path was still in the index, typically a copied v2.12 scion removed with `rm` but not with `git rm --cached`.
Nix then builds those files, never the provider's release, and the scion cannot be upgraded.

If the folder is a clone (`test -e scions/x/.git` succeeds), do what the message says:

```
git rm -r -q --cached scions/x
git add scions/x
git ls-files -s scions/x     # one line, mode 160000
```

If it holds only files (no `.git`), `git add` would record the files again. Remove them, and graft the
provider afresh at the version you want, with the URL that `.gitmodules` gives:

```
git config -f .gitmodules submodule.scions/x.url
git rm -r -q --cached scions/x && rm -rf scions/x
nix run github:danielnaab/graft/v2.14.0 -- add -C . URL@TAG x
git ls-files -s scions/x     # one line, mode 160000
```

Then commit. `graft add` refuses to graft over such a path until it leaves the index, and `sync` stops on it.

### graft: scions/foo is not a submodule: graft add URL, or remove it

A folder in the scions directory is not a submodule: files committed there by hand, or a copy-mode scion left
over from v2.12.0. If it is a provider, `git rm -r scions/foo`, then `nix run .#graft add URL`. If it is not,
move it out of the scions directory.

### graft: scions/x is a submodule .gitmodules doesn't register: …

The tree has a gitlink at `scions/x`, but `.gitmodules` has no entry for it, so nobody can fetch it. Do what
the message says: `git rm --cached scions/x`, then `nix run .#graft add URL`.

### graft: scions/x is a submodule with no folder: …

The tree records the submodule, but its folder is gone from the working tree. Fill it,
`git submodule update --init --recursive -- scions/x`, or remove the scion, `git rm scions/x`.

### graft: .gitmodules registers scions/x, which is not a submodule here: …

A `.gitmodules` entry is left over from a scion that was removed without `git rm`. Remove the entry,
`git config -f .gitmodules --remove-section submodule.scions/x`, or graft it again with `graft add`. Commit
`.gitmodules`.

### graft: scions/x: .gitmodules gives a relative URL (../x.git), which Nix can't resolve: …

git resolves a relative URL from your remote, but Nix cannot. Write the absolute URL:

```
git config -f .gitmodules submodule.scions/x.url https://forge.example/team/x.git
git submodule sync -- scions/x
```

Then commit `.gitmodules`.

### graft: flake.nix doesn't declare inputs.self.submodules = true;, so Nix sees no scions: add it (Nix 2.27 or later)

A git flake at the repository's top leaves scions out of its source without this line. Add
`inputs.self.submodules = true;` to `flake.nix`, and commit. If your flake deliberately reads no scions (it
only grafts docs, say), put the comment `# graft: no self.submodules` in `flake.nix` instead.

### graft: scions/NAME is empty: git submodule update --init --recursive -- scions/NAME (or graft sync)

The scion is not filled: a fresh clone or worktree. Run `git submodule update --init --recursive` first: it
always works, while `nix run .#graft sync` needs the flake to evaluate, which a fresh clone with a modified
tracked file cannot (below). To make it happen by itself, see
[Clones, worktrees, sandboxes and CI](clones-and-worktrees.md).

### graft: scions/NAME's own submodules are empty (…): git -C scions/NAME submodule update --init --recursive (or graft sync); …

The provider grafts other repositories, and git did not fill them: `git submodule update --init` without
`--recursive` leaves them empty. Run `git -C scions/NAME submodule update --init --recursive`, or
`nix run .#graft sync`. Until then, a build with any tracked file modified
fails with `error: Failed to fetch git repository '…/scions/NAME/…'`.

### graft: uncommitted edits in scions/NAME: FILE … (Nix builds ignore them while the rest of the tree is clean)

(The end reads `(graft try is on: Nix builds use them)` while try is on, and `(a path: flake's builds use
them)` for a flake in a subdirectory, which always builds the working tree.)

Someone edited a provider's tracked files in the scion, or staged a change there. A formatter or a linter's
`--fix` may have done it. New untracked files, such as `__pycache__/`, do not count. Look:

```
git -C scions/NAME status --short
git -C scions/NAME diff
```

- **A provider fix that someone meant:** follow [Fix a provider from a consumer](fix-a-provider.md) from
  step 1. The edit stays in the scion until you push it.
- **Anything else:** discard it with `git -C scions/NAME reset -q --hard`. Then stop the cause: tell each
  tool that the scions directory is not yours.

  | Tool | Add |
  |---|---|
  | ruff | `extend-exclude = ["scions"]` and `force-exclude = true`, under `[tool.ruff]` in `pyproject.toml` |
  | black | `extend-exclude = "^/scions/"` and `force-exclude = "^/scions/"`, under `[tool.black]` |
  | pre-commit | `exclude: ^scions/` at the top of `.pre-commit-config.yaml` |
  | Prettier | `scions/` in `.prettierignore` |
  | treefmt | `excludes = ["scions/**"]` under `[global]` in `treefmt.toml` |
  | An agent | the `AGENTS.md` section in [Graft your first pattern](graft-a-repository.md#steps), step 6 |

### graft: scions/NAME is checked out at X but the repository records Y: git add scions/NAME to record it, or git submodule update --recursive -- scions/NAME to go back

The scion's checkout and the gitlink disagree (`git submodule status` shows `+`). Either someone switched the
scion to another commit, or the gitlink moved (a pull, a merge, `git checkout` without
`submodule.recurse=true`) and the scion did not follow. While this lasts, a build reads the gitlink when the
tree is clean and the checkout when it is not.

- To go back to the recorded pin: `git submodule update --recursive -- scions/NAME`. (`graft sync` never
  moves a filled scion.) Commit or stash edits in the scion first.
- To record the checkout as the new pin: `git add scions/NAME`. Only do this at a release tag, or the next
  message follows. To take a newer release, prefer `nix run .#graft upgrade NAME`.

### graft: scions/NAME at abc1234 is not a release tag of URL (nearest: v1.0.0-1-g…): release it, then graft upgrade NAME@TAG …

The gitlink is not the commit of any version tag of the provider: an unreleased fix was committed
([Use the fix before its release](fix-a-provider.md#use-the-fix-before-its-release)), or a tag was moved or
deleted upstream. `nearest` is `git describe` of the commit.

- Once the provider has released the fix: `nix run .#graft upgrade NAME@v1.0.1` (that release), or
  `nix run .#graft upgrade NAME` (the newest release in the major of the nearest tag). Then commit.
- To go back to the last release instead: `nix run .#graft upgrade NAME@v1.0.0`, and commit.

If the scion's commit was never pushed to the provider, `nix run .#graft` cannot even evaluate (Nix fails with
`Cannot find Git revision …`, below). Run graft from its own flake instead:
`nix run github:danielnaab/graft/v2.14.0 -- check -C .`.

If a line before it says `(upstream unreadable: checked local tags)`, graft could not reach the provider, and
compared with the scion's local tags. Fix the access, and run the check again. Inside a `pot` run, the line is
`graft: upstream tags are not reachable from this sandbox run: checked each scion's local tags`: run
`graft check` on the host to compare with upstream ([Commands](../reference/commands.md#check)).

### graft: scions/NAME: tag vX.Y.Z exists locally but not at URL: push it …

The scion's commit carries a release tag in your clone only: the provider's release was tagged but never
pushed. Push it, `git -C scions/NAME push origin vX.Y.Z`, and run the check again. Until then, Nix (which
fetches from the URL) cannot fetch that commit unless a pushed branch holds it.

### graft: scions/NAME/graft.nix doesn't evaluate (above): every output of this flake fails until it does, …

The provider's `graft.nix` does not parse (the lines above show Nix's error). Nix cannot contain a parse
error, so every output of the flake fails, `nix run .#graft` too: `check` prints this only when run from
graft's own flake, `nix run github:danielnaab/graft/v2.14.0 -- check -C .`. Move the scion back to a release
that parses: `git submodule update --recursive -- scions/NAME` (back to the recorded pin, if you edited it),
or `nix run github:danielnaab/graft/v2.14.0 -- upgrade -C . NAME@TAG` (an earlier release). Tell the
provider.

### graft: the flake's tools or checks don't evaluate (above): a provider's graft.nix fails

Every rule passed, but evaluating the flake's tools or checks failed; the lines above name the provider
(`… while evaluating scions/NAME/graft.nix`) and its error. graft never leaves a failing provider out. Fix it in
the provider ([Fix a provider from a consumer](fix-a-provider.md)), or move the scion back to a release that
works: `nix run .#graft upgrade NAME@TAG`.

### .forgejo/workflows/FILE differs from the copy this graft version ships

One of graft's workflows was edited, or graft's pin moved without a sync. `nix run .#graft sync` restores
it (`graft: refreshed .forgejo/workflows/FILE`). To keep your edits instead, delete its marker line first
([Keep your own edits to a workflow](../reference/files.md#keep-your-own-edits-to-a-workflow)).

### graft: two scions directories: A, B; graft supports one per repository

Two `README.md` files carry graft's marker line. graft finds the scions directory and the flake by it. Keep
the one beside your flake, and delete the other README (or its folder, if it is a left-over copy-mode
scions directory).

### graft: note: scions/NAME has its own submodules (…)

Not a failure. The provider grafts other repositories. graft's `add`, `sync` and `upgrade` fill them on
disk, and a git flake on a clean tree fetches them from their URLs as well. If one of them is private and you
cannot read it, your evaluation fails: ask the provider to drop it ([One level](../decisions.md#one-level)).

## Nix evaluation errors

mkFlake raises these when Nix evaluates `packages`, `apps`, `checks` or `devShells`: in `nix build`,
`nix flake check`, `nix run .#…`, and `graft ci check`.

### error: Failed to fetch git repository '…/scions/NAME'

Nix's own message, before graft can say anything: the scion is empty (a fresh clone or worktree) and a
tracked file is modified, so Nix reads the working tree and finds nothing to read. The same happens for an
empty submodule *inside* a scion (`…/scions/NAME/scions/OTHER`). Fill the scions with git:

```
git submodule update --init --recursive
```

`nix run .#graft sync` may not evaluate here; git always works. A clean clone builds without this, because
Nix then fetches the scions itself ([Nix behaviours](../reference/nix-behaviours.md#what-a-build-sees-of-a-scion)).

### Cannot find Git revision '<sha>' in ref 'refs/heads/main' of repository '<url>'

Nix's own message: the consumer records a scion commit that the provider's remote does not have. On a
clean tree `nix run .#graft …` fetches every scion before graft runs, so it fails here too: run graft from
its own flake, `nix run github:danielnaab/graft/v2.14.0 -- check -C .`. The same happens to `graft add` of a
provider whose own pinned submodules are not pushed yet (before the provider's release is pushed): push
first. Push the scion's commit to the provider (any branch or tag), or go back to a release:
`git submodule update --recursive -- scions/NAME`, then `git add scions/NAME`. `push.recurseSubmodules=check` prevents
this ([Clones, worktrees, sandboxes and CI](clones-and-worktrees.md#set-up-your-machine-once)).

### graft: .gitmodules registers scions/NAME but the flake's source lacks it: add inputs.self.submodules = true; …

Nix gave mkFlake a source without submodules. Either `flake.nix` lacks `inputs.self.submodules = true;`
(add it), Nix is older than 2.27 (upgrade it), or the flake was fetched as `github:OWNER/REPO`, a tarball
with no submodules: use `git+https://…` instead ([Nix behaviours](../reference/nix-behaviours.md)).

### graft: scions/a, scions/b are empty in the flake's source, so the tools leave out what they give: run git submodule update --init --recursive (or graft sync)

A warning: `tools` built without those scions. A `path:` flake reads the working tree, and the scions are not
filled; or the flake was fetched as `github:`, which has no submodules (use `git+https://…`). The same cause
fails a provider that needs the empty scion: `graft: scions/NAME is empty in the flake's source: …`. Run either command from the
repository's top; `git submodule update --init --recursive` always works ([Clones, worktrees, sandboxes and CI](clones-and-worktrees.md)).

### graft: DIR/README.md is graft's copy-mode README (v2.12 or earlier): …

Every command and mkFlake say this. The repository is still in copy mode, and graft's pin moved to v2.13.0.
The message ends `… follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md (https://github.com/danielnaab/graft/blob/v2/CHANGELOG.md)`. Follow
[Move from copy mode](move-from-copy-mode.md).

### graft: DIR/NAME: a scion's folder name must be letters, digits, . _ and - …

Rename the submodule's folder: `git mv scions/OLD scions/NEW`. `self` is reserved for your own `perSystem`.

### graft: mkFlake's dir must be one folder name …

`dir` is a single folder name in the flake's directory, such as `".scions"`, not a path.

### graft: graft's README is in scions/ but mkFlake's dir is "DIR": set dir = "scions"; (or move the scions)

The same fix as for this message from `graft check`, above: set `dir` to the folder the message names.

### graft: DIR/README.md lacks graft's marker: is dir right? (mkFlake's dir = "DIR")

`mkFlake`'s `dir` names a folder without the marker README: `dir` is wrong, or the README was deleted or
edited. Correct `dir` in `flake.nix`, or write the README again with `nix run .#graft sync`.

### graft: A needs B, which this repository does not graft: graft add URL

Provider `A`'s `graft.nix` requires provider `B` (it uses B's tools or files). Run the `graft add` command
that the message prints, then commit. If you grafted B under another folder name, rename the folder to B's
name: `requires` finds it by name.

### error: … while evaluating scions/NAME/graft.nix (graft calls it for the scion NAME)

The provider's `graft.nix` threw: the line after names its error. It fails `packages.<system>.tools`,
`checks` and `nix flake check`; `nix run .#graft` still works. Fix it as for
[the flake's tools or checks don't evaluate](#graft-the-flakes-tools-or-checks-dont-evaluate-above-a-providers-graftnix-fails).

### graft: tools NAME (from A and B): two scions provide the same name

Two providers ship a tool (or app or package) with the same name. When one side is your own `perSystem` the
message says `(from scion A and the consumer's perSystem): both provide the same name; rename the consumer's own`:
rename your tool. Neither consumer setting can
choose between them: ask one provider to rename its tool, or stop grafting one of them
(`git rm scions/A`). Two providers that ship the same store path, such as the same `just`, do not
collide.

## Sources

- [lib/scions.sh](../../lib/scions.sh): `check` and its messages.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the evaluation errors.
- [lib/workflows.sh](../../lib/workflows.sh): the workflow comparison.
- [tests/run.sh](../../tests/run.sh): one failing fixture per `check` rule, and each mkFlake error.
- [tests/fixtures.sh](../../tests/fixtures.sh) `check-fails`: the repositories this page was read against.
- [Commands](../reference/commands.md#check).
