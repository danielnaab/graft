# ADR 004: Protocol-Based Dependency Injection

**Status**: Accepted
**Date**: 2026-01-04
**Deciders**: Implementation team
**Context**: Core architecture design

## Context

Graft needs a dependency injection strategy to enable:
- Testability (swapping real implementations with fakes)
- Flexibility (different implementations for different contexts)
- Clean architecture (decoupling services from infrastructure)

Several approaches were considered:

1. **Class-based inheritance**: Abstract base classes with concrete implementations
2. **Protocol-based (structural typing)**: Python 3.8+ Protocols
3. **Dependency injection framework**: Third-party DI containers
4. **No abstraction**: Direct instantiation everywhere

## Decision

We use **Protocol-based dependency injection** with structural typing (PEP 544).

```python
# Protocol definition
class GitOperations(Protocol):
    def clone(self, url: str, destination: str, ref: str) -> None: ...
    def fetch(self, repo_path: str, ref: str) -> None: ...
    def is_repository(self, path: str) -> bool: ...

# Production implementation
class SubprocessGitOperations:
    def clone(self, url: str, destination: str, ref: str) -> None:
        subprocess.run(["git", "clone", ...])
    # ... implements all methods

# Test implementation (no inheritance needed!)
class FakeGitOperations:
    def clone(self, url: str, destination: str, ref: str) -> None:
        self._repos[destination] = (url, ref)
    # ... implements all methods

# Service uses protocol
def resolve_dependencies(
    config: GraftConfig,
    git: GitOperations,  # Any type satisfying the protocol
    deps_dir: str,
) -> None:
    for dep in config.dependencies.values():
        git.clone(dep.git_url.value, f"{deps_dir}/{dep.name}", dep.git_ref.value)
```

## Consequences

### Positive

- **Duck Typing**: Implementations don't need to explicitly inherit
- **No Runtime Overhead**: Protocols are checked at type-check time only
- **Simpler Testing**: Fakes don't need complex inheritance hierarchies
- **Better IDE Support**: Type checkers understand protocols perfectly
- **Pythonic**: Uses Python's structural subtyping naturally
- **Clear Contracts**: Protocol definitions document the interface

### Negative

- **Python 3.8+ Required**: Older Python versions don't support Protocols
- **Less Familiar**: Some developers expect class-based OOP

## Rationale

### 1. Testability Without Inheritance

```python
# No need for this complexity:
class GitOperations(ABC):
    @abstractmethod
    def clone(...): pass

class SubprocessGit(GitOperations):
    def clone(...): pass

class FakeGit(GitOperations):
    def clone(...): pass

# Just this:
class SubprocessGit:  # No inheritance
    def clone(...): pass

class FakeGit:  # No inheritance
    def clone(...): pass

# Both satisfy GitOperations protocol automatically
```

### 2. Pythonic Design

Python favors "duck typing" - if it walks like a duck and quacks like a duck, it's a duck. Protocols formalize this in a type-safe way.

### 3. Clean Separation

```
Protocols (src/graft/protocols/)
    ↓ define interfaces
Services (src/graft/services/)
    ↓ use protocols
Adapters (src/graft/adapters/)
    ↓ implement protocols
Fakes (tests/fakes/)
    ↓ implement protocols
```

No circular dependencies, clear boundaries.

### 4. Real-World Example

```python
# Protocol in src/graft/protocols/git.py
class GitOperations(Protocol):
    def clone(self, url: str, destination: str, ref: str) -> None: ...

# Real implementation in src/graft/adapters/git.py
class SubprocessGitOperations:
    def clone(self, url: str, destination: str, ref: str) -> None:
        # Actually runs git
        result = subprocess.run(["git", "clone", ...])

# Fake in tests/fakes/fake_git.py
class FakeGitOperations:
    def __init__(self):
        self._cloned_repos = {}

    def clone(self, url: str, destination: str, ref: str) -> None:
        # Just records the call
        self._cloned_repos[destination] = (url, ref)

    # Test helpers
    def was_cloned(self, url, dest, ref) -> bool:
        return (url, dest, ref) in self._cloned_calls

# Service in src/graft/services/resolution_service.py
def resolve_dependencies(
    config: GraftConfig,
    git: GitOperations,  # Accepts ANY implementation!
    deps_dir: str,
) -> None:
    for dep in config.dependencies.values():
        git.clone(...)  # Works with real OR fake
```

## Comparison with Alternatives

### Class-Based Inheritance

**Pros**: Familiar to OOP developers
**Cons**: Requires inheritance, more boilerplate, runtime overhead
**Rejected**: Less Pythonic, unnecessary complexity

### DI Framework (e.g., dependency-injector, inject)

**Pros**: Automatic wiring, less manual passing
**Cons**: Third-party dependency, magic behavior, harder to debug
**Rejected**: Over-engineered for our needs

### No Abstraction

**Pros**: Simple, direct
**Cons**: Untestable, tightly coupled
**Rejected**: Makes testing impossible

## Implementation Patterns

### Protocol Definition
```python
# src/graft/protocols/filesystem.py
class FileSystem(Protocol):
    def read_text(self, path: str) -> str: ...
    def write_text(self, path: str, content: str) -> None: ...
```

### Production Implementation
```python
# src/graft/adapters/filesystem.py
class OsFileSystem:
    def read_text(self, path: str) -> str:
        return Path(path).read_text()

    def write_text(self, path: str, content: str) -> None:
        Path(path).write_text(content)
```

### Test Fake
```python
# tests/fakes/fake_filesystem.py
class FakeFileSystem:
    def __init__(self):
        self._files = {}

    def read_text(self, path: str) -> str:
        return self._files.get(path, "")

    def write_text(self, path: str, content: str) -> None:
        self._files[path] = content

    # Test helper
    def reset(self):
        self._files.clear()
```

### Service Usage
```python
# src/graft/services/config_service.py
def load_config(fs: FileSystem, path: str) -> Config:
    content = fs.read_text(path)  # Works with real OR fake
    return parse_yaml(content)
```

## Benefits Realized

1. **100+ unit tests** use fakes instead of mocks - clearer and more maintainable
2. **No mocking framework** needed - fakes are simple Python classes
3. **Fast tests** - no subprocess calls, no file I/O in unit tests
4. **Clear interfaces** - protocols document exactly what each abstraction needs

## Related Decisions

- See ADR 005: Functional Service Layer
- Complements immutable domain models

## References

- PEP 544: Protocols (Structural Subtyping)
- Implementation: `src/graft/protocols/`
- Adapters: `src/graft/adapters/`
- Fakes: `tests/fakes/`
- Discussion: "Growing Object-Oriented Software, Guided by Tests" (Freeman & Pryce)
