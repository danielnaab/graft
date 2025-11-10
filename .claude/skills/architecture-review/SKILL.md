# Skill: Architecture Review

## Purpose
Review code changes to ensure they follow Graft's layered architecture pattern and maintain separation of concerns.

## When to Use
Activate when the user mentions:
- Code review
- Architecture review
- Checking if code follows patterns
- Reviewing a PR
- Ensuring architecture compliance
- Before finalizing changes

## Review Checklist

### Layer Separation (ADR-0002)
- [ ] **Domain layer** contains only pure entities and value objects
- [ ] **Adapters** handle all external interactions (filesystem, YAML, etc.)
- [ ] **Services** orchestrate business logic without I/O
- [ ] **CLI** is thin presentation layer only

### Domain Layer (`src/graft/domain/`)
Check for:
- ✅ Immutable dataclasses with `@dataclass(frozen=True)`
- ✅ No I/O operations (no file reads, no network)
- ✅ No external dependencies (no imports from adapters/services)
- ✅ Type hints on all fields
- ❌ Business logic (should be in services)
- ❌ File operations
- ❌ Mutable state

### Adapter Layer (`src/graft/adapters/`)
Check for:
- ✅ Protocol-based interfaces (abstract ports)
- ✅ Concrete implementations
- ✅ Handling external interactions (files, parsing)
- ✅ No business logic (just read/write/convert)
- ❌ Direct CLI interaction
- ❌ Business rules (should be in services)
- ❌ Importing from services layer

### Service Layer (`src/graft/services/`)
Check for:
- ✅ Explicit dependency injection in `__init__`
- ✅ Business logic and use case orchestration
- ✅ Return result objects with `.to_dict()` method
- ✅ No direct I/O (use injected adapters)
- ❌ File operations (should use filesystem adapter)
- ❌ YAML parsing (should use config adapter)
- ❌ CLI/presentation logic
- ❌ Global state or singletons

### CLI Layer (`src/graft/cli.py`)
Check for:
- ✅ Thin presentation layer
- ✅ Dependency wiring at module level
- ✅ Exception mapping to exit codes
- ✅ Output formatting (JSON vs human-readable)
- ✅ Proper exit codes: 0 (success), 1 (user error), 2 (system error)
- ❌ Business logic (should be in services)
- ❌ Direct file operations (should use services)
- ❌ Complex logic beyond orchestration

## Code Quality Standards

### Type Hints
All functions should have type hints:
```python
# Good
def do_something(path: Path, config: GraftConfig) -> MyResult:
    ...

# Bad
def do_something(path, config):
    ...
```

### Dependency Injection
Services should receive dependencies explicitly:
```python
# Good
class MyService:
    def __init__(self, fs: FileSystemPort, config: ConfigAdapter):
        self.fs = fs
        self.config = config

# Bad
class MyService:
    def __init__(self):
        self.fs = LocalFileSystem()  # Hard-coded dependency
```

### Error Handling
Proper exception propagation:
```python
# CLI layer maps exceptions to exit codes
try:
    result = service.execute(path)
    print_json(result.to_dict())
except typer.BadParameter as e:
    typer.echo(f"Error: {e}", err=True)
    raise typer.Exit(code=1)
except (FileNotFoundError, yaml.YAMLError) as e:
    typer.echo(f"Error: {e}", err=True)
    raise typer.Exit(code=1)
except Exception as e:
    typer.echo(f"System error: {e}", err=True)
    raise typer.Exit(code=2)
```

## Common Anti-Patterns

### ❌ Skip Layers
```python
# Bad: CLI directly reads files
@app.command()
def my_command(artifact: str):
    yaml_path = Path(artifact) / "graft.yaml"
    data = yaml.safe_load(yaml_path.read_text())  # Should use service
```

### ❌ Business Logic in CLI
```python
# Bad: Logic in CLI layer
@app.command()
def explain(artifact: str):
    config = load_config(artifact)
    # ... 50 lines of processing logic ...  # Should be in service
    print_json(result)
```

### ❌ I/O in Domain Layer
```python
# Bad: File operations in entity
@dataclass
class Artifact:
    path: Path

    def load_config(self):
        return yaml.safe_load((self.path / "graft.yaml").read_text())  # NO!
```

### ❌ Mutable Domain Objects
```python
# Bad: Mutable entity
@dataclass
class GraftConfig:
    graft: str  # Can be modified after creation

# Good: Immutable
@dataclass(frozen=True)
class GraftConfig:
    graft: str  # Cannot be modified
```

## Review Process

### 1. Identify Changes
- Read modified files
- Understand the feature/fix being implemented
- Check which layers are affected

### 2. Verify Layer Compliance
- Check each layer follows its responsibilities
- Verify no layers are skipped
- Ensure dependencies flow correctly (inward)

### 3. Check Code Quality
- Type hints present
- Error handling appropriate
- Tests cover the changes
- No duplicate code

### 4. Validate Tests
- Black-box subprocess tests
- No internal imports in tests
- Tests verify CLI contract
- Edge cases covered

### 5. Provide Feedback
- Specific issues with file:line references
- Suggest refactorings if needed
- Highlight good patterns
- Note any architectural concerns

## Outputs
- Detailed review of architecture compliance
- Specific issues with locations
- Suggestions for improvements
- Approval or requested changes with rationale
