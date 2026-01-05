---
status: working
created: 2026-01-05
updated: 2026-01-05
purpose: "Actionable plan to improve Graft documentation compliance with meta-knowledge-base best practices"
target_completion: "Before feature/sync-with-specification merge"
related:
  - ../../notes/2026-01-05-meta-knowledge-base-compliance-analysis.md
  - ../architecture.md
  - ../../meta-knowledge-base/docs/meta.md
---

# Meta-Knowledge-Base Compliance Improvement Plan

**Plan Status:** Working (Ready for implementation)
**Created:** 2026-01-05
**Target:** 95%+ compliance before feature branch merge
**Current Compliance:** ~80%

## Overview

This plan provides specific, actionable tasks to improve Graft documentation's alignment with meta-knowledge-base best practices. Each phase includes clear success criteria, file-specific changes, and verification steps.

**Related Analysis:** [Compliance Analysis](../../notes/2026-01-05-meta-knowledge-base-compliance-analysis.md)

## Implementation Phases

Tasks are organized by priority and risk level for systematic implementation.

---

## Phase 1: Add Lifecycle Markers

**Priority:** Immediate
**Effort:** 1-2 hours
**Risk:** Low
**Compliance Impact:** +20% (40% → 60%)

### Objective

Add lifecycle status markers to all documentation files following [meta-KB lifecycle policy](../../meta-knowledge-base/policies/lifecycle.md).

### Tasks

#### Task 1.1: Add Status to Stable Documentation

Add YAML frontmatter to production-ready user and developer docs.

**Frontmatter Template:**
```yaml
---
status: stable
updated: 2026-01-05
---
```

**Files to Update:**

1. `/home/coder/graft/docs/README.md`
   - Add frontmatter at line 1
   - Status: stable (production-ready architecture doc)

2. `/home/coder/graft/docs/guides/user-guide.md`
   - Add frontmatter at line 1
   - Status: stable (comprehensive user guide, well-tested)

3. `/home/coder/graft/docs/guides/contributing.md`
   - Add frontmatter at line 1
   - Status: stable (established development patterns)

4. `/home/coder/graft/docs/cli-reference.md`
   - Add frontmatter at line 1
   - Status: stable (documents all implemented commands)

5. `/home/coder/graft/docs/configuration.md`
   - Add frontmatter at line 1
   - Status: stable (documents current config formats)

6. `/home/coder/graft/docs/index.md`
   - Add frontmatter at line 1
   - Status: stable (navigation index)

#### Task 1.2: Add Status to Working Documents

Mark status/ tracking documents with appropriate lifecycle markers.

**Frontmatter Template for Status Documents:**
```yaml
---
status: working
purpose: "Track implementation progress through Phase 1-10"
updated: 2026-01-05
archive_after: "Phase 10 completion"
archive_to: "notes/archive/2026-01-implementation-tracking.md"
---
```

**Files to Update:**

1. `/home/coder/graft/status/implementation.md`
   - Add frontmatter with archive trigger: "Phase 10 completion"

2. `/home/coder/graft/status/gap-analysis.md`
   - Add frontmatter with archive trigger: "All gaps resolved"

3. `/home/coder/graft/status/workflow-validation.md`
   - Add frontmatter with archive trigger: "Migrate to tests/docs/"

4. `/home/coder/graft/status/phase-8.md`
   - Add frontmatter with archive trigger: "Phase 8 completion"

#### Task 1.3: Mark Living Documents

Identify and mark documents that are continuously updated.

**Frontmatter Template for Living Documents:**
```yaml
---
status: living
purpose: "Session handoff - always reflects current state"
updated: 2026-01-05
archive_policy: "Snapshot before major milestones, keep latest"
---
```

**Files to Update:**

1. `/home/coder/graft/continue-here.md`
   - Add frontmatter marking as living document
   - Document snapshot policy

2. `/home/coder/graft/tasks.md`
   - Add frontmatter marking as living document
   - Clarify it tracks current work only

### Verification

**Success Criteria:**
- ✅ All 13 target files have status markers
- ✅ Status values are valid: stable, working, or living
- ✅ Living and working docs have archive/update policies

**Verification Commands:**
```bash
# Check all docs have frontmatter with status
grep -L "^status:" docs/*.md docs/guides/*.md docs/plans/*.md status/*.md continue-here.md tasks.md

# Expected output: Empty (all files have status)

# List all statuses to verify values
grep "^status:" docs/*.md docs/guides/*.md docs/plans/*.md status/*.md continue-here.md tasks.md | sort | uniq -c

# Expected distribution:
#   6 status: stable (user/dev docs)
#   4 status: working (status tracking)
#   2 status: living (continue-here, tasks)
```

---

## Phase 2: Add Provenance Sections

**Priority:** Medium (after Phase 1)
**Effort:** 3-4 hours
**Risk:** Medium
**Compliance Impact:** +30% (60% → 90%)

### Objective

Add `## Sources` sections to documents making operational or architectural claims, following [meta-KB provenance policy](../../meta-knowledge-base/policies/provenance.md).

### Tasks

#### Task 2.1: Add Sources to Architecture Documentation

**File:** `/home/coder/graft/docs/README.md`

**Location:** Add new section after existing content (end of file)

**Content to Add:**
```markdown
## Sources

This architecture documentation is grounded in:

**Canonical Specifications:**
- [Graft Architecture Specification](../../graft-knowledge/docs/architecture.md) - System design decisions
- [ADR 004: Protocol-Based DI](decisions/004-protocol-based-dependency-injection.md) - Dependency injection approach
- [ADR 005: Functional Service Layer](decisions/005-functional-service-layer.md) - Service design pattern
- [ADR 002: Filesystem Snapshots](decisions/002-filesystem-snapshots-for-rollback.md) - Rollback mechanism
- [ADR 001: Explicit Ref in Upgrade](decisions/001-require-explicit-ref-in-upgrade.md) - CLI design

**Implementation Evidence:**
- Domain models: `src/graft/domain/*.py` (frozen dataclasses)
- Services: `src/graft/services/*.py` (pure functions)
- Protocols: `src/graft/protocols/*.py` (structural subtyping)
- Adapters: `src/graft/adapters/*.py` (infrastructure implementations)
- CLI commands: `src/graft/cli/commands/*.py` (8 commands)

**Validation:**
- Tests: `tests/unit/` (12 modules, 150+ tests)
- Integration tests: `tests/integration/` (4 modules, 800+ lines)
- Workflow validation: [workflow-validation.md](../status/workflow-validation.md)
```

#### Task 2.2: Add Sources to User Guide

**File:** `/home/coder/graft/docs/guides/user-guide.md`

**Location:** Add new section before "Table of Contents" (after frontmatter)

**Content to Add:**
```markdown
## About This Guide

This guide provides practical applications of Graft specifications:

**Canonical Specifications:**
- [Change Model](../../../graft-knowledge/docs/specification/change-model.md) - Semantic change definitions
- [graft.yaml Format](../../../graft-knowledge/docs/specification/graft-yaml-format.md) - Configuration schema
- [Lock File Format](../../../graft-knowledge/docs/specification/lock-file-format.md) - Lock file schema
- [Core Operations](../../../graft-knowledge/docs/specification/core-operations.md) - Command semantics

**Implementation References:**
- CLI commands: `src/graft/cli/commands/*.py`
- Configuration parser: `src/graft/services/config_service.py`
- Examples tested against: Working implementation (Phase 1-8 complete)

---
```

#### Task 2.3: Add Sources to CLI Reference

**File:** `/home/coder/graft/docs/cli-reference.md`

**Location:** Add after title, before first command section

**Content to Add:**
```markdown
## Documentation Sources

This reference documents implemented commands with links to specifications and code.

**For each command:**
- **Specification:** [Core Operations Spec](../../graft-knowledge/docs/specification/core-operations.md)
- **Implementation:** `src/graft/cli/commands/` (linked per command below)
- **Tests:** `tests/integration/test_cli_commands.py` (805 lines of CLI tests)

---
```

**Then for each command section, add implementation reference:**

Example for `graft resolve`:
```markdown
### graft resolve

Clone or fetch all dependencies from `graft.yaml`.

**Implementation:** `src/graft/cli/commands/resolve.py`
**Specification:** [Core Operations: Resolve](../../graft-knowledge/docs/specification/core-operations.md#resolve)

```

**Repeat for all 8 commands:**
- resolve.py
- apply.py
- status.py
- changes.py
- show.py
- upgrade.py
- fetch.py
- exec_command.py

#### Task 2.4: Add Sources to Configuration Documentation

**File:** `/home/coder/graft/docs/configuration.md`

**Location:** Add after title, before first section

**Content to Add:**
```markdown
## Canonical Format Specifications

This document provides examples and guidance for configuration files.

**Authoritative Sources:**
- [graft.yaml Format Specification](../../graft-knowledge/docs/specification/graft-yaml-format.md) - Canonical schema and validation rules
- [Lock File Format Specification](../../graft-knowledge/docs/specification/lock-file-format.md) - graft.lock schema and semantics

**Implementation:**
- Parser: `src/graft/services/config_service.py` - Configuration parsing and validation
- Lock file adapter: `src/graft/adapters/lock_file.py` - YAML lock file I/O
- Domain models: `src/graft/domain/graft_config.py`, `src/graft/domain/lock_entry.py`

**Note:** This is interpretive documentation. When in doubt, refer to canonical specifications above.

---
```

### Verification

**Success Criteria:**
- ✅ 4 key documents have `## Sources` sections
- ✅ Sources reference both specs (graft-knowledge) and implementation (code)
- ✅ CLI reference links each command to implementation file
- ✅ Authority boundaries are clear (interpretation vs. canonical)

**Verification Commands:**
```bash
# Check for Sources sections in target files
grep "^## Sources" docs/README.md docs/guides/user-guide.md docs/cli-reference.md docs/configuration.md

# Expected: 4 matches

# Verify graft-knowledge references
grep -r "graft-knowledge" docs/*.md docs/guides/*.md | wc -l

# Expected: 10+ references to canonical specs
```

---

## Phase 3: Clarify Authority Boundaries

**Priority:** Medium
**Effort:** 2-3 hours
**Risk:** Medium
**Compliance Impact:** +10% (90% → 100%)

### Objective

Make explicit which documents are interpretations of canonical specifications vs. canonical themselves, following [meta-KB authority policy](../../meta-knowledge-base/policies/authority.md).

### Tasks

#### Task 3.1: Add Interpretation Notes

Add authority boundary markers to documents that interpret graft-knowledge specs.

**Files to Update:**

1. **docs/README.md** (Architecture doc)

Add after title and frontmatter:
```markdown
> **Authority Note:** This document provides a developer-friendly overview of Graft's implementation architecture. For canonical architectural decisions, see [graft-knowledge/docs/architecture.md](../../graft-knowledge/docs/architecture.md) and [ADRs](../../graft-knowledge/docs/decisions/).
```

2. **docs/guides/user-guide.md**

Add to "About This Guide" section (from Phase 2):
```markdown
> **Authority Note:** This guide interprets canonical specifications from [graft-knowledge](../../graft-knowledge/) for practical application. When specifications and this guide conflict, specifications are authoritative.
```

3. **docs/configuration.md**

Already has note in Phase 2.4. Verify it's clear.

#### Task 3.2: Update knowledge-base.yaml

Enhance the canonical sources declaration to distinguish interpretation from canonical.

**File:** `/home/coder/graft/knowledge-base.yaml`

**Current (lines 27-34):**
```yaml
sources:
  canonical:
    - path: "../graft-knowledge/docs/architecture.md"
      note: "Architecture decisions come from graft-knowledge (specs)"
    - path: "../graft-knowledge/docs/decisions/**"
      note: "ADRs are maintained in graft-knowledge"
    - path: "docs/structure.md"
      note: "Code structure documentation is canonical for implementation"
```

**Enhanced version:**
```yaml
sources:
  canonical:
    - path: "../graft-knowledge/docs/architecture.md"
      note: "Architecture decisions come from graft-knowledge (specs)"
    - path: "../graft-knowledge/docs/decisions/**"
      note: "ADRs are maintained in graft-knowledge"
    - path: "../graft-knowledge/docs/specification/**"
      note: "All format and operation specifications are canonical"
    - path: "src/graft/**/*.py"
      note: "Source code is canonical for implementation details"
    - path: "docs/decisions/**"
      note: "Implementation-specific ADRs (error handling, etc.)"

  interpretation:
    - path: "docs/README.md"
      note: "Developer-friendly architecture overview, interprets graft-knowledge specs"
    - path: "docs/guides/**"
      note: "User guides - practical application of canonical specifications"
    - path: "docs/configuration.md"
      note: "Configuration examples and guidance, interprets format specs"

  tracking:
    - path: "status/**"
      note: "Implementation tracking - working documents, not canonical"
    - path: "notes/**"
      note: "Time-bounded exploration and session logs"
```

#### Task 3.3: Reduce Duplication

Identify sections in graft docs that duplicate graft-knowledge and convert to references.

**Target:** docs/README.md sections that explain "what we're building" rather than "how we built it"

**Approach:**
- Keep implementation-specific details (protocols, adapters, CLI integration)
- Convert high-level "what is a Change?" to references to graft-knowledge
- Focus on "we implemented using..." rather than "the system does..."

**Example Transformation:**

Before:
```markdown
### Change Model

A Change represents a semantic change in a dependency. It includes fields for ref, type, description, migration command, and verification.
```

After:
```markdown
### Change Model

Implements the [Change Model specification](../../graft-knowledge/docs/specification/change-model.md) using frozen dataclasses (`src/graft/domain/change.py:15-45`).

**Implementation approach:** Immutable value objects with full type safety and validation.
```

### Verification

**Success Criteria:**
- ✅ Interpretation docs have authority boundary notes
- ✅ knowledge-base.yaml distinguishes canonical from interpretation
- ✅ Reduced duplication with graft-knowledge
- ✅ Clear "refer to spec for X, this doc for Y" pattern

**Verification:**
```bash
# Check for authority notes
grep -i "authority note" docs/*.md docs/guides/*.md

# Expected: 3 matches

# Verify knowledge-base.yaml has interpretation section
grep -A 10 "interpretation:" knowledge-base.yaml

# Should show interpretation sources listed
```

---

## Phase 4: Establish Status Document Lifecycle

**Priority:** Low (nice-to-have before merge)
**Effort:** 2 hours
**Risk:** Low
**Compliance Impact:** +5% (organizational clarity)

### Objective

Document explicit lifecycle management for status/ tracking documents and establish archival patterns.

### Tasks

#### Task 4.1: Document Lifecycle Policy

Add lifecycle guidance to architecture documentation.

**File:** `/home/coder/graft/docs/architecture.md`

**Location:** Add new section before "Sources"

**Content:**
```markdown
## Status Document Lifecycle

### Temporal Layers

Graft uses three temporal layers following [meta-KB temporal stratification](../../meta-knowledge-base/policies/temporal-layers.md):

**Ephemeral (notes/):**
- Purpose: Session logs, exploration, learning
- Lifecycle: draft → archived
- Retention: Days to weeks
- Examples: `notes/2026-01-05-*.md`

**Tracking (status/):**
- Purpose: Active implementation tracking, handoff
- Lifecycle: working → deprecated → archived
- Retention: Duration of feature work
- Examples: `status/implementation.md`, `continue-here.md`

**Durable (docs/):**
- Purpose: Architecture, guides, decisions
- Lifecycle: draft → working → stable → deprecated
- Retention: Indefinite
- Examples: This file, user guides, ADRs

### Status Directory Lifecycle

**Current Status Documents:**

| File | Purpose | Archive Trigger | Archive Destination |
|------|---------|----------------|---------------------|
| implementation.md | Phase 1-10 tracking | Phase 10 completion | `notes/archive/2026-01-implementation-tracking.md` |
| gap-analysis.md | Spec vs impl gaps | All gaps resolved | Merge into implementation.md archive |
| workflow-validation.md | E2E testing results | Feature stable | `tests/docs/workflow-validation.md` (permanent) |
| phase-8.md | CLI implementation | Phase 8 complete | Merge into implementation.md |

**continue-here.md Lifecycle:**
- Status: Living document (always current)
- Snapshot policy: Before major milestones (e.g., branch merge)
- Snapshot location: `notes/archive/YYYY-MM-DD-continue-here-snapshot.md`

**tasks.md Lifecycle:**
- Status: Living document (current work only)
- Archive policy: Completed tasks removed, not archived
- Historical tracking: Git history provides task evolution

### Archival Process

When status docs reach archive trigger:

1. **Create snapshot:**
   ```bash
   cp status/implementation.md notes/archive/2026-01-implementation-tracking.md
   ```

2. **Add archival note to original:**
   ```markdown
   ---
   status: deprecated
   deprecated_date: 2026-01-XX
   archived_to: notes/archive/2026-01-implementation-tracking.md
   reason: "Feature work complete, merged to main"
   ---

   # [ARCHIVED] Implementation Status

   This document is archived. See [archived version](../notes/archive/2026-01-implementation-tracking.md).
   ```

3. **Update docs/index.md:** Remove links to deprecated status docs

### Notes Archive Directory

Create `notes/archive/` for archived status documents:

**File:** `notes/archive/README.md`
```markdown
# Archived Notes

Historical development notes archived after feature completion.

These documents are kept for context but are not actively maintained.

## Index

- `2026-01-implementation-tracking.md` - Phase 1-10 implementation status (archived: feature merge)
- `2026-01-XX-continue-here-snapshot.md` - Session handoff at merge point
```
```

#### Task 4.2: Create Archive Directory

Set up the archive infrastructure.

```bash
mkdir -p /home/coder/graft/notes/archive
```

Create README as specified in Task 4.1.

### Verification

**Success Criteria:**
- ✅ Lifecycle documented in architecture.md
- ✅ Each status doc has clear archive trigger
- ✅ Archive directory exists with README
- ✅ Archival process is documented

**Verification:**
```bash
# Check lifecycle section exists
grep "Status Document Lifecycle" docs/architecture.md

# Verify archive directory
ls -la notes/archive/

# Should show README.md
```

---

## Phase 5: Fix Linking Issues

**Priority:** Immediate (quick win)
**Effort:** 30 minutes
**Risk:** Very Low
**Compliance Impact:** +5% (linking policy compliance)

### Objective

Ensure all links follow [meta-KB linking policy](../../meta-knowledge-base/policies/linking.md): real markdown links for navigation, backticks only for literal paths.

### Tasks

#### Task 5.1: Fix Absolute Path in index.md

**File:** `/home/coder/graft/docs/index.md`

**Line 206:**

Current:
```markdown
See [meta-knowledge-base style policy](file:///home/coder/meta-knowledge-base/policies/style.md) for full standards.
```

Fixed:
```markdown
See [meta-knowledge-base style policy](../../meta-knowledge-base/policies/style.md) for full standards.
```

#### Task 5.2: Audit for Other Linking Issues

Search for potential issues:

```bash
# Find absolute file:// paths
grep -r "file:///" docs/

# Find cases where code paths should be links
# (Manual review needed - look for src/ paths that should link)
grep -r "src/graft" docs/ | grep -v "## Sources"
```

**Review each match:**
- In prose text describing code → Keep as backtick
- In Sources sections → Convert to link or file:line reference
- In architecture explanations → Consider adding file:line references

### Verification

**Success Criteria:**
- ✅ No file:/// absolute paths in docs
- ✅ Code references follow consistent pattern
- ✅ Navigation links work correctly

**Verification:**
```bash
# Should return no matches
grep -r "file:///" docs/

# All links should be relative
grep -r "]\(../" docs/ | wc -l
# Expected: Many matches (good - using relative paths)
```

---

## Phase 6: Add Code-Level Provenance (Enhancement)

**Priority:** Low (high value, but optional for initial compliance)
**Effort:** 4-5 hours
**Risk:** Low
**Compliance Impact:** +0% (compliance satisfied, but quality improved)

### Objective

Enhance documentation with specific file:line references for stronger grounding in implementation.

### Tasks

#### Task 6.1: Add Code References to Architecture Doc

**File:** `/home/coder/graft/docs/README.md`

Enhance each subsection with implementation references.

**Pattern:**
```markdown
### [Component Name]

[Description] (`src/graft/path/file.py:start-end`)

**Specification:** [Link to graft-knowledge spec]
**Tests:** `tests/unit/test_*.py` or `tests/integration/test_*.py`
```

**Example Enhancement:**

Current:
```markdown
### Domain Models

- **Change**: Semantic change representation (breaking, feature, fix)
```

Enhanced:
```markdown
### Domain Models

Located in `src/graft/domain/`:

**Change** (`src/graft/domain/change.py:15-45`)
- Semantic change representation (breaking, feature, fix)
- Specification: [Change Model](../../graft-knowledge/docs/specification/change-model.md)
- Tests: `tests/unit/test_domain_change.py:1-92` (22 tests)
```

**Apply to all subsections:**
- Domain Models (4 models)
- Services (6 services)
- Protocols (6 protocols)
- Adapters (6 adapters)
- CLI Commands (8 commands)

#### Task 6.2: Add Test References

For claims about behavior, reference tests that validate them.

**Example:**

Current:
```markdown
Atomic upgrades with rollback mechanism ensure safe dependency updates.
```

Enhanced:
```markdown
Atomic upgrades with rollback mechanism ensure safe dependency updates (`src/graft/services/upgrade_service.py:45-120`).

**Validation:**
- Unit tests: `tests/unit/test_upgrade_service.py` (454+ lines)
- Integration tests: `tests/integration/test_snapshot_integration.py:50-95` (rollback scenarios)
- Workflow validation: [Upgrade workflow](../status/workflow-validation.md#upgrade-workflow)
```

### Verification

**Success Criteria:**
- ✅ Major architectural components have file:line references
- ✅ Behavioral claims reference tests
- ✅ References are accurate (files/lines exist)

**Verification:**
```bash
# Check for file:line references (pattern: file.py:NN)
grep -E "\\.py:[0-9]+" docs/README.md | wc -l

# Expected: 20+ references (one per major component)

# Verify files exist
grep -oE "src/graft/[a-z/_]+\\.py" docs/README.md | while read f; do
  [ -f "$f" ] || echo "Missing: $f"
done

# Expected: No output (all files exist)
```

---

## Implementation Strategy

### Recommended Order

1. **Phase 5** (30 min) - Quick win, fix linking
2. **Phase 1** (1-2 hours) - High impact, low risk
3. **Phase 2** (3-4 hours) - High value for maintainability
4. **Phase 3** (2-3 hours) - Clarity and authority
5. **Phase 4** (2 hours) - Organizational preparation
6. **Phase 6** (4-5 hours) - Optional enhancement

**Total Estimated Time:** 12-16 hours (excluding Phase 6)

### Incremental Approach

Each phase can be implemented independently:
- Commit after each phase completes
- Verify success criteria before proceeding
- Adjust priorities based on feedback

### Agent Implementation Notes

For agents implementing this plan:

**Before starting each phase:**
1. Read the phase objectives and tasks
2. Review success criteria
3. Check related policy documents (linked)

**During implementation:**
1. Make minimal, precise changes
2. Follow templates exactly
3. Preserve existing content structure
4. Don't add additional improvements beyond scope

**After completing each phase:**
1. Run verification commands
2. Check success criteria
3. Commit with descriptive message
4. Update this plan's status

## Progress Tracking

**Phase Completion:**
- [ ] Phase 5: Fix Linking Issues
- [ ] Phase 1: Add Lifecycle Markers
- [ ] Phase 2: Add Provenance Sections
- [ ] Phase 3: Clarify Authority Boundaries
- [ ] Phase 4: Establish Status Document Lifecycle
- [ ] Phase 6: Add Code-Level Provenance (optional)

**Compliance Target:** 95%+ (100% with Phase 6)

## Success Metrics

**Before (Current):**
- Lifecycle markers: 40%
- Provenance coverage: 30%
- Authority clarity: 70%
- Linking compliance: 90%
- **Overall: ~80%**

**After (Target):**
- Lifecycle markers: 100%
- Provenance coverage: 100%
- Authority clarity: 95%
- Linking compliance: 100%
- **Overall: 95-100%**

## Sources

This plan is based on:

**Analysis:**
- [Meta-KB Compliance Analysis](../../notes/2026-01-05-meta-knowledge-base-compliance-analysis.md)

**Policies:**
- [Meta-KB Lifecycle Policy](../../meta-knowledge-base/policies/lifecycle.md)
- [Meta-KB Provenance Policy](../../meta-knowledge-base/policies/provenance.md)
- [Meta-KB Authority Policy](../../meta-knowledge-base/policies/authority.md)
- [Meta-KB Linking Policy](../../meta-knowledge-base/policies/linking.md)

**Project Context:**
- [Graft Architecture](../architecture.md)
- [Graft knowledge-base.yaml](../../knowledge-base.yaml)
- [Graft-Knowledge Specifications](../../graft-knowledge/docs/specification/)

---

**Plan created:** 2026-01-05
**Status:** Ready for implementation
**Next action:** Begin Phase 5 (quick linking fix)
