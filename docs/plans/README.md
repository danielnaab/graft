# Improvement Plans

This directory contains strategic improvement plans for the Graft documentation and knowledge base.

## Active Plans

### Upgrade to graft-knowledge v2

**Plan:** [upgrade-to-graft-knowledge-v2.md](upgrade-to-graft-knowledge-v2.md)
**Created:** 2026-01-05
**Status:** Ready for implementation
**Priority:** High (enables transitive dependency support)

**Summary:** Upgrade Graft implementation to align with graft-knowledge v2 specifications for extended lock file format, transitive dependency resolution, and flat dependency layout.

**Key changes:**
- Extend graft.lock format with `direct`, `requires`, `required_by` fields
- Implement recursive dependency resolution algorithm
- Migrate from `.graft/deps/` to `.graft/` flat layout
- Add conflict detection for version mismatches
- Update all CLI commands to support new features

**Post-implementation:**
- [upgrade-analysis.md](upgrade-analysis.md) - Evaluation of upgrade process
- [graft-improvements-recommendations.md](graft-improvements-recommendations.md) - Enhancement proposals

**Implementation approach:**
- 7 phases over 3 weeks (domain models → resolution → layout → CLI → visualization → docs)
- Each phase has clear testing strategy and success criteria
- Maintains backward compatibility during migration

**For agents:** This plan serves dual purposes: (1) implement v2 specifications, (2) evaluate Graft's own dependency upgrade affordances to inform future improvements.

---

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

---

### Auto-Create PRs for Graft Dependency Updates

**Plan:** [auto-dependency-update-prs.md](auto-dependency-update-prs.md)
**Created:** 2026-01-05
**Status:** Draft (awaiting review)
**Priority:** High (enables dependency update automation)

**Summary:** CI automation strategy that automatically creates pull requests in downstream repositories when upstream Graft dependencies are updated. Includes Coder workspace links for continuity.

**Key components:**
- Push-based workflow triggers (upstream notifies downstream)
- `dependents.yaml` configuration for listing consumers
- Reusable scripts for PR creation via Forgejo API
- Coder workspace URL integration for handoff

**Initial scope:**
- `graft-knowledge` → `graft` (notify on update)
- `meta-knowledge-base` → `graft-knowledge` (notify on update)

**For agents:** This plan creates the foundation for automated dependency evolution. It serves dual purposes: (1) keep downstream projects current, (2) generate feedback on Graft's upgrade experience.

---

## Navigation

**Related documentation:**
- [Meta-knowledge-base policies](../../../meta-knowledge-base/policies/) - Upstream policies we're aligning to
- [Graft architecture](../architecture.md) - Current information architecture
- [Graft agents entrypoint](../agents.md) - Agent workflow and conventions

**Related analysis:**
- [Compliance analysis](../../notes/2026-01-05-meta-knowledge-base-compliance-analysis.md) - Deep dive into current state and gaps

---

**Purpose:** This directory organizes strategic improvement plans that guide multi-session work. Plans here are actionable, evidence-based, and structured for both human and agent implementation.
