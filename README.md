# Graft

A task runner and git-centered package manager for reproducible development workflows.

## Overview

Graft combines two core capabilities:
1. **Task Runner**: Define and execute configurable tasks in YAML
2. **Git-Centered Dependencies**: Manage dependencies via git repositories

## Quick Start

```bash
# Resolve and fetch dependencies
graft resolve

# Run a task
graft run <task-name>
```

## Documentation

- **Specifications**: See [graft-knowledge](../graft-knowledge) for architecture and design decisions
- **Implementation Docs**: See [docs/](docs/) for code structure and development guides
- **Knowledge Base Config**: [knowledge-base.yaml](knowledge-base.yaml)

## Project Structure

```
graft/
├── docs/              # Implementation documentation
├── notes/             # Development notes and logs
├── knowledge-base.yaml  # KB configuration
└── graft.yaml         # Dependencies
```

## Development

See [docs/README.md](docs/README.md) for development setup and contribution guidelines.

## Status

Early development - core architecture being established.

## Related Repositories

- [graft-knowledge](../graft-knowledge) - Specifications and architecture
- [meta-knowledge-base](../meta-knowledge-base) - KB methodology

## License

TBD
