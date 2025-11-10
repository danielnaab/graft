# CLAUDE.md — Graft Project

You are collaborating on **Graft**. Follow these practices to be effective and safe.

## Project Overview
Graft is a CLI tool for auditable, file-first derivations with DVC integration. The project uses:
- **Python 3.14** with uv for package management
- **Typer** for CLI framework
- **pytest** for black-box subprocess testing
- **Layered architecture**: Domain → Adapters → Services → CLI

## Goals
- Deliver vertical slices in order. Do not expand scope beyond the current slice.
- Keep the CLI contract (see `docs/cli-spec.md`) stable; emit `--json` where applicable.
- Follow the layered architecture pattern documented in `docs/adr/0002-layered-architecture-with-separation-of-concerns.md`

## Process
1. Read `docs/implementation-strategy.md` and `docs/roadmap/vertical-slices.md`.
2. For the current slice:
   - Add/adjust tests in `tests/` (subprocess/black-box).
   - Implement the minimal code in `src/` to satisfy tests.
   - Update docs and schemas as needed.
   - Log progress in `agent-records/work-log/<date>.md`.
3. Before implementing, use `/slice <number>` to set up the work session.
4. After making progress, use `/work-log` to document your work.

## Architecture Layers
- **Domain** (`src/graft/domain/`): Core entities, value objects (immutable with `@dataclass(frozen=True)`)
- **Adapters** (`src/graft/adapters/`): External interfaces using `Protocol` types
- **Services** (`src/graft/services/`): Use cases with explicit dependency injection
- **CLI** (`src/graft/cli.py`): Thin presentation layer using Typer

## Testing Strategy
- All tests are black-box subprocess tests via `python -m graft.cli`
- Tests use fixtures from `examples/` copied to `tmp_path`
- Assert on JSON output (`--json` flag) and exit codes
- No internal imports in tests; CLI contract is the API

## Code Style
- Use 4-space indentation for Python
- Type hints for all function signatures
- Immutable dataclasses for value objects (`@dataclass(frozen=True)`)
- Protocol-based interfaces for adapters
- Services return result objects with `.to_dict()` serialization methods

## Exit Codes
- `0`: Success
- `1`: User error (bad input, missing file, invalid YAML)
- `2`: System error (permissions, unexpected exceptions)

## Guardrails
- **No side effects or network writes** (critical constraint).
- Do not introduce tracked patch files.
- Prefer small, focused PRs with passing tests.
- Never modify files in `.venv/` directory.
- Always ensure tests pass before considering work complete.

## Helpful Commands
- `pytest -q` — Run all tests
- `uv pip install -e ".[test]"` — Install in development mode
- `python -m graft.cli explain <artifact/> --json` — Test explain command
- `python -m graft.cli run <artifact/>` — Test run command
- `/slice <N>` — Start work on slice N
- `/test` — Run tests with analysis
- `/work-log` — Update work log
- `/check-contract` — Verify CLI contract compliance
- `/adr <title>` — Create new Architecture Decision Record
- `/schema` — Validate JSON schemas

## Current State
- **Slice 0** (explain command): ✅ Complete with full layered architecture
- **Next**: Slice 1 (run command) — deterministic single derivation

## Common Patterns
```python
# Service pattern (with dependency injection)
class MyService:
    def __init__(self, adapter: AdapterPort):
        self.adapter = adapter

    def do_something(self, path: Path) -> MyResult:
        # Business logic here
        return MyResult(...)

# Result object pattern
@dataclass
class MyResult:
    field1: str
    field2: list

    def to_dict(self) -> dict:
        return {"field1": self.field1, "field2": self.field2}
```

## Resources
- Architecture decisions: `docs/adr/`
- CLI specification: `docs/cli-spec.md`
- Implementation strategy: `docs/implementation-strategy.md`
- Testing patterns: `tests/conftest.py`
- Layered architecture: `docs/adr/0002-layered-architecture-with-separation-of-concerns.md`
