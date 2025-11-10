# CLI Spec (proposed)

All commands accept `--json` where applicable. Exit codes: `0` success, `1` user error, `2` system error.

- `graft explain <artifact/> [--json]`
- `graft run <artifact/> [--id <derivation-id>]`
- `graft status <artifact/> [--json]`
- `graft validate <artifact/>`
- `graft finalize <artifact/> [--agent <name>] [--model <m>] [--params <json>]`
- `graft impact <artifact/> [--json]`
- `graft simulate <artifact/> [--cascade]`
- `graft init [<path>]` — create `graft.config.yaml` (defaults)
- `graft dvc-scaffold [<project-root>]` — generate `dvc.yaml` stages for artifacts
