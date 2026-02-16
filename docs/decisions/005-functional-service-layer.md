---
status: accepted
date: 2026-01-04
---

# ADR 005: Functional Service Layer

**Deciders**: Implementation team
**Context**: Core architecture design

## Context

The service layer contains the business logic of graft. We need to decide how to structure it:

1. **Class-based services**: Services as classes with methods
2. **Functional services**: Services as pure functions
3. **Mixed approach**: Classes for stateful services, functions for stateless

## Decision

We use **pure functions for all services**. Services are modules with functions, not classes.

```python
# NOT this (class-based)
class UpgradeService:
    def __init__(self, git: GitOperations, fs: FileSystem):
        self.git = git
        self.fs = fs

    def upgrade_dependency(self, config, dep_name, to_ref):
        # Business logic here
        pass

# THIS (functional)
def upgrade_dependency(
    config: GraftConfig,
    dep_name: str,
    to_ref: str,
    git: GitOperations,
    fs: FileSystem,
    snapshot: SnapshotOperations,
) -> UpgradeResult:
    """Upgrade a dependency to a new ref."""
    # Business logic here
    pass
```

## Consequences

### Positive

- **No Hidden State**: All inputs are explicit parameters
- **Easier Testing**: Pass different arguments, no need to set up object state
- **No Mocking**: Just pass fake implementations
- **Composability**: Functions compose naturally
- **Explicit Dependencies**: Can see exactly what each function needs
- **Immutability**: Encourages immutable data structures
- **Concurrency-Safe**: Pure functions have no shared mutable state

### Negative

- **More Parameters**: Functions can have many parameters
- **Less Encapsulation**: Can't hide dependencies inside object
- **Repetitive**: Same dependencies passed to multiple functions

### Mitigation

- Group related parameters into domain models (Config, DependencyContext)
- Keep functions focused - single responsibility
- Use context objects when many parameters are always used together

## Rationale

### 1. Testability

```python
# Testing class-based services
def test_upgrade():
    git = FakeGit()
    fs = FakeFS()
    service = UpgradeService(git, fs)  # Setup
    service.upgrade_dependency(...)     # Test
    assert git.was_called(...)          # Verify

# Testing functional services
def test_upgrade():
    result = upgrade_dependency(
        config=...,
        dep_name="test",
        to_ref="v2.0.0",
        git=FakeGit(),  # Dependencies right here, visible
        fs=FakeFS(),
        snapshot=FakeSnapshot(),
    )
    assert result.success
```

No setup phase, no object state to manage, no "gotchas" about initialization order.

### 2. Explicit Dependencies

```python
# What does this function need? Look at parameters!
def resolve_dependencies(
    config: GraftConfig,
    git: GitOperations,
    deps_dir: str,
) -> None:
    """Clone or update all dependencies.

    Clearly needs: config, git operations, and deps directory path.
    Nothing hidden!
    """
    pass

# Compare with class method - what does it need?
class ResolutionService:
    def resolve_dependencies(self, config):
        # Needs git... but where is it? In self? In config?
        # Have to read __init__ to know!
        pass
```

### 3. No Shared Mutable State

```python
# Safe - can call from multiple threads
result1 = upgrade_dependency(config1, ...)
result2 = upgrade_dependency(config2, ...)

# Unsafe - shared mutable state
service = UpgradeService(...)
result1 = service.upgrade(config1, ...)  # Might modify service state
result2 = service.upgrade(config2, ...)  # Might see state from result1!
```

### 4. Composability

```python
# Functions compose naturally
def perform_upgrade_workflow(
    config: GraftConfig,
    dep_name: str,
    to_ref: str,
    ctx: DependencyContext,
) -> None:
    # Each function is independent, just call them
    snapshot_id = create_snapshot(ctx.snapshot, "graft.lock")

    try:
        result = upgrade_dependency(
            config, dep_name, to_ref,
            ctx.git, ctx.fs, ctx.snapshot
        )

        run_migration(result.migration, ctx.command_executor)

        delete_snapshot(ctx.snapshot, snapshot_id)
    except Exception:
        restore_snapshot(ctx.snapshot, snapshot_id)
        raise
```

## Real-World Examples

### Before (Class-Based - What We Avoided)

```python
class LockService:
    def __init__(self, lock_file: LockFile):
        self._lock_file = lock_file

    def update_lock_entry(self, path: str, dep_name: str, entry: LockEntry):
        # Hard to test - need to mock self._lock_file
        # Can't easily use different lock_file for different calls
        self._lock_file.write(path, {dep_name: entry})

# Usage
service = LockService(lock_file)
service.update_lock_entry(...)
```

### After (Functional - What We Did)

```python
def update_lock_entry(
    lock_file: LockFile,
    path: str,
    dep_name: str,
    entry: LockEntry,
) -> None:
    """Update a single lock entry."""
    current = lock_file.read(path)
    updated = {**current, dep_name: entry}
    lock_file.write(path, updated)

# Usage - just call it
update_lock_entry(lock_file, "graft.lock", "my-dep", entry)

# Testing - pass a fake
update_lock_entry(FakeLockFile(), "graft.lock", "my-dep", entry)
```

## Context Objects for Ergonomics

While services are pure functions, we use context objects to group commonly-used dependencies:

```python
@dataclass(frozen=True)
class DependencyContext:
    """Grouped dependencies for ergonomics."""
    git: GitOperations
    fs: FileSystem
    snapshot: SnapshotOperations
    lock_file: LockFile
    command_executor: CommandExecutor
    deps_directory: str

# Then services can accept the context
def resolve_dependencies(
    config: GraftConfig,
    ctx: DependencyContext,
) -> None:
    """Resolve all dependencies.

    Context bundles commonly-used dependencies.
    Still explicit - ctx makes dependencies visible.
    """
    for dep in config.dependencies.values():
        ctx.git.clone(...)
```

This balances explicitness with ergonomics.

## Comparison with Alternatives

### Class-Based Services

**Pros**: Familiar OOP pattern, encapsulation
**Cons**: Hidden dependencies, harder to test, mutable state
**Rejected**: Functional approach is simpler and more testable

### Mixed Approach

**Pros**: Use classes only when needed
**Cons**: Inconsistent patterns, harder to understand when to use which
**Rejected**: Consistency is valuable

## Exceptions to the Rule

**Domain Models**: These ARE classes (dataclasses), but they're immutable value objects, not services.

```python
@dataclass(frozen=True)
class Change:
    """Domain model - NOT a service."""
    ref: str
    type: ChangeType
    description: str
    migration: str | None = None
    verify: str | None = None
```

## Benefits Realized

1. **Zero mocking in tests**: All tests use fakes or direct function calls
2. **Clear test setup**: Every test explicitly shows what it needs
3. **Easy debugging**: No hidden state, all inputs visible
4. **Simple refactoring**: Change function signature = compiler tells you what to update

## Implementation Patterns

### Service Module Structure
```python
# src/graft/services/upgrade_service.py

"""Upgrade service - pure functions for dependency upgrades."""

def upgrade_dependency(...) -> UpgradeResult:
    """Upgrade a dependency to a new ref."""
    pass

def validate_upgrade_path(...) -> list[ValidationError]:
    """Validate that an upgrade path is safe."""
    pass

def get_changes_between(...) -> list[Change]:
    """Get all changes between two refs."""
    pass
```

### Service Tests
```python
# tests/unit/test_upgrade_service.py

def test_upgrade_dependency_success():
    """Should upgrade dependency and return success result."""
    result = upgrade_service.upgrade_dependency(
        config=make_config(),
        dep_name="test-dep",
        to_ref="v2.0.0",
        git=FakeGit(),
        fs=FakeFS(),
        snapshot=FakeSnapshot(),
    )
    assert result.success
    assert result.new_ref == "v2.0.0"
```

## Related Decisions

- See ADR 004: Protocol-Based Dependency Injection (complements this)
- Immutable domain models support this approach

## References

- Implementation: `src/graft/services/`
- Tests: `tests/unit/test_*_service.py`
- Discussion: "Functional Core, Imperative Shell" pattern (Gary Bernhardt)
