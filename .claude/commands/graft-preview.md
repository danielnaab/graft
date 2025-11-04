# Preview Graft Documentation Changes

Show what will happen when documentation is regenerated, without actually regenerating.

This command helps you understand the impact of your changes before committing to a full regeneration.

Steps:

1. Run change detection analysis:
   - Execute `bin/graft status` to identify affected documents
   - For each changed stage, determine the action type (GENERATE/UPDATE/REFINE/REFRESH/MAINTAIN)
   - Analyze git diffs to understand what changed in sources vs prompts

2. For each affected document, report:
   - **Document name**: e.g., `docs/how-it-works.md`
   - **Action**: What type of regeneration will occur
   - **Reason**: What changed (sources, prompt instructions, or both)
   - **Impact**: Estimated scope (minor update vs full rewrite)
   - **Time estimate**: Based on document size and action type

3. Provide a cascade analysis:
   - Identify documents that depend on changed outputs
   - Show the propagation tree
   - Highlight multi-level impacts

4. Summary statistics:
   - Total documents affected
   - Estimated total time
   - Approximate AWS cost (at ~$0.003 per 1K tokens)
   - Breakdown by action type

Present output like:
```
## Graft Documentation Preview

### Changed Documents

1. **docs/how-it-works.md**
   - Action: UPDATE (sources changed, prompt unchanged)
   - Reason: scripts/pack_prompt.py was modified
   - Impact: Surgical updates to affected sections only
   - Estimated time: 45 seconds
   - Estimated cost: $0.05

2. **docs/command-reference.md**
   - Action: REFINE (prompt changed, sources unchanged)
   - Reason: Prompt instructions updated to add examples
   - Impact: Full document regeneration with new style
   - Estimated time: 60 seconds
   - Estimated cost: $0.07

3. **docs/overview.md**
   - Action: UPDATE (dependency cascade from #1)
   - Reason: Depends on docs/how-it-works.md which changed
   - Impact: Incorporate new information from dependency
   - Estimated time: 40 seconds
   - Estimated cost: $0.04

### Summary

- 📄 Total documents: 3
- ⏱️ Estimated time: 2 minutes 25 seconds
- 💰 Estimated cost: $0.16
- 🔄 Action breakdown:
  - UPDATE: 2 documents
  - REFINE: 1 document

### Cascade Tree

docs/how-it-works.md (changed)
  └── docs/overview.md (cascade)

Ready to regenerate? Run `/graft-regen`
```

This helps users make informed decisions about when to regenerate and what to expect.
