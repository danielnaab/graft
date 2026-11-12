---
status: stable
---
# Temporal layers policy

Knowledge bases often need three temporal layers: ephemeral, tracking, and durable. Raw sources and
historical records are optional layers for knowledge bases that keep captures or a dated record.

| Layer | Example folders | Purpose | Lifecycle | Retention | Change when |
| --- | --- | --- | --- | --- | --- |
| Ephemeral | `notes/`, `explorations/` | Scratch work, session logs | draft → archived | Days to weeks | Archive when insights are extracted or the session ends |
| Tracking | `status/`, `current-state/` | Progress, handoff | working → deprecated → archived | Weeks to months | Archive when the milestone or work is done |
| Durable | `docs/`, `decisions/` | Architecture, guides, decisions | draft → working → stable → deprecated | Indefinite | Update when the system changes significantly |
| Raw sources | `raw/` | Material as captured (transcripts, exports) that other layers cite | None; the path marks it | Indefinite, or per the source's terms | Never edited; replace with a better capture or remove |
| Historical | `records/` | Dated records written once (daily notes, session logs) | None; the dated path marks it | Indefinite | Correct errors with a dated correction, not a rewrite |

## Living documents

Some documents are always current (continue-here.md, tasks.md):
- Mark with `status: living` in frontmatter
- Snapshot before major milestones
- Latest version is always current

## Migration paths

- Ephemeral → Durable: extract insights to docs
- Raw or Historical → Durable: synthesize into docs that cite them
- Tracking → Archived: snapshot when work completes
- Living → Snapshot: preserve state at milestones

Related:
- [Lifecycle policy](lifecycle.md)
- [Authority policy](authority.md)
