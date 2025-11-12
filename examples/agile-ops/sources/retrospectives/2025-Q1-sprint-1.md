# Sprint Retrospective - 2025 Q1 Sprint 1

**Date:** 2025-11-01
**Facilitator:** Alex Chen
**Participants:** Alex, Jordan, Sam, Riley, Morgan

## What Went Well

- Successfully deployed the new authentication feature
- Improved test coverage to 87%
- Cross-functional collaboration with design team was smooth
- Documentation updates kept pace with code changes

## What Could Be Improved

- On-call handoff process was unclear—Sam missed the escalation procedure for database issues
- Async standup updates were inconsistent (only 60% participation)
- Sprint planning took 3 hours (too long)
- Deployment rollback procedure not documented

## Action Items

1. **[HIGH PRIORITY]** Document clear on-call handoff and escalation procedures in runbook
   - Owner: Sam
   - Due: Before next sprint starts
   - Context: During the database timeout incident, unclear who to escalate to for infrastructure issues

2. **[MEDIUM]** Establish working agreement about async standup expectations
   - Owner: Jordan
   - Due: Discuss in next planning
   - Proposal: Daily updates by 10am local time, minimum 3 sentences (what done, what next, blockers)

3. **[MEDIUM]** Create working agreement about meeting time limits
   - Owner: Alex
   - Due: Codify in team handbook
   - Proposal: Sprint planning max 90 minutes, use parking lot for deep dives

4. **[LOW]** Document deployment rollback procedure
   - Owner: Riley
   - Due: End of next sprint
   - Link to runbook update

## Decisions Made

- **Decision:** Adopt async-first standups with optional sync sync-ups for blockers
  - Rationale: Team spans 3 time zones; synchronous daily meetings cause scheduling friction
  - Effective: Immediately
  - Review: After 3 sprints

- **Decision:** On-call rotation stays weekly, but handoff requires 15min sync meeting
  - Rationale: Recent incidents showed context loss during handoffs
  - Effective: Next rotation cycle

## Metrics

- Sprint velocity: 34 points (up from 29)
- Incidents: 1 (database timeout, mitigated in 45min)
- Test coverage: 87% (target: 85%)
- Deployment frequency: 12 deploys (up from 8)

## Shoutouts

- Sam for quick thinking during the database incident
- Morgan for pairing with new team member on authentication flows
- Riley for automating the test report generation
