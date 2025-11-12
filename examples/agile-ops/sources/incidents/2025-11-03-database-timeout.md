# Incident Post-Mortem: Database Connection Timeout

**Date:** 2025-11-03
**Severity:** Medium (degraded service, no data loss)
**Duration:** 45 minutes
**Incident Commander:** Sam Rodriguez
**Participants:** Sam, Alex, Jordan, Infrastructure Team

## Summary

On November 3, 2025 at 14:23 UTC, users experienced intermittent 504 Gateway Timeout errors when accessing the dashboard. Investigation revealed the database connection pool was exhausted due to a query performance regression introduced in the previous deploy.

## Impact

- ~15% of dashboard requests failed with timeouts
- Approximately 200 users affected
- No data loss or corruption
- Service fully restored at 15:08 UTC (45min duration)

## Timeline (all times UTC)

- **14:23** - Monitoring alerts: elevated 504 error rate
- **14:25** - Sam paged via PagerDuty
- **14:28** - Sam confirms issue, begins investigation
- **14:32** - Sam identifies database connection pool exhaustion
- **14:35** - Unclear who to escalate to for database investigation (searched runbook, no escalation contacts)
- **14:40** - Sam contacts Alex who reaches out to Infrastructure team via Slack
- **14:45** - Infrastructure team joins incident channel
- **14:52** - Identified slow query in recent deploy (missing index on `user_sessions.last_active`)
- **14:55** - Decision: Rollback deploy vs. add index → chose rollback for speed
- **15:00** - Rollback initiated
- **15:05** - Rollback complete, connection pool recovers
- **15:08** - Monitoring confirms 504 rate back to normal
- **15:15** - Incident resolved

## Root Cause

A database migration in the previous deploy (v2.14.0) modified the `user_sessions` table but did not add an index on the `last_active` column. A new feature query filtered by this column, causing full table scans. Under normal load, these slow queries exhausted the connection pool.

## What Went Well

- Monitoring detected the issue quickly (2 min)
- Sam responded to page within 3 minutes
- Rollback procedure worked smoothly
- Clear incident channel communication
- No data loss

## What Went Wrong

- **On-call runbook did not include escalation contacts** for database/infrastructure issues
  - Sam lost ~10 minutes trying to figure out who to contact
  - Eventually found help via informal Slack message to Alex
- Missing database query performance testing in CI
- Migration review process didn't catch missing index
- No pre-deploy performance validation

## Action Items

1. **[HIGH - IN PROGRESS]** Update on-call runbook with clear escalation contacts
   - Owner: Sam
   - Due: 2025-11-05
   - Status: Draft in PR (linked to retrospective action item)
   - Include: Database/Infra contacts, Slack channels, PagerDuty escalation policy

2. **[HIGH]** Add database query performance tests to CI
   - Owner: Jordan
   - Due: 2025-11-15
   - Check: Queries on tables >10K rows must use indexes

3. **[MEDIUM]** Improve migration review checklist
   - Owner: Alex
   - Due: 2025-11-20
   - Add: Index requirements, query plan review for schema changes

4. **[MEDIUM]** Document deployment rollback procedure in runbook
   - Owner: Riley
   - Due: 2025-12-01
   - Include: Commands, health checks, rollback decision criteria

5. **[LOW]** Add pre-deploy performance smoke tests
   - Owner: Sam
   - Due: 2025-12-15
   - Run: Load test against staging before prod deploy

## Lessons Learned

- **Runbooks are living documents:** Must include current escalation contacts and be kept up to date
- **Performance testing gaps:** Schema changes need performance validation, not just functional tests
- **Escalation clarity matters:** Lost 10min during incident due to unclear escalation path
- **Rollback is valuable:** Having a documented, tested rollback procedure reduced MTTR

## Follow-Up

- [ ] Verify escalation contacts are current in updated runbook
- [ ] Test escalation procedure in next on-call rotation
- [ ] Schedule review of other runbook gaps in next retrospective
- [ ] Add incident response to onboarding checklist for new team members

---

**Related:**
- Sprint Retrospective 2025-Q1-Sprint-1 (action item #1)
- On-call Runbook (to be updated)
