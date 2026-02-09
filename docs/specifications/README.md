# Graft Project Knowledge Base

This KB follows the [meta knowledge base](../../.graft/meta-knowledge-base/docs/meta.md) system.

## Overview

Graft is a task runner and git-centered package manager that aims to simplify:
- Configurable task execution
- Git-based dependency management
- Reproducible development workflows

## Documentation Structure

- **Specifications** - All specifications (graft and grove)
  - **[Graft Specifications](graft/)** - Formal specifications for the graft dependency management system
    - [graft.yaml Format](graft/graft-yaml-format.md) - Configuration file format
    - [Lock File Format](graft/lock-file-format.md) - State tracking format
    - [Core Operations](graft/core-operations.md) - Operation semantics and behavior
    - [Change Model](graft/change-model.md) - Data model for changes
    - [Dependency Layout](graft/dependency-layout.md) - How dependencies are organized
    - [Dependency Update Notification](graft/dependency-update-notification.md) - Automated update propagation

  - **[Grove Specifications](grove/)** - Living specifications for the Grove workspace management tool
    - [Architecture](grove/architecture.md) - System design and three-layer architecture
    - [Workspace Configuration](grove/workspace-config.md) - workspace.yaml format

- **[Decisions](decisions/)** - Architecture Decision Records (ADRs)
  - Documents key architectural choices with rationale
  - Captures alternatives considered and trade-offs

- **[Architecture](architecture.md)** - Graft system design and core concepts overview

- **[Use Cases](use-cases.md)** - What Graft enables and why

- **[CHANGELOG](CHANGELOG.md)** - Track specification changes and additions

- **[Notes](../../notes/)** - Working notes, brainstorming, and design exploration

## For Implementers

See [CHANGELOG](CHANGELOG.md) for recent specification changes.

Implement against a specific version by referencing the git commit or tag.
See CHANGELOG for guidance on pinning implementations to specifications.
