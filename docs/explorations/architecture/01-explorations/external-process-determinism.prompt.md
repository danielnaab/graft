---
deps:
  - architecture-exploration/00-sources/current-implementation.md
  - architecture-exploration/00-sources/design-goals.md
  - architecture-exploration/00-sources/open-questions.md
  - architecture-exploration/01-explorations/external-process-model.md
  - architecture-exploration/01-explorations/determinism-and-caching.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Deep Exploration: External Process Model and Determinism

You are a systems architect analyzing how an external process pipeline model would handle deterministic vs non-deterministic transformations.

## Your Task

The external process model proposes that graft should pipeline output through external tools rather than handling LLM invocation natively. The determinism exploration revealed that different jobs have different reproducibility guarantees.

Think deeply about how these two architectural decisions interact and inform each other.

### The Core Tension

**Pipeline Philosophy**: "Do one thing well, compose via stdout/stdin"
```bash
graft pack prompt.md | llm-tool | post-process | output.md
```

**Determinism Challenge**: Different tools in the pipeline have different guarantees
```bash
graft pack prompt.md |  # Deterministic: same inputs → same packed prompt
llm-tool |              # Non-deterministic: temperature > 0
post-process            # Deterministic: same input → same formatted output
```

**Question**: In a pipeline model, where does determinism metadata live? How is it propagated? How does caching work?

### Design Questions

#### 1. Determinism Declaration in Pipeline Model

**Option A: Graft declares determinism of entire pipeline**
```yaml
---
deps: [sources.md]
deterministic: false  # Because LLM step is non-deterministic
pipeline: |
  graft pack | llm-tool --temp 0.7 | prettier
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Option B: Each tool declares its determinism**
```bash
# Tools emit metadata
graft pack sources.md | jq .deterministic
# → true

llm-tool --temp 0.7 | jq .deterministic
# → false

prettier | jq .deterministic
# → true

# Graft infers: pipeline is non-deterministic (any step is non-deterministic)
```

**Option C: Graft analyzes the pipeline**
```yaml
---
pipeline: |
  graft pack | llm-tool --temp {{ temp }} | prettier
params:
  temp: 0.7
---

# Graft knows: llm-tool with temp > 0 → non-deterministic
# Inference rules built into graft
```

**Option D: Don't track at all**
- External processes are black boxes
- Graft treats everything as potentially non-deterministic
- Conservative: always allow regeneration

#### 2. Metadata Flow in Pipelines

In native model, graft controls everything:
```json
{
  "action": "UPDATE",
  "deterministic": false,
  "temperature": 0.7,
  "sources_changed": ["data.csv"],
  "instructions_changed": false
}
```

In pipeline model, how does metadata flow?

**Option A: Environment variables**
```bash
export GRAFT_ACTION=UPDATE
export GRAFT_DETERMINISTIC=false
export GRAFT_TEMP=0.7
export GRAFT_SOURCES_CHANGED="data.csv"

graft pack | llm-tool | post-process
# Tools can read env vars to adjust behavior
```

**Option B: Metadata sidecar**
```bash
graft pack --metadata metadata.json sources.md > packed.txt
llm-tool < packed.txt > output.md
# metadata.json contains determinism info
```

**Option C: Structured stdin/stdout**
```bash
graft pack sources.md | \
  # Outputs JSON: {"content": "...", "metadata": {...}}
llm-tool | \
  # Reads JSON, emits JSON with updated metadata
post-process
  # Final output separates content from metadata
```

**Option D: Separate streams**
```bash
graft pack sources.md \
  1> packed.txt \        # Content on stdout
  2> metadata.json       # Metadata on stderr

llm-tool < packed.txt > output.md
```

#### 3. Caching in Pipeline Model

**Current model**: Graft controls caching via DVC
```yaml
stages:
  doc:
    cmd: python render_llm.py
    deps: [sources.md]
    outs: [output.md]  # DVC caches based on deps hash
```

**Pipeline model**: Caching becomes complex
```yaml
stages:
  doc:
    cmd: graft pack sources.md | llm-tool | prettier > output.md
    deps: [sources.md]
    outs: [output.md]
```

**Where to cache?**

**Option A: Cache entire pipeline output (current DVC behavior)**
- Pros: Simple, works with existing DVC
- Cons: Caches non-deterministic LLM output (wrong for temp > 0)

**Option B: Cache deterministic steps only**
```yaml
stages:
  pack:
    cmd: graft pack sources.md > build/packed.txt
    deps: [sources.md]
    outs: [build/packed.txt]  # Cache: deterministic

  llm:
    cmd: llm-tool < build/packed.txt > build/raw.md
    deps: [build/packed.txt]
    outs:
      - build/raw.md:
          cache: false  # Don't cache: non-deterministic

  format:
    cmd: prettier < build/raw.md > output.md
    deps: [build/raw.md]
    outs: [output.md]  # Cache: deterministic
```

**Option C: Cache with TTL**
```yaml
stages:
  doc:
    cmd: graft pack sources.md | llm-tool | prettier > output.md
    deps: [sources.md]
    outs:
      - output.md:
          cache: true
          cache_ttl: 24h  # DVC doesn't support this today
```

**Option D: Hash-based cache keys that include determinism**
```python
# Cache key for deterministic: hash(deps)
# Cache key for non-deterministic: hash(deps + timestamp)
```

#### 4. Change Detection in Pipeline Model

Current graft has intelligent change detection:
- GENERATE, REFINE, UPDATE, REFRESH, MAINTAIN

**In pipeline model, who does change detection?**

**Option A: Graft does it, passes to pipeline**
```bash
# Graft analyzes git, determines action
export GRAFT_ACTION=UPDATE
graft pack sources.md | llm-tool --action $GRAFT_ACTION
```

**Option B: Each tool is responsible**
```bash
# graft pack figures out what changed
graft pack sources.md | \
  # Outputs: {"action": "UPDATE", "diff": "...", "content": "..."}

llm-tool | \
  # Reads action, adjusts behavior (REFINE → regenerate, UPDATE → patch)

prettier
```

**Option C: DVC does it**
```yaml
# DVC knows deps changed, runs pipeline
# Tools don't know/care about change detection
```

**Option D: Hybrid**
```bash
# Graft analyzes and emits metadata
# Tools CAN use it but don't have to
graft pack --emit-metadata sources.md | \
llm-tool --use-graft-metadata | \
prettier
```

**Critical insight**: The "smart patching" behavior (UPDATE → LLM patches existing) requires the tool to understand graft's change detection. In a pure pipeline model, how does this work?

#### 5. Tool Composition and Determinism Inference

**Scenario**: User wants to compose arbitrary tools
```yaml
---
deps: [data.csv]
pipeline: |
  graft pack | python analyze.py | jq '.results' | prettier
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Questions**:
- Is `python analyze.py` deterministic? Graft doesn't know!
- Should user declare determinism of custom scripts?
- Can graft infer anything?

**Option A: Require explicit declaration**
```yaml
---
deps: [data.csv]
pipeline: |
  graft pack | python analyze.py | jq '.results' | prettier
deterministic: true  # User must specify
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Option B: Tool metadata protocol**
```bash
python analyze.py --graft-metadata
# Outputs: {"deterministic": true, "version": "1.0"}

# Graft can query each tool
graft check-pipeline
# → analyze.py is deterministic
# → jq is deterministic
# → prettier is deterministic
# → Overall: deterministic
```

**Option C: Conservative default**
- Treat all external tools as non-deterministic unless proven otherwise
- Safe but potentially over-cautious

**Option D: Sandbox and test**
```bash
# Graft runs pipeline twice with identical inputs
graft pack sources.md | analyze.py > out1.md
graft pack sources.md | analyze.py > out2.md
diff out1.md out2.md
# If identical → likely deterministic
# If different → non-deterministic
```

#### 6. Unix Philosophy and Determinism

Unix tools are typically deterministic:
```bash
cat file.txt | grep pattern | sort | uniq
# Run this 100 times → identical output
```

But graft introduces non-deterministic tools:
```bash
cat file.txt | llm-summarize | prettier
# Run this 100 times → 100 different summaries
```

**Does this violate Unix philosophy?**

**Perspective A: Yes**
- Unix tools are predictable, composable because deterministic
- Adding non-determinism breaks composability
- Pipeline caching assumptions break down

**Perspective B: No**
- Unix tools can be non-deterministic (curl, date, random)
- Pipeline model is about composition, not determinism guarantees
- Tools declare capabilities, users compose knowingly

**Perspective C: Extension**
- Traditional Unix: deterministic tools
- Modern Unix: some non-determinism (network, time)
- Graft extends this: LLMs are powerful non-deterministic tools
- Just need better metadata protocols

#### 7. Practical Examples

**Example 1: Deterministic pipeline**
```yaml
---
deps: [data.json]
pipeline: |
  graft pack | jq '.items | map(.name)' | sort | uniq > names.txt
deterministic: true
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Caching**: Aggressive (hash-based)
**Regeneration**: Only when data.json changes
**Reproducibility**: Guaranteed

**Example 2: Non-deterministic LLM pipeline**
```yaml
---
deps: [research.md]
pipeline: |
  graft pack | llm-tool --temp 0.7 | prettier > summary.md
deterministic: false
temperature: 0.7
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Caching**: None, or TTL-based
**Regeneration**: User decides (force flag)
**Reproducibility**: Not guaranteed

**Example 3: Mixed pipeline**
```yaml
---
deps: [api-spec.yaml]
pipeline: |
  graft pack |                    # Deterministic
  llm-tool --temp 0 |             # Quasi-deterministic (temp=0)
  jq '.documentation' |           # Deterministic
  prettier                        # Deterministic
output: api-docs.md
deterministic: quasi  # Mostly stable, but LLM may update
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Caching**: Moderate (temp=0 is stable in practice)
**Regeneration**: When api-spec.yaml changes
**Reproducibility**: High (temp=0), but not perfect

**Example 4: Time-dependent pipeline**
```yaml
---
deps: [query.sql]
pipeline: |
  psql -f query.sql |             # Non-deterministic (database changes)
  llm-tool --temp 0.3 |           # Non-deterministic
  prettier
output: daily-report.md
deterministic: false
cache_ttl: 24h
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Caching**: TTL-based (24h)
**Regeneration**: Daily, regardless of query.sql changes
**Reproducibility**: None (both DB and LLM change)

### Trade-off Analysis

| Aspect | Native Model | Pipeline Model (Determinism-Aware) | Pipeline Model (Determinism-Agnostic) |
|--------|--------------|-----------------------------------|--------------------------------------|
| Determinism tracking | ✓✓✓ Graft controls | ✓✓ Via metadata protocol | ✗ Unknown |
| Caching correctness | ✓✓✓ Graft decides | ✓✓ Per-tool, complex | ✓ Conservative |
| Tool composability | ✗ Graft-specific | ✓✓✓ Unix philosophy | ✓✓✓ Unix philosophy |
| Change detection | ✓✓✓ Intelligent | ✓✓ Via metadata | ✗ Simple/naive |
| Non-LLM support | ✗ Forced through LLM | ✓✓✓ Native | ✓✓✓ Native |
| Complexity | ✓✓✓ Simple | ✗ Metadata protocol needed | ✓✓ Simpler |
| User understanding | ✓✓ Graft is smart | ✓ Tools declare capabilities | ✓✓ Simple pipelines |

### Research Questions

1. **Is a metadata protocol realistic?**
   - Would external tools implement it?
   - Is JSON on stderr acceptable?
   - Or should it be opt-in (tools work without it)?

2. **Should graft standardize a "determinism protocol"?**
   ```bash
   tool --deterministic-check
   # Outputs: true/false/unknown
   ```

3. **Can DVC be extended for TTL-based caching?**
   - Current DVC: hash-based
   - Needed: time-based + hash-based
   - Is this a fundamental change?

4. **How important is change detection (UPDATE vs REFINE) in practice?**
   - If most users just want "regenerate", maybe intelligence isn't needed
   - If patching is critical, pipeline model needs metadata flow

5. **Should determinism affect default pipeline behavior?**
   ```yaml
   # No determinism specified
   # Graft assumes: deterministic for non-LLM, non-deterministic for LLM?
   ```

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: Should graft adopt a pipeline model, and how does determinism affect this decision?
2. **Metadata Protocol Design**: How should determinism metadata flow through pipelines?
3. **Caching Strategy**: How to cache deterministic vs non-deterministic pipeline steps?
4. **Change Detection**: Can intelligent change detection work in a pipeline model?
5. **Tool Composition**: How do users compose deterministic and non-deterministic tools?
6. **DVC Integration**: Concrete examples of multi-stage pipelines
7. **Unix Philosophy**: Does non-determinism violate or extend Unix principles?
8. **Practical Examples**: Real-world pipeline configurations
9. **Implementation Sketch**: What would the code/protocol look like?
10. **Trade-offs**: Honest assessment of native vs pipeline models
11. **Recommendation**: Clear position on whether to adopt pipeline model
12. **Migration Path**: If adopting pipeline model, how to migrate existing grafts?
13. **Open Questions**: What needs prototyping?

Think like a systems architect who values both Unix philosophy and practical engineering. The pipeline model is elegant in theory, but must handle the messy reality of non-deterministic LLM tools while remaining simple enough for users to understand.
