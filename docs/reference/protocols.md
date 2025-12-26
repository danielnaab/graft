---
title: Protocols Reference
status: stable
owners: [documentation-team]
---

# Protocols Reference

Reference for Protocol interfaces used in this template.

## What are Protocols?

Protocols define interfaces using structural typing (duck typing with type safety). Any class implementing the required methods satisfies the protocol - no inheritance needed.

**Example**:
```python
from typing import Protocol

class Repository(Protocol):
    def save(self, entity) -> None: ...
    def get(self, entity_id: str): ...

# Any class with save() and get() satisfies Repository
class InMemoryRepository:
    def save(self, entity): ...
    def get(self, entity_id: str): ...

# ✅ Type checker accepts this
repo: Repository = InMemoryRepository()
```

## Repository[T] Protocol

**Location**: `src/starter/protocols/repository.py`

### Definition

```python
from typing import Protocol, Generic, TypeVar, Optional

T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    """Generic repository for entity persistence."""

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
        """Delete entity. Returns True if deleted, False if not found."""
        ...
```

### Usage

```python
# In ServiceContext
@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]  # Generic protocol

# Implementations
class InMemoryRepository(Generic[T]):
    # Implements all methods -> satisfies protocol
    ...

class PostgresRepository(Generic[T]):
    # Implements all methods -> satisfies protocol
    ...
```

### Implementations

**In-Memory** (`src/starter/adapters/repository.py`):
- Stores entities in dict
- Fast, for development/testing

**Fake** (`tests/fakes/fake_repository.py`):
- In-memory with test helpers
- For unit testing

## Creating New Protocols

### Step 1: Define Protocol

```python
# src/starter/protocols/email_service.py

from typing import Protocol

class EmailService(Protocol):
    """Email sending interface."""

    def send(
        self,
        to: str,
        subject: str,
        body: str,
    ) -> bool:
        """Send email. Returns True if sent successfully."""
        ...

    def send_bulk(
        self,
        recipients: list[str],
        subject: str,
        body: str,
    ) -> int:
        """Send to multiple recipients. Returns count sent."""
        ...
```

### Step 2: Add to Context

```python
# src/starter/services/context.py

@dataclass(frozen=True)
class ServiceContext:
    repository: Repository[Entity]
    email_service: EmailService  # Add protocol
    config: Config
    logger: Logger
```

### Step 3: Implement

```python
# src/starter/adapters/email_service.py

class SMTPEmailService:
    """SMTP email service implementation."""

    def __init__(self, smtp_url: str):
        self._smtp_url = smtp_url

    def send(self, to: str, subject: str, body: str) -> bool:
        # SMTP implementation
        ...

    def send_bulk(self, recipients: list[str], subject: str, body: str) -> int:
        # Bulk send implementation
        ...
```

### Step 4: Create Fake

```python
# tests/fakes/fake_email_service.py

class FakeEmailService:
    """Fake email service for testing."""

    def __init__(self):
        self.sent_emails: list[dict] = []

    def send(self, to: str, subject: str, body: str) -> bool:
        self.sent_emails.append({"to": to, "subject": subject, "body": body})
        return True

    def send_bulk(self, recipients: list[str], subject: str, body: str) -> int:
        for to in recipients:
            self.send(to, subject, body)
        return len(recipients)

    # Test helpers
    def reset(self):
        self.sent_emails.clear()

    def assert_sent_to(self, email: str):
        assert any(e["to"] == email for e in self.sent_emails)
```

## Protocol Best Practices

### 1. Small, Focused Protocols

**Good**:
```python
class EmailSender(Protocol):
    def send(self, to: str, subject: str, body: str) -> bool: ...

class EmailBulkSender(Protocol):
    def send_bulk(self, recipients: list[str], subject: str, body: str) -> int: ...
```

**Bad**:
```python
class EmailService(Protocol):
    def send(self, ...) -> bool: ...
    def send_bulk(self, ...) -> int: ...
    def validate_email(self, ...) -> bool: ...
    def get_templates(self) -> list: ...
    def render_template(self, ...) -> str: ...
    # Too many responsibilities!
```

### 2. Use Generics

```python
T = TypeVar("T")

class Repository(Protocol, Generic[T]):
    """Works with any entity type."""
    def save(self, entity: T) -> None: ...
    def get(self, entity_id: str) -> Optional[T]: ...
```

### 3. Document Behavior

```python
class Repository(Protocol, Generic[T]):
    """Repository interface.

    Implementations must:
    - Make save() idempotent (safe to call multiple times)
    - Return None from get() if entity not found
    - Raise RepositoryError on storage failures
    """
    def save(self, entity: T) -> None:
        """Persist entity. Idempotent."""
        ...
```

### 4. Method Signatures

```python
class Repository(Protocol):
    def save(self, entity: Entity) -> None:
        """Clear return type."""
        ...

    def get(self, entity_id: str) -> Optional[Entity]:
        """Optional for nullable returns."""
        ...

    def list_all(self, limit: int = 100) -> list[Entity]:
        """Defaults allowed."""
        ...
```

## Runtime Checking (Optional)

```python
from typing import runtime_checkable

@runtime_checkable
class Repository(Protocol):
    def save(self, entity) -> None: ...
    def get(self, entity_id: str): ...

# Runtime check
repo = InMemoryRepository()
isinstance(repo, Repository)  # True
```

**Note**: Runtime checking rarely needed. Static type checking is sufficient.

## Type Checking

### mypy

```bash
# Check types
uv run mypy src/
```

**Configuration** (`pyproject.toml`):
```toml
[tool.mypy]
python_version = "3.11"
strict = true
warn_return_any = true
disallow_untyped_defs = true
```

### pyright

Built into VS Code with Pylance.

**Configuration** (`pyproject.toml`):
```toml
[tool.pyright]
pythonVersion = "3.11"
typeCheckingMode = "strict"
```

## Common Patterns

### Repository Pattern

```python
class Repository(Protocol, Generic[T]):
    def save(self, entity: T) -> None: ...
    def get(self, id: str) -> Optional[T]: ...
    def list_all(self) -> list[T]: ...
    def delete(self, id: str) -> bool: ...
```

### Service Pattern

```python
class ExternalAPI(Protocol):
    def fetch_data(self, resource_id: str) -> dict: ...
    def create_resource(self, data: dict) -> str: ...
    def update_resource(self, resource_id: str, data: dict) -> bool: ...
```

### Logger Pattern

```python
class Logger(Protocol):
    def debug(self, message: str) -> None: ...
    def info(self, message: str) -> None: ...
    def warning(self, message: str) -> None: ...
    def error(self, message: str, exc_info: bool = False) -> None: ...
```

## Related Documentation

- [Dependency Injection Architecture](../architecture/dependency-injection.md) - DI with protocols
- [ADR 003: Protocol-Based DI](../decisions/003-protocol-di.md) - Decision rationale
- [Context Objects Reference](context-objects.md) - Using protocols in context

## Sources

- [PEP 544: Protocols](https://peps.python.org/pep-0544/)
- [typing.Protocol Documentation](https://docs.python.org/3/library/typing.html#typing.Protocol)
- [mypy Protocols Guide](https://mypy.readthedocs.io/en/stable/protocols.html)
