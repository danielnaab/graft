---
status: draft
---
# runs/1: runs, for other programs (reserved)

**Reserved.** v2.16.0 has no verb for this contract. It is written down so that a program waiting for it knows
what not to read: a run's state folder is pot's private layout, and may change in any release.

| | |
|---|---|
| Version | 1 (reserved) |
| Producer | pot |
| Reader | workmux, Windmill jobs, later grove |
| Owner | pot |

## Shape (planned)

```
pot runs [ID] [--json] [--follow]
```

A run record has:

| Field | Meaning |
|---|---|
| `state` | `running`, `exited`, `stopped` (exit 125, waiting for a grant), `review` (a dispatch waiting for review) or `pending` |
| `context` | the context |
| `command.kit` | the command kit's identity, when there is one |
| `workspace[]` | the workspace's repository identities |

Events are the lines of the run log (`log.jsonl`), whose shape is frozen: `start`, `net`, `cap`, `notfound`, `exit`
and `after`, each with `ev` and `by` (`host` or `run`). A `by: "run"` line is data the run reported, never evidence.

## Compatibility

- Until the verb exists, no reader may depend on pot's state folder (`runs/<id>/`): it is hidden from every run,
  and its layout is not a contract.

## Sources

- [pot/core/pot.py](../core/pot.py): the run log
- [pot's reference: State](../../docs/reference/pot.md#state)
