---
title: Getting Started
status: stable
owners: [documentation-team]
---

# Getting Started

This guide will help you get the Python Starter Template up and running on your local machine.

## Prerequisites

Before you begin, ensure you have the following installed:

### Required

- **Python 3.11 or higher**
  ```bash
  python --version  # Should show 3.11.x or higher
  ```

- **uv** (package manager)
  ```bash
  # Install via standalone installer (recommended)
  curl -LsSf https://astral.sh/uv/install.sh | sh

  # Or via pip
  pip install uv

  # Or via homebrew (macOS)
  brew install uv

  # Verify installation
  uv --version
  ```

### Optional

- **Git** (for version control)
- **VS Code** or **PyCharm** (recommended editors)

## Installation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd python-starter
```

### 2. Install Dependencies

```bash
# Install all dependencies (including dev dependencies)
uv sync

# This will:
# - Create a virtual environment (.venv)
# - Install all dependencies from pyproject.toml
# - Generate uv.lock for reproducible installs
```

### 3. Verify Installation

```bash
# Check that CLI works
uv run starter-cli --help

# Expected output:
# Usage: starter-cli [OPTIONS] COMMAND [ARGS]...
#
# Commands:
#   create  Create example entity
#   list    List all entities
```

## Project Structure Tour

After installation, you'll see this structure:

```
python-starter/
├── .venv/                   # Virtual environment (created by uv)
├── src/
│   └── starter/             # Main package
│       ├── domain/          # Domain entities and value objects
│       ├── services/        # Service functions
│       ├── protocols/       # Protocol interfaces
│       ├── adapters/        # Adapter implementations
│       └── cli/             # CLI commands
├── tests/                   # Test suite
│   ├── unit/                # Unit tests
│   ├── integration/         # Integration tests
│   └── fakes/               # Fake implementations
├── docs/                    # Documentation
├── pyproject.toml           # Project configuration
├── uv.lock                  # Dependency lockfile
└── README.md                # Project overview
```

## Running the CLI

The template includes a command-line interface to demonstrate the patterns.

### Available Commands

```bash
# Get help
uv run starter-cli --help

# Create an entity
uv run starter-cli create "example-name" 100

# List all entities
uv run starter-cli list

# Get help for specific command
uv run starter-cli create --help
```

### Example Session

```bash
$ uv run starter-cli create "first-item" 42
Created entity: <uuid>
Name: first-item
Value: 42

$ uv run starter-cli create "second-item" 100
Created entity: <uuid>
Name: second-item
Value: 100

$ uv run starter-cli list
Entities:
- <uuid-1>: first-item (value=42)
- <uuid-2>: second-item (value=100)
```

## Running Tests

The template includes comprehensive tests demonstrating testing patterns.

### Run All Tests

```bash
# Run all tests
uv run pytest

# Run with verbose output
uv run pytest -v

# Run with coverage
uv run pytest --cov=starter --cov-report=term-missing
```

### Run Specific Tests

```bash
# Run unit tests only
uv run pytest tests/unit/

# Run integration tests only
uv run pytest tests/integration/

# Run specific test file
uv run pytest tests/unit/test_services.py

# Run specific test function
uv run pytest tests/unit/test_services.py::test_create_example
```

### Expected Output

```bash
$ uv run pytest
======================== test session starts =========================
collected 15 items

tests/unit/test_domain.py ......                              [ 40%]
tests/unit/test_services.py .........                         [100%]

========================= 15 passed in 0.23s =========================
```

## Code Quality Checks

The template uses **ruff** for linting and formatting.

### Check Code

```bash
# Lint code (check for issues)
uv run ruff check .

# Format code
uv run ruff format .

# Run both
uv run ruff check --fix . && uv run ruff format .
```

### Fix Issues Automatically

```bash
# Auto-fix linting issues
uv run ruff check --fix .

# Format all files
uv run ruff format .
```

## Development Workflow

### Typical Development Session

```bash
# 1. Sync dependencies (if pyproject.toml changed)
uv sync

# 2. Make your changes to source code

# 3. Run tests
uv run pytest

# 4. Lint and format
uv run ruff check --fix .
uv run ruff format .

# 5. Run CLI to test manually
uv run starter-cli create "test" 123

# 6. Commit your changes
git add .
git commit -m "Add new feature"
```

### Adding Dependencies

```bash
# Add production dependency
uv add package-name

# Add development dependency
uv add --dev package-name

# Example: Add requests library
uv add requests

# This will:
# - Add to pyproject.toml
# - Update uv.lock
# - Install in virtual environment
```

## IDE Setup

### VS Code

Create `.vscode/settings.json`:

```json
{
  "python.defaultInterpreterPath": "${workspaceFolder}/.venv/bin/python",
  "ruff.enable": true,
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll": true,
    "source.organizeImports": true
  },
  "[python]": {
    "editor.defaultFormatter": "charliermarsh.ruff"
  }
}
```

Install extensions:
- **Python** (Microsoft)
- **Ruff** (Astral Software)

### PyCharm

1. Go to **Preferences** → **Project** → **Python Interpreter**
2. Select the `.venv/bin/python` interpreter
3. Install **Ruff plugin** from marketplace
4. Configure Ruff as external tool for formatting

## Troubleshooting

### Virtual Environment Not Activated

**Issue**: Commands fail with "module not found"

**Solution**:
```bash
# uv run automatically uses the virtual environment
uv run starter-cli --help

# Or activate manually
source .venv/bin/activate  # Linux/macOS
.venv\Scripts\activate     # Windows
```

### Dependencies Out of Sync

**Issue**: Import errors after pulling changes

**Solution**:
```bash
# Re-sync dependencies
uv sync

# Force reinstall
rm -rf .venv
uv sync
```

### Tests Failing

**Issue**: Tests fail after setup

**Solution**:
```bash
# Ensure dependencies are installed
uv sync

# Install package in editable mode
uv pip install -e .

# Run tests
uv run pytest
```

### CLI Not Found

**Issue**: `starter-cli` command not found

**Solution**:
```bash
# Use uv run
uv run starter-cli --help

# Or install package
uv pip install -e .

# Then use directly
starter-cli --help
```

## Next Steps

Now that you have the template running, explore:

1. **[Development Workflow](development-workflow.md)** - Day-to-day development practices
2. **[Adding Features](adding-features.md)** - How to extend the template
3. **[Architecture Overview](../architecture/overview.md)** - Understand the design
4. **[Testing Guide](testing-guide.md)** - Write effective tests
5. **[CLI Usage](cli-usage.md)** - Command-line interface reference

## Quick Reference

```bash
# Dependencies
uv sync                          # Install/update dependencies
uv add package-name              # Add dependency

# Testing
uv run pytest                    # Run all tests
uv run pytest -v                 # Verbose output
uv run pytest --cov=starter      # With coverage

# Code Quality
uv run ruff check .              # Lint code
uv run ruff format .             # Format code

# CLI
uv run starter-cli --help        # CLI help
uv run starter-cli create "x" 1  # Create entity
uv run starter-cli list          # List entities
```

## Getting Help

- **Documentation**: Browse [docs/](../README.md)
- **Architecture**: See [architecture/](../architecture/overview.md)
- **Decisions**: Read [ADRs](../decisions/)
- **Issues**: Report bugs or ask questions via issue tracker

## Sources

- [uv Documentation](https://docs.astral.sh/uv/) - Package manager guide
- [pytest Documentation](https://docs.pytest.org/) - Testing framework
- [ruff Documentation](https://docs.astral.sh/ruff/) - Linting and formatting
- [Typer Documentation](https://typer.tiangolo.com/) - CLI framework
