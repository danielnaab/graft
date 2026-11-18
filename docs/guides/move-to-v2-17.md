---
status: working
---
# Move to v2.17.0

graft v2.17.0 adds pot's [dispatch](dispatch.md) (work in a clone, then review and fetch its commits), `pot runs`,
`pot doctor`, and `pot grant` with no argument and with `--commit`. It removes the six forms that v2.16.0
deprecated (seven rows below: `defaultAccount` has one for mkRoot and one for the home-manager module). This page names each removed form, its replacement and its new error, and who needs to change nothing.

## When to use this

- You have a root (`homeManagerModules.pot` or `graft.lib.mkRoot`) on graft v2.16.x.
- Or a consumer or a provider that you are moving to graft v2.17.0.

## Who changes nothing

- **A root already on the new names:** `defaultInstance.<kit>`, the claude-code kit's `api`, no `account`
  argument, and `CMD@NAME` or `--instance` on command lines. A v2.16.x build with no evaluation warning from graft
  is such a root.
- **A consumer still on graft v2.16.0, under a v2.17.0 root.** Its `pot-kits` bundle reads as before, `packages`
  included. You move its graft when you choose.
- **Your grants.** Taking v2.17.0 in the root asks for no grant again: every existing grant's source still holds.
  One caveat: pot learns a repo kit as built at the first v2.17.0 run whose tree equals its grant's. If that
  repository has moved past the granted tree before its first v2.17.0 run, it asks once (`pot grant NAME`).
  A consumer that later takes v2.17.0 changes its `flake.lock`, so it asks once for its own repo kit
  (`pot grant NAME`), and for no other kit.

## What is removed

| Removed | Use instead | What v2.17.0 says |
|---|---|---|
| `packages.<system>.graft-kits` (mkFlake no longer writes it; pot no longer reads it) | `packages.<system>.pot-kits`, which any graft v2.16.0 or later writes | A flake with `graft-kits` only: `pot (work): ~/DIR has graft-kits only: its graft predates v2.16.0, and this pot reads pot-kits: take graft v2.16.0 or later there (nix flake update graft), or run pot host CMD` |
| `kit.packages`, in a kit that graft builds | `kit.tools` | `pot: FILE: packages is tools' old name, gone in v2.17.0: give tools`; for a root's kit, `pot: the root (programs.pot.kits.NAME): packages is tools' old name, gone in v2.17.0: give tools` |
| `pot --account NAME` | `pot CMD@NAME`, or `pot --instance NAME CMD` | `pot: --account is gone (v2.17.0): --instance NAME, or CMD@NAME` (exit 2) |
| `defaultAccount` (mkRoot) | `defaultInstance.<kit> = "NAME";` | `graft.lib.mkRoot: defaultAccount is gone (v2.17.0): defaultInstance.<kit> = "NAME"` |
| `programs.pot.defaultAccount` | `programs.pot.defaultInstance.<kit> = "NAME";` | home-manager's ``The option definition `programs.pot.defaultAccount' in `FILE' no longer has any effect; please remove it.``, then `Use programs.pot.defaultInstance.<kit> = "NAME" (v2.17.0).` |
| the claude-code kit's `account` | nothing: the root's `kits` key names the instance | `graft.kits.claude-code: account is gone (v2.17.0): the instance is the root's kits key ("claude-code@NAME")` |
| the claude-code kit's `provider` and `region` | `api = graft.apis.bedrock { region = "…"; }`, or `graft.apis.anthropic` | `graft.kits.claude-code: provider and region are gone (v2.17.0): api = graft.apis.bedrock { region = "…"; } (or graft.apis.anthropic)` |

pot still reads `packages` in a bundle that graft v2.16.x built, with no warning: the removal is where a kit is
built, so nothing that already works stops.

## Move a root

1. Replace each removed form. Before:

   ```nix
   programs.pot = {
     kits."claude-code@personal" = inputs.graft.kits.claude-code { inherit pkgs; };
     kits."claude-code@flexion-bedrock" = inputs.graft.kits.claude-code {
       inherit pkgs; account = "flexion"; configDir = ".claude-accounts/flexion";
       provider = "bedrock"; region = "us-east-2";
     };
     defaultAccount = "personal";
   };
   ```

   After:

   ```nix
   programs.pot = {
     kits."claude-code@personal" = inputs.graft.kits.claude-code { inherit pkgs; };
     kits."claude-code@flexion-bedrock" = inputs.graft.kits.claude-code {
       inherit pkgs; configDir = ".claude-accounts/flexion";
       api = inputs.graft.apis.bedrock { region = "us-east-2"; };
     };
     defaultInstance.claude-code = "personal";
   };
   ```

   With mkRoot, the same arguments go to `graft.lib.mkRoot`.
2. Replace `--account NAME` in scripts and workmux agent commands: `pot --account flexion claude` becomes
   `pot claude@flexion`.
3. Move the root's `graft` input to v2.17.0 and update its lock:

   ```nix
   inputs.graft.url = "github:danielnaab/graft/v2.17.0";
   ```

   ```
   nix flake update graft
   ```

4. Build the root (`home-manager switch`, or rebuild your mkRoot flake). Each removed form that is left stops the
   evaluation with its error, one at a time: fix it and build again until the build passes.
5. Optionally, add the new root options: `limits.memoryMax`, `limits.tasksMax`, `clones.keepDays` and
   `clones.maxSize` ([pot](../reference/pot.md#home-manager-options)). The defaults suit most machines.

## Move a consumer

1. In its own `perSystem.kit`, rename `packages` to `tools`.
2. Move its `graft` input to v2.17.0 and update its lock. In flexion-assistant, that is
   `nix/agent-tools/flake.nix`:

   ```
   cd nix/agent-tools && nix flake update graft
   ```

3. Commit the flake and its lock, then run `pot grant flexion-assistant` once, at a terminal: the new `flake.lock`
   changes the repo kit's source.

## Move a provider

Rename `packages` to `tools` in your `graft.nix` kit, and release a tag. A consumer on graft v2.17.0 cannot build a
kit with `packages`. A consumer still on v2.15.0 cannot read `tools`, so say in the release's CHANGELOG that it needs
graft v2.16.0 or later.

## Check it worked

- `pot grant --diff` lists every kit as granted.
- `pot doctor` has no line that starts `!`.
- In a consumer, `pot explain claude` shows `next       ready`.

## If something goes wrong

- **An evaluation error names `defaultAccount`, `account`, `provider` or `--account`.** Replace it as the table
  says.
- **`has graft-kits only`.** That repository's graft is older than v2.16.0. Run `nix flake update graft` in its flake
  folder, or run the command with `pot host CMD`.
- **`packages is tools' old name`.** A kit that this graft builds still says `packages`: rename it. If it is a
  scion's, the provider needs a new release; until then, keep that consumer on graft v2.16.x.
- **A run stops with `changed source`.** A consumer's lock moved. Grant it once: `pot grant NAME`.

## Sources

- [CHANGELOG](../../CHANGELOG.md): v2.16.0's "Deprecated (removed in v2.17.0)"
- [pot's contracts](../../pot/contracts/README.md#how-a-contract-changes): the removed shims
- [pot/nix/root.nix](../../pot/nix/root.nix) and [pot/nix/hm.nix](../../pot/nix/hm.nix): `defaultAccount`
- [pot/kits/claude-code.nix](../../pot/kits/claude-code.nix): `account`, `provider`, `region`
- [pot/lib/kits-for.nix](../../pot/lib/kits-for.nix) and [lib/mk-flake.nix](../../lib/mk-flake.nix): `packages`,
  `graft-kits`
