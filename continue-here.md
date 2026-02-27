---
status: living
purpose: "Session handoff - always reflects current state"
updated: 2026-02-23
archive_policy: "Snapshot before major milestones, keep latest"
---

# Continue Development Here

**Last Updated**: 2026-02-27
**Branch**: `main`
**Status**: Grove TUI refactor in progress (Phase 1 complete). Transcript paradigm replacing spatial dashboard.

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

### Grove TUI Refactor — Phase 1 Complete (2026-02-27)

Replaced the old spatial dashboard TUI (~6300 lines, view stacks, overlays, cursor models)
with a transcript paradigm (~4100 lines). Scrolling content area + command prompt at bottom,
similar to Claude Code / OpenCode.

**What was done:**
- Created: `scroll_buffer.rs`, `transcript.rs`, `header.rs`, `prompt.rs`, `formatting.rs`
- Rewrote: `mod.rs`, `command_line.rs` (stripped `impl App` blocks), `status_bar.rs`, `tests.rs`
- Deleted: `app.rs`, `render.rs`, `repo_list.rs`, `hint_bar.rs`, `overlays.rs`, `repo_detail.rs`
- Kept unchanged: `text_buffer.rs`, `command_exec.rs`, `state/`

**Working commands:** `:repos`, `:repo <name|idx>`, `:help`, `:refresh`/`:r`, `:quit`/`:q`,
`:run <cmd>`, plus `j/k` scroll, `Tab/BackTab` focus, `Enter` collapse toggle.

**Verification:** `cargo fmt --check` clean, `cargo clippy -- -D warnings` clean,
88 unit tests + 26 integration tests all passing.

**Known issues to fix before Phase 2:**
- Palette Enter handler has dead branch (`buffer.is_empty()` guard prevents palette selection)
- Palette filtering uses substring instead of prefix matching
- Silent failures when `graft.yaml` parsing errors occur
- Output pushed as immutable Text blocks (needs streaming Output block for Phase 3)
- See review notes in plan file for full list

**Plan:** 5-phase refactor. Phases 2-5 remain (status/state/catalog commands, command
execution with streaming output, sequences/approvals, test migration/cleanup).
Plan file: `.claude/plans/snazzy-skipping-gosling.md`

**Previous commits** (pre-refactor):
1. Wire software-factory commands into graft.yaml; move state mapping slices here
2. Add command-state-declarations and command-output-state-capture slices

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
| `grove-run-state-view` | ready | 0/4 |
| `sequence-declarations` | draft | — |
| `dependency-graph` | draft | — |
| `sequence-resumability` | draft | — |
| `command-state-declarations` | done | 2/2 |
| `command-output-state-capture` | done | 4/4 |
| `command-run-logging` | done | 5/5 |
| `iterate-command` | done | 3/3 |

Start here: `graft run implement grove-run-state-view`

## Next Steps

### Ready to implement
- **Grove run-state view** — show `.graft/run-state/` entries in Grove's repo
  detail: state name, JSON content, producer/consumer commands. Standalone
  slice, no new primitives needed.

### Draft (open questions to resolve)
- **Sequence declarations** — named command sequences in graft.yaml. Open:
  argument passing model, whether this should be a primitive vs shell composition.
- **Dependency graph** — first-class `state → producer` map. Open: whether any
  consumer needs this beyond inline validation.
- **Sequence resumability** — skip completed steps on re-run. Blocked on
  sequence-declarations.

Design notes:
- [Sequence primitives exploration](notes/2026-02-24-sequence-primitives-exploration.md) — resolved open questions, slice critique
- [Command output state mapping](notes/2026-02-23-command-output-state-mapping.md) — two-layer model, state mapping primitive

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
