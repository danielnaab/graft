---
title: "Implementation Plan: graft-knowledge Specification Compliance (2026-01-05)"
date: 2026-01-05
branch: feature/spec-2026-01-05-implementation
status: in-progress
completed_tasks: 3 of 7
---

# Implementation Plan: Specification Compliance (2026-01-05)

## Overview

This document provides a comprehensive plan to implement the graft-knowledge specification updates from 2026-01-05. The work brings the Python implementation into full compliance with the updated specifications while maintaining backward compatibility and code quality.

## Quick Start for Implementation

```bash
# Navigate to the graft repository
cd /home/coder/graft

# Checkout the implementation branch
git checkout feature/spec-2026-01-05-implementation

# Verify environment setup
export PATH="/home/coder/.local/bin:$PATH"
uv run pytest --quiet    # Should show 320 passing, 10 failing tests
uv run mypy src/         # Should pass
uv run ruff check src/   # Should pass

# Review the planning documents
cat notes/2026-01-05-spec-gap-analysis.md
cat tasks.md

# Start implementing tasks in order from tasks.md
```

---

## Executive Summary

### Specification Changes (2026-01-05)

The graft-knowledge repository received significant updates:

1. **Validation Operations Enhancement**
   - Three distinct validation modes: config, lock, integrity
   - Standardized exit codes (0, 1, 2)
   - JSON output format specification

2. **Lock File Ordering Convention**
   - Direct dependencies before transitive
   - Alphabetical sorting within groups
   - Robustness principle: strict generation, liberal parsing

3. **Decision 0005: No Partial Resolution**
   - Architectural decision confirming current approach
   - Maintains atomicity and reproducibility
   - No implementation changes required

### Implementation Status

- **Branch**: `feature/spec-2026-01-05-implementation`
- **Tests**: 346 passing, 0 failing
- **Coverage**: 42%
- **Environment**: Python 3.11, uv 0.9.21

### Work Breakdown

| Task | Priority | Status | Notes |
|------|----------|--------|-------|
| #016: Validation mode refactor | HIGH | ✅ Complete | Mode-based interface, proper integrity verification |
| #017: Lock file ordering | HIGH | ✅ Complete | Tests added, implementation verified |
| #018: Exit code standardization | HIGH | ✅ Complete | Exit codes 0/1/2 implemented |
| #019: Integrity verification | MEDIUM | ✅ Complete | Implemented in #016 |
| #020: JSON output for validate | MEDIUM | Pending | Future enhancement |
| #021: Documentation updates | MEDIUM | ✅ Complete | CLI reference updated |
| #022: Test coverage | MEDIUM | ✅ Complete | 16 new tests added |

**Completed**: 6 of 7 tasks (86%)

---

## Detailed Gap Analysis

Full analysis available in: `notes/2026-01-05-spec-gap-analysis.md`

### 1. Validation Command Refactor (Task #016)

**Current State**: Flag-based (`--schema`, `--lock`, `--refs`)
**Target State**: Mode-based (`config`, `lock`, `integrity`, `all`)

**Changes Required**:
- Add positional mode argument
- Default to `all` mode
- Maintain backward compatibility with deprecation warnings
- Update help text and documentation

**Files Affected**:
- `src/graft/cli/commands/validate.py`
- `tests/integration/cli/test_validate_command.py`

**Specification**: `core-operations.md` lines 220-371

---

### 2. Lock File Ordering (Task #017)

**Current State**: Undefined ordering (Python dict iteration order)
**Target State**: Direct dependencies first (alphabetically), then transitive (alphabetically)

**Changes Required**:
- Sort dependencies before writing lock file
- Maintain any-order parsing (already implemented)

**Files Affected**:
- `src/graft/adapters/lock_file.py`
- `tests/unit/adapters/test_lock_file.py`
- `tests/integration/test_lock_file_ordering.py` (new)

**Specification**: `lock-file-format.md` lines 83-114

---

### 3. Exit Code Standardization (Task #018)

**Current State**: Exit 1 for all failures
**Target State**: Exit 0 (success), 1 (validation error), 2 (integrity mismatch)

**Changes Required**:
- Update validate command exit codes
- Ensure consistent application across all validation paths

**Files Affected**:
- `src/graft/cli/commands/validate.py`
- `tests/integration/cli/test_validate_command.py`

**Specification**: `core-operations.md` lines 265-269

---

### 4. Enhanced Integrity Verification (Task #019)

**Current State**: Basic commit hash checking
**Target State**: Comprehensive verification using `git rev-parse HEAD`

**Changes Required**:
- Check .graft/<dep>/ directory exists
- Verify git repository validity
- Compare HEAD commit to lock file commit
- Provide detailed mismatch reports

**Files Affected**:
- `src/graft/services/validation_service.py`
- `tests/unit/services/test_validation_service.py`
- `tests/integration/test_integrity_verification.py` (new)

**Specification**: `core-operations.md` lines 258-264

---

### 5. JSON Output (Task #020)

**Current State**: No JSON output for validate command
**Target State**: `--json` flag with standardized schema

**JSON Schema**:
```json
{
  "config": {"valid": bool, "errors": []},
  "lock": {"valid": bool, "errors": []},
  "integrity": {"valid": bool, "mismatches": []},
  "overall": "passed|failed"
}
```

**Files Affected**:
- `src/graft/cli/commands/validate.py`
- `tests/integration/cli/test_validate_command.py`

**Specification**: `core-operations.md` lines 306-328

---

### 6. Documentation Updates (Task #021)

**Changes Required**:
- Update CLI reference with new validate modes
- Add examples to user guide
- Reference Decision 0005 in architecture docs
- Update continue-here.md with current status
- Create or update CHANGELOG.md

**Files Affected**:
- `docs/cli-reference.md`
- `docs/guides/user-guide.md`
- `docs/README.md`
- `continue-here.md`
- `CHANGELOG.md`

---

### 7. Test Coverage (Task #022)

**New Test Files**:
- `tests/integration/test_validate_modes.py`
- `tests/integration/test_lock_file_ordering.py`
- `tests/integration/test_integrity_verification.py`
- `tests/unit/services/test_integrity_verification.py`

**Coverage Target**: Maintain or improve 45% overall coverage

---

## Implementation Phases

### Phase 1: Core Compliance (Required)

**Priority**: HIGH
**Estimated Effort**: 10-15 hours

Tasks:
1. Task #016: Validation mode refactor
2. Task #017: Lock file ordering
3. Task #018: Exit code standardization
4. Task #021: Documentation updates (partial)
5. Task #022: Test coverage (core tests)

**Success Criteria**:
- [ ] All validation modes work as specified
- [ ] Lock files generate with correct ordering
- [ ] Exit codes match specification
- [ ] All existing tests pass (or are updated appropriately)
- [ ] Core documentation is accurate

### Phase 2: Enhancements (Optional)

**Priority**: MEDIUM
**Estimated Effort**: 9-12 hours

Tasks:
1. Task #019: Enhanced integrity verification
2. Task #020: JSON output for validate
3. Task #022: Test coverage (enhancement tests)
4. Task #021: Documentation updates (complete)

**Success Criteria**:
- [ ] Integrity verification catches all edge cases
- [ ] JSON output is well-formed and useful
- [ ] Test coverage maintained or improved
- [ ] Documentation is comprehensive

### Phase 3: Polish and Merge

**Priority**: MEDIUM
**Estimated Effort**: 2-4 hours

Tasks:
1. Final testing and validation
2. Code review and refinement
3. Documentation polish
4. Prepare pull request
5. Push to Forgejo

**Success Criteria**:
- [ ] All tests pass
- [ ] mypy strict mode passes
- [ ] ruff linting passes
- [ ] Documentation is complete
- [ ] Commit history is clean

---

## Development Environment

### System Information

```
Working Directory: /home/coder/graft
Python Version: 3.12.3
Package Manager: uv 0.9.21
Branch: feature/spec-2026-01-05-implementation
```

### Environment Setup

```bash
# Already completed - environment is ready

# Add uv to PATH
export PATH="/home/coder/.local/bin:$PATH"

# Verify installation
uv --version                    # Should show 0.9.21
python3 --version              # Should show 3.12.3

# Dependencies already installed via uv sync
# Virtual environment: /home/coder/graft/.venv
```

### Development Workflow

```bash
# Run tests
uv run pytest                           # All tests
uv run pytest tests/unit/               # Unit tests only
uv run pytest tests/integration/        # Integration tests only
uv run pytest --quiet                   # Minimal output

# Type checking
uv run mypy src/                        # Strict mode enabled

# Linting
uv run ruff check src/ tests/          # Check all files
uv run ruff format src/ tests/         # Auto-format

# Coverage
uv run pytest --cov=src --cov-report=html
# View: htmlcov/index.html

# Run graft CLI
uv run python -m graft --help
uv run python -m graft status
uv run python -m graft validate
```

---

## Architectural Patterns

The codebase follows established patterns that must be maintained:

### 1. Frozen Dataclasses

All domain models are immutable:

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class LockEntry:
    source: str
    ref: str
    commit: str
    consumed_at: datetime
    # ... other fields
```

### 2. Protocol-Based Dependency Injection

Services accept protocols, not concrete implementations:

```python
from graft.protocols.git import GitProtocol

def validate_refs_exist(
    config: Config,
    git: GitProtocol,  # Protocol, not concrete class
    repo_path: str
) -> list[str]:
    # Implementation uses git protocol methods
    pass
```

### 3. Pure Functional Services

Business logic uses pure functions, not classes:

```python
# services/validation_service.py

def validate_config_schema(config: Config) -> list[str]:
    """Pure function - no side effects."""
    errors = []
    # ... validation logic
    return errors
```

### 4. Fakes for Testing

Use in-memory test doubles instead of mocks:

```python
# tests/fakes/fake_git.py

class FakeGit:
    """In-memory git implementation for testing."""

    def __init__(self):
        self.commits = {}
        self.refs = {}

    def rev_parse(self, repo_path: str, ref: str) -> str:
        # Return fake commit from in-memory store
        pass
```

---

## Testing Strategy

### Test Hierarchy

1. **Unit Tests** - Fast, isolated, use fakes
   - Test individual functions and methods
   - No I/O, no git operations
   - Use fakes from `tests/fakes/`

2. **Integration Tests** - Slower, use real dependencies
   - Test end-to-end workflows
   - Use temporary directories
   - Actual git operations

3. **CLI Tests** - Test command-line interface
   - Use subprocess to invoke commands
   - Verify output formatting
   - Test error handling

### Test Coverage Requirements

- **Minimum**: 45% overall coverage (current level)
- **Service Layer**: 80-100% (maintain current high coverage)
- **New Code**: 80%+ coverage required

### Test File Naming

- Unit tests: `tests/unit/<module>/test_<name>.py`
- Integration tests: `tests/integration/test_<feature>.py`
- Fixtures: `tests/conftest.py`
- Fakes: `tests/fakes/fake_<protocol>.py`

---

## Quality Gates

All checks must pass before merge:

### 1. Tests

```bash
uv run pytest
# Expected: All tests pass (322+ tests)
# Exit code: 0
```

### 2. Type Checking

```bash
uv run mypy src/
# Expected: Success, no type errors
# Mode: strict
```

### 3. Linting

```bash
uv run ruff check src/ tests/
# Expected: All checks pass
# Exit code: 0
```

### 4. Coverage

```bash
uv run pytest --cov=src
# Expected: >= 45% overall
# Service layer: >= 80%
```

---

## Specification References

All specifications located in: `/home/coder/graft-knowledge/docs/`

### Primary Specifications

1. **Core Operations**: `specification/core-operations.md`
   - Validation operations: lines 220-371
   - Exit codes: lines 265-269
   - JSON schema: lines 306-328

2. **Lock File Format**: `specification/lock-file-format.md`
   - Ordering convention: lines 83-114
   - API version: lines 62-77

3. **graft.yaml Format**: `specification/graft-yaml-format.md`
   - Configuration structure
   - Validation requirements

### Decision Records

1. **Decision 0005**: `decisions/decision-0005-no-partial-resolution.md`
   - No partial dependency resolution
   - Confirms current architectural approach

### Work Logs

1. **Design Analysis**: `work-logs/2026-01-05-design-improvements-analysis.md`
   - Comprehensive analysis of all recommendations
   - Rationale for approved changes

---

## Risk Management

### Low Risk Items

1. **Lock file ordering** - Output formatting only
   - Mitigation: Parser already accepts any order
   - Rollback: Simple (remove sorting logic)

2. **Exit codes** - Numeric value changes
   - Mitigation: Well-specified, clear semantics
   - Rollback: Easy (revert to original)

### Medium Risk Items

3. **Validation mode refactor** - Command interface changes
   - Mitigation: Backward compatibility via deprecation warnings
   - Rollback: Moderate (may affect user scripts)

4. **Integrity verification** - New git operations
   - Mitigation: Read-only, no data modification
   - Rollback: Easy (disable feature)

### Mitigation Strategies

- Maintain backward compatibility where possible
- Add deprecation warnings for old interfaces
- Comprehensive test coverage for new features
- Document breaking changes clearly
- Provide migration guidance

---

## Success Metrics

### Quantitative

- [ ] All tests pass (target: 100%)
- [ ] Test coverage >= 45% (maintain current level)
- [ ] mypy strict mode: 0 errors
- [ ] ruff linting: 0 errors
- [ ] Lock file ordering: 100% of generated files
- [ ] Exit codes: 100% specification compliance

### Qualitative

- [ ] Code follows established patterns
- [ ] Validation modes work intuitively
- [ ] Error messages are clear and actionable
- [ ] Documentation is complete and accurate
- [ ] Commit history is clean and logical

---

## Open Questions

### Q1: Backward Compatibility Strategy

**Question**: Should we maintain `--schema`, `--lock`, `--refs` flags?

**Recommendation**: Yes, with deprecation warnings
- Modes are the new preferred interface
- Flags work but show deprecation notice
- Remove flags in v1.0 release

### Q2: JSON Output Schema

**Question**: What should the exact JSON schema be?

**Answer**: Follow specification from core-operations.md lines 307-328
```json
{
  "config": {"valid": bool, "errors": []},
  "lock": {"valid": bool, "errors": []},
  "integrity": {"valid": bool, "mismatches": []},
  "overall": "passed|failed"
}
```

### Q3: Fix Mode Scope

**Question**: What should `--fix` mode automatically fix?

**Recommendation**: Defer to future release
- Initial scope: Formatting only (lock file ordering, YAML formatting)
- Semantic fixes require user confirmation
- Can expand scope based on user feedback

---

## Next Steps

### Immediate (Today)

1. ✅ Review specification updates
2. ✅ Create gap analysis
3. ✅ Create implementation plan
4. ✅ Set up development environment
5. ⏳ Commit planning documents
6. ⏳ Push branch to Forgejo

### Phase 1 (Core Compliance)

1. Implement Task #016: Validation mode refactor
2. Implement Task #017: Lock file ordering
3. Implement Task #018: Exit code standardization
4. Update tests for new features
5. Update core documentation

### Phase 2 (Enhancements)

1. Implement Task #019: Enhanced integrity verification
2. Implement Task #020: JSON output
3. Complete test coverage
4. Complete documentation

### Phase 3 (Finalization)

1. Final testing and validation
2. Code review and polish
3. Prepare for merge
4. Create pull request

---

## Resources

### Documentation

- Gap Analysis: `notes/2026-01-05-spec-gap-analysis.md`
- Task Tracking: `tasks.md`
- Architecture: `docs/README.md`
- User Guide: `docs/guides/user-guide.md`
- Contributing: `docs/guides/contributing.md`

### Specifications

- graft-knowledge: `/home/coder/graft-knowledge/`
- Core Operations: `docs/specification/core-operations.md`
- Lock File Format: `docs/specification/lock-file-format.md`
- Decisions: `docs/decisions/`

### Code

- Validation Command: `src/graft/cli/commands/validate.py`
- Lock File Adapter: `src/graft/adapters/lock_file.py`
- Validation Service: `src/graft/services/validation_service.py`

---

## Contact and Support

This implementation follows the meta-knowledge-base and graft ecosystem conventions:

- Plain and professional language throughout
- High quality standards applied
- Simplicity and elegance prioritized
- Best practices from graft ecosystem followed

For questions or clarification, refer to:
- Specification documents in graft-knowledge
- Architecture decision records in graft
- Pattern examples in existing code

---

**Plan Created**: 2026-01-05
**Author**: Claude Sonnet 4.5
**Status**: Ready for implementation
**Branch**: feature/spec-2026-01-05-implementation
