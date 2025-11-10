# Skill: Schema Validator

## Purpose
Validate and synchronize JSON schemas with CLI implementation to ensure contract compliance.

## When to Use
Activate when the user mentions:
- Schema validation
- Checking schemas
- JSON schema
- Contract validation
- Output format verification
- Schema updates needed

## Process

### 1. Identify Schemas to Validate
Main schemas in Graft:
- `schemas/graft.schema.json` — Configuration file schema
- `schemas/policy.schema.json` — Policy configuration schema
- `schemas/cli/explain.schema.json` — Explain command output
- Future CLI schemas as commands are implemented

### 2. Test Actual Output
For each CLI command with a schema:
```bash
python -m graft.cli <command> <args> --json > actual_output.json
```

### 3. Compare Schema vs Reality
For each field in the schema:
- **Required fields**: Verify they appear in actual output
- **Optional fields**: Check if presence matches implementation
- **Types**: Ensure actual types match schema types
- **Nested objects**: Recursively validate structure

For each field in actual output:
- **Is it in schema?**: If not, schema is incomplete
- **Type matches?**: Verify string/number/boolean/object/array
- **Description present?**: Schema should document field purpose

### 4. Identify Discrepancies
Common mismatches:
- Schema has field marked required, but output sometimes omits it
- Output includes field not in schema
- Type mismatch (schema says string, output is number)
- Optional marker wrong (field is always present but marked optional)
- Schema more restrictive than implementation (enum vs free string)

### 5. Determine Correct Fix
Two possible approaches:

**Update Schema** (if implementation is correct):
- Add missing fields
- Fix type definitions
- Adjust required/optional markers
- Add descriptions

**Update Implementation** (if schema is the contract):
- Modify service `.to_dict()` to match schema
- Add missing fields with proper values
- Fix type conversions

### 6. Validate Fix
- Compare updated schema/implementation
- Re-run CLI command and verify output
- Check that downstream tools would work correctly
- Consider breaking changes if modifying existing contracts

## Schema Best Practices

### Complete Schemas
Every field should have:
- `type` specification
- `description` explaining purpose
- Examples in description if helpful
- Required vs optional correctly set

### Schema Evolution
When updating schemas:
- Document breaking vs non-breaking changes
- Consider backward compatibility
- Update version in schema if needed
- Note changes in ADR if significant

### Testing Schemas
Use jsonschema library to validate:
```python
import jsonschema
import json

with open("schemas/cli/explain.schema.json") as f:
    schema = json.load(f)

with open("actual_output.json") as f:
    output = json.load(f)

jsonschema.validate(output, schema)
```

## Common Patterns

### Service Result to Schema
```python
# Service layer
@dataclass
class ExplainResult:
    artifact: str
    graft: str
    derivations: list

    def to_dict(self) -> dict:
        return {
            "artifact": self.artifact,
            "graft": self.graft,
            "derivations": [d.to_dict() for d in self.derivations]
        }
```

### Corresponding Schema
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["artifact", "graft", "derivations"],
  "properties": {
    "artifact": {
      "type": "string",
      "description": "Path to the artifact directory"
    },
    "graft": {
      "type": "string",
      "description": "Name of the graft from graft.yaml"
    },
    "derivations": {
      "type": "array",
      "description": "List of derivation specifications",
      "items": { "type": "object" }
    }
  }
}
```

## Quality Checklist
- [ ] All CLI commands with --json have matching schemas
- [ ] All required fields in schema appear in actual output
- [ ] No fields in output missing from schema
- [ ] Types match between schema and implementation
- [ ] Descriptions are clear and helpful
- [ ] Optional vs required markers are correct

## Outputs
- Validated schemas matching implementation
- Documentation of schema changes
- Updated schemas if needed
- Work log entry noting validation results
