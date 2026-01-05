---
status: stable
updated: 2026-01-05
---

# Working with Graft - Agent Guide

**Audience**: AI agents and developers starting work on the graft codebase
**Purpose**: Quick orientation and workflow guidance
**Last Updated**: 2026-01-04

---

## First Time Here?

Read these files in order:

1. **[README.md](../../README.md)** - Project overview and CLI commands (5 min)
2. **[tasks.md](../../tasks.md)** - Current development status (2 min)
3. **[continue-here.md](../../continue-here.md)** - Recent session context (3 min)
4. **[docs/README.md](../README.md)** - Architecture details (10 min)

After reading, you should understand:
- What graft does
- How the codebase is organized
- What's already implemented
- What patterns to follow

---

## Codebase Structure

```
graft/
├── src/graft/
│   ├── domain/          # Frozen dataclasses (Change, Command, LockEntry, etc.)
│   ├── services/        # Pure functions (upgrade, query, lock, snapshot)
│   ├── protocols/       # Protocol interfaces for dependency injection
│   ├── adapters/        # Infrastructure implementations (git, yaml, filesystem)
│   └── cli/             # Typer-based CLI commands
├── tests/
│   ├── unit/            # Unit tests using fakes
│   ├── integration/     # Integration tests with real adapters
│   └── fakes/           # In-memory test doubles
├── docs/                # All documentation
├── notes/               # Time-bounded development notes
└── status/              # Implementation status tracking
```

---

## Essential Patterns

### 1. Domain Models Are Immutable

All domain models use `@dataclass(frozen=True)`:

```python
@dataclass(frozen=True)
class Change:
    ref: str
    type: str | None = None
    # ...
```

Never modify domain objects. Use `.replace()` or create new instances.

### 2. Services Are Pure Functions

Services take protocol dependencies and return results:

```python
def upgrade_dependency(
    snapshot: Snapshot,
    executor: CommandExecutor,
    lock_file: LockFile,
    # ... parameters
) -> UpgradeResult:
    # Pure function logic
```

No classes, no state, no side effects beyond the protocols.

### 3. Use Protocols for Dependency Injection

Accept protocol types, not concrete implementations:

```python
from graft.protocols.git import GitOperations

def some_service(git: GitOperations, path: str) -> str:
    return git.resolve_ref(path, "main")
```

This enables easy testing with fakes.

### 4. Test with Fakes, Not Mocks

Use in-memory fakes from `tests/fakes/`:

```python
from tests.fakes.fake_snapshot import FakeSnapshot

def test_upgrade_rollback():
    snapshot = FakeSnapshot()
    # Test logic
```

Fakes behave like real implementations but run in memory.

### 5. CLI Commands Use Typer

All CLI commands follow this pattern:

```python
import typer

def command_name(
    arg: str,
    option: str = typer.Option("default", "--option", help="Description")
) -> None:
    """Command description."""
    try:
        # Implementation
    except DomainError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e
```

---

## Common Tasks

### Adding a New CLI Command

1. Create `src/graft/cli/commands/your_command.py`
2. Define command function with typer decorators
3. Register in `src/graft/cli/main.py`
4. Add tests in `tests/unit/test_your_command.py`
5. Document in README.md

**Example:** See [src/graft/cli/commands/status.py](../../src/graft/cli/commands/status.py)

### Adding a New Service Function

1. Create function in appropriate `src/graft/services/*.py`
2. Accept protocol dependencies, not concrete types
3. Return frozen dataclass result
4. Add unit tests with fakes in `tests/unit/`
5. Document in docs/README.md

**Example:** See [src/graft/services/upgrade_service.py](../../src/graft/services/upgrade_service.py)

### Adding a New Domain Model

1. Create frozen dataclass in `src/graft/domain/`
2. Add validation in `__post_init__`
3. Add unit tests in `tests/unit/test_domain_*.py`
4. Document in docs/README.md

**Example:** See [src/graft/domain/change.py](../../src/graft/domain/change.py)

### Running Tests

```bash
# All tests
uv run pytest

# Specific file
uv run pytest tests/unit/test_upgrade_service.py -v

# With coverage
uv run pytest --cov=src/graft --cov-report=html

# Type checking
uv run mypy src/

# Linting
uv run ruff check src/ tests/
```

---

## Documentation Update Protocol

When you modify code, update these docs:

| Change | Update These Files |
|--------|-------------------|
| Add CLI command | README.md (CLI Commands), docs/README.md (if complex) |
| Add service function | docs/README.md (Services section) |
| Add domain model | docs/README.md (Domain Models section) |
| Change architecture | docs/README.md, relevant ADR in docs/decisions/ |
| Fix bug | No doc update unless behavior changes |
| Add feature | README.md, possibly user-guide.md |
| Update test count | README.md (Testing Status) |

**Rule**: If a user would notice the change, document it in README.md.

---

## Workflow: Plan → Implement → Test → Document

### 1. Plan

Before writing code:
- Check if similar code exists
- Review relevant patterns in docs/README.md
- Verify tests exist for dependencies
- Update tasks.md if starting a new task

### 2. Implement

While writing code:
- Follow established patterns (frozen dataclasses, protocols, pure functions)
- Add type hints to all functions
- Add docstrings to public APIs
- Keep functions small and focused

### 3. Test

Before committing:
- Write unit tests for new service functions
- Write integration tests if adding new adapters
- Run `uv run pytest` - all tests must pass
- Run `uv run mypy src/` - no type errors
- Run `uv run ruff check src/ tests/` - no linting errors

### 4. Document

Before committing:
- Update README.md if user-visible changes
- Update docs/README.md if architecture changes
- Add ADR in docs/decisions/ if significant decision
- Update tasks.md status
- Update test count in README.md if tests added

---

## Code Quality Standards

All code must pass these quality gates before merge. Standards are enforced automatically via pre-commit hooks and CI/CD.

### Type Checking

- mypy strict mode enabled
- All functions have type hints
- All parameters have type hints
- Return types always specified
- Run: `uv run mypy src/`

### Testing

- 330 tests passing (verify with `uv run pytest`)
- Service layer: 80-100% coverage
- All service functions have unit tests
- Integration tests for adapters
- Coverage minimum: 42% overall
- Run: `uv run pytest --cov=src`

### Linting

- ruff configured in pyproject.toml
- All checks must pass
- No blind exception catches (use specific types)
- No unused imports or variables
- Run: `uv run ruff check src/ tests/`

### Pre-Commit Hooks (Recommended)

Install git hooks to catch issues before committing:

```bash
./scripts/install-hooks.sh
```

The hook runs automatically before each commit and checks:
- Tests pass
- Type checking passes
- Linting passes

Skip only in emergencies: `git commit --no-verify`

### CI/CD Pipeline (Automatic)

All pull requests are automatically checked by Forgejo Actions:
- Test suite runs with coverage check
- Type checking with mypy strict
- Linting with ruff
- All must pass before merge

View status: Check "Checks" tab on your pull request

### Quality Documentation

See [docs/quality-standards.md](../quality-standards.md) for detailed information on:
- Quality gates and requirements
- Common issues and solutions
- Development workflow
- Maintenance procedures

---

## Troubleshooting

### "Tests are failing after my changes"

1. Run `uv run pytest -v` to see specific failures
2. Check if you modified a frozen dataclass incorrectly
3. Verify protocol signatures match adapter implementations
4. Check if fakes need updating

### "Mypy shows type errors"

1. Add type hints to all function parameters
2. Check for `str | None` vs `str` correctness
3. Don't use `assert` for type narrowing
4. Use protocols, not concrete types, in signatures

### "Ruff linting fails"

1. Run `uv run ruff check src/ tests/` to see errors
2. Fix broad exception catches (use specific exception types)
3. Remove unused imports
4. Fix line length issues

### "I don't know which file to modify"

1. Use `grep` to find similar code: `grep -r "pattern" src/`
2. Check docs/README.md for architecture overview
3. Look at recent commits: `git log --oneline -10`
4. Review continue-here.md for recent context

---

## Architectural Decisions

Major decisions are documented as ADRs in [docs/decisions/](../decisions/):

- [001-require-explicit-ref-in-upgrade.md](../decisions/001-require-explicit-ref-in-upgrade.md) - Why `--to` is required
- [002-filesystem-snapshots-for-rollback.md](../decisions/002-filesystem-snapshots-for-rollback.md) - Rollback mechanism
- [003-snapshot-only-lock-file.md](../decisions/003-snapshot-only-lock-file.md) - What gets snapshotted
- [004-protocol-based-dependency-injection.md](../decisions/004-protocol-based-dependency-injection.md) - DI approach
- [005-functional-service-layer.md](../decisions/005-functional-service-layer.md) - Why services are functions

Read these to understand "why" behind the architecture.

---

## Getting Help

**Stuck on implementation?**
- Review similar code in the same directory
- Check tests for usage examples
- Read ADRs for architectural context

**Don't understand the domain?**
- Read graft-knowledge specification: `/home/coder/graft-knowledge/docs/specification/`
- Review Complete Workflow: [status/workflow-validation.md](../../status/workflow-validation.md)

**Tests are confusing?**
- Look at fakes in `tests/fakes/`
- Review unit test examples in `tests/unit/`
- Check integration tests in `tests/integration/`

---

## Quick Reference

### File Locations

| Need to... | Look in... |
|-----------|-----------|
| Add CLI command | src/graft/cli/commands/ |
| Add service function | src/graft/services/ |
| Add domain model | src/graft/domain/ |
| Add protocol | src/graft/protocols/ |
| Add adapter | src/graft/adapters/ |
| Add unit test | tests/unit/ |
| Add integration test | tests/integration/ |
| Add fake | tests/fakes/ |
| Document decision | docs/decisions/ |
| Add development note | notes/ |

### Verification Commands

```bash
# Tests passing?
uv run pytest --quiet

# Types correct?
uv run mypy src/

# Linting passing?
uv run ruff check src/ tests/

# All quality checks
uv run pytest && uv run mypy src/ && uv run ruff check src/ tests/
```

---

**Next**: Read [continue-here.md](../../continue-here.md) to understand current development state, then start contributing!
