---
status: deprecated
date: 2026-02-14
completed: 2026-02-14
archived-reason: "Documentation cleanup completed - files moved to notes/"
---

# State Panel Documentation Review & Improvement Plan

**Purpose**: Assess state panel documentation against meta-knowledge-base policies and plan cleanup

---

## Current State Assessment

### Documentation Files Created

**In Repository Root** (❌ Wrong location):
```
GROVE-STATE-INTEGRATION-CRITIQUE.md       11.6 KB  (earlier critique)
STATE-PANEL-CRITIQUE.md                   17.1 KB  (comprehensive critique)
STATE-PANEL-PHASE1-PLAN.md               19.0 KB  (implementation plan)
STATE-PANEL-PHASE1-COMPLETE.md           11.6 KB  (implementation summary)
STATE-QUERIES-COMPLETE.md                 9.8 KB  (earlier implementation)
```

**In Correct Locations** (✅ Good):
```
docs/specifications/grove/tui-behavior.md         (canonical spec - UPDATED)
grove/src/tui.rs                                  (source code - UPDATED)
grove/src/tui_tests.rs                           (tests - UPDATED)
grove/tests/test_state_panel.rs                  (integration tests)
```

---

## Policy Compliance Analysis

### ❌ Temporal Layers Policy Violations

**Issue**: All session documents are in root directory

Per [temporal-layers.md](/.graft/meta-knowledge-base/docs/policies/temporal-layers.md):

| Document Type | Current Location | Should Be In | Retention |
|--------------|------------------|--------------|-----------|
| Implementation plans | Root | `notes/` (ephemeral) | Days to weeks |
| Session summaries | Root | `notes/` (ephemeral) | Archive after insights extracted |
| Critiques | Root | `notes/` or `docs/decisions/` | Depends on reusability |

**What temporal layers mean**:
- **Ephemeral** (`notes/`): Session logs, plans, scratch work → Archive after completion
- **Tracking** (`status/`): Implementation progress → Deprecated when milestone reached
- **Durable** (`docs/`, `decisions/`): Architecture, guides → Keep indefinitely

**Our files are ephemeral** (session-specific plans and summaries) but stored as if durable.

---

### ❌ Lifecycle Status Missing

**Issue**: No frontmatter with status markers

Per [lifecycle.md](/.graft/meta-knowledge-base/docs/policies/lifecycle.md), all documents should have:

```yaml
---
status: draft | working | stable | deprecated
---
```

**Current state**: Zero files have status frontmatter.

**Should be**:
- Critiques: `status: working` (active analysis)
- Plans: `status: deprecated` (implementation complete)
- Completion summary: `status: working` (until archived)
- Specification: `status: working` (already has this ✅)

---

### ❌ Provenance Missing

**Issue**: Claims lack source citations

Per [provenance.md](/.graft/meta-knowledge-base/docs/policies/provenance.md), operational guidance and factual claims need:
- `## Sources` section
- References to file + heading or code symbols

**Example missing provenance**:
- Critique claims "Users can't tell if cache is stale" → Should cite user feedback or code inspection
- Plan estimates "3.5 hours" → Should cite similar tasks or velocity data
- Completion summary claims "A- (90%)" → Should cite grading rubric

**Should have**: Sources section in each document linking to:
- Code locations (e.g., `grove/src/tui.rs:1398-1465`)
- Related specs (e.g., `docs/specifications/grove/tui-behavior.md#state-panel`)
- Prior decisions or benchmarks

---

### ⚠️ Redundancy and Overlap

**Issue**: Multiple documents cover same ground

```
GROVE-STATE-INTEGRATION-CRITIQUE.md  ─┐
                                      ├─ Both are critiques, 60% overlap
STATE-PANEL-CRITIQUE.md              ─┘

STATE-QUERIES-COMPLETE.md            ─┐
                                      ├─ Both are "done" summaries
STATE-PANEL-PHASE1-COMPLETE.md       ─┘
```

**Should consolidate**: One critique (most recent), one completion summary per milestone.

---

### ✅ What's Done Well

1. **Canonical spec updated**: `docs/specifications/grove/tui-behavior.md` has complete scenarios
2. **Source code is authoritative**: Implementation in `grove/src/tui.rs` is the truth
3. **Tests verify spec**: 146 tests ensure spec-impl alignment
4. **Clear entrypoints**: AGENTS.md and CLAUDE.md guide both humans and agents

---

## Meta-Knowledge-Base Compliance Scorecard

| Policy | Compliance | Issues | Priority |
|--------|-----------|--------|----------|
| **Authority** | ✅ Good | Spec is canonical, source is truth | - |
| **Temporal Layers** | ❌ Poor | All ephemeral docs in root | HIGH |
| **Lifecycle** | ❌ Poor | No status frontmatter | MEDIUM |
| **Provenance** | ❌ Poor | No source citations | MEDIUM |
| **Linking** | ✅ Good | Relative paths work | - |
| **Style** | ✅ Good | Plain language, no emojis | - |
| **Write Boundaries** | ✅ Good | Specs not modified by agents | - |

**Overall Grade**: C (60%) - Functional but not policy-compliant

---

## Recommended Actions

### Phase 1: Organize by Temporal Layer (15 min)

**Move ephemeral documents to `notes/`**:

```bash
# Move session-specific documents to notes
mv STATE-PANEL-PHASE1-PLAN.md \
   notes/2026-02-14-state-panel-phase1-plan.md

mv STATE-PANEL-PHASE1-COMPLETE.md \
   notes/2026-02-14-state-panel-phase1-complete.md

mv GROVE-STATE-INTEGRATION-CRITIQUE.md \
   notes/2026-02-13-grove-state-integration-critique.md

mv STATE-QUERIES-COMPLETE.md \
   notes/2026-02-13-state-queries-complete.md
```

**Decision: Keep or archive main critique**:

Option A: Move to notes (ephemeral)
```bash
mv STATE-PANEL-CRITIQUE.md \
   notes/2026-02-14-state-panel-critique.md
```

Option B: Distill to ADR (durable)
```bash
# Extract key decisions to:
docs/decisions/state-panel-ux-decisions.md
# Then archive critique to notes/
```

**Recommendation**: Option A (move to notes) - critique is session-specific, not reusable

---

### Phase 2: Add Lifecycle Markers (10 min)

**Add frontmatter to moved documents**:

```yaml
---
status: deprecated
completed: 2026-02-14
archived-reason: "Phase 1 implementation complete, insights extracted to spec"
---
```

**Add to critique (if kept)**:

```yaml
---
status: working
last-reviewed: 2026-02-14
note: "Identifies Phase 2/3 improvements, defer until user demand"
---
```

---

### Phase 3: Add Provenance (15 min)

**Add Sources sections to key documents**:

Example for `notes/2026-02-14-state-panel-critique.md`:

```markdown
## Sources

### Code References
- [State panel rendering](../grove/src/tui.rs#L1398-L1465) - Current implementation
- [State query discovery](../grove/src/state/discovery.rs) - YAML parsing
- [Cache reading](../grove/src/state/cache.rs) - File I/O and timestamp logic

### Specifications
- [TUI Behavior Spec](../docs/specifications/grove/tui-behavior.md#state-panel) - Canonical scenarios
- [State Format Spec](../docs/specifications/graft/state-queries.md) - graft.yaml state section

### Related Decisions
- [State Panel Phase 1 Plan](2026-02-14-state-panel-phase1-plan.md) - Implementation approach
- [Phase 1 Complete](2026-02-14-state-panel-phase1-complete.md) - Delivery summary

### User Feedback
- (None yet - monitoring for Phase 2 prioritization)
```

---

### Phase 4: Consolidate Redundancy (10 min)

**Remove or merge duplicate documents**:

1. **Keep**: `notes/2026-02-14-state-panel-phase1-complete.md` (latest milestone)
2. **Archive**: `notes/2026-02-13-state-queries-complete.md` (superseded)
3. **Keep**: `notes/2026-02-14-state-panel-critique.md` (active for Phase 2/3)
4. **Archive**: `notes/2026-02-13-grove-state-integration-critique.md` (superseded)

**Add to .gitignore or document/archive**:
```bash
# Option 1: Delete superseded files (if extracted to newer docs)
rm notes/2026-02-13-state-queries-complete.md
rm notes/2026-02-13-grove-state-integration-critique.md

# Option 2: Keep in git history but note as archived
git mv notes/2026-02-13-state-queries-complete.md \
       notes/archive/2026-02-13-state-queries-complete.md
```

---

### Phase 5: Update Entrypoints (5 min)

**Ensure discoverability**:

Update `notes/index.md` (if exists) or create it:

```markdown
# Notes Index

## State Panel Implementation (2026-02-14)

Session documents from state panel Phase 1 implementation:

- [Critique](2026-02-14-state-panel-critique.md) - Analysis of 12 issues, Phase 2/3 roadmap
- [Phase 1 Plan](2026-02-14-state-panel-phase1-plan.md) - Implementation blueprint
- [Phase 1 Complete](2026-02-14-state-panel-phase1-complete.md) - Delivery summary

**Status**: Phase 1 shipped (B+ → A-). Phases 2/3 deferred pending user feedback.

**Canonical sources**:
- Spec: [docs/specifications/grove/tui-behavior.md](../docs/specifications/grove/tui-behavior.md)
- Code: [grove/src/tui.rs](../grove/src/tui.rs)
```

---

## Implementation Priority

### HIGH (Do Now)
1. **Move to notes/** (15 min) - Critical for temporal layer compliance
2. **Add status frontmatter** (10 min) - Required for lifecycle tracking

### MEDIUM (Do Soon)
3. **Add provenance** (15 min) - Important for reusability
4. **Consolidate redundancy** (10 min) - Reduces confusion

### LOW (Optional)
5. **Update notes index** (5 min) - Nice for navigation

**Total effort**: 55 minutes

---

## Success Criteria

**Before** (Current State):
- ❌ Ephemeral docs in root directory
- ❌ No lifecycle status markers
- ❌ No source citations
- ❌ Redundant/overlapping files

**After** (Compliant):
- ✅ Ephemeral docs in `notes/` with date prefixes
- ✅ All documents have `status:` frontmatter
- ✅ Key documents cite sources in `## Sources` section
- ✅ One critique, one plan, one summary per milestone
- ✅ Notes index links to session documents

**Meta-KB Grade**: C (60%) → B+ (85%)

---

## Long-Term Documentation Strategy

### Durable Knowledge Locations

**What belongs in `docs/`**:
- Guides for using state queries (if users need help)
- Architecture decisions (if state panel becomes complex)
- Examples (if patterns emerge)

**Currently**: Specification in `docs/specifications/grove/tui-behavior.md` is sufficient.

**Don't create until pain observed**:
- ❌ User guide (wait for confused users)
- ❌ Architecture doc (wait for complexity)
- ❌ Examples directory (wait for patterns)

---

## Next Steps

**Option 1: Clean up now** (Recommended)
- Run Phase 1-4 moves/updates
- ~45 minutes total
- Repo complies with meta-KB policies

**Option 2: Ship as-is, clean later**
- Documents work fine where they are
- Can move to notes/ when we have more session logs
- Lower priority than new features

**Option 3: Minimal compliance**
- Just move to notes/ and add status (25 min)
- Defer provenance and consolidation

**Recommendation**: **Option 1** - Do the full cleanup now while context is fresh.

---

## Automated Cleanup Script

```bash
#!/bin/bash
# cleanup-state-panel-docs.sh

# Create notes directory if needed
mkdir -p notes/archive

# Move current documents to notes with date prefixes
mv STATE-PANEL-PHASE1-PLAN.md notes/2026-02-14-state-panel-phase1-plan.md
mv STATE-PANEL-PHASE1-COMPLETE.md notes/2026-02-14-state-panel-phase1-complete.md
mv STATE-PANEL-CRITIQUE.md notes/2026-02-14-state-panel-critique.md

# Archive superseded documents
mv GROVE-STATE-INTEGRATION-CRITIQUE.md notes/archive/2026-02-13-grove-state-integration-critique.md
mv STATE-QUERIES-COMPLETE.md notes/archive/2026-02-13-state-queries-complete.md

# Add frontmatter (manual step - requires editing each file)
echo "Next: Add status frontmatter to each moved file"
```

---

## Sources

- [Temporal Layers Policy](/.graft/meta-knowledge-base/docs/policies/temporal-layers.md)
- [Lifecycle Policy](/.graft/meta-knowledge-base/docs/policies/lifecycle.md)
- [Provenance Policy](/.graft/meta-knowledge-base/docs/policies/provenance.md)
- [knowledge-base.yaml](/knowledge-base.yaml) - Project structure and conventions
- [State Panel Implementation](notes/2026-02-14-state-panel-phase1-complete.md) - What was built
- [State Panel Critique](notes/2026-02-14-state-panel-critique.md) - Analysis and future work
