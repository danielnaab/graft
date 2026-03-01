---
status: pending
created: 2026-03-01
depends_on:
  - scion-worktree-primitives
---

# Scion config, create, and prune

## Story

Users need `graft scion create <name>` and `graft scion prune <name>` to manage
parallel workstreams. Before these commands can exist, the graft.yaml spec needs a
`scions:` section for lifecycle hook declarations, the config parser needs to
understand it, and the engine needs scion operations that compose worktree primitives
with config awareness. This slice delivers the end-to-end path from spec through CLI
for create and prune — without hook execution (that's a later slice).

## Approach

Four layers, bottom-up:

1. **Spec** — add `scions:` section to `graft-yaml-format.md` documenting the four
   hook points (`on_create`, `pre_fuse`, `post_fuse`, `on_prune`), each accepting a
   command name or list of command names.

2. **Domain + config** — add `ScionHooks` struct to `graft-engine/src/domain.rs`
   (alongside `GraftConfig`, `Command`, `StateQuery` — it's a graft-specific domain
   type, not shared with grove) and parse the `scions:` YAML key in
   `graft-engine/src/config.rs`. Add `scion_hooks: Option<ScionHooks>` field to
   `GraftConfig`. Cross-validate that hook command names exist in the `commands:`
   section (matching the pattern for sequence steps and migration commands).

3. **Engine** — add `crates/graft-engine/src/scion.rs` with `scion_create` and
   `scion_prune`. For now these call the worktree primitives only (no hook execution).
   Engine functions apply the naming convention: `.worktrees/<name>` path and
   `feature/<name>` branch. Add `From<graft_common::GitError> for GraftError` to
   bridge the error types across the crate boundary.

4. **CLI** — add `scion create <name>` and `scion prune <name>` subcommands to
   `graft-cli/src/main.rs`.

Hook execution is explicitly deferred to `scion-hook-composition-and-fuse`. The
create and prune commands here perform only the git operations.

## Acceptance Criteria

- `graft-yaml-format.md` documents `scions:` with all four hook points
- `ScionHooks` type in `graft-engine/src/domain.rs` parses `scions:` from YAML;
  missing section yields `None`
- `GraftConfig` carries `scion_hooks: Option<ScionHooks>`
- `GraftConfig::validate()` checks that all hook command names exist in `commands:`
- `From<GitError> for GraftError` impl bridges error types
- `scion_create(repo, name)` applies `.worktrees/<name>` + `feature/<name>` convention,
  calls `git_worktree_add`, and returns the worktree path
- `scion_prune(repo, name)` calls `git_worktree_remove` then `git_branch_delete`
- `graft scion create foo` creates `.worktrees/foo` with branch `feature/foo`
- `graft scion prune foo` removes the worktree and branch
- Creating an already-existing scion prints an error and exits non-zero
- Pruning a non-existent scion prints an error and exits non-zero
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [ ] **Update spec: add `scions:` section to `graft-yaml-format.md`**
  - **Delivers** — clear contract for hook declarations before any code
  - **Done when** — `graft-yaml-format.md` documents `scions:` as an optional
    top-level key; each hook point (`on_create`, `pre_fuse`, `post_fuse`, `on_prune`)
    accepts a string (single command name) or list of strings (command names);
    command names resolve relative to the defining graft.yaml's scope; examples
    included
  - **Files** — `docs/specifications/graft/graft-yaml-format.md`

- [ ] **Add `ScionHooks` type to `graft-engine/src/domain.rs` and parse in `config.rs`**
  - **Delivers** — typed representation of scion hook configuration with validation
  - **Done when** — `ScionHooks { on_create, pre_fuse, post_fuse, on_prune }` struct
    in `domain.rs`, each field `Option<Vec<String>>` (single string normalized to
    one-element vec during parsing), derives `Serialize, Deserialize, Clone, Debug,
    PartialEq, Eq`; `GraftConfig` gains `scion_hooks: Option<ScionHooks>` field;
    `parse_graft_yaml` in `config.rs` parses `scions:` YAML key; unit tests cover:
    missing section, single command, list of commands, empty section
  - **Files** — `crates/graft-engine/src/domain.rs`,
    `crates/graft-engine/src/config.rs`

- [ ] **Cross-validate hook command names in `GraftConfig::validate()`**
  - **Delivers** — early detection of typos in hook command names (matching
    existing pattern for sequence steps and migration/verify commands)
  - **Done when** — `validate()` checks every command name in `scion_hooks`
    exists in `self.commands`; returns `ConfigValidation` error if not; test
    confirms invalid hook name is rejected, valid name passes
  - **Files** — `crates/graft-engine/src/domain.rs`

- [ ] **Add `From<GitError> for GraftError` error bridging**
  - **Delivers** — clean error propagation from git primitives to engine layer
  - **Done when** — `impl From<graft_common::GitError> for GraftError` added;
    scion engine functions can use `?` on git primitive results
  - **Files** — `crates/graft-engine/src/error.rs`

- [ ] **Add `scion_create` and `scion_prune` to `graft-engine/src/scion.rs`**
  - **Delivers** — engine-level scion lifecycle operations with naming convention
  - **Done when** — new module `scion.rs` exports `scion_create(repo_path, name)`
    and `scion_prune(repo_path, name)`; `scion_create` builds
    `.worktrees/<name>` path and `feature/<name>` branch, calls
    `git_worktree_add(repo, path, branch)`, returns the worktree path;
    `scion_prune` builds the same path/branch, calls `git_worktree_remove` then
    `git_branch_delete`; errors propagate via `From<GitError>` impl;
    module declared in `lib.rs`; integration test covers create-then-prune round-trip
  - **Files** — `crates/graft-engine/src/scion.rs`, `crates/graft-engine/src/lib.rs`

- [ ] **Add `graft scion create/prune` CLI subcommands**
  - **Delivers** — user-facing commands for scion lifecycle
  - **Done when** — `graft scion create <name>` calls `scion_create`, prints
    worktree path on success; `graft scion prune <name>` calls `scion_prune`, prints
    confirmation; both exit non-zero with descriptive error on failure; `--help`
    shows usage
  - **Files** — `crates/graft-cli/src/main.rs`
