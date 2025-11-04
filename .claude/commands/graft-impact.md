# Show Graft Documentation Impact

Show which documentation will be affected if you edit a specific file. This helps you understand the cascade effect before making changes.

**Usage**: `/graft-impact <file-path>`

**Example**: `/graft-impact docs/how-it-works.md`

Steps:

1. Ask user for the file path if not provided:
   - Suggest files from recent git changes
   - Offer to analyze all changed files in working directory

2. Run impact analysis using `bin/graft uses <file>`:
   - This shows all prompts that list the file as a dependency
   - Parse the output to extract affected documents

3. For each affected document:
   - Show which prompt depends on the file
   - Identify the generated output
   - Indicate if it's a direct or transitive dependency
   - Estimate regeneration time and cost

4. Build a dependency tree visualization:
   - Show the file at the root
   - Show all directly dependent prompts
   - Show documents that depend on those (cascade)
   - Use indentation to show hierarchy

5. Provide actionable guidance:
   - If no dependencies: "This file is not used by any prompts"
   - If few dependencies: List them with regeneration estimates
   - If many dependencies: Summarize count and total impact
   - Suggest running `/graft-preview` to see full analysis

Present output like:
```
## Impact Analysis: docs/how-it-works.md

This file is used by 2 prompts:

### Direct Dependencies

1. **docs/overview.prompt.md** → docs/overview.md
   - Uses how-it-works.md to synthesize high-level overview
   - Regeneration time: ~45 seconds
   - Cost: ~$0.05

2. **docs/use-cases.prompt.md** → docs/use-cases.md
   - References technical details from how-it-works.md
   - Regeneration time: ~50 seconds
   - Cost: ~$0.06

### Dependency Tree

📄 docs/how-it-works.md (editing this file)
  ├── 📋 docs/overview.prompt.md
  │   └── 📄 docs/overview.md (will regenerate)
  └── 📋 docs/use-cases.prompt.md
      └── 📄 docs/use-cases.md (will regenerate)

### Impact Summary

- 📄 Documents affected: 2
- ⏱️ Total regeneration time: ~1 minute 35 seconds
- 💰 Estimated cost: $0.11

If you edit this file, run `/graft-preview` to see what will change.
```

This helps users understand the blast radius of their edits and plan accordingly.
