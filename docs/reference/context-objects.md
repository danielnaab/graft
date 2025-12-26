---
title: Context Objects Reference
status: stable
owners: [documentation-team]
---

# Context Objects Reference

Reference for ServiceContext and context object patterns.

## ServiceContext

**Location**: `src/starter/services/context.py`

### Definition

```python
from dataclasses import dataclass
from typing import Protocol
from starter.protocols.repository import Repository
from starter.domain.entities import Entity

@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependency container.

    Contains all external dependencies needed by service functions.
    Immutable to ensure thread safety and predictable behavior.

    Attributes:
        repository: Entity persistence (Protocol)
        config: Application configuration
        logger: Structured logger (Protocol)
    """
    repository: Repository[Entity]
    config: Config
    logger: Logger
```

### Key Attributes

| Attribute | Purpose |
|-----------|---------|
| `frozen=True` | Immutable (cannot be modified after creation) |
| Protocol types | Flexible implementations (any class satisfying protocol) |
| Type hints | Full IDE/type checker support |

## Building Contexts

### Production Context (CLI Layer)

```python
# src/starter/cli/main.py

import os
from starter.services.context import ServiceContext
from starter.adapters.repository import PostgresRepository
from starter.config import Config
from starter.logging import setup_logger

def get_context() -> ServiceContext:
    """Build production context with real adapters.

    Reads configuration from environment variables.
    """
    return ServiceContext(
        repository=PostgresRepository(
            connection_string=os.getenv("DATABASE_URL"),
        ),
        config=Config.from_env(),
        logger=setup_logger(),
    )
```

### Test Context (Fixtures)

```python
# tests/conftest.py

import pytest
from starter.services.context import ServiceContext
from tests.fakes.fake_repository import FakeRepository

@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with fakes."""
    return ServiceContext(
        repository=FakeRepository(),
        config=Config(debug=True, environment="test"),
        logger=NullLogger(),
    )
```

### Development Context

```python
def get_dev_context() -> ServiceContext:
    """Build development context."""
    return ServiceContext(
        repository=SQLiteRepository("dev.db"),  # Local SQLite
        config=Config(debug=True),
        logger=setup_logger(level="DEBUG"),
    )
```

## Using Context in Services

### Service Function Pattern

```python
def create_example(
    ctx: ServiceContext,
    name: str,
    value: int,
) -> Entity:
    """Create example entity.

    Args:
        ctx: Service context with dependencies
        name: Entity name
        value: Entity value

    Returns:
        Created entity
    """
    # Validate
    if value < 0:
        raise ValueError("Value must be non-negative")

    # Create entity
    entity = Entity(name=name, value=value)

    # Use dependencies from context
    ctx.repository.save(entity)
    ctx.logger.info(f"Created entity: {entity.id}")

    return entity
```

### Accessing Dependencies

```python
def my_service(ctx: ServiceContext, param: str) -> Result:
    # Access repository
    entity = ctx.repository.get(param)

    # Access config
    max_retries = ctx.config.max_retries

    # Access logger
    ctx.logger.info(f"Processing {param}")
```

## Context Patterns

### Multiple Contexts (Large Applications)

Split into domain-specific contexts:

```python
@dataclass(frozen=True)
class UserServiceContext:
    """Context for user services."""
    user_repository: Repository[User]
    email_service: EmailService
    config: Config
    logger: Logger

@dataclass(frozen=True)
class OrderServiceContext:
    """Context for order services."""
    order_repository: Repository[Order]
    payment_gateway: PaymentGateway
    inventory_service: InventoryService
    config: Config
    logger: Logger
```

**Benefits**:
- Smaller, focused contexts
- Services only see relevant dependencies
- Easier to understand

### Nested Contexts

Organize dependencies hierarchically:

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
    """Main context with nested contexts."""
    db: DatabaseContext
    external: ExternalContext
    config: Config
    logger: Logger

# Usage
def create_order(ctx: ServiceContext, user_id: str) -> Order:
    user = ctx.db.user_repository.get(user_id)
    order = Order(user_id=user_id)
    ctx.db.order_repository.save(order)
    ctx.external.email_service.send_confirmation(user.email)
    return order
```

### Partial Contexts

When function needs only some dependencies:

```python
def simple_operation(
    repository: Repository[Entity],
    entity_id: str,
) -> Entity:
    """Only needs repository, not full context."""
    return repository.get(entity_id)

# Extract from context
entity = simple_operation(ctx.repository, "entity-123")
```

## Modifying Contexts

### Using dataclasses.replace

```python
from dataclasses import replace

# Original context
ctx = ServiceContext(
    repository=InMemoryRepository(),
    config=Config(debug=False),
    logger=Logger(),
)

# Create new context with different config
new_ctx = replace(ctx, config=Config(debug=True))

# Original unchanged
assert ctx.config.debug is False
assert new_ctx.config.debug is True
```

### Test-Specific Modifications

```python
@pytest.fixture
def test_context():
    return ServiceContext(...)

def test_with_custom_config(test_context):
    """Test with custom configuration."""
    ctx = replace(test_context, config=Config(feature_flag=True))

    result = my_service(ctx, param="test")
    assert result.feature_used is True
```

## Context Composition

### Building Incrementally

```python
class ContextBuilder:
    """Builder for ServiceContext."""

    def __init__(self):
        self._repository = None
        self._config = None
        self._logger = None

    def with_repository(self, repo: Repository) -> "ContextBuilder":
        self._repository = repo
        return self

    def with_config(self, config: Config) -> "ContextBuilder":
        self._config = config
        return self

    def with_logger(self, logger: Logger) -> "ContextBuilder":
        self._logger = logger
        return self

    def build(self) -> ServiceContext:
        """Build immutable context."""
        return ServiceContext(
            repository=self._repository,
            config=self._config,
            logger=self._logger,
        )

# Usage
ctx = (
    ContextBuilder()
    .with_repository(InMemoryRepository())
    .with_config(Config.from_env())
    .with_logger(setup_logger())
    .build()
)
```

## Testing with Contexts

### Fixture Pattern

```python
@pytest.fixture
def test_context():
    """Standard test context."""
    return ServiceContext(
        repository=FakeRepository(),
        config=Config(debug=True),
        logger=NullLogger(),
    )

def test_my_service(test_context):
    """Test using fixture."""
    result = my_service(test_context, "param")
    assert result is not None
```

### Fixture with Data

```python
@pytest.fixture
def context_with_users(test_context):
    """Context pre-populated with users."""
    # Add test data
    test_context.repository.save(User(email="test1@example.com", name="User 1"))
    test_context.repository.save(User(email="test2@example.com", name="User 2"))
    return test_context

def test_list_users(context_with_users):
    """Test with existing data."""
    users = list_users(context_with_users)
    assert len(users) == 2
```

### Parametrized Contexts

```python
@pytest.fixture(params=["memory", "sqlite"])
def repository(request):
    """Parametrized repository fixture."""
    if request.param == "memory":
        return InMemoryRepository()
    elif request.param == "sqlite":
        return SQLiteRepository(":memory:")

@pytest.fixture
def context(repository):
    """Context with parametrized repository."""
    return ServiceContext(
        repository=repository,
        config=Config(),
        logger=NullLogger(),
    )

def test_with_multiple_repositories(context):
    """Test runs with both repository types."""
    # Test logic...
```

## Best Practices

### 1. Frozen Dataclass

Always use `frozen=True`:

```python
@dataclass(frozen=True)  # ✅ Immutable
class ServiceContext:
    ...

@dataclass  # ❌ Mutable
class ServiceContext:
    ...
```

### 2. Protocol Types

Use Protocol types for flexibility:

```python
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]  # ✅ Protocol
    # NOT: repository: PostgresRepository  # ❌ Concrete class
```

### 3. Type Hints

Always add type hints:

```python
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]  # ✅ Type hint
    config: Config  # ✅ Type hint
```

### 4. Docstrings

Document purpose and fields:

```python
@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependencies.

    Contains all external dependencies needed by service functions.
    Frozen to ensure thread safety and predictable behavior.

    Attributes:
        repository: Entity persistence
        config: Application configuration
        logger: Structured logger
    """
    ...
```

## Common Mistakes

### 1. Mutating Context

```python
# ❌ Will fail - context is frozen
ctx.repository = different_repo

# ✅ Create new context
ctx = replace(ctx, repository=different_repo)
```

### 2. Circular Imports

```python
# ❌ Circular dependency
# services/context.py imports adapters/repository.py
# adapters/repository.py imports services/context.py

# ✅ Use protocols
# services/context.py imports protocols/repository.py
# adapters/repository.py implements protocol (no import of context)
```

### 3. Too Many Dependencies

```python
# ❌ Too many dependencies (hard to test)
@dataclass(frozen=True)
class ServiceContext:
    repo1: Repository1
    repo2: Repository2
    # ... 20 more fields ...

# ✅ Split into multiple contexts or nested contexts
```

## Related Documentation

- [Dependency Injection Architecture](../architecture/dependency-injection.md) - DI patterns
- [ADR 005: Context Objects](../decisions/005-context-objects.md) - Design rationale
- [Protocols Reference](protocols.md) - Protocol definitions
- [Testing Guide](../guides/testing-guide.md) - Testing with contexts

## Sources

- [Python Dataclasses](https://docs.python.org/3/library/dataclasses.html)
- [Dependency Injection in Python](https://www.glukhov.org/post/2025/12/dependency-injection-in-python/)
- [Cosmic Python: DI](https://www.cosmicpython.com/book/chapter_13_dependency_injection.html)
