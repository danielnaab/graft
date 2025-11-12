# Sprint Brief - Sprint 3 (Nov 29 - Dec 12)

**Sprint:** 3 of Q1 2025
**Dates:** November 29 - December 12, 2025
**PM:** Alex Chen
**Team:** Platform Engineering (Alex, Jordan, Sam, Riley, Morgan, Casey)

---

## Sprint Goals

**Primary Goal:** Deliver REST API v2 core endpoints and continue performance improvements

**Supporting Goals:**
1. Complete session management security improvements
2. Continue frontend performance work (code splitting)
3. Codify Definition of Done and technical debt process
4. Update runbooks with deployment procedures

**Alignment with Roadmap:**
- Supports Q1 Theme 1: Authentication & Security (session management)
- Supports Q1 Theme 2: Performance & Scalability (frontend optimizations)
- Supports Q1 Theme 3: API Platform (REST API v2 implementation)
- Supports Q1 Theme 4: Developer Experience (documentation, runbooks)

---

## Committed Work (35 points)

### REST API v2 Implementation (15 points)
**Stories:**
- [PLAT-145] Implement Users resource endpoints (GET, POST, PUT, DELETE) - 8pts
- [PLAT-146] Implement Projects resource endpoints - 5pts
- [PLAT-147] Add API versioning middleware - 2pts

**Acceptance Criteria:**
- RESTful endpoints following API v2 design
- Request/response validation
- OpenAPI spec updated
- Integration tests pass

**Owner:** Jordan (lead), Morgan (support)

### Session Management Improvements (8 points)
**Stories:**
- [PLAT-148] Implement session timeout with activity tracking - 5pts
- [PLAT-149] Add audit logging for auth events - 3pts

**Acceptance Criteria:**
- Sessions expire after 30min inactivity
- All auth events logged with context
- Security team review completed

**Owner:** Sam

### Frontend Performance (5 points)
**Stories:**
- [PLAT-150] Implement code splitting for dashboard routes - 3pts
- [PLAT-151] Add lazy loading for heavy components - 2pts

**Acceptance Criteria:**
- Initial bundle size reduced by 30%
- Time to Interactive <2s (p95)
- Lighthouse score >90

**Owner:** Riley

### Documentation & Process (7 points)
**Stories:**
- [PLAT-152] Document Definition of Done in working agreements - 2pts
- [PLAT-153] Update on-call runbook with deployment procedures - 2pts
- [PLAT-154] Technical debt backlog grooming - 3pts

**Acceptance Criteria:**
- Definition of Done codified and team-reviewed
- Runbook includes rollback commands and decision criteria
- Tech debt stories prioritized and estimated

**Owner:** Casey (DoD), Riley (runbook), Alex (tech debt)

---

## Stretch Goals (Optional, 5 points)

- [PLAT-155] Begin API documentation portal design - 3pts (if time permits)
- [PLAT-156] Add pre-deploy smoke tests to CI - 2pts (nice to have)

---

## Dependencies

**External Dependencies:**
- None this sprint (Infra cluster upgrade happened Sprint 2)

**Internal Dependencies:**
- API v2 implementation blocks API documentation portal (stretch goal)
- Session management needs security team review (scheduled for Dec 5)

**Team Dependencies:**
- Casey onboarding continues (pairing with Morgan on API work)

---

## Risks

**Medium Risk:**
- **API v2 scope creep** - Many "nice to have" features requested
  - *Mitigation:* Strict MVP focus, defer v1 deprecation to Q2
- **Holiday scheduling** - Next sprint (Sprint 4) has reduced capacity
  - *Mitigation:* Front-load critical work this sprint

**Low Risk:**
- **Casey still onboarding** - May need more pairing time
  - *Mitigation:* Assigned to mix of well-defined and mentored work

---

## Capacity Planning

**Total Capacity:** 35 points
- Alex: 7 points (20% on tech debt grooming, planning)
- Jordan: 8 points (REST API lead)
- Sam: 7 points (session management)
- Riley: 8 points (frontend performance, runbook)
- Morgan: 5 points (API support, mentoring Casey)
- Casey: 5 points (first full sprint, ramping up)

**Subtract 5 points for meetings/reviews/buffer = 30 committed + 5 stretch**

---

## Success Criteria

**Sprint is successful if:**
1. ✅ REST API v2 core endpoints deployed to production
2. ✅ Session management security improvements complete and reviewed
3. ✅ Frontend performance improvements show measurable impact (TTI <2s)
4. ✅ Definition of Done codified and team-approved
5. ✅ Runbook updated with clear rollback procedures

**Metrics to Track:**
- Sprint velocity (target: 35±5 points)
- Deployment frequency (target: >10)
- PR review time (target: <24h)
- Incident rate (target: <2)
- Test coverage (maintain >85%)

---

## Retrospective Topics to Explore

- How is Casey's onboarding going? What can we improve?
- Is the 20% tech debt allocation working?
- Are our Definition of Done criteria realistic?
- API v2 design: Any lessons learned from implementation?

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2025-11-29 | Initial Sprint 3 brief created | Alex Chen |

---

## How This Brief is Maintained

This sprint brief is a **graft artifact** that depends on:
- **Product Roadmap:** Strategic direction and themes
- **Backlog:** Current work items and priorities

**Update Workflow:**
1. Product roadmap changes trigger sprint brief to become "dirty"
2. PM runs: `graft run artifacts/sprint-brief/` (see template guidance)
3. During sprint planning, team selects work and updates brief
4. PM finalizes: `graft finalize artifacts/sprint-brief/ --agent "Alex Chen"`

**Why this workflow?**
- Strategic shifts in roadmap automatically surface in planning
- Sprint goals trace back to quarterly themes (provenance)
- Historical briefs show team evolution (git history)

---

**Dependencies:**
- Roadmap: `sources/roadmap/2025-Q1.md`
- Backlog: `artifacts/backlog/backlog.yaml`
