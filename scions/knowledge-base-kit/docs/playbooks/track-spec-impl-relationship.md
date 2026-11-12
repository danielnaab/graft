---
status: stable
---
# Tracking specification-to-implementation relationship

Guidance for maintaining clear tracking between specification KBs and implementation KBs.

## When to use this pattern

Use this when you have:
- **Specification KB**: Defines requirements, formats, behavior (WHAT)
- **Implementation KB**: Tracks development, status, code (HOW)

Common in: API specifications, file format definitions, protocol specs, language standards.

## Pattern: Three-part tracking

### 1. In the specification KB

**Add CHANGELOG.md** at repository root:

```markdown
# Changelog

All notable changes to specifications documented here.

Format: [Keep a Changelog](https://keepachangelog.com/)

## [2026-01-05] - Validation Enhancements

### Added
- Validation operations specification
- Error handling requirements

### Changed
- Simplified API version field

## [2026-01-01] - Initial Specifications

### Added
- Core format specification
- Required operations
```

**Keep specifications authoritative and stable**:
- No implementation status in spec files
- No temporal markers like "just added"
- Changes tracked in CHANGELOG only
- Use git commits/tags for versions

### 2. In the implementation KB

**Pin to specification version** (choose one approach):

**Option A: Config file** (recommended for larger projects)
```yaml
# spec-version.yaml
specification:
  repository: https://git.example.com/project/specs.git
  commit: 881fdd5a1b2c3d4e5f6g7h8i9j0k
  date: 2026-01-05

implemented:
  - core-format: complete
  - validation: partial
  - advanced-features: planned
```

**Option B: Metadata in build config**
```toml
# pyproject.toml, package.json, Cargo.toml, etc.
[project.specification]
repository = "https://git.example.com/project/specs.git"
commit = "881fdd5a1b2c3d4e5f6g7h8i9j0k"
date = "2026-01-05"
```

**Option C: Simple text file**
```
# SPEC-VERSION.txt
specs@881fdd5 (2026-01-05)
Implements: core format, validation operations
```

**Track implementation status separately**:

```markdown
# IMPLEMENTATION-STATUS.md

Specification version: specs@881fdd5 (2026-01-05)

## Implemented
- [x] Core format v2.0
- [x] Basic validation
- [ ] Integrity checks (in progress)

## Planned
- [ ] Advanced features
- [ ] Performance optimizations

## Not Planned
- [ ] Legacy format support (per Decision 0003)
```

### 3. Reference format

**Standard format**: `repository@commit`

Examples:
- `specs@881fdd5` (short)
- `myproject-specs@881fdd5a1b2c3d4e5f6g7h8i9j0k` (full)

**In code comments**:
```python
def validate_format(data):
    """
    Validate format per specification.

    Spec: specs@881fdd5 format.md#validation-rules
    """
```

**In task tracking**:
```markdown
- [ ] Implement validation
      Spec: format.md#validation-rules
      Ref: specs@881fdd5
```

**In work logs**:
```markdown
# Work Log: Validation Implementation

Implements specification from specs CHANGELOG 2026-01-05.
See: specs/docs/format.md#validation-rules
```

## Sync workflow

**When specification changes**:

1. Specification maintainer updates CHANGELOG.md
2. Commit and optionally tag: `git tag 2026-01-05`
3. Push to origin

**When implementation needs to sync**:

1. Check specification CHANGELOG for new entries
2. Update pinned version in implementation KB
3. Update IMPLEMENTATION-STATUS.md with new items
4. Create tasks for unimplemented features

**Discovering changes**:
```bash
# View specification changes since your pinned version
cd specs
git log YOUR_PINNED_COMMIT..main -- docs/
```

## Anti-patterns

Avoid:
- ❌ Implementation status in specification files
- ❌ Temporal markers in specifications ("just updated!")
- ❌ No clear version pinning in implementation
- ❌ Silent divergence (implementation drifts without tracking)
- ❌ Copy-paste specifications into implementation

Instead:
- ✅ Specifications are stable and authoritative
- ✅ CHANGELOG for discoverability
- ✅ Clear pinning via git commit
- ✅ Track unimplemented features explicitly
- ✅ Reference specifications, don't duplicate

## Integration with meta-kb patterns

Declare the relationship in knowledge-base.yaml:

```yaml
# In implementation KB
imports:
  - kind: git
    repo: https://git.example.com/project/specs.git
    relationship: "canonical-source"
    sync_policy: "interpret"

sources:
  canonical:
    - path: "../specs/docs/**"
      note: "Specifications are authoritative"
  interpretation:
    - path: "IMPLEMENTATION-STATUS.md"
      note: "Tracks what's implemented"
```

## Related

- [KB dependencies](manage-kb-dependencies.md) - High-level dependency patterns
- [Authority policy](../policies/authority.md) - What counts as canonical
- [Provenance policy](../policies/provenance.md) - Grounding claims in sources
