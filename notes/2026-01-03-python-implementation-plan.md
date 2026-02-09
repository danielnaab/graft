---
title: "Python Implementation Plan"
date: 2026-01-03
status: working
---

# Python Implementation Plan

## Overview

This note documents the plan to sync the Python implementation in this repository with the full specification defined in this knowledge base (graft-knowledge).

## Current State Analysis

### What Exists (graft repository)

The `graft` Python implementation currently includes:

1. **Basic dependency resolution** (`graft resolve` command)
   - Parses simple `graft.yaml` format (just `apiVersion` and `dependencies`)
   - Clones git repositories
   - Basic git operations (clone, fetch, checkout)

2. **Domain models**:
   - `DependencySpec` - name, git_url, git_ref
   - `DependencyResolution` - resolution state tracking
   - `GraftConfig` - simple config container
   - `GitUrl`, `GitRef` - value objects

3. **Architecture**:
   - Functional service layer with protocol-based DI
   - Clean separation: domain/services/protocols/adapters/cli
   - Immutable value objects
   - Comprehensive exception hierarchy

4. **Infrastructure**:
   - Full test suite with fakes (not mocks)
   - Strict mypy type checking
   - Ruff linting/formatting
   - Modern Python (3.11+) with uv package manager

### What's Missing (from specification)

The specification defines significantly more functionality:

1. **Change Model** - Not implemented
   - `Change` entity with ref, type, description, migration, verify
   - Changes section in graft.yaml
   - Change tracking and querying

2. **Command System** - Not implemented
   - `Command` entity with run, description, working_dir, env
   - Commands section in graft.yaml
   - Command execution infrastructure

3. **Lock File** (`graft.lock`) - Not implemented
   - Consumed version tracking
   - Commit hash for integrity
   - Timestamp tracking

4. **Core Operations** - Not implemented
   - `graft upgrade` - Atomic upgrades with migration/verification
   - `graft status` - Show dependency states and available updates
   - `graft changes` - List changes for dependencies
   - `graft show` - Show change details
   - `graft fetch` - Update cache
   - `graft apply` - Update lock without migration
   - `graft validate` - Validate config and lock files
   - `graft <dep>:<command>` - Execute dependency commands

5. **Atomic Upgrade Flow** - Not implemented
   - Snapshot/rollback mechanism
   - Migration execution
   - Verification execution
   - Lock file updates

## Gap Analysis Summary

| Feature | Spec | Implementation | Gap |
|---------|------|----------------|-----|
| Basic dependency resolution | ✓ | ✓ | None |
| Change model | ✓ | ✗ | Full implementation needed |
| Command system | ✓ | ✗ | Full implementation needed |
| Lock file | ✓ | ✗ | Full implementation needed |
| Atomic upgrades | ✓ | ✗ | Full implementation needed |
| Query operations | ✓ | ✗ | Full implementation needed |
| Validation | ✓ | Partial | Extend for changes/commands |

## Implementation Plan

### Phase 1: Domain Models (Foundation)

**Goal**: Extend domain layer with Change and Command entities

Tasks:
1. Create `Change` value object in `src/graft/domain/change.py`
   - Fields: ref (str), type (Optional[str]), description (Optional[str]), migration (Optional[str]), verify (Optional[str]), metadata (dict)
   - Validation rules from spec
   - Immutable dataclass

2. Create `Command` value object in `src/graft/domain/command.py`
   - Fields: name (str), run (str), description (Optional[str]), working_dir (Optional[str]), env (Optional[dict])
   - Validation rules
   - Immutable dataclass

3. Create `LockEntry` value object for lock file entries
   - Fields: source (str), ref (str), commit (str), consumed_at (datetime)
   - Validation including commit hash format

4. Extend `GraftConfig` to include:
   - `metadata: Optional[dict]` - for metadata section
   - `changes: dict[str, Change]` - ref -> Change mapping
   - `commands: dict[str, Command]` - name -> Command mapping
   - Validation that migration/verify commands exist

**Tests**: Unit tests for all new domain models with validation scenarios

### Phase 2: Configuration Parsing

**Goal**: Extend config service to parse full graft.yaml format

Tasks:
1. Update `config_service.py` to parse:
   - `metadata` section (optional)
   - `changes` section with Change objects
   - `commands` section with Command objects
   - Cross-validation (migration/verify command references)

2. Add validation for:
   - Required command fields
   - Command name uniqueness
   - Change ref format
   - Migration/verify command existence

**Tests**: Integration tests with sample graft.yaml files from spec

### Phase 3: Lock File Implementation

**Goal**: Implement graft.lock file format and operations

Tasks:
1. Create `lock_file.py` service module:
   - `read_lock_file(path) -> dict[str, LockEntry]`
   - `write_lock_file(path, entries)`
   - `update_lock_entry(path, dep_name, entry)`
   - Lock file validation

2. Create `LockFileContext` protocol for DI

3. Implement `RealLockFileOperations` adapter

**Tests**: Unit tests with fake lock file operations, integration tests with real files

### Phase 4: Command Execution

**Goal**: Implement command execution infrastructure

Tasks:
1. Create `CommandExecutionContext` protocol
   - `execute_command(command: Command, env: dict) -> CommandResult`

2. Create `command_service.py`:
   - `execute_command(context, command, args, cwd)`
   - Handle working_dir, env variables
   - Stream stdout/stderr
   - Return exit code and output

3. Create `ProcessCommandExecutor` adapter using subprocess

**Tests**: Unit tests with fake executor, integration tests with real shell commands

### Phase 5: Snapshot/Rollback Mechanism

**Goal**: Implement atomic operation support

Tasks:
1. Create `SnapshotContext` protocol:
   - `create_snapshot() -> SnapshotId`
   - `restore_snapshot(snapshot_id)`
   - `delete_snapshot(snapshot_id)`

2. Create `snapshot_service.py`:
   - File-based snapshots using temp directories
   - Git-based snapshots (stash or temp commits)
   - Selective file tracking

3. Implement `FilesystemSnapshotOperations` adapter

**Tests**: Unit and integration tests for snapshot/restore operations

### Phase 6: Core Operations - Queries

**Goal**: Implement read-only query operations

Tasks:
1. Implement `graft status`:
   - Read lock file
   - Compare with available versions
   - Display current state
   - Add `--check-updates` flag

2. Implement `graft changes`:
   - Parse dependency's graft.yaml
   - Filter changes by ref range
   - Filter by type
   - JSON and text output

3. Implement `graft show`:
   - Display full change details
   - Include command definitions
   - Reference CHANGELOG.md

4. Implement `graft fetch`:
   - Update git cache
   - Don't modify lock file

**Tests**: Integration tests with fake dependencies

### Phase 7: Core Operations - Mutations

**Goal**: Implement state-modifying operations

Tasks:
1. Implement `graft upgrade`:
   - Atomic operation flow:
     a. Validate target ref
     b. Create snapshot
     c. Update dependency files
     d. Execute migration command (if defined)
     e. Execute verification command (if defined)
     f. Update lock file
     g. On failure: rollback
   - Support `--dry-run`, `--skip-migration`, `--skip-verify`
   - Rich progress output

2. Implement `graft apply`:
   - Update lock file only
   - No migration execution
   - Warning about manual steps

3. Implement `graft validate`:
   - Schema validation
   - Ref existence validation
   - Command reference validation
   - Lock file integrity checks

**Tests**: Comprehensive integration tests for upgrade scenarios (success, migration failure, verification failure, rollback)

### Phase 8: CLI Integration

**Goal**: Wire all operations to CLI commands

Tasks:
1. Create CLI command modules in `src/graft/cli/commands/`:
   - `status.py`
   - `changes.py`
   - `show.py`
   - `fetch.py`
   - `upgrade.py`
   - `apply.py`
   - `validate.py`
   - `execute.py` (for `<dep>:<command>` syntax)

2. Update `cli/main.py` to register all commands

3. Add context factories for new services

4. Implement rich formatting with colors/tables

**Tests**: CLI integration tests

### Phase 9: Documentation

**Goal**: Document the implementation

Tasks:
1. Update `docs/README.md`:
   - Add links to graft-knowledge specs
   - Document implemented features
   - Usage examples

2. Create ADR for implementation decisions:
   - How snapshot/rollback works
   - Error handling strategy for upgrades
   - Command execution security model

3. Add implementation notes:
   - Time-bounded note documenting the sync process
   - Decisions made during implementation
   - Any deviations from spec (with rationale)

4. Update type stubs and docstrings

### Phase 10: Quality Assurance

**Goal**: Ensure high quality implementation

Tasks:
1. Run full test suite:
   - Unit tests
   - Integration tests
   - Test coverage > 90%

2. Type checking:
   - Strict mypy passes
   - No type: ignore comments

3. Linting:
   - Ruff passes with no warnings
   - Code formatted

4. Manual testing:
   - Test against real dependencies
   - Test all upgrade scenarios
   - Test error cases

## Implementation Approach

### Coding Conventions

Follow established patterns in the codebase:

1. **Functional service layer**: Services as pure functions, not classes
2. **Protocol-based DI**: Define Protocol, implement Adapter
3. **Immutable value objects**: Use frozen dataclasses
4. **Explicit contexts**: Pass dependencies via context objects
5. **Fakes over mocks**: Prefer fake implementations in tests
6. **Hybrid error handling**: Exceptions for exceptional cases, Result pattern for expected failures

### Branch Strategy

1. Create feature branch: `feature/sync-with-specification`
2. Commit frequently with descriptive messages
3. Follow existing commit conventions
4. Push to origin for review when complete

### Testing Strategy

1. **Unit tests first**: Write tests before implementation (TDD)
2. **Fakes for protocols**: Create fake implementations for testing
3. **Integration tests**: Test with real file system and git operations
4. **Edge cases**: Test validation, error paths, rollback scenarios

### Documentation Strategy

1. **Code documentation**: Comprehensive docstrings with examples
2. **Knowledge base**: Time-bounded notes in `notes/`
3. **Architecture decisions**: ADRs in `docs/decisions/`
4. **User documentation**: Updated README and guides

## Success Criteria

The implementation is complete when:

1. ✓ All domain models from spec are implemented
2. ✓ Full graft.yaml format is supported (metadata, changes, commands)
3. ✓ graft.lock file is implemented
4. ✓ All core operations work (status, changes, show, fetch, upgrade, apply, validate)
5. ✓ Command execution works
6. ✓ Atomic upgrades with rollback work
7. ✓ All tests pass with > 90% coverage
8. ✓ Mypy strict passes
9. ✓ Ruff linting passes
10. ✓ Documentation is complete
11. ✓ PR is ready for review

## Timeline Estimate

**Note**: Following the project's guidelines, we provide concrete implementation steps without time estimates. Users decide scheduling.

The work is broken into 10 phases that build on each other. Each phase has clear deliverables and can be validated independently.

## Related

- [Architecture](../docs/specifications/architecture.md) - Overall system architecture
- [Change Model Specification](../docs/specifications/graft/change-model.md)
- [graft.yaml Format Specification](../docs/specifications/graft/graft-yaml-format.md)
- [Lock File Format Specification](../docs/specifications/graft/lock-file-format.md)
- [Core Operations Specification](../docs/specifications/graft/core-operations.md)

## Next Steps

1. Review this plan with maintainers
2. Create feature branch
3. Begin Phase 1: Domain Models
4. Commit and push regularly
5. Create PR when complete

---

*This is a time-bounded implementation note. It documents the planned sync between specification and implementation as of 2026-01-03.*
