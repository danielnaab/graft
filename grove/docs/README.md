---
title: Grove Documentation
status: working
---

# Grove

Multi-repo workspace manager with graft awareness

## Quick Start

```bash
cargo build --all                # Build everything
cargo run -- --help              # CLI help
cargo test --all                 # Run all tests
```

## Project Structure

```
src/main.rs                      # CLI entry point (clap + anyhow)
crates/
├── grove-core/            # Domain types, traits, errors
└── grove-engine/          # Business logic
tests/                           # Integration tests
```

## Architecture & Patterns

This project uses the [Rust Starter Template](../.graft/rust-starter) patterns:

- [Architecture](../.graft/rust-starter/docs/architecture/architecture.md) — layers, trait-based DI, error handling
- [Decisions](../.graft/rust-starter/docs/decisions/) — ADRs for pattern choices
- [Getting Started](../.graft/rust-starter/docs/guides/getting-started.md) | [Development Guide](../.graft/rust-starter/docs/guides/development.md)
- [Reference](../.graft/rust-starter/docs/reference/project-reference.md) — structure, config, tooling

## Project-Specific

- [Agent Entrypoint](agents.md) — for AI agents working on this project
- [Knowledge Base](../knowledge-base.yaml) — project KB configuration

## Template

- **Version**: None
- **Update**: `copier update --trust`

## Sources

- [Template Documentation](../.graft/rust-starter/docs/)
- [Knowledge Base Config](../knowledge-base.yaml)
