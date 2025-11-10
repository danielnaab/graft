# Graft (proposal starter)

**Graft** is a file-first orchestration tool for **auditable data transformations**. A *graft* is both:
- a **recipe** (configuration describing inputs, transformers, outputs, and policy), and
- a **directory of files** that **grows** like a branch on a git tree as new source material feeds it.

Graft lets people, agents, CI systems, and deterministic tools collaborate on evolving files while maintaining **provenance**, **safety**, and **great PR ergonomics**.

> Scope: This starter focuses on **pure file workflows** (no network side-effects). External systems are ingested to disk by CI; Graft reads files and writes derived artifacts.

## Quick start (uv + Python 3.14)

```bash
# Install uv (see https://github.com/astral-sh/uv) then:
uv install
uv run graft --help

# Try the CLI (JSON for agents)
graft explain examples/agile-ops/artifacts/sprint-brief/ --json
graft run     examples/agile-ops/artifacts/sprint-brief/
graft status  examples/agile-ops/artifacts/sprint-brief/ --json

# Tests (black-box)
uv pip install -e ".[test]"
pytest -q
```

## Repository map
- `src/` — Python CLI stubs (outside‑in friendly, agent‑centric `--json`)
- `docs/` — proposal, architecture, DVC integration, CLI spec, philosophy, testing, implementation strategy, roadmap
- `schemas/` — JSON Schemas for `graft.yaml`, policy, provenance, CLI outputs
- `examples/` — agile demo using the graft model (no side effects)
- `skills/` — Claude Code Skill (`SKILL.md`) for this repo
- `CLAUDE.md` — best practices & guardrails for agent collaboration
- `agent-records/` — lightweight logs so agents/humans can pause/resume work
- `.github/workflows/` — CI with uv on Python 3.14

See `docs/proposal/overview.md` and `docs/user-journeys.md` for the big picture.
