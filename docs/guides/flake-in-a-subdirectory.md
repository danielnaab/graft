---
status: working
---
# A flake in a subdirectory

Keep content that must stay private out of the Nix store.

## Why

Nix copies a flake's source into the Nix store, and every user of the machine can read the store. For a
flake at a git repository's root, the source is every file that git tracks. Every `nix run .#…`,
`nix flake check` and `nix flake update` copies them. Nix 2.24 has no lazy trees.

A repository that holds private content (meeting transcripts, for example) keeps its flake in a
subdirectory, and runs it as a `path:` flake. Nix then copies only that subdirectory.

## Set up

1. Move `flake.nix` and `flake.lock` to a subdirectory, such as `nix/workspace/`. Keep no other files
   there.
2. In `mkFlake`, set `flakeDir` to that path. `dir` stays relative to the repository root.

   ```nix
   outputs = inputs: inputs.graft.lib.mkFlake {
     inherit inputs;
     flakeDir = "nix/workspace";   # where this flake.nix is, from the repository root
     dir = ".scions";              # <root>/.scions
   };
   ```

3. From the repository root, lock the flake by its absolute path. Nix does not lock a relative `path:`
   flake.

   ```
   nix flake lock "path:$PWD/nix/workspace"
   ```

4. Run `nix run path:nix/workspace#graft`. This writes the flake's path into the scions README.
5. Commit the moved `flake.nix` and `flake.lock`, and the scions directory.

The workflows need no change. They find the flake from the scions README.

## Commands

| To | Run |
|---|---|
| Sync, from anywhere in the checkout | `nix run path:nix/workspace#graft` |
| Check | `nix run path:nix/workspace#graft check` |
| Upgrade | `nix run path:nix/workspace#graft upgrade` |
| Lock again after an edit to `flake.nix` | `nix flake lock "path:$PWD/nix/workspace"` |

Always write `path:`. In a git checkout, a plain path is a `git+file` flake, which copies the whole
repository. `.` at the root has no flake to find.

## What is different

- **No `checks.<system>.graft`.** The flake's source cannot see the scions directory. `graft check` is
  the check. `graft ci check` runs it. The upgrade workflow and `graft migrate` run it and then the
  repository's own checks.
- **The app finds the repository root with git** (`git rev-parse --show-toplevel`).
- **The scions README records the flake** on its second line, `<!-- graft flake: nix/workspace -->`.
  graft's commands and workflows find the flake from that line. Nothing else is configured.
- **`-C` takes the repository root**, not the flake's directory.
- **`upgrade` locks with `--flake path:<absolute path>`**, which needs Nix 2.19 or later.
- `flakeDir` can contain only letters, digits, `.`, `_`, `-` and `/`.

Related: [Graft a repository](graft-a-repository.md), [Nix behaviours](../reference/nix-behaviours.md).
