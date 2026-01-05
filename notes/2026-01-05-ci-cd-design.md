---
title: "CI/CD Pipeline Design for Graft"
date: 2026-01-05
status: active
purpose: "Design automated quality enforcement to prevent broken tests on main"
---

# CI/CD Pipeline Design for Graft

## Problem Statement

The main branch had 10 failing tests, which should never happen. This indicates:
1. No automated quality checks before merge
2. No pre-commit hooks enforcing standards
3. No CI/CD pipeline preventing broken code from reaching main

## Solution: Multi-Layer Quality Enforcement

### Layer 1: Local Pre-Commit Hooks

**Purpose**: Catch issues before they're committed

**Tools**: Git hooks in `.git/hooks/`

**Checks**:
- Run tests on changed files
- Run mypy type checking
- Run ruff linting
- Prevent commit if any check fails

**Benefits**:
- Immediate feedback
- Fast (only checks changed code)
- Reduces CI/CD load

### Layer 2: CI/CD Pipeline (Forgejo Actions)

**Purpose**: Comprehensive checks on all pull requests and main branch

**Trigger Events**:
- Pull request opened or updated
- Push to main branch
- Manual workflow dispatch

**Jobs**:

#### Job 1: Test Suite
- Install dependencies
- Run all 330 tests with pytest
- Generate coverage report
- Fail if any test fails
- Fail if coverage < 42% (current baseline)

#### Job 2: Type Checking
- Run mypy with strict mode
- Fail if any type errors

#### Job 3: Linting
- Run ruff check on src/ and tests/
- Fail if any linting errors

#### Job 4: Build Check
- Verify package can be built
- Check for missing dependencies

**Branch Protection**:
- Require all jobs to pass before merge
- Prevent direct push to main
- Require pull request reviews

### Layer 3: Quality Standards Documentation

**Purpose**: Clear expectations for contributors

**Documents**:
- CONTRIBUTING.md with quality standards
- Pre-commit hook setup instructions
- CI/CD pipeline documentation

## Implementation Plan

### Step 1: Create Forgejo Actions Workflow

File: `.forgejo/workflows/ci.yml`

```yaml
name: CI Pipeline

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

jobs:
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh

      - name: Install dependencies
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv sync

      - name: Run tests
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv run pytest --cov=src --cov-fail-under=42

      - name: Upload coverage
        if: always()
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv run pytest --cov=src --cov-report=html

  typecheck:
    name: Type Checking
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh

      - name: Install dependencies
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv sync

      - name: Run mypy
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv run mypy src/

  lint:
    name: Linting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh

      - name: Install dependencies
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv sync

      - name: Run ruff
        run: |
          export PATH="$HOME/.local/bin:$PATH"
          uv run ruff check src/ tests/
```

### Step 2: Create Pre-Commit Hook Script

File: `.githooks/pre-commit`

```bash
#!/bin/bash
# Pre-commit hook for graft
# Runs tests, type checking, and linting before allowing commit

set -e

echo "Running pre-commit checks..."

# Ensure PATH includes uv
export PATH="$HOME/.local/bin:$PATH"

# Run tests
echo "Running tests..."
if ! uv run pytest --quiet; then
    echo "Tests failed! Fix tests before committing."
    exit 1
fi

# Run type checking
echo "Running type checking..."
if ! uv run mypy src/; then
    echo "Type checking failed! Fix type errors before committing."
    exit 1
fi

# Run linting
echo "Running linting..."
if ! uv run ruff check src/ tests/; then
    echo "Linting failed! Fix linting errors before committing."
    exit 1
fi

echo "All pre-commit checks passed!"
exit 0
```

### Step 3: Hook Installation Script

File: `scripts/install-hooks.sh`

```bash
#!/bin/bash
# Install git hooks for graft development

REPO_ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$REPO_ROOT/.githooks"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

# Create .githooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Copy pre-commit hook
cp "$HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
chmod +x "$GIT_HOOKS_DIR/pre-commit"

echo "Git hooks installed successfully!"
echo "Pre-commit hook will run tests, type checking, and linting before each commit."
```

### Step 4: Update CONTRIBUTING.md

Add section on quality standards:

```markdown
## Quality Standards

All code must pass these checks before merge:

### 1. Tests
- All 330+ tests must pass
- Coverage must be >= 42%
- Run: `uv run pytest --cov=src`

### 2. Type Checking
- mypy strict mode must pass with 0 errors
- Run: `uv run mypy src/`

### 3. Linting
- ruff must pass with 0 errors
- Run: `uv run ruff check src/ tests/`

### Installing Pre-Commit Hooks

To automatically run checks before each commit:

```bash
./scripts/install-hooks.sh
```

This will catch issues early and save CI/CD time.

### CI/CD Pipeline

All pull requests are automatically checked by Forgejo Actions:
- Test suite runs on every PR
- Type checking enforced
- Linting enforced
- All checks must pass before merge
```

## Benefits

### Immediate Benefits
1. No more broken tests on main
2. Consistent code quality
3. Faster feedback loop
4. Reduced review burden

### Long-Term Benefits
1. Increased contributor confidence
2. Easier onboarding
3. Better code maintainability
4. Professional development process

## Rollout Strategy

### Phase 1: Implement Infrastructure (Today)
- Create CI/CD workflow file
- Create pre-commit hooks
- Create installation scripts
- Test on feature branch

### Phase 2: Documentation (Today)
- Update CONTRIBUTING.md
- Add CI/CD documentation
- Update README with quality badges

### Phase 3: Enable Enforcement (After Testing)
- Enable branch protection rules
- Require CI/CD checks to pass
- Announce to contributors

## Monitoring and Maintenance

### Metrics to Track
- CI/CD pass rate
- Average build time
- Test coverage trend
- Number of commits blocked by hooks

### Maintenance Tasks
- Update dependencies monthly
- Review and update quality thresholds
- Add new checks as needed
- Monitor CI/CD costs

## Success Criteria

- [ ] All CI/CD jobs configured and passing
- [ ] Pre-commit hooks working locally
- [ ] Documentation complete
- [ ] Branch protection enabled
- [ ] No failing tests on main for 30 days
- [ ] All PRs pass CI/CD before merge

---

**Status**: Ready to implement
**Priority**: HIGH (prevents quality regressions)
**Effort**: 2-3 hours
