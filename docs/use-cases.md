# Use Cases

This document explores real-world scenarios where Graft solves hard problems in new ways. Each use case shows a narrative followed by the pattern Graft enables.

## Auditable AI Agency with Human-in-the-Loop Primitives

### The Narrative

It's 2025. Your organization wants AI agents maintaining documentation, updating runbooks, generating reports. The agents are capable—Claude can write well, understand context, make reasonable decisions. But leadership has questions: "How do we know what the AI wrote versus what humans approved?" "Can we audit what the agent accessed?" "What if it makes a mistake?"

Traditional approaches leave you building custom tracking systems, logging every API call, hoping nothing slips through. Or you treat AI output as untrustworthy, requiring humans to rewrite everything—defeating the purpose of automation.

There's a gap between "let the AI do it" (risky) and "don't use AI" (leaving value on the table). You need infrastructure that makes AI agency trustworthy.

### The Pattern Graft Enables

AI agents become first-class actors in your workflow with full provenance and policy enforcement:

An AI agent working on team documentation calls `graft explain team-handbook/ --json` to understand what changed. It receives:
- Current configuration
- What materials changed and why (git diffs)
- Evaluated template showing the context
- Policy constraints it must follow

The agent makes surgical edits to the documentation. When done, it finalizes: `graft finalize team-handbook/ --agent "claude-sonnet-4" --role agent`.

Provenance captures:
- Exactly what materials the agent accessed (hashes, git refs)
- What template was evaluated
- What the agent changed (git diff in the commit)
- When it happened
- Under what policy

If the artifact policy requires human review, the agent's finalize triggers a PR. Human reviewers see:
- What changed and why
- Full attribution (AI agent, specific model)
- Complete audit trail
- Ability to approve, adjust, or reject

For low-stakes artifacts, policy can allow auto-merge. For sensitive ones, human approval is required. The infrastructure enforces the boundary.

### What's Novel

**AI agents with provenance** — Not just "the AI wrote this," but verifiable records of what inputs it accessed, what transformation it applied, what it produced.

**Policy-enforced review boundaries** — Granular control: some grafts auto-merge, others require human review. Not a binary global choice.

**Attribution as primitive** — Agent identity (model version, role) is captured in provenance, not external logs.

**Human oversight without micromanagement** — Review when it matters, trust when it's safe, always verify.

### The Wow

"This is the infrastructure layer that makes AI agents production-ready."

You can deploy an AI agent to maintain your knowledge base with confidence. It's not a black box—every change is attributed, every input is tracked, every decision is auditable. Leadership can trust it. Compliance can verify it. Teams can review it.

---

## Composable Intelligence Workflows Across Organizational Boundaries

### The Narrative

A cybersecurity firm has built sophisticated threat intelligence pipelines: ingesting CVE feeds, OSINT sources, vendor advisories, producing normalized threat databases. Other organizations want this intelligence, but with their own extensions—industry-specific threats, their infrastructure context, their risk scoring models.

Traditional approach: download their data dumps, build your own pipelines, maintain them yourself. Or: pay for their hosted SaaS, lose control and customization.

Neither option enables the pattern you need: **composable workflows where you build on others' work without forking**.

### The Pattern Graft Enables

The security firm publishes their normalization grafts as a public git repository. Tagged releases, semantic versioning. Policy: deterministic transformers, auto-merge enabled. This is their data workflow SDK.

Your organization references their grafts as materials:

```yaml
inputs:
  materials:
    # Upstream: ThreatCorp's normalized CVE feed
    - path: "https://github.com/threatcorp/intel/raw/v2.3.0/normalized/cves.json"
      rev: v2.3.0
```

You add your own transformation layer:

```yaml
derivations:
  - id: internal-risk-assessment
    transformer:
      build: { image: "our-risk-model:local" }
    inputs:
      materials:
        - path: "https://github.com/threatcorp/intel/raw/v2.3.0/normalized/cves.json"
          rev: v2.3.0
        - path: "../../internal/infrastructure-inventory.yaml"
    outputs:
      - { path: "./risk-report.md" }
    policy:
      attest: required
```

Your transformer combines: upstream threat data + your infrastructure context + your risk model. Your security team reviews and finalizes with attestation.

When ThreatCorp releases v2.4.0 (improved normalization logic), you can:
- Update the material ref to `v2.3.0` → `v2.4.0`
- Re-run your transformer (it gets the new upstream data)
- Review the changes (what's different in risk assessment?)
- Finalize with approval

Provenance shows: "Our February risk assessment used ThreatCorp intel v2.3.0, our infrastructure snapshot from 2025-02-01, finalized by Jane (Security Analyst) on 2025-02-15."

If you discover an improvement to the risk model, you can contribute back to ThreatCorp. If they publish new data sources, you pull them. Workflow supply chain.

### What's Novel

**Workflow composition via git** — Not just data sharing, but transformation logic as versionable, referenceable units.

**Extend, don't fork** — Build on upstream workflows, track exact versions, upgrade when ready.

**Provenance across boundaries** — Your audit trail includes external dependencies with exact versions.

**Supply chain for intelligence** — Same patterns as code dependencies (semver, pinning, security advisories) applied to data workflows.

### The Wow

"This is how organizations share and compose data workflows like they share code libraries."

You're not just consuming data—you're composing workflows. Upstream provides the foundation, you add your layers, full provenance connects it all. When they improve, you benefit. When you innovate, you can share back.

---

## Agile Team Operations with Living Organizational Memory

### The Narrative

Every agile team produces the same artifacts: sprint briefs, retrospectives, working agreements, runbooks, roadmaps, incident post-mortems. These should inform each other—retrospectives should improve processes, incidents should update runbooks, roadmap shifts should flow to sprint planning—but they don't.

Instead: Wiki pages go stale. Decisions get lost in Slack. Someone creates a "team handbook" that's out of date within a month. New team members ask "why do we do it this way?" and nobody remembers the retrospective from six months ago that decided it.

The problem isn't the artifacts themselves. It's that **they're disconnected**. There's no dependency graph connecting "this retrospective identified this problem" to "this working agreement was updated" to "this runbook now reflects the new process."

### The Pattern Graft Enables

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

### What's Novel

**Organizational memory as a DAG** — Knowledge flows through explicit dependencies, not tribal knowledge.

**PR-based consensus** — Changes to working agreements, processes, policies happen in reviewable PRs with team discussion.

**Living history** — Git history + provenance answers "why do we do this?" with receipts: this retro, this decision, this person, this date.

**Policy matches culture** — Some things need consensus, some need authority, some need velocity. Graft lets you encode that.

**Onboarding as graph traversal** — New team member asks about a process? Follow the dependency graph backward: this runbook came from that incident, which prompted this working agreement change, which the team discussed in this retrospective PR.

### The Wow

"Your team's living memory. Decisions, context, evolution—all connected, all auditable."

When someone asks "why do we do standups async?" you don't say "I think we decided that last year?" You show them:
- Retrospective from 2024-08 (time zones made sync standups painful)
- Working agreement PR where team discussed it
- Final decision (4 team members approved)
- Runbook that documents the process
- All linked through the dependency graph

The team's organizational structure maps to the knowledge graph. Knowledge flows where it should, gets reviewed by who it should, never gets lost.

---

## Cross-Cutting Insights

These use cases reveal common patterns:

**Trust boundaries via policy** — Not "automate everything" or "manual everything," but granular control matching your risk tolerance.

**Provenance as currency** — Attribution, audit trails, reproducibility aren't afterthoughts—they're first-class primitives.

**Composition across boundaries** — Git-native references enable workflow supply chains: publish, extend, version, share.

**Human and agent parity** — Both are first-class actors with attribution. The infrastructure doesn't care if Jane or Claude made the edit—policy governs both.

**Git as ledger** — Every change flows through version control. PRs enable review. History answers "why?"

## Getting Started with These Patterns

**Start small** — Pick one artifact type (sprint briefs, runbooks, reports). Prove the workflow.

**Add dependencies gradually** — Start with simple material dependencies. Add artifact-to-artifact dependencies as you see value.

**Match policy to culture** — What requires consensus? What needs authority? What can move fast? Encode that in policy.

**Make provenance visible** — Show the team what's captured. Build trust in the audit trail.

**Compose when ready** — Once you have working grafts, explore remote references and workflow composition.

The power of Graft isn't in any single feature—it's in how **file-first workflows + provenance + policy + composition** unlock new patterns for how organizations create, review, and trust their artifacts.

---

Next: See [Workflows](workflows.md) for concrete patterns, or [CLI Reference](cli-reference.md) to start building.
