---
deps:
  - docs/explorations/architecture/00-sources/current-implementation.md
  - docs/explorations/architecture/00-sources/design-goals.md
  - docs/explorations/architecture/00-sources/open-questions.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Deep Exploration: Lock Mechanism

You are a systems architect analyzing how graft should handle "locked" grafts that prevent expensive regenerations.

## Your Task

Think deeply about the lock mechanism: why it's needed, how it should work, what the edge cases are, and how to make it intuitive and safe.

### The Problem

Some grafts are expensive:
- Require many LLM calls
- Generate large amounts of content
- Represent point-in-time explorations

Once generated, you may want to "freeze" them:
- Preserve the exact analysis from a specific moment
- Prevent accidental regeneration
- Keep historical snapshots
- Avoid burning API credits

### Current Behavior

Currently, when dependencies change:
```
source.md changes → DVC detects → graft regenerates → output changes
```

**There's no way to prevent this.**

### The Lock Proposal

From TODO.md:
```yaml
---
lock: true
deps: [sources.md]
---
```

**Intended behavior**: "Don't regenerate this graft, even if deps change"

But this raises many questions...

### Design Questions

#### 1. What Does Lock Mean?

**Option A: "Never regenerate"**
- Once locked, graft is frozen forever
- Ignore all dependency changes
- Require explicit unlock to update

**Option B: "Don't auto-regenerate"**
- Skip in `dvc repro`
- But allow `dvc repro -f <stage>` to force
- Or `graft regenerate --force <name>`

**Option C: "Prompt before regenerating"**
- DVC can't prompt (it's automated)
- But graft wrapper could check and ask

**Option D: "Track lock age"**
```yaml
---
lock: true
lock_date: 2025-11-07
lock_reason: "Point-in-time analysis for Q4 planning"
---
```
- Make locks visible and intentional
- Easier to review and unlock later

#### 2. DVC Integration

How does locking interact with DVC?

**Option A: Exclude from dvc.yaml**
- Locked grafts don't generate stages
- Outputs exist but aren't tracked
- Problem: How to lock/unlock? Regenerate dvc.yaml?

**Option B: Mark stage as "frozen"**
- Stage exists in dvc.yaml
- But has special marker that DVC respects
- Problem: DVC doesn't have this concept

**Option C: Dependencies hack**
```yaml
stages:
  locked_report:
    cmd: echo "This graft is locked. Use --force to regenerate."
    deps: []  # No deps = never considered out of date
    outs: [report.md]
```
- Clever: no deps means never regenerates
- But loses dependency tracking

**Option D: Separate workflow**
```yaml
# dvc.yaml
stages: {active grafts}

# dvc.lock.yaml
locked_stages: {locked grafts}
```

#### 3. Source Availability

**Scenario**: Graft is locked, but sources are deleted/moved.

```
report.graft.md (locked)
  deps: [data-2024.csv]

data-2024.csv is deleted.
```

**What should happen?**

**Option A: Lock is semantic, not validation**
- Locked = "don't regenerate"
- Missing sources are irrelevant
- Output is preserved regardless

**Option B: Lock requires source preservation**
- Locked grafts must have sources available
- Error if sources are missing
- Forces archival of source data

**Option C: Lock as snapshot**
- Locking a graft creates an immutable copy of all sources
- Stored in `locked-sources/<graft-name>/`
- Never depends on external files changing

#### 4. Unlocking

How do you unlock a graft?

**Option A: Edit frontmatter**
```yaml
---
lock: false  # or remove the field
---
```
- Simple, explicit
- But requires manual edit

**Option B: Command**
```bash
graft unlock report
graft unlock --all
```
- Convenient
- But now graft mutates source files

**Option C: Override flag**
```bash
graft regenerate --force report
graft regenerate --ignore-locks
```
- Doesn't actually unlock
- One-time override

#### 5. Per-File vs Per-Graft

For multi-file grafts:
```
exploration/graft.md
  → option-a.md
  → option-b.md
  → option-c.md
```

**Can you lock individual outputs?**

```yaml
---
outputs:
  - path: option-a.md
    lock: true
  - path: option-b.md
    lock: false
  - path: option-c.md
    lock: true
---
```

**Or is lock graft-level?**
```yaml
---
lock: true  # All outputs are locked
---
```

#### 6. Visibility and Discoverability

How do users know a graft is locked?

**In frontmatter**:
```yaml
---
lock: true
lock_reason: "Q4 2024 analysis - historical record"
---
```

**In output**:
```markdown
<!-- This document is locked. -->
<!-- Generated: 2024-11-07 -->
<!-- Sources: data-2024.csv (hash: abc123) -->
```

**In build artifacts**:
```json
{
  "name": "q4-analysis",
  "locked": true,
  "lock_date": "2024-11-07",
  "lock_reason": "Historical snapshot"
}
```

**Command to list**:
```bash
graft status
# Locked grafts:
#   - reports/q4-analysis.graft.md (locked 2024-11-07)
```

#### 7. Semantic Meaning

What does it *mean* for a graft to be locked?

**Interpretation A: "This is historical"**
- Represents a specific point in time
- Valuable as-is, don't alter
- Like a git tag for a document

**Interpretation B: "This is expensive"**
- Don't regenerate without explicit approval
- Protect against accidental API costs
- Like a build cache

**Interpretation C: "This is complete"**
- Work is done, no more changes needed
- Archived for reference
- Like marking a JIRA ticket "Closed"

**Interpretation D: "This is fragile"**
- Dependencies may have changed
- Regeneration might break things
- Preserve what works

### Use Case Analysis

#### Use Case 1: Point-in-Time Exploration
```
naming-exploration/
  recommendation.graft.md (locked after decision made)
```

**Behavior wanted**:
- Lock after decision is made
- Keep as historical record
- Don't regenerate even if new criteria emerge
- Clear indicator: "This was our thinking on 2024-11-07"

#### Use Case 2: Expensive Deep Dive
```
architecture-analysis/
  deep-dive.graft.md
  (50 pages, took $20 in API costs to generate)
```

**Behavior wanted**:
- Lock after generation
- Don't accidentally regenerate
- But allow intentional updates with `--force`
- Maybe warn: "This will cost ~$20 to regenerate. Continue?"

#### Use Case 3: Incrementally Updated Series
```
weekly-reports/
  2024-11-01-report.graft.md (locked)
  2024-11-08-report.graft.md (locked)
  2024-11-15-report.graft.md (locked)
  2024-11-22-report.graft.md (active)
```

**Behavior wanted**:
- Lock each week's report after publishing
- Keep historical record unchanged
- Current week remains unlocked
- Clear pattern: locked = published

### Edge Cases

1. **Dependency chain**:
   ```
   A.graft.md (locked) → produces data.json
   B.graft.md → depends on data.json
   ```
   If A is locked but sources change, does B regenerate with stale data.json?

2. **Lock conflict**:
   ```
   report.graft.md
   ---
   lock: true
   ---
   ```
   User runs `graft regenerate-all --force`

   Should this skip locked grafts or override them?

3. **Partial lock**:
   ```
   Multi-file graft: 10 outputs
   8 are locked, 2 are not
   ```
   What does this even mean?

4. **Lock without output**:
   ```
   New graft with lock: true but never generated yet
   ```
   Should this be an error?

### Trade-off Analysis

| Approach | Clarity | Safety | Flexibility | Implementation | DVC Integration |
|----------|---------|--------|-------------|----------------|-----------------|
| No lock (current) | ✓✓✓ | ✗ | ✓ | ✓✓✓ | ✓✓✓ |
| Simple lock flag | ✓✓ | ✓✓ | ✓ | ✓✓ | ✓ |
| Exclude from DVC | ✓✓ | ✓✓✓ | ✗ | ✓ | ✗ |
| Lock with metadata | ✓✓✓ | ✓✓ | ✓✓ | ✓ | ✓ |
| Per-file locking | ✓ | ✓ | ✓✓✓ | ✗ | ✗ |

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: Recommended lock mechanism and rationale
2. **Semantic Meaning**: What does "locked" mean to users?
3. **Design Proposal**: Specific implementation with examples
4. **DVC Integration**: How does this work with the build system?
5. **User Experience**: How do users lock, unlock, and discover locks?
6. **Use Case Validation**: Does this solve the real problems?
7. **Edge Cases**: What weird scenarios might occur?
8. **Trade-offs**: Honest assessment of approaches
9. **Recommendation**: Clear, justified design
10. **Open Questions**: What needs user validation?

Think like a product designer. The feature should be obvious in purpose, simple in use, and safe by default.
