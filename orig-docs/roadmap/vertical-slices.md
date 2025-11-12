# Vertical Slices — Detailed Acceptance Criteria

## Slice 0 — Foundations
- Contract: `explain --json` returns `{ artifact, graft, policy?, inputs?, derivations[] }`.
- Errors: missing `graft.yaml` → CLI exit code != 0 with helpful message.
- Tests: `tests/test_explain_json.py`.

## Slice 1 — Deterministic single derivation (template rendering)
- Contract: `run` writes outputs by rendering Jinja2 templates; `run --id` targets one derivation.
- Failure: missing template file → non-zero exit.
- Tests: `tests/test_run_stub.py`.

## Slice 2 — Container transformers (minimal)
- Contract: `run` builds local Docker images and executes containers to transform data.
- Contract: Materials mounted into container; outputs validated after run.
- Contract: Transformer derivations use `transformer.build` with local Dockerfile.
- Failure: missing Dockerfile → exit code 1; build failure → exit code 1; missing output → exit code 1.
- Tests: `tests/test_container_transformers.py`.
- Acceptance: Both example artifacts functional (sprint-brief renders, backlog transforms via container).
- Docker required: Exit code 1 with helpful message if Docker not available.
- See: `docs/roadmap/slice-2-container-transformers-minimal.md` for detailed specification.

## Slice 3 — Direct-edit + finalize/attest
- Contract: `status --json` returns change_origin (string), downstream[].
- Contract: `finalize` writes `.graft/provenance/finalize.json` with agent info if provided.
- Tests: `tests/test_status_and_finalize.py`.

## Slice 4 — Impact & simulate
- Contract: `impact --json` returns `{ artifact, downstream[] }`.
- Contract: `simulate --cascade` prints a confirmation line, no file changes.
- Tests: `tests/test_impact_and_simulate_stubs.py`.

## Slice 5 — DVC orchestrator integration (seamful autosync)
- Contract: Graft keeps `dvc.yaml` in sync with derivations via configurable autosync.
- Stage mapping: One stage per derivation (`graft:<artifact>:<derivation-id>`).
- Sync policies: `off`, `warn`, `apply`, `enforce` (configurable in `graft.config.yaml`).
- Per-command defaults:
  - `run`, `init`: `apply` (auto-write dvc.yaml)
  - `explain`, `status`, `impact`, `simulate`, `finalize`: `warn` (show drift, no write)
  - `dvc scaffold`: explicit write/check mode
- Drift detection: missing stages, mismatched specs, orphaned stages.
- Non-managed stages: Preserved verbatim (only stages with `managed_stage_prefix` are touched).
- JSON output: All autosyncing commands include `orchestrator` block with drift/plan info.
- Error handling: `E_ORCH_DRIFT_ENFORCED`, `E_DVC_YAML_INVALID`, etc.
- Tests: `tests/test_dvc_orchestrator.py`, `tests/test_dvc_autosync.py`.
- See: `docs/dvc-integration.md` for detailed specification.

## Slice 6 — Container-based transformers (future)
- Contract: Derivations with `transformer.build` execute via container runtime.
- Contract: Local Dockerfile builds with network/determinism policies.
- Out of scope for initial implementation.

## Slice 7 — Advanced features (future)
- Ingest-as-files flow demonstrations.
- Remote transformer refs.
- Advanced provenance tracking.
