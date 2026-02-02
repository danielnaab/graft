# Improvement Plans

This directory contains strategic improvement plans for the Graft documentation and knowledge base.

## Superseded Plans

### ~~Upgrade to graft-knowledge v2~~ (Superseded)

**Plan:** [upgrade-to-graft-knowledge-v2.md](upgrade-to-graft-knowledge-v2.md)
**Created:** 2026-01-05
**Status:** Superseded by Decision 0007 (Flat-Only Dependency Model)

> **Note:** This plan was superseded by Decision 0007, which removed transitive dependency
> resolution entirely in favor of a simpler flat-only model. The v2 lock file format
> (with `direct`, `requires`, `required_by` fields) was replaced by the simpler v3 format.
> These documents are preserved for historical reference.

**Related historical documents:**
- [upgrade-analysis.md](upgrade-analysis.md) - Analysis of the v2 upgrade attempt
- [testing-v2-upgrade.md](testing-v2-upgrade.md) - Testing notes from v2 implementation

---

## Active Plans

### Meta-Knowledge-Base Compliance Improvements

**Plan:** [meta-kb-compliance-improvements.md](meta-kb-compliance-improvements.md)
**Created:** 2026-01-05
**Status:** Ready for implementation
**Priority:** High (before feature branch merge)

**Summary:** Systematic plan to improve Graft documentation's alignment with meta-knowledge-base best practices from ~80% to 95%+ compliance.

**Key improvements:**
- Add lifecycle markers to all documentation
- Add provenance sections to ground claims in specs and code
- Clarify authority boundaries between graft and graft-knowledge
- Establish status document lifecycle and archival
- Fix linking policy issues
- Optional: Add code-level provenance for enhanced maintainability

**Supporting analysis:** [notes/2026-01-05-meta-knowledge-base-compliance-analysis.md](../../notes/2026-01-05-meta-knowledge-base-compliance-analysis.md)

**Implementation approach:**
- 6 phases from immediate quick wins to optional enhancements
- Each phase has clear success criteria and verification steps
- Total estimated time: 12-16 hours (excluding optional Phase 6)
- Recommended order: Phase 5 → 1 → 2 → 3 → 4 → 6

**For agents:** This plan is structured for systematic implementation. Read each phase carefully, follow templates exactly, and verify success criteria before proceeding.

## Navigation

**Related documentation:**
- [Meta-knowledge-base policies](../../../meta-knowledge-base/policies/) - Upstream policies we're aligning to
- [Graft architecture](../architecture.md) - Current information architecture
- [Graft agents entrypoint](../agents.md) - Agent workflow and conventions

**Related analysis:**
- [Compliance analysis](../../notes/2026-01-05-meta-knowledge-base-compliance-analysis.md) - Deep dive into current state and gaps

---

**Purpose:** This directory organizes strategic improvement plans that guide multi-session work. Plans here are actionable, evidence-based, and structured for both human and agent implementation.
