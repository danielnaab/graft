# Skill: Debug Tests

## Purpose
Systematically debug and fix test failures in Graft's black-box test suite.

## When to Use
Activate when the user mentions:
- Test failures
- Tests are failing
- Debugging tests
- Fixing broken tests
- pytest errors
- Test not passing

## Process

### 1. Gather Context
- Run `pytest -q` to see current test status
- Identify which tests are failing
- Read the full error messages and stack traces
- Check if it's an import error, assertion error, or runtime error

### 2. Analyze Root Cause
Common failure patterns in Graft:
- **Package not installed**: Module import errors → Need `uv pip install -e ".[test]"`
- **CLI exit code mismatch**: Expected 0 but got 1 → Check error handling in CLI
- **JSON parsing errors**: Empty stdout → Check if command is printing to stderr
- **Assertion failures**: Output doesn't match contract → Check service `.to_dict()` implementation
- **File not found**: Test fixtures missing → Check `examples/` directory structure

### 3. Fix Strategy
For each failure type:

**Import/Module Errors**:
- Verify package is installed with correct dependencies
- Check PYTHONPATH is set correctly
- Ensure `src/graft/__init__.py` exists

**CLI Contract Violations**:
- Review exit code mapping in CLI layer
- Check exception handling (typer.Exit with correct codes)
- Verify JSON output with `--json` flag

**Output Format Issues**:
- Compare expected vs actual JSON structure
- Check service `.to_dict()` method implementation
- Validate against schema in `schemas/cli/`

**Test Logic Issues**:
- Review test expectations in test file
- Check if contract changed but tests didn't update
- Verify fixture data in `examples/`

### 4. Implement Fix
- Make minimal changes to fix the specific failure
- Follow layered architecture (don't bypass layers)
- Ensure fix doesn't break other tests

### 5. Verify
- Run `pytest -q` again to confirm fix
- Check that ALL tests pass, not just the one that failed
- If new failures appear, repeat the process

## Debugging Tools
- `pytest -v` — Verbose output with test names
- `pytest -vv` — Very verbose with full diffs
- `pytest tests/test_specific.py::test_name` — Run single test
- `pytest --tb=short` — Shorter traceback format
- `python -m graft.cli <command> --json` — Manual CLI testing

## Common Fixes

### Fix: Module not found
```bash
cd /path/to/graft
uv pip install -e ".[test]"
```

### Fix: Wrong exit code
```python
# In CLI layer, ensure proper exception mapping
except typer.BadParameter as e:
    typer.echo(f"Error: {e}", err=True)
    raise typer.Exit(code=1)  # User error
except Exception as e:
    typer.echo(f"System error: {e}", err=True)
    raise typer.Exit(code=2)  # System error
```

### Fix: JSON output missing fields
```python
# In service layer, ensure .to_dict() is complete
def to_dict(self) -> dict:
    return {
        "field1": self.field1,
        "field2": self.field2,  # Don't forget any fields!
    }
```

## Quality Checklist
- [ ] Identified root cause of each failure
- [ ] Fixed issues without breaking architecture
- [ ] All tests now pass
- [ ] Manual CLI testing confirms fix
- [ ] No new test failures introduced

## Outputs
- Clear diagnosis of test failure root causes
- Minimal fixes that restore green tests
- Updated work log with debugging notes
