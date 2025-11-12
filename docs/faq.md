# Frequently Asked Questions

## General

### What is Graft?

Graft is a git-backed build tool for file-first data workflows. It binds together materials from different origins into artifacts that grow and adapt as your sources change. Every transformation is tracked with full provenance: what was read, what was written, who finalized, when, and under what policy.

### How is Graft different from Make/Bazel/other build tools?

Traditional build tools assume outputs are ephemeral artifacts (compiled binaries, cached results) that get regenerated and discarded. Graft treats outputs as **living files** that are versioned in git, directly editable, and evolved over time. Graft also adds provenance tracking and human-in-the-loop support that traditional build tools lack.

### How is Graft different from DVC?

DVC is an orchestration tool that manages dependency DAGs and caching for data pipelines. Graft **uses** DVC for orchestration but adds:
- Provenance tracking (complete audit trails)
- Policy enforcement (deterministic, attestation, direct edit)
- Human-in-the-loop primitives (finalize transaction, attribution)
- Template evaluation and container management

Think of it as: DVC handles execution, Graft handles auditability.

### How is Graft different from Airflow/Dagster?

Airflow and Dagster are heavyweight orchestrators for large-scale data pipelines (terabytes, complex dependencies, distributed execution). Graft is file-first and git-native, designed for medium-scale workflows (10-100 artifacts) where outputs are documents, configurations, or derived content that needs version control and provenance.

### When should I use Graft?

Use Graft when:
- You have data transformations that produce files you want to version
- You need audit trails (compliance, research, reproducibility)
- Humans or AI agents need to review/refine automated outputs
- You want to compose workflows across organizational boundaries
- Outputs are documents, configs, or data files (not compiled code)

Don't use Graft for:
- Traditional software builds (use Make, Bazel)
- Real-time or streaming pipelines
- Terabyte-scale ETL (use Airflow, Spark)
- Simple one-off LLM generations

## Installation and Setup

### How do I install Graft?

```bash
pip install graft
```

Or with uv:
```bash
uv pip install graft
```

Requires Python 3.14+.

### Do I need Docker?

Docker is required if you use **container-based transformers** (derivations with `transformer.build`). If you only use template-based or manual derivations, Docker is optional.

### Do I need to know DVC?

Not really. Graft commands (`graft run`, `graft finalize`, etc.) are the primary interface. DVC runs under the hood for orchestration. For advanced use cases, you can use DVC directly (`dvc repro`, `dvc dag`), but it's optional.

### How do I initialize a Graft project?

```bash
mkdir my-project
cd my-project
git init
graft init
```

This creates `graft.config.yaml` with defaults. Then create your first artifact directory with `graft.yaml`.

## Workflows

### What's the difference between "run" and "finalize"?

**`graft run`** executes the transformation (template rendering or container execution) to produce outputs.

**`graft finalize`** completes the transaction: records provenance (what materials, what outputs, who finalized), creates attestation, marks the artifact as "clean."

For automated workflows, these can happen together. For human-in-the-loop workflows, you run (to generate/guide), then edit outputs manually, then finalize (to capture your work).

### Can I edit generated files?

Yes! That's the point of `policy.direct_edit: true`. Graft produces files you can edit directly with any tool. When you finalize, Graft captures that you edited them.

### What happens if I re-run after editing?

Depends on the transformer:
- **Template-only derivations:** Re-running overwrites your edits (template regenerates the file)
- **Smart transformers:** Can preserve editable sections (if transformer is designed to)
- **Manual derivations (no transformer):** Re-running just evaluates the template for guidance, doesn't overwrite outputs

Design your workflow accordingly: if you want to preserve edits, use a transformer that merges intelligently, or use manual propagation (edit the file yourself, no re-run).

### How do I handle upstream changes?

1. Upstream materials change (files in `inputs.materials`)
2. `graft status` shows artifact is dirty
3. You propagate updates:
   - **Option A:** `graft run` to regenerate
   - **Option B:** Manually edit outputs based on changes
4. `graft finalize` to capture the update

### Can I have multiple derivations in one artifact?

Yes. An artifact can have multiple derivations, each producing different outputs from the same materials:

```yaml
derivations:
  - id: summary
    transformer: {...}
    outputs: [{ path: "./summary.md" }]

  - id: detailed
    transformer: {...}
    outputs: [{ path: "./detailed.md" }]
```

## Provenance and Policy

### What's recorded in provenance?

Complete audit trail:
- **Materials:** Paths, hashes, git revisions
- **Transformer:** Container image digest, parameters
- **Template:** Source, evaluated template hash
- **Outputs:** Paths, hashes
- **Attestation:** Agent name, role, timestamp
- **Policy:** Deterministic, attest, direct_edit flags

Provenance lives in `.graft/provenance/<derivation-id>.json`.

### What's the purpose of attestation?

Attestation captures **who** reviewed and approved the transformation. This is critical for:
- Compliance (regulatory requirements)
- Research (reproducibility, attribution)
- Collaboration (know who made decisions)
- Trust (verify human oversight)

### What does `policy.deterministic` mean?

`deterministic: true` means: given the same inputs, the transformation always produces the same outputs. This enables:
- Reproducibility (re-run and verify)
- Caching (DVC can cache deterministic results)
- Debugging (predictable behavior)

Use `deterministic: false` for transformers that call external APIs, use randomness, or are otherwise non-deterministic.

### What does `policy.attest: required` do?

It requires that you provide `--agent` when finalizing:

```bash
graft finalize artifacts/my-artifact/ --agent "Jane Doe"
```

If you try to finalize without `--agent`, it fails. This enforces accountability.

### What does `policy.direct_edit: true` mean?

It means outputs can be manually edited. Graft expects this and won't treat manual edits as errors. When `direct_edit: false`, manual edits are policy violations (detected by `graft status`).

## Container Transformers

### How do container transformers work?

Graft builds a Docker image from your Dockerfile, then runs a container with:
- Materials mounted as volumes
- Environment variables: `GRAFT_MATERIALS`, `GRAFT_OUTPUTS`, `GRAFT_PARAMS`
- Container writes to output paths

Your transformer script reads materials, processes them, writes outputs.

### Do I need to manage Docker images?

No. Graft builds images automatically from the `Dockerfile` in your artifact directory. Images are tagged with `graft-<artifact>:local`.

### Can I use pre-built images?

Not yet (current slice uses local builds only). Future: reference remote OCI images with digest pinning.

### What if Docker build fails?

`graft run` exits with code 1 (user error). Check Docker build logs, fix Dockerfile, try again.

### What if the container fails?

`graft run` exits with code 1. Check container logs for errors in your transformer script.

## DVC Integration

### Do I need to run `dvc init`?

Yes, if you want DVC orchestration. Run:

```bash
dvc init
graft dvc scaffold
```

This initializes DVC and generates `dvc.yaml` from your grafts.

### What is autosync?

Autosync keeps `dvc.yaml` synchronized with your `graft.yaml` files. When you add/modify/remove grafts, Graft detects "drift" and can automatically update `dvc.yaml`.

Controlled by `--sync` flag:
- `apply` — Write dvc.yaml automatically (default for `graft run`)
- `warn` — Show drift but don't write (default for read-only commands)
- `enforce` — Fail if drift exists
- `off` — Never write

### Can I edit `dvc.yaml` manually?

You can, but Graft will overwrite managed stages (those starting with `graft:`). Non-managed stages are preserved. Best practice: let Graft manage `dvc.yaml`, use `dvc.yaml` for hand-written stages with different prefixes.

### How do I use DVC for orchestration?

```bash
# Run entire pipeline
dvc repro

# Run specific stage
dvc repro graft:my-artifact:my-derivation

# See dependency graph
dvc dag

# Check what's dirty
dvc status
```

## Remote Materials

### Can I reference files from other repositories?

Yes:

```yaml
inputs:
  materials:
    - path: "https://github.com/org/repo/raw/v1.0/data.json"
      rev: v1.0
```

Graft fetches the file at the specified revision, records it in provenance.

### How do I pin versions?

Use git tags or commit SHAs in `rev`:

```yaml
- path: "https://github.com/org/repo/raw/v1.2.3/data.json"
  rev: v1.2.3
```

This ensures reproducibility: provenance records exact version used.

### Can I reference private repositories?

Not directly via HTTPS (requires auth). Workarounds:
- Use SSH URLs with configured keys
- Clone the repo locally, reference as local path
- Use git submodules

Future: support for authenticated remote fetches.

## AI Agents

### How do AI agents use Graft?

AI agents (like Claude Code) can:
1. Call `graft explain --json` to understand artifact configuration
2. See evaluated templates (what changed in materials)
3. Edit outputs directly
4. Call `graft finalize --agent "claude-sonnet-4" --role agent` to record their work

Provenance captures: agent name, model version, what it accessed, what it changed, when.

### How do I review AI agent changes?

Same as human changes: in a PR. Agent finalizes, commits, opens PR. You review the diff, check provenance, approve or request changes.

### Can I control which artifacts AI agents can auto-merge?

Yes, via policy:
- Low-stakes artifacts: `attest: optional`, allow auto-finalize
- High-stakes artifacts: `attest: required`, require human PR review (via GitHub branch protection)

Policy is declarative in `graft.yaml`, not hidden in code.

## Troubleshooting

### "graft.yaml not found"

You're either:
- Running `graft run` from wrong directory
- Missing `graft.yaml` in artifact directory

Check path: `graft run artifacts/my-artifact/` (must point to directory containing `graft.yaml`).

### "Materials not found"

Material paths are relative to repository root. Verify:
- Paths are correct
- Files are committed to git (if `rev: HEAD`)
- Git rev exists (if using specific commits/tags)

### "Docker build failed"

Check:
- Docker is running
- Dockerfile syntax is valid
- Build context is correct (usually `context: "."`)
- Dependencies available

Run manually to debug:
```bash
docker build -t test-image -f artifacts/my-artifact/Dockerfile artifacts/my-artifact/
```

### "Finalize failed: outputs don't match"

If `direct_edit: false`, you may have accidentally edited outputs. Re-run `graft run` to regenerate.

If materials changed since run, finalize expects outputs to match the latest run. Propagate the update first.

### "DVC errors"

Check:
- `dvc init` was run
- `dvc.yaml` is valid (run `graft dvc scaffold --check`)
- DVC is installed (`dvc version`)

### "Permission denied" errors

Check file permissions, especially:
- `.graft/` directory
- Output file paths
- Material file paths

Run `chmod` or `chown` as needed.

## Performance

### Is Graft slow for large files?

Graft itself is fast (Python, simple file operations). Slowness usually comes from:
- Docker builds (use layer caching, multi-stage builds)
- Large material files (use `.gitignore` for truly large files, DVC remotes for versioned large files)
- Many artifacts (100+ may slow autosync)

Optimize by:
- Minimizing Docker rebuild (cache layers)
- Using DVC remotes for large files
- Splitting large artifacts into smaller ones

### Can Graft run transformations in parallel?

DVC handles parallelization. Independent derivations run concurrently via `dvc repro`. Graft doesn't parallelize directly.

## Comparison to Other Tools

### vs. Jupyter Notebooks

Jupyter: Interactive notebooks mixing code, outputs, visualizations. Great for exploratory data science.

Graft: Batch-oriented transformations with provenance and policy. Better for production workflows, audit trails, multi-stage pipelines.

### vs. Quarto/R Markdown

Quarto: Literate programming, document generation from code + markdown.

Graft: More general (any transformation), supports containers, provenance tracking, human-in-the-loop workflows.

### vs. dbt

dbt: SQL transformations for analytics, data warehouses.

Graft: Broader (not just SQL), file-first (not database-first), supports any transformation (templates, containers, manual).

### vs. Notion/Confluence

Notion/Confluence: Wiki-style documentation, manual editing.

Graft: Derived documentation (generated from sources), provenance-tracked, version-controlled in git.

## Future Features

### Will Graft support LLM transformers natively?

Planned. Future slice: `transformer.llm` with prompt management, caching, cost tracking, model versioning.

### Will Graft support remote execution?

Possibly. Running transformers on cloud workers (Lambda, K8s) while preserving provenance is on the roadmap.

### Will Graft support signed provenance?

Yes. Cryptographic signatures (GPG, etc.) for tamper-evident audit trails are planned.

### Will Graft support other orchestrators?

Architecture supports it (protocol-based adapters). Airflow, Dagster backends could be added if there's demand.

## Getting Help

### Where can I ask questions?

- GitHub issues: https://github.com/your-org/graft/issues
- Documentation: https://docs.graft.dev
- Examples: `examples/` directory in the repo

### How do I report bugs?

Open a GitHub issue with:
- Graft version (`graft --version`)
- Python version (`python --version`)
- Platform (OS, Docker version if relevant)
- Minimal reproduction (graft.yaml, commands run, error output)

### How do I contribute?

See [Implementation Strategy](implementation-strategy.md) for development practices and contribution guidelines.

---

Still have questions? Open an issue or check the full documentation!
