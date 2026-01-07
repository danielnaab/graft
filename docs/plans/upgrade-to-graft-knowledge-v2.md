---
title: "Plan: Upgrade to graft-knowledge v2 Specification"
date: 2026-01-05
status: draft
version: 1.0
---

# Plan: Upgrade to graft-knowledge v2 Specification

## Overview

This document outlines the plan to upgrade the Graft implementation to align with the v2 specifications defined in graft-knowledge, particularly the updated lock file format and dependency layout design.

## Context

### Current State

**Implementation** (this repository):
- Lock file format: Basic format with only `source`, `ref`, `commit`, `consumed_at` fields
- Dependency layout: `.graft/deps/<dep-name>/` structure
- Resolution: Only direct dependencies (no transitive resolution)
- Lock file scope: Only direct dependencies tracked

**Specification** (graft-knowledge repository):
- Lock file format v2.0: Extended format with `direct`, `requires`, `required_by` fields
- Dependency layout v2: Flat `.graft/<dep-name>/` structure (no `/deps/` subdirectory)
- Resolution: Recursive dependency resolution with conflict detection
- Lock file scope: ALL dependencies (direct + transitive)

### Gap Analysis

The following features are specified but not yet implemented:

1. **Transitive dependency resolution**: Currently only resolves direct dependencies from graft.yaml
2. **Extended lock file format**: Missing `direct`, `requires`, `required_by` fields
3. **Flat dependency layout**: Using `.graft/deps/` instead of `.graft/`
4. **Conflict detection**: No algorithm to detect incompatible version requirements
5. **Dependency graph tracking**: Cannot reconstruct dependency relationships

### Relevant Specifications

From graft-knowledge repository:
- `/docs/specification/lock-file-format.md` (v2.0)
- `/docs/specification/dependency-layout.md` (v2)
- `/docs/specification/core-operations.md`

## Goals

### Primary Goals

1. **Implement transitive dependency resolution**: Recursively resolve all dependencies (direct + transitive)
2. **Extend lock file format**: Add `direct`, `requires`, `required_by` fields per specification
3. **Update directory layout**: Move from `.graft/deps/` to `.graft/` flat structure
4. **Add conflict detection**: Fail explicitly when version conflicts detected
5. **Maintain backward compatibility**: Support reading old lock files during migration

### Secondary Goals

6. **Improve CLI feedback**: Show transitive dependencies in status/tree commands
7. **Add validation tooling**: Verify lock file matches actual dependency state
8. **Update documentation**: Reflect new specifications in user guides

### Success Criteria

- [ ] Can resolve graft-knowledge and its transitive dependencies (if any)
- [ ] graft.lock contains all resolved dependencies with new fields
- [ ] Dependencies cloned to `.graft/<name>/` (not `.graft/deps/<name>/`)
- [ ] Conflict detection raises clear errors for version mismatches
- [ ] All existing tests pass with updated implementation
- [ ] New tests verify transitive resolution and conflict detection

## Technical Plan

### Phase 1: Extend Domain Models

**Files to modify:**
- `/src/graft/domain/dependency.py` - Add transitive dependency fields
- `/src/graft/domain/lock_entry.py` - Add `direct`, `requires`, `required_by` fields

**Changes:**
```python
@dataclass(frozen=True)
class LockEntry:
    source: GitUrl
    ref: GitRef
    commit: CommitHash
    consumed_at: datetime
    direct: bool                    # NEW: Is this a direct dependency?
    requires: tuple[str, ...]       # NEW: Dependencies this dep needs
    required_by: tuple[str, ...]    # NEW: Dependencies that need this dep
```

**Testing:**
- Unit tests for new fields
- Serialization/deserialization with new fields

### Phase 2: Update Lock File Adapter

**Files to modify:**
- `/src/graft/adapters/lock_file.py` - YamlLockFile class

**Changes:**
- Update `_serialize_entry()` to include new fields
- Update `_deserialize_entry()` to read new fields (with defaults for old format)
- Maintain backward compatibility for reading v1 lock files

**Testing:**
- Can read old format lock files (missing new fields)
- Can write new format lock files (with new fields)
- Round-trip serialization preserves all data

### Phase 3: Implement Recursive Resolution

**Files to modify:**
- `/src/graft/services/resolution_service.py` - New `resolve_all_recursive()` function

**Algorithm:**
```python
def resolve_all_recursive(
    config: GraftConfig,
    git: Git,
    deps_dir: Path
) -> dict[str, ResolvedDependency]:
    """
    Resolve all dependencies recursively with conflict detection.

    Returns flat map of name -> ResolvedDependency suitable for graft.lock.
    Raises ConflictError if incompatible versions required.
    """
    resolved: dict[str, ResolvedDependency] = {}
    queue: list[tuple[str, DependencySpec, bool, str | None]] = []

    # Initialize with direct dependencies
    for name, spec in config.dependencies.items():
        queue.append((name, spec, True, None))  # (name, spec, is_direct, parent)

    while queue:
        name, spec, is_direct, parent = queue.pop(0)

        # Check for conflicts
        if name in resolved:
            existing = resolved[name]
            if existing.source != spec.source or existing.ref != spec.ref:
                raise ConflictError(
                    f"Dependency conflict: {name}\n"
                    f"  Required by {parent}: {spec.source}#{spec.ref}\n"
                    f"  Already resolved: {existing.source}#{existing.ref}\n"
                    f"  Required by: {', '.join(existing.required_by)}"
                )
            # Same version - just update required_by
            if parent and parent not in existing.required_by:
                existing.required_by.append(parent)
            continue

        # Clone/fetch dependency
        dep_path = deps_dir / name
        clone_or_fetch(git, spec.source, spec.ref, dep_path)

        # Read transitive dependencies
        dep_config_path = dep_path / "graft.yaml"
        transitive_deps = {}
        if dep_config_path.exists():
            dep_config = parse_graft_yaml(dep_config_path)
            transitive_deps = dep_config.dependencies or {}

        # Record resolution
        commit = git.get_commit_sha(dep_path)
        resolved[name] = ResolvedDependency(
            source=spec.source,
            ref=spec.ref,
            commit=commit,
            consumed_at=datetime.now(timezone.utc),
            direct=is_direct,
            requires=list(transitive_deps.keys()),
            required_by=[parent] if parent else []
        )

        # Queue transitive dependencies
        for trans_name, trans_spec in transitive_deps.items():
            queue.append((trans_name, trans_spec, False, name))

    return resolved
```

**Testing:**
- Test direct dependencies only (no transitive)
- Test single-level transitive dependencies
- Test multi-level transitive dependencies
- Test shared dependencies (same dep required by multiple parents)
- Test conflict detection (same name, different versions)

### Phase 4: Update Directory Layout

**Files to modify:**
- `/src/graft/services/resolution_service.py` - Change deps path from `.graft/deps/` to `.graft/`
- `/src/graft/adapters/snapshot.py` - Update snapshot directory if needed

**Migration strategy:**
- New projects: Use `.graft/<name>/` directly
- Existing projects: Add migration command `graft migrate-layout` (future enhancement)
- During transition: Support both layouts for reads, but only write new layout

**Testing:**
- Dependencies cloned to `.graft/<name>/`
- Snapshots still work correctly
- Path references in code updated

### Phase 5: Update CLI Commands

**Files to modify:**
- `/src/graft/cli/resolve.py` - Use recursive resolution
- `/src/graft/cli/status.py` - Show direct vs transitive dependencies
- `/src/graft/cli/upgrade.py` - Handle transitive dependency updates
- `/src/graft/cli/validate.py` - Validate new lock file format

**Changes:**

**resolve command:**
- Call `resolve_all_recursive()` instead of `resolve_all_dependencies()`
- Show transitive dependencies in output
- Update lock file with all resolved dependencies

**status command:**
- Distinguish direct from transitive dependencies
- Show dependency tree (optional `--tree` flag)

**upgrade command:**
- Re-resolve all dependencies after upgrade
- Show impact on transitive dependencies

**validate command:**
- Check lock file has all required fields
- Verify dependency graph consistency
- Check for orphaned dependencies

**Testing:**
- Integration tests for each command
- Test output formatting
- Test error messages

### Phase 6: Add Tree Visualization

**New file:**
- `/src/graft/cli/tree.py` - New command to visualize dependency graph

**Features:**
- Read from graft.lock (not by traversing filesystem)
- Show direct vs transitive dependencies
- Show dependency relationships
- Optional graph output format

**Example output:**
```
$ graft tree
Dependencies:
  meta-kb (direct)
    └── standards-kb (transitive)
        └── templates-kb (transitive)

$ graft tree --show-all
Dependencies:
  meta-kb (v2.0.0) [direct]
    source: git@github.com:org/meta-kb.git
    requires: standards-kb

  standards-kb (v1.5.0) [transitive via meta-kb]
    source: https://github.com/org/standards.git
    requires: templates-kb

  templates-kb (v1.0.0) [transitive via standards-kb]
    source: https://github.com/org/templates.git
    requires: (none)
```

**Testing:**
- Tree rendering with various dependency graphs
- Handling of shared dependencies
- Formatting and output correctness

### Phase 7: Documentation Updates

**Files to modify:**
- `/docs/guides/user-guide.md` - Update examples with new layout
- `/docs/configuration.md` - Document new lock file fields
- `/docs/README.md` - Update architecture overview if needed

**New files:**
- `/docs/guides/transitive-dependencies.md` - Guide on transitive dependency handling
- `/docs/guides/migration-v2.md` - Migration guide from v1 to v2 format

**Content:**
- Explain new directory layout
- Show examples of transitive dependency resolution
- Document conflict detection behavior
- Provide migration path for existing projects

## Implementation Order

### Week 1: Core Infrastructure

1. **Day 1-2**: Phase 1 (Extend Domain Models)
   - Update LockEntry with new fields
   - Update tests
   - Ensure backward compatibility

2. **Day 3-4**: Phase 2 (Update Lock File Adapter)
   - Implement serialization/deserialization
   - Test with old and new formats
   - Verify round-trip preservation

3. **Day 5**: Phase 3 (Recursive Resolution) - Part 1
   - Implement basic recursive resolution algorithm
   - Test with direct dependencies only

### Week 2: Resolution & Layout

4. **Day 1-2**: Phase 3 (Recursive Resolution) - Part 2
   - Add transitive dependency traversal
   - Implement conflict detection
   - Add comprehensive tests

5. **Day 3**: Phase 4 (Update Directory Layout)
   - Change default deps directory
   - Update path references
   - Test new layout

6. **Day 4-5**: Phase 5 (Update CLI Commands)
   - Update resolve, status, upgrade commands
   - Update validate command
   - Add integration tests

### Week 3: Polish & Documentation

7. **Day 1-2**: Phase 6 (Tree Visualization)
   - Implement tree command
   - Add formatting and output options
   - Test various dependency graphs

8. **Day 3-5**: Phase 7 (Documentation)
   - Update user guides
   - Write migration guide
   - Update configuration reference

## Testing Strategy

### Unit Tests

- Domain model serialization/deserialization
- Lock file adapter read/write
- Resolution algorithm edge cases
- Conflict detection logic

### Integration Tests

- End-to-end resolve with transitive dependencies
- Upgrade with dependency chain updates
- Validation of complete lock files
- CLI command workflows

### Manual Testing

- Clone graft repository fresh
- Add graft-knowledge as dependency
- Run `graft resolve`
- Verify:
  - All dependencies cloned to `.graft/<name>/`
  - graft.lock contains all transitive dependencies
  - Lock file has all new fields
  - Can read documentation from dependencies

## Risks & Mitigations

### Risk 1: Breaking Changes

**Risk**: Existing projects using old format may break

**Mitigation**:
- Maintain backward compatibility for reading old lock files
- Add deprecation warnings for old layout
- Provide clear migration path and documentation
- Consider adding `--legacy` flag for old behavior

### Risk 2: Circular Dependencies

**Risk**: Circular dependency graphs will cause infinite loops

**Mitigation**:
- Add cycle detection in resolution algorithm
- Fail early with clear error message
- Add tests for circular dependency detection

### Risk 3: Performance

**Risk**: Recursive resolution may be slow for large dependency graphs

**Mitigation**:
- Implement efficient visited tracking
- Parallelize git clone/fetch operations where possible
- Cache dependency configs
- Profile and optimize if needed

### Risk 4: Specification Drift

**Risk**: Implementation may diverge from specification over time

**Mitigation**:
- Regular sync with graft-knowledge repository
- Automated tests that verify spec compliance
- Clear documentation of implementation decisions
- Track implementation-specific ADRs separately

## Open Questions

1. **Version ranges**: Should we support version ranges (e.g., `^v2.0.0`) to ease conflict resolution?
   - Decision: Not in initial implementation, add if ecosystem needs it

2. **Workspace support**: Should we support monorepos with shared `.graft/` directory?
   - Decision: Defer to future enhancement

3. **Migration command**: Should we implement `graft migrate-layout` now or later?
   - Decision: Document manual migration first, add command if needed

4. **Lock file version field**: Should we add `apiVersion` to lock file to track format version?
   - Decision: Yes, add `apiVersion: graft/v0` to match graft.yaml

## Related Documents

- [graft-knowledge: Lock File Format v2.0](../../../graft-knowledge/docs/specification/lock-file-format.md)
- [graft-knowledge: Dependency Layout v2](../../../graft-knowledge/docs/specification/dependency-layout.md)
- [graft: Architecture Overview](../README.md)
- [graft: Configuration Reference](../configuration.md)

## Post-Implementation Analysis

After implementation is complete, we will create a separate analysis document to:

1. **Evaluate the upgrade process**:
   - What worked well during implementation?
   - What challenges were encountered?
   - How closely did implementation match the plan?

2. **Assess specification quality**:
   - Were the specifications clear and complete?
   - What gaps or ambiguities were discovered?
   - What improvements would help future upgrades?

3. **Identify Graft improvements**:
   - What affordances would make dependency upgrades smoother?
   - What tooling or CLI features would help?
   - What patterns emerged that should be formalized?

4. **Generate recommendations**:
   - Specific proposals for Graft enhancement
   - Process improvements for specification evolution
   - Tooling ideas for dependency management

This analysis will inform future development and ensure Graft provides effective affordances for smooth dependency management.

## Changelog

- **2026-01-05**: Initial draft (v1.0)
  - Analyzed gap between implementation and specification
  - Designed 7-phase implementation plan
  - Identified risks and mitigations
  - Defined success criteria
