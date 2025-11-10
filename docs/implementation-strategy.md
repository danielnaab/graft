# Implementation Strategy (vertical slices)

Deliver narrow slices; each slice ships tests, docs, and CLI behavior.

## Slice 0 — Foundations (CLI + schemas)
- CLI boots; `graft explain` prints merged config (local-only merge for starter).
- Schemas: `schemas/graft.schema.json`, `schemas/policy.schema.json`.
- Acceptance criteria:
  - Explain emits JSON with artifact path, graft id, derivation ids.
  - Invalid artifact directory returns an error.

## Slice 1 — Deterministic single derivation
- `graft run` renders template → outputs (stub copies template to outputs).
- Acceptance criteria:
  - Output file exists and contains the template content.
  - Exit status 0; friendly errors if template missing.

## Slice 2 — Direct-edit + finalize/attest
- `graft status/validate/finalize` classify changes and write provenance stub.
- Acceptance criteria:
  - Finalize writes `.graft/provenance/finalize.json` with agent/model if provided.
  - Status returns `--json` contract (even if classification is “unknown” in the stub).

## Slice 3 — Impact & simulate
- `graft impact` lists downstream placeholders; `simulate --cascade` prints action.
- Acceptance criteria:
  - Commands exist and return valid JSON/text without side effects.

## Slice 4 — DVC scaffold
- `graft dvc-scaffold` writes `dvc.yaml` stages mapping artifacts to `graft run`.
- Acceptance criteria:
  - `dvc.yaml` contains stages with `cmd`, `deps`, `outs` constructed from `graft.yaml`.

## Slice 5 — Ingest-as-files flow (demo)
- Example shows snapshot → backlog path; `run` produces backlog.yaml (stub).
