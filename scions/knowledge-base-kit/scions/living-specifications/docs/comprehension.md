---
status: working
---

# System Comprehension

How to make the specification system legible to all collaborators—the UX of the SDLC.

## The Meta-Problem

The spec system itself must be legible. Different collaborators need different entry points, different depths, and different representations.

## UX Principles for SDLC

1. **Progressive disclosure** — Don't front-load. Level 0: "we use specs." Level 3: "here's drift detection."
2. **Wayfinding** — Navigable from any starting point (code → spec → tests, all via backlinks and indexes)
3. **Self-describing artifacts** — Each file carries enough context to be understood alone
4. **Consistency** — Every spec looks the same. One learning curve for all.
5. **Contextual orientation** — Entry points orient you (README, CLAUDE.md, frontmatter)
6. **Computed views** — Structured data enables audience-specific representations
7. **Playbooks for tasks** — "How do I write a spec?" vs. "What does the system do?"

## Entrypoint Architecture

A typical project using this methodology:

```
README.md                      → "What is this project?"
├── CLAUDE.md                  → "What are the rules?" (agents)
├── docs/                      → "What are the practices?"
│   └── specs/                 → "What does each feature do?"
│       ├── format.md          → "What goes in a spec?"
│       ├── [feature].md       → Actual specifications
│       └── ...
├── src/                       → "Implementations"
└── tests/                     → "Verification"
```

## Persona → Entry Point

| Persona | Starts at | Then navigates to |
|---------|-----------|-------------------|
| New team member | README | Playbooks |
| AI agent | CLAUDE.md | Relevant spec |
| Developer mid-flow | Backlink in code | Spec behavior section |
| Ensemble participant | Playbook | Spec being implemented |
| Stakeholder | Specs index | Status + Open Questions |
| Async reviewer | PR description | Referenced spec |

## Knowledge Types

The structure signals what kind of knowledge each file contains:

| Location | Knowledge Type | Question it answers |
|----------|---------------|-------------------|
| Reference docs | Declarative | "Understand this" — concepts, models |
| Principles | Normative | "Follow this rule" — constraints |
| Guides | Procedural | "Do this" — step-by-step guidance |

## Key Insight

**What helps AI agents helps humans.** Both cold-start into unfamiliar systems. Both need orientation, rules, and navigation. Design for agent comprehension and human comprehension follows.

## Related

- [Principles](principles.md) — #6 (AI-compatible is human-compatible), #14 (fight tribal knowledge)

## Sources

- Personal experience: Developer onboarding patterns, AI agent cold-start behavior
