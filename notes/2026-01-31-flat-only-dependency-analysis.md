---
title: "Flat-Only Dependency Model Analysis"
date: 2026-01-31
status: completed
related_decision: decision-0007-flat-only-dependencies.md
---

# Flat-Only Dependency Model Analysis

## Summary

This document captures a comprehensive exploration of Graft's dependency model, concluding that **flat-only dependency resolution** (no transitive resolution) is both viable and cleaner than full transitive resolution.

**Key insight**: Grafts are **influences** (patterns, templates, migrations that shape your repo) not **components** (runtime libraries you call). This fundamental difference means dependencies' dependencies are implementation details, not runtime requirements.

**Outcome**: [Decision 0007](../docs/specifications/decisions/decision-0007-flat-only-dependencies.md) adopted flat-only model, enabling git submodules as cloning layer and significantly simplifying lock file format.

---

## Methodology

This analysis was conducted through systematic exploration of multiple dimensions:

**Foundation**:
- Review of [2026-01-12 dependency management exploration](./2026-01-12-dependency-management-exploration.md) which originally rejected git submodules due to transitive dependency issues
- Analysis of real-world usage patterns in graft-knowledge repository
- Examination of the "influences vs components" paradigm shift

**Research Areas**:
- **Viability analysis**: Testing whether flat-only works for Graft's actual use cases (knowledge bases, templates, config injection)
- **Architecture exploration**: Git submodules integration, lock file simplification, DX workflows
- **Tool comparison**: Survey of existing multi-repo tools (git-subrepo, Google repo, vcstool, Git X-Modules)
- **Edge case analysis**: Content aggregation, large hierarchies, cross-references, security model
- **Ecosystem comparison**: How Graft's model differs from npm, pip, Go, Cargo

**Process**:
- Iterative questioning and refinement through multiple rounds
- Challenge-driven exploration (e.g., "when would you actually copy deps?")
- Evidence-based reasoning from actual codebase patterns
- Comparison with established tools and practices

**Documentation**:
- Full exploration preserved in plan file: `.claude/plans/abundant-juggling-parnas.md`
- Concise decision captured in [Decision 0007](../docs/specifications/decisions/decision-0007-flat-only-dependencies.md)
- This note provides comprehensive details for future reference

---

## The Question

### Background

The [2026-01-12 exploration](./2026-01-12-dependency-management-exploration.md) evaluated git submodules and rejected them due to four problems arising from **transitive dependencies**:

1. Nested paths (`.graft/meta-kb/.graft/standards-kb/`)
2. No deduplication
3. No conflict detection
4. Windows symlink issues with flattening

This exploration asked: **What if we eliminated transitive resolution entirely?**

### Scope

We explored:
- Whether flat-only is viable for Graft's use cases
- How git submodules fit with flat-only model
- Lock file simplification opportunities
- Day-to-day developer experience
- Edge cases and limitations
- Comparison with existing tools
- Security implications

---

## Key Findings

### 1. Why Flat-Only Works

#### Grafts Are Influences, Not Components

The critical distinction:

| Aspect | npm/pip Package | Graft |
|--------|----------------|-------|
| **Consumption** | Import and call at runtime | Reference and apply patterns |
| **Output** | Library code you depend on | Files committed to your repo |
| **Coupling** | Tight (API contracts) | Loose (output files) |
| **Updates** | Must maintain compatibility | Can diverge, you own result |
| **Transitives** | Essential (A calls B calls C) | Implementation details |

**Example**:
- npm: You call `lodash.map()` which calls internal functions - all must be present
- Graft: `meta-kb` migration runs once, generates files, commits them - you own the output

#### Dependencies' Dependencies Are Their Own Business

```yaml
# meta-kb's graft.yaml
deps:
  standards-kb: "..."  # How meta-kb structures its content
```

When YOU consume `meta-kb`:
- You get committed content in `meta-kb/`
- How it was built is irrelevant to you
- If YOU want `standards-kb`, add it because YOU want it
- Not because `meta-kb` happened to use it internally

This is like: you don't need a library's build tools to use the library.

#### Self-Contained Migrations Are Better Design

**Good (self-contained)**:
```yaml
commands:
  init:
    run: |
      # Bundle what you need
      cp bundled/eslint-config/.eslintrc .
      cp -r bundled/ci-templates/.github .
```

**Bad (fragile transitive reference)**:
```yaml
commands:
  init:
    run: |
      # Breaks if consumer doesn't have standards-kb
      cp ${DEP_ROOT}/../standards-kb/configs/.eslintrc .
```

Benefits of self-contained:
- ✓ Clear dependencies (explicit bundling)
- ✓ Works reliably (no phantom dependencies)
- ✓ Testable in isolation
- ✓ Explicit over implicit

#### Output-Level Conflicts Are Visible

**Concern**: "What if two grafts use different versions of a shared dependency?"

**Example**:
```
my-project
├── web-app-template   (internally used coding-standards v2)
│   → generates .eslintrc
└── cli-tool-template  (internally used coding-standards v1)
    → generates .prettierrc
```

**Analysis**:
- Different files → no conflict, internal versions irrelevant
- Same file → visible merge conflict you can resolve
- You care about output quality, not internal lineage

Hidden version mismatches don't matter because:
1. Each graft produces committed output
2. Conflicts happen at file level (visible)
3. Internal implementation details stay internal

---

### 2. Git Submodules as Cloning Layer

#### Why Submodules Work Now

The 2026-01-12 blockers are **all eliminated** by flat-only:

| Problem | Flat-Only Status |
|---------|-----------------|
| Nested paths | **Eliminated** - no transitive nesting |
| Deduplication | **Not needed** - no shared transitives |
| Conflict detection | **Not needed** - no transitive conflicts |
| Symlinks | **Not needed** - no flattening layer |

#### Architecture

```
project/
├── .gitmodules         # Git's native submodule tracking
├── graft.yaml          # Graft's semantic config
├── graft.lock          # Consumed state
└── .graft/             # Submodule checkouts
    ├── meta-kb/
    └── shared-utils/
```

**Two-layer separation**:
- **Physical (Git)**: `.gitmodules` tracks where repos are, what commit
- **Semantic (Graft)**: `graft.yaml` + `graft.lock` track changes, migrations, consumed state

#### What Submodules Provide

| Feature | Benefit |
|---------|---------|
| Native git integration | `git clone --recursive` just works |
| IDE support | VS Code, IntelliJ recognize submodules |
| Familiar commands | Standard git workflow |
| No special CI setup | Works with `actions/checkout@v4` |

#### What Graft Adds

| Git Submodules Alone | Graft + Submodules |
|---------------------|-------------------|
| Manual checkout management | `graft add` with simple syntax |
| No change tracking | `graft status` shows pending changes |
| No migration support | `graft upgrade` runs migrations |
| No rollback | Automatic rollback on failure |
| Commit hash only | Human-readable refs preserved |
| Detached HEAD confusion | Clear status, ref tracking |

#### DX Benefits

**Native workflow**:
```bash
# Clone with deps - no graft needed
git clone --recursive https://github.com/myorg/myproject.git

# CI/CD - standard checkout works
- uses: actions/checkout@v4
  with:
    submodules: recursive
```

**Graft enhancement**:
```bash
# Simple add syntax
graft add meta-kb git@github.com:org/meta-kb.git#v2.0.0

# Intelligent upgrade
graft upgrade meta-kb --to v3.0.0
# → Shows changes, runs migrations, atomic rollback on failure

# After teammate upgraded
git pull
graft sync  # Sync submodules to lock file state
```

---

### 3. Simplified Lock File

#### Before (Transitive Tracking)

```yaml
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-05T10:30:00Z"
    direct: true
    requires: ["standards-kb"]
    required_by: []
  standards-kb:
    source: "git@github.com:org/standards-kb.git"
    ref: "v1.5.0"
    commit: "def456..."
    consumed_at: "2026-01-05T10:30:00Z"
    direct: false
    requires: []
    required_by: ["meta-kb"]
```

#### After (Flat-Only)

```yaml
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "git@github.com:org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123..."
    consumed_at: "2026-01-05T10:30:00Z"
```

#### What's Removed

- `direct` field - unnecessary (everything is direct)
- `requires` field - no transitive tracking
- `required_by` field - no reverse dependency tracking
- All transitive dependency entries

#### What Remains

- `source` - where it came from
- `ref` - version consumed (human-readable)
- `commit` - integrity verification hash
- `consumed_at` - timestamp

**Result**: Just a simple list of "what you declared + when you consumed it."

---

### 4. Day-to-Day Workflows

#### Core Commands

| Task | Command | Notes |
|------|---------|-------|
| **Add dependency** | `graft add meta-kb <url>#v2.0.0` | Adds submodule + updates yaml/lock |
| **Check status** | `graft status` | Shows deps, updates, pending migrations |
| **See updates** | `graft outdated` | Quick view of available updates |
| **View changes** | `graft changes meta-kb` | What changed between versions |
| **Upgrade one** | `graft upgrade meta-kb` | Migrations, atomic rollback |
| **Upgrade all** | `graft update` | All deps with updates |
| **Remove** | `graft remove meta-kb` | Remove submodule + config |
| **After pull** | `graft sync` | Sync to teammate's upgrades |
| **Validate** | `graft validate` | Check integrity |

#### Local Development

**Editing a dependency**:
```bash
# Just edit files directly
vim .graft/meta-kb/docs/some-file.md
# Changes visible immediately, no rebuild needed
```

**Creating a branch for changes**:
```bash
cd .graft/meta-kb
git checkout -b fix/my-bug-fix
# Make changes, commit
git commit -am "Fix the bug"
```

**Contributing upstream**:
```bash
cd .graft/meta-kb
git push -u origin fix/my-bug-fix
gh pr create
```

**Switching between versions**:
```bash
# Use your dev branch
graft upgrade meta-kb --to fix/my-bug-fix

# Go back to released
graft upgrade meta-kb --to v2.0.0
```

**Working with forks**:
```yaml
# Edit graft.yaml to point to your fork
deps:
  meta-kb:
    source: "git@github.com:my-org/meta-kb.git"
    ref: "my-feature"
```
```bash
graft sync  # Update to fork
```

#### Teammate Upgraded a Dep

```bash
git pull                    # Pulls lock file + submodule pointer
graft status                # Shows "SYNC REQUIRED"
graft sync                  # Syncs submodule checkout
# No migrations needed - teammate already ran them
```

Lock file tracks consumed state. Teammate committed:
- Updated lock file (v2.0.0 consumed)
- Migration artifacts (modified files)
- Submodule pointer update

You just sync the checkout to match.

#### Upgrade with Migrations

```bash
graft upgrade meta-kb --to v2.0.0

# Output:
#   Changes:
#     v1.5.0 (feature): Add new templates
#     v2.0.0 (BREAKING): Rename commands
#
#   Migrations:
#     migrate-v2: Update command references
#
#   Continue? [Y/n]

# On success:
#   ✓ Migration complete
#   ✓ Verification passed
#   ✓ Lock file updated
#
#   Files changed:
#     .graft/meta-kb (submodule)
#     graft.lock
#     src/example.md (migration modified)
#
#   Commit these changes to complete upgrade
```

#### CI/CD Integration

**Basic checkout (GitHub Actions)**:
```yaml
- uses: actions/checkout@v4
  with:
    submodules: recursive  # Gets all deps automatically
```

That's it. No special Graft setup needed for cloning.

**Caching**:
```yaml
- uses: actions/cache@v4
  with:
    path: .graft
    key: graft-deps-${{ hashFiles('graft.lock') }}
```

**Automated updates (Dependabot-style)**:
```yaml
on:
  schedule:
    - cron: '0 6 * * 1'  # Weekly

steps:
  - run: graft status --check-updates --json
  - run: |
      # For each update, create PR
      graft upgrade $DEP --to $NEW
      gh pr create --title "Update $DEP to $NEW"
```

**PR validation**:
```yaml
on:
  pull_request:
    paths: ['graft.lock', '.graft/**']

steps:
  - run: graft validate integrity
  - run: make test
```

---

### 5. Edge Cases and Limitations

#### Edge Case Summary

| Edge Case | Severity | Workaround | Blocks Flat? |
|-----------|----------|------------|--------------|
| Monorepo shared deps | Low | Explicit deps | No |
| Large org hierarchies | Med-High | Flatter hierarchies | Partially |
| Security patches | Medium | Cascade updates | No |
| **Content aggregation** | **Resolved** | Build concern | No |
| Cross-validation | Low | Local validation | No |

#### Content Aggregation (Re-examined)

**Original concern**: Documentation portals need transitive content for aggregation.

**Key insight**: Aggregation is a **BUILD concern**, not a DEPENDENCY concern.

**Evidence**:
- Decision 0001 excludes build system capabilities
- No documented use case describes aggregation as core feature
- Consumption patterns are about ACCESS, not COMBINATION

**Separation of concerns**:
```
GRAFT                           BUILD TOOL (MkDocs/Hugo)
────────────────────────────────────────────────────────
Fetch sources        →          Process sources
Resolve versions     →          Transform content
Run migrations       →          Generate output
Track state          →          Publish artifacts
```

**For portal scenarios**:
- Build tool configures source paths from `.graft/`
- If build needs transitives, add them as direct deps
- Graft provides access; build tool does combination
- OR: Portal links to external hosted docs (don't embed)

#### Large Organization Hierarchies

**Scenario**: Update propagation through 4+ levels

```
foundation-kb (v1 → v2)
  ↓
standards-kb (must update to use foundation v2)
  ↓
meta-kb (must update to use standards v2)
  ↓
your-project (must update to use meta v2)
```

**Impact**: Coordination overhead, delayed security patches

**Mitigations**:
- Flatter hierarchies recommended (2-3 levels max)
- Skip-level direct deps where critical
- Automated update notifications (Decision 0006)
- Grafts document their deps prominently

**Assessment**: Manageable with conventions, not a blocker

#### Cross-Graft References

**The finding**: Cross-reference pattern is theoretical, not practiced.

**Evidence from graft-knowledge**:
```yaml
# knowledge-base.yaml
meta:
  entrypoint: "../meta-knowledge-base/docs/meta.md"  # Sibling path!
imports:
  - kind: local
    path: ../meta-knowledge-base  # NOT .graft/meta-knowledge-base
```

Real usage shows external paths, not `.graft/` references.

**Alternatives to cross-references**:

1. **External URLs**:
   ```markdown
   [Pattern](https://github.com/org/standards-kb/blob/v1.5.0/patterns.md)
   ```
   - ✓ Always works, no resolution needed
   - ✓ Works in published docs
   - ✗ Verbose, URL rot risk

2. **Add as direct dep**:
   ```yaml
   deps:
     meta-kb: "..."
     standards-kb: "..."  # If you reference it, declare it
   ```
   - ✓ Explicit
   - ✓ Content available
   - ✓ Works with flat model

3. **Bundle at publish**:
   - Graft bundles referenced content when publishing
   - Downstream gets self-contained artifact
   - Links resolve without transitive resolution

---

### 6. Existing Tools Survey

#### Category 1: Submodule Alternatives

| Tool | Model | Maturity | Notes |
|------|-------|----------|-------|
| [Git Subtree](https://www.atlassian.com/git/tutorials/git-subtree) | Merge into main repo | Built-in | Seamless clone, harder push-back |
| [git-subrepo](https://github.com/ingydotnet/git-subrepo) | Embed + metadata | Mature (2013+) | `.gitrepo` per subrepo, bidirectional |
| [Git X-Modules](https://gitmodules.com/) | Server-side sync | Commercial | Zero client overhead, $39-690/mo |

**git-subrepo** learnings:
- Single metadata file per dependency works well
- Zero contributor impact possible (they just see files)
- Bidirectional push valuable for "edit dep and upstream" workflow
- Nested support adds complexity

**Assessment**: Embedding model (subrepo/subtree) bloats history. Submodules better for Graft's "present to read" use case.

#### Category 2: Multi-Repo Orchestration

| Tool | Purpose | Maturity |
|------|---------|----------|
| [Google repo](https://source.android.com/docs/setup/reference/repo) | Manifest-driven (Android) | Very mature |
| [vcstool](https://github.com/dirk-thomas/vcstool) | YAML workspace (ROS) | Mature |
| [myrepos](https://myrepos.branchable.com/) | Command parallelization | Mature |

**Google repo** learnings:
- XML manifest defines structure (repos, paths, branches)
- `repo init` + `repo sync` = ~200 repos managed
- Heavy tooling justified for large scale
- Manifest versioning enables reproducibility

**vcstool** learnings:
- Simple YAML format (path → url + version)
- No state file beyond filesystem
- Lightweight, works with existing checkouts

**Assessment**: These solve **orchestration** (running commands across repos), not **dependency management** (semantic changes, migrations, lock files).

#### Category 3: Direct Submodule Wrappers

| Tool | Approach | Status |
|------|----------|--------|
| [gitslave](https://gitslave.sourceforge.net/) | Wrapper for "slave" repos | Abandoned |
| [git-submodules.py](https://github.com/nsensfel/git-submodules) | Simpler config | Small project |

**Assessment**: Closest to what Graft needs, but none provide semantic layer (changes, migrations, consumption tracking).

#### Could Any Replace Graft's Cloning Layer?

**No.** None are suitable because:
1. They solve different problems (orchestration vs dependency management)
2. They lack Graft's semantic layer
3. They don't have lock file semantics

**Recommendation**: Build thin wrappers around git submodules directly.

---

### 7. Security and Trust Model

#### The Core Problem

**Migrations are arbitrary shell commands** with full user permissions.

A malicious migration could:
- Exfiltrate secrets (`.env`, SSH keys, AWS credentials)
- Install backdoors
- Access network
- Modify files outside project

#### Trust Chain

When you add a graft, you trust:
1. The graft maintainer(s)
2. Everyone with commit access
3. The git hosting platform
4. **Transitives** (even in flat-only, via bundled content)

**Note**: Flat-only doesn't reduce trust surface - you still implicitly trust transitives through bundled content.

#### Protections

**Lock file + commit hash**:
```yaml
dependencies:
  meta-kb:
    ref: "v2.0.0"
    commit: "abc123..."  # Tamper detection
```

If upstream compromised AFTER locking, `graft validate integrity` catches mismatch.

**But**: Doesn't detect already-compromised version at lock time.

#### Recommendations

**For consumers**:
- Pin to commit hashes, not branches
- Review migrations before upgrade: `graft show <dep>@<ref>`
- Validate integrity regularly: `graft validate`
- Audit dependencies: limit to trusted sources

**For publishers**:
- Sign releases: `git tag -s`
- Don't access network in migrations
- Document what migrations do
- Minimal permissions (don't require sudo, etc.)

**Tooling improvements**:
- `--dry-run` for upgrades (show what will run)
- Signature verification for git tags
- Migration diff between versions
- Sandboxed execution (future)

---

### 8. Ecosystem Comparison

#### How Graft Compares to Package Managers

| Aspect | Go Modules | npm | pip | Cargo | Graft |
|--------|------------|-----|-----|-------|-------|
| **Transitives** | Yes (MVS) | Yes (hoist) | Yes | Yes (semver) | **No** |
| **Lock file** | go.sum | package-lock | manual | Cargo.lock | graft.lock |
| **Versioning** | semver | semver | flexible | semver | **git refs** |
| **What's managed** | Libraries | Packages | Packages | Crates | **Influences** |
| **Conflict handling** | MVS picks | Nesting | Fails | Semver | **N/A** |

#### Does Flat-Only Have Precedent?

**Partial precedents**:
- **Early Go (GOPATH)** - No transitive resolution, manual dep management
- **Vendoring** - Commit deps to repo, downstream sees committed results
- **Git submodules (non-recursive)** - Shallow, direct-only

**What's unique about Graft**:
- Combines flat-only with semantic lock file + migrations
- "Influences" vs "components" - absorb patterns, not link libraries
- Grafted content is committed, not linked at runtime

#### The Key Distinction

**Traditional package managers**: Dependencies are **components** that execute at runtime.
- Function A calls B calls C → all must be present
- Transitive resolution essential

**Graft**: Dependencies are **influences** that shape your repo.
- Migrations run once, output committed
- Results become part of YOUR codebase
- Downstream sees YOUR content, not graft chain

#### Learnings from Other Ecosystems

| From | Feature | Graft Relevance |
|------|---------|-----------------|
| Go | Cryptographic integrity (go.sum) | Add commit hash verification |
| Go | `go mod tidy` | `graft tidy` to remove unused |
| npm | Lifecycle scripts | Pre/post hooks for migrations |
| npm | `npm audit` | Security scanning for grafts |
| Cargo | Features (conditional compilation) | Conditional graft content |
| Cargo | Workspaces | Monorepo support |

---

## Implications for Specifications

### Documents to Update

1. **`specification/dependency-layout.md`** - Major revision
   - Remove transitive resolution algorithm
   - Document flat-only cloning
   - Git submodules integration

2. **`specification/lock-file-format.md`** - Simplification
   - Remove `direct`, `requires`, `required_by` fields
   - Simpler schema
   - Update examples

3. **`specification/graft-yaml-format.md`** - Constraints
   - Document migration self-containment requirement
   - Bundling best practices
   - Cross-reference patterns

4. **`specification/core-operations.md`** - Updates
   - `resolve` semantics (flat-only)
   - New commands: `sync`, `inspect --deps`
   - Validation operations

### Decision Updates

1. **`decision-0005-no-partial-resolution.md`**
   - Change status to `superseded`
   - Reference decision-0007

2. **`architecture.md`**
   - Update "Open Questions" section
   - Add flat-only to design principles
   - Update transitive dependency discussion

---

## Open Questions

### Resolved in This Analysis

- ✅ Is flat-only viable? **Yes**
- ✅ Can submodules work? **Yes, with flat-only**
- ✅ How to handle cross-references? **External URLs or explicit deps**
- ✅ What about content aggregation? **Build concern, not Graft's**
- ✅ Do existing tools solve this? **No, different problem space**

### Remaining Questions

- **Caching strategy**: How to cache git fetches for performance?
- **Multi-version upgrades**: Handling v1 → v2 → v3 path with intermediate migrations
- **Parallel execution**: Running migrations in parallel when safe
- **Workspace support**: Monorepo with multiple graft.yaml files
- **Windows compatibility**: Testing submodule workflows on Windows
- **Migration ordering**: When multiple deps have migrations, what order?

### Future Enhancements

- `graft inspect <dep> --deps` - Show dependency's dependencies
- `graft tidy` - Remove unused dependencies
- `graft audit` - Security scanning
- Pre/post migration hooks
- Conditional content based on environment
- Signature verification for releases

---

## Conclusion

The flat-only dependency model is **viable, cleaner, and better aligned with Graft's purpose** than full transitive resolution.

**Key reasons**:
1. Grafts are influences, not components
2. Self-contained migrations are better design
3. Enables git submodules for better DX
4. Simpler lock file, simpler implementation
5. Explicit over implicit (declare what you use)

**Trade-offs accepted**:
- Discovery friction (can't auto-see transitive deps)
- Coordination overhead (deep hierarchies require management)
- No automatic cross-references (must use URLs or explicit deps)

**Next steps**:
- [Decision 0007](../docs/specifications/decisions/decision-0007-flat-only-dependencies.md) adopted
- Specification updates needed
- Implementation in Graft CLI

---

## References

### Related Documents

- [Decision 0007: Flat-Only Dependencies](../docs/specifications/decisions/decision-0007-flat-only-dependencies.md) - The decision based on this analysis
- [2026-01-12 Dependency Management Exploration](./2026-01-12-dependency-management-exploration.md) - Original analysis
- [Decision 0005: No Partial Resolution](../docs/specifications/decisions/decision-0005-no-partial-resolution.md) - Superseded by Decision 0007

### External Resources

- **Git Submodules**: https://git-scm.com/book/en/v2/Git-Tools-Submodules
- **git-subrepo**: https://github.com/ingydotnet/git-subrepo
- **Google repo**: https://source.android.com/docs/setup/reference/repo
- **vcstool (ROS)**: https://github.com/dirk-thomas/vcstool
- **Go Modules (MVS)**: https://research.swtch.com/vgo-mvs
- **Git X-Modules**: https://gitmodules.com/
