---
date: 2025-12-26
status: stable
tags: [setup, copier, cli, tooling]
---

# Initial Project Setup with python-starter Template

## Summary

Successfully applied the python-starter Copier template to establish the graft project's basic Python tooling infrastructure and implemented the initial `graft version` command.

## What was done

### 1. Template Application

Applied python-starter template (version 0.0.0.post7.dev0+d8b60fe) to graft repository:

- Established src/graft/ package structure with:
  - `domain/` - Pure domain entities and value objects
  - `services/` - Functional service layer with context objects
  - `adapters/` - External system implementations (repositories, etc.)
  - `protocols/` - Protocol interface definitions
  - `cli/` - Typer-based CLI commands

- Set up testing infrastructure:
  - Unit tests with fake implementations pattern
  - Integration tests for adapters
  - pytest configuration with coverage reporting
  - All 40 tests passing

- Configured development tooling:
  - uv for dependency management
  - ruff for linting and formatting
  - pytest for testing
  - pyproject.toml as central configuration

- Added comprehensive documentation:
  - Architecture documentation (functional services, DI, domain model, testing)
  - ADRs for key decisions (uv, src layout, protocols, etc.)
  - Development guides (getting started, workflow, adding features, testing)
  - Reference documentation (structure, configuration, tooling, protocols)

### 2. Fixed Template Issues

**Copier template fix**: Updated ../python-starter/copier.yml to fix Jinja2 `now` filter compatibility issue by hardcoding current_year to "2025".

**Circular import fix**: Resolved circular dependency between `cli/main.py` and `cli/commands/example.py`:
- Extracted `get_context()` into new `cli/context_factory.py` module
- Updated imports in command modules to use context_factory
- Allows clean separation between CLI app definition and context creation

### 3. Command Aliases

Added both `graft` and `graft-cli` as command entry points in pyproject.toml for user convenience.

### 4. Version Command

The template already included a `version` command in `cli/main.py` that displays the package version from `__init__.py`.

```bash
$ graft version
Graft v0.1.0
```

## Repository State

- Initial commit: cacaa3f (KB structure and dependency management)
- Template application: 05de131 (Apply python-starter Copier template)
- Circular import fix: 1c0f4de (Fix circular import and add graft command alias)

## Copier Integration

The template can be updated from upstream using:

```bash
copier update --trust
```

The template configuration is tracked in `.copier-answers.yml`.

## Next Steps

Future work should:
- Remove example code (domain/entities.py, services/example_service.py, etc.) when implementing actual graft functionality
- Implement knowledge base specific domain models
- Add LSP integration commands
- Build out actual graft tooling based on specs in graft-knowledge

## Sources

- Template: .graft/python-starter
- Template documentation: .graft/python-starter/docs/
- Copier configuration: .copier-answers.yml
- Git commits: 05de131, 1c0f4de
- Meta-knowledge-base policies: .graft/meta-knowledge-base/docs/meta.md
