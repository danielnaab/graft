---
title: "Decision 0005: No Partial Dependency Resolution"
date: 2026-01-05
status: superseded
superseded_by: decision-0007-flat-only-dependencies.md
superseded_date: 2026-01-31
---

# Decision 0005: No Partial Dependency Resolution

> **Note**: This decision has been **superseded** by [Decision 0007: Flat-Only Dependency Model](./decision-0007-flat-only-dependencies.md).
>
> With the adoption of flat-only dependencies (no transitive resolution), the concept of "partial resolution" no longer applies. In the flat-only model, there are no transitive dependencies to partially resolve - only direct dependencies explicitly declared in `graft.yaml` are resolved.
>
> This decision remains as historical context for the reasoning against partial resolution when transitive dependencies were part of the design.

## Context

Some package managers (like npm with `npm install --production` or cargo with `cargo build --no-dev-dependencies`) support partial dependency resolution - installing only a subset of dependencies based on context or flags.

For Graft, this might mean:
- Resolving only specific dependencies from graft.yaml
- Skipping transitive dependencies
- Conditional dependency resolution based on environment or profile

## Decision

**Graft WILL NOT support partial dependency resolution.**

All dependencies declared in `graft.yaml` and their complete transitive dependency graphs MUST be resolved together. The lock file MUST represent the complete, resolved dependency graph.

## Rationale

### Core Principle Violations

**1. Atomicity**

Graft's design principle: "All-or-nothing operations, no partial states"

Partial resolution creates partial states where:
- Some dependencies are present, others missing
- Different machines might resolve different subsets
- Unclear what "resolved" means

**2. Reproducibility**

The lock file must enable identical states across environments. Partial resolution breaks this:

```yaml
# Machine A: graft resolve --only meta-kb
.graft/
  meta-kb/
  # Missing: standards-kb, templates-kb

# Machine B: graft resolve (full)
.graft/
  meta-kb/
  standards-kb/
  templates-kb/

# Different states from same graft.yaml + graft.lock!
```

**3. Explicitness**

"What is needed?" becomes ambiguous:
- Needed for what purpose?
- Determined by whom - user, tool, or dependency?
- Changes based on context or flags

This adds implicit behavior that violates "Explicit Over Implicit".

### Practical Problems

**1. Unclear Semantics**

```bash
# What does this mean?
graft resolve --only meta-kb

# Options:
# A) Resolve meta-kb, skip its dependencies?
#    -> Breaks dependency graph, meta-kb can't function
# B) Resolve meta-kb + its direct deps only?
#    -> What about their transitive deps?
# C) Resolve meta-kb + complete transitive graph?
#    -> Not partial resolution then!
```

**2. Knowledge Base Interdependencies**

Unlike code libraries, knowledge bases often have tight semantic coupling:

```markdown
<!-- In your KB -->
See [Architecture Pattern](../.graft/meta-kb/patterns.md)

<!-- In meta-kb -->
Uses template from [Templates](../.graft/templates-kb/header.md)

<!-- If templates-kb not resolved: broken links, incomplete content -->
```

Partial resolution would create broken knowledge graphs.

**3. Conflict with Dependency Graph Integrity**

Graft tracks complete dependency relationships via `requires`/`required_by`:

```yaml
dependencies:
  meta-kb:
    requires: ["standards-kb"]
  standards-kb:
    required_by: ["meta-kb"]
```

Partial resolution breaks these invariants - can't have meta-kb without standards-kb.

### No Demonstrated Need

**Performance is not a problem**:
- Typical knowledge base projects: 5-20 dependencies
- Resolution time: <10 seconds (including git clones)
- Parallelization can improve this further

**Disk space is not a problem**:
- Knowledge bases are small (typically <100MB per dep)
- Modern systems have ample storage
- If needed, optimize full resolution rather than support partial

**Use case unclear**:
- No user feedback requesting this
- No concrete scenario where partial resolution solves real problem
- Hypothetical optimization for non-existent bottleneck

## Alternatives Considered

### Alternative 1: Conditional Dependencies

Support environment-specific dependencies:

```yaml
deps:
  core-kb: "..."

dev_deps:
  examples-kb: "..."  # Only in development
```

**Evaluation**:
- More targeted than partial resolution
- Still adds complexity and new concepts
- Not needed for knowledge bases (no build/runtime distinction like code)
- **Decision**: Defer - can add later if proven demand exists

### Alternative 2: Lazy Resolution

Resolve dependencies on first access:

```bash
# Access triggers resolution
cat .graft/meta-kb/doc.md
# Auto-resolves meta-kb if not present
```

**Evaluation**:
- Violates explicitness - magical behavior
- Race conditions and state management complexity
- Unclear when/how lock file updates
- **Decision**: Reject - too implicit

### Alternative 3: Workspace/Profile Support

Different graft.yaml files for different contexts:

```
project/
  graft.yaml           # Full dependencies
  graft.minimal.yaml   # Minimal subset
```

**Evaluation**:
- Explicit - you choose which config to use
- No new resolution semantics needed
- Can be done today without tool support
- **Decision**: Document this pattern as a workaround if needed

### Alternative 4: Optimize Full Resolution

Instead of partial resolution, make full resolution faster:
- Parallel git cloning
- Incremental updates (fetch vs. clone)
- Local dependency caching
- Content-addressed storage (pnpm-style)

**Evaluation**:
- Maintains all-or-nothing semantics
- Actually solves performance concerns if they arise
- Aligns with existing principles
- **Decision**: This is the right path if performance becomes an issue

## Consequences

### Positive

- **Simplicity**: One resolution mode, one semantic
- **Predictability**: Always get complete dependency graph
- **Correctness**: No broken references or incomplete content
- **Maintainability**: Less code, fewer edge cases

### Negative

- **No flexibility**: Can't skip dependencies even if wanted
- **Potential performance**: Full resolution required even for minimal use
  - *Mitigated by*: Resolution is already fast, can optimize further if needed

### Neutral

- **Not a limitation in practice**: No demonstrated use case requires this
- **Can revisit**: If compelling use case emerges with clear semantics

## Implementation

### Validation

Tools SHOULD validate that all dependencies in lock file match .graft/ directory:

```bash
# All dependencies must be present
graft validate --check-deps

# Error if mismatch
Error: Lock file inconsistency
  Lock file contains: meta-kb, standards-kb, templates-kb
  .graft/ contains: meta-kb, standards-kb
  Missing: templates-kb
```

### Error Handling

If user manually deletes dependencies from .graft/:

```bash
$ rm -rf .graft/templates-kb

$ graft validate
Error: Incomplete dependency resolution
  Dependency 'templates-kb' in lock file but missing from .graft/
  Run 'graft resolve' to restore complete dependency graph
```

## Future Considerations

If partial resolution becomes genuinely needed (with concrete use cases), reconsider with these constraints:

1. **Must maintain atomicity** - Clear all-or-nothing semantics
2. **Must preserve reproducibility** - Lock file unambiguous
3. **Must be explicit** - No implicit/automatic subset selection
4. **Must handle interdependencies** - No broken links

Likely approach would be environment-specific lock files:
```
graft.yaml
graft.lock          # Full resolution
graft.minimal.lock  # Explicit minimal subset
```

## Related

- [Decision 0004: Atomic Upgrades](./decision-0004-atomic-upgrades.md) - Atomicity principle
- [Decision 0003: Explicit Change Declarations](./decision-0003-explicit-change-declarations.md) - Explicitness principle
- [Lock File Format Specification](../graft/lock-file-format.md)

## References

- **All-or-nothing transactions**: https://en.wikipedia.org/wiki/Atomicity_(database_systems)
- **Reproducible builds**: https://reproducible-builds.org/
- **npm install options**: Shows partial install complexity
