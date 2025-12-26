---
title: Project Structure Reference
status: stable
owners: [documentation-team]
---

# Project Structure Reference

Complete reference for the Python Starter Template directory structure.

## Directory Tree

```
python-starter/
├── .git/                           # Git repository
├── .github/                        # GitHub configuration (CI/CD)
│   └── workflows/
│       ├── test.yml                # Test workflow
│       └── lint.yml                # Lint workflow
├── .gitignore                      # Git ignore patterns
├── .python-version                 # Python version (3.11)
├── README.md                       # Project overview
├── pyproject.toml                  # Central configuration
├── uv.lock                         # Dependency lockfile (generated)
│
├── docs/                           # Documentation
│   ├── README.md                   # Human entrypoint
│   ├── agents.md                   # Agent entrypoint
│   ├── knowledge-base.yaml         # KB configuration
│   │
│   ├── architecture/               # Architecture documentation
│   │   ├── overview.md
│   │   ├── functional-services.md
│   │   ├── dependency-injection.md
│   │   ├── domain-model.md
│   │   └── testing-strategy.md
│   │
│   ├── decisions/                  # Architectural Decision Records
│   │   ├── 001-why-uv.md
│   │   ├── 002-src-layout.md
│   │   ├── 003-protocol-di.md
│   │   ├── 004-functional-services.md
│   │   ├── 005-context-objects.md
│   │   ├── 006-typer-cli.md
│   │   └── 007-ruff-tooling.md
│   │
│   ├── guides/                     # User guides
│   │   ├── getting-started.md
│   │   ├── development-workflow.md
│   │   ├── adding-features.md
│   │   ├── testing-guide.md
│   │   └── cli-usage.md
│   │
│   ├── reference/                  # Technical reference
│   │   ├── project-structure.md (this file)
│   │   ├── configuration.md
│   │   ├── tooling.md
│   │   ├── protocols.md
│   │   └── context-objects.md
│   │
│   └── copier/                     # Copier template docs
│       ├── template-design.md
│       ├── variable-mapping.md
│       └── customization-points.md
│
├── src/                            # Source code (src layout)
│   └── starter/                    # Package (template variable)
│       ├── __init__.py             # Package initialization
│       ├── __main__.py             # Entry: python -m starter
│       │
│       ├── domain/                 # Domain layer (pure logic)
│       │   ├── __init__.py
│       │   ├── entities.py         # Domain entities
│       │   ├── value_objects.py    # Value objects
│       │   └── exceptions.py       # Domain exceptions
│       │
│       ├── services/               # Service layer (functions)
│       │   ├── __init__.py
│       │   ├── context.py          # ServiceContext
│       │   └── example_service.py  # Service functions
│       │
│       ├── adapters/               # Adapter layer
│       │   ├── __init__.py
│       │   ├── repository.py       # Repository implementations
│       │   └── external_api.py     # External API adapters
│       │
│       ├── protocols/              # Protocol definitions
│       │   ├── __init__.py
│       │   ├── repository.py       # Repository protocol
│       │   └── external.py         # External service protocols
│       │
│       └── cli/                    # CLI layer (Typer)
│           ├── __init__.py
│           ├── main.py             # Typer app + wiring
│           └── commands/           # Command groups
│               ├── __init__.py
│               └── example.py      # Example commands
│
├── tests/                          # Test suite
│   ├── __init__.py
│   ├── conftest.py                 # Pytest fixtures
│   │
│   ├── unit/                       # Unit tests (fast)
│   │   ├── __init__.py
│   │   ├── test_domain.py          # Domain tests
│   │   └── test_services.py        # Service tests
│   │
│   ├── integration/                # Integration tests
│   │   ├── __init__.py
│   │   └── test_adapters.py        # Adapter tests
│   │
│   └── fakes/                      # Fake implementations
│       ├── __init__.py
│       ├── fake_repository.py      # Fake repository
│       └── fake_external_api.py    # Fake external API
│
└── scripts/                        # Development scripts
    ├── setup.sh                    # Initial setup
    └── test.sh                     # Run tests with coverage
```

## Layer Descriptions

### Domain Layer (`src/starter/domain/`)

**Purpose**: Pure business logic with no external dependencies.

**Contents**:
- **entities.py**: Objects with identity (User, Order, etc.)
- **value_objects.py**: Immutable values (Money, Address, etc.)
- **exceptions.py**: Domain-specific exceptions

**Rules**:
- No imports from other layers
- No external dependencies (no database, API, etc.)
- Pure Python with dataclasses
- Business rules and validation only

### Service Layer (`src/starter/services/`)

**Purpose**: Implement use cases and workflows.

**Contents**:
- **context.py**: ServiceContext definition (frozen dataclass)
- **{domain}_services.py**: Service functions for each domain area

**Rules**:
- Functions take `ServiceContext` as first parameter
- Pure functions (no side effects except via context)
- Orchestrate domain logic and adapters
- No direct database/API access (use context)

### Adapter Layer (`src/starter/adapters/`)

**Purpose**: Implement external system integrations.

**Contents**:
- **repository.py**: Database/persistence implementations
- **external_api.py**: Third-party API clients
- **{service}.py**: Other external integrations

**Rules**:
- Implement protocols from `protocols/`
- Handle external system specifics
- Translate between domain and external formats
- Manage connections and resources

### Protocol Layer (`src/starter/protocols/`)

**Purpose**: Define interfaces using structural typing.

**Contents**:
- **repository.py**: Repository[T] protocol
- **{service}.py**: Service-specific protocols

**Rules**:
- Use `typing.Protocol`
- Generic where applicable (`Protocol, Generic[T]`)
- No implementation (only signatures)
- Methods have `...` (ellipsis) body

### CLI Layer (`src/starter/cli/`)

**Purpose**: Command-line interface.

**Contents**:
- **main.py**: Typer app, context wiring, entry point
- **commands/{feature}.py**: Feature-specific commands

**Rules**:
- Thin adapter (parse, wire, call, display)
- No business logic
- Build context with real adapters
- Call service functions
- Format output

## File Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Modules | `snake_case.py` | `user_services.py` |
| Classes | `PascalCase` | `UserRepository` |
| Functions | `snake_case()` | `create_user()` |
| Constants | `UPPER_SNAKE_CASE` | `MAX_RETRIES` |
| Private | `_leading_underscore` | `_internal_helper()` |
| Tests | `test_*.py` | `test_services.py` |
| Fixtures | `conftest.py` | `conftest.py` |

## Import Organization

Imports are organized by ruff (isort):

```python
# 1. Standard library
import os
from pathlib import Path
from typing import Optional

# 2. Third-party packages
import typer
from rich.console import Console

# 3. Local application
from starter.domain.entities import User
from starter.services.context import ServiceContext
from starter.protocols.repository import Repository
```

## Package Exports

### `__init__.py` Pattern

```python
# src/starter/services/__init__.py

"""Service layer public API."""

from starter.services.context import ServiceContext
from starter.services.user_services import (
    create_user,
    get_user,
    list_users,
)

__all__ = [
    "ServiceContext",
    "create_user",
    "get_user",
    "list_users",
]
```

## Where to Put Code

### Adding New Feature

1. **Domain entity** → `src/starter/domain/entities.py`
2. **Value object** → `src/starter/domain/value_objects.py`
3. **Protocol** → `src/starter/protocols/{feature}.py`
4. **Service functions** → `src/starter/services/{feature}_services.py`
5. **Adapter** → `src/starter/adapters/{feature}.py`
6. **CLI commands** → `src/starter/cli/commands/{feature}.py`
7. **Tests** → `tests/unit/test_{feature}_services.py`
8. **Fakes** → `tests/fakes/fake_{feature}.py`

### Adding Utility Code

- **Domain utilities** → `src/starter/domain/utils.py`
- **Service helpers** → `src/starter/services/utils.py`
- **Test helpers** → `tests/helpers.py`

## Configuration Files

| File | Purpose |
|------|---------|
| `pyproject.toml` | Central configuration (project, uv, ruff, pytest) |
| `.python-version` | Python version for uv |
| `uv.lock` | Dependency lockfile (auto-generated) |
| `.gitignore` | Git ignore patterns |
| `docs/knowledge-base.yaml` | Documentation metadata |

## Generated/Ignored Files

These files are generated and should not be edited:

- `uv.lock` - Dependency lockfile
- `.venv/` - Virtual environment
- `__pycache__/` - Python bytecode
- `.pytest_cache/` - Pytest cache
- `htmlcov/` - Coverage HTML report
- `.coverage` - Coverage data

## Related Documentation

- [Configuration Reference](configuration.md) - pyproject.toml details
- [Architecture Overview](../architecture/overview.md) - Layer explanations
- [Adding Features](../guides/adding-features.md) - Step-by-step guide

## Sources

- [src layout - Python Packaging Guide](https://packaging.python.org/en/latest/discussions/src-layout-vs-flat-layout/)
- [ADR 002: src Layout](../decisions/002-src-layout.md) - Layout decision
