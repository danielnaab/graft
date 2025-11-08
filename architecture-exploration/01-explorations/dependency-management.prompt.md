---
deps:
  - architecture-exploration/00-sources/current-implementation.md
  - architecture-exploration/00-sources/design-goals.md
  - architecture-exploration/00-sources/open-questions.md
---

# Deep Exploration: Dependency Management

You are a systems architect analyzing dependency management for grafts, especially for directory-based grafts.

## Your Task

Think deeply about how dependencies should work, what constraints should exist, and how to prevent common pitfalls while maintaining flexibility.

### Current Model

```yaml
---
deps:
  - source-a.md
  - source-b.md
  - data/config.json
---
```

**Characteristics**:
- All deps are external to the graft
- Clear data flow: sources → graft → output
- No possibility of circular dependencies
- Easy to understand: graft pulls from outside

### The Directory Graft Question

For `<name>/graft.md`:

```
components/
  graft.md          (the definition)
  sources.md        (internal source)
  component-a.md    (output)
  component-b.md    (output)
  README.md         (output)
```

**Should `graft.md` be able to depend on `sources.md`?**

This opens several questions...

### Scenario Analysis

#### Scenario 1: Strict External-Only

```yaml
# components/graft.md
---
deps:
  - ../global-config.md
  - ../templates/component-template.md
  # FORBIDDEN: - sources.md
---
```

**Pros**:
- Clear data flow: external sources → graft → internal outputs
- No circular dependencies possible
- Easy to understand: graft is a "leaf" that doesn't depend on its own directory
- Encourages proper separation of concerns

**Cons**:
- Less flexible
- May require awkward directory structures
- Can't have "source materials" in the same directory

#### Scenario 2: Opt-In Internal Dependencies

```yaml
# components/graft.md
---
allow_internal_deps: true
deps:
  - sources.md        # OK because opted in
  - ../config.md
---
```

**Pros**:
- Flexibility when needed
- Clear marker that this graft is "special"
- Can keep related files together

**Cons**:
- Complexity: now there are two modes
- Need to prevent circular dependencies (sources.md can't depend on outputs)
- Harder to reason about data flow

#### Scenario 3: Fully Flexible

```yaml
# components/graft.md
---
deps:
  - sources.md
  - component-a.md  # Can depend on other outputs!
  - ../config.md
---
```

**Pros**:
- Maximum flexibility
- Supports iterative refinement workflows
- Powerful for complex transformations

**Cons**:
- Circular dependency hell
- Difficult to understand data flow
- Which files are sources vs outputs?
- DVC cycles become possible

### Circular Dependency Prevention

If we allow internal deps, how do we prevent:

```
components/
  graft.md → outputs: [a.md, b.md]
  a.md → used in generating b.md
  b.md → used in generating a.md
```

**Detection strategies**:

1. **Build-time detection**: DVC catches cycles
2. **Static analysis**: graft validates before generating DVC config
3. **Conventional prevention**: Mark files as "source" vs "output", enforce boundary
4. **Trust the user**: Let them create cycles, fail at runtime

### Mental Model Clarity

What mental model should users have?

**Model A: "Grafts are leaf transformations"**
- Grafts pull from external sources
- Grafts write to outputs
- Clear boundary, easy to understand
- Limits: sources must live outside

**Model B: "Grafts are directory scopes"**
- Grafts manage a directory
- Files in directory can be sources, outputs, or both
- Flexible but requires careful thought
- Risk: unclear data flow

**Model C: "Grafts are transformation pipelines"**
- Grafts can have staged dependencies
- `sources.md` → `intermediate.md` → `final.md`
- Powerful for complex workflows
- Risk: when does it stop being a graft and become a build system?

### Practical Examples

Let's think through concrete use cases:

#### Use Case 1: Research Report

```
research-report/
  graft.md
  01-data-sources.md      (source, manually written)
  02-analysis.md          (output from graft)
  03-conclusions.md       (output from graft)
  04-recommendations.md   (output from graft)
```

**Question**: Should `graft.md` depend on `01-data-sources.md`?

**Arguments for YES**:
- Natural: data sources live with the report
- Convenient: everything in one directory
- Clear naming: `01-` prefix signals "this is a source"

**Arguments for NO**:
- Enforces separation: sources live in `research-report/sources/`
- Clearer ownership: outputs are clearly graft-managed
- Prevents accidents: can't accidentally depend on outputs

#### Use Case 2: Exploration with Synthesis

```
naming-exploration/
  graft.md
  00-sources/
    criteria.md
    research.md
  01-explorations/
    option-a.md   (graft output)
    option-b.md   (graft output)
    option-c.md   (graft output)
  02-final/
    synthesis.md  (graft output)
```

**Question**: Should `02-final/synthesis.graft.md` depend on `01-explorations/*.md`?

This is a pipeline:
```
sources → explorations → synthesis
```

**Arguments for YES**:
- Natural workflow: synthesize the explorations
- All part of same investigation
- DVC can handle this fine

**Arguments for NO**:
- Each should be a separate graft
- Composition is better than interdependence
- Clearer to have three graft definitions

### The "Is It A Source?" Problem

If we allow internal deps, how do users know what's a source vs output?

**Strategy 1: Naming conventions**
```
00-sources/    (always sources)
01-outputs/    (always outputs)
raw-data.md    (suffix signals source)
report.md      (no suffix, assume output)
```

**Strategy 2: Explicit declaration**
```yaml
---
sources: [raw-data.md, notes.md]
outputs: [report.md, summary.md]
deps: [../global/config.md]
---
```

**Strategy 3: .gitignore-style**
```
# .graftignore
# These files are outputs, don't depend on them
*.output.md
generated/
```

**Strategy 4: Git tracking**
- Committed files = sources
- Generated files = outputs (in .gitignore)
- Graft validates: can't depend on gitignored files

### DVC Integration

How do internal deps affect DVC stages?

**Current** (external only):
```yaml
stages:
  report:
    cmd: graft render components/graft.md
    deps:
      - external/sources.md
      - components/graft.md
    outs:
      - components/output.md
```

**With internal deps**:
```yaml
stages:
  report:
    cmd: graft render components/graft.md
    deps:
      - components/sources.md      # Now a dep AND in the output directory
      - components/graft.md
      - external/config.md
    outs:
      - components/output.md
```

Is this confusing? Does it matter?

### Recommendation Framework

To decide on a policy, consider:

1. **Principle of Least Surprise**: What would users expect?
2. **Pit of Success**: What makes it easy to do the right thing?
3. **Error Prevention**: What prevents common mistakes?
4. **Flexibility vs Safety**: Where on this spectrum should graft be?
5. **Consistency**: Does this align with graft's philosophy?

### Trade-off Matrix

| Approach | Clarity | Flexibility | Safety | Complexity | Common Case |
|----------|---------|-------------|--------|------------|-------------|
| External only | ✓✓✓ | ✗ | ✓✓✓ | ✓✓✓ | ✓ |
| Opt-in internal | ✓✓ | ✓✓ | ✓✓ | ✓ | ✓✓ |
| Fully flexible | ✗ | ✓✓✓ | ✗ | ✗ | ✓ |

## Output Requirements

Produce a thorough analysis with:

1. **Executive Summary**: Recommended dependency policy and rationale
2. **Scenario Analysis**: Work through concrete examples
3. **Mental Models**: What should users understand about dependencies?
4. **Prevention Strategies**: How to avoid circular deps and confusion?
5. **DVC Implications**: How does this affect build configuration?
6. **Source vs Output**: How to distinguish? Does it matter?
7. **Trade-offs**: Honest assessment of each approach
8. **Recommendation**: Clear policy with reasoning
9. **Guidelines**: How should users structure their grafts?
10. **Open Questions**: What needs validation through use?

Think like a systems designer balancing power with clarity. The best tools make the right thing easy and the wrong thing hard (or at least obvious).
