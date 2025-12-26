---
title: Architecture Overview
status: stable
owners: [architecture-team]
---

# Architecture Overview

This document provides a high-level overview of the Python Starter Template architecture, explaining the key layers, patterns, and design principles.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Layer                             │
│  (Typer commands - thin adapter over services)              │
│  - Parses user input                                         │
│  - Builds ServiceContext with real adapters                  │
│  - Calls service functions                                   │
│  - Formats output for user                                   │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────────────────┐
│                      Service Layer                           │
│  (Pure functions taking ServiceContext)                      │
│  - Orchestrates use cases                                    │
│  - Implements business workflows                             │
│  - Calls adapters via context                                │
│  - Returns results or raises exceptions                      │
└───────┬──────────────────────────────────┬──────────────────┘
        │                                  │
        │                                  │
        ↓                                  ↓
┌──────────────────┐              ┌──────────────────┐
│  Domain Layer    │              │  Adapter Layer   │
│  (Entities, VOs) │              │  (Implementations)│
│  - Pure logic    │              │  - Repositories  │
│  - No deps       │              │  - External APIs │
│  - Immutable     │              │  - File I/O      │
└──────────────────┘              └────────┬─────────┘
                                           │
                                           │
                                           ↓
                                  ┌──────────────────┐
                                  │  Protocol Layer  │
                                  │  (Interfaces)    │
                                  │  - Protocols     │
                                  │  - Type contracts│
                                  └──────────────────┘
```

## Key Architectural Layers

### 1. CLI Layer

**Purpose**: Provide command-line interface to the application.

**Location**: `src/starter/cli/`

**Responsibilities**:
- Parse command-line arguments using Typer
- Build `ServiceContext` with real adapter implementations
- Call service functions with parsed parameters
- Format and display results to user
- Handle top-level error presentation

**Pattern**: Thin adapter - minimal logic, delegates to services.

**Example**:
```python
@app.command()
def create(name: str, value: int) -> None:
    """Create example entity."""
    ctx = get_context()  # Build context with real adapters
    entity = create_example(ctx, name=name, value=value)  # Call service
    typer.echo(f"Created: {entity}")  # Display result
```

### 2. Service Layer

**Purpose**: Implement business use cases and workflows.

**Location**: `src/starter/services/`

**Responsibilities**:
- Orchestrate business operations
- Validate business rules
- Coordinate between domain and adapters
- Implement transaction boundaries
- Return results or raise domain exceptions

**Pattern**: Pure functions taking `ServiceContext` as first parameter.

**Example**:
```python
def create_example(ctx: ServiceContext, name: str, value: int) -> Entity:
    """Create and persist example entity."""
    # Validate
    if value < 0:
        raise ValueError("Value must be non-negative")

    # Domain logic
    entity = Entity(name=name, value=value)

    # Adapter call
    ctx.repository.save(entity)

    return entity
```

**Why functions?** See [Functional Services](functional-services.md) and [ADR 004](../decisions/004-functional-services.md).

### 3. Domain Layer

**Purpose**: Encapsulate core business logic and rules.

**Location**: `src/starter/domain/`

**Responsibilities**:
- Define entities (objects with identity)
- Define value objects (immutable values)
- Implement domain business rules
- Validation logic
- No external dependencies

**Pattern**: Dataclasses for entities and value objects.

**Example**:
```python
@dataclass
class Entity:
    """Domain entity with business logic."""
    name: str
    value: int
    id: str = field(default_factory=lambda: str(uuid4()))

    def __post_init__(self):
        if not self.name:
            raise ValueError("Name required")
        if self.value < 0:
            raise ValueError("Value must be non-negative")
```

**Why dataclasses?** See [Domain Model](domain-model.md).

### 4. Adapter Layer

**Purpose**: Implement interfaces for external systems.

**Location**: `src/starter/adapters/`

**Responsibilities**:
- Implement protocol interfaces
- Handle external system communication
- Translate between domain and external formats
- Manage connections and resources
- Handle adapter-specific errors

**Pattern**: Concrete implementations of protocols.

**Example**:
```python
class InMemoryRepository(Generic[T]):
    """Repository implementation using in-memory dict."""

    def __init__(self) -> None:
        self._storage: dict[str, T] = {}

    def save(self, entity: T) -> None:
        self._storage[entity.id] = entity

    def get(self, entity_id: str) -> T | None:
        return self._storage.get(entity_id)
```

**Why separate adapters?** Dependency inversion - domain doesn't depend on infrastructure.

### 5. Protocol Layer

**Purpose**: Define interfaces that adapters must implement.

**Location**: `src/starter/protocols/`

**Responsibilities**:
- Define structural interfaces using `typing.Protocol`
- Specify method signatures
- Document interface contracts
- Enable dependency inversion

**Pattern**: Generic protocols with no implementation.

**Example**:
```python
class Repository(Protocol, Generic[T]):
    """Repository interface for entity persistence."""

    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> T | None: ...
    def list_all(self) -> list[T]: ...
```

**Why protocols?** See [Dependency Injection](dependency-injection.md) and [ADR 003](../decisions/003-protocol-di.md).

## Data Flow

### Typical Request Flow

1. **User input** → CLI layer (Typer command)
2. **CLI** → Builds `ServiceContext` with adapter instances
3. **CLI** → Calls service function with context + parameters
4. **Service** → Validates inputs
5. **Service** → Creates/modifies domain entities
6. **Service** → Calls adapters via context (e.g., `ctx.repository.save()`)
7. **Adapter** → Performs external operation (DB, API, file)
8. **Service** → Returns result or raises exception
9. **CLI** → Formats result for display
10. **User output** ← CLI layer

### Example: Create Entity Flow

```python
# 1. User runs: starter-cli create "example" 100

# 2. CLI command handler
@app.command()
def create(name: str, value: int) -> None:
    # 3. Build context with real adapters
    ctx = ServiceContext(
        repository=PostgresRepository(),
        config=Config.from_env(),
        logger=setup_logger(),
    )

    # 4. Call service function
    entity = create_example(ctx, name=name, value=value)

    # 5. Display result
    typer.echo(f"Created: {entity}")

# Service function
def create_example(ctx: ServiceContext, name: str, value: int) -> Entity:
    # 6. Validate
    if value < 0:
        raise ValueError("Value must be non-negative")

    # 7. Create domain entity
    entity = Entity(name=name, value=value)

    # 8. Persist via adapter
    ctx.repository.save(entity)

    # 9. Return result
    return entity
```

## Dependency Injection via Context

### ServiceContext Pattern

Dependencies are injected through a **frozen dataclass** called `ServiceContext`:

```python
@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependency container.

    Contains all external dependencies needed by service functions.
    Immutable to ensure thread safety and predictable behavior.
    """
    repository: Repository[Entity]
    external_api: ExternalAPI
    config: Config
    logger: Logger
```

**Benefits**:
- **Explicit**: All dependencies visible in one place
- **Type-safe**: Full IDE and type checker support
- **Immutable**: Thread-safe, no hidden mutations
- **Testable**: Easy to construct test contexts with fakes

**Production context** (CLI layer):
```python
def get_context() -> ServiceContext:
    """Build production context with real adapters."""
    return ServiceContext(
        repository=PostgresRepository(db_url=os.getenv("DB_URL")),
        external_api=HTTPExternalAPI(base_url=config.api_url),
        config=Config.from_env(),
        logger=structlog.get_logger(),
    )
```

**Test context** (test fixtures):
```python
@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with fakes."""
    return ServiceContext(
        repository=FakeRepository(),
        external_api=FakeExternalAPI(),
        config=Config(debug=True),
        logger=NullLogger(),
    )
```

See [Dependency Injection](dependency-injection.md) and [ADR 005](../decisions/005-context-objects.md).

## Key Design Principles

### 1. Clean Architecture

**Dependency Rule**: Dependencies point inward.

- Domain has no dependencies
- Services depend on domain and protocols (not adapters)
- Adapters depend on protocols
- CLI depends on services and adapters (wiring layer)

**Benefits**: Domain logic isolated from infrastructure concerns.

### 2. Dependency Inversion

**Principle**: Depend on abstractions (protocols), not concretions (adapters).

Services declare dependencies on `Repository[T]` protocol, not `PostgresRepository`. This allows:
- Easy testing (inject fakes)
- Flexible implementations (swap databases)
- No framework lock-in

### 3. Functional Core, Imperative Shell

**Pattern**: Pure functional core (services), imperative shell (CLI, adapters).

- **Services**: Pure functions, deterministic, easy to test
- **CLI/Adapters**: Handle side effects (I/O, external systems)

### 4. Explicit is Better Than Implicit

**Zen of Python**: Make dependencies and data flow visible.

- Context makes dependencies explicit
- Function signatures show inputs/outputs
- No hidden globals or singletons
- Type hints everywhere

### 5. Code is Canonical

**Authority**: Source code is the source of truth.

- Configuration in `pyproject.toml`
- Behavior in source code
- Tests document expected behavior
- Documentation explains "why"

## Testing Strategy

### Test Pyramid

```
        ┌──────────┐
        │    E2E   │  Few - slow, brittle
        ├──────────┤
        │Integration│ Some - moderate speed
        ├──────────┤
        │   Unit    │ Many - fast, isolated
        └──────────┘
```

### Unit Tests

**Focus**: Service functions and domain logic in isolation.

**Approach**: Use fakes instead of mocks.

```python
def test_create_example(test_context):
    entity = create_example(test_context, name="test", value=100)
    assert entity.name == "test"
    assert test_context.repository.get(entity.id) == entity
```

**Location**: `tests/unit/`

### Integration Tests

**Focus**: Adapters with real external systems.

**Approach**: Use test databases, test APIs.

```python
def test_repository_integration(db_context):
    entity = Entity(name="test", value=100)
    db_context.repository.save(entity)

    retrieved = db_context.repository.get(entity.id)
    assert retrieved == entity
```

**Location**: `tests/integration/`

See [Testing Strategy](testing-strategy.md) for comprehensive guidance.

## Technology Choices

### Why These Tools?

- **uv**: Fast, modern package manager - [ADR 001](../decisions/001-why-uv.md)
- **ruff**: Unified linting/formatting - [ADR 007](../decisions/007-ruff-tooling.md)
- **pytest**: Industry-standard testing
- **Typer**: Modern CLI with type hints - [ADR 006](../decisions/006-typer-cli.md)
- **typing.Protocol**: Pythonic DI - [ADR 003](../decisions/003-protocol-di.md)

### Why Functions for Services?

Services are pure functions (not classes) because:
- More Pythonic for stateless operations
- Easier to test
- Better composition
- Explicit dependencies
- Simpler mental model

See [Functional Services](functional-services.md) and [ADR 004](../decisions/004-functional-services.md).

## Common Patterns

### Adding a Use Case

1. Define service function in `services/`
2. Accept `ServiceContext` as first parameter
3. Implement business logic
4. Add unit test with fake context
5. Wire to CLI command if needed

### Adding an Adapter

1. Define protocol in `protocols/`
2. Implement in `adapters/`
3. Add to `ServiceContext`
4. Create fake in `tests/fakes/`
5. Add integration test

### Adding a Domain Entity

1. Create dataclass in `domain/entities.py`
2. Add validation in `__post_init__`
3. Add business methods if needed
4. Write unit tests

## Related Documentation

- [Functional Services](functional-services.md) - Service layer deep dive
- [Dependency Injection](dependency-injection.md) - DI pattern details
- [Domain Model](domain-model.md) - Entity and value object patterns
- [Testing Strategy](testing-strategy.md) - Testing approach
- [Architectural Decisions](../decisions/) - ADRs for all major choices

## Sources

- [Architecture Patterns with Python (Cosmic Python)](https://www.cosmicpython.com/book/)
- [Clean Architecture - Robert C. Martin](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture - Alistair Cockburn](https://alistair.cockburn.us/hexagonal-architecture/)
- [Functional Core, Imperative Shell - Gary Bernhardt](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell)
- [PEP 20 -- The Zen of Python](https://peps.python.org/pep-0020/)
