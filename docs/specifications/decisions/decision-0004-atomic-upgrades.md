---
title: "Atomic Upgrades Without Partial States"
date: 2026-01-01
status: accepted
---

# Atomic Upgrades Without Partial States

## Context

When upgrading a dependency from one version to another, the process may involve:
1. Updating code/files to new version
2. Running migrations
3. Running verification checks
4. Updating lock file

These steps could fail partway through. We must decide: should Graft track intermediate states, or should upgrades be atomic (all-or-nothing)?

Two approaches:
1. **Atomic upgrades**: Upgrade succeeds completely or fails completely, no partial states
2. **Stateful upgrades**: Track intermediate states (applied but not migrated, migrated but not verified, etc.)

This decision affects:
- Lock file complexity
- Error recovery
- User mental model
- Implementation complexity

## Decision

**Upgrades are atomic operations. The lock file tracks a single state: the current consumed version. There are no intermediate states like "installed but not migrated" or "migrated but not verified".**

An upgrade either:
- **Succeeds**: All steps complete, lock file updated to new version
- **Fails**: No changes committed, lock file remains at old version

If an upgrade fails partway through, changes are rolled back and the lock file is not updated.

## Alternatives Considered

### Alternative 1: Track Installation Separately from Consumption

**Approach**: Lock file tracks two versions:

```yaml
dependencies:
  meta-kb:
    installed: "v2.0.0"    # Files present
    consumed: "v1.5.0"     # Integrated/migrated
```

**Pros**:
- Can fetch new version without migrating yet
- Clear separation of concerns
- Could support "preview" mode

**Cons**:
- **Complex mental model**: What does it mean to have v2.0.0 installed but only v1.5.0 consumed?
- **State drift**: Two versions to keep track of
- **Unclear semantics**: If v2.0.0 has breaking changes, having it "installed" would break things
- **More failure modes**: Installed and consumed out of sync

**Why rejected**: Unclear semantics, unnecessary complexity.

### Alternative 2: Track Migration/Verification State

**Approach**: Lock file tracks fine-grained states:

```yaml
dependencies:
  meta-kb:
    version: "v2.0.0"
    migrated: false      # Migration not run yet
    verified: false      # Verification not run yet
```

**Pros**:
- Could resume failed operations
- Granular state tracking
- Could skip verification if desired

**Cons**:
- **Complex state machine**: Many possible states
- **Unclear recovery**: What to do when migrated=true but verified=false?
- **Partial states**: "Applied but not verified" is ambiguous
- **Implementation complexity**: Must handle all state combinations

**Why rejected**: Creates unnecessary complexity and ambiguous states.

### Alternative 3: Multi-Step Workflow with Explicit Confirmation

**Approach**: Upgrade is multi-step, user confirms each:

```bash
$ graft apply meta-kb v2.0.0
Applied v2.0.0 (files updated, lock file not updated)

$ graft migrate meta-kb
Ran migration, ready to verify

$ graft verify meta-kb
Verification passed

$ graft finalize meta-kb
Lock file updated to v2.0.0
```

**Pros**:
- Maximum user control
- Can inspect at each step
- Explicit confirmation points

**Cons**:
- **Verbose workflow**: Many steps for common operation
- **Error-prone**: Easy to forget steps
- **Partial states**: Lock file doesn't reflect reality
- **Manual tracking**: User must remember where they are

**Why rejected**: Too many steps, too error-prone.

### Alternative 4: Rollback Log

**Approach**: Atomic upgrade, but keep rollback log:

```yaml
# .graft/rollback.yaml
last_upgrade:
  dependency: meta-kb
  from: v1.5.0
  to: v2.0.0
  timestamp: 2026-01-01T10:00:00Z
  can_rollback: true
```

Allow `graft rollback meta-kb` to undo last upgrade.

**Pros**:
- Atomic upgrades
- Can undo mistakes
- Safety net

**Cons**:
- Extra complexity
- Rollback may not always be possible
- Git already provides this (revert commits)

**Why rejected**: Git already provides version control; Graft doesn't need to duplicate it.

## Consequences

### Positive

✅ **Simple mental model**: One version in lock file = current state
✅ **Clear semantics**: Version in lock matches reality
✅ **Transactional**: Upgrade succeeds or fails cleanly
✅ **No state drift**: Installation = consumption
✅ **Easy error recovery**: Failed upgrade = no changes
✅ **Simpler implementation**: No partial state handling

### Negative

❌ **No resume**: Failed upgrade must restart from beginning
❌ **All-or-nothing**: Can't incrementally apply parts of upgrade
❌ **Coarse granularity**: Single atomic operation

### Mitigations

- **Rollback on failure**: If migration fails, automatically rollback code changes
- **Dry-run mode**: `graft upgrade --dry-run` to preview changes
- **Git integration**: Use git for actual rollback (checkout, reset)
- **Clear error messages**: Show exactly what failed and why
- **Manual escape hatch**: User can manually fix and retry

### Error Recovery Pattern

```bash
$ graft upgrade meta-kb --to v2.0.0

Running migration: migrate-v2
  Command: npx jscodeshift -t codemods/v2.js
  ✓ Processed 15 files

Running verification: verify-v2
  Command: npm test
  ✗ 3 tests failed

Upgrade failed. Rolling back changes...
  ✓ Reverted file modifications

Lock file remains at v1.5.0

To retry after fixing:
  1. Fix failing tests
  2. Run: graft upgrade meta-kb --to v2.0.0
```

## Implementation Notes

### Lock File Format

```yaml
# Simple: just one version per dependency
dependencies:
  meta-kb:
    source: "git@github.com:user/meta-kb.git"
    ref: "v1.5.0"        # Current version - that's it
    commit: "abc123"      # Resolved commit hash
    consumed_at: "2026-01-01T10:00:00Z"
```

No `installed` vs `consumed` split. No `migrated` or `verified` flags.

### Upgrade Operation

```python
def upgrade(dep: str, to_ref: str) -> bool:
    """Atomic upgrade - all or nothing."""
    # Save state for rollback
    snapshot = create_snapshot()

    try:
        # Step 1: Update files
        update_files(dep, to_ref)

        # Step 2: Run migration (if defined)
        migration_cmd = get_migration_command(dep, to_ref)
        if migration_cmd:
            execute_command(dep, migration_cmd)

        # Step 3: Run verification (if defined)
        verify_cmd = get_verification_command(dep, to_ref)
        if verify_cmd:
            execute_command(dep, verify_cmd)

        # Step 4: Update lock file (last!)
        update_lock_file(dep, to_ref)

        return True

    except Exception as e:
        # Rollback everything
        restore_snapshot(snapshot)
        log_error(f"Upgrade failed: {e}")
        return False
```

### Rollback Mechanism

Use git for rollback:

```python
def create_snapshot() -> Snapshot:
    """Create snapshot of current state."""
    return {
        'branch': get_current_branch(),
        'commit': get_current_commit(),
        'dirty_files': get_dirty_files()
    }

def restore_snapshot(snapshot: Snapshot):
    """Restore to snapshot state."""
    # Restore modified files
    run(['git', 'checkout', '--', '.'])

    # Restore lock file
    run(['git', 'checkout', 'graft.lock'])
```

Or for non-git-tracked files, use explicit backup:

```python
def create_snapshot() -> Snapshot:
    """Backup files before modification."""
    backup_dir = tempfile.mkdtemp()
    for file in get_files_to_modify():
        shutil.copy(file, backup_dir)
    return {'backup_dir': backup_dir, 'files': files}

def restore_snapshot(snapshot: Snapshot):
    """Restore files from backup."""
    for file in snapshot['files']:
        shutil.copy(
            os.path.join(snapshot['backup_dir'], file),
            file
        )
```

### Dry-Run Mode

```bash
$ graft upgrade meta-kb --to v2.0.0 --dry-run

Would perform:
  1. Update files to v2.0.0
  2. Run migration: migrate-v2
     Command: npx jscodeshift -t codemods/v2.js
  3. Run verification: verify-v2
     Command: npm test
  4. Update graft.lock: v1.5.0 → v2.0.0

No changes made (dry-run mode)
```

### Explicit Steps (If User Wants Control)

User can run migration manually:

```bash
# Don't use upgrade, do it manually
$ graft show meta-kb@v2.0.0
Ref: v2.0.0
Migration: migrate-v2

# User manually runs migration
$ graft meta-kb:migrate-v2
Running: npx jscodeshift -t codemods/v2.js
✓ Completed

# User manually verifies
$ npm test
✓ All tests pass

# User manually updates lock
$ graft apply meta-kb --to v2.0.0
Updated graft.lock: v1.5.0 → v2.0.0
```

This is an escape hatch for manual control, but not the default workflow.

## Examples

### Successful Upgrade

```bash
$ graft upgrade meta-kb --to v2.0.0

Upgrading meta-kb: v1.5.0 → v2.0.0

Running migration: migrate-v2
  ✓ Completed

Running verification: verify-v2
  ✓ All checks passed

✓ Upgrade complete
Updated graft.lock: meta-kb@v2.0.0
```

Lock file now shows v2.0.0. Done.

### Failed Upgrade

```bash
$ graft upgrade meta-kb --to v2.0.0

Upgrading meta-kb: v1.5.0 → v2.0.0

Running migration: migrate-v2
  ✓ Completed

Running verification: verify-v2
  ✗ Tests failed

Upgrade failed. Rolling back...
  ✓ Reverted changes

Lock file remains at v1.5.0
```

Lock file still shows v1.5.0. Can retry after fixing.

### Multi-Version Upgrade

```bash
$ graft upgrade meta-kb --from v1.0.0 --to v3.0.0

Found upgrade path:
  v1.0.0 → v1.5.0 → v2.0.0 → v3.0.0

Each upgrade will be atomic. Continue? [y/N] y

Upgrading v1.0.0 → v1.5.0...
  ✓ Complete

Upgrading v1.5.0 → v2.0.0...
  ✓ Complete

Upgrading v2.0.0 → v3.0.0...
  ✗ Failed

Rolled back to v2.0.0
Lock file: meta-kb@v2.0.0
```

Each hop is atomic. Stops at last successful version.

## Relation to Git

Git is also atomic:
- Commits succeed or fail
- Merges succeed or fail (conflict resolution required)
- Rebases can be aborted

Graft follows the same pattern: operations are transactional.

## Related

- [Decision 0002: Git Refs Over Semver](./decision-0002-git-refs-over-semver.md)
- [Decision 0003: Explicit Change Declarations](./decision-0003-explicit-change-declarations.md)
- [Specification: Core Operations](../graft/core-operations.md)
- [Specification: Lock File Format](../graft/lock-file-format.md)

## References

- ACID properties: https://en.wikipedia.org/wiki/ACID
- Git transactional model
- Database migration tools (Flyway, Liquibase) - similar atomic application
