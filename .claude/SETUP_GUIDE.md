# Claude Code Setup Guide for Graft

This document describes the Claude Code configuration for the Graft project.

## Overview

The Graft project is configured with best practices for agentic development using Claude Code. This setup accelerates development, improves code quality, and maintains architectural consistency.

## Installation Steps

### 1. Install Development Dependencies

```bash
# Using uv (recommended)
uv pip install -e ".[dev]"

# Or using pip
pip install -e ".[dev]"
```

This installs:
- `pytest` — Testing framework
- `ruff` — Fast Python linter and formatter
- `mypy` — Type checker
- `pre-commit` — Git hook framework
- `types-PyYAML` — Type stubs for YAML

### 2. Set Up Pre-commit Hooks

```bash
# Install pre-commit hooks
pre-commit install

# Test it works
pre-commit run --all-files
```

Pre-commit will automatically run:
- Code formatting (ruff format)
- Linting (ruff check)
- Type checking (mypy)
- Basic file quality checks
- Security checks (bandit)

### 3. Verify Claude Code Configuration

The project includes:
- `.claude/settings.json` — Team-shared configuration
- `.claude/commands/` — Custom slash commands
- `.claude/skills/` — Specialized agent skills
- `CLAUDE.md` — Project instructions and context

## Available Slash Commands

### Development Commands
- `/slice <N>` — Start work on vertical slice N
- `/test` — Run pytest with analysis
- `/format` — Format code and run linters
- `/check-contract` — Verify CLI contract compliance

### Documentation Commands
- `/work-log` — Update today's work log
- `/adr <title>` — Create Architecture Decision Record
- `/schema` — Validate JSON schemas

## Available Skills

### graft-dev
**Purpose**: Implement Graft slices safely with outside-in TDD and clean architecture.

**Activates when you mention**:
- Implementing a vertical slice
- Working on Graft features
- CLI command implementation

### debug-tests
**Purpose**: Systematically debug and fix test failures.

**Activates when you mention**:
- Test failures
- Debugging tests
- pytest errors

### schema-validator
**Purpose**: Validate and synchronize JSON schemas with CLI implementation.

**Activates when you mention**:
- Schema validation
- JSON schema
- Contract validation

### architecture-review
**Purpose**: Review code changes for architecture compliance.

**Activates when you mention**:
- Code review
- Architecture review
- Checking if code follows patterns

## Hooks

The configuration includes hooks that provide automatic feedback:

### PostToolUse Hooks
- After editing Python files → Reminder to run tests
- After modifying schemas → Reminder to validate with `/schema`

### Stop Hooks
- When Claude finishes → Reminder to update work log

## Permissions

The configuration includes:

### Auto-allowed Operations
- Running pytest tests
- Using uv/pip for dependencies
- Running graft CLI commands
- Git status, diff, log, add, commit
- Reading project files
- Using slash commands

### Blocked Operations
- `rm -rf` commands
- Force git operations
- Network operations (curl, wget)
- Modifying .venv or .git directories
- Modifying uv.lock

### Requires Confirmation
- git push/pull/rebase/reset

## Typical Development Workflow

### Starting a New Slice

```
User: "Let's work on Slice 1"
Assistant: [graft-dev skill activates]
          [Uses /slice 1 command]
          [Creates todo list]
          [Reviews requirements]
```

### Implementing Features

```
1. Write failing tests first (black-box subprocess style)
2. Implement in layers: Domain → Adapters → Services → CLI
3. Run /test to verify
4. Use /format to clean up code
5. Use /check-contract to verify compliance
```

### Completing Work

```
1. Ensure all tests pass
2. Run /work-log to document progress
3. Create /adr if architectural decisions were made
4. Commit changes (hooks run automatically)
```

## Architecture Patterns

All code follows the layered architecture from ADR-0002:

### Domain Layer (`src/graft/domain/`)
- Immutable dataclasses
- No I/O operations
- Pure business entities

### Adapter Layer (`src/graft/adapters/`)
- Protocol-based interfaces
- External interaction handling
- No business logic

### Service Layer (`src/graft/services/`)
- Use case orchestration
- Dependency injection
- Returns result objects

### CLI Layer (`src/graft/cli.py`)
- Thin presentation layer
- Exception → exit code mapping
- Output formatting

## Testing

All tests are black-box subprocess tests:

```python
# Good
result = run_graft("explain", str(artifact_path), "--json")
assert result.returncode == 0

# Bad - don't import internals
from graft.services.explain import ExplainService
```

## Exit Codes

- `0` — Success
- `1` — User error (bad input, missing files, invalid YAML)
- `2` — System error (permissions, unexpected exceptions)

## Getting Help

- Read `CLAUDE.md` for project instructions
- Check `docs/adr/` for architectural decisions
- Review `docs/implementation-strategy.md` for slice details
- Look at `tests/conftest.py` for test patterns
- Use `/help` in Claude Code for command reference

## Troubleshooting

### Tests failing with "Module not found"
```bash
uv pip install -e ".[test]"
```

### Pre-commit hooks failing
```bash
pre-commit run --all-files
# Fix reported issues, then commit
```

### Claude Code not recognizing commands
Check that:
- Commands exist in `.claude/commands/`
- Settings in `.claude/settings.json` include command permissions
- File has proper frontmatter with `description` field

## Contributing

When contributing to Graft:

1. Follow the vertical slice development process
2. Use the provided slash commands for consistency
3. Let skills activate automatically for specialized tasks
4. Keep architecture layers separate
5. Ensure all tests pass before committing
6. Document decisions in work logs and ADRs

## Questions?

If you encounter issues with the Claude Code setup or have suggestions for improvements, discuss with the team or create an issue.
