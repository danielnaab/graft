---
title: "Grove: Agentic Workflow Orchestration"
date: 2026-02-18
status: superseded
superseded_by: 2026-03-01-scion-orchestration-design.md
participants: ["human", "agent"]
tags: [exploration, grove, agentic, orchestration, sessions, plans, design]
---

# Grove: Agentic Workflow Orchestration

> **Superseded**: This design has been replaced by the
> [scion orchestration design](2026-03-01-scion-orchestration-design.md).
> Scions (graft-owned worktree+branch+runtime) replaced the grove-owned session
> model proposed here. Slices 8-13 below have been retired — see annotations in
> the [Vertical Slices](#vertical-slices) table. Kept for historical context.

## Context

Design exploration of how agentic workflow automation (planning, orchestrating, launching, monitoring subprocess agents like Claude Code) fits into the grove and graft product areas.

Starting question: does this belong in grove, graft, or a new product?

Conclusion: extend grove's metaphor from "departure board" to "dispatch board." Same principle — Grove provides infrastructure and glue, user provides domain knowledge — but wider scope.

---

## Fit Analysis

### What fits in Graft (already works)

Graft's change model already supports agents as migration executors:

```yaml
commands:
  migrate-v2:
    run: claude-code "Rename all getUserData calls..."
  verify-v2:
    run: npm test && ! grep -r 'getUserData' src/
```

Scope, execution, verification, atomic rollback. Graft doesn't care if `run` invokes a human script or an agent. This is a use case the current model already supports, not future work.

### What fits in Grove

- **Agent visibility**: Departure board shows agent status alongside repo status. Natural extension.
- **Launching sessions**: Sessions are commands with lifecycle (Slice 7 extension).
- **Context assembly**: Unique value — no single-repo tool can provide cross-workspace context.
- **Review flow**: "what needs attention" includes "what agents have done, pending review."

### What doesn't fit in either (gap)

- Multi-agent coordination across repos
- Plan decomposition (deciding *what* to do, not just *how*)
- Agent lifecycle management

These are covered by the new session/plan primitives below.

---

## New Primitives

Three new engine concepts, all consistent with existing "shell-based extensibility" principle:

### Session

A named, trackable agent invocation against a repo. Differs from Slice 7 commands:

1. **Background** — doesn't block TUI or hold a modal
2. **Persistent** — survives Grove restarts (state on disk)
3. **Observable** — output streamed to file, tailable live
4. **Reviewable** — completion triggers review gate, not just "done"

No special "agent" type. Claude Code, aider, a shell script — it's all a shell command. Agent behavior is emergent from the command being long-running and file-changing.

```
Session {
    id: SessionId,
    repo: RepoPath,
    command: String,
    status: Pending | Running | Completed | Failed | NeedsReview,
    session_type: Headless | Tmux | Foreground,
    started_at: Option<Timestamp>,
    completed_at: Option<Timestamp>,
    output_path: PathBuf,
    pid: Option<u32>,
    tmux_session: Option<String>,
}
```

### Context

Workspace state assembled for a session before launch. Grove's unique value.

```yaml
# workspace.yaml extension
context:
  template: |
    # Workspace: {{workspace_name}}
    ## This Repository
    {{repo_status}}
    {{graft_metadata}}
    ## Sibling Repositories
    {{sibling_summary}}
    ## Recent Sessions
    {{recent_sessions}}
```

`grove context <repo>` generates a context document. On launch, injected automatically (temp file or stdin). Template variables filled from existing engine queries — no new data sources, just assembly.

### Plan

Ordered collection of sessions with dependencies and approval gates. Separate files, not workspace.yaml (workspace declares what exists; plans declare what to do).

```yaml
name: monthly-close
steps:
  - id: categorize
    repo: ~/finances
    command: claude "Categorize uncategorized transactions"

  - id: reconcile
    repo: ~/finances
    command: claude "Reconcile accounts against bank statements"
    depends_on: [categorize]
    approval: required

  - id: update-notes
    repo: ~/notes
    command: claude "Update financial summary"
    depends_on: [reconcile]
    context_from: [reconcile]   # output feeds input
```

Plans compose sessions the way workspace.yaml composes repos.

---

## Process Management

### The core tension

Two kinds of agent work need different process management:

- **Headless**: agent doesn't need human input (e.g., `--dangerously-skip-permissions`, scripts). Can truly background.
- **Interactive**: agent may ask questions, need approvals, require a terminal. Claude Code in normal mode. Can't background without a TTY.

### Headless sessions (setsid)

- Grove forks, calls `setsid` to create new process group (session leader)
- stdout/stderr redirected to `~/.local/share/grove/sessions/<id>/output.log`
- PID, start time, command, repo recorded to session metadata
- Process is independent of Grove — survives Grove exit

### Interactive sessions (tmux)

- Grove creates a named tmux session: `tmux new-session -d -s grove-<id> -c <repo> "<cmd>"`
- Fully persistent (survives Grove exit, terminal close, SSH disconnect)
- Attachable: `grove attach <id>` → `tmux attach -t grove-<id>`
- Observable: tmux capture-pane for status summary

### Foreground fallback (no tmux)

- Grove suspends TUI, runs command in foreground terminal, resumes when done
- Like shelling out from vim
- Simple, but: one at a time, blocks Grove, doesn't survive Ctrl+C

### Session lifecycle on Grove restart

1. Read `~/.local/share/grove/sessions/*/meta.json`
2. For "running" sessions: check PID liveness (`kill -0 <pid>`) or tmux session (`tmux has-session -t grove-<id>`)
3. If dead: update status to completed/failed
4. Display current state

No daemon needed. Registry is just files. Consistent with "no external database" principle.

### Attach vs. Monitor

Distinct operations:

- **Monitor** (any session type): read-only output tail. TUI pane showing streaming log. `grove monitor <id>`.
- **Attach** (tmux sessions only): interactive. Grove suspends TUI, attaches to tmux, detach returns to Grove. `grove attach <id>`.

For headless sessions, attach doesn't make sense — no terminal to connect to. Grove surfaces this clearly.

---

## Vertical Slices

Building on existing slices 1-7:

| # | Slice | Depends On | Goal | Disposition |
|---|-------|------------|------|-------------|
| 8 | **Background Sessions** | 7 | Launch command as tracked background session, see status in repo list | **Retired — done.** Scions + `SessionRuntime` + `graft scion start/stop` replace this. |
| 9 | **Session Monitor** | 8 | View live output from running sessions, tail completed logs | **Retired — partially replaced.** "Monitor" (tail output) replaced by tmux attach. "Attach" concept carries forward as grove `:attach` and `graft scion attach`. See `scion-attach` and `grove-scion-commands` slices. |
| 10 | **Context Assembly** | 8, 5 | Auto-assemble workspace context, inject on launch | **Retired — out of scope.** Explicitly a workflow-package concern, not graft/grove code. `on_create` hooks + command `run:` field handle this. |
| 11 | **Review Flow** | 9 | Review session results (diff), accept/reject/annotate | **Retired — concept carries forward.** Review reframed as artifact-based: `:review <scion>` shows diff against main. See `grove-scion-review` slice. |
| 12 | **Plans** | 10, 11 | Multi-step plans with approval gates and context chaining | **Retired — deferred.** Multi-step plans with gates remain future work. Sequences + scions partially address this but formal plan primitives aren't scoped. |
| 13 | **Session Dashboard** | 8, 9 | Cross-workspace view of all sessions | **Retired — replaced.** Grove scion list replaces cross-workspace session dashboard. See `grove-scion-commands` slice. |

Slice 8 implements headless only first. tmux/interactive support is Slice 8b or folded into Slice 9. Monitoring infrastructure is the same for both modes.

---

## Configuration

```yaml
# workspace.yaml extension
sessions:
  default_mode: headless          # headless | interactive
  multiplexer: tmux               # tmux | screen | none
  output_dir: ~/.local/share/grove/sessions/

  # Agent shortcuts (just command prefixes, same shell-extensibility principle)
  agents:
    claude: claude --dangerously-skip-permissions
    claude-interactive: claude
    aider: aider --yes-always
```

---

## Architecture Layer Mapping

```
grove-core additions:
  Session, SessionId, SessionStatus, SessionType   (domain types)
  Plan, PlanStep                                    (domain types)
  SessionRegistry, ContextAssembler                 (traits)

grove-engine additions:
  ProcessSessionRegistry      (setsid for headless, tmux for interactive)
  TmuxSessionRegistry         (tmux-based sessions)
  WorkspaceContextAssembler   (fills templates from engine queries)
  StepwisePlanExecutor        (plan state machine)

grove-cli additions:
  grove launch                (Slice 8)
  grove sessions              (Slice 8)
  grove monitor <id>          (Slice 9)
  grove attach <id>           (Slice 9, tmux only)
  grove context <repo>        (Slice 10)
  grove review <id>           (Slice 11)
  grove plan run/status       (Slice 12)
  grove dashboard             (Slice 13)

TUI additions:
  Session indicator in repo list (🤖 running, ✓ done, ✗ failed)
  Session output pane (Slice 9)
  Review view (Slice 11)
  Plan view (Slice 12)
  Dashboard mode (Slice 13)
```

Every new CLI command gets `--json`, per existing "CLI-first for every feature" principle.

---

## Composition with Existing Primitives

New concepts plug into existing extension points rather than replacing them:

- **Status scripts** detect agent state: `[ -f .session-running ] && echo "🤖 Agent working"` — already works with Slice 1, no new code needed
- **Capture routing** records session notes: `grove capture "@done categorized 15 transactions"`
- **Slice 7 commands** remain for synchronous one-shots; sessions are for background work
- **Graft commands** can be session targets: `grove launch ~/myapp "graft upgrade meta-kb"`
- **Shell extensibility** preserved: sessions launch shell commands, not agent-specific APIs

## Metaphor Shift

Grove goes from "departure board" (passive display) to "dispatch board" (active coordination). The board still shows what's happening — but you can also tell it to send work out. The dispatch mechanism is intentionally simple: "run this command in this repo." Complexity lives in the plan files and shell commands, not in Grove. Same principle as before, wider scope.

---

## Sources

- [Grove Workflow Hub Primitives (2026-02-07)](2026-02-07-grove-workflow-hub-primitives.md) — original agentic integration design
- [Grove Architecture Spec](../docs/specifications/grove/architecture.md)
- [Grove Command Execution Spec](../docs/specifications/grove/command-execution.md)
- [Grove Roadmap](../docs/grove/planning/roadmap.md)
