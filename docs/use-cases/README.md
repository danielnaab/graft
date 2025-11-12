# Use Cases

This directory explores real-world scenarios where Graft solves hard problems in new ways. Each use case shows a narrative followed by the pattern Graft enables.

## The Three Core Patterns

### [Auditable AI Agency with Human-in-the-Loop Primitives](./ai-agency-with-human-in-the-loop.md)

AI agents become first-class actors with full provenance and policy enforcement. Not just "the AI wrote this," but verifiable records of inputs, transformations, and outputs. Policy-enforced review boundaries enable granular control: some artifacts auto-merge, others require human approval.

**Key insight:** Infrastructure that makes AI agency trustworthy through attribution, audit trails, and human oversight primitives.

### [Composable Intelligence Workflows Across Organizational Boundaries](./composable-intelligence-workflows.md)

Organizations publish data workflows as versionable, referenceable git artifacts. Others extend them without forking. Upstream improvements flow downstream. Provenance tracks exact versions across organizational boundaries.

**Key insight:** Workflow supply chains—compose workflows like code libraries with semantic versioning and dependency tracking.

### [Agile Team Operations with Living Organizational Memory](./agile-team-operations.md)

Team artifacts (retrospectives, working agreements, runbooks, sprint briefs) form an explicit dependency graph. When source materials change, dependent artifacts become dirty. Knowledge flows through the graph, not tribal memory. Git history + provenance answers "why do we do this?"

**Key insight:** Organizational memory as a DAG where decisions are traceable, reviewable, and never lost.

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

Next: See [Workflows](../workflows.md) for concrete patterns, or [CLI Reference](../cli-reference.md) to start building.
