---
status: working
purpose: "Task tracking for workspace unification Ralph loop"
---

# Workspace Unification Plan

The graft Rust rewrite is complete (14 tasks, 91 tests, full parity with Python). The repo now
has 6 Rust crates that share NO code despite significant overlap. This plan extracts shared
infrastructure into `graft-common`, cleans up repo organization, and brings documentation to
meta-KB compliance.

## How to use this plan

Each task is a self-contained unit of work. Read the listed source files, implement the change,
verify, and mark complete. Follow existing patterns in the codebase.

## Issues discovered

(Record issues you discover during implementation here)

---

## Phase 1: Shared Infrastructure — Create `graft-common` (Tasks 1–4)

### Task 1: Create `graft-common` crate with timeout-protected command execution
- [x] Create `graft-common` crate and extract shared command execution
- **Source code**: `crates/grove-engine/src/git.rs` (has `run_git_with_timeout`, timeout config pattern)
- **Target**: `crates/graft-common/Cargo.toml`, `crates/graft-common/src/lib.rs`, `crates/graft-common/src/command.rs`
- **What to do**:
  - Create `crates/graft-common/` with `Cargo.toml`, `src/lib.rs`, `src/command.rs`
  - Extract the timeout-protected command runner from grove-engine into a generalized `run_command_with_timeout()` function
  - Add `graft-common` to workspace `Cargo.toml` as a workspace member
  - Dependencies: `thiserror`, `wait-timeout`, `log`
- **Acceptance**:
  - New crate compiles as workspace member
  - Has unit tests for timeout behavior (success, timeout, failure cases)
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 2: Standardize on `serde_yaml` (remove `serde_yml`)
- [ ] Migrate all `serde_yml` call sites to `serde_yaml`
- **Source code**:
  - `crates/grove-engine/src/config.rs` (~2 call sites using `serde_yml`)
  - `crates/graft-engine/src/lock.rs` (~3 call sites using `serde_yml`)
  - `Cargo.toml` (workspace dependencies)
  - Individual crate `Cargo.toml` files
- **What to do**:
  - Replace all `serde_yml::from_str` / `serde_yml::to_string` with `serde_yaml` equivalents
  - Remove `serde_yml` from workspace `Cargo.toml` and individual crate `Cargo.toml` files
  - Ensure `serde_yaml` is in workspace dependencies (it may already be)
- **Acceptance**:
  - `serde_yml` appears nowhere in the codebase (`grep -r serde_yml` returns nothing)
  - All existing tests pass unchanged
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 3: Add shared git primitives to `graft-common`
- [ ] Extract common git operations into `graft-common`
- **Source code**:
  - `crates/grove-engine/src/git.rs` (git operations with timeout protection)
  - `crates/graft-engine/src/resolution.rs` (bare `Command::new("git")` calls)
  - `crates/graft-engine/src/validation.rs` (`get_current_commit()`)
- **Target**: `crates/graft-common/src/git.rs`
- **What to do**:
  - Create `graft-common/src/git.rs` with shared git operations:
    - `git_rev_parse(path, ref) -> Result<String>`
    - `git_fetch(path) -> Result<()>`
    - `git_checkout(path, commit) -> Result<()>`
    - `get_current_commit(path) -> Result<String>`
    - `is_git_repo(path) -> bool`
  - All use the timeout-protected runner from Task 1
- **Acceptance**:
  - Functions have unit tests
  - Existing tests still pass
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 4: Extract shared state query types and cache logic to `graft-common`
- [ ] Move duplicated state types and cache logic to `graft-common`
- **Source code**:
  - `crates/graft-engine/src/state.rs` (state types and cache logic)
  - `crates/grove-cli/src/state/query.rs` (duplicated state types)
  - `crates/grove-cli/src/state/cache.rs` (duplicated cache logic)
- **Target**: `crates/graft-common/src/state.rs`
- **What to do**:
  - Move shared types: `StateMetadata`, `StateResult`
  - Move cache path computation using SHA256 workspace hash
  - Move cache read/write helpers
  - Add dependencies: `serde`, `serde_json`, `sha2`, `chrono`
- **Acceptance**:
  - Types compile with serde derives
  - Cache logic has unit tests
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 2: Consumer Migration (Tasks 5–8)

### Task 5: Migrate grove-engine to use `graft-common` command execution
- [ ] Replace grove-engine's command runner with `graft-common`
- **Source code**: `crates/grove-engine/src/git.rs`, `crates/grove-engine/Cargo.toml`
- **What to do**:
  - Add `graft-common` dependency to grove-engine
  - Replace `run_git_with_timeout` in `grove-engine/src/git.rs` with `graft_common::command::run_command_with_timeout`
  - Keep `GitoxideStatus` and grove-specific trait impls in grove-engine
- **Acceptance**:
  - All grove-engine and grove-cli tests pass
  - `run_git_with_timeout` no longer defined in grove-engine
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 6: Migrate graft-engine to use `graft-common` (commands + git ops)
- [ ] Replace graft-engine's git and command code with `graft-common`
- **Source code**:
  - `crates/graft-engine/src/resolution.rs` (bare `Command::new("git")` calls)
  - `crates/graft-engine/src/command.rs` (command execution)
  - `crates/graft-engine/src/validation.rs` (`get_current_commit()`)
  - `crates/graft-engine/Cargo.toml`
- **What to do**:
  - Add `graft-common` dependency to graft-engine
  - Replace bare `Command::new("git")...output()` with shared git ops from `graft-common`
  - Replace `graft-engine/src/command.rs` internals with shared command runner (adds timeout protection graft previously lacked)
  - Replace `get_current_commit()` with shared version
- **Acceptance**:
  - All graft tests pass
  - Git operations now have timeout protection
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 7: Migrate state query consumers to `graft-common` types
- [ ] Replace duplicated state types with `graft-common` re-exports
- **Source code**:
  - `crates/grove-cli/src/state/query.rs` (duplicated types)
  - `crates/grove-cli/src/state/cache.rs` (duplicated cache logic)
  - `crates/graft-engine/src/state.rs` (duplicated types and cache logic)
- **What to do**:
  - Replace `grove-cli/src/state/query.rs` types with re-exports from `graft-common`
  - Replace `grove-cli/src/state/cache.rs` logic with calls to `graft-common`
  - Replace duplicate types/cache logic in `graft-engine/src/state.rs` with `graft-common`
  - Keep graft-specific execution logic and grove-specific discovery logic
- **Acceptance**:
  - All tests pass
  - No duplicate `StateMetadata`/`StateResult` definitions
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

### Task 8: Deduplicate graft.yaml command/state parsing
- [ ] Extract shared graft.yaml parser for commands and state queries
- **Source code**:
  - `crates/grove-cli/src/state/discovery.rs` (re-parses graft.yaml from scratch)
  - `crates/grove-engine/src/config.rs` (`GraftYamlConfigLoader`, parses minimal subset)
  - `crates/graft-engine/src/config.rs` (graft's own parser)
- **Target**: `crates/graft-common/src/config.rs`
- **What to do**:
  - Extract a shared minimal graft.yaml command+state parser into `graft-common`
  - Both grove-cli and grove-engine use the shared parser
- **Acceptance**:
  - grove-cli/state/discovery.rs uses shared parser
  - All tests pass
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes

## Phase 3: Repository Organization (Tasks 9–11)

### Task 9: Mark Python code as deprecated
- [ ] Add deprecation notices to Python code
- **Source code**: `src/graft/__init__.py`, `src/graft/__main__.py`, `pyproject.toml`
- **What to do**:
  - Add deprecation notice to `src/graft/__init__.py` or `src/graft/__main__.py`
  - Add a `DEPRECATED.md` in `src/graft/` explaining the Rust CLI is the primary implementation
  - Update `pyproject.toml` description to note deprecated status
  - Do NOT remove any Python code or tests
- **Acceptance**:
  - Deprecation is clearly communicated
  - Python tests still pass (`uv run pytest`)

### Task 10: Merge grove/ docs into main docs structure
- [ ] Move grove docs into the main documentation tree
- **Source code**:
  - `grove/docs/` (agents.md, user-guide.md, README.md)
  - `grove/notes/`
  - `grove/knowledge-base.yaml`
  - Root `knowledge-base.yaml`
- **What to do**:
  - Move `grove/docs/agents.md` → referenced from main `AGENTS.md` (or inline grove section)
  - Move `grove/docs/user-guide.md` → `docs/guides/grove-user-guide.md`
  - Move `grove/docs/README.md` → `docs/grove-overview.md` (or merge into main README)
  - Merge relevant content from `grove/notes/` into `notes/`
  - Merge `grove/knowledge-base.yaml` content into root `knowledge-base.yaml`
  - Remove `grove/` directory (or leave minimal with redirect)
  - Update all internal links that referenced `grove/docs/`
- **Acceptance**:
  - No broken links
  - All grove docs accessible from main docs structure

### Task 11: Update entrypoints (AGENTS.md, CLAUDE.md, continue-here.md)
- [ ] Update project entrypoint documents to reflect current state
- **Source code**: `AGENTS.md`, `CLAUDE.md`, `continue-here.md`, `knowledge-base.yaml`
- **What to do**:
  - AGENTS.md: document `graft-common` crate, fix stale "rewrite in progress" reference, update test counts, remove grove/ directory references
  - CLAUDE.md: update verification commands (add `graft-common` to test commands), update key paths
  - continue-here.md: rewrite to reflect current state (Rust primary, Python deprecated, grove docs merged, graft-common exists)
  - Update root `knowledge-base.yaml` with new paths and graft-common component
- **Acceptance**:
  - All three files accurately reflect current repo state
  - No stale references to grove/ directory, Python as primary, or missing graft-common

## Phase 4: Meta-KB Documentation Compliance (Tasks 12–15)

### Task 12: Add lifecycle frontmatter to all documentation files
- [ ] Add status frontmatter to docs lacking it
- **Target files**: `docs/README.md`, `docs/guides/*.md`, `docs/cli-reference.md`, `docs/configuration.md`, `docs/index.md`, `docs/architecture.md`, `docs/decisions/*.md`, `docs/plans/*.md`
- **Reference**: `docs/plans/meta-kb-compliance-improvements.md` (existing compliance plan)
- **What to do**:
  - Add `status: stable/working/living/deprecated` frontmatter to all docs lacking it
  - Follow templates from existing meta-KB compliance plan
- **Acceptance**:
  - Every doc file in `docs/` has valid frontmatter with status field

### Task 13: Add provenance sections to key documents
- [ ] Add Sources sections to key documents
- **Target files**: `docs/README.md`, `docs/guides/user-guide.md`, `docs/cli-reference.md`, `docs/configuration.md`
- **What to do**:
  - Add `## Sources` sections with links to specs and code
  - Update source references to point to Rust crates (not just Python)
  - Ground architectural claims in spec links and code references
  - Follow provenance policy from meta-KB
- **Acceptance**:
  - 4+ documents have Sources sections linking to specs and code

### Task 14: Clarify authority boundaries and fix linking
- [ ] Add authority notes and fix broken links
- **Target files**: `docs/README.md`, `docs/guides/*.md`, docs that interpret specs
- **What to do**:
  - Add authority notes to interpretation documents
  - Fix any broken links (especially after grove/ merge in Task 10)
  - Convert backtick paths to markdown links where appropriate
  - Audit for `file:///` absolute paths
- **Acceptance**:
  - No broken links
  - Authority boundaries explicit in interpretation docs

### Task 15: Create ADR for workspace unification
- [ ] Write ADR documenting workspace unification decisions
- **Target**: `docs/decisions/008-workspace-common-crate.md` (or next available number)
- **Reference**: existing ADRs in `docs/decisions/` for format
- **What to do**:
  - Document: why graft-common was created, why serde_yaml was chosen, what was extracted, what was deferred (trait-based DI for graft), future work
  - Follow existing ADR format
  - Link from decisions/README.md (if it exists)
- **Acceptance**:
  - ADR exists and follows format
  - Linked from decisions/README.md

## Phase 5: Final Polish (Task 16)

### Task 16: Final verification and cleanup
- [ ] Full verification pass and cleanup
- **What to do**:
  - Run full `cargo fmt --check && cargo clippy -- -D warnings && cargo test` across all crates
  - Run `cargo run -p graft-cli -- status` smoke test
  - Remove dead code, unused imports, orphaned modules from extraction
  - Verify no remaining `serde_yml` references
  - Verify test counts and update CLAUDE.md if changed
  - Archive or mark deprecated `docs/plans/meta-kb-compliance-improvements.md` since its work is now done
- **Acceptance**:
  - All tests pass; no warnings
  - Smoke test works
  - Repo is clean
  - `cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes
