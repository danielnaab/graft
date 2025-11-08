---
deps:
  - architecture-exploration/00-sources/current-implementation.md
  - architecture-exploration/00-sources/design-goals.md
  - architecture-exploration/00-sources/open-questions.md
  - architecture-exploration/01-explorations/lock-mechanism.md
  - architecture-exploration/01-explorations/determinism-and-caching.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Deep Exploration: Lock Mechanism in Context of Determinism

You are a systems architect analyzing how graft's lock mechanism should interact with deterministic vs non-deterministic jobs.

## Your Task

The lock mechanism exploration identified the need to prevent expensive regenerations. The determinism exploration revealed that some jobs are inherently non-deterministic. These two concerns deeply interact.

Think comprehensively about how locking and determinism work together to create an intuitive, powerful system.

### The Interaction

**Deterministic jobs** + **Lock**:
```yaml
---
deps: [code.js]
deterministic: true
lock: true
---
```
- Clear semantics: "This output is frozen, don't regenerate even if code.js changes"
- Use case: Historical snapshot of code state
- Rationale: Preserve specific point-in-time analysis

**Non-deterministic jobs** + **Lock**:
```yaml
---
deps: [brief.md]
deterministic: false
temperature: 0.7
lock: true
---
```
- Different semantics: "This is ONE sample from the distribution, preserve it"
- Use case: Keep a specific creative exploration that user likes
- Rationale: Re-running would produce different output, want to keep this one

**Non-deterministic jobs** + **No Lock**:
```yaml
---
deps: [data-query.sql]
deterministic: false
cache_ttl: 24h
---
```
- Semantics: "Regenerate periodically even if inputs don't change"
- Use case: Daily reports, market analysis
- Rationale: External data changes, want fresh output

### Key Questions

#### 1. Lock Semantics by Determinism Type

Should lock have different meanings for deterministic vs non-deterministic jobs?

**For deterministic jobs**:
```yaml
lock: true
# Means: "This output is the result of specific inputs at a point in time.
#         Don't regenerate even if inputs change."
# Example: "Q4 2024 code analysis" - preserve analysis of Q4 code state
```

**For non-deterministic jobs**:
```yaml
lock: true
# Means: "This output is ONE sample I want to preserve.
#         Re-running could produce different output, but I like this one."
# Example: "Creative naming brainstorm" - keep this particular set of ideas
```

**Should this be explicit?**
```yaml
---
lock: true
lock_type: snapshot  # or 'favorite' or 'historical'
lock_reason: "Q4 planning decisions were based on this analysis"
---
```

#### 2. Interaction with TTL and Cache Strategies

For non-deterministic jobs with cache_ttl:
```yaml
---
deps: [query.sql]
deterministic: false
cache_ttl: 24h
lock: true
---
```

**What does this mean?**

**Option A: Lock overrides TTL**
- Locked = never regenerate, TTL is ignored
- Clear precedence: lock > ttl

**Option B: Lock + TTL = "manual refresh only after TTL"**
- After 24h, output is "stale but locked"
- User must explicitly unlock or force refresh
- Warning: "This locked output is >24h old"

**Option C: Lock applies until TTL, then auto-unlocks**
- Temporary lock: "Keep this for 24h, then allow refresh"
- Use case: Daily reports that should be stable during the day

**Option D: Incompatible**
- Can't specify both lock and cache_ttl
- Error: "A locked graft cannot have a TTL"

#### 3. Lock Visibility and Metadata

How should locked status be communicated differently for deterministic vs non-deterministic?

**Deterministic locked output**:
```markdown
<!-- Generated: 2024-11-07 -->
<!-- Locked: true -->
<!-- Deterministic: true -->
<!-- Snapshot of dependencies at commit: abc123 -->
<!-- To regenerate: git checkout abc123 && graft regenerate --force report -->
<!-- This output is reproducible from the locked dependency state -->
```

**Non-deterministic locked output**:
```markdown
<!-- Generated: 2024-11-07 -->
<!-- Locked: true -->
<!-- Deterministic: false -->
<!-- Temperature: 0.7 -->
<!-- Warning: This is ONE sample from many possible outputs -->
<!-- Regenerating (even with same inputs) would likely produce different results -->
<!-- Lock preserves this particular sample -->
```

#### 4. Lock + Dependency Changes

**Scenario 1: Deterministic job, locked, dependency changes**
```
analysis.graft.md (deterministic, locked)
  deps: [codebase/]

codebase/ changes significantly
```

**User expectations**:
- Lock means "keep the Q4 analysis, don't update it"
- Even though code changed, this is a historical record
- Clear: locked = frozen

**Scenario 2: Non-deterministic job, locked, dependency changes**
```
creative-names.graft.md (non-deterministic, locked)
  deps: [brand-brief.md]

brand-brief.md changes (new brand requirements)
```

**User expectations**:
- Lock means "I like these name ideas, keep them"
- But wait... the brief changed, shouldn't we regenerate with new requirements?
- Less clear: lock = preserve sample, or lock = ignore new inputs?

**The Tension**: For non-deterministic jobs, lock might mean "I like this output" MORE than "freeze the inputs". Should lock be advisory vs mandatory?

#### 5. Lock Granularity with Multi-File Outputs

For a graft with multiple outputs, each potentially different determinism:

```yaml
---
outputs:
  - path: summary.md
    deterministic: true
    lock: false
  - path: creative-variations.md
    deterministic: false
    temperature: 0.8
    lock: true  # Keep these specific variations
  - path: technical-details.md
    deterministic: true
    lock: true  # Snapshot for compliance
---
```

**Questions**:
- Is per-output locking too complex?
- How does this map to DVC stages?
- What's the user mental model?

#### 6. Lock Commands and Determinism Awareness

Should lock/unlock commands be aware of determinism?

```bash
# Lock a deterministic job
graft lock analysis --reason "Q4 compliance snapshot"
# → Marks as locked, stores dependency commit hash

# Lock a non-deterministic job
graft lock creative-exploration --reason "Team liked option B"
# → Marks as locked, stores that this is ONE sample

# Show locked grafts with determinism info
graft status --locked
# Output:
#   Locked Grafts:
#   ✓ analysis.md (deterministic, locked for compliance)
#     - Dependencies at: abc123
#     - Regenerable: Yes (same inputs → same output)
#   ⚠ creative-exploration.md (non-deterministic, locked to preserve sample)
#     - Generated: 2024-11-07
#     - Regenerable: No (same inputs → different output)

# Unlock with awareness
graft unlock creative-exploration
# Warning: This is a non-deterministic graft. Regenerating will produce
#          different output. Current version will be lost. Continue? [y/N]
```

#### 7. Auto-Lock Strategies

Should graft support auto-locking based on determinism and cost?

**Example configurations**:
```yaml
# Auto-lock expensive non-deterministic jobs
---
deps: [brief.md]
deterministic: false
auto_lock: after_generation
# After first successful generation, auto-lock to prevent
# accidental expensive regeneration
---
```

```yaml
# Auto-lock on schedule
---
deps: [query.sql]
deterministic: false
cache_ttl: 24h
auto_lock: after_ttl
# After 24h, lock instead of regenerating
# Require manual unlock for fresh generation
---
```

```yaml
# Auto-lock when dependencies removed
---
deps: [historical-data-2024.csv]
deterministic: true
auto_lock: on_dep_missing
# If data-2024.csv is removed, auto-lock the output
# Preserve analysis even if source data is archived
---
```

### Use Case Analysis

#### Use Case 1: Historical Code Analysis (Deterministic + Lock)

```yaml
---
deps: [src/]
deterministic: true
lock: true
lock_date: 2024-11-07
lock_reason: "Q4 architecture review baseline"
snapshot_commit: abc123
---
```

**Behavior**:
- Output frozen at specific git commit
- Clear: this is analysis of code state at abc123
- Can regenerate if needed by: `git checkout abc123 && graft regenerate --force`
- Reproducible: same code state → same analysis

#### Use Case 2: Creative Brainstorm (Non-Deterministic + Lock)

```yaml
---
deps: [brand-brief.md]
deterministic: false
temperature: 0.9
lock: true
lock_reason: "Team selected Option B for further development"
sample_number: 3  # This was the 3rd generation attempt
---
```

**Behavior**:
- Output is ONE sample from many possibilities
- Clear: we like THIS set of ideas, don't lose them
- Cannot reproduce: regenerating produces different ideas
- Lock preserves favored sample

#### Use Case 3: Daily Report (Non-Deterministic + TTL, No Lock)

```yaml
---
deps: [query.sql, template.md]
deterministic: false
cache_ttl: 24h
lock: false
---
```

**Behavior**:
- Regenerate every 24h even if query.sql and template.md unchanged
- Clear: external data changes, need fresh data
- No lock: always willing to regenerate
- TTL-based refresh

#### Use Case 4: Weekly Reports (Deterministic + Sequential Lock)

```yaml
# Pattern for week-1, week-2, etc.
---
deps: [data/week-44.csv]
deterministic: true
lock: false  # Current week

# Previous weeks auto-lock after publish
auto_lock: on_new_week
---
```

**Behavior**:
- Current week's report is unlocked (can regenerate)
- After week ends, auto-lock
- Historical reports are frozen
- Clear pattern: published = locked

### Edge Cases

1. **Locked non-deterministic graft, user wants "another sample"**:
   ```bash
   graft sample creative-exploration --keep-original
   # Generates NEW sample, keeps locked original
   # Creates: creative-exploration-sample-2.md
   ```

2. **Deterministic graft locked, but dependencies deleted**:
   ```
   analysis.graft.md (locked)
     deps: [data.csv]

   data.csv deleted
   ```
   Should this error, warn, or allow? Lock means "don't regenerate anyway"?

3. **Lock + non-deterministic + force regenerate**:
   ```bash
   graft regenerate --force creative-names
   # Warning: This is locked AND non-deterministic.
   # Regenerating will produce different output.
   # Original will be lost unless you commit first.
   # Continue? [y/N]
   ```

4. **Temperature changes for locked job**:
   ```yaml
   # Original
   ---
   temperature: 0.7
   lock: true
   ---

   # User edits to:
   ---
   temperature: 0.9  # Changed!
   lock: true
   ---
   ```
   Does this invalidate the lock? Warn? Unlock automatically?

### Trade-off Analysis

| Lock Strategy | Deterministic Jobs | Non-Deterministic Jobs | Complexity | Safety | Flexibility |
|---------------|-------------------|------------------------|------------|--------|-------------|
| Same semantics for both | Clear, simple | Potentially confusing | ✓✓✓ | ✓✓ | ✓ |
| Different semantics | Clear per-type | Requires understanding | ✓ | ✓✓✓ | ✓✓ |
| Lock + determinism flag | Explicit | Verbose | ✓✓ | ✓✓✓ | ✓✓✓ |
| Auto-lock strategies | Automatic | Magic/surprising? | ✗ | ✓✓ | ✓✓✓ |
| Per-output locking | Maximum control | Very complex | ✗ | ✓✓ | ✓✓✓ |

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: How should lock and determinism interact? Clear position.
2. **Semantic Model**: What does lock mean for deterministic vs non-deterministic jobs?
3. **Lock Types Taxonomy**: Are there different kinds of locks?
4. **TTL Integration**: How do lock, cache_ttl, and determinism work together?
5. **User Experience**: Commands, frontmatter, output metadata
6. **Multi-File Considerations**: Per-output locking or graft-level?
7. **Auto-Lock Strategies**: Should graft auto-lock anything?
8. **Use Case Validation**: Does this solve real problems intuitively?
9. **Edge Cases**: What breaks? How to handle?
10. **DVC Integration**: Concrete examples of dvc.yaml
11. **Migration Path**: How do existing grafts adapt?
12. **Trade-offs**: Honest assessment
13. **Recommendation**: Clear, justified design
14. **Open Questions**: What needs prototyping?

Think like a product designer who deeply understands distributed systems. The lock mechanism must be intuitive for both deterministic and non-deterministic jobs, with clear semantics that match user mental models.
