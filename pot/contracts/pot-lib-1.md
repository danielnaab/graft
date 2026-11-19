---
status: working
---
# pot-lib/1: kitsFor

| | |
|---|---|
| Version | 1 |
| Producer | pot: [pot/lib/kits-for.nix](../lib/kits-for.nix), the flake output `graft.lib.pot.kitsFor` |
| Reader | graft's mkFlake, once per system |
| Owner | pot |

The only Nix edge between graft and pot. A producer hands pot its kits, each with the file it is written in, and
its tools. pot validates each kit and writes the bundle that a run reads ([pot-kits/1](pot-kits-1.md)). mkFlake
reads no field of a kit.

The attribute is `lib.pot.kitsFor` in graft's flake while pot ships there. After a split it is pot's
`lib.pot.kitsFor`, with the same path below `lib`.

## Shape

```nix
graft.lib.pot.kitsFor {
  pkgs;                # the system's nixpkgs: the bundle's tools and its python come from it
  self ? null;         # the repo kit: { kit = <kit>; file = "flake.nix (perSystem)"; }, or null
  scions ? { };        # scions.<name> = { kit = <kit>; file = ".scions/<name>/graft.nix"; }
  tools ? [ ];         # packages for the bundle's bin/; kitsFor adds the repo kit's own packages
  files ? [ ];         # what the producer has pot protect in every run here
  recipes ? { };       # the producer's own recipes, for kits.just (from v2.18.0)
}                      # → a derivation: packages.<system>.pot-kits, with passthru.wrappers
```

| Argument | Type | Meaning |
|---|---|---|
| `pkgs` | nixpkgs | the system's packages |
| `self` | `{ kit; file; }` or `null` | the repo kit. `kit` is a [kit/1](kit-1.md) value. `file` is a string, where the kit is written, as pot's messages name it. |
| `scions.<name>` | `{ kit; file; }` | each scion's kit, by its folder name |
| `tools` | list of packages | the workspace's tools. mkFlake passes the merged tools and the repository's own `graft`. |
| `files` | list of strings | paths relative to the flake directory, or `**/NAME` anywhere in the repository. mkFlake passes `[ "**/graft.nix" "<dir>" ]`, its scions folder. |
| `recipes` | attrs, default `{ }` | the producer's own recipes, in [kit/1's recipe form](kit-1.md#recipes), written into the same `kits.just` as the kits' recipes. graft's are `outdated`, `upgrade` and `graft-check`. Written into `kits.just` since v2.18.0. Another shape: `pot: kitsFor: recipes.X is "<command> <args>" or { run; description; }`. |

The result has `passthru.wrappers`, the declared commands' wrappers as one package, which mkFlake puts over the
programs in a consumer's `packages.<system>.tools` (keeping the tools without them as
`tools.passthru.unwrapped`), and `passthru.kitsJust`, the text of `kits.just`, a plain string with no store paths.
In v2.17.x `wrappers` was an empty package.

mkFlake's call, for a consumer whose scions folder is `.scions`:

```nix
kitsFor {
  inherit pkgs;
  self = { kit = perSystem.kit; file = "flake.nix (perSystem)"; };
  scions.meeting-kit = { kit = <its graft.nix's kit>; file = ".scions/meeting-kit/graft.nix"; };
  tools = [ <the merged tools> <the repository's graft> ];
  files = [ "**/graft.nix" ".scions" ];
}
```

The derivation fails when a kit is invalid, with pot's message, which names the kit's `file`:
`pot: <file>: <field> is not a kit field (…)`. So a bad kit fails the producer's build, not a run. A malformed
argument fails evaluation: `pot: kitsFor: <self|scions.NAME> is { kit; file; } (file: a string naming where the kit
is written)`.

## Compatibility

- kitsFor takes no `...`: an argument it does not know is Nix's own error. Version 1 adds arguments only with a
  default, so a producer written for an earlier pot keeps evaluating.
- graft's lock pins this library, and a root pins its own pot, so the two pots meet only through
  [pot-kits/1](pot-kits-1.md).
- **graft reads no pot format.** Until v2.17.0, mkFlake built v2.15.0's `graft-kits` from the `kit.json` that
  kitsFor wrote; that bundle is gone, and with it the one place graft read pot's shapes.
- **A kit with `packages` fails the build:** `pot: FILE: packages is tools' old name, gone in v2.17.0: give tools`
  ([kit/1](kit-1.md#compatibility)).

## Sources

- [pot/lib/kits-for.nix](../lib/kits-for.nix): the function, its arguments and the bundle
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the call
