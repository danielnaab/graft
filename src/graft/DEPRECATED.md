# Python Implementation - Deprecated

**Status**: This Python implementation is deprecated as of February 2026.

## Current Status

The Python implementation of Graft is **maintained for compatibility** but is no longer the primary implementation. The Rust implementation is production-ready and is the recommended version for all new use cases.

## Migration Path

If you are using the Python CLI, please migrate to the Rust CLI:

```bash
# Install the Rust CLI (build from source)
cargo install --path crates/graft-cli

# The Rust CLI provides full feature parity with the Python version
graft status
graft resolve
graft fetch
# ... all other commands work identically
```

## Why the Migration?

The Rust rewrite provides:
- **Performance**: Significantly faster dependency resolution and state queries
- **Reliability**: Strong type safety and comprehensive error handling
- **Maintainability**: Modern Rust tooling and testing infrastructure
- **Feature Parity**: All Python CLI features are implemented in Rust

## Python Implementation Details

The Python implementation:
- Will continue to pass all tests (`uv run pytest`)
- Will continue to be linted and type-checked (`uv run mypy`, `uv run ruff check`)
- May receive critical bug fixes
- Will NOT receive new features

## Rust Implementation

The Rust implementation lives in `crates/graft-*`:
- `graft-core`: Core domain types and errors
- `graft-engine`: Dependency resolution, state queries, command execution
- `graft-cli`: CLI interface (typer equivalent)
- `graft-common`: Shared utilities (git ops, config parsing, state caching)

See `AGENTS.md` and `CLAUDE.md` for documentation on the Rust codebase.

## Timeline

- **January 2026**: Rust rewrite completed (14 tasks, 91 tests, full parity)
- **February 2026**: Python marked deprecated, Rust promoted to primary
- **Future**: Python implementation may be removed in a future major version

## Questions?

For questions about the migration or Rust implementation, see:
- `docs/specifications/` for canonical specs
- `notes/2026-02-15-rust-rewrite/` for rewrite documentation
- `crates/graft-cli/README.md` for Rust CLI documentation
