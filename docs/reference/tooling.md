---
title: Tooling Reference
status: stable
owners: [documentation-team]
---

# Tooling Reference

Quick reference for development tools used in this template.

## uv - Package Manager

### Installation

```bash
# Standalone installer (recommended)
curl -LsSf https://astral.sh/uv/install.sh | sh

# Via pip
pip install uv

# Via homebrew
brew install uv
```

### Common Commands

```bash
# Install dependencies
uv sync

# Add package
uv add package-name
uv add package-name@version
uv add --dev package-name           # Dev dependency

# Remove package
uv remove package-name

# Update dependencies
uv lock --upgrade                   # Update lockfile
uv sync                             # Install updates

# Run command in venv
uv run python script.py
uv run pytest
uv run starter-cli

# Install package in editable mode
uv pip install -e .
```

### Configuration

In `pyproject.toml`:
```toml
[tool.uv]
dev-dependencies = [
    "pytest>=8.0.0",
]
```

**See**: [uv Documentation](https://docs.astral.sh/uv/)

## ruff - Linter & Formatter

### Installation

```bash
uv add --dev ruff
```

### Common Commands

```bash
# Check code (lint)
uv run ruff check .
uv run ruff check src/
uv run ruff check file.py

# Auto-fix issues
uv run ruff check --fix .

# Format code
uv run ruff format .
uv run ruff format src/

# Check formatting (no changes)
uv run ruff format --check .

# Both lint and format
uv run ruff check --fix . && uv run ruff format .
```

### Configuration

In `pyproject.toml`:
```toml
[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = ["E", "F", "I", "UP", "B"]
ignore = ["E501"]
```

**See**: [ruff Documentation](https://docs.astral.sh/ruff/)

## pytest - Testing Framework

### Installation

```bash
uv add --dev pytest pytest-cov
```

### Common Commands

```bash
# Run all tests
uv run pytest

# Run with verbose output
uv run pytest -v

# Run specific file
uv run pytest tests/unit/test_services.py

# Run specific test
uv run pytest tests/unit/test_services.py::test_create_example

# Run tests matching pattern
uv run pytest -k "test_create"

# Run with coverage
uv run pytest --cov=starter

# Coverage with HTML report
uv run pytest --cov=starter --cov-report=html

# Stop on first failure
uv run pytest -x

# Run last failed tests
uv run pytest --lf

# Run in parallel (with pytest-xdist)
uv run pytest -n auto
```

### Useful Options

| Option | Purpose |
|--------|---------|
| `-v` | Verbose output |
| `-x` | Stop on first failure |
| `-s` | Show print statements |
| `-l` | Show local variables on failure |
| `-k EXPR` | Run tests matching expression |
| `-m MARKER` | Run tests with marker |
| `--lf` | Run last failed |
| `--ff` | Run failures first |
| `--cov=PKG` | Measure coverage |

**See**: [pytest Documentation](https://docs.pytest.org/)

## Typer - CLI Framework

### Installation

```bash
uv add typer
```

### Basic Usage

```python
import typer

app = typer.Typer()

@app.command()
def hello(name: str) -> None:
    """Say hello."""
    typer.echo(f"Hello {name}!")

if __name__ == "__main__":
    app()
```

**Run**: `python script.py hello World`

**See**: [Typer Documentation](https://typer.tiangolo.com/)

## git - Version Control

### Common Commands

```bash
# Status
git status

# Stage changes
git add .
git add file.py

# Commit
git commit -m "feat: add feature"

# Push
git push origin main

# Pull
git pull

# Branch
git checkout -b feature/my-feature
git branch
git checkout main

# View history
git log --oneline --graph

# View diff
git diff
git diff --staged
```

## Python - Interpreter

### Common Commands

```bash
# Check version
python --version

# Run script
python script.py

# Run module
python -m starter

# Interactive REPL
python

# Run with debugging
python -m pdb script.py
```

## Pre-commit Hooks (Optional)

### Installation

```bash
uv add --dev pre-commit

# Install hooks
uv run pre-commit install
```

### Configuration

Create `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.8.0
    hooks:
      - id: ruff
        args: [--fix]
      - id: ruff-format
```

**Usage**: Runs automatically on `git commit`

## IDE Integration

### VS Code

**Extensions**:
- Python (Microsoft)
- Ruff (Astral Software)
- Pylance (Microsoft)

**Settings** (`.vscode/settings.json`):
```json
{
  "python.defaultInterpreterPath": "${workspaceFolder}/.venv/bin/python",
  "ruff.enable": true,
  "editor.formatOnSave": true,
  "[python]": {
    "editor.defaultFormatter": "charliermarsh.ruff"
  }
}
```

### PyCharm

**Configuration**:
1. Set interpreter to `.venv/bin/python`
2. Install Ruff plugin
3. Configure external tools for ruff

## CI/CD Tools

### GitHub Actions

Example workflow (`.github/workflows/test.yml`):

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh
      - name: Test
        run: |
          uv sync
          uv run pytest
          uv run ruff check .
```

## Quick Reference

### Daily Workflow

```bash
# 1. Sync dependencies
uv sync

# 2. Make changes
# ...

# 3. Run tests
uv run pytest

# 4. Lint and format
uv run ruff check --fix .
uv run ruff format .

# 5. Commit
git add .
git commit -m "feat: add feature"
```

### One-Liner Commands

```bash
# Install everything
uv sync && uv pip install -e .

# Full check
uv run pytest && uv run ruff check . && uv run ruff format --check .

# Clean generated files
rm -rf .pytest_cache __pycache__ .coverage htmlcov .ruff_cache
```

## Related Documentation

- [Configuration Reference](configuration.md) - Tool configuration
- [Development Workflow](../guides/development-workflow.md) - Daily usage
- [Getting Started](../guides/getting-started.md) - Setup instructions

## Sources

- [uv Documentation](https://docs.astral.sh/uv/)
- [ruff Documentation](https://docs.astral.sh/ruff/)
- [pytest Documentation](https://docs.pytest.org/)
- [Typer Documentation](https://typer.tiangolo.com/)
- [Git Documentation](https://git-scm.com/doc)
