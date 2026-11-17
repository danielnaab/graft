---
status: working
---
# provider/2: graft.nix

The contract between a provider and graft: what a provider's `graft.nix` is, and what graft's mkFlake makes of
it. For the steps, see [Write a provider's graft.nix](../guides/write-graft-nix.md). The other contracts, which pot
owns, are listed in [pot's contracts](../../pot/contracts/README.md).

| | |
|---|---|
| Version | 2: graft's major version. A breaking change to this contract is graft v3. |
| Producer | a provider's `graft.nix`, at the top of its repository |
| Reader | graft's mkFlake, in each consumer, once per system |
| Owner | graft |

## Shape

`graft.nix` is a function:

```nix
{ pkgs, lib, system, scion, scions, consumer, kitArgs ? { }, ... }:
{
  requires ? { };     # { <name> = "<url>"; }
  tools    ? { };     # { <name> = <derivation>; }
  apps     ? { };     # { <name> = { type = "app"; program = "…"; }; }
  packages ? { };     # { <name> = <derivation>; }
  checks   ? { };     # { <name> = <derivation>; }
  kit      ? null;    # a kit, for pot; graft hands it on unread
}
```

| Argument | Type | Value |
|---|---|---|
| `pkgs`, `lib`, `system` | nixpkgs, its lib, a system string | the consumer's `inputs.nixpkgs` if it has one, else graft's |
| `scion` | a store path | this scion alone, without its `.git` file |
| `scions` | attrs | every scion of the consumer: its store path as `outPath`, merged with its `graft.nix` result |
| `consumer` | a store path | the consumer flake's source |
| `kitArgs` | attrs | the consumer's `perSystem.kitArgs.<scion>`, or `{ }`. Passed only when the function names it. |

| Attribute | Type | Becomes |
|---|---|---|
| `requires` | attrs of strings | nothing. Each named scion must exist, or evaluation fails naming `graft add URL`. |
| `tools` | attrs of derivations | part of `packages.<system>.tools` |
| `apps` | attrs of apps | `apps.<system>.<name>` |
| `packages` | attrs of derivations | `packages.<system>.<name>` |
| `checks` | attrs of derivations | `checks.<system>.<scion>-<name>` |
| `kit` | a [kit](../../pot/contracts/kit-1.md), or `null` | the scion's kit in `packages.<system>.pot-kits`, which mkFlake has pot's [kitsFor](../../pot/contracts/pot-lib-1.md) write, with the file `<dir>/<scion>/graft.nix` |
| anything else | any | visible to other providers as `scions.<name>.<attr>`; not a flake output |

Reserved names, an evaluation error when a provider or `perSystem` gives them: `apps.graft`, `packages.tools`,
and in both `apps` and `packages`: `sandbox`, `pot`, `pot-kits` and `graft-kits`. The full merge and collision
rules are in [Files](files.md#graftnix).

graft reads no field of `kit`. Its schema, its checks and their messages are pot's: an unknown field fails the
consumer's `pot-kits` build with `pot: <dir>/<scion>/graft.nix: <field> is not a kit field (…)`.

## Compatibility

- **Arguments are added, never removed.** A provider takes `...`, so a later graft can pass more.
- **Every attribute is optional.** A missing one is empty.
- **`sandbox`,** v2.14.0's attribute, is an error that names its replacement:
  ``graft: <file> gives `sandbox`: graft v2.15.0 replaced it with `kit` (pot; see graft's CHANGELOG)``.
- **A kit's format is not this contract's.** It moves with [kit/1](../../pot/contracts/kit-1.md): a provider that
  writes a newer kit needs its consumers' roots on a pot that reads it, not a newer graft.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the arguments, the merge, the reserved names, the call to kitsFor
- [Files: graft.nix](files.md#graftnix)
