# Error Handling Improvements

**Date:** 2025-12-27
**Author:** Claude Code
**Status:** Implemented

## Summary

Implemented comprehensive, Pythonic, type-safe error handling strategy for Graft following ADR 001. Introduced granular exception types, improved error messages with actionable suggestions, and added strict type checking.

## Motivation

The initial implementation used basic exception handling with generic error types. As documented in [ADR 001: Error Handling Strategy](/home/coder/graft/docs/decisions/001-error-handling-strategy.md), we needed:

1. **Type Safety** - Make errors explicit in function signatures
2. **Better UX** - Provide actionable error messages with recovery suggestions
3. **Specificity** - Catch specific error types for targeted recovery
4. **Pythonic** - Follow Python idioms (EAFP, exception chaining)
5. **Functional** - Support composition with Result pattern

## Implementation

### 1. Granular Exception Hierarchy

Created specific exception types for different failure modes:

**Configuration Errors** (`src/graft/domain/exceptions.py:39-102`):
- `ConfigFileNotFoundError` - Missing graft.yaml
- `ConfigParseError` - Malformed YAML
- `ConfigValidationError` - Invalid configuration structure

**Git Errors** (`src/graft/domain/exceptions.py:123-240`):
- `GitCloneError` - Clone operation failed
- `GitFetchError` - Fetch/checkout failed
- `GitAuthenticationError` - SSH key or credential issues
- `GitNotFoundError` - Repository or ref not found

**Key Features:**
- Structured error data (not just strings)
- Recovery suggestions included
- Exception chaining preserved (`from e`)
- Immutable error objects

### 2. Enhanced Service Layer

Updated `config_service.py:12-16` to use specific exceptions:

```python
def parse_graft_yaml(ctx: DependencyContext, config_path: str) -> GraftConfig:
    """Parse configuration.

    Raises:
        ConfigFileNotFoundError: If file doesn't exist
        ConfigParseError: If YAML is malformed
        ConfigValidationError: If structure invalid
        ValidationError: If domain rules violated
    """
```

**Benefits:**
- Type-safe: mypy knows which exceptions can be raised
- Explicit: callers see all failure modes
- Documented: Raises section acts as contract

### 3. Intelligent Error Detection

Updated `adapters/git.py:74-93` to detect specific git errors:

```python
if "Permission denied" in stderr or "publickey" in stderr:
    raise GitAuthenticationError(...)
elif "not found" in stderr.lower():
    raise GitNotFoundError(...)
else:
    raise GitCloneError(...)
```

**Detection Patterns:**
- Authentication: "Permission denied", "publickey"
- Not Found: "not found", "does not exist"
- Missing Ref: "did not match", "unknown revision"

### 4. User-Friendly CLI Messages

Updated `cli/commands/resolve.py:54-76` with structured error handling:

```python
except ConfigFileNotFoundError as e:
    typer.secho("Error: Configuration file not found", fg=RED)
    typer.echo(f"  Path: {e.path}")
    typer.secho(f"  Suggestion: {e.suggestion}", fg=YELLOW)
```

**Output Format:**
```
Error: Configuration file not found
  Path: /home/user/project/graft.yaml
  Suggestion: Create graft.yaml with 'apiVersion: graft/v0' and 'deps:'
```

### 5. Strict Type Checking

Added `pyproject.toml:101-120` mypy configuration:

```toml
[tool.mypy]
python_version = "3.11"
warn_return_any = true
disallow_untyped_defs = true
warn_unused_ignores = true
strict_equality = true
```

**Enforces:**
- All functions have type hints
- Return types explicit
- No implicit optionals
- Strict equality checks

### 6. Hybrid Error Strategy

Maintained Result pattern for batch operations:

**`DependencyResolution` Entity:**
```python
@dataclass
class DependencyResolution:
    spec: DependencySpec
    status: DependencyStatus  # PENDING | RESOLVED | FAILED
    error_message: Optional[str] = None
```

**Usage in `resolution_service.py:56-68`:**
```python
try:
    ctx.git.clone(...)
    resolution.mark_resolved(local_path)
except DependencyResolutionError as e:
    resolution.mark_failed(e.reason)  # Convert to result
```

**Benefits:**
- Batch operations continue on partial failure
- Type-safe: status must be checked
- Composable: can map/filter resolutions
- Pythonic: uses dataclasses

## Files Modified

**Domain Layer:**
- `src/graft/domain/exceptions.py` - Added 7 new exception types

**Service Layer:**
- `src/graft/services/config_service.py` - Use granular exceptions
- `src/graft/services/resolution_service.py` - Maintain Result pattern

**Adapter Layer:**
- `src/graft/adapters/git.py` - Detect specific git errors

**CLI Layer:**
- `src/graft/cli/commands/resolve.py` - User-friendly error messages

**Configuration:**
- `pyproject.toml` - Added mypy strict configuration

**Documentation:**
- `docs/decisions/001-error-handling-strategy.md` - ADR documenting approach

## Testing Strategy

Error handling tests should cover:

1. **Unit Tests:**
   - Each exception type raised correctly
   - Error detection logic works
   - Error messages include context

2. **Integration Tests:**
   - Real error scenarios (missing files, bad URLs)
   - Error propagation through layers
   - CLI error output formatting

**Test Updates Needed:**
- Update `test_config_service.py` for new exceptions
- Add git error tests to `test_git.py`
- Add CLI error formatting tests

## Related Decisions

- [ADR 001: Error Handling Strategy](/home/coder/graft/docs/decisions/001-error-handling-strategy.md)
- [Python EAFP Principle](https://docs.python.org/3/glossary.html#term-EAFP)
- [PEP 3134: Exception Chaining](https://peps.python.org/pep-3134/)

## Impact

### Positive

✅ **Type Safety** - mypy catches missing error handling
✅ **Better UX** - Users get actionable error messages
✅ **Debugging** - Structured errors easier to log/monitor
✅ **Recovery** - Can catch specific errors for retries
✅ **Maintainability** - Clear error contracts

### Considerations

⚠️ **Test Updates** - Existing tests need updating for new exceptions
⚠️ **Migration** - Update any external error handling code

## Next Steps

1. **Update Tests** - Modify tests to expect new exception types
2. **Add Error Tests** - Test specific error scenarios
3. **Documentation** - User-facing error handling guide
4. **Monitoring** - Add structured logging for errors

## Examples

### Before (Generic Error)

```python
# Generic exception
raise ConfigurationError("Invalid config")

# CLI output
Configuration error: Invalid config
```

### After (Specific Error with Context)

```python
# Specific exception with context
raise ConfigValidationError(
    path=config_path,
    field="deps.graft-knowledge",
    reason="Must use format 'url#ref', got: https://example.com/repo.git"
)

# CLI output
Error: Invalid configuration
  File: /home/user/project/graft.yaml
  Field: deps.graft-knowledge
  Reason: Must use format 'url#ref', got: https://example.com/repo.git
```

## References

- Implementation Plan: `/home/coder/.claude/plans/gleaming-coalescing-lobster.md`
- ADR 001: `/home/coder/graft/docs/decisions/001-error-handling-strategy.md`
- Python Starter Error Patterns: `../python-starter/docs/architecture/error-handling.md` (if exists)
