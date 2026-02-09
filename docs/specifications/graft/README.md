# Graft Specifications

Formal specifications for the graft dependency management system.

## Status

All specifications are currently **draft** status, defining version **v3** of graft (flat-only dependency model).

## Specifications

### Core Specifications

- [**Graft YAML Format**](./graft-yaml-format.md) - Structure and semantics of graft.yaml files
- [**Lock File Format**](./lock-file-format.md) - Structure and semantics of graft.lock files
- [**Core Operations**](./core-operations.md) - Apply, upgrade, and update operations
- [**Change Model**](./change-model.md) - How dependencies specify and apply changes

### Layout and Notifications

- [**Dependency Layout**](./dependency-layout.md) - How dependency content is organized on disk
- [**Dependency Update Notification**](./dependency-update-notification.md) - How consumers learn about available updates

## Format

These specifications use a formal markdown format with:

- **YAML frontmatter** - Status, version, and metadata
- **Overview** - Summary and purpose
- **Schema/Structure** - Detailed field definitions
- **Examples** - Concrete usage examples
- **Validation** - Rules and pseudocode
- **Related** - Links to related specifications

The format is detailed and implementer-focused, suitable for generating code from or validating implementations against.

## Version History

- **v3** (current): Flat-only dependency model with git submodules
- **v2** (deprecated): Tree-based dependency layout
- **v1** (deprecated): Initial design

## Related Documentation

- [../architecture.md](../architecture.md) - Graft system architecture
- [../use-cases.md](../use-cases.md) - Use cases driving graft design
- [../decisions/](../decisions/) - Architecture decision records
