# Agile Team Operations with Living Organizational Memory

## The Narrative

Every agile team produces the same artifacts: sprint briefs, retrospectives, working agreements, runbooks, roadmaps, incident post-mortems. These should inform each other—retrospectives should improve processes, incidents should update runbooks, roadmap shifts should flow to sprint planning—but they don't.

Instead: Wiki pages go stale. Decisions get lost in Slack. Someone creates a "team handbook" that's out of date within a month. New team members ask "why do we do it this way?" and nobody remembers the retrospective from six months ago that decided it.

The problem isn't the artifacts themselves. It's that **they're disconnected**. There's no dependency graph connecting "this retrospective identified this problem" to "this working agreement was updated" to "this runbook now reflects the new process."

## The Pattern Graft Enables

Team artifacts are grafts with explicit dependencies forming the team's knowledge graph:

**Retrospectives** are source materials (meeting notes, action items).

**Working agreements** depend on retrospectives:
```yaml
graft: working-agreements
inputs:
  materials:
    - path: "../../meetings/retros/2025-Q1/*.md"
```

When a retrospective identifies "we need clearer on-call expectations," the working agreement artifact becomes dirty. The team sees this in status, discusses the change in a PR, reaches consensus, finalizes with attribution.

**Runbooks** depend on working agreements and incident post-mortems:
```yaml
graft: on-call-runbook
inputs:
  materials:
    - path: "../working-agreements/on-call.md"
    - path: "../../incidents/2025-*.md"
```

When an incident post-mortem documents "we didn't know how to escalate to security," the runbook becomes dirty. The on-call engineer updates it, finalizes with their name, commits. The next person on-call has the improved runbook.

**Sprint briefs** depend on the roadmap and recent work:
```yaml
graft: sprint-brief
inputs:
  materials:
    - path: "../roadmap/current-quarter.md"
    - path: "../../sources/tickets/sprint-current.yaml"
```

When product updates the roadmap (strategic pivot), sprint planning automatically sees it. The brief template pulls new priorities, the team refines it in planning, finalizes with the PM's name.

**Policy varies by artifact type:**
- Sprint briefs: PM can finalize solo (velocity)
- Working agreements: require team PR review (consensus)
- Architecture decisions: require tech lead attestation (authority)
- Runbooks: any team member can finalize (shared ownership)

## What's Novel

**Organizational memory as a DAG** — Knowledge flows through explicit dependencies, not tribal knowledge.

**PR-based consensus** — Changes to working agreements, processes, policies happen in reviewable PRs with team discussion.

**Living history** — Git history + provenance answers "why do we do this?" with receipts: this retro, this decision, this person, this date.

**Policy matches culture** — Some things need consensus, some need authority, some need velocity. Graft lets you encode that.

**Onboarding as graph traversal** — New team member asks about a process? Follow the dependency graph backward: this runbook came from that incident, which prompted this working agreement change, which the team discussed in this retrospective PR.

## The Wow

"Your team's living memory. Decisions, context, evolution—all connected, all auditable."

When someone asks "why do we do standups async?" you don't say "I think we decided that last year?" You show them:
- Retrospective from 2024-08 (time zones made sync standups painful)
- Working agreement PR where team discussed it
- Final decision (4 team members approved)
- Runbook that documents the process
- All linked through the dependency graph

The team's organizational structure maps to the knowledge graph. Knowledge flows where it should, gets reviewed by who it should, never gets lost.

## Getting Started with This Pattern

**Start small** — Pick one artifact type (sprint briefs, runbooks, reports). Prove the workflow.

**Add dependencies gradually** — Start with simple material dependencies. Add artifact-to-artifact dependencies as you see value.

**Match policy to culture** — What requires consensus? What needs authority? What can move fast? Encode that in policy.

**Make provenance visible** — Show the team what's captured. Build trust in the audit trail.

The power of this pattern is in how **file-first workflows + provenance + policy** unlock living organizational memory where decisions never get lost.

---

See also:
- [Core Concepts](../concepts.md) for the domain model (artifacts, materials, derivations)
- [Workflows](../workflows.md) for concrete implementation patterns
- [Example: agile-ops](../../examples/agile-ops/) for a reference implementation
