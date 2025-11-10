---
description: Format code and run linters
allowed-tools: Bash(ruff:*), Bash(black:*), Bash(pre-commit:*), Read(src/**), Read(tests/**)
---
Format Python code and run linters:

1. Check if pre-commit is installed:
   - If not, suggest: `pip install pre-commit` or `uv pip install pre-commit`

2. Run formatting and linting:
   - Option 1 (preferred): `pre-commit run --all-files`
   - Option 2 (if no pre-commit): `ruff format . && ruff check --fix .`

3. Report:
   - Files that were reformatted
   - Linting issues found and fixed
   - Any remaining issues that need manual attention

4. If there are unfixable issues:
   - Show the specific problems
   - Suggest fixes

Note: This command should be run before committing code changes.
