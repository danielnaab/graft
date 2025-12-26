---
title: Dependency Injection
status: stable
owners: [architecture-team]
---

# Dependency Injection

This document explains the dependency injection pattern used in this template, based on `typing.Protocol` and context objects.

## Overview

**Dependency Injection (DI)** is a technique where dependencies are provided to components rather than components creating their own dependencies.

**Benefits**:
- **Testability**: Easy to inject fakes/mocks for testing
- **Flexibility**: Swap implementations without changing code
- **Decoupling**: Components depend on interfaces, not concrete classes
- **Clarity**: Dependencies are explicit and visible

## Our Approach: Protocol-Based DI

This template uses a **lightweight, Pythonic** approach to DI:

1. Define interfaces using `typing.Protocol` (structural typing)
2. Inject dependencies via `ServiceContext` (frozen dataclass)
3. No DI framework required

## Protocols: Structural Typing

### What is a Protocol?

A `Protocol` defines an interface using **structural subtyping** (duck typing with type safety).

**Key insight**: Any class implementing the required methods satisfies the protocol - **no inheritance needed**.

### Example Protocol

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
        """Delete entity by ID. Returns True if deleted, False if not found."""
        ...
```

**Note**: The `...` (ellipsis) indicates protocol method - no implementation.

### Implementing a Protocol

Implementations don't need to inherit from the protocol:

```python
from typing import Generic, TypeVar, Optional

T = TypeVar("T")

class InMemoryRepository(Generic[T]):
    """In-memory repository implementation.

    Satisfies Repository[T] protocol through structural typing.
    No explicit inheritance from Repository required.
    """

    def __init__(self) -> None:
        self._storage: dict[str, T] = {}

    def save(self, entity: T) -> None:
        """Store entity in memory."""
        self._storage[entity.id] = entity

    def get(self, entity_id: str) -> Optional[T]:
        """Retrieve from memory."""
        return self._storage.get(entity_id)

    def list_all(self) -> list[T]:
        """List all in-memory entities."""
        return list(self._storage.values())

    def delete(self, entity_id: str) -> bool:
        """Delete from memory."""
        if entity_id in self._storage:
            del self._storage[entity_id]
            return True
        return False
```

**Magic**: `InMemoryRepository` satisfies `Repository[T]` protocol because it has all the required methods with correct signatures.

### Type Checking

Type checkers (mypy, pyright) verify protocol compliance:

```python
def my_function(repo: Repository[User]) -> None:
    """Accepts any implementation of Repository[User]."""
    users = repo.list_all()

# All valid - satisfy the protocol
my_function(InMemoryRepository[User]())
my_function(PostgresRepository[User](db_url))
my_function(FakeRepository[User]())
```

### Protocols vs Abstract Base Classes (ABC)

| Aspect | Protocol | ABC |
|--------|----------|-----|
| **Typing** | Structural (duck typing) | Nominal (inheritance) |
| **Inheritance** | Not required | Required |
| **Pythonic** | Very (matches duck typing) | Less (Java/C# style) |
| **Flexibility** | High (any matching class) | Lower (must inherit) |
| **Runtime** | No enforcement | Can enforce at runtime |
| **When to use** | Preferred for most cases | When runtime checks needed |

**Example with ABC** (not used in this template):
```python
from abc import ABC, abstractmethod

class Repository(ABC, Generic[T]):
    @abstractmethod
    def save(self, entity: T) -> None: ...

# Must explicitly inherit
class InMemoryRepository(Repository[T]):  # Must inherit
    def save(self, entity: T) -> None:
        self._storage[entity.id] = entity
```

**Protocols are preferred** because they're more Pythonic and flexible.

## Context Objects: DI Container

Dependencies are injected through a **frozen dataclass** called `ServiceContext`.

### ServiceContext Definition

```python
from dataclasses import dataclass
from typing import Protocol

@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependency container.

    Contains all external dependencies needed by service functions.

    Attributes:
        repository: Entity persistence
        external_api: Third-party API client
        config: Application configuration
        logger: Structured logger

    Note:
        Frozen to ensure immutability and thread safety.
        All fields use Protocol types for maximum flexibility.
    """
    repository: Repository[Entity]
    external_api: ExternalAPI
    config: Config
    logger: Logger
```

### Why Frozen Dataclass?

**`frozen=True`** makes the context immutable:

```python
ctx = ServiceContext(...)
ctx.repository = different_repo  # ❌ FrozenInstanceError

# Context can be safely shared across functions
# No risk of accidental mutation
```

**Benefits of immutability**:
- Thread-safe
- Predictable behavior
- No hidden mutations
- Easier to reason about

### Type Safety

All fields are type-hinted:

```python
def my_service(ctx: ServiceContext) -> None:
    # IDE knows ctx.repository is Repository[Entity]
    ctx.repository.save(entity)  # ✅ Autocomplete works

    # Type checker catches errors
    ctx.repository.invalid_method()  # ❌ Type error
```

### Protocol Fields

Fields use Protocol types, not concrete classes:

```python
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]  # ✅ Protocol (flexible)
    # NOT: PostgresRepository         # ❌ Concrete (inflexible)
```

This allows **any implementation** satisfying the protocol:
- Production: `PostgresRepository`
- Testing: `FakeRepository`, `InMemoryRepository`
- Local dev: `SQLiteRepository`

## Injecting Implementations

### Production Context (CLI Layer)

The CLI layer builds the context with **real adapters**:

```python
# src/starter/cli/main.py

import os
from starter.services.context import ServiceContext
from starter.adapters.repository import PostgresRepository
from starter.adapters.external_api import HTTPExternalAPI

def get_context() -> ServiceContext:
    """Build production context with real adapters.

    Reads configuration from environment variables.
    """
    return ServiceContext(
        repository=PostgresRepository(
            connection_string=os.getenv("DATABASE_URL"),
        ),
        external_api=HTTPExternalAPI(
            base_url=os.getenv("API_BASE_URL"),
            api_key=os.getenv("API_KEY"),
        ),
        config=Config.from_env(),
        logger=setup_logger(),
    )

@app.command()
def create(name: str, value: int) -> None:
    """Create example entity."""
    ctx = get_context()  # Build context with real adapters
    entity = create_example(ctx, name=name, value=value)
    typer.echo(f"Created: {entity}")
```

### Test Context (Test Fixtures)

Tests build context with **fakes**:

```python
# tests/conftest.py

import pytest
from starter.services.context import ServiceContext
from tests.fakes.fake_repository import FakeRepository
from tests.fakes.fake_external_api import FakeExternalAPI

@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with fake adapters.

    Returns:
        ServiceContext with in-memory fakes for testing
    """
    return ServiceContext(
        repository=FakeRepository(),
        external_api=FakeExternalAPI(),
        config=Config(debug=True, environment="test"),
        logger=NullLogger(),  # No logging in tests
    )

# Using the fixture
def test_create_example(test_context):
    """Test with fake context."""
    entity = create_example(test_context, name="test", value=100)
    assert entity.name == "test"
```

### Integration Test Context

Integration tests use **real adapters** with test infrastructure:

```python
@pytest.fixture
def db_context(test_database):
    """Build context with real database (test instance).

    Args:
        test_database: Fixture providing test database connection

    Returns:
        ServiceContext with real database repository
    """
    return ServiceContext(
        repository=PostgresRepository(test_database.connection_string),
        external_api=FakeExternalAPI(),  # Still use fake for external API
        config=Config(environment="test"),
        logger=setup_logger(level="DEBUG"),
    )

def test_create_example_integration(db_context):
    """Integration test with real database."""
    entity = create_example(db_context, name="integration", value=200)

    # Verify in real database
    retrieved = db_context.repository.get(entity.id)
    assert retrieved is not None
    assert retrieved.name == "integration"
```

## How Protocols Work

### Structural Subtyping

Python's type system checks if an object has the required structure:

```python
class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> Optional[T]: ...

# Type checker asks: "Does this class have save() and get() methods?"
# If yes → satisfies protocol
# If no → type error

class InMemoryRepository(Generic[T]):
    def save(self, entity: T) -> None:
        # Implementation

    def get(self, entity_id: str) -> Optional[T]:
        # Implementation

# ✅ InMemoryRepository satisfies Repository[T] protocol
# Even without inheriting from it!
```

### Runtime vs Static Checking

**Static checking** (mypy, pyright):
```python
repo: Repository[User] = InMemoryRepository[User]()  # ✅ Type-safe
repo.save(user)  # ✅ Type checker approves
```

**Runtime checking** (optional):
```python
from typing import runtime_checkable

@runtime_checkable
class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...

repo = InMemoryRepository[User]()
isinstance(repo, Repository)  # True (but usually not needed)
```

**Note**: Runtime checking rarely needed. Static type checking is sufficient.

### Generic Protocols

Protocols can be generic to work with any entity type:

```python
from typing import Protocol, Generic, TypeVar

T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    """Generic repository for any entity type T."""
    def save(self, entity: T) -> None: ...

# Use with specific types
user_repo: Repository[User] = InMemoryRepository[User]()
order_repo: Repository[Order] = PostgresRepository[Order](db_url)

user_repo.save(user)    # ✅ Type-safe: T = User
user_repo.save(order)   # ❌ Type error: Order is not User
```

## Benefits of Our DI Approach

### 1. No Framework Required

**Lightweight**: Just standard library (`typing`, `dataclasses`)

```python
# No need for:
# - dependency-injector
# - injector
# - pinject
# - Any other DI framework

# Just use dataclasses and protocols
```

### 2. Transparent and Explicit

**No magic**: Clear where dependencies come from

```python
# Services explicitly show dependencies
def create_user(ctx: ServiceContext, email: str) -> User:
    # Clear that we need: repository, config, logger
    user = User(email=email)
    ctx.repository.save(user)
    return user

# CLI explicitly builds context
def get_context() -> ServiceContext:
    return ServiceContext(
        repository=PostgresRepository(...),
        config=Config.from_env(),
        logger=setup_logger(),
    )
```

### 3. Testability

**Easy to inject fakes**:

```python
def test_create_user():
    # Build test context with fakes
    ctx = ServiceContext(
        repository=FakeRepository(),
        config=test_config(),
        logger=NullLogger(),
    )

    # Test with fakes
    user = create_user(ctx, "test@example.com")
    assert user.email == "test@example.com"
```

### 4. Flexibility

**Swap implementations** without changing service code:

```python
# Development: SQLite
ctx = ServiceContext(repository=SQLiteRepository(...))

# Production: PostgreSQL
ctx = ServiceContext(repository=PostgresRepository(...))

# Testing: In-memory
ctx = ServiceContext(repository=InMemoryRepository(...))

# Service code unchanged - depends on Repository protocol
```

### 5. Type Safety

**Full type checking**:

```python
ctx = ServiceContext(
    repository=InMemoryRepository[User](),
    config=Config(),
    logger=Logger(),
)

# IDE autocomplete works
ctx.repository.  # Shows: save, get, list_all, delete

# Type checker catches errors
ctx.repository.invalid_method()  # ❌ Type error
```

## Context Patterns

### Multiple Contexts

For large applications, split into domain-specific contexts:

```python
@dataclass(frozen=True)
class UserServiceContext:
    """Context for user-related services."""
    user_repository: Repository[User]
    email_service: EmailService
    auth_service: AuthService

@dataclass(frozen=True)
class OrderServiceContext:
    """Context for order-related services."""
    order_repository: Repository[Order]
    payment_gateway: PaymentGateway
    inventory_service: InventoryService
```

### Context Composition

Nest contexts for organization:

```python
@dataclass(frozen=True)
class DatabaseContext:
    """All database repositories."""
    user_repository: Repository[User]
    order_repository: Repository[Order]
    product_repository: Repository[Product]

@dataclass(frozen=True)
class ExternalContext:
    """All external services."""
    payment_gateway: PaymentGateway
    email_service: EmailService
    sms_service: SMSService

@dataclass(frozen=True)
class ServiceContext:
    """Main service context."""
    db: DatabaseContext
    external: ExternalContext
    config: Config
    logger: Logger

# Usage
def create_order(ctx: ServiceContext, user_id: str, items: list[Item]) -> Order:
    user = ctx.db.user_repository.get(user_id)
    order = Order(user_id=user_id, items=items)
    ctx.db.order_repository.save(order)
    ctx.external.email_service.send_confirmation(user.email)
    return order
```

### Partial Contexts

For services needing only some dependencies:

```python
def simple_operation(repository: Repository[User], user_id: str) -> User:
    """Only needs repository, not full context."""
    return repository.get(user_id)

# Can pass individual fields
user = simple_operation(ctx.repository, "user-123")
```

## Related Documentation

- [Functional Services](functional-services.md) - Service function pattern
- [Context Objects Decision](../decisions/005-context-objects.md) - Why context pattern
- [Protocol DI Decision](../decisions/003-protocol-di.md) - Why protocols
- [Testing Strategy](testing-strategy.md) - Testing with DI
- [Architecture Overview](overview.md) - How DI fits in architecture

## Sources

- [PEP 544 -- Protocols: Structural subtyping](https://peps.python.org/pep-0544/) - Protocol specification
- [typing — Support for type hints](https://docs.python.org/3/library/typing.html#typing.Protocol) - Official documentation
- [mypy: Protocols and structural subtyping](https://mypy.readthedocs.io/en/stable/protocols.html) - Type checker guide
- [Dependency Injection: a Python Way](https://www.glukhov.org/post/2025/12/dependency-injection-in-python/) - DI patterns
- [Python Dataclasses](https://docs.python.org/3/library/dataclasses.html) - Dataclass documentation
