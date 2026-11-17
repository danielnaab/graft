---
status: working
---
# Move to v2.16.0

graft v2.16.0 puts a one-way boundary between graft and pot, graft's sandbox
([Decisions](../decisions.md#one-repository-one-way-arrows-between-graft-and-pot)). A repository now presents its
kits to pot as `packages.<system>.pot-kits`, and pot finds that flake from `.gitmodules` alone. Most repositories
and roots need no edit. This page says what changes for a consumer, a root and a provider, and what still works
for one release.

## When to use this

- You have a consumer (a repository that grafts scions) on graft v2.15.0, and a pot root.
- Or you have a root (`homeManagerModules.pot` or `graft.lib.mkRoot`) on graft v2.15.0.

You can move the consumer and the root in either order: each side reads the other's v2.15.0 form until v2.17.0.

## What still works until v2.17.0

These are deprecated in v2.16.0 and removed in v2.17.0:

| v2.15.0 | v2.16.0 | Until v2.17.0 |
|---|---|---|
| `packages.<system>.graft-kits` | `packages.<system>.pot-kits` | mkFlake writes both, so a v2.15.0 root reads a v2.16.0 consumer. A v2.16.0 pot reads `graft-kits` when a flake has no `pot-kits`, so it reads a v2.15.0 consumer. |

## What changes without an edit

- **One new grant for a repo kit with powerful requests.** Taking v2.16.0 changes your flake's `flake.lock`, so
  the tree of your flake directory changes, and a powerful grant is fixed to that tree. The next run stops with
  exit 125 (or prompts, at a terminal) with `changed source`, and the same requests:

  ```
  pot grant flexion-assistant
  ```

  Use your repository's short name. No canonical request changes in v2.16.0, so no other grant asks again.
- **graft's command left a run's standard tools.** In a graft consumer, a run still has the repository's own
  `graft`, from its bundle (`POT_TOOLS`). In any other repository, a run has no `graft` unless your root adds it
  ([below](#move-a-root)).
- **Protected paths.** `**/graft.nix` and the scions folder are now protected because the consumer's bundle names
  them, not by pot's own list. `pot explain` labels them `(repo)`; pot's own defaults are labelled `(pot)`, not
  `(graft)`. In `policy.json`, pot's defaults are `{ "root": "default" }`, not `{ "root": "graft" }`.
- **Messages name a kit's file as the bundle reports it:** `the repo kit's requests.network (flake.nix (perSystem))`,
  not `(perSystem.kit)`. Kit errors from the build start `pot:`, not `graft:`:
  `pot: <file>: <field> is not a kit field (…)`.
- **Your consumer's lock gets shorter.** graft's flake no longer has a `knowledge-base-kit` input, so it leaves
  your `flake.lock` at the next update.

## Move a consumer

1. Take v2.16.0, from the repository's top:

   ```
   nix run .#graft upgrade
   ```

   ```
   graft: graft v2.15.0 -> v2.16.0
   …
   ```

   For a flake in a subdirectory, run `nix run 'path:DIR#graft' upgrade`.
2. If the repository's own `outputs` defines `packages.<system>.pot-kits` or `apps.<system>.pot-kits`, rename it:
   the name is graft's now. The build says so:
   `graft: mkFlake's outputs has packages.<s>.pot-kits: that name is graft's; rename the repo's own`.
3. If the repository has a `perSystem.kit` but no submodules, pot no longer finds its flake: a repository with no
   `.gitmodules` has no repo kit from its flake, and pot never evaluates it. Move the kit into your root's
   `repoKits."<origin>"`, or graft a scion.
4. If a script or hook inside a run reads `/run/pot/policy.json` to decide whether it may fetch, ask pot instead:

   ```sh
   if pot why "$url" --json >/dev/null; then git ls-remote --tags "$url"; fi
   ```

   It exits 0 when the run reaches the URL, and 1 when it does not ([run-env/1](../../pot/contracts/run-env-1.md)).
5. Run `nix run .#graft check` and `nix flake check`, and commit.
6. Run `pot claude` (or any run) at a terminal, and grant the repo kit's changed source when it asks.

## Move a root

1. Move the root's `graft` input to v2.16.0, and update its lock. For example, in a home-manager flake:

   ```nix
   inputs.graft.url = "github:danielnaab/graft/v2.16.0";
   ```

   ```
   nix flake update graft
   ```

2. If your runs need `graft` outside graft consumers, add it to the root's tools:

   ```nix
   programs.pot.tools = [ inputs.graft.packages.${pkgs.system}.default ];
   ```

   With mkRoot, pass the same list as `tools`.
3. Build the root (`home-manager switch`, or rebuild your mkRoot flake). The grants file needs no edit.

## Move a provider

Nothing to change. `graft.nix` is still [provider/2](../reference/provider.md). A `kit` with an unknown field now
fails the consumer's `pot-kits` build with pot's message, `pot: <dir>/<name>/graft.nix: <field> is not a kit field
(…)`.

## Check it worked

In the consumer, from its top, paste this. It prints `ok` when the flake has both bundles and pot reads it with
no stale grant:

```sh
F=.   # or F='path:nix/agent-tools' for a flake in a subdirectory
nix eval --raw "$F#packages.x86_64-linux.pot-kits.name" | grep -qx pot-kits &&
  nix eval --raw "$F#packages.x86_64-linux.graft-kits.name" | grep -qx graft-kits &&
  pot bash -c 'test -n "$POT_TOOLS" && command -v graft >/dev/null' && echo ok
```

Use your machine's system for `x86_64-linux`.

## If something goes wrong

- **The run stops with exit 125 and `changed source`, listing `flake.lock`.** That is the one expected new grant:
  run `pot grant NAME` at a terminal.
- **`pot explain` shows no `(repo)` items in a repository that has a repo kit.** The repository has no
  submodules, so pot found no flake. Use the root's `repoKits` (step 3).
- **`graft: command not found` inside a run, in a repository that is not a graft consumer.** Add graft to the
  root's `tools` (root step 2).
- **`pot (work): ~/DIR#pot-kits: kit.json is version 2, and this pot reads version 1: …`.** The consumer's graft
  is newer than your root's pot. Move the root first.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): `pot-kits`, `graft-kits` and the reserved names
- [pot/lib/kits-for.nix](../../pot/lib/kits-for.nix): the bundle
- [pot/core/pot.py](../../pot/core/pot.py): finding the flake, the `graft-kits` fallback, `pot why --json`
- [pot's contracts](../../pot/contracts/README.md)
- [CHANGELOG](../../CHANGELOG.md)
