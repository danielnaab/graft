# Graft

**Semantic dependency management for knowledge bases**

Graft is a dependency management tool designed for knowledge bases and structured content repositories. It provides atomic upgrades with automatic rollback, migration execution, and semantic versioning of changes.

## Features

- **Atomic Upgrades**: All-or-nothing upgrades with automatic rollback on failure
- **Semantic Changes**: Track breaking changes, features, and fixes separately
- **Migration Support**: Execute migration and verification commands during upgrades
- **Lock File Management**: Track exact consumed versions with commit hashes
- **Git Integration**: Works with any git repository as a dependency
- **Snapshot/Rollback**: Filesystem-based snapshots for safe rollback
- **CLI Interface**: User-friendly command-line interface with color-coded output

## Quick Start

### Prerequisites

- Python 3.11 or higher
- [uv](https://docs.astral.sh/uv/) package manager
- Git

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd graft

# Install dependencies
uv sync

# Verify installation
uv run python -m graft --help
```

### Basic Usage

```bash
# 1. Create a graft.yaml file
cat > graft.yaml <<EOF
apiVersion: graft/v0
deps:
  my-knowledge: "https://github.com/user/knowledge.git#main"
EOF

# 2. Clone dependencies
uv run python -m graft resolve

# 3. Create initial lock file
uv run python -m graft apply my-knowledge --to main

# 4. Check status
uv run python -m graft status

# 5. List available changes
uv run python -m graft changes my-knowledge

# 6. Upgrade to a new version
uv run python -m graft upgrade my-knowledge --to v2.0.0
```

## CLI Commands

### `graft resolve`

Clone or fetch all dependencies declared in `graft.yaml`.

```bash
uv run python -m graft resolve
```

### `graft apply <dep-name> --to <ref>`

Update the lock file to acknowledge a specific version without running migrations. Useful for initial setup or manual migration workflows.

```bash
uv run python -m graft apply my-knowledge --to main
uv run python -m graft apply my-knowledge --to v1.0.0
```

### `graft status [dep-name]`

Show current consumed versions from the lock file.

```bash
# Show all dependencies
uv run python -m graft status

# Show specific dependency
uv run python -m graft status my-knowledge

# JSON output for scripting
uv run python -m graft status --json
uv run python -m graft status my-knowledge --json
```

**Options**:
- `--json`: Output as JSON for machine-readable parsing

### `graft changes <dep-name>`

List available changes/versions for a dependency.

```bash
# List all changes
uv run python -m graft changes my-knowledge

# Filter by type
uv run python -m graft changes my-knowledge --type feature
uv run python -m graft changes my-knowledge --breaking

# Filter by ref range
uv run python -m graft changes my-knowledge --from-ref v1.0.0 --to-ref v2.0.0

# JSON output for scripting
uv run python -m graft changes my-knowledge --format json
uv run python -m graft changes my-knowledge --breaking --format json
```

**Options**:
- `--format`: Output format (text or json)

### `graft show <dep-name@ref>`

Display detailed information about a specific change.

```bash
uv run python -m graft show my-knowledge@v2.0.0
```

### `graft upgrade <dep-name> --to <ref>`

Perform an atomic upgrade with migration execution and automatic rollback on failure.

```bash
# Upgrade with migration and verification
uv run python -m graft upgrade my-knowledge --to v2.0.0

# Skip migration (update lock file only)
uv run python -m graft upgrade my-knowledge --to v2.0.0 --skip-migration

# Skip verification
uv run python -m graft upgrade my-knowledge --to v2.0.0 --skip-verify
```

**Upgrade Process:**
1. Creates snapshot of current state
2. Runs migration command (if defined)
3. Runs verification command (if defined)
4. Updates lock file
5. **Automatically rolls back on any failure**

## Configuration

### graft.yaml Format

```yaml
apiVersion: graft/v0

# Dependency declarations
deps:
  my-knowledge: "https://github.com/user/knowledge.git#main"
  other-dep: "ssh://git@server/repo.git#develop"

# Optional metadata
metadata:
  description: "My project's knowledge dependencies"
  version: "1.0.0"

# Change declarations
changes:
  v1.0.0:
    type: feature
    description: "Initial release"

  v2.0.0:
    type: breaking
    description: "Major restructuring"
    migration: migrate-v2
    verify: verify-v2

# Migration commands
commands:
  migrate-v2:
    run: "./scripts/migrate-to-v2.sh"
    description: "Migrate to v2 structure"

  verify-v2:
    run: "./scripts/verify-v2.sh"
    description: "Verify v2 migration succeeded"
```

### graft.lock Format

The lock file (generated automatically) tracks exact consumed versions:

```yaml
version: 1
dependencies:
  my-knowledge:
    source: "https://github.com/user/knowledge.git"
    ref: "v2.0.0"
    commit: "abc123def456..."
    consumed_at: "2026-01-04T00:00:00+00:00"
```

**Important:** Commit `graft.lock` to version control to ensure reproducible builds.

## Development

### Running Tests

```bash
# Run all tests
uv run pytest

# Run with coverage
uv run pytest --cov=src/graft --cov-report=html

# Run specific test file
uv run pytest tests/unit/test_upgrade_service.py -v
```

### Code Quality

```bash
# Check linting
uv run ruff check src/ tests/

# Format code
uv run ruff format src/ tests/

# Run type checking (if mypy is installed)
uv run mypy src/
```

### Project Structure

```
graft/
├── src/graft/
│   ├── domain/          # Domain models (Change, Command, LockEntry, etc.)
│   ├── services/        # Service functions (upgrade, query, lock, etc.)
│   ├── protocols/       # Protocol interfaces for DI
│   ├── adapters/        # Infrastructure implementations
│   └── cli/             # Command-line interface
├── tests/
│   ├── unit/            # Unit tests with fakes
│   ├── integration/     # Integration tests
│   └── fakes/           # In-memory test doubles
└── docs/                # Documentation
```

## Architecture

Graft follows a clean architecture with:

- **Domain-Driven Design**: Core domain models (Change, Command, LockEntry)
- **Protocol-Based DI**: Structural typing for flexible dependency injection
- **Functional Services**: Pure functions accepting protocol dependencies
- **Immutable Values**: All domain models are frozen dataclasses
- **Snapshot Pattern**: Filesystem-based snapshots for rollback
- **Atomic Operations**: All-or-nothing upgrades with automatic cleanup

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for detailed architecture documentation.

## Complete Workflow Example

```bash
# 1. Initial setup
cd my-project
cat > graft.yaml <<EOF
apiVersion: graft/v0
deps:
  knowledge-base: "https://github.com/org/knowledge.git#main"
EOF

# 2. Clone dependencies
uv run python -m graft resolve
# ✓ knowledge-base: resolved to .graft/deps/knowledge-base

# 3. Create lock file
uv run python -m graft apply knowledge-base --to main
# Applied knowledge-base@main
# Updated graft.lock

# 4. Check current status
uv run python -m graft status
# Dependencies:
#   knowledge-base: main (commit: abc123..., consumed: 2026-01-04)

# 5. Explore available changes
uv run python -m graft changes knowledge-base
# Changes for knowledge-base:
#   v2.0.0 (feature)
#     New content structure
#     Migration: restructure
#   v1.5.0 (feature)
#     Additional examples

# 6. View change details
uv run python -m graft show knowledge-base@v2.0.0
# Change: knowledge-base@v2.0.0
# Type: feature
# Description: New content structure
# Migration: restructure
#   Command: ./scripts/migrate.sh
#   Description: Restructure content

# 7. Perform atomic upgrade
uv run python -m graft upgrade knowledge-base --to v2.0.0
# Upgrading knowledge-base → v2.0.0
# Migration completed:
#   Restructured 42 files
# Verification passed:
#   All files valid
# ✓ Upgrade complete
# Updated graft.lock: knowledge-base@v2.0.0

# 8. Verify upgrade
uv run python -m graft status
# Dependencies:
#   knowledge-base: v2.0.0 (commit: def456..., consumed: 2026-01-04)
```

## Troubleshooting

### "Dependency not found in configuration"

Ensure the dependency is declared in `graft.yaml`:
```yaml
deps:
  my-dep: "https://github.com/user/repo.git#main"
```

### "Lock file not found"

Run `graft apply <dep> --to <ref>` to create the initial lock file entry.

### "Git fetch failed"

For local-only repositories (no remote), this warning is expected and non-fatal. Graft will fall back to resolving refs from the local repository.

### "Snapshot path not found"

Ensure you have write permissions in the project directory. Snapshots are stored in `.graft/snapshots/`.

### Upgrade fails and doesn't rollback

If you see this, it's a bug. Graft should always rollback on failure. Please report with:
- The full command you ran
- The error message
- Contents of `graft.yaml` and `graft.lock`

## Testing Status

- **Tests**: 278 passing
- **Coverage**: 61% overall (service layer: 80-100%, CLI: 0%)
- **Linting**: All critical checks passing
- **Dogfooded**: Yes - graft manages its own dependency (graft-knowledge)

## Known Limitations

### Not Yet Implemented

1. **JSON Output**: Commands don't support `--format json`
2. **Dry Run**: Upgrade doesn't support `--dry-run` preview
3. **Update Checking**: Status doesn't support `--check-updates`
4. **Fetch Command**: No `graft fetch` to update remote cache
5. **Validate Command**: No `graft validate` for consistency checking

### Design Decisions

1. **Snapshot Only Lock File**: We only snapshot `graft.lock`, not dependency directories
   - Dependency directories are managed by git
   - Migration commands may modify consumer files (unpredictable)

2. **Required --to Flag**: Makes upgrades explicit and safer
   - User must specify target version
   - Prevents accidental upgrades

## Contributing

This project follows Python best practices:

- **Type hints** on all functions
- **Docstrings** on all public APIs
- **Unit tests** for all service functions
- **Integration tests** for adapters
- **Fakes over mocks** for testing
- **Immutable domain models**

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for architectural details.

## Documentation

- [COMPLETE_WORKFLOW.md](COMPLETE_WORKFLOW.md) - End-to-end workflow guide
- [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) - Implementation details
- [PHASE_8_IMPLEMENTATION.md](PHASE_8_IMPLEMENTATION.md) - CLI implementation
- [CONTINUE_HERE.md](CONTINUE_HERE.md) - Development session notes

## License

MIT License - see LICENSE file for details.

## Resources

- **Specification**: See `/home/coder/graft-knowledge/docs/specification/`
- **Issue Tracker**: TBD
- **Discussions**: TBD

---

**Status**: Production ready (9/10 phases complete)
