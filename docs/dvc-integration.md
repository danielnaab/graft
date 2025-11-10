# DVC Integration (scaffold-first)

Graft does **not** run `dvc init` or modify `.dvc/config`. Instead it can generate a **`dvc.yaml`** that defines stages corresponding to artifacts.

```bash
graft dvc-scaffold .
# creates dvc.yaml with stages:
#  - <graft-name>:
#      cmd: graft run examples/agile-ops/artifacts/sprint-brief/
#      deps: [materials..., graft.yaml]
#      outs: [outputs...]
```

## Recommended setup
1. `dvc init` — create `.dvc/` and `.dvc/config`.
2. `dvc remote add` — configure a remote if needed.
3. `dvc repro` — uses the generated `dvc.yaml` to run `graft` stages when dependencies change.

## Why scaffold-only
- Avoid guessing your DVC preferences (remotes, cache modes).
- Keep clean separation: **Graft** describes the logical derivations; **DVC** manages performance and storage of large artifacts.
