---
title: "ADR 005: Context Objects for Dependency Injection"
status: stable
owners: [architecture-team]
---

# ADR 005: Context Objects for Dependency Injection

## Status

Accepted

## Context

Service functions need access to external dependencies (repositories, APIs, configuration). We need a pattern to:

1. Make dependencies explicit
2. Enable testability (inject fakes/mocks)
3. Maintain type safety
4. Keep code clean and Pythonic
5. Support multiple implementations (production, test, development)

## Decision

Use **frozen dataclasses** as context objects containing all dependencies.

Service functions accept context as their first parameter.

## Rationale

### Explicit Dependencies

Context makes all dependencies visible in one place:

```python
from dataclasses import dataclass
from typing import Protocol

@dataclass(frozen=True)
class ServiceContext:
    """All service dependencies in one immutable container.

    Looking at this, I immediately know what the service layer needs.
    """
    user_repository: Repository[User]
    order_repository: Repository[Order]
    email_service: EmailService
    payment_gateway: PaymentGateway
    config: Config
    logger: Logger
```

**One place to see everything** the service layer depends on.

No hidden globals, no scattered imports, no magic.

### Type Safety

Dataclasses with type hints provide:

- **IDE autocomplete**: `ctx.user_repository.` shows all methods
- **Type checker validation**: mypy/pyright verify protocol compliance
- **Runtime type hints**: Available for introspection

```python
def create_user(ctx: ServiceContext, email: str, name: str) -> User:
    # IDE knows ctx.user_repository is Repository[User]
    ctx.user_repository.save(user)  # ✅ Autocomplete works

    # Type checker catches errors
    ctx.user_repository.invalid_method()  # ❌ Type error
    ctx.invalid_field  # ❌ Type error
```

**Full tooling support** - errors caught before runtime.

### Immutability

`frozen=True` prevents accidental mutation:

```python
ctx = ServiceContext(
    user_repository=PostgresRepository(),
    email_service=SMTPService(),
    config=Config(),
    logger=Logger(),
)

# ❌ FrozenInstanceError - can't modify
ctx.user_repository = FakeRepository()
ctx.config = different_config
```

**Immutability ensures**:
- Thread safety (safe to share across threads)
- Predictable behavior (context never changes)
- No hidden state mutations
- Easier to reason about code

### Testing

Easy to construct partial contexts for tests:

```python
import pytest
from starter.services.context import ServiceContext
from tests.fakes import FakeRepository, FakeEmailService

@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with only what tests need."""
    return ServiceContext(
        user_repository=FakeRepository(),
        order_repository=FakeRepository(),
        email_service=FakeEmailService(),
        payment_gateway=FakePaymentGateway(),
        config=Config(debug=True, environment="test"),
        logger=NullLogger(),  # No logging in tests
    )

def test_create_user(test_context):
    """Test with fake dependencies."""
    user = create_user(test_context, "test@example.com", "Test User")

    # Easy to verify behavior
    assert user.email == "test@example.com"
    assert test_context.user_repository.get(user.id) is not None
```

**No framework** needed - just build a dataclass.

### Flexibility with Protocols

Context fields use Protocol types for maximum flexibility:

```python
@dataclass(frozen=True)
class ServiceContext:
    user_repository: Repository[User]  # Protocol, not concrete class
    email_service: EmailService        # Protocol, not concrete class
    config: Config                     # Concrete (simple dataclass)
    logger: Logger                     # Protocol
```

**Any implementation** satisfying the protocol works:

- **Production**: `PostgresRepository`, `SMTPEmailService`
- **Testing**: `InMemoryRepository`, `FakeEmailService`
- **Development**: `SQLiteRepository`, `ConsoleEmailService`
- **Integration**: Real repositories with test databases

**Swap implementations** without changing service code.

## Alternatives Considered

### Global Configuration

**Pattern**:
```python
# config.py - module-level globals
REPOSITORY = None
EMAIL_SERVICE = None
CONFIG = None

def init_services(repo, email, config):
    """Initialize global dependencies."""
    global REPOSITORY, EMAIL_SERVICE, CONFIG
    REPOSITORY = repo
    EMAIL_SERVICE = email
    CONFIG = config

# services.py
from config import REPOSITORY, EMAIL_SERVICE

def create_user(email: str, name: str) -> User:
    """Uses global dependencies."""
    user = User(email=email, name=name)
    REPOSITORY.save(user)
    EMAIL_SERVICE.send_welcome(email)
    return user
```

**Pros**:
- Simple for small projects
- No context passing needed
- Familiar pattern

**Cons**:
- **Global state** (major anti-pattern)
- **Hard to test** (can't easily inject fakes)
- **Not thread-safe** (globals can be mutated)
- **Hidden dependencies** (not in function signature)
- **Implicit initialization** (must call init before use)
- **Doesn't scale** to larger projects

**Why not chosen**: Global state makes testing difficult and code harder to reason about.

### Function Parameters

**Pattern**:
```python
def create_user(
    email: str,
    name: str,
    # All dependencies as parameters
    repository: Repository,
    email_service: EmailService,
    config: Config,
    logger: Logger,
) -> User:
    """All dependencies as individual parameters."""
    user = User(email=email, name=name)
    repository.save(user)
    logger.info(f"Created: {user.id}")
    return user
```

**Pros**:
- Very explicit
- No context object needed

**Cons**:
- **Long parameter lists** (hard to read)
- **Repetitive** (every function repeats same dependencies)
- **Hard to refactor** (adding dependency touches all functions)
- **Doesn't scale** (10+ dependencies = unreadable)

**Why not chosen**: Doesn't scale. Context groups dependencies logically.

### Dependency Injection Framework

**Pattern using `dependency-injector`**:
```python
from dependency_injector import containers, providers

class Container(containers.DeclarativeContainer):
    config = providers.Configuration()

    repository = providers.Singleton(
        PostgresRepository,
        connection_string=config.db_url,
    )

    email_service = providers.Factory(
        SMTPEmailService,
        smtp_url=config.smtp_url,
    )

    user_service = providers.Factory(
        UserService,
        repository=repository,
        email_service=email_service,
    )
```

**Pros**:
- Sophisticated features (scopes, lifecycle, factories)
- Automatic dependency resolution
- Popular in enterprise contexts
- Well-documented

**Cons**:
- **Additional dependency** (external library)
- **Learning curve** (complex API)
- **Magic/implicit behavior** (auto-wiring)
- **Overkill** for most Python projects
- **Debugging harder** (magic makes issues opaque)

**Why not chosen**: Simple dataclass context is sufficient, transparent, and doesn't require external dependencies.

### Named Tuple

**Pattern**:
```python
from collections import namedtuple

ServiceContext = namedtuple('ServiceContext', [
    'repository',
    'email_service',
    'config',
    'logger',
])

# Usage
ctx = ServiceContext(
    repository=repo,
    email_service=email,
    config=cfg,
    logger=log,
)
```

**Pros**:
- Immutable
- Lightweight
- Standard library

**Cons**:
- **No type hints** (pre-Python 3.6 construct)
- **Less clear** than dataclass
- **No default values**
- **Deprecated** in favor of dataclasses

**Why not chosen**: Dataclasses are the modern, type-safe successor to namedtuples.

### Simple Dict

**Pattern**:
```python
ctx = {
    "repository": PostgresRepository(),
    "email_service": SMTPService(),
    "config": Config(),
}

# Usage
def create_user(ctx: dict, email: str) -> User:
    repository = ctx["repository"]
    ...
```

**Pros**:
- Very simple
- No class needed
- Flexible

**Cons**:
- **No type safety** (dict[str, Any])
- **No IDE support** (no autocomplete)
- **Error-prone** (typos in keys)
- **No validation** (can have wrong values)

**Why not chosen**: Type safety and IDE support are valuable.

## Consequences

### Positive

1. **Clear dependencies**: One place to see all service needs
2. **Testable**: Easy to construct test contexts with fakes
3. **Type-safe**: Full IDE and type checker support
4. **Immutable**: Thread-safe, predictable (`frozen=True`)
5. **Pythonic**: Modern Python idiom (dataclasses introduced in 3.7)
6. **Flexible**: Protocol types allow any implementation
7. **No framework**: Just standard library

### Negative

1. **Context passing**: Every service function needs context parameter
2. **Context bloat**: Can grow large with many dependencies
3. **Coupling**: All services see all dependencies (even if unused)
4. **Refactoring**: Changing context fields affects many functions

### Mitigations

#### 1. Split Contexts

For large applications, create domain-specific contexts:

```python
@dataclass(frozen=True)
class UserServiceContext:
    """Context for user-related services only."""
    user_repository: Repository[User]
    email_service: EmailService
    auth_service: AuthService
    config: Config
    logger: Logger

@dataclass(frozen=True)
class OrderServiceContext:
    """Context for order-related services only."""
    order_repository: Repository[Order]
    payment_gateway: PaymentGateway
    inventory_service: InventoryService
    config: Config
    logger: Logger
```

**Benefits**:
- Smaller, focused contexts
- Services only see dependencies they need
- Easier to understand

#### 2. Context Composition

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
    """Main service context with nested contexts."""
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

#### 3. Selective Parameters

When function needs only one dependency, pass it directly:

```python
def simple_operation(repository: Repository[User], user_id: str) -> User:
    """Only needs repository, not full context."""
    return repository.get(user_id)

# Can extract from context
user = simple_operation(ctx.user_repository, "user-123")
```

## Implementation Guidelines

### Context Definition

```python
from dataclasses import dataclass
from typing import Protocol

@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependency container.

    Contains all external dependencies needed by service functions.
    Immutable to ensure thread safety and predictable behavior.

    Attributes:
        repository: Entity persistence (Protocol)
        external_api: Third-party API client (Protocol)
        config: Application configuration
        logger: Structured logger (Protocol)
    """
    repository: Repository[Entity]  # Use Protocol types
    external_api: ExternalAPI        # Use Protocol types
    config: Config                   # Concrete type OK if simple
    logger: Logger                   # Use Protocol types
```

**Guidelines**:
- Use `frozen=True` for immutability
- Type hint all fields
- Use Protocol types for adapters
- Add docstring explaining purpose
- Group related dependencies

### Context Construction (Production)

**CLI layer** builds context with real adapters:

```python
# src/starter/cli/main.py

import os
from starter.services.context import ServiceContext
from starter.adapters.repository import PostgresRepository
from starter.adapters.external_api import HTTPExternalAPI

def get_context() -> ServiceContext:
    """Build production context with real adapters.

    Reads configuration from environment variables.
    Constructs real adapter instances.

    Returns:
        ServiceContext configured for production
    """
    return ServiceContext(
        repository=PostgresRepository(
            connection_string=os.getenv("DATABASE_URL"),
            pool_size=int(os.getenv("DB_POOL_SIZE", "10")),
        ),
        external_api=HTTPExternalAPI(
            base_url=os.getenv("API_BASE_URL"),
            api_key=os.getenv("API_KEY"),
            timeout=int(os.getenv("API_TIMEOUT", "30")),
        ),
        config=Config.from_env(),
        logger=setup_logger(level=os.getenv("LOG_LEVEL", "INFO")),
    )
```

### Context Construction (Testing)

**Test fixtures** build context with fakes:

```python
# tests/conftest.py

import pytest
from starter.services.context import ServiceContext
from tests.fakes.fake_repository import FakeRepository
from tests.fakes.fake_external_api import FakeExternalAPI

@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with fake adapters.

    Uses in-memory fakes for fast, isolated testing.

    Returns:
        ServiceContext configured for testing
    """
    return ServiceContext(
        repository=FakeRepository(),
        external_api=FakeExternalAPI(),
        config=Config(
            debug=True,
            environment="test",
        ),
        logger=NullLogger(),  # No logging in tests
    )

@pytest.fixture
def test_context_with_data(test_context) -> ServiceContext:
    """Test context pre-populated with data."""
    # Pre-populate repository
    test_context.repository.save(sample_user())
    test_context.repository.save(sample_order())
    return test_context
```

### Context Construction (Integration Tests)

**Integration tests** use real adapters with test infrastructure:

```python
@pytest.fixture
def integration_context(test_database) -> ServiceContext:
    """Build context with real database (test instance).

    Args:
        test_database: Fixture providing test database connection

    Returns:
        ServiceContext with real database, fake external services
    """
    return ServiceContext(
        repository=PostgresRepository(
            connection_string=test_database.url,
        ),
        external_api=FakeExternalAPI(),  # Still use fake for external
        config=Config(environment="test"),
        logger=setup_logger(level="DEBUG"),
    )

def test_create_user_integration(integration_context):
    """Integration test with real database."""
    user = create_user(
        integration_context,
        email="integration@example.com",
        name="Integration Test",
    )

    # Verify in real database
    retrieved = integration_context.repository.get(user.id)
    assert retrieved is not None
    assert retrieved.email == "integration@example.com"
```

### Service Function Pattern

```python
def my_use_case(ctx: ServiceContext, param: str, value: int) -> Result:
    """Use case function.

    Args:
        ctx: Service context with dependencies
        param: Business parameter
        value: Business parameter

    Returns:
        Operation result

    Raises:
        ValidationError: If business rules violated
    """
    # Validate
    if value < 0:
        raise ValidationError("Value must be non-negative")

    # Domain logic
    entity = create_entity(param, value)

    # Access dependencies via ctx
    ctx.repository.save(entity)
    ctx.logger.info(f"Created: {entity.id}")

    return Result(entity=entity)
```

## Advanced Patterns

### Context Factory

For complex context construction:

```python
class ContextFactory:
    """Factory for building contexts in different environments."""

    @staticmethod
    def production() -> ServiceContext:
        """Build production context."""
        return ServiceContext(
            repository=PostgresRepository.from_env(),
            external_api=HTTPExternalAPI.from_env(),
            config=Config.from_env(),
            logger=setup_logger(),
        )

    @staticmethod
    def test() -> ServiceContext:
        """Build test context."""
        return ServiceContext(
            repository=FakeRepository(),
            external_api=FakeExternalAPI(),
            config=Config(debug=True),
            logger=NullLogger(),
        )

    @staticmethod
    def development() -> ServiceContext:
        """Build development context."""
        return ServiceContext(
            repository=SQLiteRepository("dev.db"),
            external_api=ConsoleExternalAPI(),  # Prints to console
            config=Config(debug=True),
            logger=setup_logger(level="DEBUG"),
        )

# Usage
ctx = ContextFactory.production()
```

### Partial Context Updates

For testing specific scenarios:

```python
from dataclasses import replace

@pytest.fixture
def test_context() -> ServiceContext:
    return ServiceContext(...)

def test_with_custom_config(test_context):
    """Test with custom configuration."""
    # Replace just the config field
    ctx = replace(test_context, config=Config(feature_flag=True))

    result = my_service(ctx, param="test")
    assert result.feature_used is True
```

## Related Decisions

- [ADR 004: Functional Services](004-functional-services.md) - Service functions use context
- [ADR 003: Protocol DI](003-protocol-di.md) - Protocol interfaces in context
- [Dependency Injection Architecture](../architecture/dependency-injection.md) - DI deep dive

## Sources

- [Dependency Injection: a Python Way](https://www.glukhov.org/post/2025/12/dependency-injection-in-python/) - DI patterns
- [Python Dataclasses](https://docs.python.org/3/library/dataclasses.html) - Dataclass documentation
- [Cosmic Python: Dependency Injection](https://www.cosmicpython.com/book/chapter_13_dependency_injection.html) - DI in services
- [PEP 557 -- Data Classes](https://peps.python.org/pep-0557/) - Dataclass specification
- [Effective Python: Item 37](https://effectivepython.com/) - Compose classes instead of nesting
