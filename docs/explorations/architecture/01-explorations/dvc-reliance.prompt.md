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

# Deep Exploration: DVC Reliance and Avoiding Reinvention

You are a systems architect analyzing whether graft is reinventing capabilities that DVC already provides, and how to better leverage DVC's features.

## Your Task

Think critically about the boundary between graft and DVC. Graft was built on DVC for good reasons, but is it using DVC to its full potential? Or is it reimplementing functionality that DVC already handles well?

Be rigorous and honest. If graft is reinventing the wheel, identify it. If graft provides genuine value-add over DVC, articulate exactly what that is.

### Core Question

**What does graft actually do that DVC doesn't?**

Graft's current responsibilities:
1. **Scan for `.prompt.md` files** → Generate DVC stages
2. **Pack prompts** with dependencies and context
3. **Detect changes** (GENERATE, REFINE, UPDATE, etc.)
4. **Invoke LLM** via `render_llm.sh`
5. **Manage build artifacts** (`.promptpack.txt`, `.params.json`, etc.)

DVC's capabilities:
1. **Pipeline definition** (`dvc.yaml`)
2. **Dependency tracking** (hash-based)
3. **Change detection** (which stages are out of date)
4. **Command execution** (run stages when needed)
5. **Caching** (outputs, remote storage)
6. **Parameterization** (`params.yaml`)
7. **Metrics and plots** tracking
8. **Experiment tracking** (compare runs)

**Overlap**: Both do change detection, dependency tracking, pipeline management.

**Question**: Is graft's "value-add" worth the complexity, or should we lean harder into DVC's primitives?

### Critical Analysis

#### 1. Change Detection: Graft vs DVC

**Graft's approach**:
```python
# pack_prompt.py analyzes git
if not previous_output_exists:
    action = "GENERATE"
elif prompt_changed and not sources_changed:
    action = "REFINE"
elif sources_changed and not prompt_changed:
    action = "UPDATE"
elif both_changed:
    action = "REFRESH"
else:
    action = "MAINTAIN"
```

**DVC's approach**:
```yaml
# dvc.yaml
stages:
  doc:
    cmd: python render.py
    deps:
      - sources.md
      - prompt.md
    outs:
      - output.md
```

DVC automatically detects:
- If `sources.md` changed (hash differs)
- If `prompt.md` changed (hash differs)
- If `output.md` needs regeneration

**The Question**: Does graft's GENERATE/REFINE/UPDATE distinction actually matter?

**Potential value**:
- REFINE → Regenerate from scratch (instructions changed)
- UPDATE → Patch existing output (sources changed, instructions same)
- This enables intelligent patching vs full regeneration

**But**:
- DVC already knows which deps changed via `dvc status --show-json`
- Graft could query DVC instead of reimplementing change detection
- Example:
  ```python
  # Instead of git analysis
  status = json.loads(subprocess.check_output(["dvc", "status", "--show-json"]))
  changed_deps = status[stage_name]["changed deps"]

  if "prompt.md" in changed_deps:
      action = "REFINE"
  elif any(src in changed_deps for src in sources):
      action = "UPDATE"
  ```

**Trade-off**: Graft's current approach is git-native (reads `HEAD`), DVC's is hash-based. Which is better?

#### 2. Pipeline Generation: Auto-magic vs Explicit

**Graft's approach**:
```yaml
# docs/report.prompt.md
---
deps:
  - data.csv
  - analysis.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

Graft scans all `.prompt.md`, auto-generates `dvc.yaml`:
```yaml
stages:
  docs__report:
    cmd: python render_llm.py docs/report.prompt.md
    deps:
      - docs/report.prompt.md
      - data.csv
      - analysis.md
    outs:
      - docs/report.md
```

**Pure DVC approach**:
```yaml
# dvc.yaml (hand-written or templated)
stages:
  report:
    cmd: python render_llm.py docs/report.prompt.md
    deps:
      - docs/report.prompt.md
      - data.csv
      - analysis.md
    outs:
      - docs/report.md
    params:
      - model
      - temperature
```

**Questions**:
1. Is auto-generation worth it?
   - **Pro**: DRY, dependencies declared once in frontmatter
   - **Con**: Magic, indirection, harder to debug DVC issues

2. Could graft just be a `dvc.yaml` generator?
   ```bash
   graft generate-dvc  # Scans .prompt.md, writes dvc.yaml
   dvc repro           # Standard DVC from here
   ```

3. Should users write `dvc.yaml` directly?
   - More explicit, less magic
   - But more verbose, potential for drift between frontmatter and dvc.yaml

#### 3. Parameterization: Graft's Build Artifacts vs DVC Params

**Graft currently**:
```
build/
  report.params.json    # {"model": "claude-sonnet-4.5", "temperature": 0.7}
  report.promptpack.txt # Packed prompt
  report.context.json   # Dependency metadata
```

**DVC has native param support**:
```yaml
# params.yaml
reports:
  model: claude-sonnet-4.5
  temperature: 0.7

# dvc.yaml
stages:
  report:
    cmd: python render.py ${reports.model} ${reports.temperature}
    params:
      - reports.model
      - reports.temperature
```

**DVC benefits**:
- `dvc params diff` to compare
- `dvc exp run --set-param reports.temperature=0.9`
- Built-in experiment tracking
- Metrics and plots integration

**Question**: Should graft use DVC params instead of build/*.params.json?

**Trade-offs**:
- **DVC params**: Centralized, trackable, experiment-friendly
- **Graft params**: Per-prompt, can be in frontmatter, isolated

**Potential hybrid**:
```yaml
# docs/report.prompt.md
---
deps: [data.csv]
params:
  model: ${llm.model}      # Reference from params.yaml
  temperature: 0.7          # Or specify inline
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# params.yaml (DVC standard)
llm:
  model: claude-sonnet-4.5
  default_temperature: 0.3
```

#### 4. Caching: DVC's Native vs Graft's TTL Needs

**DVC caching**:
- Hash-based: cache key = hash(deps + cmd + params)
- Content-addressed storage
- Remote cache support (S3, GCS, etc.)
- Works great for deterministic outputs

**Graft's needs** (from determinism exploration):
- TTL-based caching for non-deterministic jobs
- "Regenerate daily even if inputs unchanged"
- Lock mechanism to prevent regeneration

**DVC doesn't support**:
- Time-based invalidation
- "Always run" stages (with caching)
- Lock/freeze mechanism

**Options**:

**Option A: Extend DVC** (contribute upstream)
```yaml
stages:
  daily_report:
    cmd: python generate.py
    deps: [query.sql]
    outs:
      - report.md:
          cache: true
          cache_ttl: 24h  # New DVC feature
```

**Option B: Work around DVC**
```yaml
stages:
  daily_report:
    cmd: |
      if [ $(find report.md -mtime +1) ]; then
        python generate.py
      fi
    deps: [query.sql]
    outs: [report.md]
```

**Option C: Graft wrapper**
```bash
# graft handles TTL, calls dvc when needed
graft regenerate --respect-ttl
# Checks timestamps, selectively runs dvc repro
```

**Question**: Is graft's TTL/lock logic better as a DVC extension or a wrapper?

#### 5. Build Artifacts: Graft-Specific vs DVC Outputs

**Graft creates**:
```
build/
  report.promptpack.txt  # Packed prompt with all context
  report.params.json     # Effective parameters
  report.context.json    # Dependency metadata
  report.attachments.json # Binary attachments list
```

**These are intermediate artifacts** for debugging and inspection.

**DVC perspective**: These could be tracked outputs:
```yaml
stages:
  pack_prompt:
    cmd: python pack_prompt.py docs/report.prompt.md
    deps:
      - docs/report.prompt.md
      - data.csv
    outs:
      - build/report.promptpack.txt
      - build/report.params.json
      - build/report.context.json

  render:
    cmd: python render_llm.py build/report.promptpack.txt
    deps:
      - build/report.promptpack.txt
      - build/report.params.json
    outs:
      - docs/report.md
```

**Benefits of multi-stage DVC**:
- Can cache packed prompts (expensive if many deps)
- Can regenerate without re-packing (if only model params change)
- More granular change detection
- Standard DVC, no graft magic

**Downsides**:
- More complex `dvc.yaml`
- Two stages per document
- Graft's auto-generation gets harder

**Question**: Should graft embrace multi-stage pipelines in DVC?

#### 6. Experiments: DVC Experiments vs Graft's Current Approach

**DVC has powerful experiment tracking**:
```bash
# Run experiment with different params
dvc exp run --set-param temperature=0.9 --name high-temp

# Compare results
dvc exp show --include-params temperature --include-metrics quality

# Track metrics
echo '{"quality": 0.85}' > metrics.json
dvc metrics diff
```

**Graft currently**: No experiment tracking, no metrics, no comparison

**Use case**: Testing different prompts or temperatures
```bash
# Current graft approach
# Edit prompt, regenerate, manually compare

# DVC experiments approach
dvc exp run --set-param reports.temperature=0.3 --name conservative
dvc exp run --set-param reports.temperature=0.9 --name creative
dvc exp show --include-params temperature
# See both outputs, compare side-by-side
```

**Question**: Should graft integrate DVC experiments?

**Example**:
```bash
graft experiment --temperature 0.9 report
# → runs dvc exp run with new params
# → generates report.md variant
# → tracks in DVC experiments

graft experiments list
# Shows all experiment runs with params and outputs
```

#### 7. Remote Storage: DVC's Strength, Graft's Opportunity

**DVC remote storage**:
```bash
dvc remote add myremote s3://bucket/path
dvc push  # Upload cached outputs
dvc pull  # Download cached outputs
```

**Graft benefit**: Expensive LLM outputs could be cached remotely
- Generate once, share across team
- Don't re-run expensive prompts
- Especially valuable for deterministic outputs (temp=0)

**Current graft**: No remote caching

**Question**: Should graft encourage DVC remote setup?

**Example workflow**:
```bash
# Developer A generates expensive docs
graft regenerate --all
dvc push  # Share outputs to team

# Developer B pulls cached results
dvc pull
# Gets all generated docs without LLM costs

# Developer B makes small change
# Only affected docs regenerate, rest from cache
```

**Graft could**:
- Document DVC remote setup in getting started
- Add `graft push` / `graft pull` aliases
- Validate remote is configured before expensive operations
- Integrate with lock mechanism (locked = always from cache)

#### 8. Pipeline Visualization: DVC DAG vs Graft's Implicit Graph

**DVC has built-in visualization**:
```bash
dvc dag
# Shows dependency graph

dvc dag docs/report.md
# Shows what report depends on
```

**Graft**: Dependencies implicit in frontmatter, not easily visualized

**Question**: Should graft encourage using `dvc dag`?

Or should graft add its own visualization?
```bash
graft graph
# Shows .prompt.md files and their dependencies
# Essentially renders dvc dag in graft-specific way
```

**Trade-off**: DVC's dag is standard and works. Graft-specific graph could show prompt-specific info (actions, determinism, locks).

### Boundary Analysis: What Should Graft Own?

Let's be ruthlessly honest about what graft should do vs delegate to DVC.

**Graft should own**:
1. ✓ **Prompt packing** - This is graft-specific (diffs, attachments, context)
2. ✓ **LLM invocation** - Unless we adopt external process model
3. ✓ **Frontmatter parsing** - `.prompt.md` is graft's format
4. ? **Pipeline generation** - Could be simpler if just a dvc.yaml generator
5. ? **Change detection logic** - DVC already does this, graft adds REFINE/UPDATE semantics
6. ? **Build artifacts** - Could be DVC-tracked outputs in multi-stage pipeline
7. ✗ **Caching** - DVC handles this, graft should lean on it
8. ✗ **Dependency tracking** - DVC's core competency
9. ✗ **Remote storage** - DVC feature, graft should encourage usage

**Graft's unique value**:
- **Synthesis-specific tooling**: Packing prompts with diffs and context
- **Documentation workflow**: The `.prompt.md` format and conventions
- **LLM intelligence**: REFINE vs UPDATE patching strategies
- **Git-native change detection**: Using git history for smart updates

**Graft should NOT**:
- Reimplement caching (use DVC)
- Reimplement dependency tracking (use DVC)
- Ignore DVC features (params, experiments, metrics)
- Hide DVC (make it transparent, not abstracted away)

### Rethinking Graft's Architecture

**Current model**: Graft is a wrapper around DVC
```
User → graft rebuild → graft generates dvc.yaml → dvc repro → graft's scripts
```

**Alternative: Graft as DVC pipeline generator + helper tools**
```
User → graft init → dvc.yaml generated
User → dvc repro → Standard DVC (graft's Python tools in stages)
User → graft status → Pretty wrapper around dvc status
User → graft new <name> → Creates .prompt.md, updates dvc.yaml
```

**Benefits**:
- DVC is the primary interface (standard tooling)
- Graft is just helpers (pack_prompt.py, render_llm.py)
- Less magic, more transparency
- Full access to DVC features (experiments, metrics, remote cache)

**Downsides**:
- Less integrated UX
- Users need to understand DVC
- `graft rebuild` convenience lost

**Hybrid**: Graft commands as aliases
```bash
graft rebuild → dvc repro
graft status → dvc status (with graft-specific formatting)
graft push → dvc push
graft pull → dvc pull
graft experiment → dvc exp run
```

### Specific Recommendations to Explore

#### Recommendation 1: Multi-Stage Pipelines

**Current**: One stage per `.prompt.md`
```yaml
stages:
  report:
    cmd: python render_llm.py docs/report.prompt.md
    deps: [report.prompt.md, data.csv]
    outs: [report.md]
```

**Proposed**: Split pack and render
```yaml
stages:
  pack_report:
    cmd: python pack_prompt.py docs/report.prompt.md -o build/report.promptpack.txt
    deps:
      - docs/report.prompt.md
      - data.csv
    outs:
      - build/report.promptpack.txt
      - build/report.params.json

  render_report:
    cmd: python render_llm.py build/report.promptpack.txt -o docs/report.md
    deps:
      - build/report.promptpack.txt
      - build/report.params.json
    outs:
      - docs/report.md
```

**Benefits**:
- Packing is deterministic → cache aggressively
- Can re-render with different params without re-packing
- DVC handles caching of intermediate artifacts
- Standard DVC pattern (multi-stage pipelines)

#### Recommendation 2: DVC Params Integration

**Current**: Params in `.prompt.md` frontmatter or env vars

**Proposed**: Use `params.yaml` (DVC standard)
```yaml
# params.yaml
llm:
  model: claude-sonnet-4.5
  default_temperature: 0.3

reports:
  monthly:
    temperature: 0.5
  creative:
    temperature: 0.9

# docs/report.prompt.md
---
deps: [data.csv]
params:
  - llm.model
  - reports.monthly.temperature
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Benefits**:
- `dvc params diff` works
- `dvc exp run --set-param` works
- Centralized configuration
- Standard DVC pattern

#### Recommendation 3: Lean Into DVC Experiments

**Add**:
```bash
graft experiment run --set temperature=0.9 report
# → dvc exp run with param override

graft experiments show
# → dvc exp show with graft-specific formatting

graft experiment compare exp1 exp2
# → dvc exp diff with side-by-side .md outputs
```

**Value**: Trying different prompts/params becomes standard workflow

#### Recommendation 4: Document DVC Remote Setup

**Current**: No guidance on remote caching

**Add to docs**:
```markdown
## Team Collaboration with DVC Remote

Share expensive LLM outputs with your team:

1. Set up remote storage:
   ```bash
   dvc remote add team-cache s3://our-bucket/graft-cache
   dvc remote default team-cache
   ```

2. Push your generated docs:
   ```bash
   graft rebuild
   dvc push
   ```

3. Team members pull cached results:
   ```bash
   dvc pull
   # All expensive docs downloaded, no LLM costs
   ```

4. Only regenerate what changes:
   ```bash
   # Edit sources
   graft rebuild
   # Only affected docs regenerate, rest from cache
   ```
```

#### Recommendation 5: Transparent DVC Usage

**Current**: DVC is hidden implementation detail

**Proposed**: Make DVC visible and celebrated
```bash
graft status
# Output:
# Graft Status (backed by DVC)
#
# Run 'dvc dag' to see dependency graph
# Run 'dvc status' for detailed DVC status
#
# Grafts:
#   ✓ docs/report.md (up to date)
#   ✗ docs/analysis.md (deps changed)
#
# Tip: Use 'dvc remote' to share expensive outputs with your team
```

### Edge Cases and Concerns

#### Concern 1: DVC Doesn't Support All Graft Needs

**TTL-based caching**: DVC is hash-based, not time-based

**Options**:
- Contribute to DVC (add TTL support)
- Work around in graft wrapper
- Accept limitation, focus on deterministic use cases

#### Concern 2: DVC Complexity

**DVC has a learning curve**. Is graft supposed to hide this or embrace it?

**Hide it**:
- Pro: Easier onboarding
- Con: Users can't leverage DVC's power
- Con: Graft reimplements features poorly

**Embrace it**:
- Pro: Users get full DVC capabilities
- Pro: Less graft-specific magic
- Con: Steeper learning curve

#### Concern 3: Graft's Identity

**If graft is "just a DVC pipeline generator", what's the value proposition?**

**Possible answer**: Graft is to DVC as Next.js is to webpack
- DVC is powerful but general-purpose
- Graft is opinionated for LLM documentation workflows
- Graft provides conventions, helpers, and best practices
- But doesn't hide or reimplement DVC

### Trade-off Analysis

| Approach | Simplicity | Power | Reinvention | DVC Integration | Learning Curve |
|----------|-----------|-------|-------------|-----------------|----------------|
| Current (wrapper) | ✓✓ | ✓ | ✗ Some overlap | ✓✓ Hidden | ✓✓✓ Low |
| Pure DVC | ✗ Complex | ✓✓✓ | ✓✓✓ None | ✓✓✓ Native | ✗ High |
| DVC generator + helpers | ✓✓ | ✓✓✓ | ✓✓✓ None | ✓✓✓ Transparent | ✓✓ Medium |
| Heavy abstraction | ✓✓✓ | ✗ | ✗ High | ✗ Hidden | ✓✓✓ Low |

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: How should graft relate to DVC? More reliance, less reinvention?
2. **Feature-by-Feature Analysis**: For each graft feature, should it use DVC or implement custom logic?
3. **Reinvention Assessment**: Where is graft reinventing DVC? Is it justified?
4. **Value Proposition**: What is graft's unique value if not reimplementing DVC?
5. **Multi-Stage Pipelines**: Should graft adopt DVC multi-stage patterns?
6. **Params Integration**: Should graft use params.yaml?
7. **Experiments Integration**: Should graft expose DVC experiments?
8. **Remote Caching**: How should graft encourage DVC remote usage?
9. **Transparency vs Abstraction**: Should graft hide or celebrate DVC?
10. **Architecture Recommendation**: What's the right graft/DVC boundary?
11. **Migration Path**: If changing architecture, how to migrate?
12. **Trade-offs**: Honest assessment of approaches
13. **Use Cases**: Does this work for real users?
14. **Open Questions**: What needs validation?

Think like an architect who values simplicity and avoiding NIH (Not Invented Here) syndrome. If DVC does it well, use DVC. Only add graft-specific logic where there's genuine value. Be ruthlessly honest about whether graft is pulling its weight or just adding complexity.
