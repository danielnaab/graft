# Implementation Strategy (vertical slices)

Deliver narrow slices; each slice ships tests, docs, and CLI behavior.

## Slice 0 — Foundations (CLI + schemas)
- CLI boots; `graft explain` prints merged config (local-only merge for starter).
- Schemas: `schemas/graft.schema.json`, `schemas/policy.schema.json`, `schemas/cli/explain.schema.json`.
- Acceptance criteria:
  - Explain emits JSON with artifact path, graft name, policy, inputs, and full derivation objects (id, transformer, outputs, template, policy).
  - Default output (without --json) is human-readable summary format.
  - Invalid artifact directory returns exit code 1 with helpful error message including path.
  - Malformed YAML returns exit code 1 with parse error.
  - Missing required fields return exit code 1.
  - System errors (permissions, etc.) return exit code 2.

## Slice 1 — Deterministic single derivation (template rendering)
- `graft run` renders Jinja2 templates → outputs.
- Acceptance criteria:
  - Output file exists and contains rendered template content.
  - Exit status 0; friendly errors if template missing.
  - `--id` flag targets specific derivation.
  - Supports multiple outputs per derivation.
  - Creates nested output directories as needed.

## Slice 2 — Container transformers (minimal)
- `graft run` builds Docker images and executes containers to transform data.
- Acceptance criteria:
  - Local Dockerfile builds with `transformer.build` specification.
  - Materials mounted into container at `/workspace`.
  - Environment variables provide paths and params.
  - All declared outputs written by container.
  - Both examples functional: sprint-brief (templates) and backlog (container transformation).
  - Missing Dockerfile, build failures, or missing outputs → exit code 1.
- Implementation:
  - Docker adapter for building and running containers.
  - Material loading adapter.
  - Simple file/env interface (no stdin/stdout yet).
  - Production features (caching, logging, multi-backend) deferred to Slice 6.

## Slice 3 — Direct-edit + finalize/attest
- `graft status/validate/finalize` classify changes and write provenance.
- Acceptance criteria:
  - Finalize writes `.graft/provenance/finalize.json` with agent/model if provided.
  - Status returns `--json` contract (even if classification is "unknown" in the stub).
  - Works for both template-based and transformer-based derivations.

## Slice 4 — Impact & simulate
- `graft impact` lists downstream placeholders; `simulate --cascade` prints action.
- Acceptance criteria:
  - Commands exist and return valid JSON/text without side effects.

## Slice 5 — DVC scaffold
- `graft dvc-scaffold` writes `dvc.yaml` stages mapping artifacts to `graft run`.
- Acceptance criteria:
  - `dvc.yaml` contains stages with `cmd`, `deps`, `outs` constructed from `graft.yaml`.
  - Works for both template and transformer derivations.

## Slice 6 — Container-based transformers (future)
- Local Dockerfile builds with `transformer.build` specification.
- Network and determinism policies enforced at container level.
- Run records capture build inputs and image digests.
- See ChatGPT design document for detailed specification.

## Slice 7 — Advanced features (future)
- Remote transformer refs (`transformer.ref` pointing to registries).
- Advanced ingest-as-files workflows.
- Multi-backend runtime support.
