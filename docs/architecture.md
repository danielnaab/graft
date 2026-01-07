# Information Architecture

**Status**: stable
**Last Updated**: 2026-01-04

## Overview

This document describes how information is organized in the graft repository to support effective distributed agentic coding while maintaining clarity for human developers.

## Principles

Following meta-knowledge-base conventions:

1. **Authority**: Specifications in `/home/coder/graft-knowledge/` are canonical
2. **Provenance**: Ground implementation claims in sources
3. **Lifecycle**: Mark document status (draft/working/stable/deprecated)
4. **Write Boundaries**: Clear zones for safe agent editing
5. **Simplicity**: Avoid unnecessary complexity

## Directory Structure

```
graft/
├── README.md                    # User-facing: Getting started, features, usage
├── CHANGELOG.md                 # User-facing: Version history (if published)
├── tasks.md                     # Active work tracking (agent-writable)
│
├── docs/                        # Authoritative documentation
│   ├── README.md                # Documentation index and architecture
│   ├── architecture.md     # This file
│   ├── agents.md                # Agent entrypoint and conventions
│   └── decisions/               # Architectural Decision Records (ADRs)
│       └── 001-*.md
│
├── notes/                       # Time-bounded development notes
│   ├── YYYY-MM-DD-topic.md      # Dated session logs, exploration
│   └── archive/                 # Old notes (optional)
│
├── status/                      # Project status snapshots
│   ├── implementation.md # Detailed implementation status
│   ├── gap-analysis.md          # Analysis vs specifications
│   ├── continue-here.md         # Quick session continuity
│   └── sessions/                # Detailed session logs
│       └── SESSION_LOG_*.md
│
└── src/graft/                   # Source code
    └── ... (production code)
```

## Information Flow

```
Specifications (graft-knowledge)
        ↓
    Tasks (tasks.md)
        ↓
Notes (scratch work, exploration)
        ↓
Status Docs (implementation tracking)
        ↓
Authoritative Docs (docs/)
        ↓
User Documentation (README.md)
```

## Document Types and Lifecycle

### 1. Scratch Notes (`notes/`)

**Purpose**: Time-bounded exploration, session logs, scratch work

**Lifecycle**: draft → working → archived

**Naming**: `YYYY-MM-DD-topic.md`

**Examples**:
- `2026-01-03-specification-sync.md` - Planning exploration
- `2026-01-04-dogfooding-session.md` - Testing notes

**Agent Permissions**: ✅ Read, ✅ Write

**When to Create**: Any exploration, testing, or planning work

**When to Archive**: After content is distilled into authoritative docs

---

### 2. Task Tracking (`tasks.md`)

**Purpose**: Active work queue for distributed agents

**Lifecycle**: Always current (tasks move from backlog → done)

**Format**: Simple markdown with task IDs and checkboxes

**Agent Permissions**: ✅ Read, ✅ Write

**Structure**:
```markdown
## Next Up (Priority Order)
- [ ] #001: Task name (Owner: name, Est: 4h)

## In Progress
- [ ] #002: Task name (Owner: name, Started: 2026-01-04)

## Done (Recent)
- [x] #003: Task name (Completed: 2026-01-04)
```

**Usage**:
- Agents pick tasks from "Next Up"
- Move to "In Progress" when starting
- Move to "Done" when complete
- Add new tasks to "Next Up" or "Backlog"

---

### 3. Status Documents (`status/`)

**Purpose**: Snapshot current state for session continuity

**Lifecycle**: draft → working → stable → deprecated

**Agent Permissions**: ✅ Read, ✅ Write (with clear ownership)

**Key Documents**:

- **continue-here.md**: Quick session continuity
  - Status: working (updated each session)
  - Use: First file to read when continuing work
  - Contains: Quick context, metrics, what's done, what's next

- **implementation.md**: Detailed implementation tracking
  - Status: working (updated at milestones)
  - Use: Comprehensive phase-by-phase status
  - Contains: All phases, metrics, completion details

- **gap-analysis.md**: Comparison to specifications
  - Status: stable (point-in-time analysis)
  - Use: Understand what's implemented vs specified
  - Contains: Detailed gap analysis, recommendations

- **sessions/SESSION_LOG_*.md**: Detailed session logs
  - Status: stable (historical record)
  - Use: Review what happened in past sessions
  - Contains: Commits, decisions, issues encountered

---

### 4. Authoritative Documentation (`docs/`)

**Purpose**: Permanent reference documentation

**Lifecycle**: draft → stable → deprecated

**Agent Permissions**: ✅ Read, ⚠️ Write (with care - these are long-lived)

**Key Documents**:

- **README.md**: Documentation index
  - Links to all other documentation
  - Architecture overview
  - Quick reference

- **architecture/**: Architecture documentation
  - Design patterns
  - System structure
  - Component relationships

- **decisions/**: Architectural Decision Records (ADRs)
  - Format: `NNN-title.md` (e.g., `001-error-handling-strategy.md`)
  - Immutable once stable
  - Documents: Context, Decision, Consequences

- **guides/**: Implementation guides (future)
  - How-to guides for common tasks
  - Development workflows
  - Troubleshooting

---

### 5. User-Facing Documentation (Root)

**Purpose**: First impression for users

**Lifecycle**: stable (only updated for releases/major changes)

**Agent Permissions**: ✅ Read, ⚠️ Write (major changes only)

**Key Documents**:

- **README.md**: Getting started, features, usage
- **CHANGELOG.md**: Version history (if published)
- **CONTRIBUTING.md**: How to contribute (future)

---

## Task Management System

### Task Format

Each task has:
- **ID**: Sequential number (#001, #002, etc.)
- **Title**: Brief description
- **Priority**: High/Medium/Low
- **Effort**: Estimated hours
- **Owner**: Agent or human (or "unassigned")
- **Status**: Next Up / In Progress / Done / Blocked
- **Dependencies**: Task IDs this depends on (optional)

### Task Workflow

```
Created → Backlog → Next Up → In Progress → Done
                        ↓
                    Blocked (with reason)
```

### Agent Task Protocol

When picking up a task:
1. Find unassigned task in "Next Up"
2. Move to "In Progress" with owner and start date
3. Create scratch notes in `notes/YYYY-MM-DD-task-name.md`
4. Do the work
5. Update relevant status docs
6. Commit with clear message
7. Move task to "Done" with completion date
8. Add any follow-up tasks discovered

### Creating New Tasks

Tasks come from:
- Gap analysis (specification compliance)
- User requests
- Bug reports
- Technical debt discovered during work
- Follow-up work from completed tasks

Add tasks to tasks.md in appropriate priority section.

---

## Agent Guidelines

### Safe Editing Zones

**Always Safe**:
- ✅ `notes/` - Scratch work, exploration
- ✅ `tasks.md` - Task tracking
- ✅ Status docs in `status/` (continue-here.md, implementation.md)

**Edit with Care**:
- ⚠️ `docs/` - Authoritative documentation (verify changes)
- ⚠️ `README.md` - User-facing (major changes only)
- ⚠️ ADRs - Immutable once stable (create new instead of editing)

**Read-Only**:
- ❌ `/home/coder/graft-knowledge/` - Specifications (external authority)

### Document Status Tags

Add to frontmatter:
```yaml
---
status: draft | working | stable | deprecated
last_updated: YYYY-MM-DD
---
```

### When to Update What

**After completing a task**:
- ✅ Move task in tasks.md to "Done"
- ✅ Update continue-here.md metrics if significant
- ✅ Commit with clear message

**After completing a phase**:
- ✅ Update implementation.md
- ✅ Update continue-here.md
- ✅ Create session log in status/sessions/
- ✅ Consider if README.md needs updates

**After significant architectural decision**:
- ✅ Create ADR in docs/decisions/
- ✅ Update docs/README.md if needed

---

## File Naming Conventions

### Notes
- Format: `YYYY-MM-DD-topic-name.md`
- Example: `2026-01-04-json-output-implementation.md`

### Session Logs
- Format: `SESSION_LOG_YYYY-MM-DD.md`
- Example: `SESSION_LOG_2026-01-04.md`

### ADRs
- Format: `NNN-decision-title.md` (sequential)
- Example: `001-error-handling-strategy.md`, `002-snapshot-strategy.md`

### Tasks
- Single file: `tasks.md` (avoids merge conflicts)
- Task IDs: `#NNN` (sequential: #001, #002, etc.)

---

## Information Cleanup

### When to Archive

**Notes**: After content distilled into authoritative docs (move to `notes/archive/`)

**Session Logs**: Keep all (historical record)

**Status Docs**:
- continue-here.md - Keep current, archive old versions
- implementation.md - Keep current, track in git history
- gap-analysis.md - Keep as point-in-time snapshot

### When to Consolidate

If documentation becomes fragmented:
1. Create issue/task for consolidation
2. Plan structure
3. Migrate content systematically
4. Update references
5. Archive old docs (don't delete - history matters)

---

## Tools and Automation

### TodoWrite Tool

The TodoWrite tool in Claude Code is for **session-local** task tracking:
- Use for tasks within a single session
- Different from tasks.md (persistent across sessions)
- Good for: "Run tests", "Fix linting", "Commit changes"

### tasks.md

The tasks.md file is for **project-wide** task tracking:
- Persists across sessions
- Shared between agents
- Good for: "Add JSON output", "Implement fetch command"

### Git Commits

Every significant change should be committed with:
- Clear message describing what changed
- Reference to task ID if applicable (e.g., "Implement #001: JSON output")
- Co-authored tag for AI work

---

## Examples

### Example: Implementing a New Feature

1. **Discovery Phase**
   - Create: `notes/2026-01-04-json-output-exploration.md`
   - Explore: Read code, check specs, plan approach
   - Document: Findings and approach

2. **Task Creation**
   - Add to tasks.md: "#004: Add JSON output to status command"
   - Set priority: High
   - Estimate effort: 4h

3. **Implementation Phase**
   - Move task to "In Progress" in tasks.md
   - Implement feature
   - Add tests
   - Update notes with decisions made

4. **Documentation Phase**
   - Update README.md with new --json flag
   - Update docs/README.md if architecture changed
   - Create ADR if significant decision made

5. **Completion Phase**
   - Commit with message: "Implement #004: Add JSON output to status command"
   - Move task to "Done" in tasks.md
   - Update continue-here.md metrics
   - Archive notes (optional)

---

## Migration from Current State

Current state has several docs at root:
- continue-here.md
- implementation.md
- phase-8.md
- gap-analysis.md
- SESSION_LOG_2026-01-03.md
- workflow-validation.md

**Plan**:
1. Create `status/` directory
2. Move status docs to `status/`
3. Move session logs to `status/sessions/`
4. Keep continue-here.md at root for visibility (symlink or copy)
5. Update references in other docs
6. This migration is itself a task!

---

## References

- Meta-KB Authority Policy: `/home/coder/meta-knowledge-base/policies/authority.md`
- Meta-KB Provenance Policy: `/home/coder/meta-knowledge-base/policies/provenance.md`
- Meta-KB Lifecycle Policy: `/home/coder/meta-knowledge-base/policies/lifecycle.md`
- Graft Specifications: `/home/coder/graft-knowledge/docs/specification/`

---

**Questions? Improvements?**

This is a living document. If you find issues with this information architecture, create a task to improve it.
