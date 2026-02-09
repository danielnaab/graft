---
title: "Decision 0001: Define initial scope"
status: working
date: 2025-12-23
---

# Decision 0001: Define initial scope

## Context

The Graft project needs a clearly defined initial scope to ensure the tool is:
- Deliverable within reasonable effort
- Focused on core value proposition
- Extensible for future enhancements

Many existing tools exist in adjacent spaces (Make, Task, Bazel, Nix, npm, etc.), so Graft must find a unique position.

## Decision

Graft's initial scope focuses on two core capabilities:

1. **Task Runner**: Configurable tasks/commands defined in a simple YAML format
2. **Git-Centered Dependency Manager**: Dependencies specified via git repository + ref

This deliberately excludes:
- Full build system capabilities (like Bazel)
- Complex incremental build logic
- Language-specific package management
- Hermetic execution environments (like Nix)

## Rationale

**Why start narrow:**
- Easier to ship a working v0.1
- Focused value proposition is clearer to users
- Can validate core assumptions before building complexity

**Why task runner + git deps:**
- Task runner provides immediate utility (replaces Makefiles, package.json scripts)
- Git deps solve real pain: sharing code across projects without publishing
- These two features complement each other naturally

**Why git-centered:**
- Git already solves versioning and distribution
- Commit SHAs provide perfect content addressing
- No need to build/maintain package registries

## Consequences

**Positive:**
- Clear MVP scope
- Simpler implementation and maintenance
- Can iterate based on real usage patterns

**Negative:**
- Won't replace full build systems initially
- May need to add features later based on user needs
- Git dependency resolution could be slow without caching

**Neutral:**
- Need to design with extensibility in mind
- Should document what's explicitly out of scope

## Sources

- This decision emerged from comparing existing tools and identifying gaps
- See [architecture.md](../architecture.md) for current design
