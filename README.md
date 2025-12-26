# Graft

Knowledge base tooling with language server support

## What's Inside

This template provides a solid foundation for building Python applications using modern best practices:

- **Functional Service Layer**: Pure functions with context objects instead of service classes
- **Protocol-Based DI**: Structural typing for flexible dependency injection
- **Modern Tooling**: uv (package manager), ruff (linting/formatting), pytest (testing)
- **Clean Architecture**: Clear separation between domain, services, adapters, and CLI
- **Type-Safe**: Full type hints with Protocol interfaces
- **Well-Tested**: Unit tests with fakes, integration tests with real adapters
- **CLI Interface**: Typer-based command-line interface
- **Template-Ready**: Designed for evolution into a Copier template

## Quick Start

### Prerequisites

- Python 3.11 or higher
- [uv](https://docs.astral.sh/uv/) package manager

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd python-starter

# Install dependencies
uv sync

# Run the example CLI
uv run graft-cli --help
```

### Running Tests

```bash
# Run all tests with coverage
uv run pytest

# Run tests with verbose output
uv run pytest -v

# Run specific test file
uv run pytest tests/unit/test_services.py
```

### Code Quality

```bash
# Check code with ruff
uv run ruff check .

# Format code with ruff
uv run ruff format .

# Run both linting and formatting
uv run ruff check . && uv run ruff format .
```

## Documentation

For comprehensive documentation, see [docs/README.md](docs/README.md).

## Project Structure

```
graft/
├── src/graft/    # Source code
│   ├── domain/            # Domain entities and value objects
│   ├── services/          # Service functions and context
│   ├── protocols/         # Protocol interface definitions
│   ├── adapters/          # External system implementations
│   └── cli/               # Command-line interface
├── tests/                 # Test suite
│   ├── unit/              # Unit tests with fakes
│   ├── integration/       # Integration tests
│   └── fakes/             # Fake implementations for testing
├── docs/                  # Documentation
└── scripts/               # Development scripts
```

## Architecture Highlights

### Functional Service Layer

Services are **pure functions** that take a context object:

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]
    config: Config

def create_example(ctx: ServiceContext, name: str, value: int) -> Entity:
    """Service function - pure, testable, composable."""
    entity = Entity(name=name, value=value)
    ctx.repository.save(entity)
    return entity
```

Benefits:
- More Pythonic than service classes
- Easier to test (pure functions)
- Better composition
- Explicit dependencies

### Protocol-Based Dependency Injection

Interfaces defined using `typing.Protocol` for structural typing:

```python
from typing import Protocol, Generic, TypeVar

T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> T | None: ...
```

Benefits:
- Duck typing with type safety
- No inheritance required
- Flexible implementations
- Modern Python idiom

## Development Workflow

### Adding a New Feature

1. Define domain entities in `src/graft/domain/`
2. Create protocol interfaces in `src/graft/protocols/`
3. Implement service functions in `src/graft/services/`
4. Create adapters in `src/graft/adapters/`
5. Add CLI commands in `src/graft/cli/commands/`
6. Write tests in `tests/`

See [docs/guides/adding-features.md](docs/guides/adding-features.md) for detailed instructions.

### Daily Development

```bash
# Sync dependencies
uv sync

# Run tests in watch mode
uv run pytest --watch

# Format and lint
uv run ruff format . && uv run ruff check .

# Run the CLI
uv run graft-cli <command>
```

## Why This Template?

Each architectural decision is documented with rationale:

- [Why uv?](docs/decisions/001-why-uv.md) - Fast, modern package management
- [Why src layout?](docs/decisions/002-src-layout.md) - Clear boundaries
- [Why Protocols?](docs/decisions/003-protocol-di.md) - Pythonic DI
- [Why functional services?](docs/decisions/004-functional-services.md) - Simplicity and testability
- [Why context objects?](docs/decisions/005-context-objects.md) - Explicit dependencies
- [Why Typer?](docs/decisions/006-typer-cli.md) - Modern CLI framework
- [Why ruff?](docs/decisions/007-ruff-tooling.md) - Fast, unified tooling

## Contributing

This template is designed to be adapted for your specific needs. Key principles:

- **Code is canonical**: Documentation explains and references code
- **First principles**: Understand the "why" behind patterns
- **Pythonic**: Follow community standards and idioms
- **Practical**: Every pattern has working examples
- **Evolvable**: Template-ready for Copier conversion

## License

MIT License - see LICENSE file for details.

## Resources

- [Architecture Documentation](docs/architecture/overview.md)
- [API Reference](docs/reference/project-structure.md)
- [Development Guides](docs/guides/getting-started.md)
- [Architectural Decisions](docs/decisions/)

---

For AI agents: See [docs/agents.md](docs/agents.md) for structured technical reference.
