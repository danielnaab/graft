---
title: "ADR 003: Protocol-Based Dependency Injection"
status: stable
owners: [architecture-team]
---

# ADR 003: Protocol-Based Dependency Injection

## Status

Accepted

## Context

Service functions need access to external dependencies (repositories, APIs, configuration). We need a way to:

1. Define interfaces that adapters must implement
2. Enable dependency injection for testability
3. Maintain type safety
4. Keep the solution Pythonic and simple
5. Avoid heavy frameworks

## Decision

Use `typing.Protocol` for defining interfaces and inject dependencies via context objects.

**No dependency injection framework** - just protocols and dataclasses.

## Rationale

### Structural Typing (Duck Typing with Type Safety)

**Protocol** provides structural subtyping:

```python
from typing import Protocol, Generic, TypeVar

T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    """Repository interface using structural typing."""

    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> T | None: ...
```

**Key insight**: Any class with these methods satisfies the protocol - **no inheritance required**.

```python
class InMemoryRepository(Generic[T]):
    """Satisfies Repository[T] without inheriting."""

    def save(self, entity: T) -> None:
        # Implementation

    def get(self, entity_id: str) -> T | None:
        # Implementation

# ✅ InMemoryRepository satisfies Repository[T] protocol
# Type checker verifies this automatically
```

### Pythonic Duck Typing

Protocols embrace Python's duck typing philosophy:

> "If it walks like a duck and quacks like a duck, it's a duck."

**Traditional duck typing** (runtime):
```python
def process(repo):
    repo.save(entity)  # Hope repo has save()
```

**Protocol** (compile-time):
```python
def process(repo: Repository[Entity]):
    repo.save(entity)  # Type checker verifies repo has save()
```

**Best of both worlds**: Duck typing flexibility + type safety.

### No Inheritance Required

**Protocols** use structural typing - implementations don't need to inherit:

```python
# ✅ This works - has required methods
class PostgresRepository:
    def save(self, entity): ...
    def get(self, entity_id): ...

# Type checker: PostgresRepository satisfies Repository protocol
repo: Repository[User] = PostgresRepository()
```

**Compare with ABC** (nominal typing):
```python
from abc import ABC, abstractmethod

class Repository(ABC):
    @abstractmethod
    def save(self, entity): ...

# ❌ Must explicitly inherit
class PostgresRepository(Repository):  # Must inherit!
    def save(self, entity): ...
```

**Why structural is better**:
- More flexible (any compatible class works)
- Works with third-party code (can't modify to inherit)
- More Pythonic (duck typing philosophy)
- Less coupling (no inheritance hierarchy)

### Type Safety Without Framework

Full type checking without DI framework:

```python
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]  # Protocol type
    config: Config

def create_entity(ctx: ServiceContext, name: str) -> Entity:
    # Type checker knows ctx.repository has save()
    ctx.repository.save(entity)  # ✅ Type-safe

    # Type checker catches errors
    ctx.repository.invalid_method()  # ❌ Type error
```

IDE autocomplete works perfectly:
```python
ctx.repository.  # IDE shows: save, get, list_all, delete
```

### Flexibility in Testing

Easy to create fakes without framework:

```python
class FakeRepository:
    """Test fake - no inheritance needed."""

    def __init__(self):
        self._storage = {}

    def save(self, entity):
        self._storage[entity.id] = entity

    def get(self, entity_id):
        return self._storage.get(entity_id)

# ✅ FakeRepository satisfies Repository protocol
# Can use in tests without any registration or configuration
```

### No Framework Complexity

**No magic** - just standard library:

```python
from typing import Protocol
from dataclasses import dataclass

# Define interface
class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...

# Define context
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]

# Inject dependencies
ctx = ServiceContext(repository=PostgresRepository())
```

**That's it!** No:
- Framework initialization
- Decorators or registration
- XML/YAML configuration
- Magic reflection
- Complex lifetime management

## Alternatives Considered

### Abstract Base Classes (ABC)

**Pattern**:
```python
from abc import ABC, abstractmethod

class Repository(ABC, Generic[T]):
    @abstractmethod
    def save(self, entity: T) -> None:
        pass

    @abstractmethod
    def get(self, entity_id: str) -> T | None:
        pass

# Must inherit explicitly
class PostgresRepository(Repository[Entity]):
    def save(self, entity: Entity) -> None:
        # Implementation
```

**Pros**:
- Runtime enforcement (`isinstance()`, `issubclass()`)
- Can provide default implementations
- Familiar to Java/C# developers
- Explicit inheritance shows intent

**Cons**:
- Requires inheritance (nominal typing)
- Less flexible than protocols
- Can't apply to third-party code
- Less Pythonic (fights duck typing)

**Why not chosen**: Protocols are more Pythonic and flexible. Runtime enforcement rarely needed.

### Dependency Injection Framework (dependency-injector)

**Pattern**:
```python
from dependency_injector import containers, providers

class Container(containers.DeclarativeContainer):
    config = providers.Configuration()

    repository = providers.Singleton(
        PostgresRepository,
        connection_string=config.db_url,
    )

    service = providers.Factory(
        UserService,
        repository=repository,
    )
```

**Pros**:
- Sophisticated features (scopes, factories, singletons)
- Automatic dependency resolution
- Popular in enterprise contexts
- Well-documented

**Cons**:
- Additional dependency
- Learning curve
- Magic behavior (implicit wiring)
- Overkill for most Python projects
- Hides dependencies

**Why not chosen**: Simple dataclass context is sufficient and more transparent.

### Factory Pattern

**Pattern**:
```python
class ServiceFactory:
    def create_user_service(self) -> UserService:
        repo = self._create_repository()
        config = self._load_config()
        return UserService(repo, config)

    def _create_repository(self) -> Repository:
        return PostgresRepository(...)
```

**Pros**:
- Simple and explicit
- No framework needed
- Easy to understand

**Cons**:
- Boilerplate for each service
- Manual wiring (error-prone)
- No type safety for dependencies
- Hard to test (need to mock factory)

**Why not chosen**: Context object is cleaner and type-safe.

### Service Locator Pattern

**Pattern**:
```python
class ServiceLocator:
    _services = {}

    @classmethod
    def register(cls, name, service):
        cls._services[name] = service

    @classmethod
    def get(cls, name):
        return cls._services[name]

# Usage
ServiceLocator.register("repository", PostgresRepository())
repo = ServiceLocator.get("repository")
```

**Pros**:
- Centralized service management
- Global access to dependencies

**Cons**:
- Global state (anti-pattern)
- Hidden dependencies (not in function signature)
- Hard to test
- No type safety
- Considered anti-pattern

**Why not chosen**: Anti-pattern. Context object is explicit and testable.

## Consequences

### Positive

1. **Type-safe**: Full IDE and type checker support
2. **Pythonic**: Embraces duck typing philosophy
3. **Flexible**: Any compatible implementation works
4. **Simple**: No framework needed
5. **Testable**: Easy to inject fakes
6. **Explicit**: Dependencies visible in context
7. **No magic**: Clear, understandable code

### Negative

1. **No runtime enforcement**: Protocol not enforced at runtime (only static)
2. **Requires Python 3.8+**: Protocols introduced in PEP 544
3. **Less familiar**: Developers from Java/C# may prefer ABC
4. **Manual wiring**: Must build context manually (not automatic)

### Mitigations

1. **Runtime checking (optional)**:
   ```python
   from typing import runtime_checkable

   @runtime_checkable
   class Repository(Protocol):
       ...

   isinstance(obj, Repository)  # Runtime check
   ```
   (Rarely needed - static checking is sufficient)

2. **Documentation**: Explain protocol pattern in architecture docs

3. **Examples**: Provide clear examples of implementing protocols

4. **Type checking**: Use mypy or pyright in CI to catch violations

## Implementation

### Defining Protocols

```python
from typing import Protocol, Generic, TypeVar, Optional

T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    """Repository interface for entity persistence.

    Any class implementing these methods satisfies this protocol.
    No explicit inheritance required (structural subtyping).
    """

    def save(self, entity: T) -> None:
        """Persist entity to storage."""
        ...

    def get(self, entity_id: str) -> Optional[T]:
        """Retrieve entity by ID. Returns None if not found."""
        ...

    def list_all(self) -> list[T]:
        """Retrieve all entities."""
        ...

    def delete(self, entity_id: str) -> bool:
        """Delete entity by ID. Returns True if deleted."""
        ...
```

**Note**: `...` (ellipsis) indicates protocol method - no implementation.

### Implementing Protocols

**No inheritance needed**:

```python
class PostgresRepository(Generic[T]):
    """PostgreSQL repository implementation.

    Satisfies Repository[T] protocol through structural typing.
    """

    def __init__(self, connection_string: str):
        self._conn = create_connection(connection_string)

    def save(self, entity: T) -> None:
        # Implementation with actual database
        ...

    def get(self, entity_id: str) -> Optional[T]:
        # Implementation
        ...

    # etc.
```

### Using in Context

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependencies.

    All fields use Protocol types for flexibility.
    """
    repository: Repository[Entity]  # Protocol, not concrete class
    external_api: ExternalAPI        # Protocol
    config: Config
    logger: Logger

# Build context with any compatible implementation
production_ctx = ServiceContext(
    repository=PostgresRepository(db_url),  # ✅ Satisfies protocol
    external_api=HTTPExternalAPI(api_url),  # ✅ Satisfies protocol
    config=Config.from_env(),
    logger=setup_logger(),
)

test_ctx = ServiceContext(
    repository=FakeRepository(),            # ✅ Satisfies protocol
    external_api=FakeExternalAPI(),         # ✅ Satisfies protocol
    config=test_config(),
    logger=NullLogger(),
)
```

### Type Checking

**mypy** and **pyright** verify protocol compliance:

```bash
# Check types
uv run mypy src/

# pyright (VS Code)
# Automatic checking in editor
```

**Example type error**:
```python
class BrokenRepository:
    def save(self, entity):
        pass
    # Missing get() method!

repo: Repository[User] = BrokenRepository()
# Type error: BrokenRepository missing 'get' method
```

## Protocol Best Practices

### 1. Keep Protocols Small

**Good** (focused):
```python
class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> T | None: ...
```

**Bad** (too large):
```python
class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> T | None: ...
    def list_all(self) -> list[T]: ...
    def delete(self, entity_id: str) -> bool: ...
    def count(self) -> int: ...
    def exists(self, entity_id: str) -> bool: ...
    # 20 more methods...
```

### 2. Use Generic Protocols

Make protocols reusable with TypeVars:

```python
T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    """Works with any entity type."""
    def save(self, entity: T) -> None: ...

# Use with specific types
user_repo: Repository[User] = ...
order_repo: Repository[Order] = ...
```

### 3. Document Protocol Contracts

```python
class Repository(Protocol, Generic[T]):
    """Repository interface for entity persistence.

    Implementations must:
    - Ensure save() is idempotent
    - Return None from get() if entity not found
    - Raise RepositoryError on storage failures
    """
    def save(self, entity: T) -> None:
        """Persist entity. Idempotent (safe to call multiple times)."""
        ...
```

## Related Decisions

- [ADR 004: Functional Services](004-functional-services.md) - Services use protocols
- [ADR 005: Context Objects](005-context-objects.md) - Context contains protocol types
- [Dependency Injection Architecture](../architecture/dependency-injection.md) - DI deep dive

## Sources

- [PEP 544 -- Protocols: Structural subtyping](https://peps.python.org/pep-0544/) - Protocol specification
- [typing — Support for type hints](https://docs.python.org/3/library/typing.html#typing.Protocol) - Official docs
- [mypy: Protocols and structural subtyping](https://mypy.readthedocs.io/en/stable/protocols.html) - Type checker guide
- [Dependency Injection: a Python Way](https://www.glukhov.org/post/2025/12/dependency-injection-in-python/) - DI patterns in Python
- [Effective Python](https://effectivepython.com/) - Python best practices
