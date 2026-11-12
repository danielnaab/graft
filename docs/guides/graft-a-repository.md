---
status: working
---
# Graft a repository

Set up a repository to graft others, then keep its scions in step with its pins.

## Set up

1. In the repository, run `nix flake init -t github:danielnaab/graft`. This writes `flake.nix` and graft's
   two CI workflows in `.forgejo/workflows/`.
2. In `flake.nix`, set `description`.
3. Add one input for each provider.
4. Pin each input to a tag.
5. Write `flake = false` on each input:

   ```nix
   knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
   ```

6. Run `nix run .#graft`. This locks the inputs and writes `scions/`.
7. Commit `flake.nix`, `flake.lock`, `scions/` and `.forgejo/`.

Rules for the inputs:

- Write `flake = false` on the input. Nix ignores `?flake=false` in a URL.
- Write `inputs` as a literal attribute set. Nix reads it without evaluation.
- Use `github:` for a public GitHub repository. For a private repository, see
  [Private providers](private-providers.md).
- Take one repository through one input only. If two inputs name one repository at one commit, Nix fetches
  it once, and both inputs get the tree of the first fetch.

To fetch one copy of an input that graft also has, add a `follows`:

```nix
graft.inputs.nixpkgs.follows = "nixpkgs";                       # if your repository has nixpkgs
graft.inputs.knowledge-base-kit.follows = "knowledge-base-kit"; # if it grafts knowledge-base-kit
```

graft uses its own `knowledge-base-kit` input only for its own docs. A consumer never evaluates it, so the
second line only keeps `flake.lock` to one entry.

## Sync and check

| To | Run |
|---|---|
| Write the scions directory from `flake.lock` | `nix run .#graft` |
| Check the working tree's scions and graft's workflows | `nix run .#graft check` |
| Check what git tracks, with the repository's other checks | `nix flake check` |

The check fails on drift. To find what changed and fix it, see
[`graft check` fails](check-fails.md).

## Options

`mkFlake` takes optional arguments beside `inputs`, for example:

```nix
outputs = { graft, nixpkgs, ... }@inputs: graft.lib.mkFlake {
  inherit inputs;
  dir = ".scions";                     # a hidden directory, out of tools that skip them (an Obsidian vault)
  scions = [ "knowledge-base-kit" ];   # when a `flake = false` input is not a scion
  outputs.devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell { };
};
```

All arguments are in [Commands](../reference/commands.md#mkflake). To keep private content out of the Nix
store, see [A flake in a subdirectory](flake-in-a-subdirectory.md).

## Try a provider's unreleased change

To graft a local checkout of a provider for one run, override its input:

```
nix run .#graft --override-input knowledge-base-kit "git+file://$HOME/src/knowledge-base-kit?exportIgnore=1"
```

- This includes uncommitted edits, and applies the provider's export rules. A `path:` URL does not
  apply them.
- `flake.lock` does not change. The scions README records the commit as `…-dirty`, so `nix flake check`
  fails if you commit this by mistake.
- To go back to the pinned version, run `nix run .#graft`.
- When the change is correct, tag the provider and run `nix run .#graft upgrade`.

Related: [Upgrade](upgrade.md), [CI](ci.md).
