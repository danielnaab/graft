# Graft

Semantic dependency management for knowledge bases

---

## What is Graft?

Graft manages dependencies between knowledge bases and structured content repositories. It provides atomic upgrades with automatic rollback, migration execution, and semantic versioning for content changes.

Think of it as a package manager for knowledge: track versions, execute migrations, and safely upgrade between semantic releases.

## Key Features

- **Atomic Upgrades** - All-or-nothing upgrades with automatic rollback on failure
- **Semantic Changes** - Explicitly track breaking changes, features, and fixes
- **Migration Support** - Execute and verify migration commands during upgrades
- **Lock File Management** - Pin exact versions with commit hashes for reproducibility
- **Git-Based** - Works with any git repository as a dependency
- **CLI Interface** - Simple command-line interface with clear output

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd graft

# Install dependencies
uv sync

# Verify installation
uv run python -m graft --help
```

**Requirements**: Python 3.11+, [uv](https://docs.astral.sh/uv/), git

## Quick Start

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

# 5. Upgrade to a new version
uv run python -m graft upgrade my-knowledge --to v2.0.0
```

See [docs/guides/USER_GUIDE.md](docs/guides/USER_GUIDE.md) for detailed tutorials and workflows.

## Documentation

### For Users

- **[User Guide](docs/guides/USER_GUIDE.md)** - Step-by-step tutorials and common workflows
- **[CLI Reference](docs/CLI_REFERENCE.md)** - Complete command documentation
- **[Configuration Guide](docs/CONFIGURATION.md)** - graft.yaml and graft.lock format details

### For Contributors

- **[Architecture Overview](docs/README.md)** - System design and implementation details
- **[Working with Graft](docs/guides/WORKING_WITH_GRAFT.md)** - Development workflow and patterns
- **[Current Status](TASKS.md)** - Development status and roadmap

### Quick Links

- **Getting started?** Read the [Quick Start](#quick-start) above, then the [User Guide](docs/guides/USER_GUIDE.md)
- **Contributing code?** Review [docs/README.md](docs/README.md) and [WORKING_WITH_GRAFT.md](docs/guides/WORKING_WITH_GRAFT.md)
- **Starting a session?** Check [CONTINUE_HERE.md](CONTINUE_HERE.md) for recent context

## Core Concepts

**Dependencies** - Git repositories that your knowledge base depends on, declared in `graft.yaml`

**Changes** - Semantic versioned modifications (breaking/feature/fix) with optional migration commands

**Lock File** - Records exact consumed versions with commit hashes in `graft.lock`

**Atomic Upgrades** - Upgrades that execute migrations, run verifications, and automatically rollback on any failure

See [docs/guides/USER_GUIDE.md](docs/guides/USER_GUIDE.md#core-concepts) for detailed explanations.

## Project Status

- **Tests**: 322 passing
- **Coverage**: 45% (service layer: 80-100%)
- **Type Checking**: mypy strict mode enabled and passing
- **Linting**: All checks passing (ruff)
- **Status**: Production ready - All core features complete

## Contributing

This project follows clean architecture principles with protocol-based dependency injection, functional service layers, and immutable domain models.

Read [docs/README.md](docs/README.md) for architectural details and [docs/guides/WORKING_WITH_GRAFT.md](docs/guides/WORKING_WITH_GRAFT.md) for development workflow.

**Code Quality Standards**:
- Type hints on all functions (mypy strict)
- Unit tests for all services
- Fakes over mocks for testing
- Professional documentation (plain language, no emojis)

## License

MIT License - see LICENSE file for details.

---

**Links**: [Documentation](docs/) | [User Guide](docs/guides/USER_GUIDE.md) | [CLI Reference](docs/CLI_REFERENCE.md) | [Architecture](docs/README.md)
