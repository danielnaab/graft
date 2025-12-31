---
title: Graft Documentation
status: working
---

# Graft Documentation

Knowledge base tooling with language server support.

**Documentation Authority**: Specifications and architectural decisions ("what to build" and "why") are maintained in [graft-knowledge](../../graft-knowledge). This KB contains implementation documentation ("how it's built") including code structure, development guides, and implementation notes.

## Architecture & Patterns

This project uses the [Python Starter Template](../python-starter) patterns and conventions.

**Core architectural patterns**:
- [Functional Service Layer](../python-starter/docs/architecture/functional-services.md) - Services as pure functions
- [Protocol-based DI](../python-starter/docs/architecture/dependency-injection.md) - Structural typing for flexibility
- [Domain Modeling](../python-starter/docs/architecture/domain-model.md) - Entities and value objects
- [Testing Strategy](../python-starter/docs/architecture/testing-strategy.md) - Unit tests with fakes, integration tests

**Template documentation** (comprehensive guides and references):
- [Architecture](../python-starter/docs/architecture/) - Detailed architectural documentation
- [Decisions (ADRs)](../python-starter/docs/decisions/) - Architectural decision records
- [Development Guides](../python-starter/docs/guides/) - How-to guides for common tasks
- [Technical Reference](../python-starter/docs/reference/) - Reference documentation

## Quick Start

1. **Install dependencies**:
   ```bash
   uv sync
   ```

2. **Run the CLI**:
   ```bash
   uv run graft --help
   uv run graft version
   ```

3. **Run tests**:
   ```bash
   uv run pytest
   ```

See the [Getting Started Guide](../python-starter/docs/guides/getting-started.md) for more details.

## Project Structure

This project follows the standard Python Starter Template layout:

```
graft/
├── src/graft/              # Source code
│   ├── domain/             # Domain entities and value objects
│   ├── services/           # Service functions with context
│   ├── adapters/           # External system implementations
│   ├── protocols/          # Interface definitions
│   └── cli/                # CLI commands
├── tests/                  # Test suite
│   ├── unit/               # Unit tests
│   ├── integration/        # Integration tests
│   └── fakes/              # Fake implementations for testing
└── docs/                   # Project documentation
```

See [Project Structure Reference](../python-starter/docs/reference/project-structure.md) for complete details.

## Development Workflow

- **Adding features**: See [Adding Features Guide](../python-starter/docs/guides/adding-features.md)
- **Writing tests**: See [Testing Guide](../python-starter/docs/guides/testing-guide.md)
- **Development workflow**: See [Development Workflow](../python-starter/docs/guides/development-workflow.md)
- **CLI usage**: See [CLI Usage Guide](../python-starter/docs/guides/cli-usage.md)

## Graft Commands

### graft resolve

Resolves dependencies specified in `graft.yaml` by cloning or fetching git repositories.

**Usage**:
```bash
graft resolve
```

**graft.yaml format**:
```yaml
apiVersion: graft/v0
deps:
  dependency-name: "git-url#ref"
```

**Example**:
```yaml
apiVersion: graft/v0
deps:
  graft-knowledge: "ssh://git@example.com/user/graft-knowledge.git#main"
  python-starter: "https://github.com/user/python-starter.git#v1.0.0"
```

**Error Handling**:

Graft provides clear, actionable error messages:

- **Missing graft.yaml**: Tells you where it was expected and how to create it
- **Invalid YAML syntax**: Shows the syntax error with suggestions
- **Authentication errors**: Provides SSH key configuration guidance
- **Repository not found**: Suggests verifying the URL
- **Partial failures**: Continues resolving other dependencies

For details, see [Error Handling ADR](decisions/001-error-handling-strategy.md).

## Graft-Specific Documentation

- **Agent Entrypoint**: [agents.md](agents.md) - For AI agents working on this project
- **Architecture Decisions**: [decisions/](decisions/) - ADRs for graft-specific decisions
- **Implementation Notes**: [../notes/](../notes/) - Time-bounded development notes
- **Knowledge Base Config**: [../knowledge-base.yaml](../knowledge-base.yaml) - Project KB configuration
- **Specifications**: [../../graft-knowledge](../../graft-knowledge) - Graft specifications and architecture decisions

## Tooling

This project uses:
- **uv** - Fast Python package manager and environment management
- **ruff** - Lightning-fast linter and formatter
- **pytest** - Testing framework with coverage reporting
- **typer** - Modern CLI framework

See [Tooling Reference](../python-starter/docs/reference/tooling.md) for details.

## Template Information

This project uses the Python Starter Template for its development infrastructure.

To update from the template:
```bash
copier update --trust
```

See [TEMPLATE_STATUS.md](../TEMPLATE_STATUS.md) for template version information.

## Sources

- [Template Documentation](../python-starter/docs/)
- [Template Repository](../python-starter)
- [Graft Specifications](../../graft-knowledge)
- [Knowledge Base Config](../knowledge-base.yaml)
- [Meta Knowledge Base](../../meta-knowledge-base)
