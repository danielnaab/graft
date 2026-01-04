# Graft Development Tasks

**Last Updated**: 2026-01-04
**System**: See [docs/INFO_ARCHITECTURE.md](docs/INFO_ARCHITECTURE.md) for task management conventions

---

## 📋 Next Up (Priority Order)

### High Priority (Enhancement Features)

---

### Medium Priority (Utility Commands)


---

### Low Priority (Polish & Convenience)



---

### Quality Improvements

- [ ] **#012: Add mypy strict type checking**
  - Priority: Low
  - Effort: 4h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Enable mypy strict mode and fix any type issues
  - Files: `pyproject.toml`, potentially many files
  - Acceptance: `mypy --strict src/` passes with no errors

---

### Documentation & Organization

- [ ] **#013: Migrate status docs to status/ directory**
  - Priority: Medium
  - Effort: 2h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Organize status docs per INFO_ARCHITECTURE.md
  - Spec Reference: docs/INFO_ARCHITECTURE.md "Migration from Current State"
  - Actions:
    - Create `status/` directory
    - Move IMPLEMENTATION_STATUS.md → status/
    - Move GAP_ANALYSIS.md → status/
    - Move PHASE_8_IMPLEMENTATION.md → status/
    - Move COMPLETE_WORKFLOW.md → status/
    - Create status/sessions/ and move SESSION_LOG_*.md
    - Keep CONTINUE_HERE.md at root (most visible)
    - Update references in all docs
  - Acceptance: Clean root directory, status docs organized

- [ ] **#014: Create USER_GUIDE.md**
  - Priority: Low
  - Effort: 4h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Step-by-step tutorial for new users
  - Spec Reference: GAP_ANALYSIS.md Phase 9 section
  - New File: `docs/guides/USER_GUIDE.md`
  - Content:
    - Getting started tutorial
    - Common workflows
    - Troubleshooting
    - Best practices
  - Note: README.md covers basics well, this would be more detailed

- [ ] **#015: Create ADRs for key architectural decisions**
  - Priority: Low
  - Effort: 6h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Document key design decisions in ADR format
  - Spec Reference: GAP_ANALYSIS.md "Design Decisions That Deviate from Spec"
  - New Files: docs/decisions/00X-*.md
  - Decisions to Document:
    - Why --to is required (vs default to latest)
    - Why filesystem snapshots (vs git-based)
    - Why snapshot only graft.lock (vs full workspace)
    - Protocol-based dependency injection rationale
    - Functional service layer rationale
  - Acceptance: 5 ADRs created with Context/Decision/Consequences

---

## 🔄 In Progress

(none)

---

## ✅ Done (Recent)

- [x] **#011: Add CLI integration tests**
  - Completed: 2026-01-04 (completed across multiple sessions)
  - Owner: Claude Sonnet 4.5 (Agent)
  - Result: Comprehensive CLI integration tests for all commands
  - New Files: `tests/integration/test_cli_commands.py` (815 lines, 65+ tests)
  - Testing: All 322 tests passing
  - Coverage: Tests for resolve, fetch, status, changes, show, validate, exec commands
  - Features: Subprocess-based tests, JSON/text output validation, error scenarios, flag combinations
  - Achievement: Exceeded goal - comprehensive CLI test coverage achieved

- [x] **#008: Add --check-updates option to status command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5 (Agent)
  - Result: Added --check-updates flag to status command
  - Modified Files: `src/graft/cli/commands/status.py` (added 100+ lines)
  - Testing: All 322 tests pass (up from 320), added 2 integration tests
  - Features: Fetches latest from remote, shows current status, supports JSON output
  - Use Cases: Check for updates before upgrading, see what's available without modifying lock
  - Note: Runs git fetch but doesn't modify lock file or working directory

- [x] **#006: Implement graft fetch command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5 (Agent)
  - Result: Implemented fetch command to update local git cache
  - New Files: `src/graft/cli/commands/fetch.py` (124 lines)
  - Modified Files: `src/graft/protocols/git.py`, `src/graft/adapters/git.py`, `tests/fakes/fake_git.py`, `src/graft/cli/main.py`
  - Testing: All 320 tests pass (up from 316), added 4 integration tests
  - Features: Fetch all or specific dependency, warns if not cloned, proper error handling
  - New Protocol Method: `fetch_all()` added to GitOperations protocol
  - Note: Fetches remote-tracking branches without modifying working directory or lock file

- [x] **#005: Implement graft validate command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5 (Agent)
  - Result: Implemented validate command with schema, refs, and lock validation
  - New Files: `src/graft/cli/commands/validate.py` (228 lines), `src/graft/services/validation_service.py` (131 lines), `tests/unit/test_validation_service.py` (130 lines)
  - Modified Files: `src/graft/protocols/git.py`, `src/graft/adapters/git.py`, `tests/fakes/fake_git.py`
  - Testing: All 314 tests pass (up from 307), added 7 integration tests for validate
  - Features: Schema validation, git ref existence checking, lock file consistency, --schema/--refs/--lock flags
  - Architectural Improvement: Clarified validation separation - domain validates at construction, service validates runtime state
  - Note: Command reference validation removed from service (redundant with domain validation)

- [x] **#011: Add CLI integration tests**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5 (Agent)
  - Result: Added 23 CLI integration tests via subprocess
  - New Files: `tests/integration/test_cli_commands.py` (670 lines)
  - Testing: All 314 tests pass (up from 278)
  - Coverage: Service layer 80-98%, adapters 77-92%, domain 85-99%
  - Tests Cover: status, changes, show, exec, validate commands with JSON/text output
  - Note: CLI commands show 0% coverage (expected - thin wrappers tested via subprocess)

- [x] **#010: Add --field option to show command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: Added `--field` option to show only specific fields
  - Files Modified: `src/graft/cli/commands/show.py`, `README.md`
  - Testing: Manual testing with all fields, all tests pass (278/278)
  - Features: Supports type, description, migration, verify; works with --format json
  - Commit: (pending)

- [x] **#009: Add --since alias to changes command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: Added `--since` convenience alias for `--from-ref`
  - Files Modified: `src/graft/cli/commands/changes.py`, `README.md`
  - Testing: Manual testing with various scenarios, all tests pass (278/278)
  - Features: Validates conflicts, works with all other options
  - Commit: (pending)

- [x] **#007: Implement graft <dep>:<command> syntax**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: Added dep:command syntax for executing dependency commands
  - New Files: `src/graft/cli/commands/exec_command.py`
  - Modified Files: `src/graft/__main__.py`, `README.md`
  - Testing: Manual testing with valid/invalid commands, all tests pass (278/278)
  - Features: Parses syntax, loads from dep graft.yaml, streams output, proper error handling
  - Commit: (pending)

- [x] **#004: Add --dry-run mode to upgrade command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: Added `--dry-run` flag to preview upgrade without execution
  - Files Modified: `src/graft/cli/commands/upgrade.py`, `README.md`, `GAP_ANALYSIS.md`
  - Testing: Manual testing with multiple scenarios, all tests pass (278/278)
  - Features: Shows planned operations, respects --skip flags, clear guidance
  - Commit: (pending)

- [x] **#003: Add JSON output to show command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: Added `--format json` option to output JSON format
  - Files Modified: `src/graft/cli/commands/show.py`, `README.md`, `GAP_ANALYSIS.md`
  - Testing: Manual testing, all tests pass (278/278)
  - Commits: 68ac4a3 (implementation) + 74ab474 (docs) + 001c290 (validation)

- [x] **#002: Add JSON output to changes command**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: Added `--format json` option to output JSON format
  - Files Modified: `src/graft/cli/commands/changes.py`, `README.md`, `GAP_ANALYSIS.md`
  - Testing: Manual testing, all tests pass (278/278)
  - Commits: f1717ed (implementation) + 74ab474 (docs) + 001c290 (validation)

- [x] **#001: Add JSON output to status command**
  - Completed: 2026-01-04 (revised for consistency)
  - Owner: Claude Sonnet 4.5
  - Result: Added `--format json` option to output JSON format
  - Files Modified: `src/graft/cli/commands/status.py`, `README.md`
  - Testing: Manual testing, all tests pass (278/278)
  - Note: Initially used --json flag, revised to --format for consistency with #002, #003
  - Commits: Initial + consistency fix + validation

- [x] **#000: Phase 8 CLI Integration**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: All 6 CLI commands implemented and working
  - Commits: 64bd9f6, 4522443, cb0bf12, 0fd5fe1
  - Notes: Included dogfooding on graft itself, found and fixed 4 bugs

- [x] **Phase 9: Documentation**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: README.md and docs/README.md completely rewritten
  - Commits: 902bf6f, b736037
  - Notes: Comprehensive user and developer documentation

- [x] **Gap Analysis: Compare implementation to specification**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: GAP_ANALYSIS.md created (567 lines)
  - Commit: 88cc460
  - Findings: 85% spec compliance, production ready

- [x] **Information Architecture: Design task tracking system**
  - Completed: 2026-01-04
  - Owner: Claude Sonnet 4.5
  - Result: docs/INFO_ARCHITECTURE.md and TASKS.md created
  - Commit: (pending)
  - Notes: Following meta-knowledge-base conventions

---

## 🚫 Blocked

(none)

---

## 📦 Backlog (Not Prioritized)

- [ ] Performance profiling and optimization (if needed)
- [ ] Add progress bars for long operations
- [ ] Bash completion scripts
- [ ] Homebrew formula for installation
- [ ] Consider git-based snapshots as alternative
- [ ] Sandbox command execution (security hardening)
- [ ] Add telemetry/metrics (opt-in)

---

## 📊 Task Statistics

- **Total Tasks**: 14 active + 5 done = 19
- **High Priority**: 0 tasks
- **Medium Priority**: 3 tasks
- **Low Priority**: 11 tasks
- **In Progress**: 0
- **Blocked**: 0
- **Done**: 5

**Estimated Work Remaining**:
- High Priority: ~14 hours (1-2 days)
- Medium Priority: ~3 days
- Low Priority: ~2 days
- **Total**: ~1 week of focused work for all enhancements

---

## 🎯 Recommended Implementation Order

### Week 1: High-Value Enhancements
1. #001, #002, #003 - JSON output (can be done in parallel, ~10h total)
2. #004 - Dry-run mode (~4h)
3. #005 - Validate command (~1 day)

### Week 2: Quality & Utility
4. #011 - CLI integration tests (~2 days)
5. #006 - Fetch command (~1 day)

### Week 3: Polish
6. #007 - Command execution syntax (~4h)
7. #008, #009, #010 - CLI conveniences (~6h total)
8. #013 - Documentation organization (~2h)

---

## 📝 Notes for Future Agents

### Picking Up a Task

1. Find an unassigned task in "Next Up" that matches your capabilities
2. Move it to "In Progress" with your name and start date
3. Create a note in `notes/YYYY-MM-DD-task-name.md` for scratch work
4. Implement the feature following existing patterns
5. Write tests (unit + integration if applicable)
6. Update documentation (README.md, docs/README.md if needed)
7. Run `uv run pytest && uv run ruff check src/ tests/`
8. Commit with message: "Implement #NNN: Task title"
9. Move task to "Done" with completion date
10. Update CONTINUE_HERE.md if significant

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
- [ ] Linting passing
- [ ] Documentation updated
- [ ] Committed with clear message
- [ ] Task moved to "Done"
- [ ] Follow-up tasks created (if any)

---

**Questions? Issues with task tracking?**

See [docs/INFO_ARCHITECTURE.md](docs/INFO_ARCHITECTURE.md) for complete task management conventions.
