---
status: working
purpose: "Pain points discovered while dogfooding the grove/graft scion workflow"
---

# Dogfood Pain Points (2026-03-02)

Tracking issues found while exercising the full scion workflow:
create slice → create scion → start worker → monitor → review → fuse.

## Pain Points

1. **No output preview for running commands in the transcript log**
   Running `:run new-slice ...` shows a spinner but no indication of what the
   command is doing. There's no streaming output preview in the log section —
   you can't tell if it's working, stuck, or erroring until it finishes. Ideally,
   the running block would show a live tail of the command's stdout (e.g. last
   2-3 lines) so you can confirm progress without needing to focus/expand.

2. **`new-slice` slug extraction is fragile**
   The script requires Claude to output `slug: <value>` as the very first
   non-empty line of its response. Claude frequently ignores this instruction
   and starts generating plan content directly, causing the script to fail with
   "missing or malformed slug: marker on first line." The prompt engineering
   is fighting model behavior. Options: retry logic, extract slug from content
   with a fallback regex, or derive the slug deterministically from the
   description (e.g. first N words → kebab-case) instead of asking Claude.
   **Action**: Fix `new-slice.sh` slug extraction reliability.

3. **`scion start` fails — no `scions:` config in root graft.yaml**
   `scion start` requires `scions.start` to name a command to launch in the
   runtime session. The root `graft.yaml` only has `deps:` — no `scions:`
   section, no commands at all. Software-factory doesn't provide a default
   `scions.start` either. The error message ("no start command configured in
   scions.start") is accurate but unhelpful for a first-time user — it doesn't
   tell you what to add or where.
   **Action**: Add `scions.start` + a worker command to root `graft.yaml`.
   Consider whether software-factory should provide a default start command.
   Even with config added, the engine doesn't resolve command args or stdin
   when launching via runtime — so `stdin: literal` prompts and `{slice}` arg
   interpolation won't work through `scion start` without engine changes.
   **Workaround**: cd into `.worktrees/<name>` and run claude manually.

4. **No path from `scion create` to working in the scion**
   After creating a scion, `scion start` fails (no config) and `attach` fails
   (no tmux session). There's no grove command to open a shell or editor in
   the scion's worktree. The user has to leave grove, `cd .worktrees/<name>`,
   and run claude manually. The scion provides branch isolation but no
   integrated way to use it.
   **Action**: At minimum, the engine needs to resolve `scions.start` commands
   with arg interpolation and stdin forwarding to the runtime. Longer term,
   software-factory should define `scions.start: implement` so it works out of
   the box.
