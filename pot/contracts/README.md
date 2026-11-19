---
status: working
---
# pot's contracts

pot, graft's sandbox, meets other parties through seven contracts. A contract gets a version when its producer
and its reader are pinned by different parties: a provider's tag and a consumer's graft, a consumer's graft and a
root's pot, or the pots of two machines that share one grants file. A file that one build both writes and reads
stays internal, so the backend protocol (`launch.json`) and the root manifest have no version.

**Who owns a contract.** pot owns every contract that pot touches. graft owns only the contract that graft alone
reads, `provider/2`. graft emits nothing that pot reads: the bundle a run reads is written by pot's own Nix
library, which graft's mkFlake calls.

| Contract | Version | Producer | Reader | Owner |
|---|---|---|---|---|
| [`provider`](../../docs/reference/provider.md) | 2 (graft's major) | a provider's `graft.nix` | graft's mkFlake | graft |
| [`kit`](kit-1.md) | 1.x (1.0, 1.1 and 1.2) | providers, a consumer's `perSystem.kit`, root kits | pot | pot |
| [`pot-lib`](pot-lib-1.md) | 1 | pot | graft's mkFlake | pot |
| [`pot-kits`](pot-kits-1.md) | 1 | `graft.lib.pot.kitsFor` | pot, at run start | pot |
| [`grants`](grants-1.md) | 1, canonical 1 (frozen) | `pot grant` | pot, on each machine | pot |
| [`run-env`](run-env-1.md) | 1 | pot's backends | graft, kit hooks, any program in a run | pot |
| [`runs`](runs-1.md) | 1 | pot | workmux, Windmill jobs, later grove | pot |

## How the arrows point

An arrow means "depends on, or reads the contract of". pot never points at graft.

```
providers ──provider/2──▶ graft's mkFlake ──pot-lib/1──▶ graft.lib.pot.kitsFor ──writes pot-kits/1──▶ pot
graft check, inside a run ──run-env/1 (pot why URL --json)──▶ pot
pot ──▶ git facts: .gitmodules and gitlinks at HEAD, the tags at a scion's URL
```

## How a contract changes

- **Reader first.** A reader accepts a new version in a release before any producer writes it.
- **Values, not fields.** Within one version, a release may add values: a capability name, a guarantee, a verb,
  a source in `policy.json`. A new field, or a changed meaning, is a new version.
- **A compatibility shim is deprecated in the release that adds it and removed in the next.** v2.16.0 added six;
  v2.17.0 removed them. Each old form now fails with an error that names its replacement:

  | Removed in v2.17.0 | Contract | Use |
  |---|---|---|
  | `packages.<system>.graft-kits`: mkFlake no longer writes it, and pot no longer reads it | `pot-kits/1` | `packages.<system>.pot-kits` (any graft v2.16.0 or later) |
  | `kit.packages`, at the source: kitsFor and mkRoot refuse it. pot still reads it in a bundle that graft v2.16.x built | `kit/1` | `kit.tools` |
  | `--account NAME` | pot's command line | `--instance NAME`, `CMD@NAME` |
  | `defaultAccount` | mkRoot, `programs.pot` | `defaultInstance.<kit>` |
  | the claude-code kit's `account` | the kit's arguments | (nothing: the root's `kits` key names the instance) |
  | the claude-code kit's `provider` and `region` | the kit's arguments | `api = graft.apis.<name>` |

- **pot's split waits for two quiet releases.** pot moves to its own repository only once `kit/1` and
  `pot-kits/1`, its two seams with graft, have held unchanged for two releases, checked after pot's phase 1b.
  v2.17.0 made every addition that v2.18.0's wrapped kit commands need (kit 1.2, and pot-kits/1's new files), so
  that v2.18.0 and v2.19.0 can be those two releases. v2.18.0 changed neither (it filled pot-kits/1's files and
  stated two of them more exactly: the wrapper's fallbacks and `bin/`). v2.19.0, the second, changed neither: its
  `pi` kit is written in kit 1.1's fields. The gate is met; the split itself is out of phase 1b.

## Sources

- [pot/lib/kits-for.nix](../lib/kits-for.nix): `pot-lib/1` and the bundle it writes
- [pot/core/pot.py](../core/pot.py): finding the flake directory, reading the bundle, `pot why --json`
- [pot/core/compose.py](../core/compose.py): kits, the bundle's checks, grants, the policy
- [lib/mk-flake.nix](../../lib/mk-flake.nix): graft's side of `provider/2` and `pot-lib/1`
