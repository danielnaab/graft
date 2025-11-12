# DVC Integration — Seamful Autosync

Graft keeps `dvc.yaml` in sync with graft derivations so `dvc repro` works seamlessly, but it does so **seamfully**: always showing the plan, preserving non-Graft stages, and letting users control when writes happen.

## Goals

- Minimize manual DVC management while keeping the seam visible
- Make `dvc repro graft:<artifact>:<derivation>` work out of the box
- Keep responsibilities clean: Graft enforces policy/provenance; DVC handles orchestration/cache

## Non-goals (this slice)

- Running `dvc repro`, managing remotes/caches, or touching `dvc.lock`
- Modeling finalize/attestation as DVC stages

## Configuration

Add an `orchestrator` section to `graft.config.yaml` (at repo root):

```yaml
version: 1
orchestrator:
  type: dvc                    # only 'dvc' in this slice
  managed_stage_prefix: "graft:"
  sync_policy: apply           # off | warn | apply | enforce
  roots: ["."]                 # optional: directories to scan for artifacts
```

### Sync Policies

- **`off`**: Never write; show plan if drift exists
- **`warn`**: Never write; show plan; exit 0
- **`apply`** (default): Write the plan automatically, print concise summary, proceed
- **`enforce`**: Fail if drift exists (print plan); no write

### Per-command defaults

| Command | Default sync | Notes |
|---------|--------------|-------|
| `graft run` | `apply` | Keep pipeline usable before/after runs |
| `graft init` | `apply` | New projects get a working dvc.yaml |
| `graft dvc scaffold` | explicit | Authoritative write/check entrypoint |
| `graft explain` | `warn` | Read-only; can override with `--sync apply` |
| `graft status` | `warn` | Read-only; can override with `--sync apply` |
| `graft impact` | `warn` | Read-only; can override with `--sync apply` |
| `graft simulate` | `warn` | Read-only; can override with `--sync apply` |
| `graft finalize` | `warn` | Keep finalize focused on provenance |

Override with `--sync off|warn|apply|enforce` on any command.

## Stage Mapping

**One stage per derivation** with canonical naming: `graft:<artifact-name>:<derivation-id>`

Example for an artifact with two derivations:

```yaml
stages:
  graft:sprint-brief:default:
    wdir: artifacts/sprint-brief
    cmd: graft run artifacts/sprint-brief --id default
    deps:
      - materials/backlog.json
      - artifacts/sprint-brief/graft.yaml
      - artifacts/sprint-brief/template.md.j2
    outs:
      - artifacts/sprint-brief/sprint-brief.md

  graft:sprint-brief:v2:
    wdir: artifacts/sprint-brief
    cmd: graft run artifacts/sprint-brief --id v2
    deps:
      - materials/backlog.json
      - artifacts/sprint-brief/graft.yaml
      - artifacts/sprint-brief/template-v2.md.j2
    outs:
      - artifacts/sprint-brief/sprint-brief-v2.md
```

### Dependency tracking

For each derivation, the stage includes:

- All `inputs.materials[].path`
- `<artifact-dir>/graft.yaml`
- Template file (if `template.source: file`)
- `<artifact-dir>/Dockerfile` (if `transformer.build` present)

**Note on Docker build context:** Only the Dockerfile itself is tracked as a dependency, not all files in the build context. This is intentional to keep DVC stages performant and avoid spurious reruns. Graft's run records (`.graft/provenance/`) remain the authoritative source of truth for all inputs.

> **TODO (future enhancement):** Revisit Docker build context dependency tracking. Consider:
> - Hashing build context directories
> - Parsing Dockerfile COPY/ADD statements
> - Configurable include/exclude patterns
> - Balance between precision and performance

### Path normalization

- `dvc.yaml` lives at repo root (detected via `git rev-parse --show-toplevel`)
- All paths are relative to repo root with POSIX separators (`/`)
- Works consistently across OSes

## Drift Detection

Drift exists when:

- **Missing managed stage**: Derivation exists but no corresponding DVC stage
- **Mismatched spec**: Stage exists but `cmd`, `wdir`, `deps`, or `outs` differ from canonical
- **Orphaned stage**: Managed stage exists for a derivation that no longer exists
- **Name mismatch**: Stage name differs from canonical (treated as remove + create)

## Autosync Behavior

On each trigger (command execution):

1. **Discover** graft derivations under configured `roots`
2. **Plan** drift: `{ create: [...], update: [...], remove: [...] }`
3. **Apply** per `sync_policy`:
   - `off`/`warn`: Don't write; show plan (included in `--json` output)
   - `apply`: Write atomically, print concise summary (e.g., "Autosync: create=1, update=0, remove=0")
   - `enforce`: If drift exists, fail with `E_ORCH_DRIFT_ENFORCED`; show plan; don't proceed

If `dvc.yaml` is invalid/unmergeable → `E_DVC_YAML_INVALID` with guidance to run `graft dvc scaffold --check`.

### Non-managed stages

Graft **only** touches stages whose names start with `managed_stage_prefix` (default: `graft:`). All other stages are preserved verbatim, allowing hand-written DVC stages to coexist.

## Commands

### `graft dvc scaffold [--check] [--json]`

Authoritative entrypoint for managing `dvc.yaml`.

**Write mode** (default):
```bash
graft dvc scaffold
# Writes/refreshes dvc.yaml with all managed stages
# Idempotent: safe to run repeatedly
```

**Check mode**:
```bash
graft dvc scaffold --check
# Read-only: shows drift plan without writing
# Exit code 1 if drift exists, 0 if none
```

**JSON output**:
```bash
graft dvc scaffold --json
# Returns plan and status in machine-readable format
```

### Autosync on other commands

Most commands autosync as a side effect:

```bash
# Apply policy (write if drift exists)
graft run artifacts/sprint-brief/

# Warn policy (show drift, don't write)
graft explain artifacts/sprint-brief/ --json

# Override to enforce (fail if drift)
graft run artifacts/sprint-brief/ --sync enforce
```

## JSON Output

All autosyncing commands with `--json` include an `orchestrator` block:

```json
{
  "artifact": "...",
  "orchestrator": {
    "type": "dvc",
    "sync_policy": "apply",
    "drift": "none|missing_stages|extra_stages|mixed",
    "plan": {
      "create": [...],
      "update": [...],
      "remove": [...]
    },
    "applied": true
  }
}
```

## Error Codes

- **`E_ORCH_DRIFT_ENFORCED`**: Drift present under `enforce` policy
- **`E_DVC_YAML_INVALID`**: Cannot parse/merge `dvc.yaml`
- **`E_STAGE_NAME_COLLISION`**: Two derivations resolve to same stage name
- **`E_WRITE_FAILED`**: Atomic write failed

## Graceful Degradation

DVC is a required dependency and will be installed with Graft. However, if the `.dvc/` directory is missing (i.e., `dvc init` has not been run):
- Autosync degrades to `warn` behavior
- Primary command proceeds (e.g., `graft run` still works)
- Guidance message shown suggesting running `dvc init`

## Recommended Workflow

### Initial setup

```bash
# Install graft (includes DVC)
pip install graft

# Create graft project
graft init

# Configure orchestrator (graft.config.yaml already includes orchestrator section)
# Edit if you want to change sync_policy or other settings

# Initialize DVC (creates .dvc/ directory and .dvc/config)
dvc init

# Configure DVC remotes (optional)
dvc remote add -d myremote s3://my-bucket/path

# Sync creates dvc.yaml with all graft stages
graft dvc scaffold
```

### Development workflow

```bash
# Graft automatically keeps dvc.yaml in sync
graft run artifacts/my-artifact/

# Use DVC for orchestration
dvc repro graft:my-artifact:default

# Push outputs to remote
dvc push
```

### CI/CD workflow

```bash
# Check for drift (fail if present)
graft dvc scaffold --check

# Or use enforce mode on run
graft run artifacts/my-artifact/ --sync enforce
```

## Design Notes

### Why seamful?

- **Visibility**: Users see when/why dvc.yaml changes
- **Control**: Explicit policies and overrides
- **Coexistence**: Preserves hand-written stages
- **Auditability**: Plans are shown and logged

### Why DVC-first?

- DVC is the de facto standard for data pipeline orchestration
- Proven caching and remote storage
- Graft focuses on provenance and policy enforcement
- Clean separation of concerns

### Why one stage per derivation?

- **Granular caching**: Only rerun affected derivations
- **Parallelism**: DVC can run independent derivations concurrently
- **Aligns with Graft's design**: Derivations are atomic units

## Limitations & Future Work

- **Build context tracking**: Only Dockerfile tracked, not all context files (see note above)
- **Comments in dvc.yaml**: Autosync may not preserve comments; use separate docs
- **Multiple orchestrators**: Only DVC supported; interface designed for future backends
- **Performance**: Large repos with many artifacts may need optimization
