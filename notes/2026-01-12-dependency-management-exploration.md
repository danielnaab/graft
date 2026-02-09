---
title: "Work Log: Dependency Management Architecture Exploration"
date: 2026-01-12
status: completed
---

# Dependency Management Architecture Exploration

## Objective

Reevaluate Graft's dependency management architecture by exploring two questions:
1. Should Graft use git submodules instead of direct checkouts to `.graft/`?
2. Should Graft clone transitive dependencies, or should upstream grafts create self-contained artifacts?

## Background

Graft uses a flat directory layout with direct git checkouts:
```
project/
├── graft.yaml
├── graft.lock
└── .graft/
    ├── meta-kb/        # Direct dependency
    ├── standards-kb/   # Transitive dependency
    └── templates-kb/   # Transitive dependency
```

This approach is documented in Decision 0005 (No Partial Resolution) and the Dependency Layout v2 specification. This exploration tests whether alternatives would better serve Graft's goals.

## Alternative 1: Git Submodules

Replace direct checkouts with git submodules for native git integration (`git clone --recurse-submodules`).

### Problems

1. **Nested structure breaks stable paths.** Transitive dependencies end up at `.graft/meta-kb/.graft/standards-kb/` instead of `.graft/standards-kb/`. References like `../.graft/standards-kb/` break.

2. **No deduplication.** If both `meta-kb` and `docs-kb` depend on `templates-kb`, submodules clone it twice in different locations.

3. **No conflict detection.** Different versions of the same dependency install silently in different locations.

4. **Flattening requires symlinks.** A post-checkout flattening step could create symlinks to restore stable paths, but Windows symlink support is inconsistent (requires Developer Mode or admin privileges).

### Assessment

Submodules would require custom flattening, a Windows compatibility layer, and a lock file for metadata anyway. The complexity does not justify the benefits.

## Alternative 2: Artifact-Based Composition

Instead of cloning transitive dependencies, upstream grafts bundle their dependencies into self-contained artifacts:
```
meta-kb-v2.0.0/
├── docs/
└── .bundled/
    ├── standards-kb/
    └── templates-kb/
```

Consumers clone only direct dependencies.

### Problems

1. **Lost traceability.** Bundled content has no git history. You cannot trace where content originated or see its evolution.

2. **Breaks collaboration.** To contribute to a transitive dependency, you must manually clone it separately. You cannot test changes in context.

3. **Duplication.** If `meta-kb` and `docs-kb` both bundle `templates-kb`, it exists twice.

4. **Conflicts with Decision 0005.** The spec requires complete transitive graphs. Artifacts create partial source availability, violating atomicity and reproducibility principles.

### Valid Use Case

Artifacts make sense for deployment (bundling final documentation for distribution), not for development workflow.

## Analysis

| Principle | Current | Git Submodules | Artifacts |
|-----------|---------|----------------|-----------|
| Git-Native | Yes | Yes | Partial |
| Explicit Over Implicit | Yes | Partial | No |
| Atomic Operations | Yes | Partial | No |
| Reproducibility | Yes | Partial | No |
| Composability | Yes | No | No |
| Minimal Primitives | Yes | No | No |

The current flat layout with transitive source cloning satisfies all six design principles. Both alternatives fail on multiple dimensions.

Key requirements that rule out alternatives:
- Stable reference paths (`../.graft/<name>/`) must work regardless of dependency graph structure
- Shared dependencies must be deduplicated
- Version conflicts must be detected explicitly
- Full source access needed for traceability and collaboration

## Decision

Continue with the current architecture. It correctly balances the competing concerns.

## Future Direction

A global cache (`~/.graft/cache/`) using hard links would reduce disk usage across projects without changing the architecture. This is consistent with patterns in pnpm and Cargo.

## Changelog

- 2026-01-12: Explored git submodules and artifact-based composition as alternatives. Both rejected due to conflicts with stable paths, deduplication, and design principles. Current architecture validated.
