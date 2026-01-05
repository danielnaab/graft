---
status: living
purpose: "Session handoff - always reflects current state"
updated: 2026-01-05
archive_policy: "Snapshot before major milestones, keep latest"
---

# Continue Development Here

**Last Updated**: 2026-01-04
**Branch**: `feature/sync-with-specification`
**Status**: Production ready - All core features complete

---

## Current State

All planned development is complete:
- 6 CLI commands fully implemented and tested
- Atomic upgrades with automatic rollback
- Complete test coverage (322 tests passing)
- Comprehensive documentation
- Production-ready codebase

See [tasks.md](tasks.md) for detailed status.

---

## Quick Start

```bash
# Get oriented
cd /home/coder/graft
git checkout feature/sync-with-specification
git status

# Verify everything works
uv run pytest --quiet                     # 322 tests passing
uv run mypy src/                          # Type checking
uv run ruff check src/ tests/            # Linting

# Try the CLI
uv run python -m graft --help
uv run python -m graft status
```

---

## Recent Changes

**Latest commits** (most recent first):
1. README restructure and documentation improvements
2. Fix ruff B017 linting errors in test files
3. Improve documentation professionalism (remove emojis, plain language)
4. Add mypy strict type checking
5. Create comprehensive user-guide.md and ADRs

Run `git log --oneline -10` for complete history.

---

## Key Files

### For Development
- [tasks.md](tasks.md) - Development status and completed work
- [docs/README.md](docs/README.md) - Architecture and implementation details
- [docs/guides/contributing.md](docs/guides/contributing.md) - Development workflow

### For Users
- [README.md](README.md) - Project introduction and quick start
- [docs/guides/user-guide.md](docs/guides/user-guide.md) - Detailed tutorials
- [docs/cli-reference.md](docs/cli-reference.md) - Complete command reference

### For Context
- [status/workflow-validation.md](status/workflow-validation.md) - End-to-end validation
- [status/implementation.md](status/implementation.md) - Detailed implementation notes
- [docs/decisions/](docs/decisions/) - Architectural decision records

---

## Available Commands

| Command | Purpose |
|---------|---------|
| `graft resolve` | Clone dependencies |
| `graft fetch` | Update remote cache |
| `graft apply` | Update lock file without migrations |
| `graft status` | Show current versions |
| `graft changes` | List available changes |
| `graft show` | Display change details |
| `graft upgrade` | Atomic upgrade with migrations |
| `graft <dep>:<cmd>` | Execute dependency command |
| `graft validate` | Validate configuration |

See [docs/cli-reference.md](docs/cli-reference.md) for detailed documentation.

---

## Development Workflow

### Before Making Changes
1. Read [docs/guides/contributing.md](docs/guides/contributing.md) for patterns
2. Check [tasks.md](tasks.md) for current status
3. Review relevant code in the same area

### While Developing
1. Follow established patterns (frozen dataclasses, protocols, pure functions)
2. Add type hints (mypy strict enabled)
3. Write tests (unit tests for services, fakes not mocks)

### Before Committing
1. Run tests: `uv run pytest`
2. Type check: `uv run mypy src/`
3. Lint: `uv run ruff check src/ tests/`
4. Update documentation as needed (see protocol in contributing.md)

---

## Current Metrics

- **Tests**: 322 passing
- **Coverage**: 45% overall (service layer: 80-100%)
- **Type Checking**: mypy strict mode enabled and passing
- **Linting**: All checks passing (ruff)

Verify with:
```bash
uv run pytest --quiet
uv run mypy src/
uv run ruff check src/ tests/
```

---

## Architecture Patterns

The codebase follows these patterns consistently:

1. **Frozen dataclasses** - All domain models are immutable
2. **Protocol-based DI** - Services accept protocols, not concrete types
3. **Functional services** - Pure functions, no classes for business logic
4. **Fakes for testing** - In-memory fakes instead of mocks

See [docs/README.md](docs/README.md) and [docs/decisions/](docs/decisions/) for details.

---

## Project Structure

```
graft/
├── src/graft/
│   ├── domain/          # Frozen dataclasses
│   ├── services/        # Pure functions
│   ├── protocols/       # Protocol interfaces
│   ├── adapters/        # Infrastructure
│   └── cli/             # CLI commands
├── tests/
│   ├── unit/            # Unit tests with fakes
│   ├── integration/     # Integration tests
│   └── fakes/           # In-memory test doubles
└── docs/                # Documentation
```

---

## Next Steps

All core features are complete. Possible future enhancements:

- Performance profiling and optimization
- Progress bars for long operations
- Bash completion scripts
- Homebrew formula for installation

See [tasks.md](tasks.md) backlog for details.

---

## Getting Help

**New to the codebase?**
Start with [README.md](README.md), then [docs/guides/contributing.md](docs/guides/contributing.md)

**Need architectural context?**
Read [docs/README.md](docs/README.md) and ADRs in [docs/decisions/](docs/decisions/)

**Debugging an issue?**
Check recent commits with `git log --oneline -10` and review related tests

---

**Ready to contribute?** Read [docs/guides/contributing.md](docs/guides/contributing.md) for development workflow.
