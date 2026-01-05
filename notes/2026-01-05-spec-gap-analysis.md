---
title: "Gap Analysis: graft-knowledge Specification Updates (2026-01-05)"
date: 2026-01-05
status: active
purpose: "Compare specification changes against implementation and plan work"
---

# Gap Analysis: Specification Updates (2026-01-05)

## Executive Summary

The graft-knowledge repository was updated on 2026-01-05 with significant specification enhancements. This document analyzes gaps between the updated specifications and the current Python implementation, and provides an actionable implementation plan.

### Key Specification Changes

1. **Validation operations** - Three validation modes (config, lock, integrity)
2. **Lock file ordering convention** - Direct dependencies before transitive, alphabetical within groups
3. **Decision 0005** - No partial dependency resolution (architectural decision)
4. **Mirror support** (deferred) - Enterprise git mirror configuration
5. **API versioning semantics** - Simplified to graft/v0 for experimental phase

### Implementation Status

**Current**: Production-ready implementation with 322 passing tests
**Branch**: feature/spec-2026-01-05-implementation (newly created)
**Base**: graft main branch

---

## Detailed Gap Analysis

### 1. Validation Operations

**Specification**: `docs/specification/core-operations.md` (lines 220-371)

**Required Modes**:
- `graft validate config` - Validate graft.yaml syntax and semantics
- `graft validate lock` - Validate graft.lock format and consistency
- `graft validate integrity` - Verify .graft/ directory matches lock file
- `graft validate all` - Run all validations (default)

**Current Implementation**: `/home/coder/graft/src/graft/cli/commands/validate.py`

**Gap Analysis**:

Status: PARTIAL ✓

Currently implemented:
- `--schema` flag (validates config schema)
- `--refs` flag (validates git refs exist)
- `--lock` flag (validates lock file consistency)
- Partial integrity checking (checks commits match)

Missing:
1. Named modes (`config`, `lock`, `integrity`, `all`) instead of flags
2. Comprehensive integrity verification using `git rev-parse HEAD`
3. Exit code specification:
   - 0: All validations passed
   - 1: Validation failed
   - 2: Integrity mismatch (spec requirement, currently uses 1 for all)
4. JSON output format (`--json` flag)
5. Fix mode (`--fix` flag for automatic corrections)

**Implementation Impact**: MEDIUM
- Refactor validate command to use mode-based arguments
- Add integrity verification service using git rev-parse
- Update exit code handling
- Add JSON output serialization
- Document --fix mode (implement if feasible)

**Files to modify**:
- `src/graft/cli/commands/validate.py` - Command interface
- `src/graft/services/validation_service.py` - Add integrity verification
- `tests/integration/cli/test_validate_command.py` - Update tests

---

### 2. Lock File Ordering Convention

**Specification**: `docs/specification/lock-file-format.md` (lines 83-114)

**Requirement**:
Dependencies SHOULD be ordered:
1. Direct dependencies first (`direct: true`)
2. Transitive dependencies second (`direct: false`)
3. Alphabetically within each group

**Robustness Principle**: "Be strict in what you generate, liberal in what you accept"
- Implementations MUST generate ordered lock files
- Parsers MUST accept any order

**Current Implementation**: `src/graft/adapters/lock_file.py`

**Gap Analysis**:

Status: NOT IMPLEMENTED ✗

Current behavior:
- Lock files written using standard YAML dict serialization
- Order is undefined (Python dict iteration order, effectively insertion order)
- No explicit sorting logic

Missing:
1. Sort dependencies by `direct` field (true before false)
2. Within each group, sort alphabetically by dependency name
3. Ensure consistent ordering on all writes

**Implementation Impact**: LOW
- Modify `YamlLockFile.write()` to sort dependencies before serialization
- Add helper function for lock file sorting
- No changes to parsing logic (already accepts any order)
- Add tests to verify ordering

**Files to modify**:
- `src/graft/adapters/lock_file.py` - Add sorting in write method
- `tests/unit/adapters/test_lock_file.py` - Test ordering
- `tests/integration/test_lock_file_ordering.py` - Integration tests

---

### 3. Decision 0005: No Partial Dependency Resolution

**Specification**: `docs/decisions/decision-0005-no-partial-resolution.md`

**Decision**: Graft will NOT support partial dependency resolution

**Rationale**:
- Violates atomicity principle
- Breaks reproducibility
- Adds complexity without proven need

**Current Implementation**: N/A

**Gap Analysis**:

Status: COMPLIANT ✓

The current implementation does not support partial resolution and has no plans to add it. This decision confirms the current architectural approach.

**Action Required**: NONE

This is an architectural decision that validates existing implementation choices. No code changes needed.

**Documentation Impact**: LOW
- Could reference Decision 0005 in architecture docs
- Mention in FAQ or design rationale sections

---

### 4. Mirror Support (Enterprise Use Case)

**Specification**: Work log analysis (lines 220-282)

**Proposed Feature**:
```yaml
mirrors:
  - pattern: "https://github.com/*"
    replace: "https://mirror.corp/*"
```

**Status**: DEFERRED (not in current specification)

**Gap Analysis**:

Status: NOT PLANNED ✗

This feature was approved for specification but not yet added to formal specs. The work log indicates this should be specified in `graft-yaml-format.md` but the spec file has not been updated yet.

**Action Required**: DEFER

Wait for graft-knowledge to formally specify mirror support before implementation. This is an enhancement for future consideration.

**Tracking**: Monitor graft-knowledge for specification updates

---

### 5. API Version Field Semantics

**Specification**: `docs/specification/lock-file-format.md` (lines 62-77)

**Current Semantics**:
- `apiVersion: graft/v0` - Experimental phase, breaking changes allowed
- Future: `graft/v1`, `graft/v2` for stable versions

**Current Implementation**: `src/graft/adapters/lock_file.py`

**Gap Analysis**:

Status: COMPLIANT ✓

Lock files currently use `apiVersion: graft/v0` which matches the specification.

**Verification**:
```python
# In YamlLockFile.write()
data = {"apiVersion": "graft/v0", "dependencies": {...}}
```

**Action Required**: NONE

Current implementation matches specification. Future version bumps will be handled when moving from experimental to stable.

---

## Priority Matrix

| Item | Priority | Effort | Impact | Status |
|------|----------|--------|--------|--------|
| Validation mode refactor | HIGH | MEDIUM | HIGH | Required |
| Lock file ordering | HIGH | LOW | MEDIUM | Required |
| Integrity verification | MEDIUM | MEDIUM | HIGH | Enhancement |
| JSON output for validate | MEDIUM | LOW | MEDIUM | Enhancement |
| Exit code specification | HIGH | LOW | LOW | Required |
| Mirror support | LOW | HIGH | LOW | Deferred |
| Decision 0005 docs | LOW | LOW | LOW | Optional |

---

## Implementation Roadmap

### Phase 1: Core Specification Compliance (Required)

**Goal**: Bring implementation into full compliance with 2026-01-05 specifications

**Tasks**:

1. **Refactor validation command** (Priority: HIGH, Effort: MEDIUM)
   - Change from flags (`--schema`, `--lock`) to modes (`config`, `lock`, `integrity`, `all`)
   - Implement default mode (`all`)
   - Update command interface
   - Update tests
   - Update documentation

2. **Implement lock file ordering** (Priority: HIGH, Effort: LOW)
   - Add sorting logic to YamlLockFile.write()
   - Sort by: direct (true first), then alphabetical
   - Add unit tests
   - Add integration tests
   - Verify no breakage

3. **Fix exit codes** (Priority: HIGH, Effort: LOW)
   - 0: Success
   - 1: Validation error
   - 2: Integrity mismatch
   - Update validate command
   - Update tests

**Acceptance Criteria**:
- All validate modes work as specified
- Lock files generate with correct ordering
- Exit codes match specification
- All existing tests pass
- New tests cover changes

### Phase 2: Enhancements (Optional)

**Goal**: Add quality-of-life improvements from specification

**Tasks**:

4. **Add JSON output to validate** (Priority: MEDIUM, Effort: LOW)
   - Implement `--json` flag
   - Define JSON schema for validation results
   - Add tests
   - Document format

5. **Enhance integrity verification** (Priority: MEDIUM, Effort: MEDIUM)
   - Use `git rev-parse HEAD` for verification
   - Check .graft/<dep>/ exists
   - Compare to lock file commit
   - Report detailed mismatches

6. **Add fix mode** (Priority: LOW, Effort: MEDIUM)
   - Implement `--fix` flag (if feasible)
   - Auto-fix common issues where safe
   - Document limitations
   - Add tests

**Acceptance Criteria**:
- JSON output is well-formed and useful
- Integrity verification catches all mismatches
- Fix mode helps users when possible
- All features documented

### Phase 3: Documentation and Polish

**Goal**: Ensure documentation matches implementation

**Tasks**:

7. **Update documentation**
   - CLI reference with new validate modes
   - User guide examples
   - Migration guide (if breaking changes)
   - Architecture docs

8. **Reference Decision 0005**
   - Link from architecture.md
   - Mention in FAQ
   - Explain why no partial resolution

9. **Update continue-here.md and tasks.md**
   - Reflect new implementation status
   - Update metrics
   - Note spec version compliance

**Acceptance Criteria**:
- Documentation is accurate and complete
- Examples work as shown
- Users can find information easily

---

## Testing Strategy

### Unit Tests

**New test files needed**:
- `tests/unit/services/test_integrity_verification.py` - Integrity checks
- `tests/unit/adapters/test_lock_file_ordering.py` - Ordering logic

**Modified test files**:
- `tests/unit/cli/test_validate_command.py` - Mode-based validation
- All existing tests (verify no regression)

### Integration Tests

**New integration tests**:
- `tests/integration/test_validate_modes.py` - End-to-end validation
- `tests/integration/test_lock_file_ordering.py` - Lock file generation
- `tests/integration/test_integrity_verification.py` - Full integrity workflow

### Validation Criteria

All tests must pass:
```bash
uv run pytest                    # All tests pass
uv run mypy src/                # Type checking passes
uv run ruff check src/ tests/   # Linting passes
```

---

## Risk Assessment

### Low Risk Items

1. **Lock file ordering** - Pure output formatting change
   - Mitigation: Parser already accepts any order
   - Rollback: Easy (just remove sorting)

2. **Exit codes** - Simple numeric changes
   - Mitigation: Well-defined in specification
   - Rollback: Easy (revert to original codes)

### Medium Risk Items

3. **Validation mode refactor** - Changes command interface
   - Mitigation: Keep backward compatibility where possible
   - Rollback: Moderate (affects user scripts)

4. **Integrity verification** - New git operations
   - Mitigation: Read-only operations, no data changes
   - Rollback: Easy (disable feature)

### High Risk Items

None identified. All changes are additive or refinements to existing features.

---

## Dependencies and Constraints

### External Dependencies

- **graft-knowledge**: Source of truth for specifications
  - Version: Commit from 2026-01-05
  - Location: `/home/coder/graft-knowledge`
  - Must track specification changes

- **Python environment**: Currently using uv for dependency management
  - Python 3.11+
  - All deps in pyproject.toml

### Internal Dependencies

- **Existing services**: validation_service, lock_service, config_service
- **Protocol contracts**: Must maintain protocol compatibility
- **Test infrastructure**: Fakes and fixtures

### Constraints

1. **Backward compatibility**: Lock files must remain parseable by older versions
2. **Type safety**: Must pass mypy strict mode
3. **Test coverage**: Maintain or improve 45% overall coverage
4. **Zero breaking changes**: Users should not need to change workflows

---

## Success Metrics

### Quantitative Metrics

- [ ] All tests pass (target: 100%)
- [ ] Test coverage maintained or improved (current: 45%, target: ≥45%)
- [ ] mypy strict mode passes (target: 0 errors)
- [ ] ruff linting passes (target: 0 errors)
- [ ] Lock files ordered correctly (target: 100% of generated files)
- [ ] Exit codes match spec (target: 100% compliance)

### Qualitative Metrics

- [ ] Validation modes work intuitively
- [ ] Error messages are clear and actionable
- [ ] Documentation is complete and accurate
- [ ] Code follows established patterns
- [ ] Commit history is clean and logical

---

## Open Questions

### Q1: Backward compatibility for validation command

**Question**: Should we maintain `--schema`, `--lock`, `--refs` flags for backward compatibility?

**Options**:
A. Remove flags entirely, use modes only
B. Keep flags as aliases to modes
C. Support both, with modes as preferred

**Recommendation**: Option C - Support both for smooth transition
- Modes are the new preferred interface
- Flags work but show deprecation warning
- Remove flags in v1.0 release

### Q2: JSON output schema

**Question**: What should the JSON output format be for `graft validate --json`?

**Proposal**:
```json
{
  "config": {
    "valid": true,
    "errors": [],
    "warnings": []
  },
  "lock": {
    "valid": true,
    "errors": [],
    "warnings": []
  },
  "integrity": {
    "valid": true,
    "mismatches": []
  },
  "overall": "passed|failed"
}
```

**Decision**: Match specification format from core-operations.md lines 307-328

### Q3: Fix mode scope

**Question**: What should `graft validate --fix` actually fix?

**Options**:
A. Only formatting (lock file ordering, YAML formatting)
B. Some semantic issues (update commit hashes, resolve simple conflicts)
C. Defer to future release

**Recommendation**: Option A initially
- Lock file reordering is safe to auto-fix
- YAML formatting improvements are safe
- Semantic fixes are risky and should require user confirmation
- Can expand scope in future releases

---

## Timeline Estimate

### Phase 1: Core Compliance (Required)
- Validation refactor: 4-6 hours
- Lock file ordering: 2-3 hours
- Exit codes: 1-2 hours
- Testing: 3-4 hours
- **Total: 10-15 hours**

### Phase 2: Enhancements (Optional)
- JSON output: 2-3 hours
- Integrity verification: 3-4 hours
- Fix mode: 4-5 hours
- **Total: 9-12 hours**

### Phase 3: Documentation (Required)
- Update docs: 2-3 hours
- Examples and guides: 2-3 hours
- Polish and review: 1-2 hours
- **Total: 5-8 hours**

### Grand Total
- **Minimum (Phase 1 + Phase 3)**: 15-23 hours
- **Full implementation**: 24-35 hours

---

## Next Steps

1. **Review this analysis** with stakeholders (if any)
2. **Create implementation plan** in tasks.md
3. **Set up development environment** (verify uv, deps, etc.)
4. **Start Phase 1 implementation**
5. **Iterate with tests** and validation
6. **Document changes** as they're made
7. **Prepare for merge** to main branch

---

## References

### Specifications
- `/home/coder/graft-knowledge/docs/specification/core-operations.md`
- `/home/coder/graft-knowledge/docs/specification/lock-file-format.md`
- `/home/coder/graft-knowledge/docs/specification/graft-yaml-format.md`
- `/home/coder/graft-knowledge/docs/decisions/decision-0005-no-partial-resolution.md`

### Work Logs
- `/home/coder/graft-knowledge/docs/work-logs/2026-01-05-design-improvements-analysis.md`
- `/home/coder/graft-knowledge/CHANGELOG.md`

### Implementation Files
- `/home/coder/graft/src/graft/cli/commands/validate.py`
- `/home/coder/graft/src/graft/adapters/lock_file.py`
- `/home/coder/graft/src/graft/services/validation_service.py`

---

**Analysis completed**: 2026-01-05
**Author**: Claude Sonnet 4.5
**Status**: Ready for implementation
