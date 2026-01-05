---
title: "Specification Synchronization"
date: 2026-01-03
status: working
---

# Specification Synchronization

## Context

The graft-knowledge repository contains a comprehensive specification for Graft's architecture and functionality. This implementation (graft Python package) currently implements only basic dependency resolution. This note documents the plan to sync the implementation with the full specification.

## Current Implementation Status

### Implemented Features ✓

1. **Basic Dependency Resolution**
   - Parse `graft.yaml` with simple format (apiVersion + dependencies)
   - Clone git repositories
   - Git operations via SubprocessGitOperations adapter
   - `graft resolve` command

2. **Domain Model**
   - `DependencySpec` - name, git_url, git_ref
   - `DependencyResolution` - resolution state tracking
   - `GraftConfig` - configuration container
   - Value objects: `GitUrl`, `GitRef`

3. **Architecture**
   - Functional service layer
   - Protocol-based dependency injection
   - Clean separation: domain/services/protocols/adapters/cli
   - Immutable value objects (frozen dataclasses)
   - Comprehensive exception hierarchy

4. **Infrastructure**
   - Test suite with fakes (not mocks)
   - Strict mypy type checking
   - Ruff linting and formatting
   - Modern Python (3.11+) with uv

### Missing Features (from Specification)

1. **Change Model** - Track semantic changes
   - Change entity with metadata
   - Changes section in graft.yaml
   - Migration and verification command references

2. **Command System** - Execute tasks from dependencies
   - Command definitions in graft.yaml
   - Command execution infrastructure
   - Environment and working directory support

3. **Lock File** - Track consumed state
   - `graft.lock` file format
   - Commit hash integrity
   - Timestamp tracking

4. **Core Operations**
   - `graft upgrade` - Atomic upgrades with migration
   - `graft status` - Show dependency states
   - `graft changes` - List available changes
   - `graft show` - Display change details
   - `graft fetch` - Update cache
   - `graft apply` - Manual migration workflow
   - `graft validate` - Validate configs
   - `graft <dep>:<command>` - Execute commands

5. **Atomic Upgrades**
   - Snapshot/rollback mechanism
   - Migration execution
   - Verification execution
   - Transaction-like semantics

## Implementation Phases

See `/home/coder/graft-knowledge/notes/2026-01-03-python-implementation-plan.md` for detailed plan.

Summary of phases:

1. **Phase 1**: Domain Models (Change, Command, LockEntry)
2. **Phase 2**: Configuration Parsing (extend graft.yaml parser)
3. **Phase 3**: Lock File Implementation
4. **Phase 4**: Command Execution
5. **Phase 5**: Snapshot/Rollback Mechanism
6. **Phase 6**: Query Operations (status, changes, show, fetch)
7. **Phase 7**: Mutation Operations (upgrade, apply, validate)
8. **Phase 8**: CLI Integration
9. **Phase 9**: Documentation
10. **Phase 10**: Quality Assurance

## Design Decisions to Make

### 1. Snapshot Strategy

**Options**:
- A. Git-based (use git stash or temporary commits)
- B. Filesystem-based (copy files to temp directory)
- C. Hybrid (git for tracked files, fs for untracked)

**Recommendation**: Start with (B) filesystem-based for simplicity, consider (C) for production.

**Rationale**:
- Simpler to implement and test
- Doesn't require git repository
- Can snapshot arbitrary files
- Easy to extend later

### 2. Command Execution Security

**Considerations**:
- Commands run arbitrary shell code
- Need to prevent malicious commands
- User must trust dependencies

**Approach**:
- Warn users when executing commands
- Display full command before execution
- Allow dry-run mode
- Consider sandboxing in future

### 3. Lock File Location

**Options**:
- A. Project root (`graft.lock`)
- B. `.graft/lock` (hidden directory)
- C. Configurable

**Recommendation**: (A) project root

**Rationale**:
- Follows npm, cargo, poetry conventions
- Easy to find and version control
- Matches specification

### 4. Change Ordering

**Specification says**: "Declaration order in graft.yaml"

**Implementation**: Maintain order using Python 3.7+ dict ordering guarantees

## Code Organization

New files to create:

```
src/graft/
  domain/
    change.py          # NEW: Change value object
    command.py         # NEW: Command value object
    lock_entry.py      # NEW: LockEntry value object
    snapshot.py        # NEW: Snapshot types

  services/
    command_service.py    # NEW: Command execution
    lock_service.py       # NEW: Lock file operations
    snapshot_service.py   # NEW: Snapshot/rollback
    upgrade_service.py    # NEW: Atomic upgrade flow
    query_service.py      # NEW: Status, changes, show

  protocols/
    command_execution.py  # NEW: Command executor protocol
    lock_file.py          # NEW: Lock file protocol
    snapshot.py           # NEW: Snapshot protocol

  adapters/
    command_executor.py   # NEW: Process-based executor
    lock_file.py          # NEW: YAML lock file adapter
    snapshot.py           # NEW: Filesystem snapshot adapter

  cli/commands/
    status.py           # NEW
    changes.py          # NEW
    show.py             # NEW
    fetch.py            # NEW
    upgrade.py          # NEW
    apply.py            # NEW
    validate.py         # NEW
    execute.py          # NEW: <dep>:<command>
```

## Branching Strategy

1. Create feature branch: `feature/sync-with-specification`
2. Implement phases sequentially
3. Commit after each significant milestone
4. Push to origin regularly
5. Create PR when all phases complete

## Testing Strategy

Following established patterns:

1. **Unit tests** with fakes for protocols
2. **Integration tests** with real file I/O and git
3. **Edge case tests** for validation and errors
4. **Rollback tests** to ensure atomic operations work

Target: > 90% test coverage

## Documentation Updates

1. **ADRs to create**:
   - Snapshot/rollback strategy
   - Command execution security model
   - Any deviations from spec

2. **Implementation notes**:
   - This note (tracking the sync)
   - Notes for significant implementation decisions

3. **User documentation**:
   - Update README with new commands
   - Add usage examples
   - Link to specification

## Success Criteria

Implementation is complete when:

- ✓ All core operations from spec are implemented
- ✓ Full graft.yaml format is supported
- ✓ graft.lock works correctly
- ✓ Atomic upgrades with rollback work
- ✓ All tests pass (> 90% coverage)
- ✓ Type checking passes (mypy strict)
- ✓ Linting passes (ruff)
- ✓ Documentation is updated
- ✓ PR is created and ready for review

## Authority

**Canonical specification**: `/home/coder/graft-knowledge/`

All implementation decisions should be grounded in the specification. Any deviations require:
1. Clear rationale
2. Documentation in ADR
3. Consideration for future alignment

## Next Steps

1. Create feature branch
2. Begin Phase 1: Domain Models
3. Follow test-driven development
4. Commit regularly with descriptive messages
5. Document decisions as they're made

---

*This is a time-bounded note documenting the specification synchronization effort started on 2026-01-03.*
