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
<out>/kits.just                the generated recipes (from v2.18.0)
<out>/wrappers/bin/<command>   each declared command's wrapper (from v2.18.0)
<out>/commands/<command>.json  each wrapper's manifest (from v2.18.0)
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

**The bundle's other files** are fixed now and built from v2.18.0, so that this contract does not change again
before pot's split:

| File | Contents |
|---|---|
| `kits.just` | the just recipes generated from every kit's [`recipes`](kit-1.md#recipes): graft's fixed header (no settings, no assignments, no backticks), then one line per recipe, `exec "{{tools}}/bin/<command>" <args> "$@"`. `graft sync` copies it to the scions folder; `graft check` compares the bytes. |
| `wrappers/bin/<command>` | each declared command's wrapper, which runs `pot run --command NAME --manifest <out>/commands/NAME.json -- ARGS`, with the `pot` that `~/.config/pot/launcher` names (the home found from the password database, never `$HOME`), or the default root's |
| `commands/<command>.json` | `{ "version": 1, "command": "<command>", "kit": "self" \| "scions.<name>" }`; the bundle is the file's `../..` |

pot never takes a kit's identity from a manifest: it establishes it from the workspace, as for every kit.

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
6. **A flake with no `pot-kits`** has no repo kit. One with v2.15.0's `graft-kits` alone is refused
   ([Compatibility](#compatibility)).
7. In clone mode, the bundle is built from the clone's flake directory at the base commit, never from the
   person's working tree.
8. Both answers, a built bundle or none, are cached in pot's state (`cache/kits/<tree hash>.json`), by a hash of
   the flake directory's tracked files, its status (untracked contents included) and its submodules' status. pot
   evaluates again only when that hash changes.

pot also protects the flake directory it evaluated, when it is a subdirectory, whatever the bundle says.

## Compatibility

- **Only `kit.json`'s fields are versioned.** A `version` other than 1, or a field of `kit.json` that this version
  does not know, is refused. **A reader ignores files in the bundle that it does not know.** So the three files
  above, new in v2.17.0's shape, leave the bundle readable by a v2.16.x pot, which never opens them.
- **Kits in an older bundle.** A bundle that graft v2.16.x built may hold kits with `packages`; pot reads it as
  `tools`, with no warning ([kit/1](kit-1.md#compatibility)).
- **graft-kits is gone.** v2.15.0's bundle was deprecated in v2.16.0. Since v2.17.0, mkFlake no longer writes it and
  pot no longer reads it. A flake with `graft-kits` but no `pot-kits` is refused, never run without its
  protections: `pot (work): ~/DIR has graft-kits only: its graft predates v2.16.0, and this pot reads pot-kits: take
  graft v2.16.0 or later there (nix flake update graft), or run pot host CMD`.
  `graft-kits` stays a name graft reserves in `apps` and `packages`.

## Sources

- [pot/lib/kits-for.nix](../lib/kits-for.nix): writing the bundle
- [pot/core/pot.py](../core/pot.py): `flake_dir`, `repo_kit`, `read_bundle`
- [pot/core/compose.py](../core/compose.py): `validate_bundle`
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the call to kitsFor
