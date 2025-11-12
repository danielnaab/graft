# graft.yaml Reference

Complete reference for `graft.yaml` configuration format.

## Overview

Each artifact directory contains a `graft.yaml` file defining:
- Artifact identity
- Input materials (dependencies)
- Derivations (transformations)
- Templates (if used)
- Outputs
- Policy constraints

## Top-Level Structure

```yaml
graft: <artifact-name>
inputs:
  materials: [...]
derivations:
  - id: <derivation-id>
    transformer: {...}
    template: {...}
    outputs: [...]
    policy: {...}
```

---

## `graft` (required)

**Type:** String

**Description:** Unique name for this artifact.

**Example:**
```yaml
graft: sprint-brief
```

**Notes:**
- Used for identification in logs, DVC stages, provenance
- Should be descriptive and unique within your project
- Recommended: kebab-case

---

## `inputs`

**Type:** Object

**Description:** Declares all materials this artifact depends on.

### `inputs.materials`

**Type:** Array of material objects

**Description:** List of files or data sources this artifact reads from.

**Material object fields:**

#### `path` (required)
**Type:** String

Path to material file. Can be:
- Relative to repository root: `../../sources/data.yaml`
- Git URL with path: `https://github.com/org/repo/raw/main/data.json`
- Glob pattern: `../../sources/tickets/*.yaml`

#### `rev` (required)
**Type:** String

Git revision to use:
- `HEAD` — Current commit
- Commit SHA: `abc1234...`
- Tag: `v1.2.3`
- Branch: `main` (for remote materials)

**Examples:**

**Local materials:**
```yaml
inputs:
  materials:
    - { path: "../../sources/tickets.yaml", rev: HEAD }
    - { path: "../other-artifact/output.md", rev: HEAD }
```

**Remote materials:**
```yaml
inputs:
  materials:
    - path: "https://github.com/threatcorp/intel/raw/v2.3.0/cves.json"
      rev: v2.3.0
```

**Glob patterns:**
```yaml
inputs:
  materials:
    - path: "../../meetings/retros/*.md"
      rev: HEAD
```

---

## `derivations`

**Type:** Array of derivation objects

**Description:** Transformations that produce outputs from materials.

An artifact can have multiple derivations, each producing different outputs.

### Derivation Object

```yaml
derivations:
  - id: <derivation-id>
    transformer: {...}
    template: {...}
    outputs: [...]
    policy: {...}
```

---

### `derivations[].id` (required)

**Type:** String

**Description:** Unique identifier for this derivation within the artifact.

**Example:**
```yaml
derivations:
  - id: brief
```

**Notes:**
- Used in DVC stage names: `graft:<artifact>:<derivation-id>`
- Used in provenance filenames: `.graft/provenance/<derivation-id>.json`

---

### `derivations[].transformer`

**Type:** Object (optional)

**Description:** Specifies how to transform materials into outputs.

If omitted: derivation is manual (no automated transformation).

#### Container Transformer

```yaml
transformer:
  build:
    image: <image-name>
    context: <build-context>
    dockerfile: <path>     # optional, default: Dockerfile
    target: <build-stage>  # optional
    args: {...}            # optional
  params: {...}            # optional
```

**Fields:**

**`build.image`** — Docker image name (e.g., `"graft-normalizer:local"`)

**`build.context`** — Build context directory (e.g., `"."` for artifact directory)

**`build.dockerfile`** — Path to Dockerfile (default: `Dockerfile` in context)

**`build.target`** — Multi-stage build target (optional)

**`build.args`** — Build arguments as key-value object (optional)

**`params`** — Parameters passed to container via `GRAFT_PARAMS` environment variable

**Example:**
```yaml
transformer:
  build:
    image: "graft-backlog:local"
    context: "."
    args:
      PYTHON_VERSION: "3.11"
  params:
    mode: "jira->yaml"
    include_archived: false
```

**Container receives:**
- `GRAFT_ARTIFACT_DIR` — Artifact directory path
- `GRAFT_MATERIALS` — JSON array of material paths
- `GRAFT_OUTPUTS` — JSON array of output paths
- `GRAFT_PARAMS` — JSON object: `{"mode": "jira->yaml", "include_archived": false}`

#### Reference Transformer (future)

```yaml
transformer:
  ref: <transformer-name>
  params: {...}
```

Points to built-in or shared transformer.

**Example:**
```yaml
transformer:
  ref: report-md
  params:
    title: "Sprint Brief"
```

---

### `derivations[].template`

**Type:** Object (optional)

**Description:** Template that provides context or generates output.

```yaml
template:
  source: <source-type>
  engine: <template-engine>
  content_type: <mime-type>
  file: <path>              # if source: file
  content: <string>         # if source: inline
  persist: <persist-mode>   # optional
  persist_path: <path>      # if persist: text
```

**Fields:**

#### `source` (required)
**Type:** Enum: `file`, `inline`, `none`

Where template content comes from:
- `file` — External template file
- `inline` — Template content in YAML
- `none` — No template

#### `engine`
**Type:** Enum: `jinja2`, `none`

Template engine to use:
- `jinja2` — Jinja2 templating (for text generation)
- `none` — No templating (pass-through)

#### `content_type`
**Type:** String (MIME type)

Content type of template:
- `text/markdown`
- `text/html`
- `application/json`
- etc.

#### `file`
**Type:** String (path)

Path to template file (relative to artifact directory). Required if `source: file`.

#### `content`
**Type:** String

Inline template content. Required if `source: inline`.

#### `persist`
**Type:** Enum: `text`, `never`

Whether to save evaluated template:
- `text` — Save evaluated template to file
- `never` — Don't save

#### `persist_path`
**Type:** String (path)

Where to save evaluated template. Required if `persist: text`.

**Examples:**

**File-based Jinja2 template:**
```yaml
template:
  source: file
  engine: jinja2
  content_type: text/markdown
  file: "./template.md"
  persist: text
  persist_path: "./.graft/evaluated/input.md"
```

**Inline template:**
```yaml
template:
  source: inline
  engine: jinja2
  content_type: text/markdown
  content: |
    # Report: {{ title }}
    Generated: {{ timestamp }}
```

**No template:**
```yaml
template:
  source: none
  engine: none
  content_type: application/json
  persist: never
```

**Template variables:**

Templates have access to `materials` as variables. Example:

If `graft.yaml` declares:
```yaml
inputs:
  materials:
    - { path: "../../sources/data.yaml", rev: HEAD }
```

And `data.yaml` contains:
```yaml
team: Platform
metrics:
  velocity: 28
```

Template can use:
```jinja2
Team: {{ materials['../../sources/data.yaml'].team }}
Velocity: {{ materials['../../sources/data.yaml'].metrics.velocity }}
```

Or with single material, simpler:
```jinja2
Team: {{ metrics.team }}
Velocity: {{ metrics.metrics.velocity }}
```

---

### `derivations[].outputs`

**Type:** Array of output objects

**Description:** Files produced by this derivation.

**Output object fields:**

#### `path` (required)
**Type:** String

Path to output file (relative to artifact directory).

#### `schema` (optional)
**Type:** String

Schema name for validation.

**Example:**
```yaml
outputs:
  - { path: "./brief.md" }
  - { path: "./summary.yaml", schema: summary_schema }
```

**Notes:**
- Outputs are committed to git
- Can be materials for other artifacts
- Schema validation (if specified) happens before finalize

---

### `derivations[].policy`

**Type:** Object (optional)

**Description:** Policy constraints governing this derivation.

```yaml
policy:
  deterministic: <bool>
  attest: <attestation-requirement>
  direct_edit: <bool>
```

#### `deterministic`
**Type:** Boolean (default: `true`)

Whether transformation is reproducible:
- `true` — Same inputs always produce same outputs
- `false` — Transformation may be non-deterministic

**Use `true` for:**
- Container transformers processing data
- Template rendering

**Use `false` for:**
- Transformers that call external APIs
- Time-dependent transformations

#### `attest`
**Type:** Enum: `required`, `optional` (default: `required`)

Whether finalize must include agent attribution:
- `required` — Must provide `--agent` when finalizing
- `optional` — Can finalize without attribution

**Use `required` for:**
- Artifacts requiring human review
- Compliance/audit scenarios
- When attribution matters

**Use `optional` for:**
- Fully automated pipelines
- CI-generated artifacts

#### `direct_edit`
**Type:** Boolean (default: `false`)

Whether outputs can be manually edited:
- `true` — Manual edits are expected workflow
- `false` — Outputs should only come from transformer

**Use `true` for:**
- Documentation requiring human refinement
- Reports with commentary
- AI agent collaboration

**Use `false` for:**
- Purely automated transformations
- Generated code (without customization)

**Examples:**

**Fully automated:**
```yaml
policy:
  deterministic: true
  attest: optional
  direct_edit: false
```

**Manual with guidance:**
```yaml
policy:
  deterministic: true
  attest: required
  direct_edit: true
```

**AI agent collaboration:**
```yaml
policy:
  deterministic: false  # AI may be non-deterministic
  attest: required      # Must attribute agent
  direct_edit: true     # Agent edits outputs
```

---

## Complete Examples

### Template-Only Artifact

```yaml
graft: status-report
inputs:
  materials:
    - { path: "../../sources/metrics.yaml", rev: HEAD }
derivations:
  - id: report
    template:
      source: file
      engine: jinja2
      content_type: text/markdown
      file: "./template.md"
      persist: text
      persist_path: "./.graft/evaluated/input.md"
    outputs:
      - { path: "./report.md" }
    policy:
      deterministic: true
      attest: required
      direct_edit: true
```

### Container Transformer

```yaml
graft: backlog-normalizer
inputs:
  materials:
    - { path: "../../sources/jira-snapshot.json", rev: HEAD }
derivations:
  - id: normalize
    transformer:
      build:
        image: "graft-backlog:local"
        context: "."
      params:
        mode: "jira->yaml"
    template:
      source: none
      engine: none
      content_type: application/json
      persist: never
    outputs:
      - { path: "./backlog.yaml", schema: backlog }
    policy:
      deterministic: true
      attest: optional
      direct_edit: false
```

### Manual Derivation (No Transformer)

```yaml
graft: architecture-decision
inputs:
  materials:
    - { path: "../../meetings/arch-review-2025-11.md", rev: HEAD }
    - { path: "../code-analysis/metrics.yaml", rev: HEAD }
derivations:
  - id: decision
    template:
      source: file
      engine: jinja2
      file: "./decision-template.md"
      persist: text
      persist_path: "./.graft/evaluated/context.md"
    outputs:
      - { path: "./ADR-042-service-mesh.md" }
    policy:
      attest: required  # Tech lead must sign off
      direct_edit: true # Manual derivation
```

### Remote Material Dependency

```yaml
graft: threat-assessment
inputs:
  materials:
    - path: "https://github.com/threatcorp/intel/raw/v2.3.0/cves.json"
      rev: v2.3.0
    - path: "../../internal/infrastructure.yaml"
      rev: HEAD
derivations:
  - id: assess
    transformer:
      build:
        image: "threat-analyzer:local"
        context: "."
    outputs:
      - { path: "./threat-report.md" }
    policy:
      deterministic: true
      attest: required
```

### Multi-Derivation Artifact

```yaml
graft: data-analysis
inputs:
  materials:
    - { path: "../../data/raw/*.csv", rev: HEAD }
derivations:
  - id: clean
    transformer:
      build: { image: "data-cleaner:local", context: "." }
    outputs:
      - { path: "./cleaned.csv" }
    policy:
      deterministic: true
      attest: optional

  - id: analyze
    transformer:
      build: { image: "data-analyzer:local", context: "." }
    outputs:
      - { path: "./analysis.md" }
    policy:
      deterministic: true
      attest: required
      direct_edit: true  # Analyst adds interpretation
```

First derivation cleans data automatically. Second derivation analyzes and produces a report that the analyst refines.

---

## Configuration Validation

Validate your configuration:

```bash
graft validate artifacts/my-artifact/
```

Common errors:

**Missing required fields:**
```
Error: 'graft' is required
Error: 'derivations[0].id' is required
```

**Invalid paths:**
```
Error: Material path '../../missing.yaml' does not exist
Error: Template file './template.md' not found
```

**Schema validation:**
```
Error: Output './data.yaml' does not match schema 'backlog'
  - Missing required field: items
  - Invalid type for field 'id': expected string, got number
```

---

## Best Practices

**Naming:**
- Use descriptive artifact names: `sprint-brief`, not `brief`
- Use clear derivation IDs: `normalize`, `analyze`, `report`

**Materials:**
- Use relative paths from repo root for consistency
- Pin remote materials to specific versions (`rev: v1.2.0`)
- Use glob patterns sparingly (can be slow)

**Templates:**
- Persist evaluated templates for debugging: `persist: text`
- Use descriptive persist paths: `./.graft/evaluated/brief-input.md`
- Keep templates in artifact directory for co-location

**Transformers:**
- Build images with descriptive names: `graft-<artifact>:local`
- Use `context: "."` to build from artifact directory
- Pass configuration via `params`, not hardcoded in Dockerfile

**Outputs:**
- Place outputs in artifact directory for clarity
- Use schema validation for structured formats
- Name outputs descriptively: `report.md`, not `output.md`

**Policy:**
- Default to `attest: required` unless purely automated
- Use `direct_edit: true` when human refinement is expected
- Use `deterministic: true` for reproducible transformations

**Multi-derivation artifacts:**
- Order derivations if one depends on another's output
- Use different IDs for each derivation
- Consider splitting into separate artifacts if too complex

---

## Migrating Configuration

### From Simple to Container Transformer

Before (template-only):
```yaml
derivations:
  - id: report
    template:
      source: file
      engine: jinja2
      file: "./template.md"
    outputs:
      - { path: "./report.md" }
```

After (add container for complex processing):
```yaml
derivations:
  - id: report
    transformer:
      build:
        image: "report-generator:local"
        context: "."
    template:
      source: file
      file: "./template.md"
      persist: text
      persist_path: "./.graft/evaluated/input.md"
    outputs:
      - { path: "./report.md" }
```

### Adding Remote Materials

Before (local only):
```yaml
inputs:
  materials:
    - { path: "../../sources/data.yaml", rev: HEAD }
```

After (add upstream dependency):
```yaml
inputs:
  materials:
    - { path: "../../sources/data.yaml", rev: HEAD }
    - path: "https://github.com/upstream/data/raw/v1.0/dataset.json"
      rev: v1.0
```

### Enabling Manual Refinement

Before (automated only):
```yaml
policy:
  deterministic: true
  attest: optional
  direct_edit: false
```

After (allow human edits):
```yaml
policy:
  deterministic: true
  attest: required  # Now require attribution
  direct_edit: true # Allow manual refinement
```

---

Next: See [Workflows](workflows.md) for patterns using these configurations.
