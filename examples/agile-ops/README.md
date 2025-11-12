# Agile Team Operations Example

**Purpose:** Reference implementation of the [Agile Team Operations with Living Organizational Memory](../../docs/use-cases/agile-team-operations.md) pattern.

**Status:** Fully functional example demonstrating multi-level dependency graphs, manual + automated workflows, and living documentation.

---

## Conceptual Dependencies

This example implements patterns from:

### Primary Use Case
- **[Agile Team Operations](../../docs/use-cases/agile-team-operations.md)** - Organizational memory as a DAG, PR-based consensus, living history

### Core Concepts
- **[Artifacts](../../docs/concepts.md#artifacts)** - Multiple graft directories with dependencies
- **[Materials](../../docs/concepts.md#materials)** - Source documents (retrospectives, incidents, roadmap)
- **[Derivations](../../docs/concepts.md#derivations)** - Manual and template-guided transformations
- **[Policy](../../docs/concepts.md#policy)** - Different policies for different artifact types
- **[Provenance](../../docs/concepts.md#provenance-and-attestation)** - Attribution and audit trails

### Patterns Demonstrated
- **Multi-level dependency chains** - Changes cascade through the graph
- **Manual derivations with template guidance** - Human editing with context
- **Policy variation by artifact type** - Solo finalize vs. PR review required
- **Living documentation** - Docs that evolve with team decisions

---

## The Dependency Graph

This example demonstrates a realistic agile team's knowledge graph:

```
sources/retrospectives/*.md ──┐
                               ├──> artifacts/working-agreements/team-handbook.md ──┐
                               │                                                      │
sources/incidents/*.md ────────┼──────────────────────────────────────────────────────┼──> artifacts/runbooks/on-call-runbook.md
                               │                                                      │
                               │                                                      │
sources/roadmap/2025-Q1.md ────┼──────────────────────────────────────────────────────┘
                               │
                               └──> artifacts/sprint-brief/brief.md

artifacts/backlog/backlog.yaml ────> artifacts/sprint-brief/brief.md
```

**Key relationships:**

1. **Retrospectives → Working Agreements**
   - Retro decisions and action items flow into team handbook
   - New agreements codified based on team learning

2. **Working Agreements + Incidents → Runbooks**
   - Incidents identify gaps in operational procedures
   - Agreements define on-call rotation and escalation
   - Runbooks capture operational knowledge

3. **Roadmap + Backlog → Sprint Briefs**
   - Strategic themes from roadmap guide sprint planning
   - Backlog items selected align with roadmap goals
   - Sprint briefs trace goals back to strategy

---

## Directory Structure

```
agile-ops/
├── README.md                   # This file
│
├── sources/                    # Source materials (inputs)
│   ├── retrospectives/         # Sprint retro notes
│   │   ├── 2025-Q1-sprint-1.md
│   │   ├── 2025-Q1-sprint-2.md
│   │   └── 2025-Q1-sprint-3.md
│   ├── incidents/              # Post-mortem documents
│   │   ├── 2025-11-03-database-timeout.md
│   │   └── 2025-11-27-deployment-rollback.md
│   └── roadmap/                # Product roadmap
│       └── 2025-Q1.md
│
└── artifacts/                  # Graft artifacts (outputs)
    ├── working-agreements/     # Team handbook
    │   ├── graft.yaml
    │   ├── template.md
    │   └── team-handbook.md
    │
    ├── runbooks/               # Operational runbooks
    │   ├── graft.yaml
    │   ├── template.md
    │   └── on-call-runbook.md
    │
    ├── sprint-brief/           # Sprint planning briefs
    │   ├── graft.yaml
    │   ├── template.md
    │   └── brief.md
    │
    └── backlog/                # Normalized backlog (from JIRA)
        ├── graft.yaml
        ├── Dockerfile
        ├── transform.py
        └── backlog.yaml
```

---

## Workflows Demonstrated

### 1. Retrospective → Working Agreement

**Scenario:** Team holds retrospective, makes decisions about processes

**Workflow:**
1. **Capture:** Write retrospective notes in `sources/retrospectives/2025-Q1-sprint-N.md`
   - Include: Decisions made, action items, context

2. **Propagate:** Working agreements artifact becomes dirty
   ```bash
   graft status artifacts/working-agreements/
   # Output: dirty (material changed: sources/retrospectives/...)
   ```

3. **Update:** Run to get guidance
   ```bash
   graft run artifacts/working-agreements/
   # Generates .graft/evaluated/guidance.md with extracted decisions
   ```

4. **Refine:** Team member updates `team-handbook.md` with new agreements
   - Add new sections, update existing ones
   - Include rationale and links to retrospectives

5. **Review:** Open PR for team discussion (for significant changes)

6. **Finalize:** After consensus, finalize with attribution
   ```bash
   graft finalize artifacts/working-agreements/ --agent "Jordan Lee"
   ```

**Policy:** `attest: required`, `direct_edit: true` - Manual editing with attribution

---

### 2. Incident → Runbook Update

**Scenario:** Production incident occurs, lessons learned need to update runbook

**Workflow:**
1. **Document:** Write post-mortem in `sources/incidents/YYYY-MM-DD-name.md`
   - Include: Timeline, root cause, what went wrong, action items, lessons

2. **Propagate:** Runbook artifact becomes dirty
   ```bash
   graft status artifacts/runbooks/
   # Output: dirty (material changed: sources/incidents/...)
   ```

3. **Update:** Engineer who handled incident updates runbook
   ```bash
   graft run artifacts/runbooks/
   # Guidance shows incident lessons and action items
   ```

4. **Document:** Update `on-call-runbook.md` while details are fresh
   - Add missing escalation contacts
   - Document rollback commands
   - Add decision criteria

5. **Finalize:** Finalize immediately (shared ownership)
   ```bash
   graft finalize artifacts/runbooks/ --agent "Riley Kumar"
   ```

**Policy:** `attest: required`, `direct_edit: true` - Any team member can update

**Why this works:**
- Updates happen when knowledge is fresh (right after incident)
- Next on-call engineer benefits immediately
- Provenance traces runbook sections to specific incidents

---

### 3. Roadmap Update → Sprint Brief

**Scenario:** Product team updates roadmap, changes need to flow to sprint planning

**Workflow:**
1. **Update:** Product team updates `sources/roadmap/2025-Q1.md`
   - Strategic pivot, new priorities, adjusted timelines

2. **Propagate:** Sprint brief becomes dirty
   ```bash
   graft status artifacts/sprint-brief/
   # Output: dirty (material changed: sources/roadmap/...)
   ```

3. **Plan:** During sprint planning, PM runs template
   ```bash
   graft run artifacts/sprint-brief/
   # Guidance pulls roadmap themes and strategic goals
   ```

4. **Select:** Team selects work items that align with updated roadmap

5. **Brief:** PM updates `brief.md` with sprint commitments and goals

6. **Finalize:** PM finalizes (solo, for velocity)
   ```bash
   graft finalize artifacts/sprint-brief/ --agent "Alex Chen" --role human
   ```

**Policy:** `attest: required`, `direct_edit: true` - PM can finalize solo

**Why this works:**
- Strategic changes automatically surface in planning
- Sprint goals trace back to roadmap themes (provenance)
- Historical briefs show how strategy evolved

---

### 4. Backlog Normalization (Automated)

**Scenario:** JIRA data needs normalization for downstream use

**Workflow:**
1. **Snapshot:** CI snapshots JIRA data to `sources/external/jira/snapshots/`

2. **Transform:** Container transformer normalizes to YAML
   ```bash
   graft run artifacts/backlog/
   # Runs Docker container (transform.py) to normalize JIRA JSON → YAML
   ```

3. **Finalize:** CI finalizes automatically (deterministic)
   ```bash
   graft finalize artifacts/backlog/ --agent "ci-bot" --role ci
   ```

**Policy:** `deterministic: true`, `attest: required` - Automated with attribution

**Why this works:**
- Container isolation for reproducibility
- Downstream artifacts (sprint brief) consume normalized data
- Provenance captures exact JIRA snapshot used

---

## How to Use This Example

### Prerequisites
- Graft CLI installed
- Docker (for backlog transformation)
- Git repository

**Note:** This example uses the Graft repository's root-level `graft.config.yaml` for defaults. This makes it easier to iterate on the example as part of the repo. Commands can be run from either the repo root or from within the example directory.

### Explore the Example

**From repo root:**
```bash
# Check status of all artifacts
graft status examples/agile-ops/artifacts/*/

# Understand dependencies
graft explain examples/agile-ops/artifacts/working-agreements/ --json | jq '.materials'
graft explain examples/agile-ops/artifacts/runbooks/ --json | jq '.materials'

# See template guidance
graft run examples/agile-ops/artifacts/working-agreements/
cat examples/agile-ops/artifacts/working-agreements/.graft/evaluated/guidance.md
```

**From example directory:**
```bash
cd examples/agile-ops

# Check status
graft status artifacts/*/

# Understand dependencies
graft explain artifacts/working-agreements/ --json | jq '.materials'
graft explain artifacts/runbooks/ --json | jq '.materials'

# See template guidance
graft run artifacts/working-agreements/
cat artifacts/working-agreements/.graft/evaluated/guidance.md
```

### Simulate an Update

1. **Edit a source material:**
   ```bash
   # Add a decision to a retrospective
   echo "\n- **Decision:** Adopt continuous deployment" >> examples/agile-ops/sources/retrospectives/2025-Q1-sprint-3.md
   ```

2. **Check what becomes dirty:**
   ```bash
   graft status examples/agile-ops/artifacts/working-agreements/
   # Should show: dirty (material changed)
   ```

3. **Run to get guidance:**
   ```bash
   graft run examples/agile-ops/artifacts/working-agreements/
   cat examples/agile-ops/artifacts/working-agreements/.graft/evaluated/guidance.md
   ```

4. **Update the handbook** based on guidance (edit `artifacts/working-agreements/team-handbook.md`)

5. **Finalize with attribution:**
   ```bash
   graft finalize examples/agile-ops/artifacts/working-agreements/ --agent "Your Name"
   ```

### Experiment with Workflows

**From repo root, add a new retrospective:**
```bash
# Create new retro
cat > examples/agile-ops/sources/retrospectives/2025-Q1-sprint-4.md <<EOF
# Sprint Retrospective - 2025 Q1 Sprint 4
## Decisions Made
- **Decision:** Adopt daily deploy target
  - Rationale: Reduce batch size, improve flow
EOF

# See what becomes dirty (from example directory)
cd examples/agile-ops && graft status artifacts/working-agreements/

# Update working agreements
graft run artifacts/working-agreements/
# Edit team-handbook.md to include new decision
graft finalize artifacts/working-agreements/ --agent "Your Name"
```

**From repo root, add a new incident:**
```bash
# Create incident post-mortem
cat > examples/agile-ops/sources/incidents/2025-12-01-cache-invalidation.md <<EOF
# Incident: Cache Invalidation Bug
## What Went Wrong
- Cache invalidation logic had race condition
- No runbook for cache debugging
## Action Items
- [HIGH] Add cache debugging section to runbook
EOF

# Update runbook (from example directory)
cd examples/agile-ops
graft run artifacts/runbooks/
# Add cache debugging section to on-call-runbook.md
graft finalize artifacts/runbooks/ --agent "Your Name"
```

---

## Key Lessons from This Example

### 1. Organizational Memory as a DAG

**Instead of:** "Why do we do async standups?" → "I think we decided that last year?"

**With Graft:** Follow the dependency graph:
```bash
git log artifacts/working-agreements/team-handbook.md
# Shows commit: "Add async standup agreement"
# References: sources/retrospectives/2025-Q1-sprint-1.md
```

Open the retrospective, see the full discussion, decision, and rationale. **Context never lost.**

### 2. Fresh Documentation

**Instead of:** Incident happens → runbook outdated → next incident repeats mistakes

**With Graft:**
- Post-mortem written → runbook becomes dirty
- Engineer updates runbook while details fresh
- Next on-call has improved runbook
- Provenance links runbook sections to incidents

### 3. Policy Matches Culture

Different artifacts have different policies:

| Artifact | Policy | Rationale |
|----------|--------|-----------|
| Sprint Briefs | PM can finalize solo | Velocity - PM owns sprint planning |
| Working Agreements | PR review recommended | Consensus - team decisions need discussion |
| Runbooks | Any team member can finalize | Shared ownership - who's on-call updates |
| Backlog | Automated (CI) | Deterministic transformation |

### 4. Traceability

Every change traces back to its source:
- Runbook escalation section → Incident 2025-11-03
- Async standup agreement → Retro Sprint 1
- Sprint 3 goals → Q1 Roadmap Theme 2

**Git log + provenance = full audit trail**

---

## Extending This Example

**Add more artifact types:**
- Architecture Decision Records (ADRs) depending on retro architecture discussions
- Post-mortem summaries depending on incident patterns
- Team metrics dashboard depending on sprint briefs

**Add remote dependencies:**
- Reference external threat intelligence feeds
- Compose with shared runbook templates from other teams
- Pull roadmap from product team's repository

**Add automation:**
- CI bot updates artifacts on schedule
- Slack notifications when artifacts become dirty
- Auto-generate PR when agreements change

---

## Related Documentation

### Use Cases
- [Agile Team Operations](../../docs/use-cases/agile-team-operations.md) - The pattern this example implements

### Concepts
- [Core Concepts](../../docs/concepts.md) - Artifacts, materials, derivations, policy
- [Transformation Lifecycle](../../docs/concepts.md#the-transformation-lifecycle) - Run → finalize workflow

### Workflows
- [Workflows](../../docs/workflows.md) - Additional patterns and best practices

### ADRs
- [ADR 0005](../../docs/adr/0005-abstractions-define-implementations-not-vice-versa.md) - Why examples declare conceptual dependencies

---

## Questions This Example Answers

**"How do decisions flow through a team?"**
→ Retrospectives → Working Agreements → Runbooks (follow the DAG)

**"Who decided to do async standups and why?"**
→ `git log team-handbook.md` → Retro Sprint 1 → timezone challenges

**"Why does the runbook have these escalation contacts?"**
→ Incident database-timeout → identified missing contacts → updated

**"How do sprint goals align with roadmap?"**
→ Sprint brief depends on roadmap → goals trace to themes → provenance

**"What if I want to change a working agreement?"**
→ Edit source retrospective or propose in new retro → PR for discussion → finalize with team

---

This example demonstrates that **organizational memory can be explicit, traceable, and living** - not locked in wiki pages or tribal knowledge, but flowing through a dependency graph with full provenance.
