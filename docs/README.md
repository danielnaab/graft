---
title: Graft Documentation
status: stable
updated: 2026-01-05
---

# Graft Documentation

**Semantic dependency management for knowledge bases**

> **Authority Note:** This document provides a developer-friendly overview of Graft's implementation architecture. For canonical architectural decisions, see [graft-knowledge/docs/architecture.md](../../graft-knowledge/docs/architecture.md) and [ADRs](../../graft-knowledge/docs/decisions/).

Graft provides atomic upgrades with automatic rollback, migration execution, and semantic versioning for git-based dependencies.

## Quick Links

- **Getting Started**: See main [README.md](../README.md)
- **Complete Workflow**: See [workflow-validation.md](../status/workflow-validation.md)
- **Implementation Status**: See [implementation.md](../status/implementation.md)
- **CLI Implementation**: See [phase-8.md](../status/phase-8.md)
- **Gap Analysis**: See [gap-analysis.md](../status/gap-analysis.md)
- **Development Notes**: See [continue-here.md](../continue-here.md)

## Architecture

Graft follows clean architecture principles with:

### Domain Models

Located in `src/graft/domain/`:

- **Change**: Semantic change representation (breaking, feature, fix)
- **Command**: Executable command with environment and working directory
- **LockEntry**: Locked dependency version with commit hash
- **GraftConfig**: Full configuration model with metadata, changes, commands

All domain models are **frozen dataclasses** (immutable value objects).

### Services

Located in `src/graft/services/`:

**Query Operations**:
- `query_service.py` - Read-only queries (status, changes, details)

**Mutation Operations**:
- `upgrade_service.py` - Atomic upgrades with rollback
- `lock_service.py` - Lock file read/write/update
- `command_service.py` - Command execution

**Infrastructure**:
- `snapshot_service.py` - Snapshot creation and restoration
- `config_service.py` - Configuration parsing

**Architecture**: Services are **pure functions** accepting protocol dependencies, not classes.

### Protocols

Located in `src/graft/protocols/`:

- `Snapshot` - Snapshot operations interface
- `LockFile` - Lock file operations interface
- `CommandExecutor` - Command execution interface
- `Git` - Git operations interface
- `Repository` - Repository management interface
- `FileSystem` - File system operations interface

**Architecture**: Uses `typing.Protocol` for structural subtyping (duck typing with type safety).

### Adapters

Located in `src/graft/adapters/`:

- `FilesystemSnapshot` - Filesystem-based snapshots in `.graft/snapshots/`
- `YamlLockFile` - YAML-based lock file (version 1 format)
- `SubprocessCommandExecutor` - Subprocess-based command execution
- `GitAdapter` - Git operations via subprocess
- `FileSystemAdapter` - File system operations
- `RepositoryAdapter` - Repository management

### CLI Commands

Located in `src/graft/cli/commands/`:

All 6 commands are fully implemented:

1. **resolve.py** - Clone/fetch dependencies
2. **apply.py** - Update lock file without migrations
3. **status.py** - Show consumed versions
4. **changes.py** - List available changes
5. **show.py** - Show change details
6. **upgrade.py** - Atomic upgrade with rollback

See [CLI Commands](#cli-commands) section below for details.

## CLI Commands

### graft resolve

Clone or fetch all dependencies from `graft.yaml`.

```bash
uv run python -m graft resolve
```

**Implementation**: `src/graft/cli/commands/resolve.py`

### graft apply

Update lock file without running migrations (manual workflow).

```bash
uv run python -m graft apply <dep-name> --to <ref>
```

**Use Cases**:
- Initial lock file creation
- Manual migration workflows
- Acknowledgment of manual upgrades

**Implementation**: `src/graft/cli/commands/apply.py`

### graft status

Show current consumed versions from lock file.

```bash
# All dependencies
uv run python -m graft status

# Specific dependency
uv run python -m graft status <dep-name>
```

**Implementation**: `src/graft/cli/commands/status.py`

### graft changes

List available changes/versions for a dependency.

```bash
# All changes
uv run python -m graft changes <dep-name>

# Filter by type
uv run python -m graft changes <dep-name> --type feature
uv run python -m graft changes <dep-name> --breaking

# Filter by ref range
uv run python -m graft changes <dep-name> --from-ref v1.0 --to-ref v2.0
```

**Implementation**: `src/graft/cli/commands/changes.py`

### graft show

Display detailed information about a specific change.

```bash
uv run python -m graft show <dep-name@ref>
```

**Output includes**:
- Change type (breaking, feature, fix)
- Description
- Migration command (if defined)
- Verification command (if defined)

**Implementation**: `src/graft/cli/commands/show.py`

### graft upgrade

Perform atomic upgrade with automatic rollback.

```bash
# Full upgrade with migration and verification
uv run python -m graft upgrade <dep-name> --to <ref>

# Skip migration
uv run python -m graft upgrade <dep-name> --to <ref> --skip-migration

# Skip verification
uv run python -m graft upgrade <dep-name> --to <ref> --skip-verify
```

**Upgrade Process**:
1. Creates snapshot of `graft.lock`
2. Executes migration command (if defined)
3. Executes verification command (if defined)
4. Updates lock file
5. **Automatically rolls back on any failure**

**Implementation**: `src/graft/cli/commands/upgrade.py`

## Configuration Format

### graft.yaml

```yaml
apiVersion: graft/v0

# Required: Dependency declarations
deps:
  my-dep: "https://github.com/user/repo.git#main"
  other-dep: "ssh://git@server/repo.git#develop"

# Optional: Project metadata
metadata:
  description: "My project"
  version: "1.0.0"

# Optional: Change definitions
changes:
  v1.0.0:
    type: feature
    description: "Initial release"

  v2.0.0:
    type: breaking
    description: "Major refactor"
    migration: migrate-v2
    verify: verify-v2

# Optional: Command definitions
commands:
  migrate-v2:
    run: "./scripts/migrate.sh"
    description: "Migrate to v2"
    working_dir: "."
    env:
      DEBUG: "true"

  verify-v2:
    run: "./scripts/verify.sh"
    description: "Verify v2 migration"
```

### graft.lock

Generated automatically, commit to version control:

```yaml
version: 1
dependencies:
  my-dep:
    source: "https://github.com/user/repo.git"
    ref: "v2.0.0"
    commit: "abc123def456789..."
    consumed_at: "2026-01-04T00:00:00+00:00"
```

## Testing

### Test Structure

```
tests/
├── unit/              # Fast unit tests with fakes
│   ├── test_upgrade_service.py
│   ├── test_snapshot_service.py
│   ├── test_query_service.py
│   ├── test_lock_service.py
│   ├── test_command_service.py
│   └── test_domain_*.py
├── integration/       # Integration tests with real adapters
│   ├── test_adapters.py
│   ├── test_snapshot_integration.py
│   └── test_resolve_integration.py
└── fakes/             # In-memory test doubles
    ├── fake_snapshot.py
    ├── fake_lock_file.py
    ├── fake_command_executor.py
    └── ...
```

### Running Tests

```bash
# All tests (278 passing)
uv run pytest

# With coverage
uv run pytest --cov=src/graft --cov-report=html

# Specific test file
uv run pytest tests/unit/test_upgrade_service.py -v

# Only unit tests
uv run pytest tests/unit/

# Only integration tests
uv run pytest tests/integration/
```

### Coverage

- **Overall**: 61% (CLI at 0%, services at 80-100%)
- **Domain models**: 85-100%
- **Services**: 80-100%
- **Adapters**: 81-92%
- **CLI**: 0% (tested via dogfooding)

## Error Handling

Graft provides clear, actionable error messages. See [Error Handling ADR](decisions/001-error-handling-strategy.md).

### Common Errors

**Missing graft.yaml**:
```
Error: No graft.yaml found in /path/to/project

Create a graft.yaml file with:
  apiVersion: graft/v0
  deps:
    my-dep: "https://github.com/user/repo.git#main"
```

**Dependency not found**:
```
Error: Dependency 'unknown-dep' not found in configuration
  Available: my-dep, other-dep
```

**Lock file not found**:
```
Error: Lock file not found
  Run: graft apply <dep-name> --to <ref>
```

**Upgrade failure with rollback**:
```
Error: Migration command failed (exit code: 1)
  Rolling back changes...
✓ Rollback complete - workspace restored
```

## Development

### Adding a New Service

1. Define domain models in `src/graft/domain/`
2. Create protocol in `src/graft/protocols/`
3. Implement service function in `src/graft/services/`
4. Create adapter in `src/graft/adapters/`
5. Create fake in `tests/fakes/`
6. Write unit tests in `tests/unit/`
7. Write integration tests in `tests/integration/`

### Code Quality

```bash
# Linting
uv run ruff check src/ tests/

# Formatting
uv run ruff format src/ tests/

# All quality checks
uv run pytest && uv run ruff check src/ tests/
```

## Project Status

**Production Ready** - 9/10 phases complete

✅ Domain models
✅ Configuration parsing
✅ Lock file operations
✅ Command execution
✅ Query operations
✅ Snapshot/Rollback system
✅ Atomic upgrade operations
✅ CLI Integration
✅ Dogfooded on graft itself
⏳ Documentation (in progress)

## Resources

### Documentation

- [README.md](../README.md) - Main documentation
- [workflow-validation.md](../status/workflow-validation.md) - End-to-end workflow
- [implementation.md](../status/implementation.md) - Detailed status
- [phase-8.md](../status/phase-8.md) - CLI details
- [gap-analysis.md](../status/gap-analysis.md) - Gap analysis
- [continue-here.md](../continue-here.md) - Development notes

### Specifications

Located in `/home/coder/graft-knowledge/docs/specification/`:
- `core-operations.md` - Core operation specifications
- `change-model.md` - Change model specification
- `graft-yaml-format.md` - Configuration format

### For AI Agents

See [agents.md](agents.md) for structured technical reference.

## Sources

This architecture documentation is grounded in:

**Canonical Specifications:**
- [Graft Architecture Specification](../../graft-knowledge/docs/architecture.md) - System design decisions
- [ADR 004: Protocol-Based DI](decisions/004-protocol-based-dependency-injection.md) - Dependency injection approach
- [ADR 005: Functional Service Layer](decisions/005-functional-service-layer.md) - Service design pattern
- [ADR 002: Filesystem Snapshots](decisions/002-filesystem-snapshots-for-rollback.md) - Rollback mechanism
- [ADR 001: Explicit Ref in Upgrade](decisions/001-require-explicit-ref-in-upgrade.md) - CLI design

**Implementation Evidence:**
- Domain models: `src/graft/domain/*.py` (frozen dataclasses)
- Services: `src/graft/services/*.py` (pure functions)
- Protocols: `src/graft/protocols/*.py` (structural subtyping)
- Adapters: `src/graft/adapters/*.py` (infrastructure implementations)
- CLI commands: `src/graft/cli/commands/*.py` (8 commands)

**Validation:**
- Tests: `tests/unit/` (12 modules, 150+ tests)
- Integration tests: `tests/integration/` (4 modules, 800+ lines)
- Workflow validation: [workflow-validation.md](../status/workflow-validation.md)

## License

MIT License - see LICENSE file for details.
