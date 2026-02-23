---
status: living
purpose: "Session handoff - always reflects current state"
updated: 2026-02-23
archive_policy: "Snapshot before major milestones, keep latest"
---

# Continue Development Here

**Last Updated**: 2026-02-23
**Branch**: `main`
**Status**: Production ready. Software factory operational. Active design work on command output state mapping.

---

## Current State

### Graft (Rust CLI) - Primary Implementation

**Production ready** - use this for all new work:
- Rust CLI in `crates/graft-*` with full feature parity
- All core operations implemented: `status`, `resolve`, `fetch`, `sync`, `apply`, `upgrade`, `validate`, `changes`, `show`, `add`, `remove`, `run`
- State queries (Stage 1) fully implemented
- 423 tests passing across workspace
- Comprehensive documentation

**Python CLI deprecated** - legacy in `src/graft/`:
- 485 tests passing (maintained for reference)
- See `src/graft/DEPRECATED.md` for migration guide
- Retained for historical reference and compatibility verification

### Grove (Rust workspace tool) - Production Ready

**Production ready** - workspace management working:
- Workspace discovery and manifest parsing
- Git status integration with TUI display
- State query system operational
- See [docs/grove-overview.md](docs/grove-overview.md) for overview
- See `docs/specifications/grove/` for specifications

### Workspace Unification Complete

The `graft-common` crate now provides shared infrastructure:
- Timeout-protected command execution
- Common git operations (fetch, checkout, rev-parse, etc.)
- State query types and cache management
- graft.yaml parsing utilities
- Used by both graft and grove to eliminate duplication

All grove documentation merged into main docs tree:
- `docs/grove-overview.md` - Grove overview
- `docs/guides/grove-user-guide.md` - User guide
- `docs/grove/implementation/` - Implementation docs
- `docs/grove/planning/` - Planning docs

---

## Quick Start

```bash
# Get oriented
cd /home/coder/src/graft
git status

# Verify Rust implementation works (primary)
cargo fmt --check                # Format check
cargo clippy -- -D warnings      # Lint
cargo test                       # 423 tests across workspace
cargo run -p graft-cli -- status # Smoke test

# Python tests (deprecated, reference only)
uv run pytest --quiet            # 485 tests
uv run mypy src/                 # Type checking
uv run ruff check src/ tests/   # Linting

# Try the Rust CLI
cargo run -p graft-cli -- --help
cargo run -p graft-cli -- status
cargo run -p grove-cli -- status
```

---

## Recent Changes

**Latest commits** (most recent first):
1. Wire software-factory commands into graft.yaml; move state mapping slices here
2. Add command-state-declarations and command-output-state-capture slices
3. Design note: command output state mapping and Grove workflow operationalization
4. Software factory: session resume, shared lib, verification step

Run `git log --oneline -10` for complete history.

---

## Key Files

### For Development
- [AGENTS.md](AGENTS.md) - Agent entrypoint with full project context
- [CLAUDE.md](CLAUDE.md) - Quick reference for Claude Code
- [tasks.md](tasks.md) - Development status and completed work
- [docs/README.md](docs/README.md) - Architecture and implementation details
- [docs/guides/contributing.md](docs/guides/contributing.md) - Development workflow

### For Graft Work
- [docs/specifications/graft/](docs/specifications/graft/) - Graft specifications
- [docs/guides/user-guide.md](docs/guides/user-guide.md) - User guide
- [docs/cli-reference.md](docs/cli-reference.md) - Command reference
- `crates/graft-*/` - Rust implementation
- `src/graft/DEPRECATED.md` - Python deprecation notice

### For Grove Work
- [docs/grove-overview.md](docs/grove-overview.md) - Grove overview
- [docs/guides/grove-user-guide.md](docs/guides/grove-user-guide.md) - User guide
- [docs/specifications/grove/](docs/specifications/grove/) - Grove specifications
- [docs/grove/implementation/](docs/grove/implementation/) - Implementation docs
- `crates/grove-*/` - Rust implementation

### For Shared Infrastructure
- `crates/graft-common/` - Shared infrastructure crate
- [docs/decisions/](docs/decisions/) - Implementation ADRs (includes workspace unification)

### For Users
- [README.md](README.md) - Project introduction and quick start
- [docs/guides/user-guide.md](docs/guides/user-guide.md) - Graft tutorials
- [docs/guides/grove-user-guide.md](docs/guides/grove-user-guide.md) - Grove tutorials
- [docs/cli-reference.md](docs/cli-reference.md) - Complete command reference

---

## Available Commands

### Graft Commands

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
| `graft add` | Add new dependency |
| `graft remove` | Remove dependency |
| `graft sync` | Sync dependencies to lock state |

### Grove Commands

| Command | Purpose |
|---------|---------|
| `grove status` | Show workspace and repository status |
| `grove config` | Display workspace configuration |

See [docs/cli-reference.md](docs/cli-reference.md) for detailed documentation.

---

## Development Workflow

### Before Making Changes
1. Read [AGENTS.md](AGENTS.md) for project context and policies
2. Check [tasks.md](tasks.md) for current status
3. Review relevant code in the same area

### While Developing (Rust - Primary)
1. Follow established patterns (newtypes, trait-based DI, library-first architecture)
2. Write tests (unit tests for services, integration tests for workflows)
3. Use `thiserror` for library errors, `anyhow` for binary errors
4. Document public APIs with doc comments

### While Developing (Python - Deprecated, Reference Only)
1. Follow established patterns (frozen dataclasses, protocols, pure functions)
2. Add type hints (mypy strict enabled)
3. Write tests (unit tests for services, fakes not mocks)

### Before Committing
```bash
# Rust (primary verification)
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Python (deprecated, optional)
uv run pytest
uv run mypy src/
uv run ruff check src/ tests/
```

---

## Current Metrics

### Rust (Primary)
- **Tests**: 74 passing across workspace
- **Type Safety**: Enforced by Rust compiler
- **Linting**: All clippy checks passing (pre-existing TUI warnings documented)

### Python (Deprecated)
- **Tests**: 485 passing
- **Coverage**: ~23% overall (service layer: 80-100%)
- **Type Checking**: mypy strict mode enabled and passing
- **Linting**: All checks passing (ruff)

---

## Architecture Patterns

### Rust Patterns (Primary)
1. **Library-first architecture** - Core logic in library crates, thin binary wrappers
2. **Trait-based boundaries** - Services accept `&impl Trait` bounds, not concrete types
3. **Newtype pattern** - Domain identity types wrap primitives for type safety
4. **Error handling as values** - `thiserror` for library errors, `anyhow` for binary errors
5. **Shared infrastructure** - `graft-common` provides shared utilities for both graft and grove

### Python Patterns (Deprecated, Reference Only)
1. **Frozen dataclasses** - All domain models are immutable
2. **Protocol-based DI** - Services accept protocols, not concrete types
3. **Functional services** - Pure functions, no classes for business logic
4. **Fakes for testing** - In-memory fakes instead of mocks

See [AGENTS.md](AGENTS.md) for details on both Python and Rust patterns.

---

## Project Structure

```
graft/
├── Cargo.toml                   # Virtual workspace manifest
├── crates/
│   ├── graft-common/            # Shared infrastructure
│   │   ├── src/
│   │   │   ├── command.rs       # Timeout-protected command execution
│   │   │   ├── git.rs           # Common git operations
│   │   │   ├── state.rs         # State query types and cache
│   │   │   └── config.rs        # graft.yaml parsing
│   │   └── Cargo.toml
│   ├── graft-core/              # Graft domain types and traits
│   ├── graft-engine/            # Graft business logic
│   ├── graft-cli/               # Graft CLI binary
│   ├── grove-core/              # Grove domain types and traits
│   ├── grove-engine/            # Grove business logic
│   └── grove-cli/               # Grove CLI binary
├── src/graft/                   # Python implementation (DEPRECATED)
│   ├── DEPRECATED.md            # Migration guide
│   ├── domain/                  # Frozen dataclasses
│   ├── services/                # Pure functions
│   ├── protocols/               # Protocol interfaces
│   ├── adapters/                # Infrastructure
│   └── cli/                     # CLI commands
├── tests/                       # Python tests (deprecated)
│   ├── unit/                    # Unit tests with fakes
│   ├── integration/             # Integration tests
│   └── fakes/                   # In-memory test doubles
├── docs/                        # Documentation
│   ├── specifications/          # Canonical specs
│   │   ├── graft/               # Graft specs
│   │   ├── grove/               # Grove specs
│   │   └── decisions/           # Spec-level ADRs
│   ├── guides/                  # User guides
│   ├── grove/                   # Grove implementation/planning docs
│   ├── decisions/               # Implementation ADRs
│   └── README.md                # Architecture overview
├── notes/                       # Exploration notes
│   ├── 2026-02-15-rust-rewrite/ # Rust rewrite session
│   └── 2026-02-16-workspace-unification/ # Workspace unification session
└── .graft/                      # Dependencies (meta-KB, starters)
```

---

## Software Factory

The software factory at `.graft/software-factory/` is operational. Factory
commands are wired directly into `graft.yaml` so you can run them from the
repo root:

```bash
graft run plan "description of what to build"   # generate a slice plan
graft run iterate <slice>                        # preview next step prompt
graft run implement <slice>                      # run Claude on next step
graft run resume <slice>                         # resume last Claude session
graft run verify                                 # run verification
```

Slices for graft/grove work live in `slices/`. Factory-internal slices
(changes to scripts/templates in `.graft/software-factory/`) live in
`.graft/software-factory/slices/`.

## Active Slices

```bash
graft state query slices   # see all slices and progress
```

| Slice | Status | Steps |
|-------|--------|-------|
| `command-state-declarations` | draft | 0/2 |
| `command-output-state-capture` | draft | 0/4 |
| `command-run-logging` | done | 5/5 |
| `iterate-command` | done | 3/3 |

Start here: `graft run implement command-state-declarations`

## Next Steps

### Active design work
- **Command output state mapping** — commands declare `writes:`/`reads:` so
  data dependencies are explicit and Grove can derive execution order.
  Design note: [notes/2026-02-23-command-output-state-mapping.md](notes/2026-02-23-command-output-state-mapping.md)
- Implement `command-state-declarations` slice first (structural foundation),
  then `command-output-state-capture` (runtime behavior + factory migration)

### Longer horizon
- Grove workflow operationalization: Grove as the layer that crystallizes
  proven ad-hoc Claude usage patterns into deterministic sequences

---

## Notes for Session Continuity

- **Primary implementation**: Use Rust crates in `crates/` for all new work
- **Python code**: Deprecated, maintained for reference only
- **Workspace unification**: Complete. Use `graft-common` for shared infrastructure
- **Documentation**: Unified structure with grove docs merged into main tree
- **Test counts**: 423 Rust tests (primary), 485 Python tests (deprecated)
- **Production status**: Both graft and grove are production ready
