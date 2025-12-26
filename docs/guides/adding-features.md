---
title: Adding Features
status: stable
owners: [documentation-team]
---

# Adding Features

This guide shows you how to extend the Python Starter Template with new features, following the established architectural patterns.

## Overview

Adding a feature typically involves:

1. **Define domain entities** (if needed)
2. **Create protocol interfaces** (for external dependencies)
3. **Write service functions** (business logic)
4. **Implement adapters** (external system integration)
5. **Add CLI commands** (user interface)
6. **Write tests** (verification)

## Example: Adding User Management

Let's walk through adding a complete user management feature.

### Step 1: Define Domain Entity

Create the User entity in `src/starter/domain/entities.py`:

```python
from dataclasses import dataclass, field
from datetime import datetime
from uuid import uuid4

@dataclass
class User:
    """User entity with identity."""
    email: str
    name: str
    id: str = field(default_factory=lambda: str(uuid4()))
    created_at: datetime = field(default_factory=datetime.utcnow)
    is_active: bool = True

    def __post_init__(self):
        """Validate user data."""
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email address")
        if not self.name.strip():
            raise ValueError("Name cannot be empty")

    def deactivate(self) -> None:
        """Deactivate user account."""
        self.is_active = False

    def __eq__(self, other) -> bool:
        """Equality based on ID."""
        if not isinstance(other, User):
            return False
        return self.id == other.id

    def __hash__(self) -> int:
        """Hash based on ID."""
        return hash(self.id)
```

### Step 2: Define Protocol Interface

Create repository protocol in `src/starter/protocols/user_repository.py`:

```python
from typing import Protocol, Optional
from starter.domain.entities import User

class UserRepository(Protocol):
    """Repository interface for User persistence."""

    def save(self, user: User) -> None:
        """Save user to storage."""
        ...

    def get(self, user_id: str) -> Optional[User]:
        """Get user by ID."""
        ...

    def get_by_email(self, email: str) -> Optional[User]:
        """Get user by email."""
        ...

    def list_all(self) -> list[User]:
        """List all users."""
        ...

    def delete(self, user_id: str) -> bool:
        """Delete user by ID."""
        ...
```

### Step 3: Update Service Context

Add UserRepository to `src/starter/services/context.py`:

```python
from dataclasses import dataclass
from starter.protocols.user_repository import UserRepository
from starter.protocols.repository import Repository
from starter.domain.entities import Entity, User

@dataclass(frozen=True)
class ServiceContext:
    """Service layer dependencies."""
    repository: Repository[Entity]
    user_repository: UserRepository  # Add this
    config: Config
    logger: Logger
```

### Step 4: Write Service Functions

Create `src/starter/services/user_services.py`:

```python
from typing import Optional
from starter.domain.entities import User
from starter.services.context import ServiceContext

class DuplicateUserError(Exception):
    """User with email already exists."""
    pass

def create_user(
    ctx: ServiceContext,
    email: str,
    name: str,
) -> User:
    """Create new user.

    Args:
        ctx: Service context
        email: User email address
        name: User full name

    Returns:
        Created user

    Raises:
        ValueError: If email or name invalid
        DuplicateUserError: If user with email exists
    """
    # Check for duplicate
    existing = ctx.user_repository.get_by_email(email)
    if existing:
        raise DuplicateUserError(f"User with email {email} already exists")

    # Create user
    user = User(email=email, name=name)

    # Save
    ctx.user_repository.save(user)
    ctx.logger.info(f"Created user: {user.id}")

    return user

def get_user(ctx: ServiceContext, user_id: str) -> Optional[User]:
    """Get user by ID.

    Args:
        ctx: Service context
        user_id: User ID

    Returns:
        User if found, None otherwise
    """
    return ctx.user_repository.get(user_id)

def list_users(ctx: ServiceContext) -> list[User]:
    """List all users.

    Args:
        ctx: Service context

    Returns:
        List of all users
    """
    return ctx.user_repository.list_all()

def deactivate_user(ctx: ServiceContext, user_id: str) -> Optional[User]:
    """Deactivate user.

    Args:
        ctx: Service context
        user_id: User ID

    Returns:
        Updated user if found, None otherwise
    """
    user = ctx.user_repository.get(user_id)
    if not user:
        return None

    user.deactivate()
    ctx.user_repository.save(user)
    ctx.logger.info(f"Deactivated user: {user.id}")

    return user
```

### Step 5: Implement Adapter

Create `src/starter/adapters/user_repository.py`:

```python
from typing import Optional
from starter.domain.entities import User

class InMemoryUserRepository:
    """In-memory user repository implementation."""

    def __init__(self) -> None:
        self._users: dict[str, User] = {}
        self._email_index: dict[str, str] = {}  # email -> user_id

    def save(self, user: User) -> None:
        """Save user."""
        self._users[user.id] = user
        self._email_index[user.email] = user.id

    def get(self, user_id: str) -> Optional[User]:
        """Get user by ID."""
        return self._users.get(user_id)

    def get_by_email(self, email: str) -> Optional[User]:
        """Get user by email."""
        user_id = self._email_index.get(email)
        if user_id:
            return self._users.get(user_id)
        return None

    def list_all(self) -> list[User]:
        """List all users."""
        return list(self._users.values())

    def delete(self, user_id: str) -> bool:
        """Delete user."""
        user = self._users.get(user_id)
        if user:
            del self._users[user_id]
            del self._email_index[user.email]
            return True
        return False
```

### Step 6: Add CLI Commands

Create `src/starter/cli/commands/user.py`:

```python
import typer
from starter.services.context import ServiceContext
from starter.services import user_services
from starter.adapters.user_repository import InMemoryUserRepository

app = typer.Typer()

def get_user_context() -> ServiceContext:
    """Build context with user repository."""
    from starter.cli.main import get_context
    ctx = get_context()
    # Replace with user repository
    from dataclasses import replace
    return replace(ctx, user_repository=InMemoryUserRepository())

@app.command("create")
def create_user(email: str, name: str) -> None:
    """Create new user."""
    ctx = get_user_context()
    try:
        user = user_services.create_user(ctx, email=email, name=name)
        typer.echo(f"Created user: {user.id}")
        typer.echo(f"Email: {user.email}")
        typer.echo(f"Name: {user.name}")
    except ValueError as e:
        typer.secho(f"Validation error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)
    except user_services.DuplicateUserError as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

@app.command("list")
def list_users() -> None:
    """List all users."""
    ctx = get_user_context()
    users = user_services.list_users(ctx)

    if not users:
        typer.echo("No users found")
        return

    typer.echo(f"Users ({len(users)}):")
    for user in users:
        status = "active" if user.is_active else "inactive"
        typer.echo(f"- {user.id}: {user.name} <{user.email}> [{status}]")

@app.command("get")
def get_user(user_id: str) -> None:
    """Get user by ID."""
    ctx = get_user_context()
    user = user_services.get_user(ctx, user_id)

    if not user:
        typer.secho(f"User not found: {user_id}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

    typer.echo(f"ID: {user.id}")
    typer.echo(f"Email: {user.email}")
    typer.echo(f"Name: {user.name}")
    typer.echo(f"Active: {user.is_active}")
    typer.echo(f"Created: {user.created_at}")

@app.command("deactivate")
def deactivate_user(user_id: str) -> None:
    """Deactivate user."""
    ctx = get_user_context()
    user = user_services.deactivate_user(ctx, user_id)

    if not user:
        typer.secho(f"User not found: {user_id}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

    typer.echo(f"Deactivated user: {user.id}")
```

Register in `src/starter/cli/main.py`:

```python
from starter.cli.commands import user

app.add_typer(user.app, name="user")
```

### Step 7: Create Fake for Testing

Create `tests/fakes/fake_user_repository.py`:

```python
from typing import Optional
from starter.domain.entities import User

class FakeUserRepository:
    """Fake user repository for testing."""

    def __init__(self) -> None:
        self._users: dict[str, User] = {}
        self._email_index: dict[str, str] = {}

    def save(self, user: User) -> None:
        self._users[user.id] = user
        self._email_index[user.email] = user.id

    def get(self, user_id: str) -> Optional[User]:
        return self._users.get(user_id)

    def get_by_email(self, email: str) -> Optional[User]:
        user_id = self._email_index.get(email)
        return self._users.get(user_id) if user_id else None

    def list_all(self) -> list[User]:
        return list(self._users.values())

    def delete(self, user_id: str) -> bool:
        if user_id in self._users:
            user = self._users[user_id]
            del self._users[user_id]
            del self._email_index[user.email]
            return True
        return False

    # Test helpers
    def reset(self) -> None:
        """Reset fake to initial state."""
        self._users.clear()
        self._email_index.clear()
```

### Step 8: Write Tests

Create `tests/unit/test_user_services.py`:

```python
import pytest
from starter.domain.entities import User
from starter.services.context import ServiceContext
from starter.services import user_services
from tests.fakes.fake_user_repository import FakeUserRepository

@pytest.fixture
def user_context():
    """Build context with fake user repository."""
    return ServiceContext(
        repository=...,  # existing repository
        user_repository=FakeUserRepository(),
        config=...,
        logger=...,
    )

def test_create_user(user_context):
    """Test creating user."""
    user = user_services.create_user(
        user_context,
        email="test@example.com",
        name="Test User",
    )

    assert user.email == "test@example.com"
    assert user.name == "Test User"
    assert user.is_active is True

def test_create_user_duplicate_email(user_context):
    """Test that duplicate email raises error."""
    # Create first user
    user_services.create_user(user_context, "test@example.com", "User 1")

    # Try to create duplicate
    with pytest.raises(user_services.DuplicateUserError):
        user_services.create_user(user_context, "test@example.com", "User 2")

def test_get_user(user_context):
    """Test getting user by ID."""
    created = user_services.create_user(user_context, "test@example.com", "Test")

    retrieved = user_services.get_user(user_context, created.id)

    assert retrieved == created

def test_list_users(user_context):
    """Test listing all users."""
    u1 = user_services.create_user(user_context, "user1@example.com", "User 1")
    u2 = user_services.create_user(user_context, "user2@example.com", "User 2")

    users = user_services.list_users(user_context)

    assert len(users) == 2
    assert u1 in users
    assert u2 in users

def test_deactivate_user(user_context):
    """Test deactivating user."""
    user = user_services.create_user(user_context, "test@example.com", "Test")
    assert user.is_active is True

    deactivated = user_services.deactivate_user(user_context, user.id)

    assert deactivated is not None
    assert deactivated.is_active is False
```

### Step 9: Test the Feature

```bash
# Run tests
uv run pytest tests/unit/test_user_services.py -v

# Test CLI
uv run starter-cli user create "alice@example.com" "Alice"
uv run starter-cli user create "bob@example.com" "Bob"
uv run starter-cli user list
uv run starter-cli user get <user-id>
uv run starter-cli user deactivate <user-id>
```

## Quick Reference Checklist

When adding a feature:

- [ ] Define domain entities in `src/starter/domain/`
- [ ] Create protocol interfaces in `src/starter/protocols/`
- [ ] Update `ServiceContext` in `src/starter/services/context.py`
- [ ] Write service functions in `src/starter/services/`
- [ ] Implement adapters in `src/starter/adapters/`
- [ ] Create fakes in `tests/fakes/`
- [ ] Write unit tests in `tests/unit/`
- [ ] Add CLI commands in `src/starter/cli/commands/`
- [ ] Register commands in `src/starter/cli/main.py`
- [ ] Update documentation if needed

## Best Practices

### 1. Start with Domain

Always start by modeling your domain:
- What are the entities (objects with identity)?
- What are the value objects (immutable values)?
- What business rules apply?

### 2. Keep Services Pure

Service functions should be pure (no side effects except via context):
- Take `ServiceContext` as first parameter
- Accept business parameters
- Return results or raise exceptions
- Don't access globals or environment directly

### 3. Protocol Over Concrete

Always define protocols for external dependencies:
- Makes testing easier (inject fakes)
- Allows flexibility in implementations
- Follows dependency inversion principle

### 4. Test with Fakes

Prefer fakes over mocks:
- Fakes are real implementations (in-memory)
- More maintainable than mocks
- Better reflect actual behavior

### 5. CLI is Thin

CLI should only:
- Parse user input
- Build context
- Call service functions
- Format output

Business logic belongs in services, not CLI.

## Common Patterns

### Adding Validation

```python
def create_user(ctx: ServiceContext, email: str, name: str) -> User:
    # Validate inputs
    if not email or "@" not in email:
        raise ValidationError("Invalid email")
    if len(name) < 2:
        raise ValidationError("Name too short")

    # Create entity
    user = User(email=email, name=name)

    # Persist
    ctx.user_repository.save(user)

    return user
```

### Adding Business Logic

```python
def upgrade_user_to_premium(ctx: ServiceContext, user_id: str) -> User:
    """Upgrade user to premium."""
    user = ctx.user_repository.get(user_id)
    if not user:
        raise NotFoundError(f"User {user_id} not found")

    if user.is_premium:
        raise BusinessRuleViolation("User already premium")

    user.upgrade_to_premium()
    ctx.user_repository.save(user)
    ctx.logger.info(f"Upgraded user {user.id} to premium")

    return user
```

### Adding External API Integration

```python
# 1. Define protocol
class EmailService(Protocol):
    def send(self, to: str, subject: str, body: str) -> bool: ...

# 2. Add to context
@dataclass(frozen=True)
class ServiceContext:
    email_service: EmailService
    ...

# 3. Use in service
def create_user_and_send_welcome(ctx: ServiceContext, email: str, name: str) -> User:
    user = create_user(ctx, email, name)
    ctx.email_service.send(email, "Welcome!", f"Hello {name}!")
    return user
```

## Related Documentation

- [Architecture Overview](../architecture/overview.md) - Design patterns
- [Testing Guide](testing-guide.md) - Testing strategies
- [Development Workflow](development-workflow.md) - Daily practices

## Sources

- [Domain-Driven Design](https://www.domainlanguage.com/ddd/) - Domain modeling
- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html) - Architecture patterns
- [Architecture Patterns with Python](https://www.cosmicpython.com/book/) - Python implementation
