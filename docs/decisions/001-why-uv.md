---
title: "ADR 001: Package Management with uv"
status: stable
owners: [architecture-team]
---

# ADR 001: Package Management with uv

## Status

Accepted

## Context

Python projects require package and virtual environment management. We need a tool that is:

- Fast and reliable
- Handles dependencies deterministically
- Manages virtual environments
- Provides good developer experience
- Modern and actively maintained

## Decision

Use **uv** for package management and virtual environment handling.

## Rationale

### Performance

uv is written in Rust and is **10-100x faster** than pip/pip-tools:

- Dependency resolution: seconds vs minutes
- Package installation: significantly faster
- Lockfile generation: near-instant

**Benchmark example** (from uv documentation):
- pip: ~45 seconds to install pandas + dependencies
- uv: ~1 second for same operation

### Unified Tool

uv replaces multiple tools:

| Tool | Purpose | Replaced by uv |
|------|---------|----------------|
| pip | Install packages | `uv pip install` |
| pip-tools | Compile requirements | `uv lock` |
| venv | Create virtual envs | `uv venv` |
| virtualenv | Enhanced venvs | `uv venv` |

**Single command** for common workflows:
```bash
# Install dependencies
uv sync

# Add package
uv add package-name

# Update lockfile
uv lock --upgrade
```

### Modern Lockfile

uv uses a modern lockfile format (`uv.lock`):

- **Deterministic**: Same dependencies every time
- **Cross-platform**: Works on Linux, macOS, Windows
- **Fast resolution**: Lockfile resolves instantly
- **Human-readable**: TOML format

**Example**:
```toml
[[package]]
name = "typer"
version = "0.12.0"
source = { registry = "https://pypi.org/simple" }
dependencies = [
    { name = "click" },
]
```

### Better Dependency Resolution

uv has a modern resolver that:

- Handles complex dependency graphs correctly
- Finds compatible versions faster
- Provides clear error messages on conflicts
- Supports version constraints properly

### Active Development

uv is actively developed by Astral (makers of ruff):

- Frequent releases with improvements
- Growing adoption in Python community
- Well-documented
- Strong community support

## Alternatives Considered

### pip + pip-tools

**Traditional approach**:
```bash
pip install -r requirements.txt
pip-compile requirements.in
```

**Pros**:
- Industry standard
- Well-known by developers
- Works everywhere

**Cons**:
- Slow (written in Python)
- Requires multiple tools (pip + pip-tools)
- Complex workflow (requirements.in → requirements.txt)
- Dependency resolution can be slow

**Why not chosen**: Speed and developer experience matter. uv provides significant improvements.

### Poetry

**Modern alternative**:
```bash
poetry install
poetry add package-name
```

**Pros**:
- All-in-one tool (dependencies + build + publish)
- Good developer experience
- Handles virtual environments
- Modern lockfile

**Cons**:
- Slower than uv (Python-based)
- Heavyweight for simple projects
- Opinionated project structure
- Can be complex for simple needs

**Why not chosen**: uv is faster and simpler for our use case. We use hatchling for building.

### PDM

**Another modern tool**:
```bash
pdm install
pdm add package-name
```

**Pros**:
- Fast (uses pip's resolver)
- Follows PEP 582 (no virtual env needed)
- Good dependency management

**Cons**:
- Less adoption than Poetry
- Still Python-based (slower than uv)
- PEP 582 is controversial

**Why not chosen**: uv is faster and has cleaner integration with standard Python workflows.

### Conda

**Scientific Python ecosystem**:
```bash
conda install package-name
```

**Pros**:
- Great for data science
- Handles non-Python dependencies (system libraries)
- Cross-language support

**Cons**:
- Heavyweight and slow
- Large environment sizes
- Not designed for pure Python projects
- Separate ecosystem from PyPI

**Why not chosen**: Overkill for general Python projects. uv is simpler and faster.

## Consequences

### Positive

1. **Faster development**: Dependency operations take seconds, not minutes
2. **Simpler workflow**: One tool instead of several
3. **Better determinism**: Lockfile ensures reproducible installs
4. **Modern tooling**: Aligns with Python ecosystem direction
5. **Active development**: Regular improvements and bug fixes

### Negative

1. **Newer tool**: Less battle-tested than pip (released 2024)
2. **Learning curve**: Developers need to learn new commands
3. **Not universally known**: Some developers unfamiliar with uv
4. **Potential changes**: Tool is still evolving rapidly

### Mitigations

1. **Documentation**: Clear instructions in README and guides
2. **Fallback**: uv generates standard `pyproject.toml` that works with pip
3. **Easy install**: `pip install uv` or use standalone installer
4. **Similar commands**: uv commands mirror pip (`uv add` vs `pip install`)

## Implementation

### Installation

Multiple options:

```bash
# Via pip
pip install uv

# Via standalone installer (recommended)
curl -LsSf https://astral.sh/uv/install.sh | sh

# Via homebrew (macOS)
brew install uv
```

### Basic Workflow

```bash
# Create new project
uv init

# Install dependencies
uv sync

# Add dependency
uv add typer

# Add dev dependency
uv add --dev pytest

# Update dependencies
uv lock --upgrade

# Run command in venv
uv run python script.py
uv run pytest
```

### Configuration

In `pyproject.toml`:

```toml
[tool.uv]
dev-dependencies = [
    "pytest>=8.0.0",
    "ruff>=0.8.0",
]
```

### Migration from pip

If migrating from requirements.txt:

```bash
# Import existing requirements
uv pip compile requirements.txt -o pyproject.toml

# Sync environment
uv sync
```

## Review Date

This decision should be reviewed in 1 year (2026) to assess:

- uv's maturity and stability
- Community adoption
- Alternative tool improvements
- Any significant issues encountered

## Sources

- [uv Documentation](https://docs.astral.sh/uv/) - Official documentation
- [uv GitHub Repository](https://github.com/astral-sh/uv) - Source code and issues
- [uv Performance Benchmarks](https://docs.astral.sh/uv/guides/performance/) - Speed comparisons
- [Python Packaging User Guide](https://packaging.python.org/) - Packaging best practices
- [Why uv by Astral](https://astral.sh/blog/uv) - Tool announcement and rationale
