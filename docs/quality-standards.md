---
title: "Quality Standards and CI/CD"
date: 2026-01-05
status: active
---

# Quality Standards and CI/CD

## Overview

Graft maintains high quality standards enforced through automated checks. All code must pass these standards before being merged to main.

## Quality Gates

###  1. Test Suite

**Requirement**: All tests must pass

```bash
uv run pytest
```

**Current Status**: 405 tests passing
**Minimum Coverage**: 46% overall, 80% for service layer

**What We Test**:
- Unit tests: Individual functions and services
- Integration tests: End-to-end workflows
- CLI tests: Command-line interface behavior

**Adding Tests**:
- Place unit tests in `tests/unit/`
- Place integration tests in `tests/integration/`
- Use fakes (in `tests/fakes/`) instead of mocks
- Follow existing patterns (see contributing.md)

---

### 2. Type Checking

**Requirement**: mypy strict mode with zero errors

```bash
uv run mypy src/
```

**Configuration**: `pyproject.toml`
```toml
[tool.mypy]
strict = true
python_version = "3.11"
```

**Type Checking Standards**:
- All function parameters must have type hints
- All return types must be specified
- No `Any` types without justification
- Use Protocol types for dependency injection

**Common Issues**:
- Missing return type: `def foo() -> None:`
- Untyped parameters: `def foo(x: int, y: str) -> bool:`
- Protocol violations: Ensure implementations satisfy protocols

---

### 3. Linting

**Requirement**: ruff with zero errors

```bash
uv run ruff check src/ tests/
```

**What ruff Checks**:
- Code style (PEP 8)
- Common errors (undefined variables, unused imports)
- Best practices (list comprehensions, string formatting)
- Security issues (SQL injection, XSS patterns)

**Auto-fixing**:
```bash
uv run ruff format src/ tests/  # Format code
uv run ruff check --fix src/     # Auto-fix some issues
```

---

## Automated Enforcement

### Local: Pre-Commit Hooks

**Installation**:
```bash
./scripts/install-hooks.sh
```

**What It Does**:
- Runs before each `git commit`
- Checks: tests, type checking, linting
- Blocks commit if any check fails
- Fast feedback (30-60 seconds)

**Skipping (Emergency Only)**:
```bash
git commit --no-verify -m "Emergency fix"
```

### CI/CD: Forgejo Actions

**Trigger**: All pull requests and pushes to main

**Pipeline**: `.forgejo/workflows/ci.yml`

**Jobs**:
1. **Test** - Run full test suite with coverage
2. **Typecheck** - Run mypy strict mode
3. **Lint** - Run ruff on all code
4. **All Checks** - Summary job (requires all to pass)

**Viewing Results**:
- Go to pull request page
- Click "Checks" tab
- View logs for each job
- All must be green before merge

---

## Development Workflow

### Before Starting Work

1. Ensure environment is set up:
   ```bash
   uv sync  # Install dependencies
   ```

2. Install git hooks (once):
   ```bash
   ./scripts/install-hooks.sh
   ```

3. Verify everything works:
   ```bash
   uv run pytest --quiet
   uv run mypy src/
   uv run ruff check src/ tests/
   ```

### While Developing

1. Write code following architectural patterns
2. Add tests for new functionality
3. Add type hints to all new code
4. Run checks frequently:
   ```bash
   uv run pytest  # After changing code
   uv run mypy src/  # After adding functions
   ```

### Before Committing

Checks run automatically via pre-commit hook:
```bash
git add .
git commit -m "Your message"
# Hook runs tests, typecheck, lint
# Commit proceeds only if all pass
```

### Creating Pull Request

1. Push your branch:
   ```bash
   git push origin feature/my-feature
   ```

2. Create PR on Forgejo

3. Wait for CI/CD checks:
   - All jobs must pass (green checkmarks)
   - Fix any failures before requesting review

4. Address review feedback

5. Merge when approved and checks pass

---

## Quality Metrics

### Current Metrics (2026-02-10)

- **Tests**: 405 passing, 0 failing
- **Coverage**: ~46% overall, 80%+ service layer
- **Type Checking**: mypy strict, 0 errors
- **Linting**: ruff, 0 errors

### Target Metrics

- **Tests**: 100% passing (always)
- **Coverage**: Maintain >= 46% overall, >= 80% service layer
- **Type Checking**: 0 errors (always)
- **Linting**: 0 errors (always)

### Tracking

Review metrics on each PR:
```bash
# Coverage report
uv run pytest --cov=src --cov-report=term

# Summary
uv run pytest --quiet  # Quick check
```

---

## Common Issues and Solutions

### Issue: Tests Fail Locally

**Symptom**: `uv run pytest` fails

**Solutions**:
1. Ensure dependencies are up to date: `uv sync`
2. Check if you're on correct branch: `git status`
3. View detailed errors: `uv run pytest -v`
4. Run specific test: `uv run pytest tests/path/to/test.py::test_name`

### Issue: Type Checking Fails

**Symptom**: `uv run mypy src/` shows errors

**Solutions**:
1. Add missing type hints to function signatures
2. Use `reveal_type(x)` to debug type inference
3. Check protocol implementations match
4. See mypy docs: https://mypy.readthedocs.io/

### Issue: Linting Fails

**Symptom**: `uv run ruff check` shows errors

**Solutions**:
1. Auto-fix what you can: `uv run ruff check --fix src/`
2. Format code: `uv run ruff format src/`
3. Fix remaining issues manually
4. See ruff docs: https://docs.astral.sh/ruff/

### Issue: Pre-Commit Hook Too Slow

**Symptom**: Hook takes too long

**Solutions**:
1. Run tests in parallel: pytest uses `pytest-xdist` (future enhancement)
2. Skip hook for WIP commits: `git commit --no-verify` (use sparingly)
3. Make smaller, focused commits

### Issue: CI/CD Fails But Local Passes

**Symptom**: Local checks pass, CI fails

**Solutions**:
1. Ensure dependencies match: `uv sync`
2. Check Python version (3.11)
3. Look at CI logs for specific error
4. Test in clean environment: `rm -rf .venv && uv sync && uv run pytest`

---

## Maintenance

### Updating Standards

When updating quality standards:
1. Update this document
2. Update CI/CD workflow if needed
3. Update pre-commit hooks if needed
4. Announce changes to contributors
5. Run on existing code to verify

### Reviewing Metrics

Monthly review:
- Test pass rate
- Coverage trends
- CI/CD build times
- Number of commits blocked by hooks

Adjust thresholds as project matures.

---

## References

- **pytest**: https://docs.pytest.org/
- **mypy**: https://mypy.readthedocs.io/
- **ruff**: https://docs.astral.sh/ruff/
- **uv**: https://github.com/astral-sh/uv
- **Forgejo Actions**: https://forgejo.org/docs/latest/user/actions/

---

**Last Updated**: 2026-02-10
**Status**: Active
**Enforcement**: Automated via pre-commit hooks and CI/CD
