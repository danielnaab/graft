# CLI Reference

Complete reference for all Graft commands.

## Global Options

All commands support:
- `--json` — Output structured JSON (where applicable)
- `--sync <policy>` — Override orchestrator sync policy (where applicable)

Exit codes:
- `0` — Success
- `1` — User error (bad input, missing file, invalid configuration)
- `2` — System error (permissions, unexpected exceptions)

## Commands

### `graft init [PATH]`

Initialize a Graft project by creating `graft.config.yaml` with defaults.

**Arguments:**
- `PATH` — Optional directory path (defaults to current directory)

**Example:**
```bash
graft init
graft init /path/to/project
```

**Output:**
Creates `graft.config.yaml`:
```yaml
version: 1
orchestrator:
  type: dvc
  managed_stage_prefix: "graft:"
  sync_policy: apply
  roots: ["."]
```

**Use case:** First step when setting up a new Graft project.

---

### `graft explain <ARTIFACT> [OPTIONS]`

Explain what an artifact does: show configuration, dependencies, derivations, and policy.

**Arguments:**
- `ARTIFACT` — Path to artifact directory containing `graft.yaml`

**Options:**
- `--json` — Output structured JSON
- `--sync <policy>` — Override sync policy (default: `warn`)

**Example:**
```bash
graft explain artifacts/sprint-brief/
graft explain artifacts/sprint-brief/ --json
```

**JSON Output:**
```json
{
  "artifact": "sprint-brief",
  "graft": "sprint-brief",
  "policy": {
    "deterministic": true,
    "attest": "required",
    "direct_edit": true
  },
  "inputs": {
    "materials": [
      {
        "path": "../../sources/tickets.yaml",
        "rev": "HEAD"
      }
    ]
  },
  "derivations": [
    {
      "id": "brief",
      "transformer": {"ref": "report-md"},
      "template": {...},
      "outputs": [{"path": "./brief.md"}]
    }
  ],
  "orchestrator": {...}
}
```

**Use case:**
- Understand artifact configuration
- AI agents query context before making changes
- Debug dependency issues

---

### `graft run <ARTIFACT> [OPTIONS]`

Execute a derivation: run transformer (container or template) to produce outputs.

**Arguments:**
- `ARTIFACT` — Path to artifact directory

**Options:**
- `--id <derivation-id>` — Run specific derivation (default: all derivations)
- `--sync <policy>` — Override sync policy (default: `apply`)

**Example:**
```bash
graft run artifacts/sprint-brief/
graft run artifacts/sprint-brief/ --id brief
```

**Behavior:**
1. Load materials from declared dependencies
2. Evaluate template (if specified)
3. Execute transformer:
   - **Container:** Build image, run container with materials mounted
   - **Template-only:** Render template to output
   - **Manual:** Evaluate template for guidance, no automated output
4. Validate outputs (check existence, schema if specified)
5. Create run record in `.graft/runs/`

**For manual workflows:**
- Template is evaluated and persisted (if `persist: text`)
- Human or agent edits outputs directly
- No automated transformation

**Use case:**
- Generate or regenerate artifact outputs
- See evaluated template for manual work
- Test transformer changes

---

### `graft status <ARTIFACT> [OPTIONS]`

Show artifact status: whether materials changed, downstream impact, what action is needed.

**Arguments:**
- `ARTIFACT` — Path to artifact directory

**Options:**
- `--json` — Output structured JSON
- `--sync <policy>` — Override sync policy (default: `warn`)

**Example:**
```bash
graft status artifacts/sprint-brief/
graft status artifacts/sprint-brief/ --json
```

**Output (human-readable):**
```
Artifact: sprint-brief
Status: dirty (materials changed)

Materials changed:
  - sources/tickets.yaml (modified)

Downstream impact:
  - artifacts/roadmap/ (depends on sprint-brief)

Action needed:
  - Run derivation to regenerate from updated materials
  - Or manually propagate changes to outputs
  - Then finalize with agent attribution
```

**JSON Output:**
```json
{
  "artifact": "sprint-brief",
  "status": "dirty",
  "change_origin": "materials",
  "materials_changed": [
    {"path": "sources/tickets.yaml", "status": "modified"}
  ],
  "downstream": [
    {"artifact": "roadmap", "status": "will_be_dirty"}
  ],
  "orchestrator": {...}
}
```

**Use case:**
- Check if artifact needs attention
- See downstream impact before making changes
- Understand what changed and why

---

### `graft validate <ARTIFACT>`

Validate artifact configuration and outputs against schemas.

**Arguments:**
- `ARTIFACT` — Path to artifact directory

**Example:**
```bash
graft validate artifacts/sprint-brief/
```

**Behavior:**
1. Parse `graft.yaml` (syntax validation)
2. Check all declared materials exist
3. Validate outputs against schemas (if specified)
4. Report errors or confirm validity

**Output:**
```
Validating: sprint-brief
✓ Configuration valid
✓ Materials exist
✓ Output schema valid (brief.md matches sprint_brief schema)
Validation passed.
```

**Use case:**
- Catch configuration errors before running
- Ensure outputs match expected format
- CI checks before merging

---

### `graft finalize <ARTIFACT> [OPTIONS]`

Finalize an artifact: record provenance, create attestation, mark transaction complete.

**Arguments:**
- `ARTIFACT` — Path to artifact directory

**Options:**
- `--agent <name>` — Agent name for attestation (required if `attest: required`)
- `--role <role>` — Agent role: `human`, `agent`, `ci`, `model` (optional)
- `--model <model>` — Model name for AI agents (optional)
- `--params <json>` — Additional metadata as JSON (optional)
- `--sync <policy>` — Override sync policy (default: `warn`)

**Example:**
```bash
graft finalize artifacts/sprint-brief/ --agent "Jane Doe"
graft finalize artifacts/sprint-brief/ --agent "claude-sonnet-4" --role agent
graft finalize artifacts/sprint-brief/ --agent "CI" --role ci --params '{"build": "1234"}'
```

**Behavior:**
1. Verify latest run record exists
2. Validate all outputs exist and match policy
3. Create provenance record in `.graft/provenance/<derivation-id>.json`
4. Record attestation (agent, role, timestamp)
5. Mark artifact as "clean"

**Provenance includes:**
- All materials (paths, hashes, git refs)
- Template (source, evaluated hash)
- Transformer (image digest, parameters)
- Outputs (paths, hashes)
- Attestation (agent, role, timestamp)
- Policy (deterministic, attest, direct_edit)

**Use case:**
- Complete transaction after manual edits
- Capture attribution for automated runs
- Create audit trail

---

### `graft impact <ARTIFACT> [OPTIONS]`

Show downstream impact: what other artifacts depend on this one.

**Arguments:**
- `ARTIFACT` — Path to artifact directory

**Options:**
- `--json` — Output structured JSON
- `--sync <policy>` — Override sync policy (default: `warn`)

**Example:**
```bash
graft impact artifacts/sprint-brief/
graft impact artifacts/sprint-brief/ --json
```

**Output:**
```
Artifact: sprint-brief
Downstream artifacts:
  - artifacts/roadmap/ (direct dependency)
  - artifacts/exec-summary/ (transitive via roadmap)
```

**JSON Output:**
```json
{
  "artifact": "sprint-brief",
  "downstream": [
    {
      "artifact": "roadmap",
      "relationship": "direct"
    },
    {
      "artifact": "exec-summary",
      "relationship": "transitive",
      "via": ["roadmap"]
    }
  ],
  "orchestrator": {...}
}
```

**Use case:**
- Before finalizing, see what will become dirty
- Understand dependency graph
- Plan batch updates

---

### `graft simulate <ARTIFACT> [OPTIONS]`

Simulate running derivations without modifying files. Shows what would happen.

**Arguments:**
- `ARTIFACT` — Path to artifact directory

**Options:**
- `--cascade` — Simulate cascade through downstream artifacts
- `--sync <policy>` — Override sync policy (default: `warn`)

**Example:**
```bash
graft simulate artifacts/sprint-brief/
graft simulate artifacts/sprint-brief/ --cascade
```

**Output:**
```
Simulating: sprint-brief
Would run derivation: brief
Would generate: artifacts/sprint-brief/brief.md
Would not modify filesystem (simulation mode)

With --cascade:
Would trigger downstream:
  - artifacts/roadmap/ (materials would change)
  - artifacts/exec-summary/ (transitive impact)
```

**Use case:**
- Preview impact before running
- Test configuration changes
- Understand cascade effects

---

### `graft dvc scaffold [OPTIONS]`

Manage `dvc.yaml` stages for all Graft artifacts. Authoritative entrypoint for DVC orchestration.

**Options:**
- `--check` — Read-only mode: show drift without writing
- `--json` — Output structured JSON

**Modes:**

**Write mode (default):**
```bash
graft dvc scaffold
```

Scans for all `graft.yaml` files, generates or updates `dvc.yaml` with managed stages. Idempotent: safe to run repeatedly.

**Check mode:**
```bash
graft dvc scaffold --check
```

Shows drift plan without writing. Exit code `1` if drift exists, `0` if none.

**Output:**
```
Scanning for grafts...
Found 3 artifacts with 5 derivations

Drift detected:
  Create: 2 stages (new derivations)
  Update: 1 stage (dependencies changed)
  Remove: 0 stages

Plan:
  + graft:sprint-brief:brief
  + graft:roadmap:plan
  ~ graft:backlog:normalize

Writing dvc.yaml... done.
```

**JSON Output:**
```json
{
  "orchestrator": {
    "type": "dvc",
    "sync_policy": "apply",
    "drift": "mixed",
    "plan": {
      "create": ["graft:sprint-brief:brief", "graft:roadmap:plan"],
      "update": ["graft:backlog:normalize"],
      "remove": []
    },
    "applied": true
  }
}
```

**Use case:**
- Ensure `dvc.yaml` is current with graft configurations
- CI checks for drift
- Manual refresh after adding artifacts

---

## Sync Policy

The `--sync` flag controls DVC autosync behavior on commands that support it.

**Policies:**
- `off` — Never write `dvc.yaml`; show plan if drift exists
- `warn` — Never write; show plan; exit 0 (default for read-only commands)
- `apply` — Write `dvc.yaml` automatically if drift exists (default for write commands)
- `enforce` — Fail with exit code 1 if drift exists; don't write

**Per-command defaults:**

| Command | Default | Rationale |
|---------|---------|-----------|
| `graft run` | `apply` | Keep pipeline usable before/after runs |
| `graft init` | `apply` | New projects get working `dvc.yaml` |
| `graft dvc scaffold` | explicit | Authoritative write/check entrypoint |
| `graft explain` | `warn` | Read-only; can override with `--sync apply` |
| `graft status` | `warn` | Read-only; can override |
| `graft impact` | `warn` | Read-only |
| `graft simulate` | `warn` | Read-only |
| `graft finalize` | `warn` | Keep finalize focused on provenance |

**Override examples:**
```bash
# Force drift check to fail in CI
graft run artifacts/sprint-brief/ --sync enforce

# Apply sync even on read-only command
graft explain artifacts/sprint-brief/ --sync apply

# Disable all sync behavior
graft run artifacts/sprint-brief/ --sync off
```

---

## Common Workflows

### Initial Setup
```bash
# Create project
mkdir my-grafts && cd my-grafts
git init

# Initialize Graft
graft init

# Initialize DVC
dvc init

# Commit configuration
git add graft.config.yaml .dvc/
git commit -m "Initialize Graft + DVC"
```

### Creating and Running an Artifact
```bash
# Create artifact directory and configuration
mkdir -p artifacts/my-artifact
# ... create graft.yaml, template, etc. ...

# Run the derivation
graft run artifacts/my-artifact/

# Finalize
graft finalize artifacts/my-artifact/ --agent "Your Name"

# Commit
git add artifacts/my-artifact/
git commit -m "Add my-artifact"
```

### Handling Material Changes
```bash
# Update source materials
# ... edit sources/data.yaml ...
git add sources/ && git commit -m "Update data"

# Check status
graft status artifacts/my-artifact/

# Propagate update (run or manual edit)
graft run artifacts/my-artifact/

# Finalize
graft finalize artifacts/my-artifact/ --agent "Your Name"

# Commit
git add artifacts/my-artifact/
git commit -m "Update my-artifact for new data"
```

### Checking Drift Before CI Merge
```bash
# In CI pipeline
graft dvc scaffold --check
# Fails if dvc.yaml is out of sync

graft validate artifacts/*/
# Validates all artifacts

graft run artifacts/*/ --sync enforce
# Fails if drift detected
```

---

## Environment Variables

Graft respects these environment variables:

**DVC_ROOT** — Override detected repository root

**GRAFT_CONFIG** — Path to `graft.config.yaml` (default: `<repo-root>/graft.config.yaml`)

**Within containers (set by Graft):**

**GRAFT_ARTIFACT_DIR** — Absolute path to artifact directory

**GRAFT_MATERIALS** — JSON array of material file paths

**GRAFT_OUTPUTS** — JSON array of output file paths

**GRAFT_PARAMS** — JSON object of transformer parameters

**Example container usage:**
```python
import os, json

materials = json.loads(os.getenv("GRAFT_MATERIALS", "[]"))
outputs = json.loads(os.getenv("GRAFT_OUTPUTS", "[]"))
params = json.loads(os.getenv("GRAFT_PARAMS", "{}"))

# Read first material
with open(materials[0], 'r') as f:
    data = f.read()

# Process...

# Write first output
with open(outputs[0], 'w') as f:
    f.write(result)
```

---

## Debugging

**Verbose output:**
```bash
export GRAFT_LOG_LEVEL=DEBUG
graft run artifacts/my-artifact/
```

**Check DVC stage execution:**
```bash
dvc repro graft:my-artifact:derivation-id
```

**Inspect provenance:**
```bash
cat artifacts/my-artifact/.graft/provenance/derivation-id.json | jq .
```

**Check evaluated template:**
```bash
cat artifacts/my-artifact/.graft/evaluated/input.md
```

**Validate configuration:**
```bash
graft validate artifacts/my-artifact/
```

---

Next: See [graft.yaml Reference](graft-yaml-reference.md) for configuration schema.
