---
title: Testing Guide
status: stable
owners: [documentation-team]
---

# Testing Guide

This guide covers testing strategies and best practices for the Python Starter Template.

## Running Tests

### Basic Commands

```bash
# Run all tests
uv run pytest

# Run with verbose output
uv run pytest -v

# Run specific test file
uv run pytest tests/unit/test_services.py

# Run specific test
uv run pytest tests/unit/test_services.py::test_create_example

# Run tests matching pattern
uv run pytest -k "test_create"
```

### With Coverage

```bash
# Run with coverage report
uv run pytest --cov=starter --cov-report=term-missing

# Generate HTML coverage report
uv run pytest --cov=starter --cov-report=html

# Open coverage report
open htmlcov/index.html  # macOS
xdg-open htmlcov/index.html  # Linux
```

### Useful Options

```bash
# Stop on first failure
uv run pytest -x

# Show local variables on failure
uv run pytest -l

# Run last failed tests only
uv run pytest --lf

# Run in parallel (fast)
uv run pytest -n auto
```

## Test Organization

### Directory Structure

```
tests/
├── __init__.py
├── conftest.py              # Shared fixtures
│
├── unit/                    # Unit tests (fast)
│   ├── test_domain.py       # Test entities/value objects
│   └── test_services.py     # Test service functions
│
├── integration/             # Integration tests
│   └── test_adapters.py     # Test with real systems
│
└── fakes/                   # Fake implementations
    ├── fake_repository.py
    └── fake_external_api.py
```

### Test File Naming

- Prefix with `test_`: `test_services.py`
- Match source module: `services.py` → `test_services.py`
- Group related tests: `test_user_services.py`

## Writing Unit Tests

### Test Structure (AAA Pattern)

```python
def test_example():
    # Arrange - setup test data
    ctx = build_test_context()
    name = "test-item"
    value = 100

    # Act - execute the function
    result = create_example(ctx, name=name, value=value)

    # Assert - verify behavior
    assert result.name == name
    assert result.value == value
    assert ctx.repository.get(result.id) == result
```

### Testing Service Functions

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
    """Test successful creation."""
    entity = example_service.create_example(
        test_context,
        name="test",
        value=100,
    )

    assert entity.name == "test"
    assert entity.value == 100
    assert test_context.repository.get(entity.id) is not None

def test_create_example_invalid_value(test_context):
    """Test that invalid value raises error."""
    with pytest.raises(ValueError, match="non-negative"):
        example_service.create_example(test_context, "test", -1)
```

### Testing Domain Entities

```python
def test_user_creation():
    """Test creating valid user."""
    user = User(email="test@example.com", name="Test User")

    assert user.email == "test@example.com"
    assert user.name == "Test User"
    assert user.id  # Generated
    assert user.is_active is True

def test_user_invalid_email():
    """Test that invalid email raises error."""
    with pytest.raises(ValueError, match="Invalid email"):
        User(email="invalid", name="Test")

def test_user_equality():
    """Test that equality is based on ID."""
    user1 = User(email="test@example.com", name="Test")
    user2 = User(email="other@example.com", name="Other")
    user2.id = user1.id  # Same ID

    assert user1 == user2  # Equal despite different attributes
```

## Using Fixtures

### Defining Fixtures

```python
# tests/conftest.py

import pytest
from starter.services.context import ServiceContext
from tests.fakes.fake_repository import FakeRepository

@pytest.fixture
def test_context():
    """Standard test context."""
    return ServiceContext(
        repository=FakeRepository(),
        config=Config(debug=True),
        logger=NullLogger(),
    )

@pytest.fixture
def sample_entity():
    """Sample entity for tests."""
    return Entity(name="test-entity", value=100)

@pytest.fixture(autouse=True)
def reset_state():
    """Reset global state before each test."""
    # Setup
    yield
    # Teardown - cleanup after test
```

### Using Fixtures

```python
def test_with_fixtures(test_context, sample_entity):
    """Test using fixtures."""
    test_context.repository.save(sample_entity)
    retrieved = test_context.repository.get(sample_entity.id)
    assert retrieved == sample_entity
```

## Parametrized Tests

### Testing Multiple Inputs

```python
@pytest.mark.parametrize("value,expected", [
    (0, True),
    (100, True),
    (-1, False),
    (1000, True),
])
def test_validate_value(value, expected):
    """Test value validation with multiple inputs."""
    result = validate_value(value)
    assert result == expected

@pytest.mark.parametrize("email", [
    "test@example.com",
    "user.name@domain.co.uk",
    "first+last@test.org",
])
def test_valid_emails(email):
    """Test various valid email formats."""
    user = User(email=email, name="Test")
    assert user.email == email

@pytest.mark.parametrize("email", [
    "invalid",
    "@example.com",
    "user@",
    "",
])
def test_invalid_emails(email):
    """Test that invalid emails raise error."""
    with pytest.raises(ValueError):
        User(email=email, name="Test")
```

## Creating Fakes

### Fake Repository Example

```python
# tests/fakes/fake_repository.py

from typing import Generic, TypeVar, Optional

T = TypeVar("T")

class FakeRepository(Generic[T]):
    """In-memory fake repository for testing."""

    def __init__(self) -> None:
        self._storage: dict[str, T] = {}
        self._save_count = 0

    def save(self, entity: T) -> None:
        """Save entity."""
        self._storage[entity.id] = entity
        self._save_count += 1

    def get(self, entity_id: str) -> Optional[T]:
        """Get entity by ID."""
        return self._storage.get(entity_id)

    def list_all(self) -> list[T]:
        """List all entities."""
        return list(self._storage.values())

    def delete(self, entity_id: str) -> bool:
        """Delete entity."""
        if entity_id in self._storage:
            del self._storage[entity_id]
            return True
        return False

    # Test helpers
    def reset(self) -> None:
        """Reset fake to initial state."""
        self._storage.clear()
        self._save_count = 0

    def assert_saved(self, entity_id: str) -> None:
        """Assert entity was saved."""
        assert entity_id in self._storage

    def get_save_count(self) -> int:
        """Get number of saves."""
        return self._save_count
```

## Testing Exceptions

### Assert Raises

```python
def test_create_with_negative_value(test_context):
    """Test that negative value raises ValueError."""
    with pytest.raises(ValueError):
        create_example(test_context, "test", -1)

def test_create_with_error_message(test_context):
    """Test exception message."""
    with pytest.raises(ValueError, match="non-negative"):
        create_example(test_context, "test", -1)

def test_not_found_error(test_context):
    """Test NotFoundError for missing entity."""
    with pytest.raises(NotFoundError) as exc_info:
        get_example(test_context, "nonexistent-id")

    assert "not found" in str(exc_info.value).lower()
```

## Integration Tests

### Testing with Real Database

```python
# tests/integration/test_repository.py

import pytest
from starter.adapters.repository import PostgresRepository

@pytest.fixture
def test_db():
    """Provide test database."""
    db = create_test_database()
    db.migrate()  # Run migrations
    yield db
    db.drop_all()  # Cleanup
    db.close()

def test_postgres_repository_save_and_get(test_db):
    """Test PostgresRepository with real database."""
    repo = PostgresRepository(test_db.connection_string)
    entity = Entity(name="test", value=100)

    repo.save(entity)
    retrieved = repo.get(entity.id)

    assert retrieved is not None
    assert retrieved.name == entity.name
```

## Test Markers

### Marking Tests

```python
import pytest

@pytest.mark.slow
def test_slow_operation():
    """Test that takes a long time."""
    ...

@pytest.mark.integration
def test_database_integration():
    """Integration test."""
    ...

@pytest.mark.skip(reason="Not implemented yet")
def test_future_feature():
    """Test for future feature."""
    ...

@pytest.mark.skipif(sys.version_info < (3, 11), reason="Requires Python 3.11+")
def test_modern_python_feature():
    """Test requiring Python 3.11+."""
    ...
```

### Running Marked Tests

```bash
# Run only fast tests
uv run pytest -m "not slow"

# Run only integration tests
uv run pytest -m integration

# Run multiple markers
uv run pytest -m "integration or slow"
```

## Best Practices

### 1. Test One Thing

```python
# Good - focused test
def test_create_saves_entity(test_context):
    """Test that create saves entity to repository."""
    entity = create_example(test_context, "test", 100)
    assert test_context.repository.get(entity.id) is not None

# Good - separate test for validation
def test_create_validates_value(test_context):
    """Test that create validates value."""
    with pytest.raises(ValueError):
        create_example(test_context, "test", -1)
```

### 2. Use Descriptive Names

```python
# Good names
def test_create_example_with_valid_data_succeeds()
def test_create_example_with_negative_value_raises_error()
def test_list_examples_returns_empty_list_when_no_entities()

# Bad names
def test1()
def test_example()
def test_create()
```

### 3. Arrange-Act-Assert

```python
def test_example():
    # Arrange
    ctx = build_context()
    data = prepare_test_data()

    # Act
    result = perform_operation(ctx, data)

    # Assert
    assert result.is_valid()
    assert ctx.repository.was_called()
```

### 4. Avoid Test Interdependence

```python
# Bad - tests depend on each other
def test_step1():
    global user
    user = create_user(...)

def test_step2():
    update_user(user, ...)  # Depends on test_step1

# Good - independent tests
def test_create_user():
    user = create_user(...)
    assert user is not None

def test_update_user():
    user = create_user(...)  # Setup in same test
    updated = update_user(user, ...)
    assert updated.name == "Updated"
```

### 5. Use Fakes Over Mocks

```python
# Preferred - fake implementation
def test_with_fake():
    fake_repo = FakeRepository()
    ctx = ServiceContext(repository=fake_repo, ...)

    entity = create_example(ctx, "test", 100)

    assert fake_repo.get(entity.id) == entity

# Less preferred - mock
from unittest.mock import Mock

def test_with_mock():
    mock_repo = Mock()
    ctx = ServiceContext(repository=mock_repo, ...)

    create_example(ctx, "test", 100)

    mock_repo.save.assert_called_once()
```

## Coverage Guidelines

### Target Coverage

- **Overall**: 80%+
- **Services**: 90%+
- **Domain**: 95%+
- **Adapters**: 70%+

### What to Test

**High priority**:
- Business logic
- Validation rules
- Error conditions
- Edge cases

**Low priority**:
- Trivial getters/setters
- Framework code
- Third-party libraries

### Viewing Coverage

```bash
# Terminal report
uv run pytest --cov=starter --cov-report=term-missing

# HTML report (detailed)
uv run pytest --cov=starter --cov-report=html
open htmlcov/index.html
```

## Troubleshooting

### Tests Not Found

**Issue**: pytest can't find tests

**Solution**:
```bash
# Ensure package is installed
uv pip install -e .

# Check test file naming (must start with test_)
# Check test function naming (must start with test_)
```

### Import Errors

**Issue**: `ModuleNotFoundError`

**Solution**:
```bash
# Install in editable mode
uv pip install -e .

# Check PYTHONPATH
export PYTHONPATH="${PYTHONPATH}:${PWD}/src"
```

### Fixture Not Found

**Issue**: pytest can't find fixture

**Solution**:
- Ensure fixture is in `conftest.py`
- Check fixture name spelling
- Verify conftest.py is in test directory

## Related Documentation

- [Testing Strategy](../architecture/testing-strategy.md) - Testing philosophy
- [Development Workflow](development-workflow.md) - Daily practices
- [Adding Features](adding-features.md) - Feature development with tests

## Sources

- [pytest Documentation](https://docs.pytest.org/) - Official pytest docs
- [Python Testing with pytest](https://pythontest.com/pytest-book/) - Comprehensive guide
- [Test Driven Development](https://www.obeythetestinggoat.com/) - TDD practices
