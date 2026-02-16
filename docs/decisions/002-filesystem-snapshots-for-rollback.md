---
status: accepted
date: 2026-01-04
---

# ADR 002: Filesystem Snapshots for Rollback

**Deciders**: Implementation team
**Context**: Phase 5 snapshot/rollback implementation

## Context

Graft needs atomic upgrades with automatic rollback on failure. We need to preserve the state before an upgrade so we can restore it if the upgrade fails.

Two main approaches were considered:

1. **Git-based snapshots**: Use git commits/stash to track state
2. **Filesystem snapshots**: Copy files to a temporary directory

## Decision

We use **filesystem-based snapshots** by copying `graft.lock` to a timestamped directory in `.graft/snapshots/`.

```python
# Create snapshot
snapshot_id = f"upgrade-{dep_name}-{timestamp}"
snapshot_path = f".graft/snapshots/{snapshot_id}"
os.makedirs(snapshot_path, exist_ok=True)
shutil.copy2("graft.lock", f"{snapshot_path}/graft.lock")

# Restore on failure
shutil.copy2(f"{snapshot_path}/graft.lock", "graft.lock")
```

## Consequences

### Positive

- **Simplicity**: No git knowledge required
- **Clear Boundaries**: Explicitly snapshot only what we manage (graft.lock)
- **No Side Effects**: Doesn't interfere with user's git workflow
- **Easy Debugging**: Snapshots are visible files users can inspect
- **Cross-Platform**: Works anywhere Python works
- **No Dependencies**: Doesn't require git to be initialized

### Negative

- **Not Git-Aware**: Doesn't integrate with version control
- **Separate from Source Control**: Snapshots aren't versioned
- **Manual Cleanup**: Old snapshots require manual deletion (though we provide helpers)

### Neutral

- **Limited Scope**: Only snapshots graft.lock, not entire workspace
  - This is actually a feature - we don't touch user files

## Rationale

1. **Graft's Responsibility**: We only manage graft.lock, so we only snapshot that
2. **User's Workspace**: Everything else is the user's responsibility
3. **Clear Contract**: Graft is a dependency manager, not a workspace manager
4. **Simplicity**: Easier to understand, debug, and maintain

## Alternatives Considered

### 1. Git Commits

**Pros**: Integrates with version control, full history
**Cons**: Requires git init, pollutes git history, complex error handling
**Rejected**: Too invasive, assumes git usage

### 2. Git Stash

**Pros**: Temporary, doesn't pollute history
**Cons**: Can conflict with user's stash, hard to manage multiple snapshots
**Rejected**: Conflicts with user workflow

### 3. Snapshot Entire Workspace

**Pros**: Complete rollback capability
**Cons**: Slow, uses lots of disk, unclear ownership
**Rejected**: Out of scope - graft only manages dependencies

### 4. Database/SQLite for Snapshots

**Pros**: Efficient storage, queryable
**Cons**: Adds dependency, harder to debug, opaque to users
**Rejected**: Over-engineered

## Implementation Details

- Snapshots stored in `.graft/snapshots/{snapshot-id}/`
- Each snapshot contains only `graft.lock`
- Snapshot ID includes operation type and timestamp
- Automatic cleanup on successful upgrade
- Users can manually restore from snapshots if needed

## Related Decisions

- See ADR 003: Snapshot Only graft.lock (not full workspace)
- Complements atomic upgrade implementation

## References

- Implementation: `src/graft/adapters/snapshot.py`
- Service: `src/graft/services/snapshot_service.py`
- Tests: `tests/unit/test_snapshot_service.py`
