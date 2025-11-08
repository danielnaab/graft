---
deps:
  - architecture-exploration/00-sources/current-implementation.md
  - architecture-exploration/00-sources/design-goals.md
  - architecture-exploration/00-sources/open-questions.md
  - architecture-exploration/01-explorations/dvc-reliance.md
  - architecture-exploration/01-explorations/determinism-and-caching.md
---

# Deep Exploration: DVC Native Features for Determinism Handling

You are a systems architect analyzing how DVC's native features could handle graft's determinism challenges, reducing the need for custom implementations.

## Your Task

Two explorations have revealed:
1. Graft may be reinventing DVC features
2. Graft needs to handle deterministic vs non-deterministic jobs

Think deeply about how DVC's existing features could solve determinism challenges, and where graft genuinely needs custom logic.

### The Central Question

**Can DVC's native features handle non-deterministic jobs, or does graft need custom solutions?**

### DVC's Native Capabilities

DVC provides several features that might address determinism:

#### 1. Stage-level Control

```yaml
stages:
  deterministic_job:
    cmd: python process.py
    deps: [data.csv]
    outs:
      - output.md  # Cached by default

  non_deterministic_job:
    cmd: python llm_generate.py
    deps: [prompt.md]
    outs:
      - report.md:
          cache: false  # Don't cache non-deterministic output
```

**DVC supports `cache: false`** - output is tracked but not cached.

**Questions**:
- Is this sufficient for non-deterministic jobs?
- What about TTL-based caching?
- How does this interact with `dvc repro`?

#### 2. `always_changed` Flag

```yaml
stages:
  daily_report:
    cmd: python generate.py
    deps: [template.md]
    outs: [report.md]
    always_changed: true  # Treat stage as always out-of-date
```

**DVC has `always_changed`** - forces stage to run every `dvc repro`.

**Questions**:
- Is this the right semantic for "regenerate daily"?
- Does it work with caching?
- Can you selectively run vs skip these stages?

#### 3. Frozen Stages

```yaml
stages:
  locked_analysis:
    cmd: python analyze.py
    deps: [historical-data.csv]
    outs: [q4-analysis.md]
    frozen: true  # Never run this stage
```

**DVC has `frozen`** - prevents stage from running in `dvc repro`.

**Questions**:
- Is this graft's "lock" mechanism?
- How do you unfreeze (unlock)?
- What happens if deps change?
- Does this solve the lock use cases from the lock exploration?

#### 4. Metrics and Plots (Quality Tracking)

```yaml
stages:
  llm_generate:
    cmd: python generate.py
    deps: [sources.md]
    outs: [output.md]
    metrics:
      - quality.json:
          cache: false
```

**DVC tracks metrics** - for comparing experiment outputs.

**Potential for graft**:
- Track "quality" of non-deterministic outputs
- Compare multiple LLM runs
- Select best result

**Questions**:
- Should graft use metrics to evaluate non-deterministic outputs?
- Could this help with "regenerate until quality threshold"?

#### 5. DVC Experiments (Built-in A/B Testing)

```bash
# Run with different temperatures
dvc exp run -n temp_03 --set-param temperature=0.3
dvc exp run -n temp_07 --set-param temperature=0.7
dvc exp run -n temp_09 --set-param temperature=0.9

# Compare outputs
dvc exp show
```

**DVC experiments** are designed for trying different parameters.

**Perfect for non-deterministic jobs**:
- Run LLM multiple times with different seeds/temps
- Compare outputs side-by-side
- Select favorite, make it the main branch

**Questions**:
- Is this how graft should handle non-determinism?
- Instead of "lock favorite sample", use experiments?
- Does this replace graft's lock mechanism?

### Solving Determinism Challenges with DVC

#### Challenge 1: Non-Deterministic Jobs Should Not Cache

**Graft need**: Don't cache LLM outputs with temp > 0

**DVC solution**:
```yaml
stages:
  creative_llm:
    cmd: python render.py --temp 0.9
    deps: [prompt.md]
    outs:
      - creative-output.md:
          cache: false
```

**Assessment**: ✓ DVC handles this natively

**Graft's role**: Auto-generate `cache: false` based on temperature in frontmatter
```yaml
# prompt.md
---
deps: [sources.md]
temperature: 0.9
---

# Generated dvc.yaml
stages:
  prompt:
    cmd: python render.py --temp 0.9
    outs:
      - output.md:
          cache: false  # Auto-added because temp > 0
```

#### Challenge 2: Lock Mechanism

**Graft need**: Prevent expensive grafts from regenerating

**DVC solution**:
```yaml
stages:
  expensive_analysis:
    cmd: python analyze.py
    deps: [data.csv]
    outs: [analysis.md]
    frozen: true  # Locked!
```

**To unlock**:
```bash
# Edit dvc.yaml, set frozen: false
# Or:
dvc unfreeze expensive_analysis
dvc repro
```

**Assessment**: ✓ DVC has this feature (`frozen`)

**Graft's role**:
- Map `lock: true` in frontmatter → `frozen: true` in dvc.yaml
- Provide `graft lock/unlock` commands that edit dvc.yaml
- Add metadata to output: `<!-- Frozen: true -->`

**Comparison to graft's custom lock**:
```yaml
# Graft custom approach
---
lock: true
lock_reason: "Q4 compliance snapshot"
---

# DVC native approach
stages:
  report:
    frozen: true
    # lock_reason stored in comment or separate metadata
```

**Question**: Should graft just use DVC's `frozen` instead of custom lock?

#### Challenge 3: Time-Based Invalidation (TTL)

**Graft need**: Regenerate daily reports even if deps unchanged

**DVC problem**: No native TTL support

**Workarounds**:

**Option A: `always_changed`**
```yaml
stages:
  daily_report:
    cmd: python generate.py
    deps: [template.md]
    outs: [report.md]
    always_changed: true
```
- Runs every `dvc repro`
- But no concept of "24 hours"

**Option B: Time-based dependency**
```yaml
stages:
  daily_report:
    cmd: |
      if [ ! -f report.md ] || [ $(find report.md -mtime +1) ]; then
        python generate.py
      fi
    deps: [template.md]
    outs: [report.md]
```
- Shell logic for TTL
- Works but hacky

**Option C: Graft wrapper tracks timestamps**
```python
# graft checks timestamps before calling dvc
if is_stale(output, ttl="24h"):
    subprocess.run(["dvc", "repro", "-f", stage_name])
else:
    print(f"{output} is fresh (< 24h old)")
```

**Option D: Date-based dependency files**
```yaml
stages:
  daily_report:
    cmd: python generate.py
    deps:
      - template.md
      - .timestamps/2025-11-07  # File updated daily
    outs: [report.md]
```
- `.timestamps/` directory has files named by date
- Cron creates new file each day
- DVC sees new dep, regenerates

**Assessment**: ✗ DVC doesn't handle TTL natively, need workaround

**Question**: Which workaround is cleanest? Or should graft contribute TTL to DVC upstream?

#### Challenge 4: Determinism Metadata

**Graft need**: Track whether job is deterministic in metadata

**DVC**: No native determinism field

**Options**:

**Option A: Custom fields in dvc.yaml**
```yaml
stages:
  report:
    cmd: python render.py
    deps: [sources.md]
    outs: [report.md]
    meta:  # DVC supports arbitrary metadata
      deterministic: false
      temperature: 0.7
      reason: "LLM with temp > 0"
```

**Option B: Infer from configuration**
```python
# graft analyzes stage
if "cache: false" in stage.outs:
    deterministic = False
elif "always_changed: true" in stage:
    deterministic = False
else:
    deterministic = True  # Assume deterministic by default
```

**Option C: Separate metadata file**
```
docs/report.prompt.md
build/report.meta.json  # {"deterministic": false, "temperature": 0.7}
```

**Assessment**: DVC's `meta` field could work, but requires custom graft parsing

#### Challenge 5: Multi-Sampling Non-Deterministic Jobs

**Graft need**: Generate multiple samples from non-deterministic LLM

**DVC solution**: Use experiments!
```bash
# Generate 5 samples
for i in {1..5}; do
  dvc exp run -n sample_$i --set-param seed=$i
done

# Compare all samples
dvc exp show

# Select favorite
dvc exp apply sample_3
```

**Assessment**: ✓✓✓ DVC experiments are perfect for this!

**Graft's role**:
```bash
graft sample creative-names --count 5
# → Runs dvc exp run 5 times with different seeds
# → Outputs: creative-names-sample-{1..5}.md
# → User can compare and select favorite

graft sample-apply creative-names sample_3
# → dvc exp apply sample_3
```

**This is better than graft's lock mechanism for non-deterministic jobs!**

#### Challenge 6: Reproducibility Tracking

**Graft need**: Document what sources/model version produced output

**DVC solution**: `dvc.lock` tracks exact hashes
```yaml
# dvc.lock (auto-generated)
schema: '2.0'
stages:
  report:
    cmd: python render.py
    deps:
    - path: sources.md
      md5: abc123...
    - path: prompt.md
      md5: def456...
    outs:
    - path: report.md
      md5: 789ghi...
```

**Assessment**: ✓ DVC tracks this automatically

**Graft's role**: Include dvc.lock hashes in output metadata
```markdown
<!-- Generated with Graft -->
<!-- Sources hash (from dvc.lock): abc123 -->
<!-- Prompt hash (from dvc.lock): def456 -->
<!-- Reproducible: git checkout <commit> && dvc repro report -->
```

### Rethinking Graft Features with DVC Natives

#### Feature: Lock Mechanism

**Original plan**: Custom `lock: true` in frontmatter

**DVC native**: `frozen: true` in stages

**Recommendation**: Use DVC's frozen
```yaml
# docs/report.prompt.md
---
deps: [data.csv]
frozen: true  # Maps directly to DVC frozen
---

# Generated dvc.yaml
stages:
  docs__report:
    frozen: true  # Direct mapping
```

**Commands**:
```bash
graft lock report
# → Edits dvc.yaml, sets frozen: true

graft unlock report
# → Edits dvc.yaml, sets frozen: false

# Or just use DVC directly:
dvc freeze docs__report
dvc unfreeze docs__report
```

#### Feature: Cache Strategy

**Original plan**: Custom `cache_strategy: none` or `cache_ttl: 24h`

**DVC native**: `cache: false`, `always_changed: true`

**Recommendation**: Map to DVC features
```yaml
# docs/report.prompt.md
---
deps: [sources.md]
temperature: 0.9  # temp > 0 → non-deterministic
---

# Generated dvc.yaml
stages:
  docs__report:
    outs:
      - docs/report.md:
          cache: false  # Auto-added for temp > 0
```

**For TTL**: Use workaround until DVC adds native support
```yaml
# docs/daily-report.prompt.md
---
deps: [query.sql]
cache_ttl: 24h  # Graft-specific, not DVC
---

# Graft wrapper checks timestamp before dvc repro
```

#### Feature: Experiments / Sampling

**Original plan**: Custom multi-sampling for non-deterministic jobs

**DVC native**: Experiments framework

**Recommendation**: Use DVC experiments
```bash
# User wants 5 creative variations
graft experiment sample creative-names --count 5
# → Translates to:
#   for i in 1..5; do
#     dvc exp run -n sample_$i --set-param seed=$i creative-names
#   done

# User compares outputs
graft experiment show creative-names
# → dvc exp show --only-changed

# User selects favorite
graft experiment apply creative-names sample_3
# → dvc exp apply sample_3
```

**This is much better than "lock to preserve favorite"!**

### Architecture Proposal: Graft as DVC Native

**Core principle**: Graft generates DVC-native pipelines, uses DVC features directly

**Graft's responsibilities**:
1. **Parse `.prompt.md` frontmatter** → Generate `dvc.yaml`
2. **Infer DVC settings from frontmatter**:
   - `temperature > 0` → `cache: false`
   - `frozen: true` → `frozen: true`
   - `params: [model]` → `params: [llm.model]`
3. **Provide convenience commands**:
   - `graft rebuild` → `dvc repro`
   - `graft lock` → `dvc freeze`
   - `graft experiment` → `dvc exp run`
4. **Pack prompts** (graft-specific logic)
5. **Render LLM** (graft-specific logic)

**What graft does NOT do**:
- ✗ Custom caching (use DVC)
- ✗ Custom dependency tracking (use DVC)
- ✗ Custom lock implementation (use DVC frozen)
- ✗ Custom experiments (use DVC exp)
- ✗ Reinvent change detection (use DVC's hash-based detection)

**Generated pipeline example**:
```yaml
# docs/creative-names.prompt.md
---
deps: [brand-brief.md]
temperature: 0.9
frozen: false
params:
  - llm.model
---

# Generated dvc.yaml
stages:
  pack_docs__creative_names:
    cmd: python pack_prompt.py docs/creative-names.prompt.md
    deps:
      - docs/creative-names.prompt.md
      - brand-brief.md
    params:
      - llm.model
    outs:
      - build/creative-names.promptpack.txt

  render_docs__creative_names:
    cmd: python render_llm.py build/creative-names.promptpack.txt --temp 0.9
    deps:
      - build/creative-names.promptpack.txt
    params:
      - llm.model
    outs:
      - docs/creative-names.md:
          cache: false  # Auto-added because temp > 0
```

**User workflow**:
```bash
# Standard DVC commands work
dvc repro
dvc status
dvc dag

# Graft convenience commands
graft rebuild  # → dvc repro
graft status   # → dvc status (pretty-printed)

# DVC experiments for non-determinism
graft experiment sample creative-names --count 3
# → dvc exp run -n sample_{1,2,3} --set-param seed={1,2,3}

# DVC frozen for locks
graft lock expensive-analysis
# → dvc freeze docs__expensive_analysis

# DVC remotes for sharing
graft push  # → dvc push
graft pull  # → dvc pull
```

### Trade-off Analysis

| Feature | Custom Graft | DVC Native | Hybrid |
|---------|-------------|-----------|---------|
| **Lock** | Custom implementation | `frozen: true` | Graft syntax → DVC frozen |
| **Cache control** | Custom cache_strategy | `cache: false` | Infer from temp |
| **TTL** | Custom timestamp tracking | ✗ Not supported | Graft wrapper |
| **Experiments** | Custom multi-sample | `dvc exp` ✓✓✓ | Graft commands → dvc exp |
| **Determinism metadata** | Custom tracking | `meta` field | Use DVC meta |
| **Params** | Frontmatter only | `params.yaml` ✓✓✓ | Both (frontmatter → params.yaml) |
| **Remote cache** | Not implemented | `dvc remote` ✓✓✓ | Document usage |
| **Change detection** | Custom git analysis | Hash-based ✓✓✓ | Use DVC, add graft semantics |

**Assessment**:
- ✓✓✓ Use DVC native where possible
- ✓✓ Graft adds convenience layer
- ✓ Custom only where DVC lacks features (TTL)

### Edge Cases and Concerns

#### Concern 1: DVC Learning Curve

**Pro DVC native**: Users get full power of DVC
**Con**: Must learn DVC concepts (stages, deps, outs, dvc.lock, etc.)

**Mitigation**: Graft documentation teaches DVC concepts in context
- "Understanding Graft's DVC Pipeline"
- "How Graft Uses DVC Experiments"

#### Concern 2: DVC Doesn't Have TTL

**Reality**: Time-based invalidation not in DVC

**Options**:
1. **Contribute to DVC**: Add TTL support upstream
2. **Graft wrapper**: Check timestamps, selectively run stages
3. **Don't support**: Focus on deterministic use cases
4. **Workaround**: Date-based dependency files

**Recommendation**: Start with #2 (graft wrapper), consider #1 (contribute) if widely needed

#### Concern 3: Frozen Stages and Dependency Changes

**Scenario**: Frozen stage, deps change

```yaml
stages:
  q4_analysis:
    frozen: true
    deps: [data-q4.csv]  # This file changes
    outs: [q4-analysis.md]
```

**DVC behavior**: Stage stays frozen, doesn't regenerate

**Is this what users want?** Probably yes for "historical snapshot" use case.

**Graft's role**: Warn if frozen stage has changed deps
```bash
graft status
# Warning: q4_analysis is frozen but deps have changed
# - data-q4.csv (hash: old123 → new456)
# Run 'graft unlock q4_analysis' to allow regeneration
```

### Recommendations

1. **Use DVC's `frozen` for locks** instead of custom implementation
2. **Use DVC's `cache: false` for non-deterministic outputs** (auto-infer from temp)
3. **Use DVC experiments for multi-sampling** non-deterministic jobs
4. **Use DVC params.yaml** for model configuration
5. **Use DVC remote** for team caching (document well)
6. **Implement TTL as graft wrapper** until DVC adds native support
7. **Generate multi-stage pipelines** (pack + render) for better caching
8. **Make DVC visible and celebrated**, not hidden

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: Should graft use DVC native features for determinism? Clear recommendation.
2. **Feature Mapping**: For each graft need, which DVC feature solves it?
3. **Lock Mechanism**: DVC frozen vs custom lock implementation
4. **Caching Strategy**: DVC cache controls vs custom caching
5. **Experiments Integration**: Using dvc exp for non-deterministic jobs
6. **TTL Implementation**: How to handle time-based invalidation without native DVC support
7. **Multi-Stage Pipelines**: Should pack and render be separate stages?
8. **Params Integration**: How to use params.yaml effectively
9. **Remote Caching**: How to encourage/enable DVC remote usage
10. **Architecture Proposal**: What does "DVC native graft" look like?
11. **Migration Path**: How to move from current to DVC-native approach
12. **Trade-offs**: Honest assessment of DVC native vs custom
13. **Edge Cases**: Where does DVC not fit? Where is custom logic needed?
14. **User Experience**: Is DVC-native approach intuitive?
15. **Open Questions**: What needs prototyping?

Think like an architect who values using existing, proven tools over custom implementations. If DVC has a feature, use it. Only build custom where DVC genuinely lacks capability. Be specific about what maps to DVC features and what requires graft-specific logic.
