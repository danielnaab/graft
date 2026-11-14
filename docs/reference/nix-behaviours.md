---
status: working
---
# Nix behaviours

The Nix and git behaviours that graft depends on or works around, and the versions where they were seen.
Check this page again when you change Nix versions: graft's tests have a regression test for each row marked
**tested**.

Seen with Nix 2.34.6 and git 2.56.0, unless a row says otherwise.

## What a build sees of a scion

For a git flake (`.`, `git+file:`, `git+https:`) with `inputs.self.submodules = true;`:

| Your tree | What Nix builds from |
|---|---|
| Clean: no tracked file modified or staged. Untracked files and changes inside a submodule do not count. | Each submodule **at the gitlink in `HEAD`, fetched from the URL in `.gitmodules`**. Uncommitted edits in a scion, local commits in it, and even a staged gitlink move are ignored. |
| Dirty: any tracked file modified or staged | The working tree, **including each scion's uncommitted edits** and whatever commit it has checked out. |

So the same `nix build` gives two results for one scion edit, depending on an unrelated file. **tested**

More of the same rule:

- **Only files tracked in the scion.** With a dirty parent, Nix reads a scion's working tree but only the files
  that git tracks *in the scion*. A new file there is missing from the build
  (`opening file '/nix/store/…-scion-NAME/FILE': No such file or directory`) until `git -C scions/NAME add FILE`.
  **tested**
- **A stale scion.** After `git pull` without `submodule.recurse`, a scion can be checked out at an older commit
  than the gitlink (`git submodule status` shows `+`). The clean tree builds the gitlink; the same commit with a
  note edited builds the old checkout. `graft check` fails on `+`. **tested**
- **An empty scion and a dirty tree.** A fresh clone builds while it is clean (Nix fetches the scions). Modify
  any tracked file, and Nix reads the empty folder and fails inside its fetcher, before graft can say anything:
  `error: Failed to fetch git repository 'file:///…/scions/NAME'` (or a similar `Failed to fetch` naming the
  scion's path). Fill the scions: `git submodule update --init`. **tested**

The reliable ways:

- **To test a provider edit:** the provider's own tests, inside the scion. For a flake in a subdirectory,
  `nix build 'path:DIR#tools'` always reads the working tree. Never build `path:.` at a repository's top: it
  copies untracked and ignored files into the world-readable store.
- **To check what will merge:** commit, then `nix flake check` on a clean tree.

Other fetch rules, all **tested**:

- A gitlink committed at a commit that the provider's remote does not have fails evaluation:
  `Cannot find Git revision '<sha>' in ref 'refs/heads/main' of repository '<url>'`. Push the commit first,
  to any branch or tag.
- A commit reachable only from a tag, or only from a branch other than the default, is found.
- A fresh clone, or a new worktree, with empty scions builds while its tree is clean: Nix fetches them
  itself. With a tracked file modified, it fails (below).

## Nix 2.27 or later

`inputs.self.submodules` is new in Nix 2.27. Older Nix ignores it, and the next row applies. Check with
`nix --version`. The `nixos/nix:latest` image that graft's workflows run on has 2.27 or later; to pin it,
map the runner label `nixos-latest` to an image with a fixed tag, such as `docker://nixos/nix:2.34.6`.

## Without `inputs.self.submodules`, a scion is absent

A git flake without `inputs.self.submodules = true;` has no `scions/NAME` in its source at all: not empty,
absent. Discovery by folder would then find no scions and build an empty `tools`. mkFlake reads `.gitmodules`,
which is a tracked file and always present, and fails:
`graft: .gitmodules registers scions/NAME but the flake's source lacks it: add inputs.self.submodules = true; …`.
**tested**

## `github:` drops submodules; `git+https:` keeps them

A flake fetched as `github:OWNER/REPO` (a tarball) has no submodules, even when it declares
`inputs.self.submodules`. A `git+https://…` reference keeps them: Nix adds `submodules=1` to the locked URL by
itself for a flake that declares it. To use a consumer's outputs from elsewhere (another flake's input, or
`nix run`), use `git+https:`. **tested**

A `github:` tarball has the submodule folders, empty. mkFlake then warns that the scions are empty, and
builds `tools` without them.

graft's own flake does not declare `inputs.self.submodules`: Nix 2.34 refuses that attribute when the flake
is used as a `path:` input (`error: input attribute 'submodules' not supported by scheme 'path'`). To check
graft with its own scions: `nix flake check '.?submodules=1'`.

## Nested submodules

Nix fetches a scion's own submodules, recursively, and ignores `update = none` and `active = false` in
`.gitmodules`. A nested submodule whose URL the consumer cannot read fails the consumer's evaluation:
`error: Failed to fetch git repository 'ssh://…'`. **tested**

git itself does not fill nested submodules: `git submodule update --init` (graft's `sync`), and a checkout
with `submodule.recurse=true`, leave them empty. So a scion's own scions are empty on disk, present in a git
flake's build, and empty in a `path:` flake's build. A provider's `graft.nix` must never read its own scions. See [One level](../decisions.md#one-level).

## `path:` flakes in a subdirectory

A `path:` flake copies its directory's working tree, including filled submodules under it, their uncommitted
edits, and each submodule's `.git` file. A scion must be under the flake's directory. After a fresh clone the
scions are empty, and mkFlake warns `graft: … are empty in the flake's source, so the tools leave out what they give: run git submodule update --init (or graft sync)`.
**tested**

Nix 2.24 does not write the lock of a relative `path:` flake (`cannot fetch input … because it uses a relative
path`); Nix 2.34 does. Lock with the absolute path, `nix flake lock "path:$PWD/DIR"`, to work on both.

## `exportIgnore` and `submodules` together

`error: exportIgnore and submodules are not supported together yet` (`builtins.fetchTree`, 2.34). An input
that a flake fetches with `?exportIgnore=1` therefore cannot see a scion. A flake that needs another
repository's scion takes that provider as an input of its own instead. **tested**

## A whole-source path drags in the whole consumer

Interpolating a sub-path of `inputs.self` (`"${inputs.self}/scions/kb"`) makes a store path that depends on
the whole consumer source, so every tool that uses it rebuilds on every consumer commit. mkFlake copies each
scion alone with `builtins.path` (filtering its `.git` file). A tool's store path then stays the same across
consumer commits. In a `graft.nix`, use `scion` and `scions.NAME` for a scion's files, never `consumer`. **tested**

## A flake at the repository's top copies every tracked file

A git flake's source is every file that git tracks, with uncommitted edits, copied into the world-readable
Nix store on every evaluation. Nix 2.24 and 2.34 have no lazy trees. See
[Keep private files out of the Nix store](../guides/flake-in-a-subdirectory.md).

## Sources

- [tests/run.sh](../../tests/run.sh): the Nix semantics regression tests (clean parent ignores a scion edit;
  dirty parent uses it; an unpushed gitlink fails; a tag-only commit is fetched; `path:` uses the working
  tree), discovery without `self.submodules`, nested scions, and scion isolation.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the `.gitmodules` cross-check, the empty-scion error, and the
  per-scion `builtins.path`.
- The v2.13.0 design's prototypes P1 to P12 (Nix 2.34.6, git 2.56.0), from which the tests were written.
