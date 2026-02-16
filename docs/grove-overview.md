---
title: Grove Overview
status: working
---

# Grove

Multi-repo workspace manager with graft awareness

> **Authority Note:** Working document providing Grove overview. For canonical architecture decisions, see [Grove Specifications](specifications/grove/) and [Implementation ADRs](grove/implementation/).

## Quick Start

```bash
cargo build --release -p grove-cli   # Build grove
cargo run -p grove-cli -- --help     # CLI help
cargo test                           # Run all tests
```

## What is Grove?

Grove is a terminal-based workspace manager that displays git status across multiple repositories in a unified view. It's designed for developers working with multi-repo setups and integrates with graft's state query system to show repository health.

## Project Structure

```
crates/
├── grove-core/      # Domain types, traits, errors
├── grove-engine/    # Business logic (config, git status, registry)
└── grove-cli/       # TUI and CLI entry point
```

## Architecture & Patterns

This project uses the [Rust Starter Template](.graft/rust-starter) patterns:

- **Three-layer architecture**: Binary → Engine → Core
- **Trait-based DI**: Services accept trait bounds, not concrete types
- **Error handling**: `thiserror` for libraries, `anyhow` for binaries
- **Domain newtypes**: Type-safe wrappers with validation at construction
- **Graceful degradation**: One broken repo doesn't break the whole workspace

See [Architecture Overview](grove/implementation/architecture-overview.md) for detailed explanation.

## Documentation

- **User Guide**: [Grove User Guide](guides/grove-user-guide.md) - Installation, configuration, usage
- **Architecture**: [Implementation Architecture](grove/implementation/architecture-overview.md) - Three-layer design, TUI event loop, git querying
- **Planning**: [Roadmap](grove/planning/roadmap.md) - Feature planning and slices
- **Decisions**: [Implementation ADRs](grove/implementation/) - Architecture decision records
- **Specifications**: [Grove Specs](specifications/grove/) - Canonical requirements

## Key Features (Slice 1)

- ✅ Multi-repo workspace configuration
- ✅ Real-time git status display (branch, dirty state, ahead/behind)
- ✅ Terminal UI with keyboard navigation (j/k, arrows)
- ✅ Timeout protection for slow git operations
- ✅ Graceful error handling (one broken repo doesn't break the whole view)
- ✅ State query integration (show graft state results)

## For AI Agents

When working on Grove:

1. **Read**: [Agent Entrypoint for Grove](#grove-agent-section) in main AGENTS.md
2. **Follow**: Rust patterns from [rust-starter](.graft/rust-starter/)
3. **Verify**: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
4. **Authority**: Code is canonical for implementation, specs are canonical for behavior

## Sources

- [Specifications](specifications/grove/) - Canonical requirements
- [Rust Starter Template](.graft/rust-starter/docs/) - Architectural patterns
- [Knowledge Base](knowledge-base.yaml) - Project structure
