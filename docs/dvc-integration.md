# DVC Integration

Graft uses DVC (Data Version Control) for orchestration: managing the dependency DAG, running transformations incrementally, and parallelizing independent work.

This document explains how Graft and DVC work together, when to use DVC directly, and how the autosync mechanism keeps everything in sync.

## Why DVC?

DVC is a mature, well-tested tool for data pipeline orchestration. Rather than build our own DAG executor, Graft leverages DVC for:

**Dependency tracking** — DVC understands which artifacts depend on which materials.

**Incremental execution** — Only run what changed, skip what's current.

**Parallelization** — Run independent derivations concurrently.

**Caching** — Cache deterministic outputs (though Graft uses git, not DVC cache, for outputs).

**Standard tooling** — Teams already know DVC, documentation exists, community support available.

Graft focuses on what's unique: provenance, policy, attestation. DVC handles execution.

## How It Works

### Graft Generates `dvc.yaml`

Graft scans your repository for `graft.yaml` files and generates a `dvc.yaml` file with DVC stages.

**One stage per derivation:**
```yaml
stages:
  graft:sprint-brief:brief:
    wdir: artifacts/sprint-brief
    cmd: graft run artifacts/sprint-brief/ --id brief
    deps:
      - ../../sources/tickets.yaml
      - graft.yaml
      - template.md
    outs:
      - brief.md:
          cache: false
```

**Stage naming:** `graft:<artifact-name>:<derivation-id>`

**Dependencies:** All materials, graft.yaml, template files, Dockerfiles (if present).

**Outputs:** Files produced by the derivation.

**Cache: false** — Outputs live in git, not DVC cache.

### DVC Executes the DAG

When materials change:
1. DVC detects which stages are dirty (dependencies changed)
2. Runs `graft run` for each dirty derivation
3. Parallelizes independent stages
4. Outputs are produced, committed to git

### Provenance is Created by Finalize

`graft run` produces outputs. `graft finalize` creates provenance.

For automated workflows (policy allows auto-finalize), this can happen in the same step.

For manual workflows, human/agent finalizes after reviewing/editing outputs.

## Autosync Mechanism

Graft keeps `dvc.yaml` synchronized with your `graft.yaml` files via **seamful autosync**.

**Seamful means:**
- You see when sync happens
- You control when writes happen (via sync policy)
- Graft shows the plan before applying
- You can override per-command

### Sync Policies

Configured in `graft.config.yaml`:

```yaml
orchestrator:
  type: dvc
  managed_stage_prefix: "graft:"
  sync_policy: apply  # Default policy
```

**Policies:**
- `off` — Never write dvc.yaml; show plan if drift exists
- `warn` — Show plan, don't write, exit 0 (default for read-only commands)
- `apply` — Write dvc.yaml automatically if drift detected (default for write commands)
- `enforce` — Fail if drift detected; don't write

**Per-command defaults:**
- `graft run`: `apply` (keep pipeline usable)
- `graft dvc scaffold`: explicit write or check mode
- `graft explain`, `graft status`: `warn` (read-only, no side effects)

**Override:**
```bash
graft run artifacts/sprint-brief/ --sync enforce  # Fail if drift
graft explain artifacts/sprint-brief/ --sync apply  # Force sync
```

### Drift Detection

Drift exists when:
- **Missing stage:** Derivation exists but no DVC stage
- **Mismatched spec:** Stage exists but deps/outs/cmd differ
- **Orphaned stage:** Managed stage for non-existent derivation
- **Name changed:** Stage name doesn't match canonical

### Autosync Workflow

On each command (that supports `--sync`):
1. Scan for graft.yaml files
2. Compare to current dvc.yaml
3. Compute drift plan: `{create: [...], update: [...], remove: [...]}`
4. Apply per sync policy:
   - `off`/`warn`: Don't write, show plan
   - `apply`: Write dvc.yaml, print summary
   - `enforce`: Fail if drift exists

**Example output (apply mode):**
```
Autosync: create=1, update=0, remove=0
Writing dvc.yaml... done.
```

## Using DVC Directly

You can use DVC commands for advanced workflows:

### Run the Full Pipeline

```bash
dvc repro
```

Runs all dirty stages in dependency order.

### Run a Specific Stage

```bash
dvc repro graft:sprint-brief:brief
```

### Visualize the DAG

```bash
dvc dag
```

Shows dependency graph.

### Check Status

```bash
dvc status
```

Shows which stages are dirty.

### Manage Remote Storage (Optional)

For large files, configure DVC remote:

```bash
dvc remote add -d myremote s3://my-bucket/path
dvc push
dvc pull
```

Graft artifacts are typically small (documents, configs), so DVC remotes are optional.

## Non-Managed Stages

Graft only touches stages whose names start with `managed_stage_prefix` (default: `graft:`).

All other stages in `dvc.yaml` are preserved verbatim. This allows:
- Hand-written DVC stages to coexist
- External pipelines managed separately
- Gradual migration to Graft

**Example `dvc.yaml` with mixed stages:**
```yaml
stages:
  # Graft-managed stages
  graft:sprint-brief:brief:
    cmd: graft run artifacts/sprint-brief/ --id brief
    ...

  # Hand-written stage (preserved)
  preprocess-data:
    cmd: python scripts/preprocess.py
    deps: [raw-data.csv]
    outs: [clean-data.csv]
```

Graft won't modify `preprocess-data`.

## Configuration

In `graft.config.yaml`:

```yaml
version: 1
orchestrator:
  type: dvc                     # Only 'dvc' supported currently
  managed_stage_prefix: "graft:"  # Prefix for managed stages
  sync_policy: apply            # Default sync behavior
  roots: ["."]                  # Directories to scan for artifacts
```

**`type`** — Orchestrator type (only `dvc` in current implementation).

**`managed_stage_prefix`** — Graft only modifies stages with this prefix.

**`sync_policy`** — Default policy for autosync.

**`roots`** — List of directories to scan for `graft.yaml` files.

## Commands

### `graft dvc scaffold`

Authoritative command for managing `dvc.yaml`.

**Write mode (default):**
```bash
graft dvc scaffold
```

Generates or updates `dvc.yaml` with all managed stages. Idempotent.

**Check mode:**
```bash
graft dvc scaffold --check
```

Read-only: shows drift without writing. Exit code 1 if drift, 0 if clean.

**JSON output:**
```bash
graft dvc scaffold --json
```

Machine-readable output with drift plan.

### Autosync on Other Commands

Most commands autosync as a side effect:

```bash
graft run artifacts/sprint-brief/     # Default: apply
graft explain artifacts/sprint-brief/ # Default: warn
```

Override with `--sync`:
```bash
graft run artifacts/sprint-brief/ --sync off     # No sync
graft explain artifacts/sprint-brief/ --sync apply  # Force sync
```

## Recommended Workflow

### Initial Setup

```bash
# Initialize Graft
graft init

# Initialize DVC
dvc init

# Generate dvc.yaml
graft dvc scaffold

# Commit
git add graft.config.yaml .dvc/ dvc.yaml
git commit -m "Initialize Graft + DVC"
```

### Development

```bash
# Run Graft commands (autosync keeps dvc.yaml current)
graft run artifacts/my-artifact/

# Use DVC for orchestration
dvc repro

# Push large files to remote (if configured)
dvc push
```

### CI/CD

```bash
# Check for drift (fail if present)
graft dvc scaffold --check

# Or use enforce mode on run
graft run artifacts/my-artifact/ --sync enforce
```

## Limitations and Future Work

**Docker build context tracking** — Only the Dockerfile itself is tracked as a dependency, not all files in the build context. This keeps DVC stages performant but may miss some changes. Graft's run records remain the source of truth for all inputs.

**Comments in dvc.yaml** — Autosync may not preserve comments. Use separate documentation for explaining stages.

**Performance with many artifacts** — Large repos with 100+ artifacts may see slow autosync. Future: optimize scanning and diffing.

**Multiple orchestrators** — Only DVC supported currently. Architecture supports future backends (Airflow, Dagster).

## Troubleshooting

**"DVC not found"**
- Ensure DVC is installed: `dvc version`
- Graft installs DVC as dependency, but check installation

**"dvc.yaml is invalid"**
- Run `graft dvc scaffold --check` to diagnose
- Check for manual edits that broke YAML syntax
- Regenerate: `graft dvc scaffold`

**"Stage already exists"**
- Name collision with non-managed stage
- Change artifact name or derivation ID
- Or change `managed_stage_prefix` in config

**"Drift detected in enforce mode"**
- Run `graft dvc scaffold` to sync
- Or fix graft.yaml configuration
- Or change sync policy to `apply`

**"DVC stages not running"**
- Ensure dvc.yaml exists: `graft dvc scaffold`
- Check DVC status: `dvc status`
- Run manually: `dvc repro`

## Advanced: Custom Orchestrators

Graft's architecture supports other orchestrators via the `OrchestratorPort` protocol.

To add Airflow support:
1. Implement `AirflowAdapter` with `scaffold()`, `detect_drift()` methods
2. Update `graft.config.yaml` schema to support `type: airflow`
3. Generate Airflow DAG Python files instead of `dvc.yaml`

This is future work, but the interface is designed for it.

---

**Summary:** DVC handles execution, Graft handles provenance. Autosync keeps them in sync. Use Graft commands for everyday work, DVC commands for advanced orchestration.

Next: See [Testing Strategy](testing-strategy.md) for development practices.
