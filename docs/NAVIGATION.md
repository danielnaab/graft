# Documentation Navigation

Complete index of all graft documentation.

---

## Quick Reference

| I want to... | Read this |
|--------------|-----------|
| **Understand what graft is** | [README.md](../README.md) |
| **Get started using graft** | [README.md](../README.md#quick-start) → [USER_GUIDE.md](guides/USER_GUIDE.md) |
| **Look up a CLI command** | [CLI_REFERENCE.md](CLI_REFERENCE.md) |
| **Understand graft.yaml format** | [CONFIGURATION.md](CONFIGURATION.md) |
| **Start contributing code** | [WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md) |
| **Understand the architecture** | [docs/README.md](README.md) |
| **Continue a development session** | [CONTINUE_HERE.md](../CONTINUE_HERE.md) |
| **See what's implemented** | [TASKS.md](../TASKS.md) |
| **Understand a design decision** | [decisions/](decisions/) (ADRs) |

---

## Documentation Structure

```
graft/
├── README.md                    # Project introduction and index
├── CONTINUE_HERE.md             # Session handoff document
├── TASKS.md                     # Development status
├── docs/
│   ├── NAVIGATION.md            # This file - documentation index
│   ├── README.md                # Architecture and developer docs
│   ├── CLI_REFERENCE.md         # Complete command reference
│   ├── CONFIGURATION.md         # graft.yaml and graft.lock formats
│   ├── guides/
│   │   ├── USER_GUIDE.md        # Step-by-step tutorials
│   │   └── WORKING_WITH_GRAFT.md # Development workflow
│   ├── decisions/               # Architectural decision records
│   │   ├── 001-require-explicit-ref-in-upgrade.md
│   │   ├── 002-filesystem-snapshots-for-rollback.md
│   │   ├── 003-snapshot-only-lock-file.md
│   │   ├── 004-protocol-based-dependency-injection.md
│   │   └── 005-functional-service-layer.md
│   └── status/                  # Implementation status tracking
│       ├── COMPLETE_WORKFLOW.md
│       ├── IMPLEMENTATION_STATUS.md
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

2. **[USER_GUIDE.md](guides/USER_GUIDE.md)** - Comprehensive guide
   - Getting started tutorial
   - Core concepts explained
   - 7 common workflows
   - Troubleshooting guide
   - Best practices
   - Advanced topics

3. **[CLI_REFERENCE.md](CLI_REFERENCE.md)** - Command reference
   - All 9 commands documented
   - Options and flags
   - Examples for each command
   - Use cases

4. **[CONFIGURATION.md](CONFIGURATION.md)** - File format reference
   - graft.yaml format and fields
   - graft.lock format
   - Examples and best practices

### For Regular Users

- **Quick command lookup**: [CLI_REFERENCE.md](CLI_REFERENCE.md)
- **Configuration questions**: [CONFIGURATION.md](CONFIGURATION.md)
- **Workflow help**: [USER_GUIDE.md](guides/USER_GUIDE.md#common-workflows)
- **Troubleshooting**: [USER_GUIDE.md](guides/USER_GUIDE.md#troubleshooting)

---

## Developer Documentation

### For New Contributors

1. **[README.md](../README.md)** - Project overview
2. **[WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md)** - Development guide
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

- **Session handoff**: [CONTINUE_HERE.md](../CONTINUE_HERE.md)
- **Current status**: [TASKS.md](../TASKS.md)
- **Architecture**: [docs/README.md](README.md)
- **Design decisions**: [decisions/](decisions/)
- **Implementation notes**: [status/IMPLEMENTATION_STATUS.md](status/IMPLEMENTATION_STATUS.md)

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
- **Is graft ready?** → [TASKS.md](../TASKS.md), [README.md](../README.md#project-status)

### Learn to Use Graft

- **Installation** → [README.md](../README.md#installation)
- **First steps** → [README.md](../README.md#quick-start)
- **Detailed tutorials** → [USER_GUIDE.md](guides/USER_GUIDE.md)
- **Command reference** → [CLI_REFERENCE.md](CLI_REFERENCE.md)
- **Configuration** → [CONFIGURATION.md](CONFIGURATION.md)

### Contribute Code

- **Get started** → [WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md)
- **Architecture** → [docs/README.md](README.md)
- **Patterns** → [WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md#essential-patterns)
- **Quality standards** → [WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md#code-quality-standards)

### Continue Development

- **Current state** → [CONTINUE_HERE.md](../CONTINUE_HERE.md)
- **Recent work** → [TASKS.md](../TASKS.md)
- **What to do next** → [TASKS.md](../TASKS.md#backlog-not-prioritized)

### Understand Design Decisions

- **All decisions** → [decisions/](decisions/)
- **Upgrade design** → [decisions/001-require-explicit-ref-in-upgrade.md](decisions/001-require-explicit-ref-in-upgrade.md)
- **Rollback design** → [decisions/002-filesystem-snapshots-for-rollback.md](decisions/002-filesystem-snapshots-for-rollback.md)
- **DI approach** → [decisions/004-protocol-based-dependency-injection.md](decisions/004-protocol-based-dependency-injection.md)
- **Service design** → [decisions/005-functional-service-layer.md](decisions/005-functional-service-layer.md)

---

## Documentation Maintenance

### When to Update Documentation

See the **Documentation Update Protocol** in [WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md#documentation-update-protocol).

Quick reference:

| Change Type | Update These |
|-------------|--------------|
| Add CLI command | README.md, CLI_REFERENCE.md, docs/README.md |
| Add service | docs/README.md |
| Add domain model | docs/README.md |
| Change architecture | docs/README.md, new ADR in decisions/ |
| Fix bug | No doc update (unless behavior changes) |
| Add feature | README.md, possibly USER_GUIDE.md |
| Update test count | README.md, CONTINUE_HERE.md |

### Documentation Quality Standards

All documentation follows these principles:

- **Plain language** - Clear, concrete, specific
- **Professional tone** - No emojis, no casual language
- **Well-structured** - Headings, short sections, progressive disclosure
- **Accurate** - All examples tested, all links verified
- **Maintained** - Updated when code changes

See [meta-knowledge-base style policy](file:///home/coder/meta-knowledge-base/policies/style.md) for full standards.

---

## External References

- **Specification**: `/home/coder/graft-knowledge/docs/specification/`
- **Meta-Knowledge-Base**: `/home/coder/meta-knowledge-base/docs/meta.md`

---

**Need help finding something?** This index should answer "where is X documented?" If you can't find what you need, consider:

1. Check if it exists: `grep -r "your search term" .`
2. It may need documentation: See [WORKING_WITH_GRAFT.md](guides/WORKING_WITH_GRAFT.md#documentation-update-protocol)
3. Ask for clarification

---

Last Updated: 2026-01-04
