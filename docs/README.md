---
title: Python Starter Template Documentation
status: stable
owners: [template-team]
---

# Python Starter Template Documentation

Welcome to the Python Starter Template documentation. This template provides a modern, well-architected foundation for building Python applications using clean architecture principles and functional programming patterns.

## What's Inside

This template demonstrates:

- **Modern 2025 Python tooling**: uv, ruff, pytest
- **Functional service layer**: Pure functions with context objects
- **Clean patterns**: Protocol-based dependency injection
- **Domain modeling**: Entities and value objects
- **Well-tested code**: Unit and integration test patterns
- **CLI interface**: Typer-based command-line application
- **Template-ready**: Designed for Copier conversion

## Quick Start

New to this template? Start here:

1. [Getting Started Guide](guides/getting-started.md) - Installation and first steps
2. [Development Workflow](guides/development-workflow.md) - Day-to-day development
3. [CLI Usage](guides/cli-usage.md) - Using the command-line interface

## Architecture

This template uses a **functional service layer architecture** where services are pure functions that accept a context object containing all dependencies.

```python
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]
    config: Config
    logger: Logger

def create_example(ctx: ServiceContext, name: str, value: int) -> Entity:
    """Pure function - testable, composable, explicit."""
    entity = Entity(name=name, value=value)
    ctx.repository.save(entity)
    ctx.logger.info(f"Created: {entity.id}")
    return entity
```

**Key Benefits**:
- More Pythonic than service classes
- Easier to test (pure functions with fakes)
- Better composition and reusability
- Explicit dependencies (no hidden state)
- Simpler mental model

Learn more: [Architecture Overview](architecture/overview.md)

## Navigation

### For Users

Getting up and running:
- [Getting Started](guides/getting-started.md) - Installation and setup
- [Development Workflow](guides/development-workflow.md) - Daily development
- [Adding Features](guides/adding-features.md) - Extending the template
- [Testing Guide](guides/testing-guide.md) - Writing and running tests
- [CLI Usage](guides/cli-usage.md) - Command-line interface reference

### For Architects

Understanding the design:
- [Architecture Overview](architecture/overview.md) - High-level architecture
- [Functional Services](architecture/functional-services.md) - Service layer pattern
- [Dependency Injection](architecture/dependency-injection.md) - Protocol-based DI
- [Domain Model](architecture/domain-model.md) - Entities and value objects
- [Testing Strategy](architecture/testing-strategy.md) - Testing philosophy

### For Decision Makers

Understanding the choices:
- [ADR 001: Why uv?](decisions/001-why-uv.md) - Package management
- [ADR 002: Why src layout?](decisions/002-src-layout.md) - Project structure
- [ADR 003: Protocol-based DI](decisions/003-protocol-di.md) - Interface definition
- [ADR 004: Functional Services](decisions/004-functional-services.md) - Service pattern
- [ADR 005: Context Objects](decisions/005-context-objects.md) - DI mechanism
- [ADR 006: Typer CLI](decisions/006-typer-cli.md) - CLI framework
- [ADR 007: Ruff Tooling](decisions/007-ruff-tooling.md) - Linting/formatting

### For Template Maintainers

Evolving the template:
- [Template Design](copier/template-design.md) - Copier conversion strategy
- [Variable Mapping](copier/variable-mapping.md) - Template variables
- [Customization Points](copier/customization-points.md) - User customization

### Technical Reference

Looking up specifics:
- [Project Structure](reference/project-structure.md) - Directory layout
- [Configuration](reference/configuration.md) - pyproject.toml deep dive
- [Tooling](reference/tooling.md) - uv, ruff, pytest reference
- [Protocols](reference/protocols.md) - Protocol catalog
- [Context Objects](reference/context-objects.md) - Context patterns

### For AI Agents

See [agents.md](agents.md) for structured technical reference optimized for AI tools.

## Why This Template?

### Modern Tooling

**uv**: Fast, Rust-based package manager that replaces pip, pip-tools, and virtualenv with better performance and UX.

**ruff**: Single tool for linting and formatting (replaces Black, Flake8, isort) with incredible speed.

**pytest**: Industry-standard testing framework with powerful fixtures and plugins.

**Typer**: Modern CLI framework with type-hint based interface and excellent developer experience.

### Clean Architecture

The template follows clean architecture principles:

- **Domain layer**: Pure business logic (entities, value objects)
- **Service layer**: Use cases and workflows (pure functions)
- **Adapter layer**: External system implementations
- **Protocol layer**: Interface definitions (structural typing)
- **CLI layer**: Thin adapter over services

### Functional Patterns

We use **functions over classes** for the service layer because:

1. **Pythonic**: Python embraces multiple paradigms; functions are natural
2. **Testable**: Pure functions are trivial to test
3. **Composable**: Functions compose naturally
4. **Explicit**: Context makes dependencies visible
5. **Simple**: No instance state or lifecycle to manage

### Protocol-Based DI

We use `typing.Protocol` for dependency injection because:

1. **Structural typing**: Duck typing with type safety
2. **No inheritance**: Implementations don't need to inherit
3. **Flexible**: Easy to create fakes and mocks
4. **Pythonic**: Modern Python idiom (PEP 544)
5. **No framework**: Simple and transparent

## Design Principles

This template follows key principles:

1. **Code is canonical**: Source code is the source of truth; docs explain
2. **First principles**: Documentation explains "why" not just "what"
3. **Evolvable**: Template-ready for Copier conversion
4. **Pythonic**: Follow Python idioms and community standards
5. **Practical**: Every pattern has working code examples
6. **Well-documented**: Architecture decisions are recorded and explained

## Getting Help

- **Issues**: Report bugs or request features via your issue tracker
- **Discussions**: Ask questions about architecture or usage
- **Documentation**: All major decisions are documented in [ADRs](decisions/)
- **Code**: Source code includes comprehensive docstrings and comments

## Contributing

When extending or modifying this template:

1. Follow existing patterns and conventions
2. Add tests for new functionality
3. Update documentation to reflect changes
4. Run linting and formatting: `uv run ruff check . && uv run ruff format .`
5. Ensure tests pass: `uv run pytest`
6. Document significant decisions in [decisions/](decisions/)

## Template Evolution

This template is designed to evolve into a **Copier template** for easy project generation. Key aspects:

- Parameterization points are documented in [copier/](copier/)
- Core architecture patterns remain fixed
- Project-specific details (names, authors) become template variables
- See [Template Design](copier/template-design.md) for conversion strategy

## Sources

This template synthesizes best practices from:

- [Architecture Patterns with Python (Cosmic Python)](https://www.cosmicpython.com/book/)
- [Python Packaging User Guide](https://packaging.python.org/)
- [PEP 544 -- Protocols: Structural subtyping](https://peps.python.org/pep-0544/)
- [uv Documentation](https://docs.astral.sh/uv/)
- [ruff Documentation](https://docs.astral.sh/ruff/)
- [Typer Documentation](https://typer.tiangolo.com/)
- [pytest Documentation](https://docs.pytest.org/)

---

**Note**: This documentation follows the conventions defined in [../meta-knowledge-base/docs/meta.md](../meta-knowledge-base/docs/meta.md).
