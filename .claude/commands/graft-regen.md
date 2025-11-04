# Regenerate Graft Documentation

Regenerate all stale Graft documentation by running the full DVC pipeline.

Steps:

1. First, run validation to understand what needs regeneration:
   - Execute `bin/graft status` to see which stages have changed
   - Count and report the number of docs that will regenerate
   - Estimate time: ~30-60 seconds per document

2. Ask user for confirmation:
   - Show the list of docs that will regenerate
   - Estimate total time and approximate AWS cost
   - Ask: "Proceed with regeneration? (yes/no)"

3. If confirmed, run `bin/graft rebuild`:
   - Monitor the output and show progress
   - Report each document as it completes
   - Track timing for performance awareness
   - Handle errors gracefully with clear messages

4. After completion:
   - Run `git status` to show what changed
   - Suggest reviewing the changes with `git diff`
   - Remind user to commit the regenerated files
   - Optionally offer to show a summary of the changes

Present progress updates like:
```
Regenerating 3 documents...
⏳ Rendering: how-it-works (1/3)...
✅ Completed: how-it-works (took 45s)
⏳ Rendering: api-reference (2/3)...
✅ Completed: api-reference (took 52s)
⏳ Rendering: overview (3/3)...
✅ Completed: overview (took 38s)

🎉 All documents regenerated successfully!

Git status:
  M docs/how-it-works.md
  M docs/api-reference.md
  M docs/overview.md

Next steps:
1. Review changes: git diff docs/
2. Commit: git add docs/ && git commit -m "Regenerate documentation"
```

If errors occur, parse the error messages and provide actionable guidance.
