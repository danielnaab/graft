---
status: working
---

# Living Specifications Principles

Normative principles governing how specifications are written, maintained, and used.

1. **Specs live with the code** — Co-located in the repository. Version-controlled, PR-reviewable, AI-readable.

2. **Source-of-truth requires enforcement** — Without mechanism (executable checks, CI, or cultural Definition of Done), "source of truth" is wishful thinking.

3. **Plain language over formal notation** — For team consensus, accessibility matters more than precision. Structured natural language with examples.

4. **Explicit decision states** — Every spec has clear status: draft → working → stable. Async collaboration requires knowing "can I build against this?"

5. **Low ceremony, high signal** — Updating specs must be lighter than the cost of stale docs. If updating is harder than coding, people won't do it.

6. **AI-compatible is human-compatible** — Unambiguous, complete, well-structured specs serve both audiences. Design for agent comprehension and humans benefit too.

7. **Separate what from why from how** — Specs describe behavior (what), decision records capture rationale (why), code implements (how). Distinct concerns, distinct artifacts.

8. **Async-first, synchronous-to-resolve** — Default to async proposals with clear timeboxes. Escalate to ensemble when not converging. Record outcomes either way.

9. **Tests and specs are complementary** — Tests verify behavior (executable proof); specs explain intent (readable context). Both needed, linked by backlinks.

10. **Evolution over prescription** — Specs should be easy to change. The process encourages refinement, not punishes change.

11. **Open Questions are first-class** — Explicitly marking uncertainty invites collaboration without blocking progress.

12. **Non-goals are as valuable as goals** — Explicitly scoping out prevents scope creep and clarifies boundaries.

13. **References flow upstream, views are computed** — Implementation references specs (backlinks). Tooling aggregates views. Forward links are a maintenance burden.

14. **Fight tribal knowledge** — If it's not in the repo, it doesn't exist. Make implicit conventions explicit.

15. **Specs fill the gap code can't** — Intent, constraints, alternatives, rationale. Don't re-describe what readable code already shows.

16. **Dogfood the practices** — Run your own tools on your own repo. Specs verified against their own implementations catch drift that tests miss. The KB's structural integrity tools enforce the KB's own rules.

## Sources

- Personal experience: Agile team collaboration, spec-driven delivery practices
- Observation: AI-assisted development workflow patterns (2025-2026)
