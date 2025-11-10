---
description: Verify CLI contract compliance for implemented commands
allowed-tools: Bash(python -m graft.cli:*), Read(docs/cli-spec.md), Read(schemas/cli/**), Read(src/graft/cli.py)
---
Verify that all implemented commands follow the CLI contract specification:

1. Read `docs/cli-spec.md` to understand the contract

2. For each implemented command, verify:
   - Accepts `--json` flag where applicable
   - Returns proper exit codes:
     - 0 for success
     - 1 for user errors (bad input, missing files, invalid YAML)
     - 2 for system errors (permissions, unexpected exceptions)
   - JSON output (when `--json` is used) matches the schema in `schemas/cli/`
   - Error messages are helpful and include relevant paths
   - Human-readable output (without `--json`) is clear and informative

3. Test each command with:
   - Valid inputs → should succeed with exit code 0
   - Missing files → should fail with exit code 1 and helpful message
   - Invalid YAML → should fail with exit code 1 and parse error
   - Missing required fields → should fail with exit code 1

4. Report:
   - ✅ Commands that comply with the contract
   - ❌ Commands that violate the contract (with specific issues)
   - Suggestions for fixes
