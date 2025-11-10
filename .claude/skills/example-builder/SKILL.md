# Skill: Example Builder

## Purpose
Create and maintain example Graft artifacts that demonstrate features, serve as test fixtures, and provide templates for users.

## When to Use
Activate when the user mentions:
- Creating examples
- Building test fixtures
- Example artifacts
- Demo artifacts
- Sample graft.yaml
- Creating templates

## Example Artifact Structure

A complete example artifact includes:

```
examples/my-example/
├── graft.yaml              # Configuration (required)
├── template.md             # Template file (if using templates)
├── input-data.csv          # Input materials (if needed)
├── README.md               # Documentation (recommended)
└── expected-output.md      # Expected result (for testing)
```

## Creating a New Example

### 1. Determine Purpose

Examples serve different purposes:

**Demonstration Examples** (`examples/demos/`):
- Show features to users
- Document usage patterns
- Marketing/tutorial content

**Test Fixtures** (`examples/` or `tests/fixtures/`):
- Used by test suite
- Must be stable and comprehensive
- Cover edge cases

**Templates** (`examples/templates/`):
- Starting points for users
- Minimal, customizable
- Well-documented

### 2. Create Directory Structure

```bash
mkdir -p examples/my-example
cd examples/my-example
```

### 3. Write graft.yaml

Follow the schema in `schemas/graft.schema.json`:

```yaml
# Minimal example
graft: my-example

derivations:
  - id: main
    transformer: {ref: template-render}
    template:
      file: template.md
      engine: jinja2
    outputs:
      - path: output.md
```

```yaml
# Complete example with all features
graft: my-example
policy:
  deterministic: true
  network: off
  attest: required

inputs:
  materials:
    - path: ../shared-data/config.yaml
      rev: main
  parameters:
    project_name: "Example Project"
    version: "1.0.0"

derivations:
  - id: documentation
    transformer: {ref: jinja-render}
    template:
      file: template.md
      engine: jinja2
    outputs:
      - path: docs/output.md
    policy:
      deterministic: true
```

### 4. Validate graft.yaml

```bash
# Ensure it's valid YAML
python -c "import yaml; yaml.safe_load(open('graft.yaml'))"

# Test with explain command
python -m graft.cli explain . --json
```

Should succeed without errors.

### 5. Create Template Files

If using templates:

```markdown
# template.md
# {{ project_name }}

This is an example artifact demonstrating Graft features.

Version: {{ version }}
Generated: {{ date }}
```

### 6. Add Input Materials

If artifact uses materials:
- Add actual files referenced in graft.yaml
- Ensure paths are correct (relative to artifact directory)
- Keep materials minimal and focused

### 7. Document with README

```markdown
# My Example Artifact

## Purpose
Brief description of what this example demonstrates.

## Structure
- `graft.yaml` — Configuration
- `template.md` — Template file
- `input-data.csv` — Sample input

## Usage
\`\`\`bash
graft explain examples/my-example/
graft run examples/my-example/
\`\`\`

## Expected Output
Running `graft run` should produce `output.md` with...

## Notes
Any special considerations or variations.
```

### 8. Test the Example

```bash
# From project root
python -m graft.cli explain examples/my-example/ --json
python -m graft.cli run examples/my-example/

# Verify output exists and is correct
cat examples/my-example/output.md
```

### 9. Use in Tests (if applicable)

If creating test fixture:

```python
# tests/test_something.py
def test_my_feature(tmp_path):
    # Copy example to temp directory
    src = Path("examples/my-example")
    dst = tmp_path / "artifact"
    shutil.copytree(src, dst)

    # Run test against example
    result = run_graft("run", str(dst))
    assert result.returncode == 0
```

## Maintaining Examples

### When to Update

Update examples when:
- Schema changes (new fields in graft.yaml)
- CLI contract changes
- New features added that should be demonstrated
- Bug found in existing example
- Test expectations change

### Update Process

1. Identify affected examples
2. Modify graft.yaml or related files
3. Run `graft explain` to verify still valid
4. Run `graft run` to verify executes correctly
5. Update tests if expectations changed
6. Update README if usage changed

### Validation Checklist

For each example:
- [ ] `graft.yaml` is valid YAML
- [ ] Follows current schema (`schemas/graft.schema.json`)
- [ ] `graft explain` succeeds
- [ ] `graft run` succeeds (if applicable)
- [ ] All referenced files exist
- [ ] Paths are relative and correct
- [ ] README is up to date
- [ ] Tests using example pass

## Example Categories

### 1. Minimal Example
Bare minimum to demonstrate basic functionality:

```yaml
graft: minimal
derivations:
  - id: simple
    transformer: {ref: copy}
    outputs:
      - path: output.txt
```

Use for: First-time users, basic tests

### 2. Template Example
Demonstrates template rendering:

```yaml
graft: template-demo
derivations:
  - id: render
    transformer: {ref: jinja}
    template:
      file: template.md
      engine: jinja2
    outputs:
      - path: rendered.md
```

Use for: Template feature demonstration

### 3. Multi-Derivation Example
Shows multiple derivations in one artifact:

```yaml
graft: multi-step
derivations:
  - id: step1
    transformer: {ref: process}
    outputs:
      - path: intermediate.json

  - id: step2
    transformer: {ref: transform}
    outputs:
      - path: final.md
```

Use for: Pipeline demonstrations

### 4. Policy Example
Demonstrates policy configuration:

```yaml
graft: policy-demo
policy:
  deterministic: true
  network: off
  attest: required

derivations:
  - id: controlled
    transformer: {ref: generate}
    outputs:
      - path: output.md
    policy:
      attest: required
```

Use for: Governance features

### 5. Materials Example
Shows input material dependencies:

```yaml
graft: with-materials
inputs:
  materials:
    - path: ../shared/config.yaml
      rev: main
    - path: data/source.csv
      rev: v1.0

derivations:
  - id: process
    transformer: {ref: merge}
    outputs:
      - path: merged.yaml
```

Use for: Dependency tracking

## Common Patterns

### Agile-Ops Example (Current)
```
examples/agile-ops/
└── artifacts/
    └── sprint-brief/
        ├── graft.yaml
        ├── template.md
        └── brief.md (output)
```

This demonstrates:
- Artifact organization
- Template rendering
- Real-world use case

### Future Examples to Consider

**Data Pipeline**:
```
examples/data-pipeline/
├── ingest/
│   └── graft.yaml (pulls from snapshot)
├── transform/
│   └── graft.yaml (processes data)
└── export/
    └── graft.yaml (generates report)
```

**Documentation Generation**:
```
examples/doc-gen/
└── api-docs/
    ├── graft.yaml
    ├── api-spec.yaml (material)
    └── docs-template.md
```

**Configuration Management**:
```
examples/config-management/
└── app-config/
    ├── graft.yaml
    ├── base-config.yaml
    └── environment-overrides.yaml
```

## Quality Standards

### All Examples Must:
- Have valid, complete graft.yaml
- Include README with purpose and usage
- Work with current CLI implementation
- Follow project conventions
- Be minimal (no unnecessary complexity)
- Have stable paths and structure

### Test Fixtures Must Also:
- Be used by at least one test
- Cover specific test scenario clearly
- Have predictable outputs
- Remain stable (changes break tests)

### Demo Examples Must Also:
- Be self-documenting
- Show realistic use cases
- Be easy to understand
- Have clear expected outputs

## Troubleshooting

### Example fails validation
```bash
# Check YAML syntax
python -c "import yaml; print(yaml.safe_load(open('graft.yaml')))"

# Validate against schema
# (Future: graft validate command)

# Test explain
graft explain . --json
```

### Example works locally but fails in tests
- Check paths are relative, not absolute
- Verify no dependencies on local environment
- Ensure all materials are included
- Check tmp_path usage in test

### Example outdated after changes
1. Review what changed in implementation/schema
2. Update graft.yaml to match new schema
3. Update templates/materials if needed
4. Re-run tests to verify
5. Update README

## Outputs
- Complete, working example artifacts
- Clear documentation
- Stable test fixtures
- Demonstrated features
- Updated when schema evolves
