---
title: Functional Service Layer
status: stable
owners: [architecture-team]
---

# Functional Service Layer

This template uses **service functions** instead of service classes, embracing a functional programming approach for the service layer.

## Pattern Overview

### Traditional Service Class (NOT used here)

```python
class ExampleService:
    """Traditional service class with dependency injection."""

    def __init__(self, repository: Repository, config: Config, logger: Logger):
        self._repository = repository
        self._config = config
        self._logger = logger

    def create(self, name: str, value: int) -> Entity:
        """Create example entity."""
        entity = Entity(name=name, value=value)
        self._repository.save(entity)
        self._logger.info(f"Created: {entity.id}")
        return entity

    def list_all(self) -> list[Entity]:
        """List all entities."""
        return self._repository.list_all()
```

### Functional Service (USED in this template)

```python
@dataclass(frozen=True)
class ServiceContext:
    """Immutable context containing all dependencies."""
    repository: Repository[Entity]
    config: Config
    logger: Logger

def create_example(ctx: ServiceContext, name: str, value: int) -> Entity:
    """Create example entity.

    Pure function - no hidden state, explicit dependencies.

    Args:
        ctx: Service context with dependencies
        name: Entity name
        value: Entity value

    Returns:
        Created entity

    Raises:
        ValueError: If value is negative
    """
    if value < 0:
        raise ValueError("Value must be non-negative")

    entity = Entity(name=name, value=value)
    ctx.repository.save(entity)
    ctx.logger.info(f"Created: {entity.id}")
    return entity

def list_examples(ctx: ServiceContext) -> list[Entity]:
    """List all example entities.

    Args:
        ctx: Service context

    Returns:
        List of all entities
    """
    return ctx.repository.list_all()
```

## Why Functions Over Classes?

### 1. Pythonic Philosophy

Python is a multi-paradigm language that embraces functional programming. For stateless operations, functions are often more natural and idiomatic than classes.

**Pythonic** (functions for stateless operations):
```python
def calculate_total(items: list[Item]) -> Decimal:
    """Calculate total price of items."""
    return sum(item.price for item in items)

def format_currency(amount: Decimal, currency: str = "USD") -> str:
    """Format amount as currency string."""
    return f"{currency} {amount:.2f}"
```

**Less Pythonic** (over-engineering with classes):
```python
class TotalCalculator:
    def calculate(self, items: list[Item]) -> Decimal:
        return sum(item.price for item in items)

class CurrencyFormatter:
    def format(self, amount: Decimal, currency: str = "USD") -> str:
        return f"{currency} {amount:.2f}"
```

Service layer operations are fundamentally **stateless workflows** - functions are the natural fit.

### 2. Testability

Pure functions with explicit dependencies are trivial to test:

```python
def test_create_example():
    """Test create_example with fake dependencies."""
    # Arrange - build test context
    fake_repo = FakeRepository()
    fake_logger = FakeLogger()
    ctx = ServiceContext(
        repository=fake_repo,
        config=Config(debug=True),
        logger=fake_logger,
    )

    # Act - call the function
    entity = create_example(ctx, name="test", value=100)

    # Assert - verify behavior
    assert entity.name == "test"
    assert entity.value == 100
    assert fake_repo.get(entity.id) == entity
    assert len(fake_logger.messages) == 1
```

**No need to**:
- Instantiate service classes
- Manage instance state
- Mock methods
- Deal with class lifecycle

### 3. Composition and Reuse

Functions compose naturally using standard function composition:

```python
def complex_workflow(ctx: ServiceContext, data: dict) -> WorkflowResult:
    """Complex workflow composing multiple service functions."""
    # Create user
    user = create_user(ctx, email=data["email"], name=data["name"])

    # Create order
    order = create_order(ctx, user_id=user.id, items=data["items"])

    # Send confirmation
    send_confirmation(ctx, order_id=order.id)

    # Return combined result
    return WorkflowResult(user=user, order=order)
```

Functions can be:
- Composed into workflows
- Passed as parameters
- Returned from other functions
- Used with decorators
- Partially applied with `functools.partial`

### 4. No Hidden State

Everything needed is in the context - no instance variables to track:

```python
def process_order(ctx: ServiceContext, order_id: str) -> Order:
    """Process order - all dependencies explicit."""
    # No hidden state - all data is:
    # 1. Passed as parameters (order_id)
    # 2. In context (ctx.repository, ctx.payment_gateway)
    # 3. Created locally (order, payment)

    order = ctx.repository.get(order_id)
    if not order:
        raise NotFoundError(f"Order {order_id} not found")

    payment = ctx.payment_gateway.charge(order.total)
    order.mark_paid(payment.transaction_id)
    ctx.repository.save(order)

    return order
```

With classes, you might have:
```python
class OrderService:
    def __init__(self, repo, gateway):
        self._repo = repo
        self._gateway = gateway
        self._current_order = None  # ❌ Hidden mutable state
        self._last_payment = None   # ❌ Hidden mutable state

    def process(self, order_id: str) -> Order:
        self._current_order = self._repo.get(order_id)  # ❌ Mutation
        # ... more mutations ...
```

### 5. Easier Refactoring

Moving functions between modules is simpler than reorganizing class hierarchies:

```python
# Easy - just move function to different module
# From: services/order_services.py
def validate_order(ctx: ServiceContext, order: Order) -> bool: ...

# To: services/validation.py
def validate_order(ctx: ServiceContext, order: Order) -> bool: ...

# Update imports and done
```

With classes:
- Might have inheritance hierarchies
- Might have circular dependencies
- Might have tight coupling between classes
- Harder to split or reorganize

## Context Object Pattern

The **context object** contains all dependencies that services need.

### Definition

```python
from dataclasses import dataclass
from typing import Protocol

@dataclass(frozen=True)
class ServiceContext:
    """Immutable context for service functions.

    Contains all external dependencies needed by service layer.
    Frozen ensures no accidental mutation during execution.

    Attributes:
        repository: Entity persistence
        external_api: Third-party API client
        payment_gateway: Payment processing
        config: Application configuration
        logger: Structured logger
    """
    repository: Repository[Entity]
    external_api: ExternalAPI
    payment_gateway: PaymentGateway
    config: Config
    logger: Logger
```

### Key Attributes

**`frozen=True`**: Makes dataclass immutable after creation
```python
ctx = ServiceContext(...)
ctx.repository = fake_repo  # ❌ FrozenInstanceError
```

**Type hints**: Enable IDE autocomplete and type checking
```python
def my_service(ctx: ServiceContext, param: str) -> Result:
    # IDE knows ctx.repository is Repository[Entity]
    # Type checker verifies protocol compliance
    ctx.repository.save(entity)
```

**Protocol types**: Allow any implementation satisfying the protocol
```python
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]  # Protocol, not concrete class
```

Any implementation works:
- Production: `PostgresRepository`
- Testing: `InMemoryRepository`, `FakeRepository`
- Integration: `SQLiteRepository`

See [Context Objects](../decisions/005-context-objects.md) for the decision rationale.

## Service Function Structure

### Signature Pattern

```python
def service_function_name(
    ctx: ServiceContext,
    # Business parameters follow context
    business_param1: str,
    business_param2: int,
    optional_param: bool = False,
) -> ResultType:
    """One-line summary.

    Longer description if needed explaining what this function does,
    when to use it, and any important considerations.

    Args:
        ctx: Service context with dependencies
        business_param1: Description of first parameter
        business_param2: Description of second parameter
        optional_param: Description of optional parameter (default: False)

    Returns:
        Description of return value

    Raises:
        ValidationError: When business rules are violated
        NotFoundError: When required entity doesn't exist
        RepositoryError: When persistence fails
    """
    # Implementation
```

### Implementation Pattern

1. **Validate inputs** - Check business rules
2. **Execute domain logic** - Create/modify entities
3. **Call adapters** - Persist changes, call external systems
4. **Return results** - Or raise exceptions

### Complete Example

```python
def create_and_notify_user(
    ctx: ServiceContext,
    email: str,
    name: str,
    send_welcome: bool = True,
) -> tuple[User, bool]:
    """Create user and optionally send welcome email.

    Creates a new user entity, persists it, and sends a welcome
    email if requested.

    Args:
        ctx: Service context
        email: User email address (must be valid format)
        name: User full name (must be non-empty)
        send_welcome: Whether to send welcome email (default: True)

    Returns:
        Tuple of (created user, email sent successfully)

    Raises:
        ValidationError: If email format invalid or name empty
        DuplicateUserError: If user with email already exists
    """
    # 1. Validate inputs
    if not email or "@" not in email:
        raise ValidationError("Invalid email address")
    if not name.strip():
        raise ValidationError("Name cannot be empty")

    # Check for duplicates
    existing = ctx.repository.find_by_email(email)
    if existing:
        raise DuplicateUserError(f"User with email {email} already exists")

    # 2. Domain logic
    user = User(email=email, name=name)

    # 3. Adapter calls
    ctx.repository.save(user)
    ctx.logger.info(f"Created user: {user.id}")

    email_sent = False
    if send_welcome:
        email_sent = ctx.email_service.send_welcome(user.email, user.name)
        if email_sent:
            ctx.logger.info(f"Sent welcome email to {user.email}")
        else:
            ctx.logger.warning(f"Failed to send welcome email to {user.email}")

    # 4. Return
    return user, email_sent
```

## Testing Service Functions

### Unit Tests with Fake Context

```python
def test_create_and_notify_user_success():
    """Test successful user creation with welcome email."""
    # Arrange
    fake_repo = FakeRepository()
    fake_email = FakeEmailService()
    fake_logger = FakeLogger()

    ctx = ServiceContext(
        repository=fake_repo,
        email_service=fake_email,
        config=Config(),
        logger=fake_logger,
    )

    # Act
    user, email_sent = create_and_notify_user(
        ctx,
        email="test@example.com",
        name="Test User",
        send_welcome=True,
    )

    # Assert
    assert user.email == "test@example.com"
    assert user.name == "Test User"
    assert fake_repo.get(user.id) == user
    assert email_sent is True
    assert len(fake_email.sent_emails) == 1
    assert fake_email.sent_emails[0]["to"] == "test@example.com"


def test_create_user_invalid_email():
    """Test that invalid email raises ValidationError."""
    ctx = ServiceContext(
        repository=FakeRepository(),
        email_service=FakeEmailService(),
        config=Config(),
        logger=FakeLogger(),
    )

    with pytest.raises(ValidationError, match="Invalid email"):
        create_and_notify_user(ctx, email="invalid", name="Test")


def test_create_user_duplicate_email():
    """Test that duplicate email raises DuplicateUserError."""
    fake_repo = FakeRepository()

    # Pre-populate with existing user
    existing = User(email="test@example.com", name="Existing")
    fake_repo.save(existing)

    ctx = ServiceContext(
        repository=fake_repo,
        email_service=FakeEmailService(),
        config=Config(),
        logger=FakeLogger(),
    )

    with pytest.raises(DuplicateUserError):
        create_and_notify_user(ctx, email="test@example.com", name="New User")
```

### Integration Tests with Real Context

```python
def test_create_user_integration(db_context):
    """Integration test with real database."""
    # db_context fixture provides ServiceContext with real database

    user, email_sent = create_and_notify_user(
        db_context,
        email="integration@example.com",
        name="Integration Test",
    )

    # Verify in real database
    retrieved = db_context.repository.get(user.id)
    assert retrieved is not None
    assert retrieved.email == "integration@example.com"
```

## Organizing Service Functions

### Single File Per Domain Area

Group related operations in domain-specific modules:

```python
# services/user_services.py
"""User-related service functions."""

def create_user(ctx: ServiceContext, email: str, name: str) -> User:
    """Create new user."""
    ...

def update_user(ctx: ServiceContext, user_id: str, **updates) -> User:
    """Update existing user."""
    ...

def delete_user(ctx: ServiceContext, user_id: str) -> bool:
    """Delete user by ID."""
    ...

def authenticate_user(ctx: ServiceContext, email: str, password: str) -> User | None:
    """Authenticate user credentials."""
    ...


# services/order_services.py
"""Order-related service functions."""

def create_order(ctx: ServiceContext, user_id: str, items: list[Item]) -> Order:
    """Create new order."""
    ...

def fulfill_order(ctx: ServiceContext, order_id: str) -> Order:
    """Mark order as fulfilled."""
    ...

def cancel_order(ctx: ServiceContext, order_id: str, reason: str) -> Order:
    """Cancel order with reason."""
    ...
```

### Module Structure

```
services/
├── __init__.py              # Export main functions
├── context.py               # ServiceContext definition
├── user_services.py         # User-related operations
├── order_services.py        # Order-related operations
├── payment_services.py      # Payment operations
└── notification_services.py # Notification operations
```

### Exporting from `__init__.py`

```python
# services/__init__.py
"""Service layer public API."""

from starter.services.context import ServiceContext
from starter.services.user_services import (
    authenticate_user,
    create_user,
    delete_user,
    update_user,
)
from starter.services.order_services import (
    cancel_order,
    create_order,
    fulfill_order,
)

__all__ = [
    "ServiceContext",
    "authenticate_user",
    "create_user",
    "delete_user",
    "update_user",
    "cancel_order",
    "create_order",
    "fulfill_order",
]
```

## When to Use Classes

Service functions are preferred for **stateless operations**, but classes are appropriate for:

### 1. Stateful Operations

When maintaining state across operations:
```python
class DatabaseConnectionPool:
    """Maintains pool of database connections."""

    def __init__(self, max_connections: int):
        self._pool = []
        self._max_connections = max_connections

    def acquire(self) -> Connection:
        # Return connection from pool or create new
        ...

    def release(self, conn: Connection) -> None:
        # Return connection to pool
        ...
```

### 2. Complex Lifecycle

When setup/teardown is complex:
```python
class BackgroundJobWorker:
    """Worker that processes background jobs."""

    def __init__(self, queue: Queue):
        self._queue = queue
        self._running = False
        self._thread = None

    def start(self) -> None:
        """Start worker thread."""
        ...

    def stop(self) -> None:
        """Stop worker gracefully."""
        ...

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *args):
        self.stop()
```

### 3. Framework Requirements

When frameworks expect classes:
```python
# Some frameworks require class-based views/handlers
class UserAPIView:
    """API view for user operations (framework requirement)."""

    def __init__(self, context: ServiceContext):
        self._ctx = context

    async def post(self, request):
        """Delegate to service function."""
        data = await request.json()
        user = create_user(self._ctx, **data)
        return {"user_id": user.id}
```

For most service layer use cases, **functions + context** is simpler and more Pythonic.

## Comparison Summary

| Aspect | Service Classes | Service Functions |
|--------|----------------|-------------------|
| **State** | Instance variables | No state (pure) |
| **Dependencies** | Constructor injection | Context parameter |
| **Testing** | Instantiate + mock | Pass fake context |
| **Composition** | Inheritance, delegation | Function composition |
| **Pythonic** | OOP approach | Functional approach |
| **Boilerplate** | `__init__`, `self` | Minimal |
| **Complexity** | Higher | Lower |
| **When to use** | Stateful, lifecycle | Stateless workflows |

## Related Documentation

- [Context Objects Decision](../decisions/005-context-objects.md) - Why context pattern
- [Service Layer Decision](../decisions/004-functional-services.md) - Why functions
- [Testing Strategy](testing-strategy.md) - Testing approach
- [Dependency Injection](dependency-injection.md) - DI patterns
- [Architecture Overview](overview.md) - High-level architecture

## Sources

- [Architecture Patterns with Python (Cosmic Python)](https://www.cosmicpython.com/book/chapter_04_service_layer.html) - Service layer pattern
- [Functional Python Programming (O'Reilly)](https://www.oreilly.com/library/view/functional-python-programming/9781492048633/) - Functional patterns
- [Python Design Patterns for Clean Architecture](https://www.glukhov.org/post/2025/11/python-design-patterns-for-clean-architecture/) - Architecture patterns
- [Guido van Rossum on Functions vs Classes](https://www.artima.com/weblogs/viewpost.jsp?thread=240808) - Python creator's perspective
- [PEP 20 -- The Zen of Python](https://peps.python.org/pep-0020/) - Python philosophy
