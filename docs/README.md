---
title: Graft Documentation
status: stable
updated: 2026-01-05
---

# Graft Documentation

Semantic dependency management for knowledge bases.

> **Authority Note:** Developer-friendly implementation overview. For canonical architecture, see [specifications/architecture.md](specifications/architecture.md) and [specification ADRs](specifications/decisions/).

Graft provides atomic upgrades with automatic rollback, migration execution, and semantic versioning for git-based dependencies.

---

## Architecture

Clean architecture with protocols, immutable domain models, and pure functions.

**Domain Models** (`src/graft/domain/`):
- Change, Command, LockEntry, GraftConfig
- Frozen dataclasses (immutable)

**Services** (`src/graft/services/`):
- Query: status, changes, details
- Mutation: upgrade, lock, command execution
- Infrastructure: snapshot, config parsing
- Pure functions, protocol-based DI

**Protocols** (`src/graft/protocols/`):
- Snapshot, LockFile, CommandExecutor, Git, Repository, FileSystem
- Structural subtyping via `typing.Protocol`

**Adapters** (`src/graft/adapters/`):
- FilesystemSnapshot, YamlLockFile, SubprocessCommandExecutor
- GitAdapter, FileSystemAdapter, RepositoryAdapter

**CLI** (`src/graft/cli/commands/`):
- resolve, apply, status, changes, show, upgrade, fetch, exec

---

## Key Patterns

**Protocol-based DI** - structural subtyping, no runtime DI framework:
```python
def upgrade(snapshot: Snapshot, lock: LockFile, ...) -> Result:
    # Pure function, protocols injected
```

**Functional services** - pure functions, not classes:
```python
# Good
def parse_config(path: str) -> GraftConfig

# Not used
class ConfigService:
    def parse(self) -> GraftConfig
```

**Immutable domain** - frozen dataclasses:
```python
@dataclass(frozen=True)
class Change:
    ref: str
    type: ChangeType
```

**Atomic operations** - snapshot before, rollback on failure:
```python
snapshot = create_snapshot()
try:
    apply_migrations()
    update_lock()
except Exception:
    restore_snapshot()
```

---

## Project Structure

```
src/graft/
├── domain/          # Immutable models
├── services/        # Pure functions
├── protocols/       # Interfaces
├── adapters/        # Implementations
└── cli/             # Commands

tests/
├── unit/            # 12 modules, 150+ tests
├── integration/     # 4 modules, 800+ lines
└── fakes/           # Test doubles
```

---

## Development

**Setup:**
```bash
uv sync
uv run python -m graft --help
```

**Testing:**
```bash
pytest                      # All tests
pytest tests/unit           # Unit only
pytest -k test_upgrade      # Pattern
```

**Type checking:**
```bash
mypy src tests              # Strict mode enabled
```

**Linting:**
```bash
ruff check src tests
```

---

## Key Decisions

Architectural decisions documented in `docs/decisions/`:
- [Protocol-Based DI](decisions/004-protocol-based-dependency-injection.md)
- [Functional Service Layer](decisions/005-functional-service-layer.md)
- [Filesystem Snapshots](decisions/002-filesystem-snapshots-for-rollback.md)
- [Explicit Ref in Upgrade](decisions/001-require-explicit-ref-in-upgrade.md)

---

## Specifications

Canonical specifications for the Graft ecosystem live in [`specifications/`](specifications/):

- **[Graft Specifications](specifications/graft/)** - Formal specs for graft.yaml, lock file, core operations, change model, dependency layout
- **[Grove Specifications](specifications/grove/)** - Living specs for workspace management (architecture, workspace config)
- **[Specification Decisions](specifications/decisions/)** - ADRs 0001-0007 (scope, git refs, change model, atomicity, flat-only deps)
- **[Architecture Overview](specifications/architecture.md)** - System design and core concepts
- **[Use Cases](specifications/use-cases.md)** - What Graft enables and why

---

## Documentation

**User docs:**
- [User Guide](guides/user-guide.md) - tutorials and workflows
- [CLI Reference](cli-reference.md) - command documentation
- [Configuration](configuration.md) - file formats

**Developer docs:**
- [Contributing](guides/contributing.md) - development guide
- [Architecture](architecture.md) - detailed system design
- [Index](index.md) - navigation

**Plans:**
- [Upgrade to graft-knowledge v2](plans/upgrade-to-graft-knowledge-v2.md)
- [Upgrade Analysis](plans/upgrade-analysis.md) (pending implementation)
- [Graft Improvements Recommendations](plans/graft-improvements-recommendations.md) (pending implementation)

**Status:**
- [Implementation Status](../status/implementation.md)
- [Gap Analysis](../status/gap-analysis.md)
- [Continue Here](../continue-here.md)

---

## Sources

**Canonical Specifications:**
- [Graft Architecture](specifications/architecture.md)
- [ADR 004: Protocol-Based DI](decisions/004-protocol-based-dependency-injection.md)
- [ADR 005: Functional Services](decisions/005-functional-service-layer.md)
- [ADR 002: Filesystem Snapshots](decisions/002-filesystem-snapshots-for-rollback.md)

**Implementation:**
- Domain: `src/graft/domain/*.py`
- Services: `src/graft/services/*.py`
- Protocols: `src/graft/protocols/*.py`
- Adapters: `src/graft/adapters/*.py`
- CLI: `src/graft/cli/commands/*.py`

**Validation:**
- Tests: `tests/unit/` (12 modules), `tests/integration/` (4 modules)
- Workflow: [workflow-validation.md](../status/workflow-validation.md)
