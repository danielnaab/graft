# Validate Graft Documentation

Run a complete validation of the Graft documentation pipeline to check for:
1. DVC pipeline synchronization (dvc.yaml matches prompt files)
2. Missing dependencies (all files referenced in deps exist)
3. Stale documentation (generated docs match their sources and prompts)

Execute these checks:

1. First, run `bin/graft sync` to generate dvc.yaml and check if it differs from the committed version:
   - If different, report: "dvc.yaml needs regeneration. Run 'bin/graft sync' and commit."
   - If same, report: "✅ dvc.yaml is synchronized"

2. Check for missing dependencies by:
   - Finding all `*.prompt.md` files
   - Extracting deps from frontmatter
   - Verifying each file exists
   - Report any missing files with the prompt that references them

3. Check for stale documentation by running `bin/graft status`:
   - Parse the output to identify stages that have "changed"
   - List all docs that need regeneration
   - If none, report: "✅ All documentation is up to date"
   - If any, report which docs are stale and suggest running `bin/graft rebuild`

Present results in a clear summary:
```
## Graft Validation Results

✅ DVC pipeline synchronized
✅ All dependencies present
❌ Stale documentation detected:
  - docs/how-it-works.md (sources changed)
  - docs/api-reference.md (prompt changed)

Fix: Run `bin/graft rebuild` to regenerate stale documentation
```

If all checks pass, celebrate with: "🎉 All validation checks passed! Documentation is synchronized."
