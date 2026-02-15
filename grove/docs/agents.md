---
title: Grove - Agent Entrypoint
status: working
---

# Agent Entrypoint - Grove

You are working on **Grove**: Multi-repo workspace manager with graft awareness

## Before Making Changes

1. **Read project configuration**: [../../knowledge-base.yaml](../../knowledge-base.yaml)
2. **Follow meta-KB policies**: [../../.graft/meta-knowledge-base/AGENTS.md](../../.graft/meta-knowledge-base/AGENTS.md)
3. **Understand template patterns**: [../../.graft/rust-starter/docs/agents.md](../../.graft/rust-starter/docs/agents.md)
4. **Follow living-specifications format**: [../../.graft/living-specifications/docs/format.md](../../.graft/living-specifications/docs/format.md)

## Core Pattern

Layered architecture: **Binary** (CLI) → **Engine** (logic) → **Core** (types, traits).

- Core defines traits (ports); engine uses `&impl Trait` bounds
- Libraries use `thiserror`; binary uses `anyhow` with `.context()`
- Newtypes for domain identifiers, validation at construction
- Tests use hand-written **fakes**, not mock frameworks

## File Organization

```
crates/grove-cli/src/
├── main.rs                      # CLI (clap, anyhow, wiring)
├── lib.rs                       # Library exports for tests
├── tui.rs                       # TUI implementation
└── state/                       # State query integration
crates/grove-core/src/
├── domain.rs                    # Domain types (newtypes, enums)
├── error.rs                     # Error enums (thiserror)
├── traits.rs                    # Trait ports
└── lib.rs                       # Re-exports
crates/grove-engine/src/
├── service.rs                   # Business logic functions
└── lib.rs                       # Re-exports
crates/grove-cli/tests/
├── common/mod.rs                # Shared fakes
└── integration_test.rs          # End-to-end tests
```

## Common Tasks

**Add domain type**: Define in `crates/grove-core/src/domain.rs`, re-export from `lib.rs`, add unit tests inline.

**Add trait (port)**: Define in `crates/grove-core/src/traits.rs`, use via `&impl Trait` bounds in engine, write fake in `crates/grove-cli/tests/common/mod.rs`.

**Add engine function**: Define in `crates/grove-engine/src/`, accept trait bounds, return `Result`, re-export from `lib.rs`.

**Add CLI subcommand**: Add variant to `Command` enum in `crates/grove-cli/src/main.rs`, implement handler function, wire in `match`.

**Quality checks**: `cargo fmt --check` | `cargo clippy -- -D warnings` | `cargo test`

## Authority

**Canonical**: `crates/grove-*/`, `Cargo.toml` | **Interpretive**: `grove/docs/`

When code and docs conflict, **code is correct**.

## Template Resources

- [Architecture](../../.graft/rust-starter/docs/architecture/architecture.md) — patterns and design
- [Decisions](../../.graft/rust-starter/docs/decisions/) — ADRs for pattern choices
- [Guides](../../.graft/rust-starter/docs/guides/) — getting started and development
- [Agent reference](../../.graft/rust-starter/docs/agents.md) — template agent entrypoint

## Sources

- [Project KB](../../knowledge-base.yaml) — project configuration
- [Template KB](../../.graft/rust-starter/knowledge-base.yaml) — template configuration
- [Meta KB](../../.graft/meta-knowledge-base/AGENTS.md) — KB methodology
