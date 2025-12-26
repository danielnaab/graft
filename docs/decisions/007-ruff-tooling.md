---
title: "ADR 007: Ruff for Linting and Formatting"
status: stable
owners: [architecture-team]
---

# ADR 007: Ruff for Linting and Formatting

## Status

Accepted

## Context

Python projects need code quality tools for:

1. **Linting**: Check for errors, code smells, and style issues
2. **Formatting**: Automatically format code consistently
3. **Import sorting**: Organize imports

Traditionally, this required multiple tools (Flake8, Black, isort). We need a solution that is:

- Fast
- Comprehensive (linting + formatting)
- Easy to configure
- Actively maintained
- Compatible with modern Python

## Decision

Use **ruff** for both linting and formatting.

Replace Flake8, Black, isort, and related tools with single tool.

## Rationale

### Speed

ruff is **10-100x faster** than existing Python tools:

**Benchmark** (on large codebase):
- Flake8: ~60 seconds
- Black: ~5 seconds
- isort: ~10 seconds
- **ruff check**: ~0.5 seconds
- **ruff format**: ~0.3 seconds

**Why fast?** Written in Rust, highly optimized.

**Benefits**:
- Runs instantly in editor
- Fast CI/CD pipelines
- No waiting for feedback

### Unified Tool

ruff replaces multiple tools:

| Tool | Purpose | ruff Equivalent |
|------|---------|-----------------|
| Flake8 | Linting | `ruff check` |
| Black | Formatting | `ruff format` |
| isort | Import sorting | `ruff check --select I` |
| pyupgrade | Modernize syntax | `ruff check --select UP` |
| autoflake | Remove unused imports | `ruff check --select F401` |

**Before** (multiple tools):
```toml
[tool.black]
line-length = 100

[tool.isort]
profile = "black"
line_length = 100

[tool.flake8]
max-line-length = 100
extend-ignore = E203, W503
```

**After** (single tool):
```toml
[tool.ruff]
line-length = 100

[tool.ruff.lint]
select = ["E", "F", "I", "UP", "B"]
```

**Simpler configuration, single command.**

### Black-Compatible Formatting

ruff's formatter is **Black-compatible**:

```python
# Same formatting as Black
def long_function(
    parameter_1: str,
    parameter_2: int,
    parameter_3: bool,
) -> dict[str, Any]:
    """Format matches Black exactly."""
    return {
        "param1": parameter_1,
        "param2": parameter_2,
        "param3": parameter_3,
    }
```

**Benefits**:
- Easy migration from Black
- Familiar style
- No reformatting shock

### Comprehensive Linting

ruff implements **over 800 rules** from many tools:

- **Pyflakes** (F): Logic errors
- **pycodestyle** (E, W): PEP 8 style
- **McCabe** (C90): Complexity
- **isort** (I): Import sorting
- **pep8-naming** (N): Naming conventions
- **pyupgrade** (UP): Modern syntax
- **flake8-bugbear** (B): Bug patterns
- **flake8-comprehensions** (C4): Comprehension improvements
- And many more...

**Example rules**:
- F401: Unused import
- E501: Line too long
- I001: Import not sorted
- UP008: Use `super()` without arguments
- B006: Mutable default argument

### Auto-Fixing

ruff can **automatically fix** many issues:

```bash
# Check for issues
ruff check .

# Fix automatically
ruff check --fix .

# Format code
ruff format .
```

**Example auto-fixes**:
- Remove unused imports
- Sort imports
- Modernize syntax (`%` → `.format()` → `f-strings`)
- Add trailing commas
- Fix whitespace

### Modern and Active

**Development**:
- Created by Astral (makers of uv)
- Very active development
- Frequent releases
- Growing adoption
- Well-funded company

**Adoption**:
- Used by major projects (FastAPI, Pydantic, pandas)
- Recommended by Python Packaging Authority
- Integrated into VS Code, PyCharm
- GitHub Actions support

### Easy Configuration

**Minimal configuration** in `pyproject.toml`:

```toml
[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = [
    "E",   # pycodestyle errors
    "W",   # pycodestyle warnings
    "F",   # pyflakes
    "I",   # isort
    "N",   # pep8-naming
    "UP",  # pyupgrade
    "B",   # flake8-bugbear
    "C4",  # flake8-comprehensions
]
ignore = [
    "E501",  # line too long (handled by formatter)
]

[tool.ruff.lint.isort]
known-first-party = ["starter"]
```

**That's it!** No need for separate config files.

### IDE Integration

**VS Code**:
```json
{
    "ruff.enable": true,
    "editor.formatOnSave": true,
    "[python]": {
        "editor.defaultFormatter": "charliermarsh.ruff"
    }
}
```

**PyCharm**:
- Ruff plugin available
- External tool configuration
- File watcher integration

**Automatic formatting** on save.

## Alternatives Considered

### Black + Flake8 + isort

**Traditional stack**:
```bash
black .
isort .
flake8 .
```

**Pros**:
- Mature and stable tools
- Well-known in community
- Extensive documentation
- Large ecosystem

**Cons**:
- **Slow** (Python-based)
- **Three tools** to configure
- **Configuration drift** (different files)
- **Inconsistent** (tools can conflict)

**Why not chosen**: ruff is faster and simpler.

### Pylint

**Comprehensive linter**:
```bash
pylint src/
```

**Pros**:
- Very comprehensive checks
- Customizable
- Well-established

**Cons**:
- **Very slow** (most thorough = slowest)
- **Noisy** (many false positives by default)
- **Complex config** (steep learning curve)
- **No formatting**

**Why not chosen**: Too slow and complex. ruff provides good balance.

### autopep8

**Auto-formatter**:
```bash
autopep8 --in-place --recursive .
```

**Pros**:
- Fixes PEP 8 violations automatically
- Conservative (minimal changes)

**Cons**:
- **Slow**
- **No linting**
- **Less opinionated** than Black (more config needed)

**Why not chosen**: Black-style formatting is preferred, ruff provides it.

### YAPF (Google)

**Google's formatter**:
```bash
yapf --in-place --recursive .
```

**Pros**:
- Very configurable
- Used by Google

**Cons**:
- **Too configurable** (debates about style)
- **Slow**
- **No linting**

**Why not chosen**: Black-compatible formatting preferred.

### Ruff Check Only (with Black)

**Hybrid approach**:
```bash
ruff check .
black .
```

**Pros**:
- Use ruff for fast linting
- Use Black (established formatter)

**Cons**:
- Still two tools
- Two commands to run
- ruff format is Black-compatible anyway

**Why not chosen**: ruff format is Black-compatible, so why use both?

## Consequences

### Positive

1. **Faster feedback**: Near-instant linting/formatting
2. **Simpler setup**: One tool instead of many
3. **Unified config**: Single pyproject.toml section
4. **Comprehensive**: Covers many tools' rules
5. **Auto-fixing**: Many issues fixed automatically
6. **Active development**: Frequent improvements
7. **Modern**: Rust-based, built for speed

### Negative

1. **Newer tool**: Less mature than Black/Flake8 (released 2022)
2. **Potential changes**: Rapidly evolving (breaking changes possible)
3. **Rust dependency**: Can't modify source easily (not Python)
4. **Less customizable**: Fewer options than Pylint

### Mitigations

1. **Pin version**: Use specific ruff version in dependencies
2. **Review updates**: Test ruff updates before deploying
3. **Trust Astral**: Well-funded, reliable company
4. **Fallback**: Can always revert to Black/Flake8 if needed

## Implementation

### Installation

In `pyproject.toml`:

```toml
[project.optional-dependencies]
dev = [
    "ruff>=0.8.0",
    # ... other dev dependencies
]
```

Install:
```bash
uv sync
```

### Configuration

**Complete configuration** in `pyproject.toml`:

```toml
[tool.ruff]
line-length = 100
target-version = "py311"

[tool.ruff.lint]
select = [
    "E",   # pycodestyle errors
    "W",   # pycodestyle warnings
    "F",   # pyflakes
    "I",   # isort
    "N",   # pep8-naming
    "UP",  # pyupgrade
    "B",   # flake8-bugbear
    "C4",  # flake8-comprehensions
    "SIM", # flake8-simplify
]
ignore = [
    "E501",  # line too long (handled by formatter)
]

[tool.ruff.lint.isort]
known-first-party = ["starter"]

[tool.ruff.format]
quote-style = "double"
indent-style = "space"
```

### Usage

```bash
# Check code
uv run ruff check .

# Fix issues automatically
uv run ruff check --fix .

# Format code
uv run ruff format .

# Both check and format
uv run ruff check --fix . && uv run ruff format .
```

### Pre-Commit Hook (Optional)

`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.8.0
    hooks:
      - id: ruff
        args: [--fix]
      - id: ruff-format
```

### CI/CD Integration

**GitHub Actions**:

```yaml
- name: Lint with ruff
  run: |
    uv run ruff check .
    uv run ruff format --check .
```

**Check mode** ensures code is formatted without modifying.

### VS Code Integration

`.vscode/settings.json`:

```json
{
    "ruff.enable": true,
    "ruff.organizeImports": true,
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

## Best Practices

### 1. Run in CI

Ensure code passes checks:

```yaml
# .github/workflows/lint.yml
name: Lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh
      - name: Lint
        run: |
          uv run ruff check .
          uv run ruff format --check .
```

### 2. Format on Save

Configure editor to format automatically:
- VS Code: `editor.formatOnSave`
- PyCharm: File Watcher

### 3. Fix Before Committing

Add to workflow:

```bash
# Before committing
uv run ruff check --fix .
uv run ruff format .
git add -u
git commit -m "Your message"
```

### 4. Ignore Specific Lines

When needed:

```python
result = some_function()  # noqa: F841  # Unused variable OK here

# ruff: noqa: E501 (ignore line length for this line)
long_url = "https://example.com/very/long/url/that/exceeds/line/length"
```

### 5. Per-File Ignores

In `pyproject.toml`:

```toml
[tool.ruff.lint.per-file-ignores]
"__init__.py" = ["F401"]  # Allow unused imports in __init__
"tests/**" = ["S101"]     # Allow assert in tests
```

## Migration from Black/Flake8

1. **Remove old tools**:
   ```bash
   uv remove black flake8 isort
   ```

2. **Add ruff**:
   ```bash
   uv add --dev ruff
   ```

3. **Format codebase**:
   ```bash
   uv run ruff format .
   ```

4. **Fix issues**:
   ```bash
   uv run ruff check --fix .
   ```

5. **Update CI/CD** to use ruff

6. **Remove old config** sections for black, flake8, isort

## Related Decisions

- [ADR 001: uv](001-why-uv.md) - Package manager (same company)
- [Reference: Tooling](../reference/tooling.md) - Tooling documentation

## Sources

- [ruff Documentation](https://docs.astral.sh/ruff/) - Official documentation
- [ruff GitHub](https://github.com/astral-sh/ruff) - Source code
- [Astral](https://astral.sh/) - Company behind ruff and uv
- [Black Documentation](https://black.readthedocs.io/) - Formatting style
- [PEP 8](https://peps.python.org/pep-0008/) - Python style guide
