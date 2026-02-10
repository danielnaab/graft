---
status: living
purpose: "Track current work only - completed tasks removed"
updated: 2026-02-10
archive_policy: "Git history provides task evolution"
---

# Graft Development Tasks

**Last Updated**: 2026-02-10

---

## Next Up (Priority Order)

- [ ] #016: Grove Slice 2+ development (continuing vertical slice implementation)

---

## In Progress

- [ ] #017: Grove workspace tool development (active in `grove/` submodule, Slice 1 complete)

---

## Done (Recent)

- [x] #014: Create user-guide.md (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #012: Add mypy strict type checking (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #015: Create ADRs for architectural decisions (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #013: Migrate status docs to status/ directory (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #011: Add CLI integration tests (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #008: Add --check-updates option to status command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #006: Implement graft fetch command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #005: Implement graft validate command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #010: Add --field option to show command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #009: Add --since alias to changes command (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
- [x] #007: Implement graft <dep>:<command> syntax (Completed: 2026-01-04, Owner: Claude Sonnet 4.5)
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

---

## Project Status

Graft CLI is production-ready. Grove is under active development.

- Complete CLI implementation (all 6 commands)
- 405 tests passing, ~46% coverage
- Strict type checking (mypy strict mode enabled)
- Comprehensive documentation (README + USER_GUIDE)
- Architectural decision records (5 ADRs)
- Clean codebase (ruff linting passing)
- Grove Slice 1 implemented and reviewed (workspace discovery, manifest parsing, status display)

---

## Notes for Future Agents

### Picking Up a Task

1. Find an unassigned task in "Next Up"
2. Move to "In Progress" with your name and start date
3. Create note in `notes/YYYY-MM-DD-task-name.md` for scratch work
4. Implement the feature following existing patterns
5. Write tests (unit + integration if applicable)
6. Update documentation (README.md, docs/README.md if needed)
7. Run `uv run pytest && uv run ruff check src/ tests/`
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
- [ ] Linting passing
- [ ] Documentation updated
- [ ] Committed with clear message
- [ ] Task moved to "Done"
- [ ] Follow-up tasks created (if any)

---

See [docs/architecture.md](docs/architecture.md) for complete task management conventions.
