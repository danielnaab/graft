# Graft - Claude Code Entrypoint

Read [AGENTS.md](AGENTS.md) for full project context, architectural principles, and policies.

## Verification commands

```bash
uv run pytest                    # 405 tests, ~46% coverage
uv run mypy src/                 # Strict mode type checking
uv run ruff check src/ tests/   # Linting
```

## Key paths

- `src/graft/` - Python source (domain/services/protocols/adapters/cli)
- `grove/` - Rust workspace tool ([its own agent entrypoint](grove/docs/agents.md))
- `docs/specifications/` - Canonical specs
- `docs/` - Implementation docs
- `notes/` - Exploration notes ([index](notes/index.md))
- `.graft/` - Dependencies (meta-knowledge-base, python-starter, rust-starter)
- `knowledge-base.yaml` - KB structure declaration
- `continue-here.md` - Session handoff context
