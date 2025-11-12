# Tutorial: Your First Graft

This tutorial walks you through creating your first graft artifact: a weekly team status report that evolves as your source data changes.

## Prerequisites

- Python 3.14+
- Git repository (initialized with `git init`)
- Docker (if using container transformers)

Install Graft:
```bash
pip install graft
```

## Step 1: Initialize Your Project

Create a new git repository and initialize Graft:

```bash
mkdir my-team-grafts
cd my-team-grafts
git init
graft init
```

This creates `graft.config.yaml` with sensible defaults:

```yaml
version: 1
orchestrator:
  type: dvc
  managed_stage_prefix: "graft:"
  sync_policy: apply
  roots: ["."]
```

Commit the configuration:
```bash
git add graft.config.yaml
git commit -m "Initialize Graft project"
```

## Step 2: Create Source Materials

Create a directory for source materials. These will feed your graft:

```bash
mkdir -p sources
```

Create `sources/team-metrics.yaml` with sample data:

```yaml
sprint: 2025-W46
team: Platform Engineering
metrics:
  stories_completed: 12
  stories_planned: 15
  velocity: 28
highlights:
  - "Deployed new API gateway"
  - "Migrated 3 services to Kubernetes"
  - "Reduced deployment time by 40%"
blockers:
  - "Waiting on security review for auth service"
```

Commit the source material:
```bash
git add sources/
git commit -m "Add team metrics for W46"
```

## Step 3: Create Your First Artifact

Create an artifact directory:

```bash
mkdir -p artifacts/status-report
```

Create `artifacts/status-report/graft.yaml`:

```yaml
graft: status-report
inputs:
  materials:
    - { path: "../../sources/team-metrics.yaml", rev: HEAD }
derivations:
  - id: report
    template:
      source: file
      engine: jinja2
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

This configuration says:
- **graft: status-report** — Name of this artifact
- **materials** — Depends on `team-metrics.yaml` from sources
- **template** — Use Jinja2 template to generate context
- **outputs** — Produce `report.md`
- **policy**:
  - `deterministic: true` — Template rendering is reproducible
  - `attest: required` — Must finalize with agent attribution
  - `direct_edit: true` — Report can be manually edited after generation

Create `artifacts/status-report/template.md`:

```markdown
# Team Status Report

**Sprint**: {{ metrics.sprint }}
**Team**: {{ metrics.team }}

## Metrics

- Stories Completed: {{ metrics.metrics.stories_completed }}/{{ metrics.metrics.stories_planned }}
- Velocity: {{ metrics.metrics.velocity }}

## Highlights

{% for highlight in metrics.highlights %}
- {{ highlight }}
{% endfor %}

## Blockers

{% for blocker in metrics.blockers %}
- {{ blocker }}
{% endfor %}

## Commentary

_Add your analysis and context here._
```

The template reads from `materials` (available as variables) and generates a structured report. The "Commentary" section is left for manual input.

## Step 4: Run Your First Derivation

Execute the transformation:

```bash
graft run artifacts/status-report/
```

You'll see output like:
```
Running derivation: status-report/report
Evaluated template: .graft/evaluated/input.md
Generated: artifacts/status-report/report.md
Run complete. Ready to finalize.
```

Inspect the generated `artifacts/status-report/report.md`:

```markdown
# Team Status Report

**Sprint**: 2025-W46
**Team**: Platform Engineering

## Metrics

- Stories Completed: 12/15
- Velocity: 28

## Highlights

- Deployed new API gateway
- Migrated 3 services to Kubernetes
- Reduced deployment time by 40%

## Blockers

- Waiting on security review for auth service

## Commentary

_Add your analysis and context here._
```

## Step 5: Add Human Context

Edit `artifacts/status-report/report.md` directly. Replace the Commentary section:

```markdown
## Commentary

The deployment time improvement came from our new CI pipeline work. This unblocks
several teams who were frustrated with slow feedback loops.

The auth service security review is critical path for next sprint's customer portal
launch. We should escalate if not resolved by Wednesday.
```

This is the key insight: **you edit the generated file directly**. Graft tracks that you did this.

## Step 6: Finalize the Artifact

Complete the transaction with attribution:

```bash
graft finalize artifacts/status-report/ --agent "Jane Doe"
```

This creates `.graft/provenance/report.json` with:
- Hashes of all materials used
- Hash of the evaluated template
- Hash of the final output
- Who finalized (Jane Doe)
- When finalized
- Policy constraints

Commit everything together:
```bash
git add artifacts/status-report/
git commit -m "Add W46 status report"
```

This commit is your atomic transaction: the report + its provenance.

## Step 7: Handle Material Changes

The next week, update your source materials:

Edit `sources/team-metrics.yaml`:

```yaml
sprint: 2025-W47
team: Platform Engineering
metrics:
  stories_completed: 15
  stories_planned: 15
  velocity: 32
highlights:
  - "Completed auth service security review"
  - "Launched customer portal beta"
  - "Onboarded 2 new team members"
blockers: []
```

Commit the changes:
```bash
git add sources/team-metrics.yaml
git commit -m "Update metrics for W47"
```

## Step 8: Check Status

See what's affected:

```bash
graft status artifacts/status-report/
```

Output shows:
```
Artifact: status-report
Status: dirty (materials changed)

Materials changed:
  - sources/team-metrics.yaml (modified)

Action needed:
  - Run derivation to regenerate from updated materials
  - Or manually propagate changes to outputs
  - Then finalize with agent attribution
```

## Step 9: Propagate the Update

You have choices for how to propagate:

**Option A: Re-run the template**
```bash
graft run artifacts/status-report/
```

This re-generates the report from the new metrics. Your previous commentary is **overwritten** because the template re-creates the whole file.

**Option B: Manual propagation**

Instead, you can manually edit `report.md` to incorporate the new data while preserving your commentary. Check the evaluated template for context:

```bash
cat artifacts/status-report/.graft/evaluated/input.md
```

Then manually update `report.md` with new metrics.

**Option C: Smarter template**

For this use case, a better approach is a template that only generates data sections, with commentary preserved separately. But that's for later.

For now, let's re-run and add new commentary:

```bash
graft run artifacts/status-report/
```

Edit the generated file to add new commentary:

```markdown
## Commentary

Great week! We cleared the auth service blocker and launched the customer portal
beta ahead of schedule. The velocity increase reflects the two new team members
ramping up quickly.

Focus next sprint: scaling portal infrastructure as beta users grow.
```

## Step 10: Finalize Again

```bash
graft finalize artifacts/status-report/ --agent "Jane Doe"
```

Commit:
```bash
git add artifacts/status-report/
git commit -m "Update status report for W47"
```

## Step 11: Review Your History

Check your git history:

```bash
git log --oneline artifacts/status-report/
```

You'll see:
```
abc1234 Update status report for W47
def5678 Add W46 status report
```

Each commit includes:
- Updated `report.md`
- Updated provenance in `.graft/provenance/report.json`

Inspect a provenance file:

```bash
cat artifacts/status-report/.graft/provenance/report.json | jq .
```

You'll see the complete audit trail: materials used, template hash, output hash, who finalized, when.

## Step 12: Impact Analysis (Optional)

If you had other artifacts depending on this report, you could check downstream impact:

```bash
graft impact artifacts/status-report/
```

This would show what else needs updating when your report changes.

## What You've Learned

1. **Creating artifacts** — `graft.yaml` defines materials, transformation, outputs, policy
2. **Running derivations** — `graft run` executes transformations
3. **Editing outputs** — Direct file editing is normal workflow when `direct_edit: true`
4. **Finalizing** — `graft finalize` creates provenance with attribution
5. **Handling changes** — Materials change, you propagate updates, finalize again
6. **Audit trails** — Provenance captures complete history

## Next Steps

### Add a Container Transformer

For more complex transformations (data processing, API calls, computations), use container-based transformers.

Create `artifacts/data-summary/Dockerfile`:

```dockerfile
FROM python:3.11-slim

WORKDIR /workspace
RUN pip install pyyaml

COPY transform.py /transform.py
CMD ["python", "/transform.py"]
```

Create `artifacts/data-summary/transform.py`:

```python
import os, json, yaml

# Graft provides these via environment
materials = json.loads(os.getenv("GRAFT_MATERIALS", "[]"))
outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))
params = json.loads(os.getenv("GRAFT_PARAMS", "{}"))

# Read materials
with open(materials[0], 'r') as f:
    data = yaml.safe_load(f)

# Process (example: aggregate metrics)
summary = {
    "completion_rate": data["metrics"]["stories_completed"] / data["metrics"]["stories_planned"],
    "velocity": data["metrics"]["velocity"]
}

# Write output
with open(outputs[0], 'w') as f:
    yaml.dump(summary, f)
```

Update `graft.yaml`:

```yaml
derivations:
  - id: summary
    transformer:
      build:
        image: "graft-data-summary:local"
        context: "."
    outputs:
      - { path: "./summary.yaml" }
    policy:
      deterministic: true
```

Run it:
```bash
graft run artifacts/data-summary/
graft finalize artifacts/data-summary/ --agent "CI"
```

The container executes in isolation, produces outputs deterministically.

### Create Artifact Dependencies

Make one artifact depend on another:

```yaml
graft: executive-summary
inputs:
  materials:
    - { path: "../status-report/report.md", rev: HEAD }
    - { path: "../data-summary/summary.yaml", rev: HEAD }
derivations:
  - id: exec-summary
    template:
      source: file
      engine: jinja2
      file: "./template.md"
    outputs:
      - { path: "./executive-summary.md" }
```

Now when `status-report` changes and is finalized, `executive-summary` becomes dirty. Updates flow through the DAG.

### Reference Remote Materials

Pull data from external repositories:

```yaml
inputs:
  materials:
    - { path: "https://github.com/your-org/metrics/raw/main/data.yaml", rev: v1.2.3 }
```

Graft fetches the exact version, records it in provenance, enabling reproducibility.

## Troubleshooting

**"graft.yaml not found"**
- Ensure you're running `graft run` from the repository root or providing the full path
- The artifact directory must contain `graft.yaml`

**"Materials not found"**
- Check that material paths are relative to repository root
- Verify git refs exist (e.g., `rev: HEAD` requires committed files)

**"Docker build failed"**
- Ensure Docker is running
- Check Dockerfile syntax
- Review build logs for dependency issues

**"Finalize failed: outputs don't match"**
- If `direct_edit: false`, you may have accidentally edited outputs
- Re-run `graft run` to regenerate from transformer

**"DVC errors"**
- Graft uses DVC for orchestration
- Ensure DVC is installed: `dvc version`
- Check that `graft.config.yaml` exists at repository root

## Further Reading

- **[Workflows](workflows.md)** — Patterns for automated, manual, and hybrid transformations
- **[Use Cases](use-cases.md)** — Real-world scenarios and narratives
- **[graft.yaml Reference](graft-yaml-reference.md)** — Complete configuration schema
- **[CLI Reference](cli-reference.md)** — All commands and options

Start building grafts that bind your materials into living artifacts!
