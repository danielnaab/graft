---
title: Testing Strategy
status: stable
owners: [architecture-team]
---

# Testing Strategy

This document outlines the testing philosophy and patterns used in this template, emphasizing practical, maintainable tests.

## Testing Philosophy

### Key Principles

1. **Tests document behavior**: Tests are executable documentation
2. **Fast feedback**: Unit tests run in milliseconds
3. **Fakes over mocks**: Use real implementations when possible
4. **Test the interface**: Test behavior, not implementation
5. **Pragmatic coverage**: Focus on business logic, not getters/setters

### Goals

- **Confidence**: Deploy with confidence that code works
- **Regression prevention**: Catch breaks early
- **Documentation**: Show how to use the code
- **Design feedback**: Tests reveal design issues

## Test Pyramid

```
          ┌─────────────────┐
          │       E2E       │  ← Few (slow, brittle, expensive)
          │  (End-to-End)   │
          ├─────────────────┤
          │   Integration   │  ← Some (moderate speed/cost)
          │    (Adapters)   │
          ├─────────────────┤
          │      Unit       │  ← Many (fast, cheap, focused)
          │  (Services +    │
          │    Domain)      │
          └─────────────────┘
```

### Unit Tests (Many)

**Scope**: Service functions and domain logic in isolation

**Speed**: Milliseconds

**Dependencies**: Fakes

**Purpose**: Verify business logic correctness

**Location**: `tests/unit/`

### Integration Tests (Some)

**Scope**: Adapters with real external systems

**Speed**: Seconds

**Dependencies**: Test databases, test APIs

**Purpose**: Verify adapters work with real infrastructure

**Location**: `tests/integration/`

### End-to-End Tests (Few)

**Scope**: Complete workflows through CLI

**Speed**: Seconds to minutes

**Dependencies**: Full system

**Purpose**: Verify critical user workflows

**Location**: `tests/e2e/` (optional in this template)

## Fakes Over Mocks

### What are Fakes?

**Fake**: A real, working implementation designed for testing.

**Characteristics**:
- Implements the protocol fully
- Simpler than production (e.g., in-memory instead of database)
- Deterministic and fast
- Can add test helpers

### Example Fake

```python
from typing import Generic, TypeVar, Optional

T = TypeVar("T")

class FakeRepository(Generic[T]):
    """Fake repository implementation for testing.

    Stores entities in memory. Fully implements Repository[T] protocol.
    """

    def __init__(self) -> None:
        self._storage: dict[str, T] = {}
        self._save_count = 0
        self._delete_count = 0

    def save(self, entity: T) -> None:
        """Save entity in memory."""
        self._storage[entity.id] = entity
        self._save_count += 1

    def get(self, entity_id: str) -> Optional[T]:
        """Get entity from memory."""
        return self._storage.get(entity_id)

    def list_all(self) -> list[T]:
        """List all entities in memory."""
        return list(self._storage.values())

    def delete(self, entity_id: str) -> bool:
        """Delete entity from memory."""
        if entity_id in self._storage:
            del self._storage[entity_id]
            self._delete_count += 1
            return True
        return False

    # Test helpers
    def reset(self) -> None:
        """Reset fake to initial state."""
        self._storage.clear()
        self._save_count = 0
        self._delete_count = 0

    def assert_saved(self, entity_id: str) -> None:
        """Assert entity was saved."""
        assert entity_id in self._storage, f"Entity {entity_id} not saved"

    def get_save_count(self) -> int:
        """Get number of save() calls."""
        return self._save_count
```

### Fakes vs Mocks

| Aspect | Fake | Mock |
|--------|------|------|
| **Implementation** | Real working code | Stub that records calls |
| **Reusability** | Reused across tests | Created per test |
| **Maintenance** | One fake for all tests | Update many mocks |
| **Refactoring** | Resilient to refactoring | Brittle |
| **Verification** | State-based | Interaction-based |

**Example with Mock** (less preferred):
```python
from unittest.mock import Mock

def test_with_mock():
    # Mock setup
    mock_repo = Mock()
    mock_repo.save = Mock()
    mock_repo.get = Mock(return_value=entity)

    ctx = ServiceContext(repository=mock_repo, ...)

    # Act
    result = create_example(ctx, "test", 100)

    # Verify mock interactions
    mock_repo.save.assert_called_once_with(result)
    mock_repo.get.assert_called()
```

**Example with Fake** (preferred):
```python
def test_with_fake():
    # Fake setup (simpler)
    fake_repo = FakeRepository()
    ctx = ServiceContext(repository=fake_repo, ...)

    # Act
    result = create_example(ctx, "test", 100)

    # Verify state (more natural)
    assert fake_repo.get(result.id) == result
    assert fake_repo.get_save_count() == 1
```

**When to use mocks**:
- Testing error conditions (raise exceptions)
- Verifying specific call patterns
- External API that's too complex to fake

**Prefer fakes when possible** - they're more maintainable.

## Unit Testing Patterns

### Testing Service Functions

**Pattern**: Use test context with fakes.

```python
import pytest
from starter.services.context import ServiceContext
from starter.services import example_service
from tests.fakes.fake_repository import FakeRepository

@pytest.fixture
def test_context():
    """Build test context with fakes."""
    return ServiceContext(
        repository=FakeRepository(),
        config=Config(debug=True),
        logger=NullLogger(),
    )

def test_create_example_success(test_context):
    """Test successful entity creation."""
    # Act
    entity = example_service.create_example(
        test_context,
        name="test",
        value=100,
    )

    # Assert
    assert entity.name == "test"
    assert entity.value == 100
    assert test_context.repository.get(entity.id) == entity

def test_create_example_negative_value(test_context):
    """Test that negative value raises ValueError."""
    with pytest.raises(ValueError, match="non-negative"):
        example_service.create_example(
            test_context,
            name="test",
            value=-1,
        )

def test_list_examples_empty(test_context):
    """Test listing when no entities exist."""
    entities = example_service.list_examples(test_context)
    assert entities == []

def test_list_examples_multiple(test_context):
    """Test listing multiple entities."""
    # Arrange - create several entities
    e1 = example_service.create_example(test_context, "first", 1)
    e2 = example_service.create_example(test_context, "second", 2)

    # Act
    entities = example_service.list_examples(test_context)

    # Assert
    assert len(entities) == 2
    assert e1 in entities
    assert e2 in entities
```

### Testing Domain Logic

**Pattern**: Test entities and value objects directly.

```python
import pytest
from starter.domain.entities import User
from starter.domain.value_objects import Money
from decimal import Decimal

def test_user_creation_valid():
    """Test creating user with valid data."""
    user = User(email="test@example.com", name="Test User")
    assert user.email == "test@example.com"
    assert user.name == "Test User"
    assert user.id  # ID generated
    assert user.is_active is True

def test_user_creation_invalid_email():
    """Test that invalid email raises ValueError."""
    with pytest.raises(ValueError, match="Invalid email"):
        User(email="invalid", name="Test")

def test_user_deactivate():
    """Test user deactivation."""
    user = User(email="test@example.com", name="Test")
    assert user.is_active is True

    user.deactivate()

    assert user.is_active is False

def test_money_immutable():
    """Test that Money is immutable."""
    money = Money(Decimal("100"), "USD")

    with pytest.raises(AttributeError):  # Frozen dataclass
        money.amount = Decimal("200")

def test_money_add_same_currency():
    """Test adding money in same currency."""
    m1 = Money(Decimal("100"), "USD")
    m2 = Money(Decimal("50"), "USD")

    result = m1.add(m2)

    assert result.amount == Decimal("150")
    assert result.currency == "USD"

def test_money_add_different_currency():
    """Test that adding different currencies raises error."""
    m1 = Money(Decimal("100"), "USD")
    m2 = Money(Decimal("50"), "EUR")

    with pytest.raises(ValueError, match="Cannot add USD and EUR"):
        m1.add(m2)
```

## Integration Testing Patterns

### Testing Adapters

**Pattern**: Use real external systems (test instances).

```python
import pytest
from starter.adapters.repository import PostgresRepository
from starter.domain.entities import User

@pytest.fixture
def test_database():
    """Provide test database connection."""
    # Setup test database
    db = create_test_database()
    db.migrate()

    yield db

    # Teardown
    db.drop_all()
    db.close()

def test_postgres_repository_save_and_get(test_database):
    """Test PostgresRepository with real database."""
    # Arrange
    repo = PostgresRepository(test_database.connection_string)
    user = User(email="test@example.com", name="Test User")

    # Act
    repo.save(user)
    retrieved = repo.get(user.id)

    # Assert
    assert retrieved is not None
    assert retrieved.id == user.id
    assert retrieved.email == user.email
    assert retrieved.name == user.name

def test_postgres_repository_delete(test_database):
    """Test entity deletion."""
    repo = PostgresRepository(test_database.connection_string)
    user = User(email="test@example.com", name="Test")

    # Save then delete
    repo.save(user)
    result = repo.delete(user.id)
    assert result is True

    # Verify deleted
    retrieved = repo.get(user.id)
    assert retrieved is None

def test_postgres_repository_list_all(test_database):
    """Test listing all entities."""
    repo = PostgresRepository(test_database.connection_string)

    # Create multiple users
    u1 = User(email="user1@example.com", name="User 1")
    u2 = User(email="user2@example.com", name="User 2")
    repo.save(u1)
    repo.save(u2)

    # List all
    users = repo.list_all()

    assert len(users) == 2
    assert any(u.id == u1.id for u in users)
    assert any(u.id == u2.id for u in users)
```

### Testing with External APIs

**Pattern**: Use test API instances or contract testing.

```python
@pytest.fixture
def test_api_client():
    """Provide test API client."""
    return HTTPExternalAPI(
        base_url="https://api-test.example.com",
        api_key="test_key_12345",
    )

def test_external_api_fetch_data(test_api_client):
    """Test fetching data from external API."""
    result = test_api_client.fetch_data(resource_id="test-123")

    assert result is not None
    assert result["id"] == "test-123"
```

## Test Organization

### Directory Structure

```
tests/
├── __init__.py
├── conftest.py              # Pytest fixtures (shared)
│
├── unit/                    # Unit tests
│   ├── __init__.py
│   ├── test_domain.py       # Domain entity/value object tests
│   └── test_services.py     # Service function tests
│
├── integration/             # Integration tests
│   ├── __init__.py
│   └── test_adapters.py     # Adapter tests with real systems
│
└── fakes/                   # Fake implementations
    ├── __init__.py
    ├── fake_repository.py   # Fake repository
    └── fake_external_api.py # Fake external API
```

### conftest.py

**Purpose**: Shared pytest fixtures.

```python
"""Shared pytest fixtures."""

import pytest
from starter.services.context import ServiceContext
from starter.domain.entities import Entity
from tests.fakes.fake_repository import FakeRepository

@pytest.fixture
def test_context() -> ServiceContext:
    """Build test context with fakes.

    Returns:
        ServiceContext with in-memory fakes for testing
    """
    return ServiceContext(
        repository=FakeRepository(),
        config=Config(debug=True, environment="test"),
        logger=NullLogger(),
    )

@pytest.fixture
def sample_entity() -> Entity:
    """Create sample entity for tests."""
    return Entity(name="test-entity", value=100)

@pytest.fixture(autouse=True)
def reset_fakes():
    """Reset all fakes before each test."""
    # Reset any global fakes
    yield
    # Cleanup after test
```

## Coverage Guidelines

### What to Test

**High priority** (must test):
- Business logic in services
- Domain entity validation and business methods
- Value object immutability and operations
- Error conditions and edge cases

**Medium priority** (should test):
- Adapter implementations (integration tests)
- Complex data transformations
- Algorithm correctness

**Low priority** (can skip):
- Trivial getters/setters
- Framework code (Django models, SQLAlchemy, etc.)
- Third-party libraries
- Configuration loading

### Coverage Targets

**Recommended**:
- Overall: 80%+
- Services: 90%+
- Domain: 95%+
- Adapters: 70%+ (integration tests)

**Not all code needs tests** - focus on business logic.

### Running Coverage

```bash
# Run tests with coverage
uv run pytest --cov=starter --cov-report=term-missing

# Generate HTML coverage report
uv run pytest --cov=starter --cov-report=html

# View report
open htmlcov/index.html
```

## Test Naming Conventions

### Pattern

```python
def test_<function>_<scenario>():
    """Test that <expected behavior> when <conditions>."""
```

### Examples

```python
def test_create_user_valid_data():
    """Test that user is created when data is valid."""

def test_create_user_invalid_email():
    """Test that ValueError is raised when email is invalid."""

def test_list_users_empty():
    """Test that empty list is returned when no users exist."""

def test_list_users_multiple():
    """Test that all users are returned when multiple exist."""
```

## Pytest Features

### Fixtures

**Reusable test setup**:

```python
@pytest.fixture
def user():
    """Create test user."""
    return User(email="test@example.com", name="Test")

def test_with_fixture(user):
    """Test uses the fixture."""
    assert user.email == "test@example.com"
```

### Parametrization

**Test multiple inputs**:

```python
@pytest.mark.parametrize("value,expected", [
    (0, True),
    (17, False),
    (18, True),
    (21, True),
    (150, True),
])
def test_is_adult(value, expected):
    """Test adult validation with multiple ages."""
    user = User(email="test@example.com", name="Test", age=value)
    assert user.is_adult() == expected
```

### Markers

**Organize tests**:

```python
@pytest.mark.slow
def test_slow_operation():
    """Test that takes a long time."""
    ...

@pytest.mark.integration
def test_database_integration():
    """Integration test with database."""
    ...

# Run only fast tests
# pytest -m "not slow"

# Run only integration tests
# pytest -m integration
```

## Best Practices

### 1. Arrange-Act-Assert (AAA)

Structure tests clearly:

```python
def test_example():
    # Arrange - setup test data
    ctx = test_context()
    name = "test"
    value = 100

    # Act - call the function
    result = create_example(ctx, name, value)

    # Assert - verify behavior
    assert result.name == name
    assert result.value == value
```

### 2. One Assertion Per Test (guideline, not rule)

Prefer focused tests:

```python
# Good - focused
def test_user_email_valid():
    user = User(email="test@example.com", name="Test")
    assert user.email == "test@example.com"

def test_user_name_valid():
    user = User(email="test@example.com", name="Test")
    assert user.name == "Test"

# Acceptable - related assertions
def test_user_creation():
    user = User(email="test@example.com", name="Test")
    assert user.email == "test@example.com"
    assert user.name == "Test"
    assert user.is_active is True
```

### 3. Test Behavior, Not Implementation

**Bad** (tests implementation):
```python
def test_create_user_calls_repo_save():
    # Testing implementation detail
    mock_repo.save.assert_called_once()
```

**Good** (tests behavior):
```python
def test_create_user_persists_entity():
    # Testing observable behavior
    user = create_user(ctx, "test@example.com", "Test")
    assert ctx.repository.get(user.id) is not None
```

### 4. Avoid Test Interdependence

Each test should be independent:

```python
# Bad - tests depend on order
def test_step1():
    global user
    user = create_user(...)

def test_step2():
    # Depends on test_step1
    update_user(user, ...)

# Good - independent
def test_update_user():
    user = create_user(...)  # Setup in same test
    updated = update_user(user, ...)
    assert updated.name == "Updated"
```

## Related Documentation

- [Functional Services](functional-services.md) - Testing service functions
- [Dependency Injection](dependency-injection.md) - Using fakes with context
- [Domain Model](domain-model.md) - Testing domain logic
- [Architecture Overview](overview.md) - Testing in overall architecture

## Sources

- [pytest Documentation](https://docs.pytest.org/) - Testing framework
- [Test Pyramid - Martin Fowler](https://martinfowler.com/bliki/TestPyramid.html) - Testing strategy
- [Mocks Aren't Stubs - Martin Fowler](https://martinfowler.com/articles/mocksArentStubs.html) - Mocks vs fakes
- [Architecture Patterns with Python - Testing](https://www.cosmicpython.com/book/chapter_03_abstractions.html) - Testing with abstractions
- [Python Testing with pytest](https://pythontest.com/pytest-book/) - pytest best practices
