# Graft - Claude Code Entrypoint

Read [AGENTS.md](AGENTS.md) for full project context, architectural principles, and policies.

## Verification commands

```bash
# Graft (Rust) - primary implementation
cargo fmt --check                # Format check
cargo clippy -- -D warnings      # Lint
cargo test                       # All Rust tests (49 tests: 42 unit, 7 integration)
cargo run -p graft-cli -- status # Smoke test

# Graft (Python) - legacy, maintained for compatibility
uv run pytest                    # 405 tests, ~46% coverage
uv run mypy src/                 # Strict mode type checking
uv run ruff check src/ tests/   # Linting
```

## Key paths

- `Cargo.toml` - Virtual workspace manifest (all Rust crates)
- `crates/` - All Rust crates (grove-core, grove-engine, grove-cli, graft-core, graft-engine, graft-cli)
- `src/graft/` - Python source (legacy, maintained for compatibility)
- `grove/docs/` - Grove-specific docs ([agent entrypoint](grove/docs/agents.md))
- `docs/specifications/` - Canonical specs
- `docs/` - Implementation docs
- `notes/` - Exploration notes ([index](notes/index.md))
- `.graft/` - Dependencies (meta-knowledge-base, python-starter, rust-starter)
- `knowledge-base.yaml` - KB structure declaration
- `continue-here.md` - Session handoff context
