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

# Deep Exploration: Naming Conventions

You are a systems architect analyzing the naming patterns for graft definitions and their outputs.

## Your Task

Think deeply about naming conventions that are:
- Intuitive and discoverable
- Consistent with the "graft" metaphor
- Clear about intent and scope
- Compatible with git and DVC
- Extensible for future features

### Current State

**Existing pattern**:
```
docs/api-reference.prompt.md → docs/api-reference.md
```

- Convention: `<name>.prompt.md` produces `<name>.md`
- Clear 1:1 mapping
- "prompt" signals "this is a graft definition"

### Proposed Patterns

The TODO.md suggests:

#### Pattern A: `<name>.graft.md`
```
api-reference.graft.md
  → api-reference.md
  → api-reference.json
  → api-reference/examples/*.md
```

**Semantics**: Can produce any `<name>.*` outputs, including subdirectories

**Questions**:
- What's the "primary" output?
- How do you reference outputs in dependencies?
- Can `foo.graft.md` produce `bar.md`? (probably not, but why?)

#### Pattern B: `<name>/graft.md`
```
api-reference/
  graft.md (the definition)
  → api-reference.md (or REPORT.md or README.md?)
  → examples.md
  → schema.json
  → api-reference/v2/endpoints.md
```

**Semantics**: Directory-scoped graft, produces files within directory

**Questions**:
- What's the "primary" output convention?
- Can it produce files outside the directory?
- How do external grafts depend on files inside?
- Is the graft definition itself considered part of the graft?

### Metaphor Consistency

"Graft" as noun and verb:

**Noun usage**:
- "The api-reference graft" - what file is this?
- "Show me the graft for api docs" - where do I look?
- "This graft produces three outputs" - clear which file defines it?

**Verb usage**:
- "Graft the API docs onto the tree" - what command?
- "This graft grows multiple branches" - metaphor works?

**Question**: Do these naming patterns reinforce or confuse the metaphor?

### Discovery and Intuition

Imagine a new user exploring the repository:

```
docs/
  overview.graft.md
  api/
    graft.md
    reference.md
    examples.md
  architecture.prompt.md
  README.md
```

**Questions**:
- Can they tell what's a graft definition vs output?
- Is it obvious what produces what?
- Can they guess the pattern without docs?
- What about mixing `.graft.md` and `.prompt.md`?

### DVC and Build Integration

How do these patterns affect automation?

**Current** (`*.prompt.md`):
```python
prompt_files = glob("**/*.prompt.md")
for prompt in prompt_files:
    output = prompt.replace(".prompt.md", ".md")
    create_stage(prompt, output)
```

**With `.graft.md`**:
- Still 1:1 mapping for single outputs?
- Need frontmatter for multiple outputs?
- How does generate_dvc.py discover patterns?

**With `*/graft.md`**:
- Must read graft.md to know outputs?
- Or glob all files in directory?
- How to distinguish outputs from source files?

### Primary Output Conventions

For `<name>/graft.md`, what should the primary output be named?

**Option A: Match the directory name**
```
api-reference/
  graft.md
  → api-reference.md (primary)
  → supporting.md
```

**Option B: Standard primary name**
```
api-reference/
  graft.md
  → REPORT.md (always the primary)
  → other files
```

**Option C: Declared in frontmatter**
```yaml
---
primary: api-reference.md
outputs: [api-reference.md, schema.json, examples.md]
---
```

**Option D: No distinction**
- All outputs are equal
- Dependencies reference specific files

### Coexistence

Should `.prompt.md`, `.graft.md`, and `graft.md` all be supported?

**Scenarios**:

1. **Migration path**: `.prompt.md` exists, add `.graft.md` gradually
2. **Mixed usage**: Use `.prompt.md` for simple, `.graft.md` for complex
3. **Eventually consistent**: `.prompt.md` is legacy, `.graft.md` is future
4. **Permanent coexistence**: Different patterns for different use cases

### File Extension Considerations

Why `.graft.md` vs `.graft` vs `graft.md` vs `graftfile`?

**`.graft.md`**:
- Pro: Renders as markdown in GitHub
- Pro: Can document the graft in the same file
- Con: Non-standard double extension

**`.graft`**:
- Pro: Clean, single extension
- Pro: Clear "this is a graft definition"
- Con: Won't render in GitHub UI
- Con: Need syntax highlighting config

**`graft.md`** (in directory):
- Pro: Standard markdown
- Pro: Clear location (always at directory root)
- Con: Generic name, harder to glob

**`Graftfile`** (like Makefile, Dockerfile):
- Pro: Follows precedent
- Pro: No extension needed
- Con: Not markdown, loses documentation benefit

### Trade-off Matrix

| Pattern | Discoverability | Metaphor Fit | DVC Integration | Flexibility | Migration |
|---------|----------------|--------------|-----------------|-------------|-----------|
| `.prompt.md` (current) | ✓ | ✗ | ✓✓ | ✗ | N/A |
| `.graft.md` | ✓ | ✓✓ | ✓ | ✓✓ | ✓ |
| `<name>/graft.md` | ✓✓ | ✓✓ | ? | ✓✓✓ | ✓ |
| `.graft` | ? | ✓ | ✓ | ✓✓ | ? |
| `Graftfile` | ✓ | ✓ | ✓ | ✓✓ | ? |

### Edge Cases

Consider:

1. **Nested grafts**:
   ```
   docs/
     graft.md
     api/
       graft.md
   ```
   Does this make sense? Is it allowed?

2. **Name collisions**:
   ```
   api.graft.md
   api/graft.md
   ```
   What if both exist?

3. **Cross-references**:
   ```yaml
   deps:
     - api-reference/schema.json  # from api-reference/graft.md
   ```
   Is this clear? Should deps point to the graft definition instead?

## Output Requirements

Produce a comprehensive analysis with:

1. **Executive Summary**: Recommended naming convention(s) and rationale
2. **Pattern Analysis**: Deep dive into each pattern's implications
3. **Concrete Examples**: Show real-world usage of each pattern
4. **Metaphor Assessment**: How well does each align with "graft"?
5. **User Experience**: How intuitive is each for different users?
6. **Implementation Impact**: What code changes are needed?
7. **Migration Strategy**: How to evolve from current `.prompt.md`?
8. **Trade-offs**: Honest comparison table
9. **Recommendation**: Clear, justified position
10. **Open Questions**: What needs user testing?

Think like a language designer. Small details matter. Consistency is key. Intuition trumps cleverness.
