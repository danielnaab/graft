# Team Handbook - Working Agreements

**Last Updated:** 2025-11-29
**Team:** Platform Engineering
**Status:** Living document (updated via Graft workflow)

This handbook codifies our working agreements—how we work together, make decisions, and maintain our processes. These agreements emerge from our retrospectives and evolve as we learn.

---

## Communication

### Async Standups

**Agreement:** Daily async standup updates by 10am local time

**Format:**
- What I completed yesterday
- What I'm working on today
- Any blockers or help needed
- Minimum 3 sentences

**Rationale:** Team spans 3 time zones; synchronous daily meetings cause scheduling friction (Retro Sprint 1, 2025-11-01)

**Participation Target:** 80%+ compliance

**Sync Meetings:** Optional sync-ups for blockers, not required for daily updates

**Effective:** 2025-11-01
**Review:** After 3 sprints (2025-12-15)

---

## Code Review

### Review Expectations

**Agreement:** Code reviews within 24 hours, or explicit communication if unavailable

**Standards:**
- Respond to PR within 24 hours
- If unable to review, comment with timeline: "Can't review until Friday"
- Focus on: correctness, maintainability, test coverage, documentation

**Escalation:** If 48 hours without review, author can merge with one approval

**Rationale:** Reducing PR wait time improves flow and prevents context switching (Retro Sprint 2, 2025-11-15)

**Effective:** Sprint 2 (2025-11-15)

---

## Meetings

### Sprint Planning

**Agreement:** Sprint planning maximum 90 minutes

**Format:**
- Review roadmap and priorities (15min)
- Refine top stories (45min)
- Capacity planning and commitment (20min)
- Parking lot for deep dives (10min)

**Rationale:** Previous plannings ran 3+ hours, causing fatigue (Retro Sprint 1)

**Facilitation:** Rotate facilitator each sprint

**Effective:** Sprint 2 (2025-11-01)

---

## Pairing and Collaboration

### Pair Programming

**Agreement:** Pairing encouraged but not required; teams decide per-task

**When to Pair:**
- Complex features requiring multiple perspectives
- Onboarding new team members
- Knowledge sharing on unfamiliar code
- Debugging gnarly issues

**When Solo is Fine:**
- Well-understood tasks
- Individual deep work (concentration required)
- Small bug fixes or refactoring

**How to Request Pairing:**
- Post in team chat: "Looking for pairing partner for [task]"
- No obligation to accept; respect focus time
- Use Tuple or VS Code Live Share for remote pairing

**Rationale:** Different tasks benefit differently from pairing; team autonomy preferred over mandates (Retro Sprint 3, 2025-11-29)

**Effective:** Sprint 3 (2025-11-29)

---

## Quality and Definition of Done

### Definition of Done

**Agreement:** A story is "Done" when:

1. **Code Complete:**
   - Implementation matches acceptance criteria
   - Code reviewed and approved
   - No known bugs

2. **Tests:**
   - Unit tests for business logic
   - Integration tests for API endpoints
   - Test coverage: New code >80%

3. **Documentation:**
   - README updated if setup changed
   - API docs updated if endpoints changed
   - Runbook updated if operational procedures changed

4. **Deployed to Staging:**
   - Deployed to staging environment
   - Smoke tests pass
   - Product Owner reviewed (if customer-facing)

5. **Monitoring:**
   - Logs include context for debugging
   - Metrics instrumented (if performance-critical)
   - Alerts configured (if reliability-critical)

**Rationale:** Recent rollback showed we shipped without adequate verification; deployment verification prevents production issues (Retro Sprint 3, 2025-11-29)

**Effective:** Sprint 4 (2025-12-01)

### Technical Debt

**Agreement:** Allocate 20% of sprint capacity to technical debt

**Implementation:**
- Reserve ~7 points per sprint for refactoring/cleanup
- Can be used for: refactoring, dependency upgrades, test improvements, documentation
- Not a hard rule; adjust based on sprint priorities

**Tracking:** "Tech Debt" label in Jira, reviewed in sprint planning

**Rationale:** Preventing debt accumulation maintains long-term velocity (Retro Sprint 3, 2025-11-29)

**Review:** Quarterly assessment of debt trends

**Effective:** Sprint 4 (2025-12-01)

---

## Architecture and Technical Decisions

### Architecture Decision Records (ADRs)

**Agreement:** Use ADRs for significant architectural decisions

**When to Write an ADR:**
- Technology choices (languages, frameworks, databases)
- Architectural patterns (microservices vs. monolith, event-driven, etc.)
- API design decisions (REST vs. GraphQL, versioning strategy)
- Infrastructure decisions (cloud provider, orchestration, CI/CD)

**Format:**
- Markdown files in `docs/adr/` directory
- Use template: Context, Decision, Consequences
- Number sequentially: `0001-title.md`

**Process:**
1. Draft ADR
2. Share for team feedback (async or in meeting)
3. Update based on discussion
4. Merge when consensus reached

**Rationale:** Team needs shared understanding of technical choices and their context (Retro Sprint 2, 2025-11-15)

**Effective:** Sprint 2 (2025-11-15)

---

## On-Call

### On-Call Rotation

**Agreement:** Weekly on-call rotation with 15-minute handoff sync

**Rotation:**
- Weekly rotation (Monday 9am - Monday 9am local time)
- Handoff meeting required: 15min sync between outgoing and incoming
- Handoff template: Current issues, ongoing investigations, pending follow-ups

**Escalation:**
- See [On-Call Runbook](../runbooks/on-call-runbook.md) for escalation contacts
- Database/Infrastructure issues: Escalate to #infra-oncall Slack channel
- Security issues: Page security team via PagerDuty

**Rationale:** Recent incidents showed context loss during handoffs (Retro Sprint 1, 2025-11-01)

**Effective:** Sprint 2 (2025-11-08)

---

## Change History

This section tracks when agreements were added or modified:

| Date | Change | Source |
|------|--------|--------|
| 2025-11-01 | Initial async standup agreement | Retro Sprint 1 |
| 2025-11-01 | Sprint planning time limit | Retro Sprint 1 |
| 2025-11-01 | On-call handoff meeting | Retro Sprint 1 |
| 2025-11-15 | Code review SLA | Retro Sprint 2 |
| 2025-11-15 | ADR process | Retro Sprint 2 |
| 2025-11-29 | Pairing guidelines | Retro Sprint 3 |
| 2025-11-29 | Definition of Done | Retro Sprint 3 |
| 2025-11-29 | Technical debt allocation | Retro Sprint 3 |

---

## How to Update This Handbook

This handbook is a **graft artifact** that depends on team retrospectives.

**Process:**
1. Retrospectives capture decisions and action items
2. When retrospectives change, this artifact becomes "dirty"
3. Run: `graft run artifacts/working-agreements/`
4. Review the evaluated template guidance in `.graft/evaluated/guidance.md`
5. Update `team-handbook.md` to reflect new agreements
6. Open PR for team discussion (for significant changes)
7. Finalize: `graft finalize artifacts/working-agreements/ --agent "Your Name"`

**Why this workflow?**
- Decisions trace back to retrospectives (provenance)
- Changes require team visibility (PR review for consensus)
- History shows evolution of agreements (git log)
- New team members see context (linked retrospectives)

---

**Dependencies:**
- Retrospectives: `sources/retrospectives/2025-Q1-*.md`
- Related: [On-Call Runbook](../runbooks/on-call-runbook.md)
