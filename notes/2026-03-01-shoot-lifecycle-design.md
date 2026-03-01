---
status: working
purpose: "Design session: shoot lifecycle (worktree-based parallel workstreams) with composable hooks"
---

# Shoot Lifecycle Design

Session exploring how graft bootstraps and manages parallel workstreams using git
worktrees, and how Claude Code integrates as a worker runtime. Refines ideas from
the [graft as context provider](2026-02-28-graft-as-context-provider.md) exploration.

## Key Decisions

### Vocabulary: shoot / fuse / prune

Git worktrees map to the horticultural metaphor. A **shoot** is new growth from an
existing tree — grows independently, gets its own physical space, and is either
incorporated back into the trunk or pruned away. Terminology:

- **shoot** — a worktree + branch pair for parallel work
- **fuse** — incorporate a shoot into the trunk (merge to main + cleanup).
  In tree biology, fusion is when cambium layers grow together and become
  structurally continuous. The branch history *becomes* trunk history — no seam.
- **prune** — discard a shoot (delete worktree + branch)

### "Slice" stays in software-factory

Graft core knows commands, sequences, state queries, and dependencies. The
work-unit concept (slice, issue, task) is workflow-defined. `graft shoot` operates
on names, not domain-specific work units. The workflow package maps its work units
to shoots.

A `work` command that understands slices belongs in software-factory's graft.yaml as
a command, not in graft core:

```yaml
# In software-factory's graft.yaml
commands:
  work:
    run: "bash scripts/work.sh"
    args:
      - name: slice
        type: choice
        options_from: slices
```

### Layer separation

| Layer | Owns |
|---|---|
| Graft | Primitives: shoot create/list/fuse/prune, branch-scoped state queries |
| Workflow package | Opinions: work-unit definition, worker bootstrap, work protocol |
| Claude Code config | Runtime: .claude/rules (generated), skills (protocol), hooks (enforcement) |

### Artifacts over actors

Don't track worker processes. Derive workstream state from artifacts:

- **Git commits** — progress (via `git log`)
- **Ahead/behind** — integration status (via `git rev-list`)
- **Plan checkmarks** — step completion (workflow-defined)
- **Run-state files** — verify results, context snapshots
- **Dirty state** — active work indicator (via `git status`)

No worker registry, no heartbeat, no coordination protocol. "Last activity: 47m
ago" is more honest than a stale "running" status.

## Shoot Lifecycle

### Commands

```
graft shoot create <name>     # worktree + branch, run on_create hooks
graft shoot list               # enumerate with artifact-derived state
graft shoot fuse <name>        # merge to main + cleanup, with hook gates
graft shoot prune <name>       # discard, with cleanup hooks
```

### Create

```
1. git worktree add .worktrees/<name> -b feature/<name>
2. Run on_create hook chain
   → on failure: remove worktree (rollback), exit with error
3. Print worktree path
```

### Fuse

```
1. Merge feature/<name> into main (to temp ref)
2. Run pre_fuse hook chain
   → on failure: discard temp ref (rollback), exit with error
3. Fast-forward main to merged result
4. Run post_fuse hook chain
   → on failure: leave shoot intact (main already moved), exit with error
   → re-running fuse detects "already merged", retries from step 4
5. Remove worktree + branch
```

### Prune

```
1. Run on_prune hook chain
   → on failure: worktree untouched, exit with error
2. Remove worktree + branch
```

### List

Pure query, no hooks. Enumerates worktrees with git-derived state:

```
retry-logic       3 ahead, 0 behind   last: 2m ago    verify: pass
input-validation  1 ahead, 3 behind   last: 31m ago   verify: —
error-handling    (not started)
```

## Composable Lifecycle Hooks

### Principle: composition over override

Hooks defined in dependencies and the project **compose** — both run. The
dependency provides infrastructure, the project customizes on top. Neither needs
to know about the other.

This avoids the override problem where the project must duplicate or wrap
dependency behavior to extend it.

### Hook specification in graft.yaml

```yaml
shoots:
  on_create: setup-worker          # single command
  pre_fuse:                        # or a list
    - verify-result
    - check-coverage
  post_fuse: update-status
  on_prune: archive-context
```

Command names resolve relative to the defining graft.yaml's scope. Unqualified
names in a dependency's graft.yaml resolve to that dependency's namespace. Cross-
scope references use qualification: `software-factory:setup-worker`.

### Resolution algorithm

Hooks compose across scopes. Dependencies run first (in declaration order), then
the project. Within each scope, list items run in order.

```
resolve_hook_chain(event):
  chain = []
  for dep in project.dependencies (declaration order):
    if dep.graft_yaml.shoots[event] defined:
      append dep's hooks, qualified to dep's namespace
  if project.graft_yaml.shoots[event] defined:
    append project's hooks (unqualified)
  return chain
```

**Example:**

```yaml
# Project graft.yaml
dependencies:
  software-factory:          # hooks run first
    git_url: ...
  compliance-checks:         # hooks run second
    git_url: ...

shoots:
  on_create: project-setup   # runs third (last)
```

Resolved `on_create` chain:
1. `software-factory:setup-worker`
2. `compliance-checks:init-audit-log`
3. `project-setup`

Each hook sees the effects of previous hooks (files written, state updated).

### Failure semantics

**All hooks are gates.** Hook fails, operation fails. No silent failures.

If a hook is defined, it expresses a requirement. Graft honors that by failing
loudly when the requirement isn't met. If you want fire-and-forget behavior, make
your hook handle errors internally and exit 0.

**Fail-fast, sequential execution:**

```
execute_hook_chain(chain, env):
  completed = []
  for hook in chain:
    result = run_command(hook, env)
    if result.failed:
      return Err { failed_hook, completed_hooks, error }
    completed.append(hook)
  return Ok { completed_hooks }
```

Error output reports which hook failed AND which hooks already ran.

**Rollback by event:**

| Event | On chain failure | Rationale |
|---|---|---|
| `on_create` | Remove worktree | Incomplete setup is a trap for the worker |
| `pre_fuse` | Discard temp merge | Nothing irreversible happened |
| `post_fuse` | Leave shoot intact | Main moved; re-run retries from failure point |
| `on_prune` | Don't delete worktree | Archive didn't complete; data preserved |

**Idempotency contract:** hooks must be safe to re-run. When retrying after
`post_fuse` failure, the entire chain runs again. Documented as a contract, not
enforced by graft.

### Hook environment

All hooks receive shoot identity via environment variables:

```
GRAFT_SHOOT_NAME=retry-logic
GRAFT_SHOOT_BRANCH=feature/retry-logic
GRAFT_SHOOT_WORKTREE=.worktrees/retry-logic
```

Hooks run inside the worktree as working directory (for `on_create`, `pre_fuse`,
`on_prune`) or the project root (for `post_fuse`, since the worktree may be about
to be removed).

### No override mechanism (yet)

Start with pure composition. If the need arises to disable a dependency's hook, add
a targeted skip mechanism rather than wholesale override. Don't design for the
uncommon case upfront.

## Claude Code Integration

Minimal configuration surface, layered by investment:

### Layer 0: Generated rules (start here)

`on_create` hook generates `.claude/rules/work-assignment.md` in the worktree:

```markdown
# Work Assignment
You are working on: retry-logic
Current step: implement
Branch: feature/retry-logic

## Protocol
- Run `graft status` to see current state
- When you hit a checkpoint, stop. Do not continue past gates.
- Run `graft run verify` before declaring a step complete
```

Works today. Claude Code reads rules files. Graft CLI available via Bash. The
"integration" is templating a file.

### Layer 1: Skills (worth doing)

`.claude/skills/` wrapping common operations:

- `/context` — assembles full worker context (state + assignment + focus)
- `/checkpoint` — records progress, signals gate
- `/verify` — runs verify, captures output

Three skills. Markdown files wrapping shell commands. Shipped with the workflow
package, copied into the worktree by `on_create`.

### Layer 2: Hooks (selective)

- `Stop` hook — auto-sync state when Claude finishes responding (automatic
  protocol compliance, solves the "cooperative agent" problem)

### Layer 3: MCP server (skip for now)

First-class graft tools in Claude's tool palette. Build after validating the
pattern with layers 0-1.

## Open Questions

- **Shoot state queries**: how much of `shoot list` is built into graft (git
  operations) vs composed from existing state queries (verify status, step
  progress)? Ahead/behind is pure git; verify status is workflow-defined.
- **Multi-shoot coordination**: when shoot A fuses, shoot B may need rebase.
  Does graft detect this? Surface it? Auto-act? Or is this workflow/grove
  territory?
- **Human feedback channel**: `:request-changes` writes feedback where? A file
  in the worktree? A convention in run-state? Probably a workflow-package
  concern.
- **Dependency hook resolution for transitive deps**: current model is flat
  (project's direct dependencies only). Dependencies are opaque — their internal
  hook composition is encapsulated within their own commands.
- **`pre_fuse` merge strategy**: merge to temp ref, then fast-forward? Or
  optimistic merge with rollback? Temp ref is cleaner but adds a step.

## Sources

- [Graft as Context Provider](2026-02-28-graft-as-context-provider.md) — parent
  exploration (worker model, artifacts over actors, local PR workflow)
- [graft-yaml-format.md](../docs/specifications/graft/graft-yaml-format.md) —
  current graft.yaml specification
- [Agentic Orchestration](2026-02-18-grove-agentic-orchestration.md) — dispatch
  board metaphor
- [Sequence Primitives](2026-02-24-sequence-primitives-exploration.md) — sequence
  design decisions
