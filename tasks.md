---
status: living
purpose: "Track current work only - completed tasks removed"
updated: 2026-01-05
archive_policy: "Git history provides task evolution"
---

# Graft Development Tasks

**Last Updated**: 2026-01-05
**System**: See [docs/architecture.md](docs/architecture.md) for task management conventions

---

## Current Initiative: Specification Compliance (2026-01-05)

**Goal**: Implement graft-knowledge specification updates from 2026-01-05

**Specification Reference**: graft-knowledge commit from 2026-01-05
- Validation operations enhancement
- Lock file ordering conventions
- Exit code standardization

**Analysis**: See [notes/2026-01-05-spec-gap-analysis.md](notes/2026-01-05-spec-gap-analysis.md)

**Branch**: feature/spec-2026-01-05-implementation

---

## Next Up (Priority Order)

### #016: Refactor validation command to use modes ✅
**Status**: Complete (2026-01-05)
**Priority**: HIGH

Mode-based interface implemented with proper integrity verification using git rev-parse HEAD.
Legacy flags deprecated with backward compatibility maintained.

**Files**: validate.py, validation_service.py, test_validate_modes.py (11 tests)
**Tests**: 346 passing, 42% coverage

---

### #017: Implement lock file ordering convention ✅
**Status**: Complete (2026-01-05)
**Priority**: HIGH

Verified existing implementation and added comprehensive tests.
Direct dependencies before transitive, alphabetical within groups.

**Files**: test_lock_file_ordering.py (5 tests)
**Tests**: All passing, robustness principle verified

---

### #018: Standardize validation exit codes ✅
**Status**: Complete (2026-01-05) - implemented with #016
**Priority**: HIGH

Exit codes 0/1/2 implemented based on ErrorType enum.
Exit 2 for integrity mismatches with actionable error messages.

**Files**: validate.py
**Specification**: core-operations.md lines 265-269

---

### #019: Enhance integrity verification
**Priority**: MEDIUM
**Effort**: MEDIUM (3-4 hours)
**Owner**: Unassigned

**Description**:
Implement comprehensive integrity verification using `git rev-parse HEAD` to check .graft/ directories match lock file commits.

**Specification**:
- graft-knowledge/docs/specification/core-operations.md lines 258-264

**Implementation Requirements**:
1. For each dependency in lock file:
   - Check .graft/<dep-name>/ exists
   - Run `git rev-parse HEAD` in the repository
   - Compare to commit hash in lock file
   - Report detailed mismatches
2. Provide actionable error messages

**Files Affected**:
- `src/graft/services/validation_service.py` - Integrity verification function
- `tests/unit/services/test_validation_service.py` - Unit tests
- `tests/integration/test_integrity_verification.py` - Integration tests

**Acceptance Criteria**:
- [ ] Detects when .graft/<dep>/ is missing
- [ ] Detects when commit hash differs from lock file
- [ ] Reports which dependencies have mismatches
- [ ] Suggests `graft resolve` to fix
- [ ] All tests pass
- [ ] Uses exit code 2 for mismatches

**Depends On**: #016 (validation modes)

---

### #020: Add JSON output to validate command
**Priority**: MEDIUM
**Effort**: LOW (2-3 hours)
**Owner**: Unassigned

**Description**:
Add `--json` flag to validation command for machine-readable output.

**Specification**:
- graft-knowledge/docs/specification/core-operations.md lines 306-328

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
- `src/graft/cli/commands/validate.py` - JSON serialization
- `tests/integration/cli/test_validate_command.py` - JSON output tests

**Acceptance Criteria**:
- [ ] `--json` flag outputs valid JSON
- [ ] JSON schema matches specification
- [ ] Includes all validation results
- [ ] Can be piped to jq or other tools
- [ ] All tests pass

**Depends On**: #016 (validation modes), #019 (integrity verification)

---

### #021: Update documentation for specification compliance
**Priority**: MEDIUM
**Effort**: LOW (2-3 hours)
**Owner**: Unassigned

**Description**:
Update all documentation to reflect specification changes and new implementations.

**Documentation Updates**:
1. CLI reference - New validate modes
2. User guide - Examples with new syntax
3. Architecture docs - Reference Decision 0005
4. continue-here.md - Update status
5. CHANGELOG.md - Document changes

**Files Affected**:
- `docs/cli-reference.md`
- `docs/guides/user-guide.md`
- `docs/README.md`
- `continue-here.md`
- `CHANGELOG.md` (if exists, otherwise create)

**Acceptance Criteria**:
- [ ] All examples use current syntax
- [ ] Validate command documented with modes
- [ ] Lock file ordering mentioned
- [ ] Exit codes documented
- [ ] Decision 0005 referenced
- [ ] No broken links

**Depends On**: #016, #017, #018, #019, #020 (all implementation tasks)

---

### #022: Add test coverage for specification compliance
**Priority**: MEDIUM
**Effort**: MEDIUM (3-4 hours)
**Owner**: Unassigned

**Description**:
Comprehensive test suite ensuring specification compliance.

**Test Categories**:
1. Unit tests - Individual functions
2. Integration tests - End-to-end workflows
3. Regression tests - Ensure no breakage

**New Test Files**:
- `tests/integration/test_validate_modes.py`
- `tests/integration/test_lock_file_ordering.py`
- `tests/integration/test_integrity_verification.py`
- `tests/unit/services/test_integrity_verification.py`

**Acceptance Criteria**:
- [ ] All validate modes tested
- [ ] Lock file ordering tested
- [ ] Exit codes tested
- [ ] Integrity verification tested
- [ ] Edge cases covered
- [ ] Test coverage maintained or improved (≥45%)
- [ ] All tests pass

**Depends On**: #016, #017, #018, #019 (implementation tasks)

---

## In Progress

(none)

---

## Done (Recent)

- [x] #015: Create ADRs for architectural decisions (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #014: Create user-guide.md (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #013: Migrate status docs to status/ directory (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #012: Add mypy strict type checking (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #011: Add CLI integration tests (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #010: Add --field option to show command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #009: Add --since alias to changes command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #008: Add --check-updates option to status command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #007: Implement graft <dep>:<command> syntax (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #006: Implement graft fetch command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #005: Implement graft validate command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #004: Add --dry-run mode to upgrade command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #003: Add JSON output to show command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #002: Add JSON output to changes command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #001: Add JSON output to status command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #000: Phase 8 CLI Integration (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)

---

## Blocked

(none)

---

## Backlog (Not Prioritized)

- [ ] Performance profiling and optimization
- [ ] Add progress bars for long operations
- [ ] Bash completion scripts
- [ ] Homebrew formula for installation
- [ ] Consider git-based snapshots as alternative
- [ ] Sandbox command execution (security hardening)
- [ ] Add telemetry/metrics (opt-in)
- [ ] Implement `--fix` mode for validate command (deferred from spec compliance)
- [ ] Mirror support for enterprise (wait for specification update)

---

## Project Status

**Current Phase**: Specification Compliance (2026-01-05)

**Previous Status**: Production ready with all core features complete

**New Work**: Implementing specification updates from graft-knowledge
- 7 new tasks (#016-#022)
- Estimated effort: 15-23 hours (core), 24-35 hours (with enhancements)
- Priority: HIGH (specification compliance)

**Quality Metrics**:
- Tests: 322 passing (target: maintain 100%)
- Coverage: 45% overall (target: ≥45%)
- Type checking: mypy strict enabled (target: 0 errors)
- Linting: ruff passing (target: 0 errors)

---

## Notes for Future Agents

### Picking Up a Task

1. Find an unassigned task in "Next Up"
2. Move to "In Progress" with your name and start date
3. Create note in `notes/YYYY-MM-DD-task-name.md` for scratch work
4. Implement the feature following existing patterns
5. Write tests (unit + integration if applicable)
6. Update documentation (README.md, docs/README.md if needed)
7. Run `uv run pytest && uv run mypy src/ && uv run ruff check src/ tests/`
8. Commit with message: "Implement #NNN: Task title"
9. Move task to "Done" with completion date
10. Update continue-here.md if significant

### Creating New Tasks

When you discover new work:
1. Add to "Next Up" or "Backlog" with:
   - Sequential ID number
   - Clear title and description
   - Priority (High/Medium/Low)
   - Effort estimate
   - Spec references if applicable
   - Files affected
   - Acceptance criteria
2. Sort by priority
3. Consider dependencies

### Task Completion Checklist

- [ ] Feature implemented
- [ ] Tests written and passing
- [ ] Type checking passing (mypy)
- [ ] Linting passing (ruff)
- [ ] Documentation updated
- [ ] Committed with clear message
- [ ] Task moved to "Done"
- [ ] Follow-up tasks created (if any)

---

See [docs/architecture.md](docs/architecture.md) for complete task management conventions.
