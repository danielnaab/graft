---
status: working
purpose: "Session note: first software-factory template + consumer wiring, and engine gaps discovered"
---

# Plan Template Implementation

Session implementing the first software-factory vertical slice: a plan template that agents use to decompose tasks into vertical slices.

## What shipped

### Software-factory submodule

- `templates/plan.md` -- Tera template with `{% if %}` guarded state sections (verify, changes) and story + vertical slices output format
- `docs/specifications/prompt-templates.md` -- Living spec defining the template/consumer contract
- `knowledge-base.yaml` -- Added `templates/**` to sources and write rules
- `AGENTS.md` -- Added templates to orientation, write boundaries, navigation

### Consumer wiring (graft.yaml)

- `plan` command: `run: "cat"`, renders template with verify + changes context
- `verify` state query: cargo fmt/clippy/test with section headers, `deterministic: true` cache
- `changes` state query: git log + diff stat, `deterministic: false` (uncommitted changes aren't commit-keyed)

## Review findings and fixes applied

1. **`changes` cache was wrong**: `deterministic: true` caches by commit hash, but `git diff --stat` varies independently. Fixed to `deterministic: false`.
2. **Template had no framing**: Added AGENTS.md reference and codebase exploration instruction. Added "Plan the task given to you" to signal the template is a companion to a user-provided task.
3. **Verify query lost errors**: `echo ---` separators looked like YAML frontmatter; `tail` lost the beginning of output where errors appear. Replaced with `## Format`/`## Lint`/`## Tests` section headers and `head` for error-first capture.

## Engine gaps discovered

### Task injection into templates

Templates currently have access to built-in variables (`repo_name`, `git_branch`, `commit_hash`, `repo_path`) and state query results (`state.<name>`). There is no mechanism to inject user-provided input (like a task description) into the template context.

**Current state**: `graft run plan [ARGS]...` passes ARGS to the `run:` command (e.g., `cat`), not to the template renderer. The `TemplateContext` in `crates/graft-engine/src/template.rs` only accepts `repo_path`, `commit_hash`, `git_branch`, and `state_results`.

**Impact**: The plan template defines an output format and injects project state, but has no slot for the task being planned. Users must compose the task description externally:
- Chat context: paste template output alongside task description
- CLI composition: `graft run plan | claude --print "implement feature X" -`
- State query workaround: `task` query that reads from a file

**Proposed fix**: Add CLI argument injection into `TemplateContext`. Could expose as `{{ args }}` (full args string) or `{{ args[0] }}` etc. This would enable `graft run plan "implement feature X"` where the template uses:

```jinja2
{% if args is defined %}
## Task

{{ args }}
{% endif %}
```

**Files to modify**:
- `crates/graft-engine/src/template.rs` -- Add `args` to `TemplateContext::new()`
- `crates/graft-engine/src/command.rs` -- Pass CLI args to template context before rendering
- `crates/graft-engine/src/domain.rs` -- Possibly extend `CommandDef` if args need config

This is a small, focused engine change. The template renderer already uses Tera context injection; adding one more variable is straightforward.

### Deterministic cache for working-tree queries

`deterministic: true` caches by commit hash, which is wrong for queries that inspect the working tree (uncommitted changes, formatting status). The only alternative is `deterministic: false` which disables caching entirely.

A middle ground would be useful: cache keyed by commit hash + working tree dirty state, or a short TTL. The current Stage 1 comment in `domain.rs` notes TTL is not yet implemented.

## Related

- [Software-factory workflow design research](../.graft/software-factory/notes/2026-02-21-workflow-design-research.md)
- [Prompt templates spec](../.graft/software-factory/docs/specifications/prompt-templates.md)
