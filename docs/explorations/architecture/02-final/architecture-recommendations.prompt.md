---
deps:
  - docs/explorations/architecture/00-sources/current-implementation.md
  - docs/explorations/architecture/00-sources/design-goals.md
  - docs/explorations/architecture/00-sources/open-questions.md
  - docs/explorations/architecture/01-explorations/external-process-model.md
  - docs/explorations/architecture/01-explorations/multi-file-outputs.md
  - docs/explorations/architecture/01-explorations/naming-conventions.md
  - docs/explorations/architecture/01-explorations/dependency-management.md
  - docs/explorations/architecture/01-explorations/lock-mechanism.md
lock:
  enabled: true
  reason: "Completed architecture exploration - historical record"
  date: 2024-11-08T07:07:00Z
---

# Graft Architecture Recommendations: Final Synthesis

You are a principal systems architect synthesizing multiple deep explorations into a coherent, principled set of recommendations for graft's evolution.

## Your Task

Read all the exploration documents and synthesize them into a unified architectural vision that:

1. **Is Internally Consistent**: Decisions reinforce each other
2. **Honors Core Principles**: Aligns with graft's design goals
3. **Balances Trade-offs**: Makes clear, justified compromises
4. **Provides Clear Guidance**: Implementable and testable
5. **Thinks Long-term**: Will these decisions age well?

## Synthesis Approach

### Step 1: Identify Themes

What patterns emerge across the explorations?
- Common tensions (e.g., flexibility vs. simplicity)
- Shared principles (e.g., git-native, composability)
- Interdependencies (e.g., naming affects multi-file, which affects locking)

### Step 2: Find Coherence

How do the pieces fit together?

**Example coherence check**:
- If we adopt external process model → changes how change detection works → affects what metadata goes in build/ → impacts debugging experience
- If we support multi-file outputs → changes what "locking" means → affects DVC integration → influences naming patterns

**Your task**: Map these interdependencies and find a coherent whole.

### Step 3: Make Integrated Decisions

Don't just list five separate recommendations. Instead, present a unified architectural vision:

```
Given that [decision A], it follows that [decision B], which means [decision C] makes sense...
```

### Step 4: Validate Against Principles

For each major decision, check:
- ✓ Composability: Does this play well with other tools?
- ✓ Transparency: Can users understand what's happening?
- ✓ Reproducibility: Same inputs → same outputs?
- ✓ Git-native: Embraces git as truth?
- ✓ Flexibility: Supports diverse use cases?
- ✓ Incrementality: Only regenerate what changed?

### Step 5: Plan Evolution

What's the migration path?

**Phase 1**: Can current `.prompt.md` files continue working?
**Phase 2**: How do we introduce new features without breaking existing usage?
**Phase 3**: What's the "new normal" once fully migrated?

## Output Structure

### 1. Executive Summary (1-2 paragraphs)

The 10,000-foot view: What is the recommended direction and why?

### 2. Core Architectural Decisions

For each major area, provide:

#### External Process Model
- **Decision**: [Clear position]
- **Rationale**: [Why this choice?]
- **Implications**: [What changes?]
- **Open Questions**: [What needs prototyping?]

#### Multi-File Outputs
- **Decision**: ...
- **Rationale**: ...
- **Implications**: ...
- **Open Questions**: ...

#### Naming Conventions
- **Decision**: ...
- **Rationale**: ...
- **Implications**: ...
- **Open Questions**: ...

#### Dependency Management
- **Decision**: ...
- **Rationale**: ...
- **Implications**: ...
- **Open Questions**: ...

#### Lock Mechanism
- **Decision**: ...
- **Rationale**: ...
- **Implications**: ...
- **Open Questions**: ...

### 3. Unified Design Example

Show how all these decisions work together with a concrete example:

```
# Example graft structure
research/
  deep-dive.graft.md
  sources/
    data.csv
    context.md
  outputs/
    report.md
    summary.md
    visualization.html
```

Walk through:
- How this graft is defined
- What the frontmatter looks like
- How dependencies flow
- What happens when sources change
- How locking works
- What DVC stages are generated
- Where build artifacts live

### 4. Coherence Analysis

Explain how the decisions reinforce each other:

"Because we chose X for naming, it makes Y approach to multi-file natural, which means Z for locking is straightforward..."

### 5. Principles Validation

For each design goal from `design-goals.md`, explicitly show how the architecture honors it:

- **Composability**: [How these decisions enable composition]
- **Transparency**: [How users understand what's happening]
- **Reproducibility**: [How builds are deterministic]
- **Git-native**: [How git is embraced]
- **Flexibility**: [How diverse use cases are supported]
- **Incrementality**: [How waste is minimized]

### 6. Migration Strategy

**Phase 0: Current State**
- `.prompt.md` files work as they do today

**Phase 1: Backward Compatible Additions**
- What features can be added without breaking existing usage?
- How do `.prompt.md` and new patterns coexist?

**Phase 2: Transition Period**
- How do users gradually migrate?
- What tools help with migration?
- How long should this phase last?

**Phase 3: New Steady State**
- What's the "blessed" way to use graft?
- Is `.prompt.md` legacy or still primary?
- What documentation captures the patterns?

### 7. Implementation Priorities

What should be built first?

1. **Must-Have**: [Critical path items]
2. **Should-Have**: [Important but not blocking]
3. **Could-Have**: [Nice to have, lower priority]
4. **Won't-Have (Yet)**: [Explicitly deferred]

### 8. Validation Plan

How do we know if these decisions are right?

- **Prototyping**: What needs a spike/proof-of-concept?
- **User Testing**: What patterns need validation with real users?
- **Metrics**: What would we measure to assess success?
- **Failure Modes**: What would indicate we chose wrong?

### 9. Open Questions

What remains uncertain?

For each question:
- What's the uncertainty?
- Why couldn't we decide now?
- What information would resolve it?
- How do we proceed without that answer?

### 10. Long-Term Vision

What could graft become?

Paint a picture of graft in 2-3 years:
- What problems does it solve elegantly?
- How do users think about it?
- What's the "aha moment" for new users?
- How has it stayed true to its principles while growing?

## Guidelines for Synthesis

### Think Holistically

Don't just concatenate the five exploration documents. Find the through-lines, the tensions, the synergies.

### Be Decisive But Humble

Make clear recommendations, but acknowledge uncertainty where it exists.

### Favor Simplicity

When two approaches are close, choose the simpler one. Complexity is a cost users pay forever.

### Think About Users

How will a new user discover these patterns? How will an expert user compose powerful workflows? Make it obvious.

### Consider Implementation Realities

Some designs look great on paper but are nightmares to implement or maintain. Factor in engineering reality.

### Honor the Metaphor

"Graft" is a beautiful metaphor. Does this architecture make the metaphor stronger or weaker? Let the metaphor guide design.

## Output Requirements

This should be the **definitive architecture document** for graft's evolution.

Someone should be able to:
1. Understand the complete vision
2. See how pieces fit together
3. Know what to build first
4. Understand the reasoning behind each decision
5. Validate whether decisions were correct
6. Explain the architecture to others

This is a design document that will be referenced for years. Make it comprehensive, clear, and compelling.

**Aim for 8-12 pages of thoughtful, well-structured analysis.**
