---
status: working
---
# Move to v2.18.0

Since v2.20.0 a kit command inside a run is a sibling, not a nested child: see [Move to v2.20.0](move-to-v2-20.md).

graft v2.18.0 wraps kit commands: a consumer's tools hold a wrapper for each command its kits declare, which runs
the command in pot under its own policy, nested when it starts inside an agent's run. Runs nest, and `pot dispatch`
works inside a run. This page says what to change, in order: roots, then consumers.

## Who changes nothing

- **A consumer whose kits declare no `commands`.** Its tools are unchanged, and `kits.just` holds only graft's
  own recipes (`outdated`, `upgrade`, `graft-check`), which you may ignore.
- **Your grants.** No grant asks again when a root takes v2.18.0. A consumer that takes v2.18.0 changes its
  `flake.lock`, so it asks once for its own repo kit (`pot grant NAME`). A scion kit whose commands request
  nothing (notebook-kit's) needs no grant.

## Move a root first

Take graft v2.18.0 in the root, then `home-manager switch`, so `~/.config/pot/launcher` names a pot that runs
wrappers. `pot doctor`'s `launcher` row then names this pot, with no `!`.

A consumer that moves first still works: under a root older than v2.18.0, a wrapper runs its program unwrapped
and says so on stderr:

```
pot: check-kb runs without pot (your root's pot predates wrapped commands: take graft v2.18.0 in your root (home-manager switch))
```

## Move a consumer

1. Take the release: `nix run .#graft upgrade` (or `nix flake update graft`).
2. Import `<scions>/kits.just` in your justfile after any provider's own `.just` file. With
   `set allow-duplicate-recipes`, just keeps the first of two recipes with one name, so a provider's hand-written recipe runs; it calls the same
   wrapper from `tools`. Then run `nix run .#graft sync`, which writes and stages the file (graft writes it only once the justfile
   names it or it exists). Commit it with the lock: `graft check` fails while it is stale.

   ```just
   set allow-duplicate-recipes
   import? "nix/agent-tools/.scions/notebook-kit/tools/kit.just"
   import? "nix/agent-tools/.scions/kits.just"
   ```

3. Define `tools` as the tools **package**, not its `bin/`: `kits.just` runs `{{tools}}/bin/<command>`. If your
   justfile already has a `tools` that names a `bin/`, take its parent and fix its other uses:

   ```just
   tools := parent_directory(`sh nix/agent-tools/ensure.sh`)
   test *args:
       "{{tools}}/bin/bun" test "$@"
   ```

4. Make each git hook that a kit now ships as a hook command exec the wrapper:

   ```sh
   #!/bin/sh
   set -eu
   cd "$(git rev-parse --show-toplevel)"
   exec "$(sh nix/agent-tools/ensure.sh)/nb-pre-commit" "$@"
   ```

   On the host, `git commit` runs it under the command's own policy; inside `pot claude`, nested under it.
   A dispatch clone carries `core.hooksPath` only when it is relative and names a tracked folder, so set it that
   way, once per clone: `git config core.hooksPath .githooks`. An absolute path (such as
   `/home/daniel/src/flexion-assistant/.githooks`) still works in place, but a dispatch clone does not carry it,
   and commits there run no hook.
   `git commit --no-verify` skips it, as for any hook.

5. **Anything that must not run pot** (a CI job, a Nix check, a container, a scheduler such as a Windmill worker)
   takes the unwrapped tools, `packages.<system>.tools.passthru.unwrapped`:

   ```nix
   outputs.packages = genAttrs systems (system: {
     meeting-agent-tools = inputs.self.packages.${system}.tools.passthru.unwrapped;
   });
   ```

   Build the wrapped tools for your own shell into a separate out-link, so the two never overwrite each other
   (flexion-assistant: `.local/tools` for the worker, `.local/kit-tools` for the justfile and hooks). Without
   this, a wrapper there finds no launcher and runs unwrapped with a stderr line, which works but is noise.

## Check it worked

- `just --list` shows the kits' recipes; `just check` passes.
- `pot explain claude` has a `children` row listing the kit's commands as `nest`.
- A test commit runs the hook (`git commit --allow-empty -m test`, then `git reset HEAD~`).
- `nix run .#graft check` passes.

## If something goes wrong

- **`graft: <scions>/kits.just is stale (or edited): graft sync`.** Run `nix run .#graft sync` on the host and
  commit the file. Inside a run the scions folder is read-only, so the message says to run it on the host.
- **`…/bin/bin/check-kb: not found`.** `tools` names a `bin/` (step 3).
- **`graft: scions/kits.just's recipe X has the name of the justfile's own X: rename one (a kit's recipe never
  replaces yours)`.** Your justfile defines a recipe a kit also generates. Drop yours if it did the same thing, or
  rename it.
- **`pot: check-kb runs without pot (…)`.** The wrapper found no usable pot: no root here (a Mac with pot off, a
  container), or a root older than v2.18.0. The command still ran; the parenthesis says what to do.
- **On macOS, `git commit` inside `pot claude` stops at the pre-commit hook:** `… Seatbelt cannot nest (a run on
  macOS starts no runs inside it): run it on the host (…)`. The hook command would need its own, narrower sandbox.
  Commit on the host, or `git commit --no-verify`.
- **`… needs X, which this run (claude) lacks: run it on the host (just RECIPE)`.** Inside a run, that kit
  command asks for reach the agent's run does not have. Run the recipe on the host.

## Sources

- [CHANGELOG](../../CHANGELOG.md): v2.18.0
- [pot's reference: Wrapped commands](../reference/pot.md#wrapped-commands)
- [Files: kits.just](../reference/files.md#kitsjust)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `tools` and `passthru.unwrapped`
