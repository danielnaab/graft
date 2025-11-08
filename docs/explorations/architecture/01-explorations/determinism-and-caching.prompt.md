---
deps:
  - architecture-exploration/00-sources/current-implementation.md
  - architecture-exploration/00-sources/design-goals.md
  - architecture-exploration/00-sources/open-questions.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Deep Exploration: Determinism and Caching Strategy

You are a systems architect analyzing how graft should handle deterministic vs non-deterministic jobs and what this means for caching, reproducibility, and change detection.

## Your Task

Think deeply about the fundamental tension between reproducibility (a core design goal) and the reality that many graft jobs are inherently non-deterministic.

### The Determinism Spectrum

**Fully Deterministic Jobs**:
- Text processing scripts (sed, awk, jq)
- Code formatters (prettier, black, gofmt)
- Data transformations with fixed logic
- Template rendering with fixed inputs
- Compilation/build steps

**Non-Deterministic Jobs**:
- LLMs with temperature > 0 (most common case!)
- Database queries (data changes over time)
- Web scraping (content changes)
- API calls (external data evolves)
- Jobs using current timestamps
- Jobs with explicit randomization

**Quasi-Deterministic** (deterministic within a time window):
- LLMs with temperature = 0 (mostly stable, but models update)
- Cached API results (deterministic until cache expires)
- Snapshot-based queries (deterministic within snapshot)

### Critical Questions

#### 1. What Does "Changed" Mean?

For deterministic jobs:
```
source.txt changes (hash differs)
→ output will be different
→ must regenerate
```

For non-deterministic jobs:
```
source.txt unchanged (hash identical)
→ output COULD be different (LLM, API, etc.)
→ should we regenerate?
```

**The Paradox**: If the job is non-deterministic, then the output hash changing doesn't necessarily mean anything important changed. If the output hash NOT changing doesn't mean the output wouldn't be different if regenerated.

#### 2. Caching Strategy

**For deterministic jobs**:
- Cache is valid as long as inputs don't change
- DVC's content-addressed storage works perfectly
- `dvc repro` only runs when inputs change

**For non-deterministic jobs**:
- Cache might be stale even if inputs unchanged
- Should there be a TTL (time-to-live)?
- Should there be a "force refresh" option?
- Should cache be per-run vs per-input-hash?

**Examples**:
```yaml
# Database-backed report
---
deps: [query.sql, config.yaml]
deterministic: false
cache_ttl: 24h  # Regenerate if output is >24h old
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# LLM synthesis
---
deps: [sources.md]
deterministic: false
temperature: 0.7
# Every run could produce different output
# Should this be cached at all?
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Code formatter
---
deps: [messy-code.js]
deterministic: true
# Cache forever (until inputs change)
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

#### 3. Change Detection Intelligence

Current graft has these actions:
- **GENERATE**: First time creation
- **REFINE**: Instructions changed, sources same
- **UPDATE**: Sources changed, instructions same
- **REFRESH**: Both changed
- **MAINTAIN**: Nothing changed

**For non-deterministic jobs, should there be new actions?**

**REGENERATE**: Inputs unchanged, but output might benefit from fresh generation
```
# Weekly report with LLM
# Sources don't change, but you want fresh perspective
```

**RESAMPLE**: For LLM jobs with temperature > 0
```
# Generate multiple variations
# Same inputs, different outputs
```

**EXPIRE**: Time-based invalidation
```
# Market analysis
# Data from yesterday is stale
```

#### 4. Reproducibility Guarantees

Graft's design goal: "Same inputs should produce same outputs"

**But what if the job is fundamentally non-deterministic?**

**Option A: Give up on reproducibility**
- Accept that non-deterministic jobs can't be reproduced
- Make this explicit in metadata
- Focus on auditability instead (track what was generated when)

**Option B: Enforce determinism**
- Require temperature=0 for LLMs
- Require snapshot/timestamp for database queries
- Require cached/versioned data for APIs
- Make determinism a hard requirement

**Option C: Hybrid approach**
- Support both, but make it explicit
- Different caching/invalidation strategies
- Different user expectations
- Clear labeling in output

**Metadata examples**:
```markdown
<!-- Generated: 2025-11-07 14:23:45 -->
<!-- Deterministic: false -->
<!-- Temperature: 0.7 -->
<!-- Note: Regenerating may produce different output -->
```

vs

```markdown
<!-- Generated: 2025-11-07 14:23:45 -->
<!-- Deterministic: true -->
<!-- Reproducible hash: abc123def456 -->
<!-- Regenerating with same inputs will produce identical output -->
```

#### 5. DVC Integration

DVC assumes determinism:
- Stage outputs are content-addressed
- Cache is based on input hashes
- `dvc repro` only runs if inputs changed

**How should graft configure DVC for non-deterministic jobs?**

**Option A: Add cache: false**
```yaml
stages:
  llm_synthesis:
    cmd: python render_llm.py
    deps: [sources.md]
    outs:
      - path: output.md
        cache: false  # Don't cache non-deterministic output
```

**Option B: Add persist: false**
```yaml
stages:
  api_report:
    cmd: python fetch_and_render.py
    deps: [config.yaml]
    outs:
      - path: report.md
        persist: false  # Always regenerate
```

**Option C: Use "always_changed" marker**
```yaml
stages:
  daily_summary:
    cmd: python generate_summary.py
    deps: [template.md]
    always_changed: true  # Treat as always out-of-date
    outs: [summary.md]
```

**Option D: Time-based dependencies**
```yaml
stages:
  market_analysis:
    cmd: python analyze.py
    deps:
      - data.csv
      - fresh: 24h  # Invalidate if output >24h old
    outs: [analysis.md]
```

#### 6. User Experience

How do users understand and control determinism?

**Frontmatter declaration**:
```yaml
---
deps: [sources.md]
deterministic: false
temperature: 0.7
cache_strategy: none  # or 'ttl' or 'hash-based'
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Or inferred from configuration**:
```yaml
---
deps: [sources.md]
model: claude-sonnet-4.5
temperature: 0.7  # Non-zero temp → infer non-deterministic
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Or explicit per-output**:
```yaml
---
outputs:
  - path: analysis.md
    deterministic: false
    reason: "LLM synthesis with temperature=0.7"
  - path: formatted-data.json
    deterministic: true
    reason: "Deterministic jq transformation"
---
```

**Commands**:
```bash
# Regenerate even if inputs unchanged (for non-deterministic jobs)
graft regenerate --force report

# Regenerate all jobs older than 24h
graft regenerate --stale-after 24h

# Show determinism status
graft status --show-determinism

# Regenerate only deterministic jobs (safe/reproducible)
graft regenerate --deterministic-only
```

### Use Case Analysis

#### Use Case 1: Code Documentation (Deterministic)
```
source code → LLM (temp=0) → documentation
```
**Desired behavior**:
- Cache aggressively
- Only regenerate when code changes
- Output should be reproducible
- High confidence in caching

#### Use Case 2: Creative Exploration (Non-Deterministic)
```
design-brief.md → LLM (temp=1.0) → 5 creative variations
```
**Desired behavior**:
- Don't cache (or cache with awareness it's just one sample)
- Allow regeneration even with unchanged inputs
- Each run produces different output
- User expects variability

#### Use Case 3: Data Analysis (Time-Dependent)
```
query.sql → database → LLM analysis → report.md
```
**Desired behavior**:
- Cache with TTL (e.g., 24 hours)
- Regenerate if output is stale
- Clear timestamp in output
- User knows when data was fetched

#### Use Case 4: Release Notes (Hybrid)
```
git log → template processor → release-notes.md
```
**Desired behavior**:
- Deterministic for given git history
- But git history changes
- Cache based on git commit hash
- Regenerate when new commits exist

### Edge Cases and Challenges

1. **Dependency chain with mixed determinism**:
   ```
   A.graft.md (deterministic) → data.json
   B.graft.md (non-deterministic) → depends on data.json
   ```
   If A doesn't change, B's input hash is stable. But B might still benefit from regeneration.

2. **Model updates**:
   ```
   LLM with temp=0 on Monday → output X
   LLM with temp=0 on Friday (model updated) → output Y
   ```
   Is this deterministic? The inputs didn't change, but the model did.

3. **Timestamp in output**:
   ```markdown
   <!-- Generated: 2025-11-07 -->
   ```
   If we include timestamp, output hash always changes, even for deterministic jobs.

4. **Partial non-determinism**:
   ```
   Job: Fetch API data → format with deterministic script
   ```
   The fetch is non-deterministic, but formatting is deterministic. How to model?

### Trade-off Analysis

| Approach | Reproducibility | Flexibility | Complexity | DVC Fit | User Understanding |
|----------|-----------------|-------------|------------|---------|-------------------|
| Assume all deterministic | ✓✓✓ | ✗ | ✓✓✓ | ✓✓✓ | ✓✓ |
| Assume all non-deterministic | ✗ | ✓✓✓ | ✓✓ | ✗ | ✓✓ |
| Explicit determinism flag | ✓✓ | ✓✓✓ | ✓✓ | ✓ | ✓ |
| Infer from config | ✓✓ | ✓✓ | ✓ | ✓✓ | ✓✓✓ |
| TTL-based caching | ✓ | ✓✓✓ | ✗ | ✗ | ✓ |
| Ignore the problem | ✓ | ✓ | ✓✓✓ | ✓✓✓ | ✗ |

### Research Questions

1. **Do users care about reproducibility for LLM-generated docs?**
   - If I regenerate my API documentation with same sources, does it matter if wording changes slightly?
   - vs "I need bit-for-bit identical output"

2. **Is temperature=0 "deterministic enough"?**
   - In practice, temp=0 is very stable
   - But not guaranteed identical across API versions
   - Should we treat it as deterministic?

3. **What's the granularity of determinism marking?**
   - Per-graft?
   - Per-output-file?
   - Per-step in a pipeline?

4. **How do time-based dependencies work with git?**
   - Git doesn't know about "24 hours ago"
   - How to make this git-native?

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: How should graft handle determinism? One clear position.
2. **Conceptual Framework**: What does determinism mean in the context of graft?
3. **Technical Design**: Specific proposals for frontmatter, DVC config, commands
4. **Caching Strategy**: How should deterministic vs non-deterministic jobs be cached?
5. **Change Detection**: How should GENERATE/REFINE/UPDATE/etc. work differently?
6. **User Mental Model**: How do users think about and control this?
7. **DVC Integration**: Concrete examples of dvc.yaml for different scenarios
8. **Use Case Validation**: Does this solve real problems users face?
9. **Edge Cases**: What breaks? What's weird?
10. **Trade-offs**: Honest assessment of approaches
11. **Recommendation**: Clear, justified design with migration path
12. **Open Questions**: What needs prototyping or user research?

Think like a distributed systems engineer. Determinism and caching are fundamental properties that affect everything. Get this right and the system is predictable and efficient. Get it wrong and users will be confused and frustrated.
