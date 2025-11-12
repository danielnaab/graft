# Sprint Retrospective - 2025 Q1 Sprint 2

**Date:** 2025-11-15
**Facilitator:** Jordan Lee
**Participants:** Alex, Jordan, Sam, Riley, Morgan

## What Went Well

- Async standups working much better (85% participation)
- On-call handoff meetings are valuable—caught 2 potential issues proactively
- No production incidents this sprint
- Documentation quality has improved significantly
- Sprint planning finished in 75 minutes (within target)

## What Could Be Improved

- Need better visibility into what's blocking work—blockers mentioned in standups but not tracked
- CI pipeline is slow (20min builds), slowing down iteration
- Unclear decision-making process for architectural changes
- Working agreement about code review SLAs needed—some PRs sat for 3 days

## Action Items

1. **[HIGH]** Create working agreement for code review expectations
   - Owner: Morgan
   - Due: Before next sprint
   - Proposal: Reviews within 24 hours, or explicit "can't review until..." message

2. **[HIGH]** Establish architectural decision record (ADR) process
   - Owner: Alex
   - Due: Draft template by end of sprint
   - Context: Confusion about whether to use GraphQL or REST for new API

3. **[MEDIUM]** Add blocker tracking to sprint board
   - Owner: Riley
   - Due: Implement in Jira next week
   - Board column: "Blocked" with required blocker description

4. **[LOW]** Investigate CI performance improvements
   - Owner: Sam
   - Due: Spike next sprint
   - Target: Under 10 minutes for PR builds

## Decisions Made

- **Decision:** Adopt ADR process for significant architectural decisions
  - Rationale: Team needs shared understanding of technical choices and their context
  - Format: Markdown files in `docs/adr/` directory
  - Effective: Immediately for new decisions

- **Decision:** Code review SLA: 24 hours or explicit communication
  - Rationale: Reducing PR wait time improves flow
  - Escalation: If 48 hours without review, author can merge with one approval
  - Effective: Next sprint

## Metrics

- Sprint velocity: 37 points (up from 34)
- Incidents: 0 🎉
- Test coverage: 89% (trending up)
- Deployment frequency: 15 deploys
- Async standup participation: 85% (up from 60%)

## Shoutouts

- Jordan for running an efficient sprint planning
- Sam for the improved on-call runbook (used it twice already)
- Everyone for adapting to async standup format
