---
date: 2025-12-27
updated: 2025-12-27
status: accepted
---

# ADR 001: Error Handling Strategy

## Status

Accepted

## Context

Graft is a knowledge base dependency management tool that performs I/O operations (file reading, git cloning, network requests). We need a consistent, Pythonic, and type-safe approach to error handling that:

1. Makes errors explicit and visible to type checkers
2. Follows Python idioms and best practices
3. Supports functional programming patterns (immutability, composition)
4. Provides excellent developer experience with clear error messages
5. Enables proper error recovery and graceful degradation

### Current Implementation

We currently use a hybrid approach:

**Exceptions for exceptional cases:**
- `ValidationError` - Domain rule violations (invalid input)
- `ConfigurationError` - Invalid or missing configuration
- `DependencyResolutionError` - Git operation failures
- `EntityNotFoundError` - Missing entities

**Result pattern for expected failures:**
- `DependencyResolution` entity with `status: DependencyStatus`
- Status can be `PENDING`, `CLONING`, `RESOLVED`, or `FAILED`
- Failed resolutions include `error_message: Optional[str]`

### Design Goals

1. **Type Safety** - Errors should be visible in type signatures
2. **Pythonic** - Follow Python conventions (EAFP, exception handling)
3. **Functional** - Support composition and immutability
4. **Explicit** - Make failure modes obvious
5. **Recoverable** - Enable retry logic and fallbacks
6. **Informative** - Provide actionable error messages

## Decision

We adopt a **Hybrid Error Handling Strategy** combining exceptions with result types:

### 1. Use Exceptions for Exceptional Cases

**When to use exceptions:**
- Programmer errors (bugs, contract violations)
- Domain rule violations (validation failures)
- Unrecoverable errors (missing required config, malformed data)
- Errors that should stop execution

**Exception Hierarchy:**
```python
DomainError (base)
├── ValidationError (domain rules violated)
├── ConfigurationError (config issues)
│   ├── ConfigFileNotFoundError
│   ├── ConfigParseError
│   └── ConfigValidationError
├── DependencyResolutionError (git operations failed)
│   ├── GitCloneError
│   ├── GitFetchError
│   ├── GitAuthenticationError
│   └── GitNotFoundError
└── EntityNotFoundError (missing entity)
```

**Exception Guidelines:**
- Include structured context (not just strings)
- Use exception chaining (`raise ... from e`)
- Provide recovery suggestions in messages
- Keep exceptions immutable (frozen dataclasses)

### 2. Use Result Pattern for Expected Failures

**When to use Result pattern:**
- I/O operations that may fail (network, disk)
- Operations with multiple failure modes
- Batch operations (continue on partial failure)
- Operations where failure is a valid outcome

**Result Pattern Implementation:**
```python
@dataclass
class DependencyResolution:
    spec: DependencySpec
    status: DependencyStatus  # PENDING | CLONING | RESOLVED | FAILED
    local_path: Optional[str] = None
    error_message: Optional[str] = None

    def is_success(self) -> bool:
        return self.status == DependencyStatus.RESOLVED

    def is_failure(self) -> bool:
        return self.status == DependencyStatus.FAILED
```

**Advantages:**
- Type-safe: mypy can verify status checks
- Composable: can map/filter/fold over results
- Explicit: callers must handle status
- Pythonic: uses standard dataclasses

### 3. Granular Exception Types

Create specific exception types for different error scenarios:

```python
class GitCloneError(DependencyResolutionError):
    """Git clone operation failed."""

    def __init__(
        self,
        dependency_name: str,
        url: str,
        ref: str,
        reason: str,
        *,
        returncode: int,
        stderr: str,
    ) -> None:
        super().__init__(dependency_name, reason)
        self.url = url
        self.ref = ref
        self.returncode = returncode
        self.stderr = stderr
```

**Benefits:**
- Catch specific errors: `except GitAuthenticationError:`
- Structured data for logging/monitoring
- Recovery strategies based on error type
- Better error messages with context

### 4. Type-Safe Error Handling

Use type hints to make errors explicit:

```python
def parse_graft_yaml(
    ctx: DependencyContext,
    config_path: str,
) -> GraftConfig:
    """Parse configuration.

    Raises:
        ConfigFileNotFoundError: If file doesn't exist
        ConfigParseError: If YAML is malformed
        ConfigValidationError: If config violates rules
        ValidationError: If domain rules violated
    """
    ...
```

Enable strict mypy checking:
```ini
[tool.mypy]
strict = true
warn_return_any = true
warn_unused_configs = true
disallow_untyped_defs = true
```

### 5. Structured Error Messages

Provide actionable, contextual error messages:

```python
# Bad
raise ConfigurationError("Invalid config")

# Good
raise ConfigFileNotFoundError(
    path=config_path,
    suggestion="Run 'graft init' to create graft.yaml",
)
```

**Error Message Template:**
1. What happened (the error)
2. Why it happened (context)
3. How to fix it (suggestion)

### 6. Error Recovery Strategies

**At Service Layer:**
```python
def resolve_dependency(ctx: DependencyContext, spec: DependencySpec) -> DependencyResolution:
    """Returns DependencyResolution with status."""
    try:
        # Attempt operation
        ctx.git.clone(...)
        return DependencyResolution(spec=spec, status=RESOLVED, local_path=path)
    except DependencyResolutionError as e:
        # Convert exception to failed result
        return DependencyResolution(spec=spec, status=FAILED, error_message=str(e))
```

**At CLI Layer:**
```python
def resolve_command() -> None:
    """CLI command with user-friendly error handling."""
    try:
        config = config_service.parse_graft_yaml(ctx, "graft.yaml")
    except ConfigFileNotFoundError as e:
        typer.echo(f"Error: {e}")
        typer.echo(f"Suggestion: {e.suggestion}")
        raise typer.Exit(code=1)
```

### 7. Avoid Anti-Patterns

**Don't:**
- ❌ Catch all exceptions (`except Exception:`)
- ❌ Silent failures (empty except blocks)
- ❌ String-based error codes
- ❌ Returning None for errors
- ❌ Mixing exceptions and sentinel values

**Do:**
- ✅ Catch specific exceptions
- ✅ Log before re-raising
- ✅ Use exception chaining
- ✅ Make errors explicit in types
- ✅ Provide context and suggestions

## Consequences

### Positive

1. **Type Safety** - mypy catches missing error handling
2. **Clarity** - Error modes are explicit in code
3. **Maintainability** - Consistent patterns throughout codebase
4. **User Experience** - Actionable error messages
5. **Testability** - Easy to test error cases
6. **Functional** - Result pattern enables composition

### Negative

1. **Verbosity** - More code than simple exception handling
2. **Learning Curve** - Team must understand patterns
3. **Migration** - Existing code needs updates

### Neutral

1. **Hybrid Approach** - Mix of exceptions and results
2. **Python 3.11+** - Leverages modern Python features
3. **Mypy Required** - Strict typing enforcement needed

## Implementation Plan

1. **Create granular exception types** - Split DependencyResolutionError
2. **Add type hints** - Document all raises in docstrings
3. **Configure mypy** - Enable strict type checking
4. **Update error messages** - Add context and suggestions
5. **Add error handling tests** - Test failure scenarios
6. **Update documentation** - Error handling guide

## Sources

This decision is grounded in:
- **Python language specifications**: [PEP 3134 (Exception Chaining)](https://peps.python.org/pep-3134/), [PEP 484 (Type Hints)](https://peps.python.org/pep-0484/)
- **Python idioms**: [EAFP Principle](https://docs.python.org/3/glossary.html#term-EAFP) from official Python documentation
- **Type safety tooling**: [Mypy documentation](https://mypy.readthedocs.io/) on strict checking
- **Implementation evidence**: `src/graft/domain/exceptions.py:1-240`, `src/graft/services/resolution_service.py:14-83`
- **Testing validation**: `tests/unit/test_git_errors.py`, `tests/unit/test_config_service.py`

## References

- [Effective Python Item 14](https://effectivepython.com/) - Prefer Exceptions to Returning None
- [Python Starter Template Error Handling](../python-starter/docs/architecture/error-handling.md) (if exists)

## Alternatives Considered

### Option 1: Pure Exception-Based (Traditional Python)

**Pros:** Pythonic, familiar, simple
**Cons:** Easy to forget to handle errors, invisible to type checker

**Decision:** Rejected - doesn't meet type safety goals

### Option 2: Result[T, E] Type (Rust-style)

Using library like `returns` or `result`:

```python
def parse_config(path: str) -> Result[GraftConfig, ConfigError]:
    ...

# Usage
result = parse_config("graft.yaml")
match result:
    case Success(config):
        ...
    case Failure(error):
        ...
```

**Pros:** Explicit errors, composable, type-safe
**Cons:** Not idiomatic Python, requires external library, unfamiliar to Python devs

**Decision:** Rejected - too much friction for Python ecosystem

### Option 3: Optional[T] / None Returns

```python
def parse_config(path: str) -> Optional[GraftConfig]:
    """Returns None on error."""
    ...
```

**Pros:** Simple, type-safe
**Cons:** Loss of error information, can't distinguish error types

**Decision:** Rejected - insufficient error context

### Option 4: Union Return Types

```python
def parse_config(path: str) -> Union[GraftConfig, ConfigError]:
    ...
```

**Pros:** Type-safe, explicit
**Cons:** Awkward in Python, requires isinstance checks

**Decision:** Rejected - not Pythonic

### Option 5: Our Hybrid Approach (Selected)

Combines exceptions (for exceptional cases) with result pattern (for expected failures).

**Pros:** Pythonic, type-safe, explicit, composable
**Cons:** Two patterns to learn, more verbose

**Decision:** Selected - best balance for Python

## Related Decisions

- ADR 002: Type Safety Strategy (pending)
- ADR 003: Testing Strategy (references python-starter)
- ADR 004: Logging and Observability (pending)

## Notes

This ADR documents our error handling philosophy. Implementation details may evolve, but the core principles remain:

1. Make errors explicit
2. Provide context and recovery suggestions
3. Use type system to prevent bugs
4. Follow Python idioms
5. Optimize for maintainability

Last updated: 2025-12-27
