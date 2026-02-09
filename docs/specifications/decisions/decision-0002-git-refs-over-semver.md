---
title: "Use Git Refs Over Semantic Versioning"
date: 2026-01-01
status: accepted
---

# Use Git Refs Over Semantic Versioning

## Context

Graft needs to identify and track changes in dependencies. We must decide what primitives to use for change identity.

Two primary approaches:
1. **Require semantic versioning** - Force dependencies to use semver tags (v1.0.0, v2.0.0, etc.)
2. **Use git refs directly** - Support any git reference (commits, branches, tags of any format)

This decision affects:
- What versioning strategies are supported
- How flexible Graft is for different workflows
- Whether we impose opinions on dependency maintainers
- The complexity of implementation

## Decision

**Graft will use git refs (commits, branches, tags) as the identity for changes, without requiring semantic versioning.**

Changes are identified by any valid git reference:
- Commit hashes (`abc123def456`)
- Branches (`main`, `stable`, `feature-auth`)
- Tags in any format (`v2.0.0`, `release-2026-01`, `r42`)

Semantic versioning is an **optional convention** that Graft can recognize and provide helpers for, but is not required for core functionality.

## Alternatives Considered

### Alternative 1: Require Semantic Versioning

**Approach**: Force all dependencies to use semver tags (v1.0.0, v2.0.0, etc.)

**Pros**:
- Clear breaking change detection (major version bumps)
- Standardized version ordering
- Well-understood conventions
- Could validate compliance automatically

**Cons**:
- Excludes valid workflows (date-based releases, sequential numbering, etc.)
- Forces opinions on dependency maintainers
- Doesn't work for commit-granular tracking
- Incompatible with branch-based workflows
- Too restrictive for a general-purpose tool

**Why rejected**: Too opinionated, excludes legitimate use cases.

### Alternative 2: Abstract Version Interface

**Approach**: Define a version abstraction that different strategies implement (semver, dates, etc.)

**Pros**:
- Flexible
- Could support multiple strategies
- Extensible

**Cons**:
- Complex abstraction layer
- Unnecessary when git already provides refs
- Over-engineering
- Harder to implement and understand

**Why rejected**: Git refs already provide the abstraction we need.

### Alternative 3: Semver Required, Custom Allowed

**Approach**: Default to semver, but allow "escape hatch" for custom versions

**Pros**:
- Encourages best practices (semver)
- Still allows flexibility

**Cons**:
- Creates two classes of dependencies (standard vs. custom)
- Unclear when to use which
- Still too opinionated
- Complexity without clear benefit

**Why rejected**: The "default" still imposes opinions.

## Consequences

### Positive

✅ **Maximum flexibility**: Works with any git-based workflow
✅ **Git-native**: Leverages existing primitives, no new concepts
✅ **No opinions**: Doesn't force versioning strategies
✅ **Commit-granular**: Can track changes at commit level if desired
✅ **Branch support**: Pre-release testing on branches works naturally
✅ **Simple implementation**: Use git commands directly

### Negative

❌ **No automatic breaking change detection**: Can't infer from version number alone
❌ **Ordering ambiguity**: Must define order explicitly or use git log
❌ **Less standardization**: Different projects may use different conventions

### Mitigations

- **Explicit type field**: Changes can declare `type: breaking` explicitly
- **Optional semver helpers**: When refs match semver pattern, provide helpful features (breaking detection, --to-next-major, etc.)
- **Declared ordering**: Changes in graft.yaml define application order
- **Git log fallback**: Use chronological order when needed

### Implementation Notes

#### Change Model
```typescript
interface Change {
  ref: string           // Any git ref - required
  type?: string         // Optional explicit type
  migration?: string
  verify?: string
}
```

#### Lock File
```yaml
dependencies:
  meta-kb:
    ref: "v1.5.0"      # Could be any ref format
    commit: "abc123"    # Resolved hash for integrity
```

#### Query Operations
```bash
# Works with any ref format
graft changes meta-kb --from v1.5.0 --to v2.0.0
graft changes meta-kb --from abc123 --to def456
graft changes meta-kb --from release-2025-12 --to release-2026-01
```

#### Optional Semver Detection
```python
def is_semver(ref: str) -> bool:
    return re.match(r'^v?\d+\.\d+\.\d+', ref) is not None

def get_major_version(ref: str) -> Optional[int]:
    if not is_semver(ref):
        return None
    match = re.match(r'^v?(\d+)', ref)
    return int(match.group(1)) if match else None

# Enable semver features only when applicable
if is_semver(from_ref) and is_semver(to_ref):
    if get_major_version(to_ref) > get_major_version(from_ref):
        warn("Major version change detected - breaking changes likely")
```

## Examples

### Semver Tags (Optional)
```yaml
changes:
  v1.0.0:
    migration: migrate-v1
  v2.0.0:
    type: breaking
    migration: migrate-v2
```

### Date-Based Releases
```yaml
changes:
  release-2025-12:
    migration: migrate-dec
  release-2026-01:
    migration: migrate-jan
```

### Sequential Numbering
```yaml
changes:
  r41:
    migration: migrate-r41
  r42:
    type: breaking
    migration: migrate-r42
```

### Commit-Granular
```yaml
changes:
  abc123:
    type: feature
    migration: migrate-auth
  def456:
    type: fix
```

### Branch-Based
```yaml
changes:
  stable:
    migration: migrate-stable
  feature-auth:
    migration: migrate-auth-preview
```

## Related

- [Decision 0001: Initial Scope](./decision-0001-initial-scope.md)
- [Decision 0003: Explicit Change Declarations](./decision-0003-explicit-change-declarations.md)
- [Specification: Change Model](../graft/change-model.md)
- [Specification: graft.yaml Format](../graft/graft-yaml-format.md)

## References

- Git refs documentation: https://git-scm.com/book/en/v2/Git-Internals-Git-References
- Semantic Versioning: https://semver.org/
- Calendar Versioning: https://calver.org/
