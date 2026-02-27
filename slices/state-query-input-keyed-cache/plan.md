---
status: done
created: 2026-02-27
---

# Fix state query caching to use input-keyed invalidation

## Story

State query results cache against the wrong key. The `deterministic` flag is
misnamed — both `verify` and `slices` are deterministic given the same inputs,
but the flag actually models whether the commit hash is a sufficient cache key.
This causes `verify` to return stale results when source files have local edits,
and prevents `options_from` completions from working in grove because `slices`
(non-deterministic) is never cached.

## Approach

Replace `deterministic: bool` with `inputs: Option<Vec<String>>` (glob patterns
declaring which files the query reads). Cache key is determined by working tree
state for those inputs:

- `inputs` declared, working tree clean: commit hash as key (zero overhead, shareable)
- `inputs` declared, working tree dirty: content hash of matching files as key (correct for local edits)
- No `inputs` declared (slices, changes): never cache — always run fresh

For grove `options_from`: when no cached result exists, run the state query's
bash command directly as a subprocess. Cache result in-memory on `RepoContext`;
clear after any command execution.

## Acceptance Criteria

- `deterministic` field removed; `inputs` accepted as optional list of globs
- `verify` after editing source files produces a fresh result, not stale commit-keyed one
- `verify` twice without changes hits cache on second run
- `slices` and `changes` have no `inputs`; they always run fresh
- Grove `:run software-factory:implement ` shows slice completions without pre-warming cache
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Update spec: replace `deterministic` with `inputs` in graft-yaml-format.md**
  - **Delivers** — clear contract before any code changes
  - **Done when** — `graft-yaml-format.md` documents `inputs` as optional list of globs,
    explains three caching modes, removes `deterministic`
  - **Files** — `docs/specifications/graft/graft-yaml-format.md`

- [x] **Implement `inputs`-based cache key in `graft-common` and `graft-engine`**
  - **Delivers** — correct cache invalidation for `verify`-class queries
  - **Done when** — `CacheConfig` gains `inputs: Option<Vec<String>>`; `get_state`
    checks `git status --porcelain -- {inputs}` to determine cache key; clean tree
    uses commit hash, dirty tree hashes input file contents; no-inputs queries skip
    cache entirely; `StateMetadata` stores cache key; unit tests cover all three modes
  - **Files** — `crates/graft-common/src/config.rs`, `crates/graft-common/src/state.rs`,
    `crates/graft-engine/src/state.rs`

- [x] **Update `graft.yaml` files and remove `deterministic`**
  - **Delivers** — correct config for all state queries
  - **Done when** — `verify` gains `inputs`; `slices` and `changes` have no cache config;
    both local and software-factory graft.yaml updated
  - **Files** — `graft.yaml`, `.graft/software-factory/graft.yaml`

- [x] **Fix grove `options_from` to run queries fresh when no cache**
  - **Delivers** — slice completions work immediately
  - **Done when** — `commands_with_resolved_options` falls back to running the state
    query's `run` command when `read_latest_cached` returns `None`; result cached
    in-memory; cleared after command execution
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`
