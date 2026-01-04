# Graft Development Tasks

**Last Updated**: 2026-01-04
**System**: See [docs/INFO_ARCHITECTURE.md](docs/INFO_ARCHITECTURE.md) for task management conventions

---

## 📋 Next Up (Priority Order)

### High Priority (Enhancement Features)

- [ ] **#001: Add JSON output to status command**
  - Priority: High
  - Effort: 4h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Add `--json` flag to output machine-readable JSON
  - Spec Reference: GAP_ANALYSIS.md line 29-61
  - Files: `src/graft/cli/commands/status.py`
  - Acceptance: `graft status --json` outputs valid JSON with all fields

- [ ] **#002: Add JSON output to changes command**
  - Priority: High
  - Effort: 3h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Add `--format json` option to output JSON
  - Spec Reference: GAP_ANALYSIS.md line 99-135, core-operations.md line 137-160
  - Files: `src/graft/cli/commands/changes.py`
  - Acceptance: `graft changes <dep> --format json` outputs valid JSON
  - Depends: None (can be done in parallel with #001)

- [ ] **#003: Add JSON output to show command**
  - Priority: High
  - Effort: 3h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Add `--format json` option to output JSON
  - Spec Reference: GAP_ANALYSIS.md line 147-178, core-operations.md line 235-254
  - Files: `src/graft/cli/commands/show.py`
  - Acceptance: `graft show <dep@ref> --format json` outputs valid JSON
  - Depends: None (can be done in parallel with #001, #002)

- [ ] **#004: Add --dry-run mode to upgrade command**
  - Priority: High
  - Effort: 4h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Preview upgrade without executing (show what would happen)
  - Spec Reference: GAP_ANALYSIS.md line 216-254, core-operations.md line 352, 416-417
  - Files: `src/graft/cli/commands/upgrade.py`, `src/graft/services/upgrade_service.py`
  - Acceptance: `graft upgrade <dep> --to <ref> --dry-run` shows preview without changes
  - Depends: None

---

### Medium Priority (Utility Commands)

- [ ] **#005: Implement graft validate command**
  - Priority: Medium
  - Effort: 1 day
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Validate graft.yaml and graft.lock for correctness
  - Spec Reference: GAP_ANALYSIS.md line 287-320, core-operations.md line 506-593
  - New Files: `src/graft/cli/commands/validate.py`, `src/graft/services/validation_service.py`
  - Features:
    - Validate graft.yaml schema
    - Check migration/verify commands exist
    - Check refs exist in git
    - Validate lock file consistency
  - Acceptance: `graft validate` reports validation status with clear error messages

- [ ] **#006: Implement graft fetch command**
  - Priority: Medium
  - Effort: 1 day
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Update local cache of dependency's remote state
  - Spec Reference: GAP_ANALYSIS.md line 200-214, core-operations.md line 291-335
  - New Files: `src/graft/cli/commands/fetch.py`
  - Features:
    - Fetch latest from git remote
    - Update local cache
    - Do not modify lock file
    - Show latest available versions
  - Acceptance: `graft fetch` or `graft fetch <dep>` updates cache without changing lock

- [ ] **#007: Implement graft <dep>:<command> syntax**
  - Priority: Medium
  - Effort: 4h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Execute commands from dependency's graft.yaml
  - Spec Reference: GAP_ANALYSIS.md line 337-360, core-operations.md line 597-657
  - Files: `src/graft/cli/main.py`, possibly new command handler
  - Features:
    - Parse `<dep>:<command>` syntax
    - Load command from dep's graft.yaml
    - Execute in consumer context
    - Stream stdout/stderr
  - Acceptance: `graft meta-kb:migrate-v2` executes command and streams output

---

### Low Priority (Polish & Convenience)

- [ ] **#008: Add --check-updates option to status command**
  - Priority: Low
  - Effort: 4h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Fetch latest and show available updates
  - Spec Reference: GAP_ANALYSIS.md line 475-483, core-operations.md line 30-31, 83-90
  - Files: `src/graft/cli/commands/status.py`, `src/graft/services/query_service.py`
  - Depends: #006 (graft fetch) for fetch logic
  - Acceptance: `graft status --check-updates` shows available newer versions

- [ ] **#009: Add --since alias to changes command**
  - Priority: Low
  - Effort: 1h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Convenience alias for `--from <ref> --to latest`
  - Spec Reference: GAP_ANALYSIS.md line 490-497, core-operations.md line 111
  - Files: `src/graft/cli/commands/changes.py`
  - Acceptance: `graft changes <dep> --since v1.0.0` same as `--from-ref v1.0.0 --to-ref latest`

- [ ] **#010: Add --field option to show command**
  - Priority: Low
  - Effort: 1h
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Show only specific field (migration, verify, etc.)
  - Spec Reference: GAP_ANALYSIS.md line 490-497, core-operations.md line 210-211
  - Files: `src/graft/cli/commands/show.py`
  - Acceptance: `graft show <dep>@<ref> --field migration` outputs only migration details

---

### Quality Improvements

- [ ] **#011: Add CLI integration tests**
  - Priority: Medium
  - Effort: 2 days
  - Owner: unassigned
  - Created: 2026-01-04
  - Description: Add integration tests for all CLI commands
  - Spec Reference: GAP_ANALYSIS.md line 499-508
  - New Files: `tests/integration/test_cli_*.py`
  - Coverage Goal: >80% overall (currently 61% due to CLI at 0%)
  - Features:
    - Test all 6 commands with real file I/O
    - Test error scenarios
    - Test flag combinations
    - Test output formatting
  - Acceptance: CLI coverage >80%, overall coverage >80%

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

(empty - pick a task from "Next Up" to start working!)

---

## ✅ Done (Recent)

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

- **Total Tasks**: 15 active + 4 done = 19
- **High Priority**: 4 tasks
- **Medium Priority**: 4 tasks
- **Low Priority**: 7 tasks
- **In Progress**: 0
- **Blocked**: 0
- **Done**: 4

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
