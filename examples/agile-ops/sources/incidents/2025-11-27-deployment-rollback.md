# Incident Post-Mortem: Failed Deployment Requiring Rollback

**Date:** 2025-11-27
**Severity:** Low (caught before user impact)
**Duration:** 10 minutes
**Incident Commander:** Riley Kumar
**Participants:** Riley, Sam, Morgan

## Summary

On November 27, 2025 at 16:15 UTC, a deployment to production (v2.18.0) caused the health check endpoint to fail. The deployment was caught during post-deploy verification and rolled back within 10 minutes. No users experienced degraded service.

## Impact

- No user impact (caught before traffic routed to new instances)
- ~10 minutes of deployment pipeline blocked
- Development team paused work briefly during rollback
- No data loss or corruption

## Timeline (all times UTC)

- **16:15** - Deployment v2.18.0 initiated via CI/CD
- **16:18** - New instances deployed but health checks failing
- **16:19** - Riley notices health check failures in deployment dashboard
- **16:20** - Riley declares incident, begins investigation
- **16:21** - Morgan joins to assist
- **16:22** - Quick check: `/health` endpoint returning 500 (missing environment variable `FEATURE_FLAGS_URL`)
- **16:23** - Decision: Rollback immediately (low-risk change, fast recovery)
- **16:24** - Riley initiates rollback (had to search for command in docs)
- **16:25** - Rollback command executed: `kubectl rollout undo deployment/api-server`
- **16:27** - Previous version (v2.17.3) restored
- **16:28** - Health checks pass
- **16:29** - Monitoring confirms normal operation
- **16:30** - Incident resolved

## Root Cause

The deployment introduced a new feature flag system requiring a new environment variable (`FEATURE_FLAGS_URL`). The configuration was added to the deployment manifest but not deployed to the Kubernetes cluster before the code deploy. The application startup failed without this variable.

## What Went Well

- **Health checks caught the issue before user traffic was routed**
- Riley recognized the issue immediately
- **Team knew to rollback quickly rather than debug in production**
- Rollback was fast and effective
- Post-deploy verification prevented user impact

## What Went Wrong

- **Deployment rollback procedure not documented in runbook**
  - Riley had to search documentation during incident
  - Lost ~2 minutes finding the correct `kubectl` command
- Environment variable change not coordinated with deploy
- No staging verification that new environment variable was applied
- Deployment checklist didn't include "verify all required env vars in cluster"

## Action Items

1. **[HIGH - IN PROGRESS]** Document deployment rollback procedure in runbook
   - Owner: Riley
   - Due: 2025-11-29 (this week, fresh from experience)
   - Status: Drafting (linked to retrospective sprint 3 action item)
   - Include:
     - Rollback command: `kubectl rollout undo deployment/<service>`
     - Health check verification steps
     - When to rollback vs. debug (decision criteria)
     - Notification procedure (who to tell)
     - Post-rollback analysis steps

2. **[HIGH]** Add environment variable verification to deployment checklist
   - Owner: Morgan
   - Due: 2025-12-05
   - Check: Compare required env vars in code vs. cluster config before deploy

3. **[MEDIUM]** Improve staging deployment validation
   - Owner: Sam
   - Due: 2025-12-15
   - Add: Automated check that staging deploy succeeds before prod deploy

4. **[LOW]** Create deployment pre-flight checklist
   - Owner: Alex
   - Due: 2025-12-20
   - Include: Env vars, migrations, feature flags, dependencies

## Lessons Learned

- **Runbooks need operational procedures, not just architectural info:**
  - The "how to rollback" is as important as "what is our deployment architecture"
  - Must include commands, decision criteria, not just concepts

- **Post-deploy verification is critical:**
  - Health checks prevented this from becoming a user-facing incident
  - We should celebrate catching this before impact

- **Fresh documentation is best documentation:**
  - Riley documenting this immediately after the incident ensures accuracy
  - Waiting weeks means forgetting important details

- **Rollback is a valid resolution:**
  - Team made the right call to rollback quickly vs. debugging in prod
  - Clear decision criteria helps: "Can we fix in <5min? No → rollback"

## Follow-Up

- [x] Rollback procedure documented (Riley, 2025-11-28)
- [ ] Verify rollback procedure in next deployment
- [ ] Add deployment checklist to team handbook
- [ ] Include deployment practices in onboarding for Casey (new team member)
- [ ] Review other operational procedures that might be missing from runbooks

---

**Related:**
- Sprint Retrospective 2025-Q1-Sprint-3 (action item #1)
- On-call Runbook (deployment section to be updated)
- Working Agreement: Definition of Done (deployment verification)
