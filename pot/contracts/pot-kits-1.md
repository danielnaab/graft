---
status: working
---
# pot-kits/1: a repository's kits

| | |
|---|---|
| Version | 1: the bundle's field `version` |
| Producer | [kitsFor](pot-lib-1.md), as `packages.<system>.pot-kits` of a repository's flake |
| Reader | pot, at run start |
| Owner | pot |

## Shape

```
<out>/kit.json   { "version": 1, "self": <kit> | null, "scions": { "<name>": <kit> }, "protect": [ … ] }
<out>/bin/       the workspace's tools, and the repo kit's tools
<out>/closure    the store paths a run needs, one per line
```

| Field | Type | Meaning |
|---|---|---|
| `version` | number | `1` |
| `self` | a kit, or `null` | the repo kit |
| `scions.<name>` | a kit | each scion's kit, by its folder name |
| `protect` | list of strings | paths that every run here sees read-only: relative to the flake directory, or `**/NAME` anywhere in the repository |

Each kit is a [kit/1](kit-1.md) value, with store paths as strings, plus `file`: a string naming where the kit
is written, which pot's messages show, after the flake directory pot evaluated (`requests.network in the repo kit
(nix/agent-tools/flake.nix, perSystem)`). pot never
builds that name from a convention of its own.

The bundle says nothing about identity. pot establishes each scion kit's identity from `.gitmodules` and the
gitlinks at `HEAD`, and does not trust the bundle's list of scions ([kit/1: Identity](kit-1.md#identity)).

pot checks the bundle when it reads it, and so does kitsFor when it writes it: the version, the fields, each kit
(named by its `file`), and the protect list. A failure stops the run:
`pot (<context>): ~/DIR#pot-kits: kit.json is version 2, and this pot reads version 1: take a pot that reads it,
or run pot host CMD`. Every bundle check ends `: take a pot that reads it, or run pot host CMD`.

## Finding the flake directory

pot finds the flake that holds a repository's kits from git facts alone:

1. It reads the submodule paths in `.gitmodules` at `HEAD`, keeping only those whose path is a gitlink (a commit)
   in `HEAD`'s tree, as for scion kits. A stale `.gitmodules` entry, with no gitlink, decides nothing.
2. For each, it walks up from the submodule's parent folder to the repository's top, and takes the nearest folder
   holding `flake.nix`. So `nix/agent-tools/.scions/meeting-kit` gives `nix/agent-tools`, and
   `scions/knowledge-base-kit` gives the top.
3. When the submodules give different folders, the deepest wins; between equals, the first in `.gitmodules`.
4. **No submodules, or no `flake.nix` above them: no flake repo kit.** pot evaluates nothing in the repository
   and copies nothing of it into the store. The root's `repoKits."<origin>"` still applies.
5. pot asks the flake which bundle it has (`nix eval` of `packages.<system>`), then builds it: always as a `path:`
   flake, with `--no-write-lock-file`, never `git+file:`. A missing `flake.lock` stops the run.
6. **A flake with no `pot-kits`** is read from `graft-kits`, until v2.17.0 ([Compatibility](#compatibility)). A
   flake with neither has no repo kit.
7. Both answers, a built bundle or none, are cached in pot's state (`cache/kits/<tree hash>.json`), by a hash of
   the flake directory's tracked files, its status (untracked contents included) and its submodules' status. pot
   evaluates again only when that hash changes.

pot also protects the flake directory it evaluated, when it is a subdirectory, whatever the bundle says.

## Compatibility

- **A `version` other than 1, or a field this version does not know,** is refused.
- **graft-kits**, v2.15.0's bundle (`kit.json`: `{ version = 1; dir; self; file; scions; }`, no `protect`), is read
  when a flake has no `pot-kits`, so a v2.16.0 root works with a v2.15.0 consumer. pot protects what v2.15.0
  protected for it: `**/graft.nix` and its `dir`. The startup line says so: `~/DIR has graft-kits only: its graft
  predates v2.16.0 (pot reads graft-kits until v2.17.0)`. Deprecated in v2.16.0, removed in v2.17.0.
- **mkFlake still writes graft-kits** in v2.15.0's exact shape beside `pot-kits`, so a v2.15.0 root works with a
  v2.16.0 consumer: each kit as v2.15.0 reads it ([kit/1](kit-1.md#compatibility)). Deprecated in v2.16.0, removed
  in v2.17.0.

## Sources

- [pot/lib/kits-for.nix](../lib/kits-for.nix): writing the bundle
- [pot/core/pot.py](../core/pot.py): `flake_dir`, `repo_kit`, `read_bundle`
- [pot/core/compose.py](../core/compose.py): `validate_bundle`
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `graft-kits`
