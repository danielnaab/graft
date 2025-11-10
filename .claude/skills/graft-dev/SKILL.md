# Skill: Graft Dev

## Purpose
Implement Graft slices safely with outside-in TDD and clean architecture.

## When to Use
Activate when the user mentions:
- Implementing a vertical slice
- Working on Graft features
- Following the Graft development process
- CLI command implementation
- Testing Graft functionality
- Starting work on a slice

## Steps per Slice

### 1. Preparation
- Read slice requirements in `docs/roadmap/vertical-slices.md`
- Create todo list with TodoWrite tool tracking all implementation steps
- Review `docs/adr/0002-layered-architecture-with-separation-of-concerns.md` for architecture pattern
- Check current work log in `agent-records/work-log/<date>.md`
- Use `/slice <number>` command to set up the work session automatically

### 2. Test-First Development
- Add failing tests in `tests/` (black-box subprocess style)
- Follow existing test patterns from `tests/conftest.py`
- Tests must use `python -m graft.cli` (subprocess, not imports)
- Assert on `--json` output and exit codes
- Test error cases with helpful messages including paths
- Test edge cases: missing files, malformed YAML, missing required fields

### 3. Implementation (Layered Architecture)
Follow the layered architecture pattern from ADR-0002:

**Domain Layer** (`src/graft/domain/entities.py`):
- Immutable dataclasses with `@dataclass(frozen=True)`
- Core entities: Artifact, GraftConfig, Derivation
- Value objects: Inputs, Material, Output, Template, Policy

**Adapter Layer** (`src/graft/adapters/`):
- Protocol-based interfaces (e.g., `FileSystemPort`)
- Concrete implementations (e.g., `LocalFileSystem`, `ConfigAdapter`)
- Handle external interactions (filesystem, YAML parsing)

**Service Layer** (`src/graft/services/`):
- Use case orchestration
- Explicit dependency injection in `__init__`
- Return result objects with `.to_dict()` methods
- Business logic lives here

**CLI Layer** (`src/graft/cli.py`):
- Thin presentation layer using Typer
- Wire up dependencies at module level
- Handle output formatting (JSON vs human-readable)
- Map exceptions to proper exit codes

### 4. Validation
- Run `pytest -q` to verify all tests pass
- Test CLI manually: `python -m graft.cli <command> --json`
- Verify exit codes: 0 (success), 1 (user error), 2 (system error)
- Check JSON output matches schemas with `/schema` command
- Use `/check-contract` to verify CLI contract compliance

### 5. Documentation
- Update relevant docs in `docs/`
- Update or create schemas in `schemas/`
- Create ADR with `/adr <title>` if architectural decisions were made
- Write comprehensive work log entry with `/work-log`

## Critical Constraints
- **No side effects or network writes** — This is non-negotiable
- **No scope expansion** — Deliver only what the slice requires
- **Black-box testing** — Never import internal modules in tests
- **CLI contract stability** — All commands must support `--json`
- **Exit code compliance** — 0 for success, 1 for user error, 2 for system error

## Quality Checklist
Before marking a slice complete, verify:
- [ ] All tests pass (`pytest -q`)
- [ ] CLI returns proper exit codes (0, 1, or 2)
- [ ] JSON output matches contract and schemas
- [ ] Error messages are helpful and include paths
- [ ] Code follows layered architecture (Domain → Adapters → Services → CLI)
- [ ] Work log updated with session details
- [ ] Docs and schemas updated if needed
- [ ] No files modified in `.venv/` or `.git/`

## Common Patterns

### Service with Dependency Injection
```python
class MyService:
    def __init__(self, config_adapter: ConfigAdapter, fs: FileSystemPort):
        self.config_adapter = config_adapter
        self.fs = fs

    def execute(self, path: Path) -> MyResult:
        # Business logic here
        return MyResult(...)
```

### Result Object with Serialization
```python
@dataclass
class MyResult:
    field1: str
    field2: list

    def to_dict(self) -> dict:
        return {
            "field1": self.field1,
            "field2": self.field2
        }
```

### CLI Command Structure
```python
@app.command()
def my_command(
    artifact: str,
    json_out: bool = typer.Option(False, "--json")
):
    """Command description."""
    try:
        artifact_path = _artifact_path(artifact)
        result = my_service.execute(artifact_path)

        if json_out:
            print_json(result.to_dict())
        else:
            # Human-readable output
            typer.echo(f"Result: {result.field1}")
    except typer.BadParameter as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
    except Exception as e:
        typer.echo(f"System error: {e}", err=True)
        raise typer.Exit(code=2)
```

## Outputs
- Green tests with comprehensive coverage
- Minimal, focused implementation following layered architecture
- Updated documentation and schemas
- Work log entry with architectural notes and decisions
