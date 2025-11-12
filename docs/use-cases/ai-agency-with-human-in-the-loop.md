# Auditable AI Agency with Human-in-the-Loop Primitives

## The Narrative

It's 2025. Your organization wants AI agents maintaining documentation, updating runbooks, generating reports. The agents are capable—Claude can write well, understand context, make reasonable decisions. But leadership has questions: "How do we know what the AI wrote versus what humans approved?" "Can we audit what the agent accessed?" "What if it makes a mistake?"

Traditional approaches leave you building custom tracking systems, logging every API call, hoping nothing slips through. Or you treat AI output as untrustworthy, requiring humans to rewrite everything—defeating the purpose of automation.

There's a gap between "let the AI do it" (risky) and "don't use AI" (leaving value on the table). You need infrastructure that makes AI agency trustworthy.

## The Pattern Graft Enables

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

## What's Novel

**AI agents with provenance** — Not just "the AI wrote this," but verifiable records of what inputs it accessed, what transformation it applied, what it produced.

**Policy-enforced review boundaries** — Granular control: some grafts auto-merge, others require human review. Not a binary global choice.

**Attribution as primitive** — Agent identity (model version, role) is captured in provenance, not external logs.

**Human oversight without micromanagement** — Review when it matters, trust when it's safe, always verify.

## The Wow

"This is the infrastructure layer that makes AI agents production-ready."

You can deploy an AI agent to maintain your knowledge base with confidence. It's not a black box—every change is attributed, every input is tracked, every decision is auditable. Leadership can trust it. Compliance can verify it. Teams can review it.

---

See also:
- [Core Concepts](../concepts.md) for provenance and attestation mechanics
- [CLI Reference](../cli-reference.md) for agent integration patterns
