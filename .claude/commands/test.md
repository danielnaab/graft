---
description: Run pytest tests with coverage analysis
allowed-tools: Bash(pytest:*), Bash(uv pip install:*), Read(tests/**), Read(src/**), Read(pyproject.toml)
---
Run pytest tests for the Graft project:

1. First check if dependencies are installed (look for .venv)
2. If needed, remind about installing with `uv pip install -e ".[test]"`
3. Run `pytest -q` to execute all tests
4. If tests fail:
   - Analyze the failures in detail
   - Identify the root cause (package not installed, import errors, assertion failures)
   - Suggest specific fixes
5. Report:
   - Number of tests passed/failed
   - Test coverage gaps if any
   - Next steps for fixing failures

Remember: All tests should be black-box subprocess tests via `python -m graft.cli`
