# Grove

Multi-repo workspace manager with graft awareness

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)

### Build & Run

```bash
cargo build --all
cargo run -- --help
```

### Running Tests

```bash
cargo test --all
```

### Code Quality

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --all-targets -- -D warnings

# Both
cargo fmt --all && cargo clippy --all-targets -- -D warnings
```

## Project Structure

```
grove/
├── src/main.rs                    # CLI entry point (clap)
├── crates/
│   ├── grove-core/          # Domain types, traits, errors
│   └── grove-engine/        # Business logic
├── tests/                         # Integration tests
│   ├── common/mod.rs              # Shared fakes
│   └── integration_test.rs        # Integration tests
└── docs/                          # Documentation
```

## Architecture

This project follows a layered architecture:

```
Binary (CLI) → Engine (logic) → Core (types, traits)
```

- **Core**: Domain types, trait definitions, error types. No I/O.
- **Engine**: Business logic using trait bounds. Pure functions.
- **Binary**: Thin CLI wrapper. Parses args, wires dependencies, formats output.

Dependencies point inward — core depends on nothing.

For detailed patterns, see the [Architecture Documentation](.graft/rust-starter/docs/architecture/architecture.md).

## Architectural Decisions

Each pattern choice is documented:

- [Cargo Workspace](.graft/rust-starter/docs/decisions/001-cargo-workspace.md)
- [Library-First](.graft/rust-starter/docs/decisions/002-library-first.md)
- [Trait-Based DI](.graft/rust-starter/docs/decisions/003-trait-based-di.md)
- [Error Handling](.graft/rust-starter/docs/decisions/004-error-handling.md)
- [clap CLI](.graft/rust-starter/docs/decisions/005-clap-cli.md)
- [Newtype Pattern](.graft/rust-starter/docs/decisions/007-newtype-pattern.md)
- [Testing Without Mocks](.graft/rust-starter/docs/decisions/008-testing-without-mocks.md)

## Documentation

- [Project Docs](docs/README.md)
- [Development Guide](.graft/rust-starter/docs/guides/development.md)
- [Getting Started](.graft/rust-starter/docs/guides/getting-started.md)
- [Reference](.graft/rust-starter/docs/reference/project-reference.md)

---

For AI agents: See [docs/agents.md](docs/agents.md) for structured technical reference.
