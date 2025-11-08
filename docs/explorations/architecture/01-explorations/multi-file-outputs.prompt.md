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

# Deep Exploration: Multi-File Output Support

You are a systems architect analyzing how graft should support generating multiple output files from a single graft specification.

## Your Task

Thoroughly explore the design space for multi-file grafts. Be comprehensive and consider edge cases.

### Current Constraints

- One `.prompt.md` → exactly one `.md` file
- DVC stages have `outs: [single-file]`
- Change detection logic assumes single output
- Build artifacts (`.promptpack.txt`, `.params.json`) map 1:1 to outputs

### Use Cases to Consider

1. **Exploration with variations**
   ```
   naming-exploration/
     brainstorm.graft.md
     → option-a.md
     → option-b.md
     → option-c.md
   ```

2. **Documentation with assets**
   ```
   api-docs.graft.md
     → api-docs.md
     → api-docs.json (machine-readable)
     → api-docs.html (formatted)
   ```

3. **Directory as output**
   ```
   components/graft.md
     → component-a.md
     → component-b.md
     → README.md
   ```

4. **Conditional outputs**
   - Generate file B only if condition X is met
   - Skip expensive generations if source unchanged

### Design Questions

#### 1. Output Declaration

How should grafts declare multiple outputs?

**Option A: Explicit list in frontmatter**
```yaml
---
deps: [sources.md]
outputs: [output-a.md, output-b.md, output-c.md]
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Option B: Pattern-based**
```yaml
---
deps: [sources.md]
output_pattern: "variations/*.md"
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---
```

**Option C: Discovery-based**
- Graft runs, writes files, graft discovers what was written
- Pros: Maximum flexibility
- Cons: Less predictable, harder for DVC

**Option D: Manifest-based**
- Graft outputs a manifest file listing what it produced
- Second pass updates DVC config

#### 2. DVC Integration

How do multiple outputs work with DVC stages?

**Considerations**:
- DVC tracks each output separately
- Can mark some outputs as cache:false
- Metrics and plots have special handling
- Dependencies are at stage level, not per-output

**Questions**:
- One DVC stage per graft (multiple outs)?
- Multiple stages per graft (one out each)?
- Hybrid approach?

#### 3. Change Detection

Current model:
```
sources changed → check git diff → determine action → regenerate output
```

With multiple outputs:
- Do all outputs regenerate together?
- Can individual outputs regenerate independently?
- How do we know which outputs need updating?

**Example scenario**:
```
docs/graft.md produces:
  - overview.md (depends on all sources)
  - api-reference.md (depends on api-schema.json only)

api-schema.json changes.
```

Should overview.md regenerate? It technically doesn't depend on the schema.

#### 4. Primary vs Secondary Outputs

Some outputs are "primary" (the main document), others are "artifacts" (supporting files).

**Questions**:
- Does this distinction matter?
- Should primary output be named after the graft?
- How do dependencies reference specific outputs?

#### 5. Naming Conventions

How do naming patterns interact with multi-file?

- `name.graft.md` → `name.md` + `name-*.md` + `name/*.md`?
- `name/graft.md` → all files in `name/`?
- Explicit output paths in frontmatter?

### Partial Regeneration

**Scenario**: A graft produces 10 files. Source A changes, which only affects 2 outputs.

**Options**:
1. Regenerate all 10 (simple, potentially wasteful)
2. Regenerate only affected 2 (complex, efficient)
3. Let graft decide (requires graft to understand dependencies)

### Build Artifacts

Currently:
```
build/
  name.promptpack.txt
  name.params.json
  name.context.json
```

With multiple outputs:
```
build/
  name.promptpack.txt (still one packed prompt?)
  name.params.json
  name.context.json
  name.manifest.json (list of outputs?)
```

Or per-output artifacts?

### Trade-off Analysis

Create a comprehensive comparison:

| Approach | Complexity | Flexibility | DVC Integration | Performance | Debugging |
|----------|-----------|-------------|-----------------|-------------|-----------|
| Single output (current) | ... | ... | ... | ... | ... |
| Explicit output list | ... | ... | ... | ... | ... |
| Pattern-based | ... | ... | ... | ... | ... |
| Discovery-based | ... | ... | ... | ... | ... |

## Output Requirements

Produce a thorough analysis with:

1. **Executive Summary**: Recommended approach and why
2. **Use Case Analysis**: Which use cases are most important?
3. **Design Proposals**: 2-3 concrete designs with examples
4. **Implementation Sketch**: What code would change?
5. **Trade-offs**: Honest assessment of each approach
6. **Migration Path**: How do existing grafts continue working?
7. **Recommendation**: Clear position with reasoning

Be rigorous. Think about edge cases. Consider what makes debugging easy.
