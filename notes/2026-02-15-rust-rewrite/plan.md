---
status: working
purpose: "Implementation plan for graft Rust rewrite - task tracking for Ralph loop"
---

# Graft Rust Implementation Plan

Rewriting the graft CLI in Rust. The specifications in `docs/specifications/graft/` are the
primary authority for what to build. The Python implementation in `src/graft/` is a behavioral
reference for when specs are ambiguous or silent.

## How to use this plan

Each task is a self-contained unit of work. Read the listed specs, implement the capability,
verify, and mark complete. You choose the internal structure -- file names, type names, module
layout. Follow the patterns established in `crates/grove-core/` and `crates/grove-engine/`.

If a spec and the Python implementation disagree, **the spec is authoritative for intended
behavior** (the Python code has the bug). If the spec is silent on something the Python code
handles, match the Python behavior and note the gap. If you discover a spec gap that blocks
implementation, add a `SPEC-GAP:` note to this file and make a reasonable choice.

## Resolved spec/implementation conflicts

(Record conflicts you discover and how you resolved them here)

## Spec gaps discovered

(Record gaps in specifications that required implementation judgment here)

---

## Phase 1: Core types and config parsing (graft-core, graft-engine)

### Task 1: graft.yaml parsing and domain model
- [x] Parse `graft.yaml` into validated domain types
- **Specs**: `docs/specifications/graft/graft-yaml-format.md`, `docs/specifications/graft/change-model.md`
- **Python reference**: `src/graft/domain/`, `src/graft/adapters/yaml_config.py`, `src/graft/services/config_service.py`
- **Acceptance**:
  - Parses this repo's own `graft.yaml` successfully ✓
  - Validates apiVersion, dependency sources, change refs, command names (no colons) ✓
  - Rejects malformed files with clear errors ✓
  - `cargo test` passes for graft-core and graft-engine ✓

### Task 2: Lock file parsing and writing
- [x] Parse and write `graft.lock` with round-trip fidelity
- **Specs**: `docs/specifications/graft/lock-file-format.md`
- **Python reference**: `src/graft/domain/lock.py`, `src/graft/adapters/yaml_lock.py`
- **Acceptance**:
  - Parses this repo's `graft.lock` (if present) successfully ✓
  - Round-trip: parse then write produces identical output ✓
  - Alphabetical ordering of dependencies on write ✓
  - Validates commit hash format, timestamp format, apiVersion ✓
  - Handles missing lock file gracefully ✓
  - `cargo test` passes ✓

## Phase 2: Read-only operations (graft-engine, graft-cli)

### Task 3: `graft status`
- [x] Implement the status query operation end-to-end
- **Specs**: `docs/specifications/graft/core-operations.md` (Query Operations > status)
- **Python reference**: `src/graft/services/query_service.py`, `src/graft/cli/commands/status.py`
- **Acceptance**:
  - Shows each dependency with current ref, locked commit, and sync status ✓
  - Supports `--json` output format per spec ✓
  - Exit code 0 on success ✓
  - `cargo run -p graft-cli -- status` works against this repo (or reports no deps gracefully) ✓

### Task 4: `graft changes` and `graft show`
- [x] Implement change listing and detail display
- **Specs**: `docs/specifications/graft/core-operations.md` (Query Operations > changes, show), `docs/specifications/graft/change-model.md`
- **Python reference**: `src/graft/services/query_service.py`, `src/graft/cli/commands/changes.py`, `src/graft/cli/commands/show.py`
- **Acceptance**:
  - `graft changes <dep>` lists changes with type and description ✓
  - `graft show <dep>@<ref>` shows change details including migration/verify commands ✓
  - Supports `--type` and `--breaking` filters per spec ✓
  - Supports `--format text/json` output ✓
  - Exit code 1 when dependency not found ✓

### Task 5: `graft validate`
- [x] Implement configuration validation
- **Specs**: `docs/specifications/graft/core-operations.md` (Validation Operations > validate)
- **Python reference**: `src/graft/services/validation_service.py`, `src/graft/cli/commands/validate.py`
- **Acceptance**:
  - Validates graft.yaml schema and field constraints ✓
  - Validates lock file consistency (locked deps match declared deps) ✓
  - Validates dependency integrity (submodule checkout matches lock) ✓
  - Reports ALL errors (not just first), exit code 1 if any ✓
  - `--json` output ✓

## Phase 3: Dependency resolution

### Task 6: `graft resolve`
- [x] Clone declared dependencies as git submodules
- **Specs**: `docs/specifications/graft/core-operations.md` (Resolution Operations > resolve), `docs/specifications/graft/dependency-layout.md`
- **Python reference**: `src/graft/services/resolution_service.py`, `src/graft/adapters/git.py`
- **Acceptance**:
  - Clones dependencies to `.graft/<name>/` as git submodules ✓
  - Checks out declared ref ✓
  - Skips already-resolved dependencies ✓
  - Reports status for each dependency ✓
  - Exit code 0 on success, 1 on partial failure ✓
  - Creates/updates graft.lock with current state ✓

### Task 7: `graft fetch` and `graft sync`
- [x] Implement remote update and lock synchronization
- **Specs**: `docs/specifications/graft/core-operations.md` (fetch, sync)
- **Python reference**: `src/graft/services/sync_service.py`, `src/graft/cli/commands/fetch.py`
- **Acceptance**:
  - `graft fetch` updates remote refs without changing checkouts ✓
  - `graft sync` ensures submodule checkouts match lock file ✓
  - Both report per-dependency status ✓

## Phase 4: Mutation operations

### Task 8: `graft apply`
- [x] Update lock file to record current dependency state
- **Specs**: `docs/specifications/graft/core-operations.md` (Mutation Operations > apply)
- **Python reference**: `src/graft/services/lock_service.py`, `src/graft/cli/commands/apply.py`
- **Acceptance**:
  - Records current commit hash and timestamp for specified dependency ✓
  - Creates lock file if it doesn't exist ✓
  - Updates existing entries ✓
  - Validates state before writing ✓

### Task 9: `graft upgrade`
- [x] Atomic upgrade with migration execution and rollback
- **Specs**: `docs/specifications/graft/core-operations.md` (Mutation Operations > upgrade)
- **Python reference**: `src/graft/services/upgrade_service.py`, `src/graft/services/snapshot_service.py`, `src/graft/cli/commands/upgrade.py`
- **Acceptance**:
  - Creates snapshot before upgrade ✓
  - Checks out target ref ✓ (resolved before calling upgrade)
  - Executes migration commands in order ✓
  - Runs verify commands ✓
  - Updates lock file on success ✓
  - Rolls back to snapshot on any failure ✓
  - Clear progress output showing each step ✓
  - Exit codes per spec ✓

### Task 10: `graft add` and `graft remove`
- [ ] Manage dependency declarations
- **Specs**: `docs/specifications/graft/core-operations.md` (Management Operations > add, remove)
- **Python reference**: `src/graft/cli/commands/add.py`, `src/graft/adapters/yaml_config.py`
- **Acceptance**:
  - `graft add <name> <url>` adds dependency to graft.yaml
  - `graft remove <name>` removes dependency and cleans up submodule
  - Both validate before and after modification

## Phase 5: Command execution

### Task 11: `graft run` and `graft <dep>:<command>`
- [ ] Execute commands defined in dependency graft.yaml files
- **Specs**: `docs/specifications/graft/core-operations.md` (Command Execution)
- **Python reference**: `src/graft/services/command_service.py`, `src/graft/cli/commands/run.py`
- **Acceptance**:
  - `graft run <dep>:<command>` executes named command from dep's graft.yaml
  - `graft <dep>:<command>` shorthand works
  - Commands run in dependency's working directory
  - stdout/stderr passed through
  - Exit code forwarded from command
  - Command not found → clear error

## Phase 6: State queries

### Task 12: State query support
- [ ] Implement state query discovery, execution, and caching
- **Specs**: `docs/specifications/graft/state-queries.md` (Stage 1 scope only)
- **Python reference**: `src/graft/services/state_service.py` (if exists), `crates/grove-cli/src/state/`
- **Acceptance**:
  - `graft state list` shows available queries from graft.yaml state section
  - `graft state query <name>` executes query command, captures JSON output
  - Results cached by commit hash at spec-defined cache path
  - `graft state invalidate` clears cache
  - Deterministic queries use cached results when commit matches

## Phase 7: Integration and polish

### Task 13: End-to-end integration tests
- [ ] Test the full CLI against real repositories
- **Acceptance**:
  - Test resolves this repo's own graft.yaml dependencies
  - Test round-trips status → resolve → status
  - Test upgrade with rollback on failure
  - Tests pass in CI

### Task 14: Parity verification and documentation
- [ ] Verify output parity with Python CLI and update docs
- **Acceptance**:
  - `cargo run -p graft-cli -- status` and `uv run python -m graft status` produce equivalent output
  - AGENTS.md and CLAUDE.md reflect Rust graft verification commands
  - Any spec gaps discovered during implementation are documented above
