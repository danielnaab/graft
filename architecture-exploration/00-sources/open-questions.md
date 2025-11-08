# Open Architectural Questions

These questions come from the graft TODO.md and represent key design decisions to be explored.

## 1. External Process Pipeline Model

**Question**: Should graft refactor from native LLM patch handling to pipelining output through external processes?

**Context**: Currently, pack_prompt.py and render_llm.sh handle LLM invocation directly. An alternative would be to make graft a "prompt packing" tool that outputs to stdout, allowing external tools to process the packed prompt.

**Implications**:
- How would this affect change detection intelligence?
- What are the benefits for supporting non-LLM transformations?
- How does this align with Unix philosophy?
- What's the boundary between graft's responsibility and external tools?

## 2. Multi-File Output Support

**Question**: How should grafts support generating multiple output files from a single specification?

**Context**: Currently, one `.prompt.md` produces one `.md` file. But some grafts naturally produce multiple artifacts (e.g., an exploration producing multiple variations, or documentation with associated assets).

**Implications**:
- Should all outputs be tracked in DVC?
- How do dependencies work for multi-file grafts?
- How do we handle partial regeneration?

## 3. Naming Conventions

**Question**: What should be the supported patterns for graft definitions?

**Current proposal**:
- `<name>.graft.md`: Produces `<name>.*` outputs (possibly in subdirectories)
- `<name>/graft.md`: Defines `<name>` directory as a graft, produces files within it

**Implications**:
- What's the primary output for directory-based grafts?
- How do these patterns interact with DVC?
- Should `.graft.md` replace `.prompt.md` or coexist?

## 4. Dependency Management

**Question**: Should directory-based grafts be able to depend on files within their own directory, or only external dependencies?

**Context**: For `<name>/graft.md`, should dependencies be:
- Only from outside the directory (enforced isolation)?
- Allow internal dependencies if user opts in?
- Completely flexible?

**Implications**:
- Risk of circular dependencies
- Clarity of data flow
- Flexibility vs. footguns

## 5. Lock Mechanism

**Question**: How should "locked" grafts work to prevent expensive regenerations?

**Context**: Some explorations are point-in-time analyses that should be preserved even if dependencies change. A `lock: true` frontmatter field could prevent regeneration.

**Implications**:
- Should locked grafts be in DVC at all?
- How do we make it obvious a graft is locked?
- What happens if a locked graft's sources are missing?
- Should locking be per-file or per-directory?

## 6. Consistency with System Goals

**Meta-question**: How do all these changes work together to create a coherent, principled system that remains true to graft's core philosophy while becoming more flexible and powerful?
