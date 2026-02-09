# Changelog

All notable changes to Graft specifications documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### In Progress
- Integration test strategy
- CI/CD integration examples

## [2026-01-05] - Specification Enhancements

### Added
- **Decision 0005**: No Partial Dependency Resolution - Documents architectural choice to maintain atomicity and reproducibility
- **Validation operations** in core-operations.md - Three validation modes (config, lock, integrity) with clear requirements
- **Lock file ordering conventions** in lock-file-format.md - Conventional ordering with rationale inline

### Changed
- **Simplified API version field** in lock-file-format.md to document current state (graft/v0) without premature versioning strategy
- **Removed implementation details** from core-operations.md - Specifications now focus on requirements (WHAT), not implementation (HOW)

### Removed
- Python implementation code from specifications (1,300+ lines) - Specifications are now implementation-agnostic

### Documentation
- Added work log documenting design analysis and decision rationale
- Enhanced decision index with clear categorization
- Inlined formatting conventions into specifications (not separate ADRs)

## [2026-01-01] - Initial Specifications

### Added
- **Core architecture decisions**:
  - Decision 0001: Initial Scope - Task runner + dependency manager for knowledge bases
  - Decision 0002: Git Refs Over Semver - Use git refs, don't require semantic versioning
  - Decision 0003: Explicit Change Declarations - Changes in structured YAML
  - Decision 0004: Atomic Upgrades - All-or-nothing operations with rollback

- **Specifications**:
  - Change model specification
  - graft.yaml format specification
  - Lock file format specification (v2.0 with flat layout)
  - Dependency layout specification
  - Core operations specification

- **Documentation**:
  - Architecture overview
  - Use cases
  - Decision records structure

---

## How to Reference

**For implementations:**

Reference graft-knowledge by git commit or tag:
```yaml
# In pyproject.toml or equivalent
[tool.graft.spec]
repository = "ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git"
commit = "881fdd5..."  # Or use git tag
date = "2026-01-05"
```

**For tasks and planning:**

Reference specific sections:
```markdown
- [ ] Implement validation operations
      Spec: core-operations.md#validation-operations
      Ref: graft-knowledge@881fdd5
```

**For work logs:**

Reference related specification changes:
```markdown
## Implementation: Validation Operations

Implements specification from graft-knowledge CHANGELOG 2026-01-05.
See: graft-knowledge/docs/specification/core-operations.md#validation-operations
```
