---
title: "Decision 0007: Flat-Only Dependency Model"
date: 2026-01-31
status: accepted
---

# Decision 0007: Flat-Only Dependency Model

## Summary

**Decision**: Graft adopts flat-only dependency resolution. Only direct dependencies declared in `graft.yaml` are resolved—no automatic transitive resolution.

**Key insight**: Grafts are **influences** (patterns, templates, migrations that shape your repo), not **components** (runtime libraries you call). Dependencies' dependencies are implementation details, not runtime requirements.

**Benefits**:
- Uses git submodules as required cloning layer (better DX, native git integration)
- Significantly simpler lock file (removes `requires`/`required_by` tracking)
- Self-contained migrations (better design constraint)
- Explicit over implicit (declare what you use)

**Trade-offs**: Discovery friction (can't auto-see transitive deps), coordination overhead for deep hierarchies, no automatic cross-references.

**Impact**: Supersedes [Decision 0005](./decision-0005-no-partial-resolution.md), requires specification updates.

---

## Context

### Prior Work

The [2026-01-12 dependency management exploration](../../../notes/2026-01-12-dependency-management-exploration.md) evaluated git submodules as a cloning mechanism and rejected them due to four problems:

1. **Nested paths** - Transitive dependencies create paths like `.graft/meta-kb/.graft/standards-kb/`
2. **No deduplication** - Same dependency cloned multiple times
3. **No conflict detection** - Different versions install silently
4. **Windows symlink issues** - Flattening layer has compatibility problems

This led to designing a custom resolution mechanism with full transitive dependency support.

### The Question

A [subsequent exploration](../../../notes/2026-01-31-flat-only-dependency-analysis.md) asked: **What if Graft only resolved direct dependencies, not transitives?**

This "flat-only" model would mean:
- Each project declares only its direct dependencies
- `graft resolve` clones only what's in `graft.yaml`, nothing more
- Dependencies' dependencies are their own implementation details
- No transitive resolution, no dependency graph walking

### Key Insight: Grafts Are Influences, Not Components

Traditional package managers (npm, pip, Cargo) manage **runtime components**:
- Function A calls function B calls function C
- All must be present at runtime
- Transitive dependencies are essential

Graft manages **influences** - patterns, templates, knowledge bases:
- Migrations run once, output is committed to your repo
- You read documentation from direct dependencies
- Results of graft operations become part of your codebase
- Downstream consumers see YOUR committed content, not the graft chain

This fundamental difference means transitive dependencies serve a different purpose than in traditional package managers.

## Decision

**Graft WILL adopt a flat-only dependency model.**

- Only dependencies explicitly declared in `graft.yaml` will be resolved
- No automatic transitive resolution
- Each graft's migrations must be self-contained
- If you reference another graft's content, add it as a direct dependency

## Rationale

### 1. Dependencies' Dependencies Are Implementation Details

```yaml
# meta-kb's graft.yaml
deps:
  standards-kb: "..."  # How meta-kb structures its content internally
```

When you consume `meta-kb`:
- You get the committed content in `meta-kb/`
- How `meta-kb` was built is irrelevant to you
- If you want `standards-kb`, add it because YOU want it
- Not because `meta-kb` happened to use it

**Analogy**: You don't need to install a library's build tools to use the library.

### 2. Self-Contained Migrations Are the Right Constraint

Migrations in flat-only model:
```yaml
commands:
  init:
    run: |
      # Uses bundled content, not references to transitives
      cp bundled/config/.eslintrc .
      cp -r bundled/templates/.github .
```

This is **better design**:
- ✓ Clear about dependencies (bundle what you need)
- ✓ Works reliably (no phantom dependencies)
- ✓ Explicit over implicit (declare what you use)

Migrations that reference transitives are fragile:
```yaml
# BAD - breaks if consumer doesn't have transitive
run: cp ${DEP_ROOT}/../standards-kb/config.json .
```

### 3. Output-Level Conflicts Are Visible

Concern: "What if two grafts use different versions of a common dependency?"

**Answer**: Conflicts surface at the file level:
- Different files → no conflict, internal versions irrelevant
- Same file → visible merge conflict you can resolve
- You care about output quality, not internal lineage

Example:
```
web-app-template  (internally used coding-standards v2)
  → generates .eslintrc with rule X

cli-tool-template (internally used coding-standards v1)
  → generates .prettierrc with rule Y

No conflict - different files, both work fine in your repo
```

### 4. Git Submodules as the Required Cloning Layer

With flat-only, **all four original blockers are eliminated**:

| Problem | Status with Flat-Only |
|---------|----------------------|
| Nested paths | **Eliminated** - no transitive nesting |
| Deduplication | **Eliminated** - no shared transitives to deduplicate |
| Conflict detection | **Eliminated** - no transitive conflicts possible |
| Symlinks | **Eliminated** - no flattening needed |

This makes git submodules the **required** cloning layer:
```
.gitmodules         # Git's native submodule tracking (required)
graft.yaml          # Graft's semantic layer (changes, migrations)
graft.lock          # Consumed state tracking
.graft/             # Submodule checkouts
  meta-kb/
  shared-utils/
```

**Benefits**:
- `git clone --recursive` works natively
- No special CI/CD integration for cloning
- Familiar git commands for contributors
- IDE support for submodules

**Synchronization guarantee**: The commit hash in `graft.lock` MUST match the submodule's checked-out commit.

### 5. Ecosystem Conventions Are Reasonable

**Discovery pattern**:
- Grafts document their dependencies in README
- "This graft builds on standards-kb - add it if you want to reference our patterns"
- Mirrors conventions in Go, Python, Rust

**Explicit declaration**:
```yaml
# If you reference another graft's content, declare it
deps:
  meta-kb: "..."
  standards-kb: "..."  # Explicitly added because YOU use it
```

Better than implicit transitives that "just appear."

### 6. Lock File Simplification

**Current (with transitive tracking)**:
```yaml
dependencies:
  meta-kb:
    direct: true
    requires: ["standards-kb"]
    required_by: []
  standards-kb:
    direct: false
    requires: []
    required_by: ["meta-kb"]
```

**Flat-only (simplified)**:
```yaml
dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-05T10:30:00Z"
```

Removed: `direct`, `requires`, `required_by`, all transitive entries.

Just a simple list of "what you declared + when you consumed it."

## Alternatives Considered

### Alternative 1: Configurable Resolution Modes

Support both flat and full transitive resolution:
```yaml
apiVersion: graft/v0
resolution: flat  # or "full"
deps:
  meta-kb: "..."
```

**Evaluation**:
- Adds complexity - two resolution modes to maintain
- Confusion - which mode should I use?
- Test matrix - all features must work in both modes
- **Decision**: Reject - flat-only is sufficient for all identified use cases

### Alternative 2: Keep Full Transitive Resolution

Maintain the original design with full transitive dependency resolution.

**Evaluation**:
- More complex lock file (`requires`/`required_by` tracking)
- More complex resolution algorithm (graph walking, conflict detection)
- Submodules remain non-viable (nested structure issues)
- **Decision**: Reject - added complexity not justified by use cases

### Alternative 3: Read-Only Transitives

Clone transitives but restrict migrations to direct dependencies only:
```
.graft/
├── meta-kb/          # direct - migrations can run
├── standards-kb/     # transitive - read-only, no migrations
```

**Evaluation**:
- Enables cross-references without explicit declaration
- Maintains some complexity (transitive resolution, tracking)
- Unclear semantics - why clone if migrations can't use?
- **Decision**: Reject - doesn't simplify enough, violates explicitness

### Alternative 4: Content Aggregation Special Case

Support transitive resolution only for specific use cases (documentation portals):
```yaml
resolution: flat
aggregation: true  # Special mode for portals
```

**Evaluation**:
- Aggregation is a **build concern**, not a dependency concern
- Build tools (MkDocs, Hugo, etc.) should handle aggregation
- Graft's role: provide access to sources
- If build needs transitives, user adds them as direct deps
- **Decision**: Reject - separation of concerns is clearer

## Consequences

### Positive

**Simplicity**:
- One resolution mode, simpler mental model
- Smaller lock file, less tracking overhead
- No graph walking, conflict resolution, or deduplication logic

**Explicitness**:
- Clear what you depend on (it's in your `graft.yaml`)
- No phantom dependencies
- "If you use it, declare it"

**Git Submodules as Required Cloning Layer**:
- Uses native git for cloning
- `git clone --recursive` works
- Better IDE/tool integration
- Familiar workflows for contributors
- Lock file and submodule state stay synchronized

**Better Design Constraints**:
- Self-contained migrations are more robust
- Bundled content vs. fragile references
- Clear dependency boundaries

### Negative

**Discovery Friction**:
- Can't automatically see what dependencies use internally
- Must read README or inspect their `graft.yaml`
- *Mitigated by*: `graft inspect <dep> --deps` command (future)

**Coordination Overhead (Large Hierarchies)**:
- Deep graft hierarchies (4+ levels) require coordination for updates
- Security patches propagate through multiple releases
- *Mitigated by*:
  - Flatter hierarchies recommended
  - Skip-level direct deps where needed
  - Automated update notifications (Decision 0006)

**No Cross-References Without Declaration**:
- Can't link to `../.graft/transitive-dep/file.md` unless explicitly added
- Must use external URLs or add as direct dependency
- *Mitigated by*: External URLs work in most cases, explicit deps for critical references

### Neutral

**Not a Limitation in Practice**:
- Real usage (graft-knowledge) shows transitives rarely needed
- "Echo model" (patterns vs. wholesale copying) fits naturally
- Most grafts are self-contained or use external references

**Template "Init Once" Pattern**:
- Starter templates work naturally (bundle what they need)
- Generated code becomes yours
- No ongoing transitive relationship needed

## Implementation

### Lock File Changes

Remove fields:
- `direct: boolean` - all deps are direct
- `requires: string[]` - no transitive tracking
- `required_by: string[]` - no reverse tracking

Keep fields:
- `source` - where it came from
- `ref` - version consumed
- `commit` - integrity hash
- `consumed_at` - timestamp

### Migration Constraints

Migrations MUST be self-contained:
- ✓ Use `${DEP_ROOT}/bundled-content/`
- ✗ Use `${DEP_ROOT}/../transitive-dep/`

Grafts that need transitive content should:
1. Bundle it during publishing
2. Document the direct dependency requirement
3. Use external URLs for references

### Validation

`graft validate` checks:
- All deps in `graft.yaml` are in `.graft/`
- No extra deps in `.graft/` not in `graft.yaml`
- Lock file matches `graft.yaml` declarations
- Commit hashes match checked-out versions

### Commands

New commands to support the model:
- `graft inspect <dep> --deps` - Show what a dependency depends on (curiosity/debugging)
- `graft sync` - Sync submodule checkouts to lock file state (after teammate upgraded)
- `graft add <dep> <source>#<ref>` - Add submodule + update yaml/lock
- `graft remove <dep>` - Remove submodule + config entries

## Related

- **Supersedes**: [Decision 0005: No Partial Dependency Resolution](./decision-0005-no-partial-resolution.md) - Transitive deps no longer exist, making partial resolution moot
- **Builds on**: [Decision 0003: Explicit Change Declarations](./decision-0003-explicit-change-declarations.md) - Explicit over implicit
- **Builds on**: [Decision 0004: Atomic Upgrades](./decision-0004-atomic-upgrades.md) - All-or-nothing operations
- **Detailed analysis**: [Flat-Only Dependency Analysis (2026-01-31)](../../../notes/2026-01-31-flat-only-dependency-analysis.md)

## Specifications to Update

This decision requires updates to:
- `specifications/graft/dependency-layout.md` - Remove transitive resolution algorithm
- `specifications/graft/lock-file-format.md` - Simplify structure (remove `requires`/`required_by`)
- `specifications/graft/graft-yaml-format.md` - Document migration self-containment constraint
- `specifications/graft/core-operations.md` - Update `resolve` semantics

## References

- **Go Modules (MVS)**: Example of transitive resolution complexity - https://research.swtch.com/vgo-mvs
- **git-subrepo**: Tool showing alternative to submodules - https://github.com/ingydotnet/git-subrepo
- **Google repo**: Manifest-driven multi-repo management - https://source.android.com/docs/setup/reference/repo
- **2026-01-12 exploration**: Original analysis rejecting submodules - [notes/2026-01-12-dependency-management-exploration.md](../../../notes/2026-01-12-dependency-management-exploration.md)
