# Vertical Slices — Detailed Acceptance Criteria

## Slice 0 — Foundations
- Contract: `explain --json` returns `{ artifact, graft, policy?, inputs?, derivations[] }`.
- Errors: missing `graft.yaml` → CLI exit code != 0 with helpful message.
- Tests: `tests/test_explain_json.py`.

## Slice 1 — Deterministic single derivation
- Contract: `run` writes outputs; `run --id` targets one derivation.
- Failure: missing template file → non-zero exit.
- Tests: `tests/test_run_stub.py`.

## Slice 2 — Direct-edit + finalize/attest
- Contract: `status --json` returns change_origin (string), downstream[].
- Contract: `finalize` writes `.graft/provenance/finalize.json` with agent info if provided.
- Tests: `tests/test_status_and_finalize.py`.

## Slice 3 — Impact & simulate
- Contract: `impact --json` returns `{ artifact, downstream[] }`.
- Contract: `simulate --cascade` prints a confirmation line, no file changes.
- Tests: `tests/test_impact_and_simulate_stubs.py`.

## Slice 4 — DVC scaffold
- Contract: `dvc-scaffold .` writes `dvc.yaml` with one stage per artifact:
  - `cmd: graft run <artifact/>`
  - `deps:` includes materials and `graft.yaml`
  - `outs:` includes declared outputs
- Tests: `tests/test_dvc_scaffold.py`.

## Slice 5 — Ingest-as-files (demo only)
- Contract: example shows snapshot → backlog path; `run` produces backlog.yaml (stub).
