# Workflows and Patterns

This document describes common patterns for building and maintaining grafts in different scenarios.

## Fully Automated Workflows

Automated workflows use container transformers to process materials deterministically, producing outputs without human intervention.

### Pattern: Data Normalization Pipeline

**Use case:** Ingest external data (API snapshots, CSV exports), normalize to standard format.

```yaml
graft: backlog-normalizer
inputs:
  materials:
    - { path: "../../sources/external/jira/snapshot.json", rev: HEAD }
derivations:
  - id: normalize
    transformer:
      build:
        image: "graft-backlog:local"
        context: "."
      params: { mode: "jira->yaml" }
    outputs:
      - { path: "./backlog.yaml", schema: backlog }
    policy:
      deterministic: true
      attest: optional
```

**Workflow:**
1. CI ingests JIRA snapshot to `sources/external/jira/snapshot.json`
2. Commit triggers DVC pipeline via `graft dvc scaffold`
3. Container executes, normalizes data, produces `backlog.yaml`
4. Schema validation ensures output format
5. Auto-finalize (since `attest: optional`)
6. Downstream grafts see updated backlog

**Key characteristics:**
- No human in the loop
- Deterministic (same input → same output)
- Can auto-merge if policy allows
- Schema validation catches errors

### Pattern: Scheduled Reports

**Use case:** Generate weekly/monthly reports from accumulated data.

```yaml
graft: monthly-metrics
inputs:
  materials:
    - { path: "../../sources/metrics/2025-11-*.json", rev: HEAD }
derivations:
  - id: aggregate
    transformer:
      build: { image: "metrics-aggregator:local" }
    template:
      source: file
      engine: jinja2
      file: "./report-template.md"
    outputs:
      - { path: "./2025-11-report.md" }
    policy:
      deterministic: true
```

**Workflow:**
1. End of month, CI commits all daily metrics
2. Graft detects materials changed
3. Container aggregates data
4. Template renders report
5. Review and finalize with analyst attribution

**Key characteristics:**
- Automated processing
- Human review before finalization
- Template provides structure
- Audit trail for compliance

## Manual Workflows with Guidance

Manual workflows use templates to provide context, but humans or agents do the actual work.

### Pattern: Context-Driven Documentation

**Use case:** Sprint briefs where template shows what changed, human adds interpretation.

```yaml
graft: sprint-brief
inputs:
  materials:
    - { path: "../../sources/tickets/sprint-2025-W47.yaml", rev: HEAD }
    - { path: "../roadmap/current-quarter.md", rev: HEAD }
derivations:
  - id: brief
    template:
      source: file
      engine: jinja2
      file: "./template.md"
      persist: text
      persist_path: "./.graft/evaluated/brief-input.md"
    outputs:
      - { path: "./brief.md" }
    policy:
      direct_edit: true
      attest: required
```

**Workflow:**
1. Tickets and roadmap update during the week
2. Friday, PM runs `graft run sprint-brief/`
3. Template evaluates, showing structured data
4. PM edits `brief.md` directly:
   - Adds commentary on progress
   - Highlights risks
   - Adds human context templates can't capture
5. `graft finalize sprint-brief/ --agent "Jane Doe, PM"`
6. Commit includes brief + provenance

**Key characteristics:**
- Template provides scaffolding
- Human adds value (interpretation, context)
- Evaluated template persisted for reference
- Attribution captures who made decisions

### Pattern: AI Agent Collaboration

**Use case:** AI agent maintains documentation with human oversight.

```yaml
graft: api-documentation
inputs:
  materials:
    - { path: "../../src/api/**/*.ts", rev: HEAD }
    - { path: "../architecture/api-design.md", rev: HEAD }
derivations:
  - id: docs
    template:
      source: file
      file: "./template.md"
    outputs:
      - { path: "./api-reference.md" }
    policy:
      direct_edit: true
      attest: required
```

**Workflow:**
1. Developer changes API code
2. AI agent (Claude Code) calls `graft explain api-documentation/ --json`
3. Agent sees: what files changed, evaluated template (shows API structure)
4. Agent updates `api-reference.md` surgically (only affected sections)
5. Agent finalizes: `graft finalize --agent "claude-sonnet-4" --role agent`
6. Opens PR for human review
7. Developer reviews: AI got it right, approves and merges

**Key characteristics:**
- AI agent as first-class actor
- Human review via PR
- Full provenance: what API code, what agent, what changes
- Can reject and ask agent to revise

## Hybrid Workflows

Hybrid workflows combine automated transformation with human refinement.

### Pattern: Generate Then Refine

**Use case:** Container produces draft, human polishes.

```yaml
graft: data-analysis
inputs:
  materials:
    - { path: "../../data/raw/*.csv", rev: HEAD }
derivations:
  - id: analyze
    transformer:
      build: { image: "data-analyzer:local" }
    outputs:
      - { path: "./analysis.md" }
    policy:
      deterministic: true
      direct_edit: true
      attest: required
```

**Workflow:**
1. Raw data arrives
2. `graft run` executes container
3. Container produces analysis.md with:
   - Statistical summaries
   - Auto-generated charts
   - Initial findings
4. Data scientist reviews, adds:
   - Interpretation of outliers
   - Recommendations
   - Caveats and limitations
5. Finalizes with attribution
6. Downstream grafts (executive summary, presentation) pull from refined analysis

**Key characteristics:**
- Automation does heavy lifting (stats, charts)
- Human adds expertise (interpretation)
- Both transformer and human contribute
- Provenance shows both automated and manual contributions

### Pattern: Smart Merge on Update

**Use case:** Re-running transformer that preserves human edits.

Design a transformer that:
- Reads previous output
- Detects human-edited sections (markers, special comments)
- Regenerates automated sections
- Preserves human sections
- Merges intelligently

Example transformer logic:
```python
# Read previous output
try:
    with open(outputs[0], 'r') as f:
        previous = f.read()
    human_sections = extract_marked_sections(previous, marker="<!-- HUMAN EDIT -->")
except FileNotFoundError:
    human_sections = {}

# Generate new content
generated = process_materials(materials)

# Merge: keep human edits, update automated sections
final = merge(generated, human_sections)

# Write output
with open(outputs[0], 'w') as f:
    f.write(final)
```

This enables: re-run the graft, get updated data sections, preserve commentary.

## PR-Based Review Patterns

### Pattern: Team Consensus on Policy Changes

**Use case:** Working agreements require team discussion.

```yaml
graft: working-agreements
inputs:
  materials:
    - { path: "../../meetings/retros/*.md", rev: HEAD }
derivations:
  - id: agreements
    template:
      source: file
      file: "./template.md"
    outputs:
      - { path: "./team-agreements.md" }
    policy:
      direct_edit: true
      attest: required
```

**Workflow:**
1. Retrospective identifies process improvement
2. Team member updates working-agreements/team-agreements.md
3. Finalizes: `graft finalize --agent "Alice"`
4. Creates PR with changes
5. Team discusses in PR comments:
   - "Do we all agree async standups work?"
   - "What about new hires who need more sync time?"
   - Consensus emerges
6. Team approves PR
7. Merge captures: what changed, who proposed, what team discussed, final decision

**Key characteristics:**
- PR is the consensus mechanism
- Graft provides: provenance, attribution, audit trail
- GitHub provides: review, discussion, approval workflow
- Combined: traceable team decisions

### Pattern: Authority-Based Approval

**Use case:** Architecture decisions require tech lead sign-off.

Set up GitHub branch protection:
- Require review from `@tech-leads` team
- Require status checks (tests pass)

```yaml
graft: architecture-decision
policy:
  attest: required
```

**Workflow:**
1. Engineer proposes architecture change
2. Updates architecture-decision graft
3. Finalizes: `graft finalize --agent "Bob, Engineer"`
4. Opens PR
5. Tech lead reviews:
   - Provenance shows: what inputs Bob considered
   - Diff shows: what Bob changed
   - Comments: discussion, questions
6. Tech lead approves (or requests changes)
7. Merge: decision is made, attribution clear

**Key characteristics:**
- Graft: provenance and attribution
- GitHub: authority enforcement (required reviewers)
- Combined: authorized, auditable decisions

## Remote References and Composition

### Pattern: Upstream Data Dependency

**Use case:** Reference external datasets with version pinning.

```yaml
graft: threat-analysis
inputs:
  materials:
    - path: "https://github.com/threatcorp/intel/raw/v2.3.0/cves.json"
      rev: v2.3.0
    - path: "../../internal/infrastructure.yaml"
      rev: HEAD
derivations:
  - id: analyze
    transformer:
      build: { image: "threat-analyzer:local" }
    outputs:
      - { path: "./threat-report.md" }
```

**Workflow:**
1. Pin to specific upstream version (`v2.3.0`)
2. Run analysis combining upstream + internal data
3. When ready to upgrade:
   - Change ref to `v2.4.0`
   - Re-run analysis
   - Review what changed in results
   - Finalize if acceptable
4. Provenance shows: exactly what upstream version used

**Key characteristics:**
- Version pinning for reproducibility
- Upgrade when ready (not forced)
- Provenance tracks external dependencies
- Can audit "what version did we use in February report?"

### Pattern: Workflow Extension

**Use case:** Build on shared transformation logic.

Upstream org publishes grafts:
```
https://github.com/data-team/normalizers/
  artifacts/
    csv-normalizer/
      graft.yaml
      transform.py
      Dockerfile
```

Your org references their outputs:
```yaml
graft: custom-analysis
inputs:
  materials:
    - path: "https://github.com/data-team/normalizers/raw/main/artifacts/csv-normalizer/output.csv"
      rev: v1.2.0
derivations:
  - id: analyze
    transformer:
      build: { image: "our-analyzer:local" }
```

**Workflow:**
1. Upstream team improves normalization
2. Publishes new release (v1.3.0)
3. You update material ref: `v1.2.0` → `v1.3.0`
4. Your analysis runs on improved data
5. Both provenance trails connect:
   - Upstream: how they normalized
   - Your artifact: what upstream version you used

**Key characteristics:**
- Workflow supply chain
- Semantic versioning
- Clear provenance across organizational boundaries
- Can contribute improvements back upstream

## Multi-Stage Pipelines

### Pattern: Progressive Refinement

**Use case:** Raw data → cleaned → aggregated → summarized.

```
sources/raw-logs.json
  ↓
artifacts/cleaned-data/
  graft: clean
  output: cleaned.csv
  ↓
artifacts/aggregated-metrics/
  graft: aggregate
  materials: [../cleaned-data/cleaned.csv]
  output: metrics.yaml
  ↓
artifacts/executive-summary/
  graft: summarize
  materials: [../aggregated-metrics/metrics.yaml]
  output: summary.md (human edits)
```

**Workflow:**
1. Raw logs update
2. `cleaned-data` becomes dirty → run cleaner container
3. Cleaned data finalizes (automated)
4. `aggregated-metrics` becomes dirty → run aggregator
5. Metrics finalize (automated)
6. `executive-summary` becomes dirty → template evaluates
7. Analyst edits summary, adds interpretation
8. Analyst finalizes with attribution

**Key characteristics:**
- Multi-stage transformation
- Some stages automated, some manual
- DVC orchestrates the cascade
- Each stage has provenance
- Can inspect intermediate outputs

### Pattern: Fan-Out, Fan-In

**Use case:** One source feeds multiple analyses, which combine into final report.

```
sources/customer-data.csv
  ↓
  ├→ artifacts/sentiment-analysis/
  ├→ artifacts/usage-metrics/
  └→ artifacts/churn-prediction/
      ↓
      └→ artifacts/quarterly-report/
          materials: [all three analyses above]
```

**Workflow:**
1. Customer data updates
2. Three analyses run in parallel (DVC parallelizes)
3. Each analysis finalizes
4. Quarterly report becomes dirty
5. Template pulls from all three analyses
6. Executive adds strategic commentary
7. Finalizes quarterly report

**Key characteristics:**
- Parallel execution (DVC handles)
- Multiple perspectives on same data
- Final synthesis combines insights
- Provenance connects everything

## Validation and Quality Gates

### Pattern: Schema Validation

Ensure outputs match expected format:

```yaml
derivations:
  - id: normalize
    transformer:
      build: { image: "normalizer:local" }
    outputs:
      - path: "./data.yaml"
        schema: standard-schema
    policy:
      deterministic: true
```

If output doesn't match schema, finalize fails. Fix transformer and re-run.

### Pattern: Test-Driven Transformations

Write tests for your transformers:

```python
# test_transform.py
def test_normalization():
    input_data = load_fixture("sample-jira.json")
    output = transform(input_data)
    assert output["items"][0]["id"] == "PROJ-123"
    assert "title" in output["items"][0]
```

Run tests before committing transformer changes. Ensures transformations work as expected.

### Pattern: Simulation Before Finalize

Preview downstream impact:

```bash
graft simulate artifacts/sprint-brief/ --cascade
```

Shows: "If I finalize this, these 3 downstream artifacts will become dirty."

Decide: is this the right time? Should I update downstream artifacts too? Can I batch the updates?

## Common Patterns Summary

| Pattern | Transformer | Direct Edit | Attest | Use Case |
|---------|-------------|-------------|--------|----------|
| **Fully Automated** | Container | No | Optional | ETL, normalization, scheduled reports |
| **Manual with Guidance** | None | Yes | Required | Briefs, documentation, commentary |
| **AI Agent** | None | Yes | Required | Maintained docs, knowledge base |
| **Generate Then Refine** | Container | Yes | Required | Analysis with human interpretation |
| **Team Consensus** | Any | Yes | Required | Policies, agreements, processes |
| **Authority Approval** | Any | Either | Required | Architecture decisions, compliance |
| **Upstream Dependency** | Container | No | Optional | External data ingestion |
| **Multi-Stage Pipeline** | Container | Mixed | Mixed | Progressive transformation |

## Best Practices

**Start simple** — Begin with one graft, prove the workflow, then add complexity.

**Match policy to risk** — Low-stakes artifacts can auto-merge. High-stakes require review.

**Make provenance visible** — Show the team what's captured. Build trust.

**Use schema validation** — Catch errors early, ensure output quality.

**Test transformers** — Treat containers like code, write tests.

**Pin external dependencies** — Use specific versions for reproducibility.

**Review downstream impact** — Before finalizing, check what else is affected.

**Commit atomically** — Outputs + provenance together in one commit.

**Document policy decisions** — Why does this graft require attestation? Why not that one?

Next: See [CLI Reference](cli-reference.md) for command details, or [graft.yaml Reference](graft-yaml-reference.md) for configuration options.
