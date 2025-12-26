---
title: "ADR 002: src Layout for Project Structure"
status: stable
owners: [architecture-team]
---

# ADR 002: src Layout for Project Structure

## Status

Accepted

## Context

Python projects can be organized in two main ways:

1. **Flat layout**: Package directly in root
2. **src layout**: Package under `src/` directory

We need to choose a structure that:
- Prevents common packaging errors
- Provides clear boundaries
- Follows modern best practices
- Works well with testing

## Decision

Use **src layout** with the package under `src/starter/`.

## Rationale

### Project Structure

**src layout**:
```
python-starter/
├── src/
│   └── starter/         # Package code
│       ├── __init__.py
│       ├── domain/
│       ├── services/
│       └── ...
├── tests/               # Test code
├── docs/                # Documentation
├── pyproject.toml
└── README.md
```

**Flat layout** (not used):
```
python-starter/
├── starter/             # Package code (in root)
│   ├── __init__.py
│   ├── domain/
│   └── ...
├── tests/
├── docs/
├── pyproject.toml
└── README.md
```

### Prevents Accidental Imports

**Problem with flat layout**:

When developing, Python can accidentally import from the source tree instead of the installed package:

```bash
# With flat layout
$ python -m pytest tests/
# ⚠️  Imports starter/ from source tree (not installed package)

$ python
>>> import starter  # Imports from ./starter/ (development copy)
```

This can hide packaging issues:
- Missing files in `MANIFEST.in`
- Incorrect `pyproject.toml` configuration
- Import errors that only appear after installation

**src layout prevents this**:

```bash
# With src layout
$ python -m pytest tests/
# ✅ Must install package first (imports from installed location)

$ python
>>> import starter  # Must install first, or error
```

If the package isn't installed, you get a clear error immediately.

### Forces Proper Installation

src layout **forces** you to install your package during development:

```bash
# Install in editable mode
uv pip install -e .

# Now imports work correctly
python -m pytest tests/
```

**Benefits**:
- Tests run against installed package (like production)
- Catches packaging problems early
- Ensures `pyproject.toml` is correct
- Prevents "works on my machine" issues

### Testing Best Practice

**pytest best practice**: Test the **installed** package, not source tree.

src layout enforces this:

```python
# tests/test_example.py

# ✅ Imports from installed package
from starter.services import create_example

# With flat layout, might accidentally import from:
# ./starter/services.py (source tree)
# instead of installed package
```

### Clear Boundaries

src layout provides **visual separation**:

- `src/` = **production code**
- `tests/` = **test code**
- `docs/` = **documentation**
- `scripts/` = **development scripts**

This makes it immediately clear what's shipped vs what's development-only.

### Modern Standard

src layout is increasingly the **recommended approach**:

- Python Packaging Authority recommends it
- Used by major projects (pytest, requests, etc.)
- Supported by all modern tools (uv, hatch, Poetry)
- Documented in Python Packaging User Guide

## Alternatives Considered

### Flat Layout

**Structure**:
```
project/
├── package/          # Code in root
├── tests/
└── pyproject.toml
```

**Pros**:
- Simpler directory structure
- One less level of nesting
- Familiar to many developers
- Works for small scripts

**Cons**:
- Can import from source tree accidentally
- Doesn't enforce installation
- Hides packaging problems
- Mixes production and dev code

**Why not chosen**: The accidental import problem is significant and src layout prevents it.

### Src Layout with Package Name Mismatch

Some projects use:
```
src/
└── my_package/     # Different from project name
```

for project `my-project`.

**Pros**:
- Can have project name != package name
- Flexibility in naming

**Cons**:
- Confusing (two names to remember)
- Less discoverable
- Can cause confusion

**Why not chosen**: We use consistent naming (`python-starter` project, `starter` package).

### Namespace Packages

For large organizations:
```
src/
└── company/
    └── product/
```

**Pros**:
- Prevents package name collisions
- Good for large organizations
- Multiple packages under one namespace

**Cons**:
- More complex setup
- Overkill for single projects
- Extra nesting

**Why not chosen**: Not needed for standalone starter template.

## Consequences

### Positive

1. **Prevents import errors**: Can't accidentally import from source tree
2. **Forces installation**: Ensures package is properly installable
3. **Catches issues early**: Packaging problems surface during development
4. **Clear structure**: Visual separation of code types
5. **Modern standard**: Aligns with community best practices

### Negative

1. **Extra nesting**: One more directory level
2. **Must install**: Can't just `python -m starter` without installing
3. **Initial confusion**: Developers unfamiliar with pattern need to learn it

### Mitigations

1. **Editable install**: `uv pip install -e .` for development
2. **Documentation**: Clear instructions in README and getting started guide
3. **Scripts**: Provide `uv run` commands that handle installation

## Implementation

### Package Structure

```
src/
└── starter/              # Package name
    ├── __init__.py       # Package initialization
    ├── __main__.py       # python -m starter entry point
    ├── domain/           # Domain layer
    ├── services/         # Service layer
    ├── adapters/         # Adapter layer
    ├── protocols/        # Protocol definitions
    └── cli/              # CLI layer
```

### pyproject.toml Configuration

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "python-starter"
# ... other config

# hatchling automatically discovers packages in src/
# No explicit package configuration needed
```

### Development Workflow

```bash
# Install in editable mode (once)
uv pip install -e .

# Now imports work
python -m starter
python -c "import starter"

# Tests work
uv run pytest

# Scripts work
uv run starter-cli
```

### Testing

```python
# tests/test_services.py

# Imports from installed package (under src/)
from starter.services import create_example
from starter.domain.entities import Entity

def test_create_example():
    # Tests the installed package
    entity = create_example(...)
```

## Package Discovery

Modern build tools (hatchling, setuptools) **automatically discover** packages in `src/`:

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

# No need to specify packages explicitly
# hatchling finds src/starter/ automatically
```

If using setuptools:
```toml
[tool.setuptools]
package-dir = {"" = "src"}

[tool.setuptools.packages.find]
where = ["src"]
```

## Related Decisions

- [ADR 001: uv](001-why-uv.md) - Package manager that works well with src layout
- [Architecture Overview](../architecture/overview.md) - Overall project structure

## Sources

- [src layout vs flat layout - Python Packaging Guide](https://packaging.python.org/en/latest/discussions/src-layout-vs-flat-layout/) - Official guidance
- [Testing & Packaging - Hynek Schlawack](https://hynek.me/articles/testing-packaging/) - Detailed explanation
- [Python Packaging Best Practices](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/) - Modern packaging
- [pytest Good Integration Practices](https://docs.pytest.org/en/stable/explanation/goodpractices.html) - Testing recommendations
