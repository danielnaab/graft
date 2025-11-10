# CLAUDE.md — Graft Project

You are collaborating on **Graft**. Follow these practices to be effective and safe.

## Goals
- Deliver vertical slices in order. Do not expand scope beyond the current slice.
- Keep the CLI contract (see `docs/cli-spec.md`) stable; emit `--json` where applicable.

## Process
1. Read `docs/implementation-strategy.md` and `docs/roadmap/vertical-slices.md`.
2. For the current slice:
   - Add/adjust tests in `tests/` (subprocess/black-box).
   - Implement the minimal code in `src/` to satisfy tests.
   - Update docs and schemas as needed.
   - Log progress in `agent-records/work-log/<date>.md`.

## Guardrails
- No side effects or network writes.
- Do not introduce tracked patch files.
- Prefer small, focused PRs with passing tests.

## Helpful commands
- `pytest -q`
- `graft explain <artifact/> --json`
- `graft run <artifact/>`
- `graft dvc-scaffold .`
