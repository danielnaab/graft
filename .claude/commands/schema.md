---
description: Validate or update JSON schemas
allowed-tools: Read(schemas/**), Edit(schemas/**), Bash(python -m graft.cli:*), Read(src/graft/services/**), Read(docs/cli-spec.md)
---
Work with Graft JSON schemas to ensure they match CLI output:

1. Identify which schemas to validate:
   - `schemas/graft.schema.json` - Main graft.yaml configuration schema
   - `schemas/policy.schema.json` - Policy configuration schema
   - `schemas/cli/explain.schema.json` - Explain command output schema
   - Other CLI command schemas as they're implemented

2. For each schema:
   - Read the schema file
   - Run the corresponding CLI command with `--json` to get actual output
   - Compare the actual output structure with the schema
   - Identify discrepancies:
     - Missing fields in schema
     - Extra fields in schema that aren't in output
     - Type mismatches
     - Required vs optional field differences

3. Validate consistency:
   - Check that service layer result objects match schemas
   - Verify `.to_dict()` methods produce schema-compliant output
   - Ensure CLI contract in `docs/cli-spec.md` aligns with schemas

4. Suggest updates:
   - Propose schema changes to match implementation
   - OR propose code changes if schema is correct but implementation differs
   - Consider which approach maintains contract stability

5. Report findings and recommendations

Remember: Schemas are the contract for external tools consuming Graft's JSON output.
