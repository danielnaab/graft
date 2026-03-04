---
status: done
purpose: "Pain points discovered while dogfooding the grove/graft scion workflow"
---

# Dogfood Pain Points (2026-03-02)

Tracking issues found while exercising the full scion workflow:
create slice → create scion → start worker → monitor → review → fuse.

## Pain Points

1. **No output preview for running commands in the transcript log** — DEFERRED
   Running `:run new-slice ...` shows a spinner but no indication of what the
   command is doing. There's no streaming output preview in the log section —
   you can't tell if it's working, stuck, or erroring until it finishes. Ideally,
   the running block would show a live tail of the command's stdout (e.g. last
   2-3 lines) so you can confirm progress without needing to focus/expand.
   **Status**: Deferred as independent grove UX work (see pain-point-strategy.md).

2. **`new-slice` slug extraction is fragile** — RESOLVED
   The script required Claude to output `slug: <value>` as the very first
   non-empty line of its response, which was unreliable.
   **Resolution**: software-factory refactor (c5721e8) replaced the extraction
   approach with wrapper scripts that take slug as a CLI argument via `{slug}`
   placeholder interpolation. The new `new-slice-create.sh` validates the slug
   and exports it as `GRAFT_NEW_SLUG` env var.

3. **`scion start` fails — no `scions:` config in root graft.yaml** — RESOLVED
   `scion start` required `scions.start` to name a command but the root
   `graft.yaml` had no `scions:` section or commands.
   **Resolution**: Engine now supports `dep:command` format in `scions.start`
   (e606162), so the root config can reference commands from dependencies.
   Root `graft.yaml` now has `scions.start: "software-factory:implement"`
   (278dbc8). Full command resolution pipeline (state queries, env vars,
   stdin rendering, launcher script) works through `scion start`.

4. **No path from `scion create` to working in the scion** — RESOLVED
   After creating a scion, `scion start` failed and there was no integrated
   way to launch a worker in the scion's worktree.
   **Resolution**: `scion_start` now performs full command resolution identical
   to `graft run` — arg interpolation, stdin forwarding, state queries, env
   vars — and writes a self-contained launcher script for tmux (e606162).
   The `dep:command` format lets the root config delegate to
   `software-factory:implement`, which works out of the box.
