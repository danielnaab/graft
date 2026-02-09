---
title: "Brainstorming: Upgrade and Evolution Mechanisms"
date: 2026-01-01
status: working
participants: ["human", "agent"]
---

# Brainstorming: Upgrade and Evolution Mechanisms

## Context

How should Graft handle dependency upgrades and software evolution? The goal is to help downstream consumers stay current with upstream changes through automated, deterministic methods where possible, with human oversight where appropriate.

**Core principle**: Facilitate evolvable software with clean abstractions that work for both humans and AI coding agents.

## Key Requirements

- Intuitive way to discover and apply updates
- Surface changelogs and upgrade paths
- Automate repetitive migration tasks
- Provide rich context for AI-assisted upgrades
- Remain consistent with traditional SDLC practices
- Keep abstractions simple and general-purpose

## Design Evolution

### Initial Ideas (Abandoned)

Early exploration included:
- Separate `migration.yaml` format with built-in rules
- Baked-in find/replace logic in Graft
- Separate `--agent` flag for AI interface
- Complex structured changelog parsing

**Problem**: Too much special-case logic, too complex, not flexible enough.

### Key Insight: Migrations Are Commands

Since Graft is already a task runner, migrations can just be **commands** defined in the dependency's `graft.yaml`:

```yaml
# In dependency's graft.yaml
commands:
  migrate-v1-to-v2:
    run: "./scripts/migrate.sh"
    description: "Migrate consumer from v1.x to v2.0"
```

Benefits:
- No special formats needed
- Dependency controls migration logic
- Uses existing infrastructure
- Flexible (any executable)
- Same interface for humans and AI

### Unified Interface Principle

Don't design separate interfaces for humans vs. AI. Instead:
- Name operations semantically
- Make information clear and actionable
- Both humans and AI use the same commands
- AI can read the same changelogs humans read (if structured well)

## Core Concept: Changes as a Versioned Stream

Think of upstream evolution as a **stream of semantic changes** that downstream consumers apply incrementally.

Analogies:
- **Git commits**: Atomic, sequential, reversible
- **Database migrations**: Apply in order, track what's applied
- **RSS feeds**: Subscribe, consume new items

The dependency publishes changes, consumers consume them incrementally.

## Minimal Design Proposal

### What Dependencies Provide

1. **Structured CHANGELOG.md**
   - Human-readable Markdown
   - Follows conventions that make it AI-parseable
   - Each entry includes:
     - What changed (description)
     - Why it changed (rationale)
     - How to adapt (migration guidance)
     - How to verify (test strategy)

2. **Git tags with semantic versioning**
   - Clear version boundaries
   - Enables "changes between X and Y" queries

3. **Optional migration commands** (in graft.yaml)
   - Automate repetitive changes
   - Consumer can run: `graft <dep>:migrate-v2`

### What Consumers Track

**Lock file** (`graft.lock`):
```yaml
resolved:
  meta-knowledge-base:
    url: "ssh://..."
    ref: "main"
    commit: "abc123..."
    version: "1.5.0"  # Last version applied
    updated_at: "2026-01-01T10:00:00Z"
```

The `version` field means: "I've consumed all changes through v1.5.0"

### What Graft Provides

Core operations:
- `graft status` - Check for available updates
- `graft changes <dep>` - View changelog entries
- `graft apply <dep>` - Update to newer version
- `graft <dep>:<command>` - Run dependency commands

Workflow:
```bash
# Discover updates
$ graft status
meta-knowledge-base: 1.5.0 → 2.0.0 available

# Review changes
$ graft changes meta-knowledge-base
[Shows changelog from 1.5.0 to 2.0.0]

# Apply update
$ graft apply meta-knowledge-base
✓ Updated graft.lock
Next: Run migration if needed

# Run migration
$ graft meta-knowledge-base:migrate-v2
```

## AI Integration Points

Where AI coding agents can help:

1. **Change analysis**: Read structured changelog, understand impact
2. **Impact assessment**: Scan consumer codebase for affected code
3. **Migration execution**: Run migration commands or propose manual changes
4. **Verification**: Run tests, check for errors
5. **Documentation updates**: Update consumer docs to reflect changes

The dependency provides structured metadata (changelog, migration commands).
The AI uses this context to propose changes.
The human reviews and approves.

## Properties That Enable AI Effectiveness

### 1. Addressable Changes
Each changelog entry is a discrete, versioned unit that can be reasoned about independently.

### 2. Machine-Readable Semantics
Structure conveys meaning:
- `BREAKING:` prefix = must adapt code
- `Feature:` prefix = optional to adopt
- `Fix:` prefix = likely no action needed
- **Migration** section = actionable steps
- **Verification** section = test strategy

### 3. Incremental Application
Like database migrations:
- Apply changes sequentially
- Track what's been applied
- Can't skip required migrations

### 4. Context-Rich
Each entry provides:
- **What** changed (technical details)
- **Why** it changed (rationale)
- **How** to adapt (migration steps)
- **When** it's correct (verification)

### 5. Composable
- Commands can call other tools
- Small discrete changes > large batch changes
- Each change independently verifiable

## Example Changelog Format

```markdown
# Changelog

## [2.0.0] - 2026-01-01

### BREAKING: Renamed getUserData → fetchUserData

**Rationale**: Clarify that this function performs async I/O

**Impact**: All call sites must be updated

**Migration**:
- Automatic: Run `graft dep:migrate-v2`
- Manual: Find/replace `getUserData(` → `fetchUserData(`
- Files: `**/*.{js,ts}`

**Verification**:
- Run test suite - should pass
- Check for "getUserData is not defined" errors

---

### Feature: Added response caching

**Description**: API responses now cached for 5min by default

**Migration**: None required (backward compatible)

**Optional**: Configure `cache.ttl` to customize

**Verification**: Repeated calls should be faster
```

This format:
- ✅ Human-readable
- ✅ AI-parseable (clear structure, conventional headings)
- ✅ Actionable (explicit migration steps)
- ✅ Verifiable (test strategy included)

## Alignment with Traditional Practices

Uses existing conventions:
- ✅ CHANGELOG.md files
- ✅ Semantic versioning
- ✅ Lock files
- ✅ Migration scripts (like DB migrations, codemods)

Enhanced for AI:
- 📊 Structured format within Markdown
- 🎯 Explicit migration guidance
- ✓ Verification strategies
- 🔄 Incremental application tracking

## Open Questions

1. **Changelog format enforcement**
   - Strict schema vs. conventional Markdown?
   - Validation in dependency CI?
   - Tooling to generate from commits?

2. **Transitive dependencies**
   - If A → B → C, and C updates, how does B handle it?
   - Cascade updates? Lock transitive deps?

3. **Version acknowledgment**
   - Separate "reviewed" vs. "applied" states?
   - Or is upgrade implicit acknowledgment?

4. **Rollback strategy**
   - If update breaks things, how to revert?
   - Just git revert the lock file?

5. **Command execution model**
   - Where do migration commands run?
   - How do they access consumer files?
   - Security/sandboxing?

6. **Multi-step upgrades**
   - Jumping from v1.0 to v3.0 - apply intermediate migrations?
   - Or single migration that handles the gap?

## Next Steps

- [ ] Document use cases for upgrade/evolution workflows
- [ ] Create decision record on chosen approach
- [ ] Design lock file format in detail
- [ ] Specify changelog conventions
- [ ] Define command execution model

## Conclusion (2026-01-01 Evening)

After extensive discussion and analysis, the design has been refined and finalized:

### Key Decisions Made

1. **Git refs over semver** ([Decision 0002](../docs/specifications/decisions/decision-0002-git-refs-over-semver.md))
   - Changes identified by any git ref (commit, tag, branch)
   - No requirement for semantic versioning
   - Optional semver awareness for convenience

2. **Explicit change declarations** ([Decision 0003](../docs/specifications/decisions/decision-0003-explicit-change-declarations.md))
   - Changes defined in graft.yaml, not parsed from markdown
   - Deterministic, validatable, reliable
   - CHANGELOG.md remains for human context

3. **Atomic upgrades** ([Decision 0004](../docs/specifications/decisions/decision-0004-atomic-upgrades.md))
   - All-or-nothing operations
   - No intermediate states
   - Rollback on failure

### Minimal Primitives

The design converged on two core primitives:

1. **Change**: git ref + optional metadata (type, migration, verify)
2. **Dependency**: source + consumed ref + commit hash

Everything else is metadata or operations built on these primitives.

### Architecture

The finalized architecture positions Graft as:
- **Higher-order change tracking** (like git for code, Graft for semantic changes)
- **Git-native** (leverages existing primitives)
- **Minimal and composable** (simple abstractions that combine powerfully)
- **Tool-agnostic** (works with any migration tool via commands)

### Implementation-Ready

Documentation has been created:
- **4 decision records** capturing architectural choices
- **4 specification documents** ready for implementation:
  - [Change Model](../docs/specifications/graft/change-model.md)
  - [graft.yaml Format](../docs/specifications/graft/graft-yaml-format.md)
  - [Lock File Format](../docs/specifications/graft/lock-file-format.md)
  - [Core Operations](../docs/specifications/graft/core-operations.md)
- **Updated architecture** document with complete system design

The Python implementation can now proceed based on these specs.

### Key Insights

1. **Parsing is brittle** - Explicit declarations are more reliable than markdown parsing
2. **Installation = Consumption** - Separating these states adds unnecessary complexity
3. **Git already solved this** - Use git primitives instead of inventing new ones
4. **Simple is powerful** - Two primitives are sufficient for the entire system

## Related

- [Architecture](../docs/specifications/architecture.md)
- [Decision 0001: Initial Scope](../docs/specifications/decisions/decision-0001-initial-scope.md)
- [Decision 0002: Git Refs Over Semver](../docs/specifications/decisions/decision-0002-git-refs-over-semver.md)
- [Decision 0003: Explicit Change Declarations](../docs/specifications/decisions/decision-0003-explicit-change-declarations.md)
- [Decision 0004: Atomic Upgrades](../docs/specifications/decisions/decision-0004-atomic-upgrades.md)
