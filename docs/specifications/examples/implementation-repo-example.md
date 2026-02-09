---
title: "Example: Implementation Repository Structure"
---

# Example: How graft Python Repository Would Track Specifications

This shows the recommended structure for the graft Python implementation repository.

## Repository Root Files

```
graft/  (Python implementation repo)
├── graft-spec.yaml              # Pin to specification version
├── IMPLEMENTATION-STATUS.md     # Track what's implemented
├── pyproject.toml
├── src/graft/
├── tests/
├── docs/
└── notes/                       # Implementation notes
```

## graft-spec.yaml

```yaml
# Reference to specification repository
specification:
  repository: ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git
  commit: 881fdd5a1b2c3d4e5f6g7h8i9j0k
  date: 2026-01-05

# What's implemented from this spec version
implemented:
  lock-file-format:
    status: complete
    version: "2.0"
    notes: Flat layout with ordering conventions

  core-operations:
    status: partial
    implemented:
      - validation (config, lock modes)
    planned:
      - validation (integrity mode)
      - status command
      - upgrade command

  dependency-layout:
    status: planned
    notes: Waiting for resolve command implementation

# Decisions acknowledged
decisions:
  - id: "0005"
    title: "No Partial Resolution"
    status: acknowledged
    notes: Not implementing partial resolution per architectural decision

notes: |
  Current focus: Validation operations
  Next milestone: Dependency resolution
```

## IMPLEMENTATION-STATUS.md

```markdown
# Graft Implementation Status

**Specification Version:** graft-knowledge@881fdd5 (2026-01-05)

Last updated: 2026-01-06

## Implementation Progress

### ✅ Completed

#### Lock File Format (v2.0)
- [x] Read/write graft.lock
- [x] Flat dependency layout
- [x] All required fields (source, ref, commit, consumed_at, direct, requires, required_by)
- [x] Conventional ordering (direct → transitive, alphabetical)
- [x] Parser accepts any order (robustness principle)

**Files:** `src/graft/lock_file.py`
**Tests:** `tests/test_lock_file.py`

#### Validation Operations (partial)
- [x] Config validation (graft.yaml syntax and semantics)
- [x] Lock validation (graft.lock format and consistency)
- [ ] Integrity validation (pending)

**Files:** `src/graft/cli/commands/validate.py`
**Tests:** `tests/test_validate.py`
**Spec:** `core-operations.md#validation-operations`

### 🔄 In Progress

#### Integrity Validation
**Status:** Implementing
**Spec:** `core-operations.md#validation-operations` (mode: integrity)
**Branch:** `feature/integrity-validation`
**Estimate:** 2 days

Verify `.graft/<dep>/` matches commit hashes in lock file.

### 📋 Planned (Next Sprint)

#### Dependency Resolution
**Spec:** `dependency-layout.md#recursive-dependency-resolution-strategies`
**Priority:** High
**Estimate:** 1 week

Implement recursive resolution with conflict detection.

- [ ] Clone/fetch dependencies
- [ ] Resolve transitive dependencies
- [ ] Detect version conflicts
- [ ] Generate lock file

#### Status Command
**Spec:** `core-operations.md#graft-status`
**Priority:** Medium
**Estimate:** 3 days

Show current dependency states.

### 🚫 Not Implementing

#### Partial Resolution
**Decision:** 0005 - No Partial Dependency Resolution
**Rationale:** Violates atomicity and reproducibility principles

#### Mirror Support
**Status:** Deferred
**Rationale:** Premature - will revisit if enterprise demand emerges

## Testing Coverage

- Lock file: 95%
- Validation: 80% (pending integrity mode)
- Overall: 45%

## Next Milestone: v0.2.0

**Target:** 2026-01-15

**Goals:**
- [ ] Complete integrity validation
- [ ] Implement dependency resolution
- [ ] Add status command
- [ ] Reach 70% test coverage

**Blocks:** None

**Spec Updates Needed:** None - implementing against current spec
```

## Example Code with Spec References

### src/graft/lock_file.py

```python
"""
Lock file operations per graft-knowledge specification.

Specification: graft-knowledge@881fdd5 (2026-01-05)
- lock-file-format.md - Format and semantics
- lock-file-format.md#ordering-convention - Ordering rules
"""

from dataclasses import dataclass
from datetime import datetime
from typing import Dict
import yaml


@dataclass
class LockEntry:
    """
    Lock file entry per lock-file-format.md#schema.

    Spec: graft-knowledge/lock-file-format.md (2026-01-05)
    """
    source: str  # Git URL or path
    ref: str  # Consumed git ref
    commit: str  # Resolved commit hash (40-char SHA-1)
    consumed_at: str  # ISO 8601 timestamp
    direct: bool  # Direct or transitive dependency
    requires: list[str]  # Dependencies this dep needs
    required_by: list[str]  # Dependencies that need this dep


def write_lock_file(dependencies: Dict[str, LockEntry], path: str = "graft.lock") -> None:
    """
    Write lock file with conventional ordering.

    Specification: lock-file-format.md#ordering-convention
    Version: graft-knowledge@881fdd5 (2026-01-05)

    Ordering:
    1. Direct dependencies first (alphabetical)
    2. Transitive dependencies second (alphabetical)

    Note: Parsers must accept any order per robustness principle.
    """
    # Separate direct and transitive
    direct = {k: v for k, v in dependencies.items() if v.direct}
    transitive = {k: v for k, v in dependencies.items() if not v.direct}

    lock_data = {
        "apiVersion": "graft/v0",
        "dependencies": {}
    }

    # Write direct deps first (alphabetical)
    for name in sorted(direct.keys()):
        lock_data["dependencies"][name] = _serialize_entry(direct[name])

    # Write transitive deps (alphabetical)
    for name in sorted(transitive.keys()):
        lock_data["dependencies"][name] = _serialize_entry(transitive[name])

    with open(path, "w") as f:
        yaml.dump(lock_data, f, default_flow_style=False, sort_keys=False)


# TODO: Implement integrity validation
# Spec: core-operations.md#validation-operations (mode: integrity)
# Requires: For each dep in lock file, verify .graft/<name>/ exists and git HEAD matches commit
# Priority: High (next sprint)
# Ref: graft-knowledge@881fdd5
def validate_integrity(lock_path: str = "graft.lock") -> list[str]:
    """
    Validate .graft/ directory matches lock file.

    Spec: core-operations.md#validation-operations (mode: integrity)
    TODO: Implementation pending
    """
    raise NotImplementedError(
        "See graft-knowledge@881fdd5 core-operations.md#validation-operations"
    )
```

## Example Task Tracking (TODO.md or Issues)

```markdown
# Implementation Tasks

**Spec Version:** graft-knowledge@881fdd5 (2026-01-05)

## Sprint: 2026-01-06 to 2026-01-12

### High Priority

- [x] **Config validation**
      Spec: core-operations.md#validation-operations (mode: config)
      Status: Complete
      PR: #45

- [ ] **Integrity validation**
      Spec: core-operations.md#validation-operations (mode: integrity)
      Status: In progress
      Assignee: @developer
      Branch: feature/integrity-validation
      Estimate: 2 days

      Requirements:
      - For each dep in lock file, check .graft/<name>/ exists
      - Run `git rev-parse HEAD` in dep directory
      - Compare to commit hash in lock file
      - Report mismatches with clear error messages

### Medium Priority

- [ ] **Dependency resolution**
      Spec: dependency-layout.md#recommended-design
      Status: Planned
      Estimate: 1 week

      Subtasks:
      - [ ] Implement clone/fetch logic
      - [ ] Recursive resolution algorithm
      - [ ] Conflict detection
      - [ ] Lock file generation

### Backlog

- [ ] **Status command**
      Spec: core-operations.md#graft-status

- [ ] **Tree visualization**
      Spec: core-operations.md#graft-tree

## Blocked

None

## Questions for Spec Authors

- [ ] Should integrity validation be strict (error) or warn-only by default?
      Context: Spec doesn't specify, need guidance
      Asked: 2026-01-06
```

## Example Note

### notes/2026-01-06-validation-implementation.md

```markdown
# Work Log: Validation Implementation

**Date:** 2026-01-06
**Author:** Developer Name
**Spec Reference:** graft-knowledge@881fdd5 (2026-01-05)

## Context

Implementing validation operations per core-operations.md specification update.

**Specification:**
- graft-knowledge CHANGELOG 2026-01-05
- core-operations.md#validation-operations
- Three modes: config, lock, integrity

## Implementation

### Config Validation (Complete)

Validates graft.yaml syntax and semantics:
- Parse YAML
- Check required fields
- Validate git URLs
- Verify command references

**Code:** `src/graft/cli/commands/validate.py#validate_config()`
**Tests:** `tests/test_validate.py::test_config_validation`

### Lock Validation (Complete)

Validates graft.lock format:
- Check apiVersion field
- Verify all required fields present
- Validate commit hash format (40-char hex)
- Validate ISO 8601 timestamps

**Code:** `src/graft/cli/commands/validate.py#validate_lock()`
**Tests:** `tests/test_validate.py::test_lock_validation`

### Integrity Validation (In Progress)

Verifies .graft/ matches lock file.

**Spec requirements:**
> For each dependency in lock file:
>   - Check .graft/<dep-name>/ exists
>   - Run `git rev-parse HEAD` in repository
>   - Compare to commit hash in lock file
>   - Report any mismatches

**Implementation approach:**
```python
def validate_integrity(lock_path: str) -> list[ValidationError]:
    lock = read_lock_file(lock_path)
    errors = []

    for name, dep in lock["dependencies"].items():
        dep_path = f".graft/{name}"
        if not os.path.exists(dep_path):
            errors.append(f"Missing: {name} (expected at {dep_path})")
            continue

        actual_commit = git_get_head(dep_path)
        expected_commit = dep["commit"]

        if actual_commit != expected_commit:
            errors.append(
                f"Mismatch: {name}\n"
                f"  Expected: {expected_commit}\n"
                f"  Actual:   {actual_commit}"
            )

    return errors
```

**Status:** Implementing, 80% complete
**Remaining:** Error message formatting, exit codes

## Testing

All tests passing:
- Config validation: 15 test cases
- Lock validation: 20 test cases
- Integrity validation: 8 test cases (in progress)

## Next Steps

1. Complete integrity validation implementation
2. Add integration tests
3. Update documentation
4. Move to status command implementation

## References

- Spec: graft-knowledge@881fdd5 core-operations.md#validation-operations
- Note: graft-knowledge/notes/2026-01-05-design-improvements-analysis.md
```

## Summary

This pattern enables:
- **Clean specification** - No implementation status in specs
- **Clear tracking** - Implementation status in implementation repo
- **Discoverability** - CHANGELOG shows what changed
- **Traceability** - Code references spec sections
- **Communication** - Work logs reference spec work logs
