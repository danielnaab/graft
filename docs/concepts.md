# Core Concepts

This document explains Graft's mental model: how artifacts grow on your git tree, how transformations work, and what makes changes auditable.

## The Graft Metaphor

Like grafting branches in horticulture, Graft binds together content from different origins into living artifacts. Each graft is a directory on your git tree—shaped by its configuration, fed by source materials, producing files that grow and adapt as those materials change.

Unlike traditional build systems where outputs are ephemeral artifacts (compiled binaries, cached results), Graft produces **living files**: generated, edited by humans or agents, committed to version control, and evolved over time.

## Core Entities

### Artifacts

An **artifact** is a graft—a directory containing:
- `graft.yaml` (configuration defining the artifact)
- Source materials (or references to them)
- Templates (if used)
- Outputs (the generated files)
- Provenance records (in `.graft/provenance/`)

Example structure:
```
artifacts/sprint-brief/
├── graft.yaml              # Configuration
├── template.md             # Jinja2 template
├── brief.md                # Generated output
└── .graft/
    └── provenance/
        └── brief.json      # Audit record
```

Each artifact is a self-contained unit with explicit dependencies and transformation logic.

### Materials

**Materials** are the inputs that feed a graft. They can be:
- Local files in your repository
- Files from remote repositories (via git URL + ref)
- Snapshots ingested by CI
- Outputs from other grafts

Materials are declared in `graft.yaml`:
```yaml
inputs:
  materials:
    - { path: "../../sources/tickets.yaml", rev: HEAD }
    - { path: "https://github.com/org/data/raw/main/dataset.csv", rev: main }
```

Graft tracks material changes via git. When materials change, dependent artifacts become "dirty" and need attention.

### Derivations

A **derivation** is a transformation definition within an artifact. It specifies:
- What transformer to use (container, template, or manual)
- What template provides context (if any)
- What outputs to produce
- What policy governs the transformation

An artifact can have multiple derivations, each producing different outputs from the same materials.

```yaml
derivations:
  - id: brief
    transformer: { ref: report-md }
    template:
      source: file
      engine: jinja2
      file: "./template.md"
    outputs:
      - { path: "./brief.md" }
    policy:
      deterministic: true
      attest: required
      direct_edit: true
```

### Transformers

A **transformer** is what executes the derivation. Three types:

**Container transformers** — Docker containers that receive materials and produce outputs:
```yaml
transformer:
  build:
    image: "graft-normalizer:local"
    context: "."
  params: { mode: "jira->yaml" }
```

The container receives environment variables:
- `GRAFT_MATERIALS` — JSON array of material paths
- `GRAFT_OUTPUTS` — JSON array of output paths
- `GRAFT_PARAMS` — JSON object of transformer parameters

**Template transformers** — Reference to built-in or shared transformers (future):
```yaml
transformer: { ref: report-md, params: { title: "Sprint Brief" } }
```

**Manual transformers** — No automated transformation; human or agent does the work:
```yaml
# No transformer specified = manual derivation
```

### Templates

**Templates** provide scaffolding or context for transformations. Evaluated before the transformer runs, they can:
- Generate the actual output (for template-only derivations)
- Provide guidance for manual work (evaluated template shows what changed)
- Pass context to containers (via evaluated template file)

Templates support:
- Jinja2 (for text generation)
- Inline content (for simple cases)
- File-based (for reusable templates)

The evaluated template can be persisted for inspection:
```yaml
template:
  source: file
  engine: jinja2
  file: "./template.md"
  persist: text
  persist_path: "./.graft/evaluated/brief-input.md"
```

### Outputs

**Outputs** are the files produced by a derivation. They're committed to git, making them:
- Reviewable in PRs
- Versioned over time
- Usable as materials for other grafts

Outputs can be:
- Directly edited (if `policy.direct_edit: true`)
- Schema-validated (if `schema` specified)
- Referenced by downstream grafts

## The Dependency DAG

Graft uses DVC to model artifacts as a directed acyclic graph (DAG):

```
sources/tickets.yaml ──┐
                        ├──> sprint-brief/brief.md ──> roadmap/plan.md
sources/backlog.yaml ──┘
```

When `tickets.yaml` changes:
1. `sprint-brief` becomes dirty
2. After updating and finalizing `sprint-brief`, `roadmap` becomes dirty
3. Updates cascade through the graph

DVC handles orchestration: running only what changed, parallelizing independent branches, caching deterministic results.

## The Transformation Lifecycle

### 1. Run

`graft run <artifact/>` executes the derivation:

**For automated transformations:**
- Loads materials
- Evaluates template (if present)
- Runs container or executes transformation
- Produces outputs
- Creates a run record

**For manual transformations:**
- Evaluates template (if present) to show context
- Human or agent edits outputs directly
- No automated execution

### 2. Finalize

`graft finalize <artifact/>` completes the transaction:
- Validates all outputs exist
- Verifies policy compliance
- Records provenance (what materials, what transformer, who finalized)
- Creates attestation (agent name, role, timestamp)
- Marks the artifact as "clean"

Finalize is the commit boundary: one git commit includes updated outputs + provenance file.

### 3. Status

`graft status <artifact/>` shows:
- Whether materials changed (artifact is dirty)
- What downstream artifacts are affected
- What action is needed (run, manual edit, finalize)

### States

**Clean** — Materials unchanged since last finalize, outputs current

**Dirty** — Materials changed, artifact needs regeneration or manual propagation

**Error** — Outputs edited when `direct_edit: false`, or other policy violations

## Provenance and Attestation

Every finalize records **provenance**: a complete audit trail of the transformation.

**Provenance captures:**
- **Read set** — Exact materials (hashes, git refs)
- **Transformer** — Container image digest, parameters
- **Template** — Template source and evaluated template hash
- **Outputs** — File hashes, schema validation results
- **Policy** — What constraints governed the transformation
- **Timing** — When run prepared, when finalized

**Attestation captures:**
- **Who** — Agent name (human, AI, CI)
- **Role** — Optional classification (human, agent, model, ci)
- **Metadata** — Optional freeform context

This enables:
- **Auditability** — Verify exactly what happened
- **Reproducibility** — Re-run with same inputs and get same outputs (for deterministic transformations)
- **Attribution** — Know who approved what
- **Compliance** — Demonstrate regulatory requirements met

Provenance lives in `.graft/provenance/<derivation-id>.json` and is committed with outputs.

## Policy

**Policy** defines the rules governing a derivation:

```yaml
policy:
  deterministic: true    # Transformation must be reproducible
  attest: required       # Must finalize with agent attribution
  direct_edit: true      # Outputs can be manually edited
```

**`deterministic: true`** means:
- Transformation produces identical outputs given identical inputs
- Usually applies to container transformers
- Enables caching and verification

**`attest: required`** means:
- Must finalize with `--agent` attribution
- Creates accountability for the change

**`direct_edit: true`** means:
- Outputs can be manually edited after transformation
- Manual edits are expected workflow, not errors
- Useful for human/agent refinement of generated content

**`direct_edit: false`** means:
- Outputs should only come from transformation
- Manual edits are policy violations (detected by status)

## Automated vs. Manual Workflows

### Fully Automated

Derivation with transformer, no direct edit:
```yaml
derivations:
  - id: normalize
    transformer:
      build: { image: "normalizer:local", context: "." }
    outputs:
      - { path: "./data.yaml" }
    policy:
      deterministic: true
      attest: optional
```

Workflow:
1. Materials change
2. `graft run` executes container
3. Container produces outputs
4. `graft finalize` (or auto-finalize if policy allows)

### Manual with Guidance

Derivation with template, direct edit allowed:
```yaml
derivations:
  - id: brief
    template:
      source: file
      engine: jinja2
      file: "./template.md"
    outputs:
      - { path: "./brief.md" }
    policy:
      direct_edit: true
      attest: required
```

Workflow:
1. Materials change
2. `graft run` evaluates template (provides context/scaffolding)
3. Human or agent edits `brief.md` directly
4. `graft finalize --agent "Jane"` captures attribution

### Hybrid

Derivation with transformer AND direct edit:
```yaml
derivations:
  - id: analysis
    transformer: { build: { image: "analyze:local" } }
    outputs:
      - { path: "./results.md" }
    policy:
      direct_edit: true
      attest: required
```

Workflow:
1. Materials change
2. `graft run` executes container (generates draft)
3. Human reviews and refines the outputs
4. `graft finalize --agent "Dr. Smith"` captures both automation and human review

## Remote References and Composition

Grafts can reference materials and workflows from remote repositories:

```yaml
inputs:
  materials:
    - { path: "https://github.com/threatcorp/intel/normalized/cves.json", rev: v2.3.0 }
```

This enables:
- **Workflow supply chains** — Publish grafts, others extend them
- **Versioned dependencies** — Pin to specific releases
- **Cross-org collaboration** — Reference public datasets, share transformations
- **Composability** — Layer your transformations on upstream workflows

Provenance captures exact versions used, enabling reproducibility even with external dependencies.

## Key Takeaways

1. **Grafts are living files** — Not ephemeral build artifacts; they're versioned, editable, evolved
2. **Full provenance** — Every transformation records what was read, what was written, who finalized
3. **Humans and agents are first-class** — Manual work is a valid transformation with attribution
4. **Policy enforces trust boundaries** — Granular control over what's automated vs. reviewed
5. **Git is the ledger** — All changes flow through version control with complete history
6. **Composable across boundaries** — Reference remote workflows, build on others' work

Next: See [Tutorial](tutorial.md) to build your first graft, or [Workflows](workflows.md) for common patterns.
