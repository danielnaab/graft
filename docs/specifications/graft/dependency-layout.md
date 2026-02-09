---
title: "Dependency Layout Specification"
date: 2026-01-31
status: draft
version: v3
supersedes: v2
---

# Dependency Layout Specification

## Overview

This specification defines how Graft organizes dependencies on disk using a **flat-only dependency model**.

**Design Goals:**
1. Enable ergonomic markdown linking between repositories
2. Provide predictable, short paths for human use
3. Avoid path length issues and naming conflicts
4. Leverage git submodules for native git integration
5. Enable reproducible builds via lock files

**Key Change from v2:** Graft no longer resolves transitive dependencies. Each project declares and manages only its direct dependencies. This aligns with Graft's purpose: dependencies are **influences** that shape your repository, not **components** that couple at runtime.

---

## Flat-Only Dependency Model

### Core Principle

**Each project declares only its direct dependencies.** Transitive dependencies (dependencies of dependencies) are not automatically resolved or cloned.

If you reference content from a dependency, you must declare it as YOUR direct dependency.

### Why Flat-Only Works

Graft dependencies are fundamentally different from traditional package dependencies:

| Aspect | npm/pip package | Graft |
|--------|-----------------|-------|
| Consumption | Import and call at runtime | Reference and apply patterns |
| Output | Library code you depend on | Files committed to your repo |
| Coupling | Tight (API contracts) | Loose (output files) |
| Updates | Must maintain compatibility | Can diverge, you own result |

**Grafts are influences, not components:**
- They produce **output** (files in your repository)
- That output is **committed to your repo**
- Downstream sees YOUR content, not the original dependency chain
- Dependencies of your dependencies are their implementation details

### Implications

**For consumers:**
- You only see and manage dependencies YOU declared
- No hidden transitive dependencies to track
- Simpler mental model: "my deps, my responsibility"
- If you need content from a transitive, add it as a direct dependency

**For graft authors:**
- Migrations must be self-contained (bundle what you need)
- Cannot assume consumers have your dependencies
- Document which grafts complement yours (ecosystem guidance)

---

## Consumption Patterns Analysis

### How Dependencies Are Consumed

**1. Direct Reference** - Markdown links to dependency content
```markdown
[Architecture Patterns](../.graft/meta-kb/docs/architecture.md)
![Diagram](../.graft/meta-kb/assets/flow.svg)
```

**2. Asset Usage** - Images, diagrams, files referenced in builds
```bash
# Copy shared assets
cp .graft/brand-kb/logos/*.svg public/
```

**3. Search and Indexing** - Tools scanning dependency content
```bash
# Index all knowledge
grep -r "concept" . .graft/*/docs/
```

**4. Script Execution** - Running migration/utility commands
```bash
# Run migration from dependency
graft upgrade meta-kb --to v2.0.0
```

**5. Pattern Application** - Using dependencies as templates/guides
- Read documentation
- Apply patterns to your code
- Generate files using dependency's commands

### Critical Requirements

From these patterns, we derive:

- **Short paths**: Human-friendly linking `../.graft/dep-name/`
- **Predictable locations**: Always know where a dep lives
- **No magic**: Transparent, inspectable structure
- **Git-native**: Works with standard git tools
- **Self-contained migrations**: Each graft bundles what it needs

---

## Directory Structure

### Flat-Only Layout

```
project/
├── .gitmodules         # Git's native submodule tracking
├── graft.yaml          # Graft's semantic configuration
├── graft.lock          # Consumed state (migrations, versions)
└── .graft/             # Direct dependencies only
    ├── meta-kb/        # Direct dependency
    └── coding-standards/  # Direct dependency
```

**Key characteristics:**
- Only **direct dependencies** are cloned to `.graft/`
- Each dependency is a git submodule
- Paths are short and predictable: `.graft/<name>/`
- No transitive dependencies in `.graft/`

### Two-Layer Architecture

Graft uses git submodules as the cloning layer:

| Layer | File | Responsibility |
|-------|------|----------------|
| **Physical** | `.gitmodules` | Git's tracking of where repos are, what commit (required) |
| **Semantic** | `graft.yaml` + `graft.lock` | Changes, migrations, consumed state |

**Physical layer (Git):**
- Handles cloning, checkout, commit pinning
- Enables `git clone --recursive` workflow
- Familiar to git users

**Semantic layer (Graft):**
- Tracks consumption state (when migrations ran)
- Manages change model and migrations
- Provides upgrade/verification operations

---

## Git Submodules as the Cloning Layer

### Why Submodules Are Used

Previous exploration (2026-01-12) rejected submodules due to:
1. **Nested paths** - Transitives create deep nesting → **Eliminated** (no transitives)
2. **No deduplication** - Same dep cloned multiple times → **Not needed** (no shared transitives)
3. **No conflict detection** - Different versions coexist → **Not applicable** (no transitive conflicts)

The flat-only model removes all previous blockers.

### Benefits

**1. Native git workflow:**
```bash
git clone --recursive https://github.com/myorg/myproject.git
# Dependencies are already cloned!
```

**2. Familiar commands:**
```bash
git submodule update --init     # Clone deps
git submodule update --remote   # Update deps
```

**3. CI/CD compatibility:**
```yaml
# GitHub Actions - no special Graft setup for cloning
- uses: actions/checkout@v4
  with:
    submodules: recursive
```

**4. IDE integration:**
- VS Code, IntelliJ, etc. understand submodules
- Navigation, search work out of the box

### What Graft Adds

Git submodules alone provide:
- Cloning and checkout management
- Commit pinning

Graft adds:
- Change tracking (`graft status` shows pending changes)
- Migration execution (`graft upgrade` runs migrations)
- Verification (`verify` commands after migration)
- Atomic rollback (on migration failure)
- Human-readable refs (submodules only store commit hash)

### Synchronization Guarantee

**The commit hash in `graft.lock` MUST match the submodule commit.**

When `graft.lock` is updated, the corresponding submodule's checked-out commit must match the `commit` field in the lock file. This ensures:
- Lock file and submodule state are always in sync
- `graft validate integrity` can verify both lock file AND submodule state
- Reproducible builds across machines

### Example Workflow

```bash
# Add dependency (creates submodule + updates graft files)
graft add meta-kb git@github.com:org/meta-kb.git#v2.0.0

# Check status
graft status
# Shows: meta-kb v2.0.0, no updates

# Upgrade (runs migrations, updates submodule)
graft upgrade meta-kb --to v3.0.0

# Commit changes
git add .gitmodules .graft/ graft.yaml graft.lock
git commit -m "Upgrade meta-kb to v3.0.0"
```

---

## Migration Self-Containment

### The Constraint

**Migrations MUST be self-contained.** They cannot reference files in transitive dependencies.

**Invalid migration:**
```yaml
commands:
  migrate-v2:
    # BAD - references transitive dependency
    run: cp ${DEP_ROOT}/../standards-kb/template.md ./
```

**Valid migration:**
```yaml
commands:
  migrate-v2:
    # GOOD - uses bundled content
    run: cp ${DEP_ROOT}/bundled/template.md ./
```

### Bundling Strategy

If your graft depends on content from other grafts, **bundle it**:

```
my-graft/
  bundled/
    standards-kb/       # Copied from standards-kb at publish time
      template.md
      config.yaml
  commands/
  graft.yaml
```

### Documenting Complementary Grafts

Use README to document which grafts work well together:

```markdown
# My Graft

This graft provides web app scaffolding.

## Recommended Complementary Grafts

- **coding-standards** - Provides linting and style configs
- **security-policies** - Provides security checklists

Add these as direct dependencies:
​```yaml
deps:
  web-app-template: "..."
  coding-standards: "..."
  security-policies: "..."
​```
```

---

## Cross-References

### External URLs (Recommended)

For documentation links that humans read:

```markdown
<!-- Instead of relative path -->
[Pattern](../.graft/standards-kb/patterns.md)

<!-- Use external URL -->
[Pattern](https://github.com/org/standards-kb/blob/v1.5.0/patterns.md)
```

**Benefits:**
- Always works, regardless of what dependencies consumer has
- Works in published docs (GitHub, GitBook)
- No coupling to consumer's dependency choices

### Explicit Dependencies

If you need content from another graft **for your migrations**:

1. Bundle that content in your graft
2. Document that consumers should add both grafts
3. Your migration uses YOUR bundled content, not references to other grafts

---

## Benefits

### For Users

**Ergonomic linking:**
```markdown
<!-- Short, predictable paths -->
[Pattern](../.graft/meta-kb/docs/pattern.md)
[Diagram](../.graft/meta-kb/assets/flow.svg)
```

**Simple mental model:**
```bash
# Only YOUR dependencies
ls .graft/
# meta-kb  coding-standards

# Lock file shows what you consumed
cat graft.lock
# Shows: meta-kb v2.0.0, coding-standards v1.5.0
```

**No hidden complexity:**
- Only dependencies YOU declared are present
- No transitive dependency resolution
- No version conflict between transitives (doesn't apply)
- Clear: "If I need it, I declare it"

**Git-native workflow:**
```bash
# Clone with dependencies
git clone --recursive myproject

# Dependencies are already there
cd myproject && ls .graft/
```

### For Graft Authors

**Self-contained by design:**
- Bundle what your migrations need
- No assumptions about consumer's dependencies
- Clear contract: "Here's what I provide"

**Simple upgrade path:**
- Migrations run in isolation
- No coordination with transitive dependency migrations
- You control your content and commands

### For Tool Builders

**Validation:**
```bash
# Check deps match lock file
graft validate integrity

# Validate configuration
graft validate config
```

**Inspection:**
```bash
# Show dependency metadata
graft inspect meta-kb

# Show a dependency's own dependencies
graft inspect meta-kb --deps
```

---

## Package Manager Comparison

### How Graft Differs

| Aspect | Go Modules | npm | Cargo | Graft (Flat) |
|--------|------------|-----|-------|--------------|
| Transitives | Yes (MVS) | Yes (hoist) | Yes (semver) | **No** |
| Lock file | go.sum | package-lock | Cargo.lock | graft.lock |
| Version scheme | semver | semver | semver | **git refs** |
| What's managed | Libraries | Packages | Crates | **Influences** |
| Conflict handling | MVS picks | Nesting | Semver | **N/A** |

**Key distinction:** Traditional package managers manage **components** that execute at runtime (A calls B calls C, so C must be present). Graft manages **influences** that shape your repo through patterns and migrations.

### Partial Precedents

- **Early Go (GOPATH)** - No transitive resolution, each package managed its own deps
- **Vendoring** - Commit deps to repo, downstream gets committed results
- **Git submodules (non-recursive)** - Shallow, direct-only cloning

**What's unique about Graft:**
- Combines flat-only with semantic lock file + migrations
- "Influences" vs "components" - you absorb patterns, not link libraries
- Grafted content is committed, not linked at runtime

---

## Open Questions

### Workspace Support

Support monorepos with multiple projects sharing deps?

```
workspace/
  project-a/
    graft.yaml
  project-b/
    graft.yaml
  .graft/  # Shared deps
```

**Decision:** Future enhancement, not part of v3 spec

### Version Ranges

Should we support version ranges in graft.yaml?

```yaml
deps:
  meta-kb: "https://github.com/org/meta.git#^v2.0.0"
```

**Decision:** Start without, add if needed based on ecosystem usage

---

## References

- **Related specifications:**
  - [graft.yaml Format](./graft-yaml-format.md)
  - [Lock File Format](./lock-file-format.md)
  - [Core Operations](./core-operations.md)

- **Related decisions:**
  - [Decision 0007: Flat-Only Dependency Model](../decisions/decision-0007-flat-only-dependencies.md)
  - [Analysis Note: Flat-Only Exploration](../../../notes/2026-01-31-flat-only-dependency-analysis.md)

---

## Appendix: Example Scenarios

### Scenario 1: Simple Project (Flat-Only)

**Setup:**
```yaml
# graft.yaml
deps:
  meta-kb: "https://github.com/org/meta.git#v2.0.0"
```

**After `graft resolve`:**
```
project/
├── .gitmodules       # Submodule tracking
├── graft.yaml
├── graft.lock
└── .graft/
    └── meta-kb/      # Only direct dependency (submodule)
```

**graft.lock contents:**
```yaml
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "https://github.com/org/meta.git"
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-31T10:30:00Z"
```

**Linking:**
```markdown
[Concept](../.graft/meta-kb/docs/concept.md)
```

### Scenario 2: Multiple Direct Dependencies

**Setup:**
```yaml
# graft.yaml
deps:
  meta-kb: "https://github.com/org/meta.git#v2.0.0"
  coding-standards: "https://github.com/org/standards.git#v1.5.0"
```

**After `graft resolve`:**
```
project/
├── .gitmodules       # Submodule tracking
├── graft.yaml
├── graft.lock
└── .graft/
    ├── meta-kb/           # Submodule
    └── coding-standards/  # Submodule
```

**graft.lock shows both:**
```yaml
apiVersion: graft/v0
dependencies:
  coding-standards:
    source: "https://github.com/org/standards.git"
    ref: "v1.5.0"
    commit: "def456..."
    consumed_at: "2026-01-31T10:30:00Z"

  meta-kb:
    source: "https://github.com/org/meta.git"
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-31T10:30:00Z"
```

**Note:** Alphabetical ordering for consistency.

### Scenario 3: Complementary Grafts

`meta-kb` uses content from `standards-kb` internally. How does this work?

**Option 1: meta-kb bundles what it needs**
```
meta-kb/
  bundled/
    standards-content/
      patterns.md
  commands/
    migrate: uses bundled/standards-content/
```

**Option 2: meta-kb documents the recommendation**
```markdown
# meta-kb README

## Recommended Setup

This graft works best with:
- **standards-kb** - Provides coding patterns
- **templates-kb** - Provides file templates

Add both:
​```yaml
deps:
  meta-kb: "..."
  standards-kb: "..."
  templates-kb: "..."
​```
```

**Consumer sees:**
```
.graft/
├── meta-kb/          # Direct dependency
├── standards-kb/     # Direct dependency (you added it)
└── templates-kb/     # Direct dependency (you added it)
```

---

## Changelog

- **2026-01-31 (v3.0)**: Flat-only dependency model
  - Removed transitive dependency resolution
  - Simplified lock file (removed `direct`, `requires`, `required_by` fields)
  - Made git submodules the required cloning layer
  - Added synchronization guarantee (lock file ↔ submodule state)
  - Added migration self-containment requirements
  - Updated examples to reflect flat-only model
  - Supersedes v2

- **2026-01-05 (v2.1)**: Extended lock file
  - Extended graft.lock to include all resolved dependencies
  - Added fields: `direct`, `requires`, `required_by`

- **2026-01-05 (v2.0)**: Initial draft
  - Proposed flat layout with transitive resolution
