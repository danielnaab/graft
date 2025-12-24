---
title: "Development Setup"
status: draft
---

# Development Setup

## Prerequisites

TBD - will be documented as implementation language and tooling are chosen.

## Getting Started

### 1. Clone repositories

```bash
# Clone main implementation repo
git clone ssh://forgejo@platform-vm:2222/daniel/graft.git
cd graft

# Clone specification repo (for reference)
git clone ssh://forgejo@platform-vm:2222/daniel/graft-knowledge.git ../graft-knowledge

# Clone meta-KB (for methodology)
git clone ssh://forgejo@platform-vm:2222/daniel/meta-knowledge-base.git ../meta-knowledge-base
```

### 2. Set up local development

Create `graft.local.yaml` for local path overrides:

```yaml
apiVersion: graft/v0
deps:
  graft-knowledge: "/home/coder/graft-knowledge"
```

**Note**: `graft.local.yaml` is gitignored and used only for local development.

### 3. Resolve dependencies

```bash
# Once graft is implemented:
graft resolve
```

## Running Tasks

```bash
# Once tasks are defined in graft.yaml:
graft run <task-name>
```

## Testing

TBD

## Contributing

1. Review specifications in [graft-knowledge](../../graft-knowledge/docs/README.md)
2. Follow the [agent workflow](../../meta-knowledge-base/playbooks/agent-workflow.md)
3. Update implementation docs as code evolves

## Sources

- Specifications: [graft-knowledge](../../graft-knowledge/docs/README.md)
- Meta-KB methodology: [meta.md](../../meta-knowledge-base/docs/meta.md)
