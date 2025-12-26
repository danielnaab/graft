---
title: "ADR 004: Functional Service Layer"
status: stable
owners: [architecture-team]
---

# ADR 004: Functional Service Layer

## Status

Accepted

## Context

The service layer orchestrates use cases and workflows. Two common patterns exist:

1. **Service classes** with injected dependencies
2. **Service functions** taking context objects

We need to choose an approach that is:
- Pythonic and idiomatic
- Easy to test
- Simple to understand
- Flexible for composition

## Decision

Use **service functions** that take **context objects** as the first parameter.

Services are pure functions, not classes with instance state.

## Rationale

### Pythonic Philosophy

Python embraces multiple paradigms. For stateless operations, functions are more idiomatic than classes:

**More Pythonic** (functions for stateless operations):
```python
def calculate_total(items: list[Item]) -> Decimal:
    """Calculate total price of items."""
    return sum(item.price for item in items)

def validate_email(email: str) -> bool:
    """Validate email format."""
    return "@" in email and "." in email.split("@")[1]
```

**Less Pythonic** (over-engineering with classes):
```python
class TotalCalculator:
    """Unnecessary class for stateless operation."""

    def calculate(self, items: list[Item]) -> Decimal:
        return sum(item.price for item in items)

class EmailValidator:
    """Unnecessary class for stateless operation."""

    def validate(self, email: str) -> bool:
        return "@" in email and "." in email.split("@")[1]
```

**Service layer operations are fundamentally stateless workflows** - functions are the natural fit.

### Testability

Pure functions with explicit dependencies are trivial to test:

```python
def test_create_user():
    """Test create_user with fake dependencies."""
    # Arrange - build test context
    fake_repo = FakeRepository()
    fake_logger = FakeLogger()
    ctx = ServiceContext(
        repository=fake_repo,
        email_service=FakeEmailService(),
        config=Config(debug=True),
        logger=fake_logger,
    )

    # Act - call the function
    user = create_user(ctx, email="test@example.com", name="Test User")

    # Assert - verify behavior
    assert user.email == "test@example.com"
    assert user.name == "Test User"
    assert fake_repo.get(user.id) == user
    assert len(fake_logger.messages) == 1
```

**No need to**:
- Instantiate service classes
- Manage instance state
- Mock methods
- Deal with class lifecycle
- Worry about __init__ side effects

**Simpler** - just build context and call function.

### Composition and Reuse

Functions compose naturally using standard function composition:

```python
def complex_workflow(ctx: ServiceContext, data: dict) -> WorkflowResult:
    """Complex workflow composing multiple service functions."""
    # Create user
    user = create_user(
        ctx,
        email=data["email"],
        name=data["name"],
    )

    # Create order for user
    order = create_order(
        ctx,
        user_id=user.id,
        items=data["items"],
    )

    # Send confirmation email
    confirmation_sent = send_order_confirmation(
        ctx,
        order_id=order.id,
    )

    # Return combined result
    return WorkflowResult(
        user=user,
        order=order,
        confirmation_sent=confirmation_sent,
    )
```

**Functions can be**:
- Composed into workflows
- Passed as parameters (callbacks)
- Returned from other functions
- Used with decorators
- Partially applied with `functools.partial`
- Piped/chained with libraries like `toolz`

### No Hidden State

Everything needed is in the context - no instance variables to track:

```python
def process_payment(ctx: ServiceContext, order_id: str, amount: Decimal) -> Payment:
    """Process payment - all dependencies explicit.

    Everything needed is either:
    1. Passed as parameter (order_id, amount)
    2. In context (ctx.payment_gateway, ctx.repository)
    3. Created locally (payment, order)

    No hidden state!
    """
    # Validate
    if amount <= 0:
        raise ValueError("Amount must be positive")

    # Get order
    order = ctx.repository.get_order(order_id)
    if not order:
        raise NotFoundError(f"Order {order_id} not found")

    # Process payment
    payment = ctx.payment_gateway.charge(
        amount=amount,
        currency="USD",
        description=f"Order {order_id}",
    )

    # Update order
    order.mark_paid(payment.transaction_id)
    ctx.repository.save_order(order)

    # Log
    ctx.logger.info(f"Processed payment {payment.id} for order {order_id}")

    return payment
```

**With classes**, you might have:
```python
class PaymentService:
    def __init__(self, repo, gateway, logger):
        self._repo = repo
        self._gateway = gateway
        self._logger = logger
        self._current_order = None      # ❌ Hidden mutable state
        self._last_payment = None       # ❌ Hidden mutable state
        self._processing_count = 0      # ❌ Hidden mutable state

    def process(self, order_id: str, amount: Decimal) -> Payment:
        self._current_order = self._repo.get_order(order_id)  # ❌ Mutation
        self._processing_count += 1  # ❌ Mutation
        # ... more mutations ...
```

### Easier Refactoring

Moving functions between modules is simpler than reorganizing class hierarchies:

```python
# Easy - just move function to different module
# From: services/order_services.py
def validate_order(ctx: ServiceContext, order: Order) -> bool:
    """Validate order."""
    ...

# To: services/validation.py
def validate_order(ctx: ServiceContext, order: Order) -> bool:
    """Validate order."""
    ...

# Update imports and done!
```

**With classes**:
- Might have inheritance hierarchies
- Might have circular dependencies between classes
- Might have tight coupling
- Harder to split or reorganize
- Need to update all instantiation points

### Explicit Dependencies

Context parameter makes **all dependencies visible**:

```python
def create_user_and_send_welcome(
    ctx: ServiceContext,
    email: str,
    name: str,
) -> tuple[User, bool]:
    """Create user and send welcome email.

    Looking at ctx: ServiceContext, I immediately know this function needs:
    - repository (for persistence)
    - email_service (for sending email)
    - config (for application settings)
    - logger (for logging)

    All dependencies are explicit and visible in signature!
    """
    # Validate
    if not email or "@" not in email:
        raise ValidationError("Invalid email")

    # Create user
    user = User(email=email, name=name)

    # Persist
    ctx.repository.save_user(user)

    # Send email
    sent = ctx.email_service.send_welcome(email, name)

    # Log
    ctx.logger.info(f"Created user {user.id}, email sent: {sent}")

    return user, sent
```

**With service classes**, dependencies are in `__init__`:
```python
class UserService:
    def __init__(self, repo, email_service, config, logger):
        # Dependencies hidden from method signatures
        self._repo = repo
        self._email_service = email_service
        self._config = config
        self._logger = logger

    def create_and_send_welcome(self, email, name):
        # What dependencies does this use? Must read implementation!
        ...
```

## Alternatives Considered

### Service Classes with Dependency Injection

**Pattern**:
```python
class UserService:
    """Service class with dependency injection."""

    def __init__(self, repository: Repository, config: Config, logger: Logger):
        self._repository = repository
        self._config = config
        self._logger = logger

    def create_user(self, email: str, name: str) -> User:
        """Create new user."""
        user = User(email=email, name=name)
        self._repository.save(user)
        self._logger.info(f"Created user: {user.id}")
        return user

    def delete_user(self, user_id: str) -> bool:
        """Delete user by ID."""
        result = self._repository.delete(user_id)
        if result:
            self._logger.info(f"Deleted user: {user_id}")
        return result
```

**Pros**:
- Familiar to developers from OOP backgrounds
- Common in enterprise frameworks (Spring, NestJS, Django class-based views)
- Supports method polymorphism (override in subclasses)
- Can share state across methods (if needed)

**Cons**:
- More boilerplate (`__init__`, `self._` everywhere)
- Instance state can lead to bugs (mutability)
- Less composable than functions
- Not particularly Pythonic for stateless operations
- Harder to test (need to instantiate class)

**Why not chosen**: Functions provide same benefits with less complexity. For **stateless workflows**, classes add unnecessary overhead.

### Module-Level Functions with Global Dependencies

**Pattern**:
```python
# config.py - module-level globals
_repository = None
_logger = None

def init_services(repository, logger):
    """Initialize global dependencies."""
    global _repository, _logger
    _repository = repository
    _logger = logger

# services.py
from config import _repository, _logger

def create_user(email: str, name: str) -> User:
    """Create user using global dependencies."""
    user = User(email=email, name=name)
    _repository.save(user)
    _logger.info(f"Created: {user.id}")
    return user
```

**Pros**:
- Very simple for small projects
- Minimal boilerplate
- No context passing needed

**Cons**:
- Global state (major anti-pattern)
- Hard to test (can't inject fakes easily)
- Not thread-safe
- Hidden dependencies (not in signature)
- Doesn't scale
- Implicit initialization order

**Why not chosen**: Global state is an anti-pattern. Testing and explicitness are critical.

### Dataclass Services (Frozen)

**Pattern**:
```python
@dataclass(frozen=True)
class UserService:
    """Frozen dataclass service."""
    repository: Repository
    config: Config
    logger: Logger

    def create_user(self, email: str, name: str) -> User:
        user = User(email=email, name=name)
        self.repository.save(user)
        self.logger.info(f"Created: {user.id}")
        return user
```

**Pros**:
- Immutable service objects (`frozen=True`)
- Clean dependency declaration
- Type-safe
- Less boilerplate than regular classes

**Cons**:
- Still class overhead for stateless operations
- Methods vs functions (minor aesthetic difference)
- Less composable than pure functions

**Why not chosen**: Very close to our choice! But **functions are slightly simpler** for stateless operations. If we needed stateful services, frozen dataclasses would be great.

### Higher-Order Functions (Currying)

**Pattern**:
```python
def make_create_user(ctx: ServiceContext):
    """Return create_user function with ctx bound."""
    def create_user(email: str, name: str) -> User:
        user = User(email=email, name=name)
        ctx.repository.save(user)
        ctx.logger.info(f"Created: {user.id}")
        return user
    return create_user

# Usage
create_user = make_create_user(production_ctx)
user = create_user("test@example.com", "Test")
```

**Pros**:
- Very functional programming style
- Context bound once, used many times
- Composition-friendly

**Cons**:
- More complex conceptually
- Extra indirection
- Harder for beginners to understand
- Debugging more difficult

**Why not chosen**: Context-as-first-parameter is simpler and more explicit.

## Consequences

### Positive

1. **Less boilerplate**: No class definitions or `__init__` methods
2. **Easier testing**: Pure functions with explicit inputs
3. **Better composition**: Functions naturally chain and compose
4. **More Pythonic**: Embraces functional programming strengths
5. **Clearer code**: Dependencies visible in function signature
6. **Simpler refactoring**: Move functions freely between modules
7. **No hidden state**: All data flow is explicit

### Negative

1. **Unfamiliar pattern**: Less common in enterprise Java/C# world
2. **Context passing**: Every service function needs context parameter
3. **No method polymorphism**: Can't override functions in subclasses
4. **Refactoring context**: Changing `ServiceContext` fields affects all functions

### Mitigations

1. **Documentation**: Clear examples and architectural guides
2. **Context organization**: Group related dependencies, use dataclass composition
3. **Polymorphism alternative**: Use Protocol types in context for flexibility
4. **Tooling**: Type hints and IDE support catch context changes
5. **Convention**: Consistent pattern across all services

## Implementation Notes

### Service Function Signature Pattern

```python
def service_function_name(
    ctx: ServiceContext,
    # Business parameters follow context
    business_param1: str,
    business_param2: int,
    optional_param: bool = False,
) -> ResultType:
    """One-line summary.

    Detailed description of what this function does.

    Args:
        ctx: Service context with dependencies
        business_param1: Description
        business_param2: Description
        optional_param: Description (default: False)

    Returns:
        Description of return value

    Raises:
        ValidationError: When business rules violated
        NotFoundError: When entity not found
    """
    # 1. Validate inputs
    # 2. Execute domain logic
    # 3. Call adapters via context
    # 4. Return results or raise exceptions
```

### Context Structure

Use frozen dataclass:

```python
from dataclasses import dataclass
from typing import Protocol

@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependency container.

    Immutable to ensure thread safety and predictable behavior.
    All fields use Protocol types for maximum flexibility.
    """
    repository: Repository[Entity]
    external_api: ExternalAPI
    payment_gateway: PaymentGateway
    email_service: EmailService
    config: Config
    logger: Logger
```

### CLI Integration

Wire context in CLI layer:

```python
# cli/main.py

import typer
from starter.services.context import ServiceContext
from starter.adapters import PostgresRepository, HTTPExternalAPI
from starter.services import create_user

app = typer.Typer()

def get_context() -> ServiceContext:
    """Build production context with real adapters."""
    return ServiceContext(
        repository=PostgresRepository(db_url=os.getenv("DB_URL")),
        external_api=HTTPExternalAPI(api_url=os.getenv("API_URL")),
        payment_gateway=StripeGateway(api_key=os.getenv("STRIPE_KEY")),
        email_service=SMTPEmailService(smtp_url=os.getenv("SMTP_URL")),
        config=Config.from_env(),
        logger=setup_logger(),
    )

@app.command()
def create(email: str, name: str) -> None:
    """Create new user."""
    ctx = get_context()  # Build context
    user = create_user(ctx, email=email, name=name)  # Call service
    typer.echo(f"Created user: {user.id}")  # Display result
```

### Testing Pattern

```python
# tests/conftest.py

import pytest
from starter.services.context import ServiceContext
from tests.fakes import FakeRepository, FakeEmailService

@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with fakes."""
    return ServiceContext(
        repository=FakeRepository(),
        external_api=FakeExternalAPI(),
        payment_gateway=FakePaymentGateway(),
        email_service=FakeEmailService(),
        config=Config(debug=True, environment="test"),
        logger=NullLogger(),
    )

# tests/test_services.py

def test_create_user(test_context):
    """Test create_user with fake context."""
    user = create_user(test_context, email="test@example.com", name="Test")

    assert user.email == "test@example.com"
    assert test_context.repository.get_user(user.id) == user
```

## When to Use Classes

Service functions are preferred for **stateless operations**, but classes are appropriate for:

### 1. Stateful Operations

When maintaining state across operations:

```python
class DatabaseConnectionPool:
    """Maintains pool of database connections (stateful)."""

    def __init__(self, max_connections: int):
        self._pool: list[Connection] = []
        self._max_connections = max_connections
        self._active_count = 0

    def acquire(self) -> Connection:
        """Get connection from pool."""
        # Manages state
        ...

    def release(self, conn: Connection) -> None:
        """Return connection to pool."""
        # Updates state
        ...
```

### 2. Complex Lifecycle

When setup/teardown is complex:

```python
class BackgroundJobWorker:
    """Worker with complex lifecycle."""

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
# FastAPI class-based views
class UserAPI:
    def __init__(self, context: ServiceContext):
        self._ctx = context

    async def create(self, user_data: UserCreate):
        """Delegate to service function."""
        return create_user(self._ctx, **user_data.dict())
```

**For most service layer use cases, functions + context is simpler and more Pythonic.**

## Related Decisions

- [ADR 005: Context Objects](005-context-objects.md) - Context pattern details
- [ADR 003: Protocol DI](003-protocol-di.md) - Protocol-based dependency injection
- [Functional Services Architecture](../architecture/functional-services.md) - Detailed guide

## Sources

- [Cosmic Python: Service Layer](https://www.cosmicpython.com/book/chapter_04_service_layer.html) - Service layer patterns
- [Functional Python Programming](https://www.oreilly.com/library/view/functional-python-programming/9781492048633/) - Functional patterns
- [Python Design Patterns for Clean Architecture](https://www.glukhov.org/post/2025/11/python-design-patterns-for-clean-architecture/) - Architecture patterns
- [Guido van Rossum on Functions vs Classes](https://www.artima.com/weblogs/viewpost.jsp?thread=240808) - Python creator's perspective
- [PEP 20 -- The Zen of Python](https://peps.python.org/pep-0020/) - Python philosophy
