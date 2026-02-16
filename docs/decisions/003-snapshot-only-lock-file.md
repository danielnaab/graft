---
status: accepted
date: 2026-01-04
---

# ADR 003: Snapshot Only graft.lock, Not Full Workspace

**Deciders**: Implementation team
**Context**: Phase 5 snapshot/rollback implementation

## Context

When performing atomic upgrades with rollback capability, we must decide what to snapshot:

1. **Just graft.lock**: The dependency lock file we manage
2. **Dependency directories**: The cloned repositories in `.graft/deps/`
3. **Entire workspace**: All files in the project

## Decision

We snapshot **only `graft.lock`**, not dependency directories or workspace files.

```python
# What we snapshot
snapshot_files = ["graft.lock"]

# What we DON'T snapshot
# - .graft/deps/* (dependency checkouts)
# - User files (anything outside .graft/)
# - graft.yaml (user's source file, shouldn't be modified)
```

## Consequences

### Positive

- **Fast**: Copying one small file is instant
- **Minimal Disk Usage**: Lock file is typically < 1KB
- **Clear Ownership**: We only touch what we own
- **Simple Rollback**: Just restore one file
- **Predictable**: Users know exactly what changes

### Negative

- **Partial Rollback**: If `.graft/deps/` gets corrupted, it's not restored
- **Requires Re-fetch**: After rollback, might need `graft resolve` to fix deps

### Mitigation

- Dependency directories are **read-only** to graft operations
- If deps are corrupt, `graft resolve` fixes them
- Lock file is the source of truth - everything else is derivable

## Rationale

### 1. Graft.lock is the Source of Truth

The lock file contains all necessary information to reproduce the dependency state:
- Which ref was consumed
- The commit hash
- When it was consumed

Everything in `.graft/deps/` can be reconstructed from the lock file by running `graft resolve`.

### 2. Principle of Least Surprise

Users expect dependency managers to:
- Manage lock files (package-lock.json, Gemfile.lock, poetry.lock)
- NOT modify user files
- NOT make assumptions about workspace structure

### 3. Performance

Snapshotting entire dependency directories would:
- Be slow (could be GBs of data)
- Use excessive disk space
- Provide little benefit (we can always re-fetch)

### 4. Clear Boundaries

```
User's Responsibility:
- graft.yaml (source configuration)
- All user files
- Git workflow

Graft's Responsibility:
- graft.lock (generated lock file)
- .graft/deps/ (cached dependencies - derivable)
```

## Real-World Scenario

Consider what happens during a failed upgrade:

**Before Upgrade**:
```
graft.lock: my-dep@v1.0.0 (commit: abc123)
.graft/deps/my-dep/: Contains v1.0.0 code
```

**Failed Upgrade to v2.0.0**:
```
graft.lock: PARTIALLY UPDATED (might be corrupt)
.graft/deps/my-dep/: PARTIALLY UPDATED (might be broken)
```

**After Rollback (current approach)**:
```
graft.lock: RESTORED to v1.0.0 (commit: abc123)
.graft/deps/my-dep/: Still potentially broken
```

**User runs `graft resolve`**:
```
graft.lock: v1.0.0 (unchanged)
.graft/deps/my-dep/: RE-FETCHED to match lock (now correct)
```

This is correct behavior - the lock file is the authority, and dependencies are re-synced to match it.

## Alternatives Considered

### 1. Snapshot Entire `.graft/deps/`

**Pros**: Complete restoration of dependency state
**Cons**: Slow, uses lots of disk, not necessary (can re-fetch)
**Rejected**: Performance cost outweighs benefit

### 2. Snapshot User Workspace

**Pros**: Complete rollback of everything
**Cons**: Way out of scope, interferes with user's git workflow, dangerous
**Rejected**: Not our responsibility

### 3. No Snapshots (Manual Recovery Only)

**Pros**: Simpler implementation
**Cons**: Users can't easily recover from failed upgrades
**Rejected**: Core requirement for atomic operations

## Implementation Notes

During upgrade, we:
1. Create snapshot of graft.lock
2. Perform upgrade operations
3. On success: Delete snapshot
4. On failure: Restore graft.lock from snapshot, suggest running `graft resolve`

## Related Decisions

- See ADR 002: Filesystem Snapshots for Rollback
- Complements atomic upgrade guarantee

## References

- Implementation: `src/graft/adapters/snapshot.py`
- Upgrade flow: `src/graft/services/upgrade_service.py`
- Dogfooding notes: `status/workflow-validation.md` (discovered during testing)
