---
title: "Graft Code Structure"
status: draft
---

# Graft Code Structure

## Overview

This document describes the organization of the Graft codebase.

## Current Structure

```
graft/
├── docs/              # Implementation documentation (this KB)
│   ├── README.md      # Human entrypoint
│   ├── agents.md      # Agent entrypoint
│   ├── structure.md   # This file
│   └── development.md # Development setup
├── notes/             # Weekly development logs
├── knowledge-base.yaml  # KB configuration
├── graft.yaml         # Dependencies
├── graft.local.yaml   # Local path overrides (gitignored)
└── README.md          # Project overview
```

## Future Structure (Planned)

As implementation progresses, additional directories will be added:

```
graft/
├── cmd/               # CLI entry points
├── pkg/               # Public library packages
├── internal/          # Private application code
├── docs/              # Documentation
├── tests/             # Test files
└── examples/          # Example configurations
```

## Module Organization

TBD - will be documented as implementation progresses.

## Sources

- Architecture specification: [graft-knowledge/docs/architecture.md](../../graft-knowledge/docs/architecture.md)
- Initial scope decision: [graft-knowledge/docs/decisions/decision-0001-initial-scope.md](../../graft-knowledge/docs/decisions/decision-0001-initial-scope.md)
