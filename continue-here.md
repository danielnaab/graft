---
status: living
purpose: "Session handoff - always reflects current state"
updated: 2026-02-10
archive_policy: "Snapshot before major milestones, keep latest"
---

# Continue Development Here

**Last Updated**: 2026-02-10
**Branch**: `main`
**Status**: Production ready (Graft CLI). Grove actively under development.

---

## Current State

### Graft (Python CLI)

All planned CLI development is complete:
- 6 CLI commands fully implemented and tested
- Atomic upgrades with automatic rollback
- 405 tests passing, ~46% coverage
- Comprehensive documentation
- Production-ready codebase

### Grove (Rust workspace tool)

Active development in `crates/grove-*/`:
- Slice 1 implemented and reviewed (two review phases completed)
- Workspace discovery, manifest parsing, and status display working
- See [AGENTS.md - Grove Section](AGENTS.md#grove-specific-guidance) for Grove-specific context
- See [docs/grove-overview.md](docs/grove-overview.md) for Grove overview
- See `docs/specifications/grove/` for Grove specifications

---

## Quick Start

```bash
# Get oriented
cd graft
git status

# Verify Graft works
uv run pytest --quiet                     # 405 tests passing
uv run mypy src/                          # Type checking
uv run ruff check src/ tests/            # Linting

# Try the CLI
uv run python -m graft --help
uv run python -m graft status
```

---

## Recent Changes

**Latest commits** (most recent first):
1. Comprehensive review of Grove Slice 1 (Phase 2)
2. Grove Slice 1: Phase 1 improvements
3. Comprehensive Slice 1 review and improvement plan
4. Add Grove submodule and implementation note
5. Update rust-starter with template bug fixes

Run `git log --oneline -10` for complete history.

---

## Key Files

### For Development
- [AGENTS.md](AGENTS.md) - Agent entrypoint with full project context
- [tasks.md](tasks.md) - Development status and completed work
- [docs/README.md](docs/README.md) - Architecture and implementation details
- [docs/guides/contributing.md](docs/guides/contributing.md) - Development workflow

### For Grove Work
- [AGENTS.md - Grove Section](AGENTS.md#grove-specific-guidance) - Grove agent guidance
- [docs/grove-overview.md](docs/grove-overview.md) - Grove overview
- [docs/specifications/grove/](docs/specifications/grove/) - Grove specifications
- [notes/index.md](notes/index.md) - Recent Grove exploration notes

### For Users
- [README.md](README.md) - Project introduction and quick start
- [docs/guides/user-guide.md](docs/guides/user-guide.md) - Detailed tutorials
- [docs/cli-reference.md](docs/cli-reference.md) - Complete command reference

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
1. Read [AGENTS.md](AGENTS.md) for project context and policies
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
4. Update documentation as needed

---

## Current Metrics

- **Tests**: 405 passing
- **Coverage**: ~46% overall (service layer: 80-100%)
- **Type Checking**: mypy strict mode enabled and passing
- **Linting**: All checks passing (ruff)

---

## Architecture Patterns

The codebase follows these patterns consistently:

1. **Frozen dataclasses** - All domain models are immutable
2. **Protocol-based DI** - Services accept protocols, not concrete types
3. **Functional services** - Pure functions, no classes for business logic
4. **Fakes for testing** - In-memory fakes instead of mocks

See [AGENTS.md](AGENTS.md) for details on both Python and Rust patterns.

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
├── grove/               # Rust workspace tool (submodule)
├── docs/                # Documentation
│   └── specifications/  # Canonical specs
└── notes/               # Exploration notes
```

---

## Next Steps

- Continue Grove development (Slice 2+)
- Performance profiling and optimization
- Progress bars for long operations
- Bash completion scripts

See [tasks.md](tasks.md) for details.
