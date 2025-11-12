# CLI Spec

All commands accept `--json` where applicable. Exit codes: `0` success, `1` user error, `2` system error.

## Commands

- `graft explain <artifact/> [--json] [--sync off|warn|apply|enforce]`
- `graft run <artifact/> [--id <derivation-id>] [--sync off|warn|apply|enforce]`
- `graft status <artifact/> [--json] [--sync off|warn|apply|enforce]`
- `graft validate <artifact/>`
- `graft finalize <artifact/> [--agent <name>] [--model <m>] [--params <json>] [--sync off|warn|apply|enforce]`
- `graft impact <artifact/> [--json] [--sync off|warn|apply|enforce]`
- `graft simulate <artifact/> [--cascade] [--sync off|warn|apply|enforce]`
- `graft init [<path>]` — create `graft.config.yaml` (defaults)
- `graft dvc scaffold [--check] [--json]` — manage `dvc.yaml` stages for all artifacts

## Orchestrator Integration

The `--sync` flag controls DVC autosync behavior on commands that support it:

- `off`: Never write dvc.yaml; show plan if drift exists
- `warn`: Never write; show plan; exit 0 (default for read-only commands)
- `apply`: Write dvc.yaml automatically if drift exists (default for write commands)
- `enforce`: Fail with exit code 1 if drift exists; don't write

If not specified, each command uses its default policy (see docs/dvc-integration.md).
