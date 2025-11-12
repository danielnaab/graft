# Sprint Retrospective - 2025 Q1 Sprint 3

**Date:** 2025-11-29
**Facilitator:** Sam Rodriguez
**Participants:** Alex, Jordan, Sam, Riley, Morgan, Casey (new team member)

## What Went Well

- New ADR process working well—clarity on API design decisions
- Code review turnaround improved dramatically (avg 6 hours)
- Casey's onboarding smooth thanks to updated runbooks and working agreements
- Blocker tracking on board helps visibility
- CI improvements shipped—builds now 12min (40% faster)

## What Could Be Improved

- Need working agreement about pair programming—when is it helpful vs. overhead?
- Incident response procedure needs updating—had deployment rollback but process wasn't clear
- Definition of Done is vague—different interpretations between team members
- Technical debt starting to accumulate—need strategy for addressing it

## Action Items

1. **[HIGH]** Update incident response runbook with deployment rollback steps
   - Owner: Riley
   - Due: This week (fresh from experience)
   - Include: Rollback command, health check steps, notification procedure

2. **[HIGH]** Codify Definition of Done in working agreements
   - Owner: Casey (good onboarding task)
   - Due: Draft by end of next sprint
   - Include: Tests, docs, PR review, deployment checklist

3. **[MEDIUM]** Create working agreement about pairing expectations
   - Owner: Morgan
   - Due: Discuss in next planning
   - Questions: When to pair? How to request? Remote pairing tools?

4. **[MEDIUM]** Schedule technical debt discussion for next planning
   - Owner: Alex
   - Due: Add to planning agenda
   - Goal: Allocate 20% of sprint capacity to debt

## Decisions Made

- **Decision:** Allocate 20% of sprint capacity to technical debt
  - Rationale: Preventing debt accumulation, maintaining velocity
  - Implementation: Reserve ~7 points per sprint for refactoring/cleanup
  - Review: Quarterly assessment of debt trends
  - Effective: Next sprint

- **Decision:** Pair programming is encouraged but not required; teams decide per-task
  - Rationale: Different tasks benefit differently from pairing
  - Guidelines: Default to pairing for: complex features, onboarding, knowledge sharing
  - Solo work fine for: well-understood tasks, individual deep work
  - Effective: Immediately

- **Decision:** Update Definition of Done to include deployment verification
  - Rationale: Recent rollback showed we shipped without adequate verification
  - New requirement: Deployed to staging + smoke tests pass before marking Done
  - Effective: Next sprint

## Metrics

- Sprint velocity: 35 points (within normal range)
- Incidents: 1 (deployment issue, rolled back in 10min using runbook)
- Test coverage: 88% (slight dip but within tolerance)
- Deployment frequency: 14 deploys
- Code review avg time: 6 hours (target: <24h) ✓

## Shoutouts

- Casey for jumping in quickly and contributing from day one
- Riley for handling the deployment rollback calmly and documenting it
- Morgan for mentoring Casey on team processes
- Sam for CI improvements that save everyone time daily
