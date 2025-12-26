---
title: Configuration Reference
status: stable
owners: [documentation-team]
---

# Configuration Reference

Complete reference for `pyproject.toml` and other configuration files.

## pyproject.toml

Central configuration file for the project.

### [project] - Project Metadata

```toml
[project]
name = "python-starter"              # Package name (template variable)
version = "0.1.0"                    # Semantic version
description = "..."                  # Short description
readme = "README.md"                 # README file
requires-python = ">=3.11"           # Minimum Python version
license = {text = "MIT"}             # License
authors = [{name = "...", email = "..."}]
keywords = ["template", "starter"]   # PyPI keywords
classifiers = [                      # PyPI classifiers
    "Development Status :: 3 - Alpha",
    "Programming Language :: Python :: 3.11",
]
```

**Key fields**:
- `name`: Package name (must match directory under `src/`)
- `version`: Follows [semantic versioning](https://semver.org/)
- `requires-python`: Minimum Python version required

### [project.dependencies] - Production Dependencies

```toml
dependencies = [
    "typer>=0.12.0",                # CLI framework
]
```

**Managing**:
```bash
# Add dependency
uv add package-name

# Add with version constraint
uv add "package-name>=1.0.0"
```

### [project.optional-dependencies] - Dev Dependencies

```toml
[project.optional-dependencies]
dev = [
    "pytest>=8.0.0",
    "pytest-cov>=4.1.0",
    "ruff>=0.8.0",
]
```

**Managing**:
```bash
# Add dev dependency
uv add --dev package-name
```

### [project.scripts] - Entry Points

```toml
[project.scripts]
starter-cli = "starter.cli.main:app"
```

**Format**: `command-name = "module.path:callable"`

**Result**: Creates `starter-cli` command that calls `app()` from `starter.cli.main`

### [build-system] - Build Configuration

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

Uses [hatchling](https://hatch.pypa.io/) for building packages.

### [tool.uv] - uv Configuration

```toml
[tool.uv]
dev-dependencies = [
    "pytest>=8.0.0",
    "pytest-cov>=4.1.0",
    "ruff>=0.8.0",
]
```

uv-specific dev dependencies (alternative to `[project.optional-dependencies]`).

### [tool.ruff] - Ruff Configuration

```toml
[tool.ruff]
line-length = 100                    # Max line length
target-version = "py311"             # Python version for rules
```

**Options**:
- `line-length`: Maximum line length (default: 88)
- `target-version`: Python version (`"py38"`, `"py39"`, `"py310"`, `"py311"`, `"py312"`)
- `exclude`: Patterns to exclude (`["migrations/", "build/"]`)

### [tool.ruff.lint] - Linting Rules

```toml
[tool.ruff.lint]
select = [
    "E",   # pycodestyle errors
    "W",   # pycodestyle warnings
    "F",   # pyflakes
    "I",   # isort (import sorting)
    "N",   # pep8-naming
    "UP",  # pyupgrade (modernize syntax)
    "B",   # flake8-bugbear (bug patterns)
    "C4",  # flake8-comprehensions
    "SIM", # flake8-simplify
]
ignore = [
    "E501",  # line too long (handled by formatter)
]
```

**Rule categories**:
- **E/W**: PEP 8 style
- **F**: Logic errors
- **I**: Import sorting
- **N**: Naming conventions
- **UP**: Modern Python syntax
- **B**: Common bugs
- **C4**: Comprehension improvements
- **SIM**: Simplification suggestions

**See**: [Ruff rules](https://docs.astral.sh/ruff/rules/)

### [tool.ruff.lint.isort] - Import Sorting

```toml
[tool.ruff.lint.isort]
known-first-party = ["starter"]      # First-party packages
```

**Options**:
- `known-first-party`: Local packages
- `section-order`: Order of import sections

### [tool.ruff.format] - Formatting

```toml
[tool.ruff.format]
quote-style = "double"               # "double" or "single"
indent-style = "space"               # "space" or "tab"
```

Black-compatible by default.

### [tool.pytest.ini_options] - Pytest Configuration

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]                # Test directories
python_files = ["test_*.py"]         # Test file pattern
python_classes = ["Test*"]           # Test class pattern
python_functions = ["test_*"]        # Test function pattern
addopts = [
    "--strict-markers",              # Only use registered markers
    "--strict-config",               # Error on config issues
    "--cov=starter",                 # Coverage for 'starter' package
    "--cov-report=term-missing",     # Show missing lines
    "--cov-report=html",             # Generate HTML report
]
```

**Common options**:
- `testpaths`: Where to find tests
- `addopts`: Default arguments to pytest
- `markers`: Register custom markers
- `filterwarnings`: Control warning behavior

### [tool.coverage.run] - Coverage Configuration

```toml
[tool.coverage.run]
source = ["src"]                     # Source directories
omit = ["tests/*", "**/__pycache__/*"]  # Exclude patterns
```

### [tool.coverage.report] - Coverage Reporting

```toml
[tool.coverage.report]
exclude_lines = [
    "pragma: no cover",              # Explicit exclusion
    "def __repr__",                  # __repr__ methods
    "raise AssertionError",          # Assertions
    "raise NotImplementedError",     # Abstract methods
    "if __name__ == .__main__.:",    # Main blocks
    "if TYPE_CHECKING:",             # Type checking blocks
]
```

## .python-version

Specifies Python version for uv:

```
3.11
```

**Format**: `MAJOR.MINOR` or `MAJOR.MINOR.PATCH`

**Usage**: `uv` automatically uses this version

## .gitignore

Git ignore patterns (see file for complete list).

**Key patterns**:
```gitignore
__pycache__/
*.pyc
.venv/
uv.lock
.pytest_cache/
.coverage
htmlcov/
```

## Environment Variables

### Common Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `ENVIRONMENT` | Environment name | `development`, `test`, `production` |
| `DEBUG` | Debug mode | `true`, `false` |
| `LOG_LEVEL` | Logging level | `DEBUG`, `INFO`, `WARNING`, `ERROR` |
| `DATABASE_URL` | Database connection | `postgresql://localhost/db` |

### Using .env File

Create `.env` (gitignored):

```bash
ENVIRONMENT=development
DEBUG=true
LOG_LEVEL=DEBUG
DATABASE_URL=postgresql://localhost/starter_dev
```

Load with python-dotenv:

```python
from dotenv import load_dotenv
load_dotenv()

import os
db_url = os.getenv("DATABASE_URL")
```

## Configuration Best Practices

### 1. Use pyproject.toml

Centralize configuration in `pyproject.toml`:
- ✅ One file for all tools
- ✅ Standard format
- ❌ Avoid separate config files (`setup.py`, `setup.cfg`, etc.)

### 2. Pin Dependencies

```toml
dependencies = [
    "typer>=0.12.0,<1.0",           # Specify version range
]
```

### 3. Semantic Versioning

Follow [semver](https://semver.org/):
- `MAJOR.MINOR.PATCH`
- `0.1.0` → `0.2.0` (new features)
- `0.2.0` → `0.2.1` (bug fixes)
- `0.x.y` → `1.0.0` (stable API)

### 4. Environment-Specific Config

```python
from pathlib import Path

class Config:
    def __init__(self):
        self.environment = os.getenv("ENVIRONMENT", "development")
        self.debug = os.getenv("DEBUG", "false").lower() == "true"

    @classmethod
    def from_env(cls):
        return cls()
```

## Customizing Configuration

### Adding New Tool

```toml
[tool.mypy]
python_version = "3.11"
strict = true
warn_return_any = true
```

### Adding Custom Scripts

```toml
[project.scripts]
my-command = "starter.module:function"
```

### Adding Test Markers

```toml
[tool.pytest.ini_options]
markers = [
    "slow: marks tests as slow",
    "integration: marks integration tests",
]
```

## Related Documentation

- [Tooling Reference](tooling.md) - Tool usage
- [Development Workflow](../guides/development-workflow.md) - Configuration in practice
- [ADR 001: uv](../decisions/001-why-uv.md) - Package manager choice

## Sources

- [pyproject.toml Specification](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/)
- [uv Documentation](https://docs.astral.sh/uv/)
- [ruff Configuration](https://docs.astral.sh/ruff/configuration/)
- [pytest Configuration](https://docs.pytest.org/en/stable/reference/customize.html)
