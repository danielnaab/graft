---
title: "Implementation Tracking Pattern"
date: 2026-01-05
---

# Implementation Tracking Pattern

This document defines how implementations (like the graft Python tool) track their relationship to these specifications.

## Principles

1. **Specifications are authoritative and stable** - They don't contain implementation status
2. **Implementations pin to spec versions** - Via git commit or tag
3. **Changes are discoverable** - Via CHANGELOG.md
4. **References are clear** - Standard format for linking specs to code

## Pattern: Pinning Implementation to Specification

### In the Implementation Repository

**Option A: Configuration File (Recommended)**

Create `graft-spec.yaml` at repository root:

```yaml
# Reference to specification repository
specification:
  repository: ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git
  commit: 881fdd5a1b2c3d4e5f6g7h8i9j0k  # Or use tag: "2026-01-05"
  date: 2026-01-05

# What's implemented from this spec version
implemented:
  - lock-file-format: complete
  - core-operations: partial (validation only)
  - dependency-layout: planned

notes: |
  Implements validation operations and lock file ordering.
  Next: dependency resolution with transitive deps.
```

**Option B: Metadata in pyproject.toml**

```toml
[tool.graft.specification]
repository = "ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git"
commit = "881fdd5a1b2c3d4e5f6g7h8i9j0k"
date = "2026-01-05"
```

**Option C: Simple Text File**

Create `SPEC-VERSION.txt`:

```
graft-knowledge @ 881fdd5a1b2c3d4e5f6g7h8i9j0k (2026-01-05)
Implements: lock file ordering, validation operations
```

### In Task Tracking

Reference specific specification sections:

```markdown
## Planned

- [ ] Implement dependency resolution
      Spec: dependency-layout.md#recursive-dependency-resolution-strategies
      Ref: graft-knowledge@881fdd5

- [ ] Add validation command
      Spec: core-operations.md#validation-operations
      Ref: graft-knowledge@881fdd5

## In Progress

- [x] Lock file ordering
      Spec: lock-file-format.md#ordering-convention
      Implemented: src/graft/lock_file.py#write_lock_file
```

### In Code Comments

When implementing from spec, reference the source:

```python
def write_lock_file(dependencies: dict) -> None:
    """
    Write graft.lock with conventional ordering.

    Specification: graft-knowledge/lock-file-format.md#ordering-convention
    Version: graft-knowledge@881fdd5 (2026-01-05)

    Orders dependencies as: direct first (alphabetical), then transitive (alphabetical).
    Parsers must accept any order (robustness principle).
    """
    direct = {k: v for k, v in dependencies.items() if v.direct}
    transitive = {k: v for k, v in dependencies.items() if not v.direct}

    # Write direct dependencies first, alphabetically
    for name in sorted(direct.keys()):
        ...
```

### In Work Logs

Reference specification changes that motivated implementation:

```markdown
# Work Log: 2026-01-06 - Validation Implementation

## Context

Implementing validation operations per specification update 2026-01-05.

**Specification Reference:**
- graft-knowledge CHANGELOG 2026-01-05
- core-operations.md#validation-operations
- Commit: 881fdd5

## Implementation Approach

The specification defines three validation modes:
1. config - validate graft.yaml
2. lock - validate graft.lock
3. integrity - verify .graft/ matches lock

[implementation details...]
```

## Pattern: Discovering Specification Changes

### For Implementers

**Check what's new:**

```bash
# View specification changes
cd graft-knowledge
git log --oneline --since="2 weeks ago" docs/specification/

# Or check CHANGELOG
cat CHANGELOG.md
```

**Update to latest specifications:**

```bash
# In implementation repo
cd graft
# Update spec reference in graft-spec.yaml
sed -i 's/commit: .*/commit: NEW_COMMIT_HASH/' graft-spec.yaml

# Or create git submodule (alternative pattern)
git submodule add ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git specs/
git submodule update --remote
```

### For Specification Authors

**When merging specification changes:**

1. Update CHANGELOG.md with changes
2. Commit to main
3. Optionally tag significant releases:
   ```bash
   git tag -a 2026-01-05 -m "Specification enhancements: validation, ordering"
   git push origin 2026-01-05
   ```

## Pattern: Tracking Unimplemented Features

### In Implementation Repository

**Create IMPLEMENTATION-STATUS.md:**

```markdown
# Implementation Status

Reference: graft-knowledge@881fdd5 (2026-01-05)

## Implemented

- [x] Lock file format v2.0
  - [x] Flat dependency layout
  - [x] Conventional ordering
  - [x] All required fields

- [x] Validation operations (partial)
  - [x] Config validation
  - [x] Lock file validation
  - [ ] Integrity validation (planned)

## Planned

- [ ] Dependency resolution
  - [ ] Recursive resolution
  - [ ] Conflict detection
  - [ ] Transitive dependencies

- [ ] Change tracking
  - [ ] Read changes from graft.yaml
  - [ ] Migration command execution

## Not Planned

- [ ] Workspace support (deferred per Decision 0005)
- [ ] Partial resolution (explicitly not supported per Decision 0005)
```

### In TODO Comments

```python
# TODO: Implement integrity validation
# Spec: core-operations.md#validation-operations (mode: integrity)
# Requires: git rev-parse HEAD for each dep, compare to lock file
# Priority: Medium
# Ref: graft-knowledge@881fdd5

def validate_integrity():
    raise NotImplementedError("See graft-knowledge core-operations.md#validation-operations")
```

## Anti-Patterns to Avoid

**❌ Don't put implementation status in specifications:**

```markdown
<!-- BAD - in specification -->
## Status
Currently implemented in graft v0.1.0
Python implementation: 50% complete
```

**❌ Don't put temporary markers in specifications:**

```markdown
<!-- BAD - in specification -->
## Latest Updates
**2026-01-05**: Just added validation operations!
```

**❌ Don't version specifications prematurely:**

```markdown
<!-- BAD - unnecessary versioning -->
version: 1.2.3
last-updated: 2026-01-05
```

**✅ Instead: Use CHANGELOG and git history**

## Summary

**Specifications (graft-knowledge):**
- Authoritative, stable, implementation-agnostic
- Changes tracked in CHANGELOG.md
- Tagged for significant releases
- No implementation status

**Implementation (graft Python):**
- Pins to spec via commit/tag
- Tracks implementation status separately
- References spec sections in code and tasks
- Work logs reference spec changes

**Bridge:**
- Clear reference format: `graft-knowledge@<commit>`
- Section references: `<file>.md#<section>`
- Discoverable via CHANGELOG

This pattern maintains clean separation while enabling clear tracking and communication between specifications and implementations.
