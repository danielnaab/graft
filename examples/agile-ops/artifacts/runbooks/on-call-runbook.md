# On-Call Runbook

**Last Updated:** 2025-11-29
**Team:** Platform Engineering
**Status:** Living document (updated via Graft workflow after incidents)

This runbook provides operational procedures for on-call engineers. It's updated continuously based on incidents and team working agreements.

---

## Table of Contents

1. [On-Call Basics](#on-call-basics)
2. [Escalation Contacts](#escalation-contacts)
3. [Common Incidents](#common-incidents)
4. [Deployment Procedures](#deployment-procedures)
5. [Monitoring and Alerts](#monitoring-and-alerts)
6. [Post-Incident Procedures](#post-incident-procedures)

---

## On-Call Basics

### Rotation Schedule

**Schedule:** Weekly rotation (Monday 9am - Monday 9am local time)

**Handoff Procedure:**
1. Schedule 15-minute sync meeting with incoming on-call
2. Review:
   - Current issues or ongoing investigations
   - Pending follow-ups or maintenance windows
   - Recent changes or deployments
   - Any unusual patterns in monitoring
3. Share access to on-call playbook and incident channel
4. Confirm PagerDuty alerts are routing correctly

**Source:** Working Agreement (effective 2025-11-08)

### Response Expectations

**Response Times:**
- **P0 (Critical):** Acknowledge within 5 minutes, initial assessment within 15 minutes
- **P1 (High):** Acknowledge within 15 minutes, initial assessment within 30 minutes
- **P2 (Medium):** Acknowledge within 1 hour, investigate during business hours

**Communication:**
- Acknowledge alerts promptly (even if investigation ongoing)
- Post updates to incident channel every 15-30 minutes for P0/P1
- Declare incidents early if severity unclear—better to over-communicate

---

## Escalation Contacts

**Last Updated:** 2025-11-05 (after database timeout incident)

### Team Contacts

| Role | Name | Slack | PagerDuty | Notes |
|------|------|-------|-----------|-------|
| Engineering Lead | Alex Chen | @alex | @alex-oncall | Escalate for major incidents |
| Database Expert | Jordan Lee | @jordan | @jordan-oncall | Database performance, migrations |
| Infra Lead | Sam Rodriguez | @sam | @infra-oncall | Kubernetes, networking, infra |
| Security Lead | Morgan Kim | @morgan | @security-oncall | Security incidents, auth issues |

### External Escalations

| Service | Contact | When to Escalate |
|---------|---------|------------------|
| Infrastructure Team | #infra-oncall (Slack) | Kubernetes issues, networking, cloud resources |
| Database Team | #database-oncall (Slack) | Query performance, connection issues, migrations |
| Security Team | PagerDuty: @security-oncall | Auth failures, potential breaches, CVEs |
| Product Team | #product-oncall (Slack) | Customer-impacting issues requiring product decisions |

### Escalation Decision Tree

```
Incident detected
   ├─ Can I resolve in <15 min with runbook? → Proceed, update incident channel
   ├─ Unclear severity or scope? → Escalate to Engineering Lead
   ├─ Database-related? → Escalate to Database Team
   ├─ Infrastructure/K8s? → Escalate to Infrastructure Team
   ├─ Security concern? → Escalate to Security Team immediately
   └─ Customer impact unknown? → Escalate to Product Team for assessment
```

**Source:** Action item from database timeout incident (2025-11-03)

---

## Common Incidents

### Database Connection Timeouts

**Symptoms:**
- 504 Gateway Timeout errors
- `/health` endpoint failing
- Application logs: "connection pool exhausted"

**Investigation:**
1. Check database connection pool status:
   ```bash
   kubectl exec -it deployment/api-server -- psql -c "SELECT count(*) FROM pg_stat_activity;"
   ```
2. Identify slow queries:
   ```bash
   kubectl exec -it deployment/api-server -- psql -c "SELECT * FROM pg_stat_activity WHERE state = 'active' AND query_start < now() - interval '30 seconds';"
   ```
3. Check for missing indexes on recent migrations

**Resolution:**
- **Quick fix:** Restart application pods to reset connection pool
  ```bash
  kubectl rollout restart deployment/api-server
  ```
- **Root cause:** Identify slow query, add index or optimize query
- **Escalation:** If query performance issue unclear, escalate to Database Team

**Rollback:** If recent deploy introduced the issue, see [Deployment Rollback](#deployment-rollback)

**Post-Incident:** Document findings, update query performance tests

**Source:** Incident 2025-11-03-database-timeout

---

### Failed Deployments

**Symptoms:**
- Health checks failing after deploy
- New pods not reaching Ready state
- Deployment dashboard shows red status

**Investigation:**
1. Check pod status:
   ```bash
   kubectl get pods -l app=api-server
   ```
2. Check pod logs for startup errors:
   ```bash
   kubectl logs deployment/api-server --tail=50
   ```
3. Check recent changes: environment variables, config maps, secrets

**Common Causes:**
- Missing environment variables
- Incorrect configuration
- Container image build failure
- Database migration failure

**Decision Criteria:**
- Can I fix in <5 minutes? → Attempt quick fix (env var, config)
- Fix unclear or risky? → **Rollback immediately**
- Customer traffic already affected? → **Rollback immediately**

**Source:** Incident 2025-11-27-deployment-rollback

---

## Deployment Procedures

### Deployment Rollback

**When to Rollback:**
- Health checks failing after deploy
- Elevated error rates (>5% increase)
- Customer reports of issues correlated with deploy timing
- Unclear root cause and fix will take >5 minutes

**Rollback Procedure:**

1. **Declare incident** in #incidents Slack channel:
   ```
   🚨 Deployment rollback in progress for api-server
   Reason: [health checks failing / elevated errors / etc]
   ```

2. **Execute rollback:**
   ```bash
   # Kubernetes deployment
   kubectl rollout undo deployment/api-server

   # Verify rollback in progress
   kubectl rollout status deployment/api-server
   ```

3. **Verify health checks:**
   ```bash
   # Check pod health
   kubectl get pods -l app=api-server

   # Test health endpoint
   curl https://api.example.com/health
   ```

4. **Monitor error rates:**
   - Open Grafana dashboard
   - Verify error rate returns to baseline within 5 minutes
   - Check user-facing metrics (request latency, success rate)

5. **Notify stakeholders:**
   ```
   ✅ Rollback complete. Service restored to v2.17.3.
   Health checks passing. Error rate back to baseline.
   Post-mortem to follow.
   ```

6. **Post-Rollback:**
   - Investigate root cause in rolled-back version
   - Fix issue in new PR
   - Re-deploy with fix + verification

**Commands Reference:**
```bash
# Rollback to previous version
kubectl rollout undo deployment/<service>

# Rollback to specific revision
kubectl rollout undo deployment/<service> --to-revision=3

# Check rollout history
kubectl rollout history deployment/<service>

# Monitor rollout progress
kubectl rollout status deployment/<service>
```

**Post-Deploy Verification Checklist:**
- [ ] Health checks pass on new pods
- [ ] Error rate <1% in Grafana
- [ ] Response time p95 <500ms
- [ ] No elevated alerts in PagerDuty
- [ ] Smoke tests pass in production

**Source:** Action item from deployment rollback incident (2025-11-27)

---

### Deployment Health Checks

**Pre-Deploy:**
1. Verify staging deployment succeeded
2. Run smoke tests against staging
3. Check no ongoing incidents
4. Verify required environment variables in cluster config

**Post-Deploy:**
1. Watch pod rollout: `kubectl rollout status deployment/<service>`
2. Check health endpoint: `curl https://api.example.com/health`
3. Monitor Grafana: Error rate, latency, throughput
4. Wait 5 minutes, verify no elevated alerts
5. Check Sentry for new error patterns

**If any check fails:** Follow [Deployment Rollback](#deployment-rollback) procedure

**Source:** Action items from incidents 2025-11-03, 2025-11-27

---

## Monitoring and Alerts

### Alert Response

**When Alert Fires:**
1. Acknowledge in PagerDuty (stop the noise)
2. Check incident channel for ongoing issues
3. Investigate based on alert type (see sections below)
4. Post updates to incident channel
5. Escalate if needed (see [Escalation Contacts](#escalation-contacts))

### Key Dashboards

- **Production Overview:** `https://grafana.example.com/d/prod-overview`
- **API Performance:** `https://grafana.example.com/d/api-perf`
- **Database Health:** `https://grafana.example.com/d/database`
- **Kubernetes Cluster:** `https://grafana.example.com/d/k8s-cluster`

### Common Alerts

| Alert | Meaning | First Steps |
|-------|---------|-------------|
| HighErrorRate | >5% API errors | Check recent deploys, review error logs |
| DatabaseConnectionPoolExhausted | DB connections maxed | Check slow queries, restart pods if needed |
| PodCrashLooping | Pod repeatedly failing | Check logs: `kubectl logs <pod>` |
| HighMemoryUsage | >85% memory | Check for memory leaks, scale up if needed |
| DiskSpaceLow | <15% disk free | Clean up logs, scale storage |

---

## Post-Incident Procedures

### Incident Documentation

1. **During Incident:** Take notes on timeline, actions taken
2. **After Resolution:** Write post-mortem (template: `sources/incidents/TEMPLATE.md`)
3. **Post-Mortem Includes:**
   - Summary (what happened, impact, duration)
   - Timeline (key events, actions, decisions)
   - Root cause
   - What went well / what went wrong
   - Action items with owners and due dates
   - Lessons learned

4. **Share:** Post post-mortem in incident channel, review in next retrospective

### Runbook Updates

**After every incident:**
1. Identify what was missing or unclear in runbook
2. Run: `graft run artifacts/runbooks/` (generates guidance from incident)
3. Update runbook with new procedures, contacts, or commands
4. Finalize: `graft finalize artifacts/runbooks/ --agent "Your Name"`
5. Commit and push (runbook updates are high priority)

**Why update immediately?**
- Details are fresh (you'll forget in a week)
- Next on-call engineer benefits from your learning
- Prevents same issue from repeating

---

## How This Runbook is Maintained

This runbook is a **graft artifact** that depends on:
- **Incidents:** Lessons learned flow into runbook updates
- **Working Agreements:** On-call procedures from team decisions

**Update Workflow:**
1. After incident, write post-mortem in `sources/incidents/`
2. Runbook becomes "dirty" (depends on incidents)
3. Run: `graft run artifacts/runbooks/` (see guidance template)
4. Update runbook sections based on incident learnings
5. Finalize: `graft finalize artifacts/runbooks/ --agent "Your Name"`
6. Commit with message linking to incident

**Why this workflow?**
- Incidents drive runbook improvements (provenance)
- Runbook evolution is traceable (git history)
- New team members see context (incident links)
- Runbook stays current (dependency tracking)

---

## Change History

| Date | Change | Source |
|------|--------|--------|
| 2025-11-05 | Added escalation contacts | Incident: database-timeout |
| 2025-11-05 | Documented on-call handoff procedure | Working Agreement Sprint 1 |
| 2025-11-29 | Added deployment rollback procedure | Incident: deployment-rollback |
| 2025-11-29 | Added post-deploy verification checklist | Incident: deployment-rollback |

---

**Dependencies:**
- Working Agreements: `artifacts/working-agreements/team-handbook.md`
- Incidents: `sources/incidents/2025-*.md`
