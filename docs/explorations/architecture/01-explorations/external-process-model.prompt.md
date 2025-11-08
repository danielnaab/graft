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

# Deep Exploration: External Process Pipeline Model

You are a systems architect conducting a thorough analysis of whether graft should refactor from native LLM patch handling to a pipeline model where output is streamed through external processes.

## Your Task

Think deeply and comprehensively about this architectural decision. Consider:

### Technical Architecture

1. **Current Model**: graft directly invokes LLMs via `render_llm.sh` and `pack_prompt.py`
   - What are the exact responsibilities of each component?
   - Where are the abstraction boundaries?
   - What coupling exists?

2. **Pipeline Model**: graft produces packed prompts, external tools process them
   - How would the data flow work?
   - What would stdin/stdout contain?
   - How would change detection work?
   - Where would the LLM invocation logic live?

3. **Unix Philosophy Alignment**
   - Does pipelining make graft more composable?
   - What's the "do one thing well" boundary?
   - How does this relate to tools like `jq`, `pandoc`, `awk`?

### Support for Non-LLM Transformations

1. **Current Limitations**: Everything goes through LLM prompt packing
2. **Pipeline Benefits**: Could pipe through any transformation tool
3. **Use Cases**:
   - Simple text processing (sed, awk, jq)
   - Code formatters (prettier, black, gofmt)
   - Data transformations (csvkit, xsv)
   - Custom scripts

### Change Detection Intelligence

The current system's smartest feature is understanding what changed:
- GENERATE vs REFINE vs UPDATE vs REFRESH vs MAINTAIN

**Critical Question**: In a pipeline model, where does this intelligence live?

Options:
1. Graft still does change detection, passes metadata to external tools
2. External tools are responsible for change detection
3. Hybrid: graft provides context, tools decide how to use it

### Practical Implications

1. **Backward Compatibility**: Can existing `.prompt.md` files work unchanged?
2. **Complexity**: Does this make graft simpler or more complex?
3. **Debuggability**: How easy is it to understand what went wrong?
4. **Build Artifacts**: What gets written to `build/`?

### Trade-off Analysis

Create a comprehensive trade-offs table:

| Aspect | Native Model (current) | Pipeline Model |
|--------|----------------------|----------------|
| Simplicity | ... | ... |
| Flexibility | ... | ... |
| Change detection | ... | ... |
| Debugging | ... | ... |
| Non-LLM support | ... | ... |
| Performance | ... | ... |

## Output Requirements

Produce a thorough analysis document with:

1. **Executive Summary**: One paragraph answering "should we do this?"
2. **Deep Dive**: Detailed exploration of each aspect above
3. **Concrete Examples**: Show what code/config would look like in each model
4. **Trade-offs Table**: Comprehensive comparison
5. **Recommendation**: Clear position with nuanced reasoning
6. **Open Questions**: What needs prototyping to validate?

Think like a principal engineer who needs to make a decision they'll live with for years. Be rigorous, nuanced, and honest about trade-offs.
