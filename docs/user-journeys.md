# User & System Journeys

**Personas**: contributor, agent (Claude Code), CI, PR reviewer, external system (via CI).

## Weekly brief
- CI ingests snapshots → files.
- Contributor runs `graft run` on sprint brief (deterministic), then edits the brief directly.
- `graft status/validate/finalize` produce provenance; later `propose` (future) opens a PR.
- Reviewer sees semantic diffs and downstream impact (future slice).

## Agent-maintained roadmap
- Agent reads config with `explain --json`, edits specific sections, validates, simulates cascade, finalizes with attribution.
