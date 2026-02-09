---
status: stable
updated: 2026-01-05
---

# Documentation Navigation

Complete index of all graft documentation.

---

## Quick Reference

| I want to... | Read this |
|--------------|-----------|
| **Understand what graft is** | [README.md](../README.md) |
| **Get started using graft** | [README.md](../README.md#quick-start) → [user-guide.md](guides/user-guide.md) |
| **Look up a CLI command** | [cli-reference.md](cli-reference.md) |
| **Understand graft.yaml format** | [configuration.md](configuration.md) |
| **Start contributing code** | [contributing.md](guides/contributing.md) |
| **Understand the architecture** | [docs/README.md](README.md) |
| **Continue a development session** | [continue-here.md](../continue-here.md) |
| **See what's implemented** | [tasks.md](../tasks.md) |
| **Understand a design decision** | [decisions/](decisions/) (ADRs) |

---

## Documentation Structure

```
graft/
├── README.md                    # Project introduction and index
├── continue-here.md             # Session handoff document
├── tasks.md                     # Development status
├── docs/
│   ├── index.md            # This file - documentation index
│   ├── README.md                # Architecture and developer docs
│   ├── cli-reference.md         # Complete command reference
│   ├── configuration.md         # graft.yaml and graft.lock formats
│   ├── guides/
│   │   ├── user-guide.md        # Step-by-step tutorials
│   │   └── contributing.md # Development workflow
│   ├── decisions/               # Architectural decision records
│   │   ├── 001-require-explicit-ref-in-upgrade.md
│   │   ├── 002-filesystem-snapshots-for-rollback.md
│   │   ├── 003-snapshot-only-lock-file.md
│   │   ├── 004-protocol-based-dependency-injection.md
│   │   └── 005-functional-service-layer.md
│   └── status/                  # Implementation status tracking
│       ├── workflow-validation.md
│       ├── implementation.md
│       └── ...
└── notes/                       # Time-bounded development notes
```

---

## User Documentation

### For First-Time Users

1. **[README.md](../README.md)** - Start here
   - What is graft?
   - Key features
   - Installation
   - Quick start
   - Links to detailed docs

2. **[user-guide.md](guides/user-guide.md)** - Comprehensive guide
   - Getting started tutorial
   - Core concepts explained
   - 7 common workflows
   - Troubleshooting guide
   - Best practices
   - Advanced topics

3. **[cli-reference.md](cli-reference.md)** - Command reference
   - All 9 commands documented
   - Options and flags
   - Examples for each command
   - Use cases

4. **[configuration.md](configuration.md)** - File format reference
   - graft.yaml format and fields
   - graft.lock format
   - Examples and best practices

### For Regular Users

- **Quick command lookup**: [cli-reference.md](cli-reference.md)
- **Configuration questions**: [configuration.md](configuration.md)
- **Workflow help**: [user-guide.md](guides/user-guide.md#common-workflows)
- **Troubleshooting**: [user-guide.md](guides/user-guide.md#troubleshooting)

---

## Developer Documentation

### For New Contributors

1. **[README.md](../README.md)** - Project overview
2. **[contributing.md](guides/contributing.md)** - Development guide
   - Codebase orientation
   - Essential patterns
   - Common tasks
   - Documentation update protocol
   - Quality standards
3. **[docs/README.md](README.md)** - Architecture details
   - System design
   - Domain models
   - Services
   - Protocols and adapters
   - Testing approach

### For Active Contributors

- **Session handoff**: [continue-here.md](../continue-here.md)
- **Current status**: [tasks.md](../tasks.md)
- **Architecture**: [docs/README.md](README.md)
- **Design decisions**: [decisions/](decisions/)
- **Implementation notes**: [status/implementation.md](status/implementation.md)

### For Architectural Context

Read the ADRs (Architectural Decision Records):

1. **[001-require-explicit-ref-in-upgrade.md](decisions/001-require-explicit-ref-in-upgrade.md)**
   - Why `--to` flag is required for upgrades

2. **[002-filesystem-snapshots-for-rollback.md](decisions/002-filesystem-snapshots-for-rollback.md)**
   - Rollback mechanism design

3. **[003-snapshot-only-lock-file.md](decisions/003-snapshot-only-lock-file.md)**
   - What gets snapshotted and why

4. **[004-protocol-based-dependency-injection.md](decisions/004-protocol-based-dependency-injection.md)**
   - Dependency injection approach

5. **[005-functional-service-layer.md](decisions/005-functional-service-layer.md)**
   - Why services are functions, not classes

---

## Documentation by Purpose

### Understand the Project

- **What is graft?** → [README.md](../README.md)
- **Why use graft?** → [README.md](../README.md#key-features)
- **Is graft ready?** → [tasks.md](../tasks.md), [README.md](../README.md#project-status)

### Learn to Use Graft

- **Installation** → [README.md](../README.md#installation)
- **First steps** → [README.md](../README.md#quick-start)
- **Detailed tutorials** → [user-guide.md](guides/user-guide.md)
- **Command reference** → [cli-reference.md](cli-reference.md)
- **Configuration** → [configuration.md](configuration.md)

### Contribute Code

- **Get started** → [contributing.md](guides/contributing.md)
- **Architecture** → [docs/README.md](README.md)
- **Patterns** → [contributing.md](guides/contributing.md#essential-patterns)
- **Quality standards** → [contributing.md](guides/contributing.md#code-quality-standards)

### Continue Development

- **Current state** → [continue-here.md](../continue-here.md)
- **Recent work** → [tasks.md](../tasks.md)
- **What to do next** → [tasks.md](../tasks.md#backlog-not-prioritized)

### Understand Design Decisions

- **All decisions** → [decisions/](decisions/)
- **Upgrade design** → [decisions/001-require-explicit-ref-in-upgrade.md](decisions/001-require-explicit-ref-in-upgrade.md)
- **Rollback design** → [decisions/002-filesystem-snapshots-for-rollback.md](decisions/002-filesystem-snapshots-for-rollback.md)
- **DI approach** → [decisions/004-protocol-based-dependency-injection.md](decisions/004-protocol-based-dependency-injection.md)
- **Service design** → [decisions/005-functional-service-layer.md](decisions/005-functional-service-layer.md)

---

## Documentation Maintenance

### When to Update Documentation

See the **Documentation Update Protocol** in [contributing.md](guides/contributing.md#documentation-update-protocol).

Quick reference:

| Change Type | Update These |
|-------------|--------------|
| Add CLI command | README.md, cli-reference.md, docs/README.md |
| Add service | docs/README.md |
| Add domain model | docs/README.md |
| Change architecture | docs/README.md, new ADR in decisions/ |
| Fix bug | No doc update (unless behavior changes) |
| Add feature | README.md, possibly user-guide.md |
| Update test count | README.md, continue-here.md |

### Documentation Quality Standards

All documentation follows these principles:

- **Plain language** - Clear, concrete, specific
- **Professional tone** - No emojis, no casual language
- **Well-structured** - Headings, short sections, progressive disclosure
- **Accurate** - All examples tested, all links verified
- **Maintained** - Updated when code changes

See [meta-knowledge-base style policy](../../meta-knowledge-base/policies/style.md) for full standards.

---

## External References

- **Specification**: `docs/specifications/graft/`
- **Meta-Knowledge-Base**: `/home/coder/meta-knowledge-base/docs/meta.md`

---

**Need help finding something?** This index should answer "where is X documented?" If you can't find what you need, consider:

1. Check if it exists: `grep -r "your search term" .`
2. It may need documentation: See [contributing.md](guides/contributing.md#documentation-update-protocol)
3. Ask for clarification

---

Last Updated: 2026-01-04
