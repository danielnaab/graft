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
3. Run the check again. On success it prints `graft: N scions clean and at release tags`, and exits 0.
4. Commit what the fixes changed: `git status --short` names it.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the check passes, nothing is left to commit,
and `nix flake check` passes on the committed tree:

```sh
nix run '.#graft' check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## `graft check` messages

### graft: no scions here (no README.md carrying graft's marker); graft add URL grafts one

Not a failure (exit 0). graft found no `README.md` with its marker line: the repository grafts nothing yet,
or the scions README was deleted. If you have scions, run `nix run .#graft sync`, which writes it again, and
commit `scions/README.md`.

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

### graft: scions/NAME is empty: graft sync (or git submodule update --init -- scions/NAME)

The scion is not filled: a fresh clone or worktree. Run `nix run .#graft sync`, or
`git submodule update --init --recursive`. To make it happen by itself, see
[Clones, worktrees, sandboxes and CI](clones-and-worktrees.md).

### graft: scions/NAME's own submodules are empty (…): graft sync (Nix fails on them while the repo's tree has a modified file)

The provider grafts other repositories, and git did not fill them: `git submodule update --init --recursive` without
`--recursive` leaves them empty. Run `nix run .#graft sync`, or
`git submodule update --init --recursive -- scions/NAME`. Until then, a build with any tracked file modified
fails with `error: Failed to fetch git repository '…/scions/NAME/…'`.

### graft: uncommitted edits in scions/NAME: FILE … (Nix builds ignore them while the rest of the tree is clean)

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

### graft: scions/NAME is checked out at X but the repository records Y: git add scions/NAME to record it, or git submodule update -- scions/NAME to go back

The scion's checkout and the gitlink disagree (`git submodule status` shows `+`). Either someone switched the
scion to another commit, or the gitlink moved (a pull, a merge, `git checkout` without
`submodule.recurse=true`) and the scion did not follow. While this lasts, a build reads the gitlink when the
tree is clean and the checkout when it is not.

- To go back to the recorded pin: `git submodule update -- scions/NAME`. (`graft sync` never moves a filled
  scion.)
- To record the checkout as the new pin: `git add scions/NAME`. Only do this at a release tag, or the next
  message follows. To take a newer release, prefer `nix run .#graft upgrade NAME`.

### graft: scions/NAME at abc1234 is not a release tag of URL (nearest: v1.0.0-1-g…): release it, or graft upgrade NAME

The gitlink is not the commit of any version tag of the provider: an unreleased fix was committed
([Use the fix before its release](fix-a-provider.md#use-the-fix-before-its-release)), or a tag was moved or
deleted upstream. `nearest` is `git describe` of the commit.

- Move the scion to a release by hand: the provider's release of the fix, or the last release before it.
  `graft upgrade` refuses a scion that is not at a release
  (`graft: NAME is at abc1234…, not a release tag; move it by hand (git -C scions/NAME checkout TAG, then git add)`):

  ```
  git -C scions/NAME fetch --tags -q
  git -C scions/NAME checkout -q --detach v1.1.0
  git add scions/NAME
  ```

  Then commit.

If the commit was never pushed to the provider, `nix run .#graft` cannot even evaluate (Nix fails with
`Cannot find Git revision …`, below). Run graft from its own flake instead:
`nix run github:danielnaab/graft/v2.13.0 -- check -C .`.

If the line before says `(upstream unreadable: checked local tags)`, graft could not reach the provider, and
compared with the scion's local tags. Fix the access, and run the check again.

### .forgejo/workflows/FILE differs from the copy this graft version ships

One of graft's workflows was edited, or graft's pin moved without a sync. `nix run .#graft sync` restores
it (`graft: refreshed .forgejo/workflows/FILE`). To keep your edits instead, delete its marker line first
([Keep your own edits to a workflow](../reference/files.md#keep-your-own-edits-to-a-workflow)).

### graft: two scions directories: A, B; graft supports one per repository

Two `README.md` files carry graft's marker line. graft finds the scions directory and the flake by it. Keep
the one beside your flake, and delete the other README (or its folder, if it is a left-over copy-mode
scions directory).

### graft: note: scions/NAME has its own submodules (…); Nix fetches them when evaluating

Not a failure. The provider grafts other repositories, and Nix fetches them too whenever it evaluates your
flake. If one of them is private and you cannot read it, your evaluation fails: ask the provider to drop it
([One level](../decisions.md#one-level)).

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

Nix's own message: the consumer records a scion commit that the provider's remote does not have. Push the
scion's commit to the provider (any branch or tag), or go back to a release:
`git submodule update -- scions/NAME`, then `git add scions/NAME`. `push.recurseSubmodules=check` prevents
this ([Clones, worktrees, sandboxes and CI](clones-and-worktrees.md#set-up-your-machine-once)).

### graft: .gitmodules registers scions/NAME but the flake's source lacks it: add inputs.self.submodules = true; …

Nix gave mkFlake a source without submodules. Either `flake.nix` lacks `inputs.self.submodules = true;`
(add it), Nix is older than 2.27 (upgrade it), or the flake was fetched as `github:OWNER/REPO`, a tarball
with no submodules: use `git+https://…` instead ([Nix behaviours](../reference/nix-behaviours.md)).

### graft: scions/a, scions/b are empty in the flake's source, so the tools leave out what they give: run git submodule update --init (or graft sync)

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

### graft: DIR/README.md lacks graft's marker: is dir right? (mkFlake's dir = "DIR")

`mkFlake`'s `dir` names a folder without the marker README: `dir` is wrong, or the README was deleted or
edited. Correct `dir` in `flake.nix`, or write the README again with `nix run .#graft sync`.

### graft: A needs B, which this repository does not graft: graft add URL

Provider `A`'s `graft.nix` requires provider `B` (it uses B's tools or files). Run the `graft add` command
that the message prints, then commit. If you grafted B under another folder name, rename the folder to B's
name: `requires` finds it by name.

### graft: tools NAME (from A and B): two scions provide the same name

Two providers ship a tool (or app or package) with the same name. `self` in the message is your own
`perSystem`: rename your tool. Neither consumer setting can
choose between them: ask one provider to rename its tool, or stop grafting one of them
(`git rm scions/A`). Two providers that ship the same store path, such as the same `just`, do not
collide.

## Sources

- [lib/graft.sh](../../lib/graft.sh): `check` and its messages.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the evaluation errors.
- [lib/workflows.sh](../../lib/workflows.sh): the workflow comparison.
- [tests/run.sh](../../tests/run.sh): one failing fixture per `check` rule, and each mkFlake error.
- [tests/fixtures.sh](../../tests/fixtures.sh) `check-fails`: the repositories this page was read against.
- [Commands](../reference/commands.md#check).
