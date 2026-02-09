---
title: "Grove as Workflow Hub: Design Primitives"
date: 2026-02-07
status: working
participants: ["human", "agent"]
tags: [exploration, grove, primitives, workflows, agentic, design, hub]
---

# Grove as Workflow Hub: Design Primitives

## Context

Following the [Grove Vertical Slices](./2026-02-06-grove-vertical-slices.md) plan, this session explored Grove's role as a **workflow hub** for personal multi-repo workspaces. The key question: what simple primitives make Grove genuinely useful as a "home page" for daily work, while maintaining composability and avoiding feature bloat?

The exploration considered:
- Real daily workflows with personal repos (finances, notes, brainstorming)
- Agentic workflows (how Grove intersects with Claude Code, etc.)
- The "workspace map" vs "workspace replacement" framing
- What makes a good "home page" for managing multiple workstreams

**Graduated to specifications:**
- [Grove Architecture](../docs/specifications/grove/architecture.md) - Agentic integration, CLI-first design
- [Workspace Configuration](../docs/specifications/grove/workspace-config.md) - Status scripts, tags, capture routing, workstreams

---

## Key Insights

### 1. Grove is a Workspace Map, Not a Replacement

**The mental model:** Grove is a departure board, not the airport. You check it to know where to go, then you go there using your real tools ($EDITOR, terminal, agent sessions).

**Implications:**
- Time-to-useful-information must be near-zero
- The dashboard/status view is the primary interface
- File navigation and git operations are secondary features
- Grove should excel at cross-cutting operations no single tool handles well

**What Grove should be faster at than alternatives:**
- "What's the state across all my repos?" (no single tool does this)
- "Find this thing I wrote somewhere" (rg works per-directory)
- "Capture this thought to the right place" (requires knowing where)
- "What commands are available here?" (requires remembering per-repo)

### 2. The Workspace is the Missing Context Layer for Agents

**Current state:** Coding agents (Claude Code, Cursor, Aider) work in single-repo silos. They have no awareness of sibling repos, dependencies, or the broader workspace context that the human holds.

**Grove's role in agentic workflows:**

**Between agent sessions:** Grove shows workspace state — what changed, what's dirty, what needs attention. The "departure board" function.

**Before an agent session:** Grove provides context. "Finances has 3 uncommitted transaction files and the monthly report is overdue."

**During an agent session:** Grove provides workspace awareness via `grove status --json` or as an MCP server. The agent can query repo state, search across repos, understand dependencies.

**After an agent session:** Return to Grove, see updated status, capture follow-up thoughts.

**Grove is the workspace consciousness that persists between agent sessions.**

### 3. Graft Commands are Well-Suited for Agent Execution

Graft's upgrade model already provides what agent workflows typically lack:

```yaml
changes:
  v2.0.0:
    migration: migrate-v2    # ← do the work
    verify: verify-v2        # ← check the work
```

With atomic rollback on verification failure. This pattern:
- Clear scope (consumer repo, specific change)
- Clear input (migration description + dependency content)
- Execution (the migration command)
- Verification (the verify command)
- Atomicity (rollback on failure)

**Agent-powered migrations are possible today:**
```yaml
commands:
  migrate-v2:
    run: |
      claude-code "Rename all getUserData calls to fetchUserData.
        See ${DEP_ROOT}/CHANGELOG.md for migration guidance."
  verify-v2:
    run: |
      npm test
      ! grep -r 'getUserData' src/
```

The `verify` command is the "human-in-the-loop" check that agent workflows need. Graft doesn't care if `run` invokes an agent — the semantics are preserved.

---

## Design Primitives

Six primitives that keep Grove simple while making it a powerful workflow hub:

### Primitive 1: Repos Define Their Own Status

Instead of Grove imposing "git status = repo status," let repos declare what status means for them via shell scripts:

```yaml
repositories:
  - path: ~/finances
    status:
      - name: overdue
        run: |
          days=$(( ($(date +%s) - $(git log -1 --format=%ct)) / 86400 ))
          [ $days -gt 30 ] && echo "⚠️ Monthly close overdue ($days days)"

      - name: uncategorized
        run: |
          count=$(grep -c "TODO" transactions/*.ledger 2>/dev/null || echo 0)
          [ $count -gt 0 ] && echo "📋 $count uncategorized transactions"

  - path: ~/notes
    status:
      - name: inbox-overflow
        run: |
          count=$(ls inbox/ | wc -l)
          [ $count -gt 10 ] && echo "📥 $count captures to organize"
```

**Why this is elegant:**
- Just shell commands — maximally flexible, zero DSL
- Each repo defines what "needs attention" means for its domain
- Grove doesn't need domain knowledge (ledger files, inbox conventions)
- Composes with everything (Python scripts, database queries, API calls)
- Same pattern as graft commands

The home page runs all status scripts and surfaces active signals.

### Primitive 2: Workstreams are Named Workspace Configs

Instead of "workstream" being a concept inside Grove, it's simply: **you can have multiple workspace files and switch between them.**

```bash
~/.config/grove/
  workspace-personal.yaml   # finances, notes, brainstorming
  workspace-grove.yaml      # graft-knowledge, grove-brainstorming
  workspace-work.yaml       # work repos
```

Launch with `grove --workspace grove` or `grove` (uses last workspace or shows chooser).

**Why this is elegant:**
- Reuses the workspace primitive with zero modification
- Workstreams are emergent from "which repos go together?"
- Switching workstreams = switching workspace files
- Solves "10 repos but only 3 relevant" without filtering

### Primitive 3: Tags for Cross-Cutting Views

Simple labels on repos for filtering and prioritization:

```yaml
repositories:
  - path: ~/finances
    tags: [finances, monthly-cadence, high-priority]
  - path: ~/notes
    tags: [knowledge, daily-cadence]

tag_weights:
  high-priority: 100
  monthly-cadence: 50
```

Press `t` in the TUI to filter by tag. Weighted tags affect home page sorting.

**Why this is elegant:**
- Tags are just strings — no complex taxonomy
- Optional — repos without tags work fine
- Compose with other primitives (status scripts can check tags)
- Read-only primitive — tags just GROUP

### Primitive 4: Session Memory

Grove remembers:
- Last workspace used
- Last repo focused
- Last view mode (detail pane, search results)
- What you were doing (viewing file, running command)

On reopen, defaults to last state. Keybinding to "jump back to where you were before this detour."

**Why this is elegant:**
- State, not config — just works
- Solves "what was I doing?" without explicit save
- Invisible when not needed

### Primitive 5: Capture Routing with Contexts

```yaml
capture:
  default_inbox: ~/notes/inbox/
  routes:
    - prefix: "@finances"
      path: ~/finances/notes/
      template: "expense"

    - prefix: "@grove"
      path: ~/grove-brainstorming/ideas/

    - prefix: "@todo"
      path: ~/notes/todo/
      template: "task"
```

From anywhere:
```bash
grove capture "@finances lunch $15"
grove capture "@todo review grove design"
grove capture "random thought"  # → default inbox
```

Makes Grove a **universal capture inbox for the entire workspace**.

**Why this is elegant:**
- Declarative routing — simple prefix map
- Prefixes are just strings
- Composes with workspace concept (each workspace defines routes)

### Primitive 6: Shell-Based Extensibility Everywhere

The meta-primitive: **Grove doesn't hard-code logic; it runs scripts.**

- Status = shell commands that emit signals
- Priority = shell commands that emit numbers
- Capture templates = shell commands that generate content
- Commands = shell commands that do work (graft already does this)

Grove becomes an orchestrator that:
- Knows which repos exist
- Runs scripts in the right context
- Aggregates results
- Displays them usefully

Custom logic lives in shell scripts, not Grove's config DSL.

---

## Home Page Vision

With these primitives, the home page becomes attention-based:

```
┌─ Grove: personal workspace ─────────────────────────────────┐
│                                                              │
│ Needs Attention (2):                                         │
│   ● personal-finances  ⚠️ Monthly close overdue (32 days)   │
│   ● general-notes      📥 12 captures to organize           │
│                                                              │
│ Active (1):                                                  │
│   ● grove-brainstorming  main · 2 uncommitted files         │
│                                                              │
│ Clean (3):                                                   │
│   graft-knowledge, meta-knowledge-base, tax-prep            │
│                                                              │
│ Recent Activity:                                             │
│   2h ago  Captured "grove should support configurable..."   │
│   3h ago  Committed "Update vertical slices" (graft-know…)  │
│   1d ago  Captured "@finances dinner $45"                   │
│                                                              │
│ Quick Actions:                                               │
│   c  Capture     /  Search     w  Switch workspace          │
│   t  Filter by tag              r  Run recent command       │
└──────────────────────────────────────────────────────────────┘
```

Surfaces what matters, provides context about recent work, offers quick actions.

---

## Agentic Integration with Primitives

**Before agent session:**
```bash
grove status --json  # Structured workspace state
```
Agent gets context about the entire workspace, including custom status signals.

**During agent session:**
Agent queries Grove's search index, understands dependencies. Grove is the context layer the agent consumes (passive) or queries via MCP (active).

**After agent session:**
```bash
grove capture "@done reviewed finances, categorized transactions, generated report"
```
Activity log of agent work.

**Status scripts as agent integration:**
A status script could check for agent-created marker files:
```yaml
status:
  - name: overdue-review
    run: |
      if [ -f .needs-review ]; then
        echo "🤖 Pending agent review"
      fi
```

Integration via files and exit codes — no special protocol needed.

---

## Core Design Principle

**The primitives are extension points, not features:**

- Status scripts = "how does this repo know it needs attention?"
- Workspaces = "which repos go together for this context?"
- Tags = "how do I slice repos across workspaces?"
- Capture routes = "where does this thought belong?"
- Session memory = "what was I doing?"

Grove provides infrastructure (git integration, TUI, search, command execution) and **glue** (running scripts, aggregating results, providing views). The user provides domain knowledge (what "overdue" means for finances, what "inbox overflow" means for notes).

This keeps Grove small and focused while making it genuinely useful as a workflow hub.

---

## Impact on Vertical Slices

Every slice should have a **CLI/machine-readable interface** alongside the TUI:

| Slice | CLI/Agent Integration |
|-------|----------------------|
| 1 - Repo list | `grove status --json` → agent-queryable workspace summary |
| 2 - Detail pane | Data shown is context for agents |
| 3 - Quick capture | `grove capture "@repo message"` as CLI command |
| 4 - File nav + $EDITOR | `$EDITOR` could be an agent session |
| 5 - Graft metadata | Dependency context feeds agents |
| 6 - Search | `grove search --json` for agent queries |
| 7 - Commands | Agent sessions are just commands |

The engine layer already separates logic from display; the CLI is a thin wrapper over the engine, same as the TUI.

---

## Open Questions

1. **Status script format**: Should these live in workspace.yaml, per-repo .grove.yaml, or both?
2. **Status script caching**: Run on every render, or cache with invalidation rules?
3. **MCP server priority**: How soon should Grove support MCP for real-time agent queries?
4. **Background daemon**: Should Grove run as a daemon to maintain search index and status cache, or rebuild on launch?
5. **Capture CLI vs TUI**: Should capture be primarily a CLI command that can be called from anywhere, or primarily a TUI feature?
6. **Workspace discovery**: Explicit workspace file selection vs auto-discovery based on current directory?

---

## Next Steps

1. **Validate primitives** - Do these six primitives cover the "workflow hub" use case without gaps?
2. **Prototype status scripts** - Implement the shell-based status primitive in Slice 1 or 2
3. **CLI-first design** - Ensure every slice ships with a `--json` output mode for agent consumption
4. **Multiple workspace support** - Add workspace switching to the implementation plan
5. **Capture routing** - Expand Slice 3 to include prefix-based routing

---

## Sources

- [Grove Vertical Slices (2026-02-06)](./2026-02-06-grove-vertical-slices.md) - Seven end-to-end slices for building Grove
- [Workspace UI Exploration (2026-02-06)](./2026-02-06-workspace-ui-exploration.md) - Original Grove architecture and phasing
- [Graft Architecture](../docs/specifications/architecture.md) - Commands and change model
- [Graft Use Cases](../docs/specifications/use-cases.md) - Use Case 5: AI collaboration on upgrades
