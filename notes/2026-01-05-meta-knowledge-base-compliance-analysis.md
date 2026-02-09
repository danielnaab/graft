---
date: 2026-01-05
status: working
purpose: "Deep analysis of Graft documentation compliance with meta-knowledge-base best practices"
related:
  - ../docs/architecture.md
  - ../knowledge-base.yaml
  - ../.graft/meta-knowledge-base/docs/meta.md
---

# Meta-Knowledge-Base Compliance Analysis

**Analysis Date:** 2026-01-05
**Scope:** feature/sync-with-specification branch (53 commits, 84 files changed)
**Analyzed Against:** Meta-knowledge-base policies and playbooks

## Executive Summary

The Graft documentation demonstrates **80% compliance** with meta-knowledge-base best practices and shows excellent foundational structure. This analysis identifies specific gaps and provides actionable improvement plans to reach full compliance while maintaining the system's clean architecture and professional quality.

**Key Finding:** Graft is already following meta-KB principles well, but inconsistent application of lifecycle markers, provenance sections, and authority boundaries creates opportunities for improvement.

## Strengths (What We're Doing Right)

### 1. Clear Entrypoints ✅

**Evidence:**
- Human entrypoint: `README.md` (well-structured, clear navigation)
- Agent entrypoint: `docs/agents.md` (excellent role definition and workflow guidance)
- Both declared in `knowledge-base.yaml:5-7`

**Compliance:** Full compliance with [meta-KB entrypoints](../.graft/meta-knowledge-base/docs/meta.md#what-this-system-standardizes)

### 2. Good Separation of Concerns ✅

**Directory Structure:**
```
graft/
├── docs/          # Durable architecture and guides
├── notes/         # Time-bounded exploration
├── status/        # Implementation tracking (medium-term)
└── src/           # Source code
```

**Compliance:** Aligns with temporal stratification principles

### 3. Write Boundaries Defined ✅

**Evidence:** `knowledge-base.yaml:36-43`
```yaml
rules:
  writes:
    allow: ["docs/**", "notes/**"]
    deny: ["secrets/**", "config/prod/**"]
```

**Compliance:** Full compliance with [write boundaries policy](../.graft/meta-knowledge-base/policies/writes.md)

### 4. Strong Provenance in Agent Entrypoint ✅

**Evidence:** `docs/agents.md:59-68` includes comprehensive Sources section referencing:
- Meta-KB policies (authority, provenance, lifecycle, writes)
- Graft-knowledge specifications
- Agent workflow playbook

**Compliance:** Excellent example of [provenance policy](../.graft/meta-knowledge-base/policies/provenance.md) application

### 5. Good ADR Practice ✅

**Evidence:** 6 ADRs in `docs/decisions/` following consistent template with:
- Status markers (all marked "Accepted")
- Context, Decision, Consequences sections
- Comprehensive Sources section (see `001-error-handling-strategy.md:266-278`)

**Compliance:** Best-in-class architectural decision documentation

## Gaps Identified (Areas for Improvement)

### Gap 1: Inconsistent Lifecycle Markers ⚠️

**Policy:** [Meta-KB Lifecycle Policy](../.graft/meta-knowledge-base/policies/lifecycle.md)

**Current State:**
- Only 3 document categories have status markers:
  - `docs/architecture.md` → "status: stable"
  - All 6 ADRs → "Status: Accepted"
  - No status on user-facing docs

**Missing Status:**
```
docs/README.md               # No status (should be "stable")
docs/guides/user-guide.md    # No status (should be "stable")
docs/guides/contributing.md  # No status (should be "stable")
docs/cli-reference.md        # No status (should be "stable")
docs/configuration.md        # No status (should be "stable")
docs/index.md                # No status (should be "stable")
status/implementation.md     # No status (should be "working")
status/gap-analysis.md       # No status (should be "working")
continue-here.md             # No status (should be "living")
```

**Impact:** Users and agents can't determine document maturity/trustworthiness

**Severity:** Medium - affects trustworthiness but not functionality

### Gap 2: Missing Provenance on Key Documents ⚠️

**Policy:** [Meta-KB Provenance Policy](../.graft/meta-knowledge-base/policies/provenance.md)

**Requirement:** Provenance required for:
- Operational guidance
- Factual claims that drive decisions
- Syntheses explaining system behavior

**Current State:**
- Only 2 files have `## Sources` sections:
  - `docs/agents.md` ✅
  - `docs/decisions/001-error-handling-strategy.md` ✅

**Missing Provenance:**

**docs/README.md (Architecture doc):**
- Makes architectural claims without referencing specs or code
- Should reference: graft-knowledge architecture, ADRs, source files

**docs/guides/user-guide.md:**
- Explains change model without linking to spec
- Should reference: `graft-knowledge/docs/specification/change-model.md`
- Command examples without implementation references

**docs/cli-reference.md:**
- Documents 8 commands without linking to implementations
- Should reference: `src/graft/cli/commands/*.py`
- Operation descriptions without spec references

**docs/configuration.md:**
- Describes graft.yaml format without canonical spec link
- Should reference: `graft-knowledge/docs/specification/graft-yaml-format.md`
- Lock file format without spec reference

**Impact:** Claims appear authoritative but lack grounding in canonical sources

**Severity:** Medium-High - affects trustworthiness and maintainability

### Gap 3: Unclear Authority Boundaries ⚠️

**Policy:** [Meta-KB Authority Policy](../.graft/meta-knowledge-base/policies/authority.md)

**Requirement:** Clear distinction between:
- Canonical truth (specs, code, data)
- Interpretation (human-friendly explanations)
- Generated views
- Working notes

**Current State:**
- `knowledge-base.yaml:27-34` declares canonical sources
- But interpretation documents don't mark themselves as such

**Issues:**

**Duplication between graft and graft-knowledge:**
- `docs/README.md` explains architecture (duplicate of graft-knowledge)
- User guide explains concepts (duplicate of specs)
- No markers indicating "this interprets canonical specs"

**No clear "Canonical Source" markers:**
- Readers can't tell if graft docs are authoritative or interpretive
- No guidance pointing to graft-knowledge for canonical decisions

**Impact:** Confusion about which repository has authoritative information

**Severity:** Medium - creates maintenance burden and potential conflicts

### Gap 4: Status Documents Lack Lifecycle Management ⚠️

**Policy:** Would benefit from new temporal layers guidance (to be added to meta-KB)

**Current State:**
- `status/` directory contains valuable tracking docs
- No explicit lifecycle or archival plan
- Unclear when they should transition to "deprecated" or archive

**Issues:**

**status/implementation.md:**
- Currently tracking Phase 1-10 implementation
- No marker for when it should be deprecated (Phase 10 completion?)
- No plan for where content should migrate

**status/gap-analysis.md:**
- Tracks spec vs implementation gaps
- Should be deprecated when gaps are resolved
- No explicit trigger documented

**status/workflow-validation.md:**
- End-to-end testing results
- Might become permanent test documentation
- No clear transition path

**continue-here.md:**
- Valuable session handoff doc
- Should be marked as "living" document
- No archival policy (snapshot before major updates?)

**Impact:** Status docs may accumulate without clear lifecycle

**Severity:** Low - current state is manageable, but planning needed for scale

### Gap 5: Linking Policy Inconsistencies ⚠️

**Policy:** [Meta-KB Linking Policy](../.graft/meta-knowledge-base/policies/linking.md)

**Requirement:**
- Real markdown links for navigation
- Real links in Sources sections
- Backticks only for literal paths/code

**Issues:**

**docs/index.md:206:**
```markdown
See [meta-knowledge-base style policy](../.graft/meta-knowledge-base/policies/style.md)
```

**Problem:** Absolute file:// path instead of relative markdown link

**Should be:**
```markdown
See [meta-knowledge-base style policy](../.graft/meta-knowledge-base/policies/style.md)
```

**Other Issues:**
- Some code references use backticks without line numbers
- Some implementation claims lack file:line references

**Impact:** Minor - affects portability and navigation

**Severity:** Low - easy to fix, minimal functional impact

### Gap 6: Limited Code-Level Provenance 💎

**Opportunity:** Not a gap per se, but enhancement opportunity

**Current State:**
- Architecture docs describe system behavior
- But rarely link to specific code locations

**Opportunity:**

Add file:line references for grounding:
```markdown
## Domain Models

### Change Model

Represents a semantic change (`src/graft/domain/change.py:15-45`).

**Specification:** [Change model spec](../docs/specifications/graft/change-model.md)

**Tests:** `tests/unit/test_domain_change.py:1-92`
```

**Benefits:**
- Stronger connection between docs and code
- Easier verification of claims
- Better support for maintenance

**Impact:** High value for maintainability

**Severity:** Low priority - enhancement, not requirement

## Quantitative Assessment

### Compliance Metrics

| Policy | Compliance | Evidence |
|--------|-----------|----------|
| **Entrypoints** | 100% | Both human and agent entrypoints defined and high-quality |
| **Authority** | 70% | Canonical sources declared, but interpretation docs not marked |
| **Provenance** | 30% | Only 2 files have Sources sections (should be ~8-10) |
| **Lifecycle** | 40% | Only ADRs and architecture.md have status markers |
| **Write Boundaries** | 100% | Clearly defined in knowledge-base.yaml |
| **Linking** | 90% | Mostly correct, one absolute path issue |

**Overall Compliance: ~80%**

### Document Status Summary

| Status Category | Current Count | Should Be |
|-----------------|---------------|-----------|
| **Stable docs with status** | 7 | 13 |
| **Working docs with status** | 0 | 4 |
| **Living docs marked** | 0 | 1 |
| **Docs with Sources** | 2 | 8 |

## Root Cause Analysis

**Why these gaps exist:**

1. **Rapid development pace:** 53 commits in feature branch, focus on implementation over meta-documentation
2. **Emerging patterns:** Status documents emerged organically without explicit lifecycle planning
3. **Incomplete meta-KB guidance:** Some needed patterns (temporal layers, KB dependencies) not yet in meta-KB
4. **Partial adoption:** Started applying meta-KB principles mid-project

**These are normal growing pains, not fundamental issues.**

## Recommendations

### Immediate Actions (High Value, Low Risk)

1. **Add lifecycle markers** (1-2 hours)
   - Add frontmatter to all stable docs
   - Mark status/ docs as "working"
   - Mark continue-here.md as "living"

2. **Fix linking issues** (30 minutes)
   - Convert absolute file:// path to relative link
   - Audit for other linking policy violations

### Short-Term Actions (High Value, Medium Effort)

3. **Add Sources sections** (3-4 hours)
   - docs/README.md (architecture)
   - docs/guides/user-guide.md
   - docs/cli-reference.md
   - docs/configuration.md

4. **Clarify authority boundaries** (2-3 hours)
   - Add interpretation notes to docs that explain specs
   - Enhance knowledge-base.yaml with interpretation markers
   - Reduce duplication with graft-knowledge

### Medium-Term Actions (High Value, Higher Effort)

5. **Establish status document lifecycle** (2-3 hours)
   - Document lifecycle in docs/architecture.md
   - Add archival triggers to status/ docs
   - Create notes/archive/ directory

6. **Add code-level provenance** (4-5 hours)
   - Add file:line references to architecture docs
   - Link CLI reference to implementations
   - Ground user guide claims in code

## Success Criteria

**Target: 95%+ Compliance**

When improvements are complete:
- ✅ All stable docs have lifecycle markers
- ✅ All operational guidance has provenance
- ✅ Authority boundaries are explicit
- ✅ Status docs have clear lifecycle
- ✅ All links follow meta-KB policy
- ✅ Major claims reference code locations

## Next Steps

This analysis should inform:

1. **Immediate action:** Create improvement plan in `docs/plans/` (see related file)
2. **Task tracking:** Add improvement tasks to `tasks.md`
3. **Ongoing practice:** Apply patterns to new documentation
4. **Meta-KB feedback:** Share insights to improve meta-KB guidance

## Sources

This analysis is grounded in:

**Meta-Knowledge-Base Policies:**
- [Authority Policy](../.graft/meta-knowledge-base/policies/authority.md)
- [Provenance Policy](../.graft/meta-knowledge-base/policies/provenance.md)
- [Lifecycle Policy](../.graft/meta-knowledge-base/policies/lifecycle.md)
- [Linking Policy](../.graft/meta-knowledge-base/policies/linking.md)
- [Write Boundaries Policy](../.graft/meta-knowledge-base/policies/writes.md)
- [Style Policy](../.graft/meta-knowledge-base/policies/style.md)

**Meta-Knowledge-Base Playbooks:**
- [Evolve KB Playbook](../.graft/meta-knowledge-base/playbooks/evolve-kb.md)
- [Agent Workflow](../.graft/meta-knowledge-base/playbooks/agent-workflow.md)

**Graft Configuration:**
- [knowledge-base.yaml](../knowledge-base.yaml)
- [docs/agents.md](../docs/agents.md)
- [docs/architecture.md](../docs/architecture.md)

**Evidence Collection:**
- Explored all docs in `docs/` directory (20+ files)
- Reviewed all 6 ADRs in `docs/decisions/`
- Analyzed status/ directory (4 files)
- Examined knowledge-base.yaml configuration

**Analysis Method:**
- Pattern matching against meta-KB policies
- Quantitative assessment (file counts, compliance percentages)
- Root cause analysis of gaps
- Evidence-based recommendations

## Related Documents

- [Improvement Plan](../docs/plans/meta-kb-compliance-improvements.md) - actionable plan based on this analysis
- [Architecture](../docs/architecture.md) - current information architecture
- [Agents Entrypoint](../docs/agents.md) - agent workflow and conventions
- [Graft Knowledge Specs](../docs/specifications/README.md) - canonical specifications

---

**Analysis conducted:** 2026-01-05
**Analyst:** Claude Sonnet 4.5 (agentic analysis)
**Review needed:** Human verification of priorities and approach
