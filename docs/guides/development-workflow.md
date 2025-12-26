---
title: Development Workflow
status: stable
owners: [documentation-team]
---

# Development Workflow

This guide describes the day-to-day development workflow when working with this Python starter template.

## Daily Development Cycle

### 1. Start Your Work

```bash
# Navigate to project
cd python-starter

# Ensure dependencies are up to date
uv sync

# Create feature branch (optional)
git checkout -b feature/my-feature
```

### 2. Make Changes

Edit code in your preferred editor:
- Domain logic: `src/starter/domain/`
- Services: `src/starter/services/`
- Adapters: `src/starter/adapters/`
- CLI: `src/starter/cli/`

### 3. Run Tests Frequently

```bash
# Run relevant tests
uv run pytest tests/unit/test_services.py

# Or run all tests
uv run pytest

# Watch mode (with pytest-watch)
uv run ptw  # Reruns tests on file changes
```

### 4. Check Code Quality

```bash
# Lint (check for issues)
uv run ruff check .

# Auto-fix issues
uv run ruff check --fix .

# Format code
uv run ruff format .
```

### 5. Test Manually (if needed)

```bash
# Run CLI to test functionality
uv run starter-cli create "test-item" 42
uv run starter-cli list
```

### 6. Commit Your Work

```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "Add user validation to create_example service"

# Push to remote
git push origin feature/my-feature
```

## Adding Dependencies

### Production Dependencies

```bash
# Add package
uv add package-name

# Example: Add httpx for HTTP requests
uv add httpx

# This automatically:
# - Updates pyproject.toml
# - Updates uv.lock
# - Installs in virtual environment
```

### Development Dependencies

```bash
# Add dev dependency
uv add --dev package-name

# Example: Add mypy for type checking
uv add --dev mypy
```

### Updating Dependencies

```bash
# Update all dependencies
uv lock --upgrade

# Update specific package
uv add package-name@latest

# Sync after updates
uv sync
```

## Code Organization

### Directory Structure

Follow the established patterns:

```
src/starter/
├── domain/              # Pure domain logic (no dependencies)
│   ├── entities.py      # Objects with identity
│   └── value_objects.py # Immutable values
│
├── services/            # Service functions (business logic)
│   ├── context.py       # ServiceContext definition
│   └── example_service.py  # Service functions
│
├── protocols/           # Interface definitions
│   ├── repository.py    # Repository protocol
│   └── external.py      # External service protocols
│
├── adapters/            # Implementations
│   ├── repository.py    # Repository implementations
│   └── external_api.py  # External API adapters
│
└── cli/                 # CLI layer
    ├── main.py          # Main Typer app
    └── commands/        # Command groups
```

### File Naming Conventions

- **Modules**: `snake_case.py`
- **Classes**: `PascalCase`
- **Functions**: `snake_case()`
- **Constants**: `UPPER_SNAKE_CASE`
- **Private**: `_leading_underscore`

### Import Organization

ruff automatically organizes imports:

```python
# Standard library
import os
from pathlib import Path

# Third-party
import typer
from rich.console import Console

# Local application
from starter.domain.entities import Entity
from starter.services.context import ServiceContext
```

## Testing Workflow

### Test-Driven Development (TDD)

**Red-Green-Refactor cycle**:

1. **Red**: Write failing test
   ```python
   def test_validate_email():
       with pytest.raises(ValidationError):
           validate_email("invalid")
   ```

2. **Green**: Make test pass
   ```python
   def validate_email(email: str) -> bool:
       if "@" not in email:
           raise ValidationError("Invalid email")
       return True
   ```

3. **Refactor**: Improve code
   ```python
   def validate_email(email: str) -> bool:
       import re
       pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
       if not re.match(pattern, email):
           raise ValidationError("Invalid email format")
       return True
   ```

### Running Tests Efficiently

```bash
# Run only failed tests
uv run pytest --lf

# Run tests in parallel
uv run pytest -n auto

# Run with coverage
uv run pytest --cov=starter --cov-report=html

# Open coverage report
open htmlcov/index.html
```

### Test Organization

```
tests/
├── unit/                    # Fast, isolated tests
│   ├── test_domain.py       # Test entities, value objects
│   └── test_services.py     # Test service functions
│
├── integration/             # Tests with real dependencies
│   └── test_adapters.py     # Test adapters with real systems
│
├── fakes/                   # Fake implementations
│   └── fake_repository.py   # In-memory repository for tests
│
└── conftest.py              # Shared fixtures
```

## Git Workflow

### Branch Strategy

```bash
# Feature branches
git checkout -b feature/add-user-validation

# Bug fixes
git checkout -b fix/handle-null-email

# Documentation
git checkout -b docs/update-readme
```

### Commit Messages

**Format**: `<type>: <description>`

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code refactoring
- `test`: Add/update tests
- `chore`: Maintenance

**Examples**:
```bash
git commit -m "feat: add email validation to User entity"
git commit -m "fix: handle None value in get_user service"
git commit -m "docs: update architecture overview"
git commit -m "refactor: extract validation to separate module"
git commit -m "test: add tests for edge cases in validation"
```

### Pre-Commit Checklist

Before committing, ensure:

- [ ] Tests pass: `uv run pytest`
- [ ] Code linted: `uv run ruff check .`
- [ ] Code formatted: `uv run ruff format .`
- [ ] Type hints added
- [ ] Docstrings written
- [ ] Changes documented (if needed)

### Useful Git Commands

```bash
# See what changed
git status
git diff

# Stage specific files
git add src/starter/services/user_services.py

# Amend last commit
git commit --amend

# Stash changes temporarily
git stash
git stash pop

# View commit history
git log --oneline --graph
```

## Code Review

### Creating Pull Request

1. **Push branch**:
   ```bash
   git push origin feature/my-feature
   ```

2. **Create PR** with description:
   ```markdown
   ## Summary
   - Add email validation to User entity
   - Update tests for validation logic

   ## Test Plan
   - [x] Unit tests pass
   - [x] Integration tests pass
   - [x] Manual testing with CLI
   ```

### Self-Review Checklist

Before requesting review:

- [ ] Code follows project patterns
- [ ] Tests cover new functionality
- [ ] Documentation updated
- [ ] No commented-out code
- [ ] No debug print statements
- [ ] Type hints on all functions
- [ ] Docstrings on public functions
- [ ] ruff checks pass
- [ ] Tests pass

## Performance Optimization

### Profiling

```bash
# Profile code
python -m cProfile -o profile.stats script.py

# View results
python -m pstats profile.stats
```

### Benchmarking

```bash
# Install pytest-benchmark
uv add --dev pytest-benchmark

# Write benchmark test
def test_performance(benchmark):
    result = benchmark(my_function, arg1, arg2)
    assert result is not None
```

## Debugging

### Using Python Debugger (pdb)

```python
def problematic_function(param: str) -> int:
    import pdb; pdb.set_trace()  # Add breakpoint
    result = process_data(param)
    return result
```

### Using VS Code Debugger

Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Python: Current File",
      "type": "python",
      "request": "launch",
      "program": "${file}",
      "console": "integratedTerminal",
      "cwd": "${workspaceFolder}"
    },
    {
      "name": "Python: pytest",
      "type": "python",
      "request": "launch",
      "module": "pytest",
      "args": ["-v"],
      "console": "integratedTerminal",
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Print Debugging

```python
# Use logger instead of print
from starter.services.context import ServiceContext

def my_service(ctx: ServiceContext, param: str) -> Result:
    ctx.logger.debug(f"Processing param: {param}")
    result = process(param)
    ctx.logger.debug(f"Result: {result}")
    return result
```

## Environment Management

### Development Environment

```bash
# Use development settings
export ENVIRONMENT=development
export DEBUG=true
export LOG_LEVEL=DEBUG

# Run with dev settings
uv run starter-cli create "test" 1
```

### Testing Environment

```bash
# Set test environment
export ENVIRONMENT=test

# Run tests
uv run pytest
```

### Environment Variables

Create `.env` file (gitignored):

```bash
# .env
DATABASE_URL=postgresql://localhost/starter_dev
API_KEY=dev_key_12345
LOG_LEVEL=DEBUG
```

Load with python-dotenv:

```bash
uv add python-dotenv
```

```python
from dotenv import load_dotenv
load_dotenv()
```

## Continuous Integration

### GitHub Actions

Automated checks on push/PR:

- Run tests
- Check linting
- Check formatting
- Build package

### Pre-Push Checks

Run locally before pushing:

```bash
# All checks
uv run pytest && \
uv run ruff check . && \
uv run ruff format --check .

# If all pass, push
git push
```

## Documentation

### When to Update Docs

Update documentation when:

- Adding new features
- Changing architecture
- Making breaking changes
- Fixing bugs (if behavior changes)

### Documentation Types

- **ADRs**: Major architectural decisions
- **Guides**: How-to documentation
- **Reference**: API documentation
- **Architecture**: Design explanations

## Best Practices

### 1. Keep Functions Small

**Good**:
```python
def create_user(ctx: ServiceContext, email: str, name: str) -> User:
    """Create user - single responsibility."""
    user = User(email=email, name=name)
    ctx.repository.save(user)
    return user
```

**Bad**:
```python
def create_user_and_send_email_and_log_and_notify(ctx, email, name):
    """Too many responsibilities."""
    # 50 lines of mixed concerns
```

### 2. Write Tests First

**TDD approach**:
1. Write test (fails)
2. Write minimal code (passes)
3. Refactor (still passes)

### 3. Commit Often

**Small, focused commits** are better than large ones:

```bash
git commit -m "Add User entity"
git commit -m "Add user validation"
git commit -m "Add tests for validation"
```

### 4. Use Type Hints

**Always use type hints**:

```python
def process(data: dict[str, Any], count: int) -> list[Result]:
    """Type hints help IDE and catch errors."""
    ...
```

### 5. Write Docstrings

**Document public functions**:

```python
def create_user(ctx: ServiceContext, email: str, name: str) -> User:
    """Create new user entity.

    Args:
        ctx: Service context with dependencies
        email: User email address
        name: User full name

    Returns:
        Created user entity

    Raises:
        ValidationError: If email or name invalid
    """
    ...
```

## Troubleshooting Common Issues

### Import Errors

**Issue**: `ModuleNotFoundError: No module named 'starter'`

**Solution**:
```bash
# Install package in editable mode
uv pip install -e .
```

### Type Errors

**Issue**: mypy reports type errors

**Solution**:
```bash
# Run mypy to see errors
uv run mypy src/

# Add type hints to fix
```

### Test Failures

**Issue**: Tests fail unexpectedly

**Solution**:
```bash
# Run with verbose output
uv run pytest -vv

# Run specific failing test
uv run pytest tests/unit/test_services.py::test_create_user -vv

# Check if fixtures are correct
```

## Related Documentation

- [Getting Started](getting-started.md) - Initial setup
- [Adding Features](adding-features.md) - Extending the template
- [Testing Guide](testing-guide.md) - Testing strategies
- [Architecture Overview](../architecture/overview.md) - Design patterns

## Sources

- [Git Best Practices](https://git-scm.com/book/en/v2) - Git documentation
- [pytest Documentation](https://docs.pytest.org/) - Testing guide
- [Python Testing with pytest](https://pythontest.com/pytest-book/) - Testing book
- [Conventional Commits](https://www.conventionalcommits.org/) - Commit message format
