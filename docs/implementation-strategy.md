# Implementation Strategy

This document is for contributors: how Graft is developed, coding practices, and how to add features.

## Development Setup

### Prerequisites

- Python 3.14+
- uv (for package management)
- Docker (for container transformer testing)
- Git

### Clone and Install

```bash
git clone https://github.com/your-org/graft.git
cd graft

# Create virtual environment and install with dev dependencies
uv venv
source .venv/bin/activate  # or `.venv\Scripts\activate` on Windows

# Install in editable mode with test dependencies
uv pip install -e ".[test]"
```

### Run Tests

```bash
pytest
```

### Run Linters

```bash
ruff check src/
mypy src/
```

## Project Structure

```
graft/
├── src/graft/               # Source code
│   ├── cli.py               # CLI layer (Typer commands)
│   ├── utils.py             # Shared utilities
│   ├── domain/              # Domain entities (immutable dataclasses)
│   ├── adapters/            # External interfaces (Protocol-based)
│   └── services/            # Use cases (business logic)
├── tests/                   # Black-box subprocess tests
├── examples/                # Reference artifacts for testing/demos
├── docs/                    # Documentation
└── pyproject.toml           # Package configuration
```

## Layered Architecture

Graft follows strict layered architecture:

```
CLI → Services → Adapters → Domain
```

**Rules:**
- **Domain** depends on nothing (pure Python, immutable)
- **Adapters** depend on domain
- **Services** depend on domain and adapters (via Protocol types)
- **CLI** depends on services

**Anti-pattern:** Service calling CLI, adapter importing service, domain importing anything.

## Coding Practices

### Use Type Hints

All functions and methods must have type hints:

```python
def load_config(self, path: Path) -> ArtifactConfig:
    ...
```

Use `mypy` to verify:
```bash
mypy src/
```

### Immutable Domain Objects

All domain entities are frozen dataclasses:

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class Material:
    path: str
    rev: str
    content_sha256: str
```

**Why:** Prevents accidental mutation, thread-safe, easier to reason about.

### Protocol-Based Adapters

Define interfaces with `Protocol`:

```python
from typing import Protocol

class FileSystemPort(Protocol):
    def read_file(self, path: Path) -> str: ...
    def write_file(self, path: Path, content: str) -> None: ...
```

Implement concretely:

```python
class LocalFileSystem:
    def read_file(self, path: Path) -> str:
        return path.read_text()

    def write_file(self, path: Path, content: str) -> None:
        path.write_text(content)
```

**Benefits:** Easy to mock for testing, swappable implementations.

### Explicit Dependency Injection

Services receive dependencies in `__init__`:

```python
class RunService:
    def __init__(
        self,
        config_adapter: ConfigAdapter,
        fs: LocalFileSystem,
        material_loader: LocalMaterialLoader,
        container_adapter: DockerAdapter
    ):
        self.config_adapter = config_adapter
        self.fs = fs
        self.material_loader = material_loader
        self.container_adapter = container_adapter
```

CLI wires up dependencies:

```python
# In cli.py
fs = LocalFileSystem()
config_adapter = ConfigAdapter(fs)
material_loader = LocalMaterialLoader(fs)
container_adapter = DockerAdapter()
run_service = RunService(config_adapter, fs, material_loader, container_adapter)
```

**Benefits:** Testable, explicit, clear dependencies.

### Result Objects with `.to_dict()`

Services return result objects, not raw dicts:

```python
@dataclass
class RunResult:
    artifact: str
    derivation_id: str
    success: bool
    outputs: List[str]
    error: Optional[str] = None

    def to_dict(self) -> dict:
        return {
            "artifact": self.artifact,
            "derivation_id": self.derivation_id,
            "success": self.success,
            "outputs": self.outputs,
            "error": self.error
        }
```

CLI serializes to JSON:

```python
result = run_service.run(artifact_path)
if json_out:
    print_json(result.to_dict())
```

**Benefits:** Type-safe, explicit serialization.

## Adding Features

### Example: Adding a New Command

**Step 1: Add service method**

```python
# src/graft/services/myfeature.py

class MyFeatureService:
    def __init__(self, config_adapter: ConfigAdapter, fs: LocalFileSystem):
        self.config_adapter = config_adapter
        self.fs = fs

    def do_something(self, artifact_path: Path) -> MyResult:
        # Business logic
        ...
        return MyResult(...)

@dataclass
class MyResult:
    artifact: str
    data: List[str]

    def to_dict(self) -> dict:
        return {"artifact": self.artifact, "data": self.data}
```

**Step 2: Wire up in CLI**

```python
# In cli.py

# At top: create service
my_feature_service = MyFeatureService(config_adapter, fs)

@app.command()
def mycommand(
    artifact: str,
    json_out: bool = typer.Option(False, "--json")
):
    try:
        artifact_path = _artifact_path(artifact)
        result = my_feature_service.do_something(artifact_path)

        if json_out:
            print_json(result.to_dict())
        else:
            typer.echo(f"Result: {result.data}")

    except Exception as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(code=1)
```

**Step 3: Add tests**

```python
# tests/test_mycommand.py

def test_mycommand(agile_ops_example):
    artifact = agile_ops_example / "artifacts" / "sprint-brief"

    result = subprocess.run(
        ["graft", "mycommand", str(artifact), "--json"],
        cwd=agile_ops_example,
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert "data" in data
```

**Step 4: Update documentation**

- Add to `docs/cli-reference.md`
- Add example to `docs/workflows.md` if applicable

### Example: Adding a New Adapter

**Step 1: Define protocol**

```python
# src/graft/adapters/myservice.py

from typing import Protocol

class MyServicePort(Protocol):
    def fetch_data(self, url: str) -> str: ...
```

**Step 2: Implement adapter**

```python
class MyServiceAdapter:
    def fetch_data(self, url: str) -> str:
        # Implementation (e.g., HTTP request)
        ...
        return data
```

**Step 3: Use in service**

```python
class RunService:
    def __init__(self, ..., my_service: MyServicePort):
        self.my_service = my_service

    def run(self, ...):
        data = self.my_service.fetch_data(url)
        ...
```

**Step 4: Wire up in CLI**

```python
my_service = MyServiceAdapter()
run_service = RunService(..., my_service=my_service)
```

### Example: Adding Domain Entity

**Step 1: Define immutable dataclass**

```python
# src/graft/domain/myentity.py

from dataclasses import dataclass
from typing import Tuple

@dataclass(frozen=True)
class MyEntity:
    name: str
    values: Tuple[str, ...]  # Use Tuple for immutability

    def __post_init__(self):
        # Validation if needed
        if not self.name:
            raise ValueError("name cannot be empty")
```

**Step 2: Use in services**

```python
entity = MyEntity(name="test", values=("a", "b"))
```

## Vertical Slices

Graft is developed in **vertical slices**: end-to-end features from CLI to domain.

**Slice approach:**
1. Define acceptance criteria (what should work)
2. Write failing black-box test
3. Implement:
   - Domain entities (if needed)
   - Adapter (if needed)
   - Service method
   - CLI command
4. Make tests pass
5. Refactor, document
6. Commit

**Benefits:**
- Each slice delivers user-visible value
- No partial features
- Tests guide implementation

### Current Slices (Completed)

- **Slice 0:** `graft explain` command
- **Slice 1:** `graft run` with templates
- **Slice 2:** Container transformers
- **Slice 3:** `graft status` and `graft finalize`
- **Slice 4:** `graft impact` and `graft simulate`
- **Slice 5:** DVC orchestrator integration with autosync

See `docs/roadmap/vertical-slices.md` for details.

## Testing Approach

All tests are **black-box via subprocess**. See [Testing Strategy](testing-strategy.md) for details.

**Quick summary:**
- Tests invoke `graft` CLI, no internal imports
- Assert on exit codes, JSON output, file state
- Use fixtures (copy examples to tmp_path)
- Initialize git in test fixtures

**Example:**
```python
def test_run(agile_ops_example):
    result = subprocess.run(
        ["graft", "run", "agile-ops/artifacts/sprint-brief/"],
        cwd=agile_ops_example
    )
    assert result.returncode == 0
```

## Code Review Guidelines

### What to Look For

**Architecture compliance:**
- No circular dependencies
- Layers respected (CLI → Services → Adapters → Domain)
- Explicit dependency injection

**Type safety:**
- All functions have type hints
- Mypy passes

**Immutability:**
- Domain entities are frozen dataclasses
- No mutable default arguments

**Testing:**
- Black-box tests via subprocess
- Tests verify CLI contract
- Edge cases covered

**Documentation:**
- Public CLI commands documented in `docs/cli-reference.md`
- New concepts explained in `docs/concepts.md`
- Examples updated if needed

### Approval Criteria

Before merging:
- All tests pass
- Mypy passes
- Ruff (linter) passes
- Documentation updated
- Vertical slice complete (no partial features)

## Continuous Integration

CI runs:

```bash
# Tests
pytest -v

# Type checking
mypy src/

# Linting
ruff check src/

# Coverage
pytest --cov=graft --cov-report=term-missing
```

## Release Process

1. Update version in `pyproject.toml`
2. Update `CHANGELOG.md`
3. Tag release: `git tag v1.0.0`
4. Push: `git push && git push --tags`
5. CI builds and publishes to PyPI

## Common Pitfalls

**Don't import from tests in src/** — Tests are black-box, no coupling.

**Don't bypass dependency injection** — Always inject dependencies in `__init__`, don't create them inside methods.

**Don't mutate domain objects** — They're frozen for a reason.

**Don't skip `.to_dict()`** — Services return result objects, not dicts. CLI serializes.

**Don't mix layers** — CLI doesn't call adapters directly, services don't call CLI.

**Don't write unit tests of internal functions** — Test via CLI (black-box).

## Getting Help

- **Documentation:** Read `docs/` (start with `docs/concepts.md`)
- **Examples:** Study `examples/agile-ops/`
- **Tests:** Look at `tests/` for patterns
- **Architecture:** Read `docs/architecture.md` and `docs/philosophy-of-design.md`
- **Issues:** Check GitHub issues for context

## Contributing

1. **Fork** the repository
2. **Create a branch** for your feature: `git checkout -b feature/my-feature`
3. **Implement** following the patterns above
4. **Write tests** (black-box via subprocess)
5. **Run tests and linters** locally
6. **Commit** with clear messages
7. **Push** and open a PR
8. **Respond** to review feedback

---

Welcome to the Graft project! Follow these guidelines and you'll write maintainable, well-tested code that fits the architecture.

Next: See [Testing Strategy](testing-strategy.md) for detailed testing practices.
