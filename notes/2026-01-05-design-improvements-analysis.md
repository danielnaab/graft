---
title: "Work Log: Design Improvements Analysis"
date: 2026-01-05
status: in-progress
authors: ["Design Review Team"]
---

# Work Log: Design Improvements Analysis

## Objective

Review design-related recommendations from `graft-improvements-recommendations.md` and synthesize them into elegant, cohesive improvements to Graft's architecture and specifications.

## Methodology

1. **Extract design-relevant recommendations** - Identify which recommendations impact Graft's fundamental design vs. implementation details
2. **Validate against philosophy** - Ensure recommendations align with Graft's core design principles
3. **Synthesize improvements** - Combine related recommendations into cohesive design enhancements
4. **Specify changes** - Update specifications with concrete, implementable changes

## Graft's Core Design Principles

From `docs/architecture.md`, Graft's philosophy is:

1. **Git-Native** - Use git refs as identity, leverage git primitives
2. **Explicit Over Implicit** - Declarative, validatable, not magical
3. **Minimal Primitives** - Only Change and Dependency as core concepts
4. **Separation of Concerns** - graft.yaml for automation, CHANGELOG for humans
5. **Atomic Operations** - All-or-nothing, no partial states
6. **Composability** - Commands can chain, migrations can compose

## Analysis of Recommendations

### Recommendation #7: Specification Enhancements

**Status**: ✅ Design-relevant, High Priority

**Proposed improvements:**

1. **Lock file ordering convention**
   - Specify direct deps before transitive
   - Improves readability and human understanding
   - **Assessment**: ✅ Valid - Aligns with "Explicit Over Implicit"

2. **API version semantics**
   - Clarify `graft/v0`, `graft/v1` meaning
   - Document when to bump versions
   - **Assessment**: ✅ Valid - Critical for ecosystem stability

3. **Conflict detection examples**
   - Show concrete scenarios where conflicts occur
   - Document expected error format
   - **Assessment**: ✅ Valid - Supports "Explicit Over Implicit"

4. **Migration guide section**
   - v1 → v2 upgrade path
   - Automatic migration strategy
   - **Assessment**: ✅ Valid - Reduces friction, improves adoption

5. **Extended examples**
   - Multi-level dependency chains
   - Conflict scenarios
   - **Assessment**: ✅ Valid - Clarifies semantics

6. **Decision log**
   - Design rationale (ADR style)
   - Why flat layout, why tuples, etc.
   - **Assessment**: ✅ Valid - Helps implementers understand trade-offs

**Validation**: All sub-recommendations are valid and align with Graft's philosophy.

---

### Exploration Idea #1: Workspace Support (Monorepo)

**Status**: 🤔 Requires design consideration

**Question**: Should Graft support monorepos with multiple knowledge bases sharing dependencies?

**Proposed structure:**
```
workspace/
  project-a/
    graft.yaml
  project-b/
    graft.yaml
  .graft/  # Shared deps for entire workspace
  workspace.yaml  # Workspace configuration
```

**Design considerations:**

1. **Alignment with philosophy:**
   - ✅ **Composability** - Multiple projects compose in workspace
   - ⚠️ **Minimal Primitives** - Adds "Workspace" as new concept
   - ✅ **Explicit Over Implicit** - Workspace must be declared
   - ❌ **Simplicity** - Increases complexity significantly

2. **Trade-offs:**
   - **Pros**:
     - Efficiency (shared deps clone once)
     - Common pattern in modern tools (npm workspaces, cargo workspace)
     - Natural for organizations with multiple related KBs
   - **Cons**:
     - Adds complexity to resolution algorithm
     - New failure modes (workspace vs project conflicts)
     - Scope creep - not core to knowledge dependency management

3. **Alternative approaches:**
   - **Option A**: Explicit workspace support with `workspace.yaml`
   - **Option B**: Document pattern using symlinks or scripts
   - **Option C**: Defer until proven demand exists

**Recommendation**:
- **Short-term**: Document workaround patterns (symlink .graft/)
- **Long-term**: Add workspace support if demand emerges
- **Spec impact**: Add to "Future Considerations" section, not core spec
- **Rationale**: Respects "Minimal Primitives" - don't add until needed

---

### Exploration Idea #2: Content Addressing / Integrity Verification

**Status**: 🤔 Requires security analysis

**Question**: Is git commit hash sufficient for integrity, or should we add content checksums?

**Current state:**
- Lock file stores `commit` (SHA-1 hash of git commit)
- Git already provides integrity via commit hash
- Assumes git itself is trusted

**Design considerations:**

1. **Threat model:**
   - **Scenario 1**: Attacker compromises git remote, changes commit history
     - Git commit hash changes → detected
   - **Scenario 2**: Attacker performs SHA-1 collision on git commit
     - Theoretically possible but extremely difficult
     - Git moving to SHA-256
   - **Scenario 3**: Local .graft/ directory tampered with
     - Content differs from commit → not currently detected

2. **Alignment with philosophy:**
   - ✅ **Git-Native** - Git already provides integrity model
   - ❌ **Minimal Primitives** - Adding content checksums duplicates git
   - ⚠️ **Explicit Over Implicit** - Could make integrity more explicit

3. **Comparison to other systems:**
   - **npm**: Uses SHA-512 content hashes (`integrity` field)
   - **cargo**: Uses checksums in Cargo.lock
   - **git submodules**: Uses commit hash only (like Graft)

**Recommendations:**

**Tier 1 (High priority - Spec now):**
- Add `graft validate --integrity` command that:
  - Verifies `.graft/<dep>/` is at expected commit
  - Runs `git rev-parse HEAD` and compares to lock file
  - Detects local tampering or drift
- **Spec impact**: Add to validation operations
- **Rationale**: Uses existing git primitives, no new concepts

**Tier 2 (Future - Explore later):**
- Research content-addressed storage (similar to pnpm)
- Consider if SHA-256 migration requires spec changes
- **Spec impact**: Future considerations section

**Assessment**: ✅ Tier 1 is valid design improvement, Tier 2 deferred

---

### Exploration Idea #3: Partial Dependency Resolution

**Status**: 🤔 Premature optimization

**Question**: Should Graft support resolving only needed dependencies, not all transitive deps?

**Proposed scenario:**
```yaml
# Project has:
deps:
  meta-kb:   # Requires: standards-kb, templates-kb, utils-kb

# User only wants:
graft resolve --only meta-kb standards-kb
# Skip templates-kb, utils-kb
```

**Design considerations:**

1. **Alignment with philosophy:**
   - ❌ **Atomic Operations** - Partial resolution creates partial state
   - ❌ **Reproducibility** - Different machines might resolve differently
   - ❌ **Explicit Over Implicit** - Which deps are "needed" is ambiguous

2. **Trade-offs:**
   - **Pros**:
     - Faster resolution for large graphs
     - Saves disk space
   - **Cons**:
     - Breaks reproducibility (core requirement)
     - Complex to implement correctly
     - Unclear semantics ("needed" by what?)

3. **Performance concerns:**
   - Is resolution actually slow enough to matter?
   - Typical KB projects: 5-20 dependencies
   - Resolution time: <10 seconds typical
   - **Current state**: No evidence of performance problem

**Recommendation**:
- ❌ **Reject** - Violates core principles (Atomic, Reproducibility)
- **Alternative**: Optimize full resolution (parallel cloning, caching)
- **Spec impact**: None - explicitly not supported
- **Document why**: Add to decisions/ explaining rejection rationale

**Assessment**: ❌ Invalid - conflicts with core philosophy

---

### Exploration Idea #4: Dependency Caching / Mirror Support

**Status**: ✅ Valid, Medium Priority

**Question**: Should Graft support git mirrors for enterprise/offline use?

**Proposed configuration:**
```yaml
# In project or global config
mirrors:
  "https://github.com/*": "https://internal-mirror.corp/*"
  "git@github.com:*": "git@internal-git.corp:*"
```

**Design considerations:**

1. **Alignment with philosophy:**
   - ✅ **Git-Native** - Leverages git's existing mirror capabilities
   - ✅ **Explicit Over Implicit** - Mirrors explicitly configured
   - ✅ **Minimal Primitives** - No new concepts, just URL rewriting

2. **Real-world use cases:**
   - Corporate environments with security policies
   - Air-gapped or offline development
   - Reliability (internal mirrors always available)
   - Speed (local mirrors faster than internet)

3. **Implementation complexity:**
   - **Low** - Simple URL pattern matching and rewriting
   - Git itself handles mirroring, Graft just rewrites URLs
   - Similar to npm registry configuration, cargo mirrors

**Recommendation**:
- ✅ **Accept** - Add mirror support to specification
- **Scope**:
  - Add `mirrors` section to graft.yaml or global config
  - Simple glob-pattern based URL rewriting
  - Fallback to original if mirror unavailable
- **Spec impact**:
  - Add to graft.yaml specification
  - Add to configuration documentation
  - Document in security/enterprise guide

**Specification additions:**

```yaml
# In graft.yaml (project-level) or ~/.graft/config.yaml (global)
mirrors:
  - pattern: "https://github.com/*"
    replace: "https://mirror.corp/*"
  - pattern: "git@github.com:*"
    replace: "git@mirror.corp:*"
```

**Semantics:**
- Patterns matched in order (first match wins)
- Original URL preserved in lock file (not rewritten URL)
- Mirrors are transparent - reproducibility maintained
- If mirror fails, optionally fall back to original

**Assessment**: ✅ Valid design improvement, should be specified

---

## Design Decisions Summary

### ✅ Approved for Specification

1. **Recommendation #7: Specification Enhancements** - All 6 sub-items
2. **Integrity Verification** - Add `graft validate --integrity` using git primitives
3. **Mirror Support** - Add mirror configuration for enterprise use
4. **Decision Documentation** - Create ADR-style decision logs

### 🔄 Deferred for Future Consideration

1. **Workspace Support** - Document workarounds, add to future considerations
   - Rationale: Premature - wait for proven demand
   - Action: Add to "Future Enhancements" section

### ❌ Rejected (Document Rationale)

1. **Partial Dependency Resolution** - Conflicts with atomicity and reproducibility
   - Rationale: Violates core principles
   - Action: Create decision document explaining why not supported

---

## Synthesis: Elegant Design Improvements

After analyzing all recommendations, we synthesize the following cohesive improvements:

### Improvement 1: Enhanced Lock File Specification

**What**: Clarify lock file semantics and conventions

**Changes**:
1. Specify ordering convention (direct → transitive, alphabetical within each group)
2. Add integrity verification semantics
3. Clarify version field semantics
4. Add complete conflict detection examples

**Rationale**: Makes lock file specification more complete and implementer-friendly

---

### Improvement 2: API Versioning and Evolution

**What**: Formalize how graft.yaml/graft.lock versions evolve

**Changes**:
1. Define `apiVersion` semantics clearly:
   - `graft/v0` - Experimental, breaking changes allowed
   - `graft/v1` - Stable, backward compatible within v1.x
   - Future versions signal breaking changes
2. Document migration strategy between versions
3. Add compatibility matrix showing which tool versions support which specs

**Rationale**: Critical for ecosystem stability as Graft matures

---

### Improvement 3: Validation and Integrity Operations

**What**: Formalize validation semantics in specification

**Changes**:
1. Add validation operations to core operations spec:
   - `graft validate config` - graft.yaml syntax and semantics
   - `graft validate lock` - graft.lock format and consistency
   - `graft validate integrity` - verify .graft/ matches lock commits
2. Specify validation failure modes and error formats
3. Document exit codes and machine-readable output

**Rationale**: Enables CI/CD integration, improves reliability

---

### Improvement 4: Mirror and Offline Support

**What**: Add enterprise-friendly mirror configuration

**Changes**:
1. Add `mirrors` configuration section
2. Define URL pattern matching and rewriting semantics
3. Specify fallback behavior
4. Document that lock file stores original URLs (mirrors are transparent)

**Rationale**: Supports enterprise adoption without compromising principles

---

### Improvement 5: Decision Documentation

**What**: Capture design rationale in ADR format

**Changes**:
1. Create new decision documents:
   - Decision 0005: Lock file ordering conventions
   - Decision 0006: Why no partial resolution
   - Decision 0007: Mirror support design
   - Decision 0008: API versioning semantics
2. Link decisions from specifications
3. Explain trade-offs and alternatives considered

**Rationale**: Helps future implementers understand why, not just what

---

## Implementation Plan

### Phase 1: Specification Updates (Current)

1. ✅ Create this work log
2. ⏳ Update `lock-file-format.md` with ordering convention and examples
3. ⏳ Update `core-operations.md` with validation semantics
4. ⏳ Update `graft-yaml-format.md` with mirror configuration
5. ⏳ Create new decision documents (0005-0008)
6. ⏳ Add migration guide for v1→v2 lock file changes

### Phase 2: Reference Implementation (Graft tool)

1. Implement validation commands
2. Implement mirror support
3. Update lock file generation with ordering
4. Add migration tooling

### Phase 3: Documentation (graft-knowledge)

1. Add examples covering all new features
2. Create enterprise/CI guide covering mirrors and validation
3. Update README with links to new decisions

---

## Open Questions

### Q1: Mirror configuration location

**Question**: Should mirrors be in project graft.yaml or separate config?

**Options**:
- A: Project `graft.yaml` - Explicit, versioned with project
- B: Global `~/.graft/config.yaml` - Machine-specific, not in git
- C: Both, with project overriding global

**Recommendation**: Option C - Most flexible
- Global for machine/org defaults
- Project can override for specific deps
- Explicit merge semantics (project overrides global)

### Q2: Validation exit codes

**Question**: What exit codes should `graft validate` use?

**Proposal**:
- `0` - All validations passed
- `1` - Validation failed (invalid config/lock)
- `2` - Integrity mismatch (lock vs .graft/)
- `3` - Conflict detected

**Recommendation**: Keep simple - Use 0 for success, 1 for any failure
- More complex exit codes can be added later if needed
- Machine-readable JSON output for detailed errors

### Q3: Conflict detection strictness

**Question**: Should identical source+ref but different commit be error or warning?

**Scenario**:
```yaml
# meta-kb depends on standards-kb main@abc123
# docs-kb depends on standards-kb main@def456
```

**Options**:
- A: Error - Strict, refs must resolve to same commit
- B: Warning - Allow, but inform user
- C: Configurable strictness

**Recommendation**: Option A (Error) - Predictability over flexibility
- Knowledge bases need consistency
- User can upgrade one dep to resolve
- Avoids subtle content inconsistencies

---

## Next Steps

1. ✅ Complete this work log
2. Review and approve synthesis with stakeholders
3. Update specifications per approved improvements
4. Create decision documents
5. Implement in reference tool (graft)
6. Test and validate changes
7. Publish updated specifications

---

## Changelog

- **2026-01-05 14:00**: Initial work log created
  - Analyzed all 7 recommendations plus 4 exploration ideas
  - Validated against core design principles
  - Approved: Spec enhancements, integrity verification, mirrors, decision docs
  - Deferred: Workspace support
  - Rejected: Partial resolution
  - Synthesized 5 cohesive design improvements
  - Identified 3 open questions for resolution
