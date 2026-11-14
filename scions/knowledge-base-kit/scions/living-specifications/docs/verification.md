---
status: working
---

# Verification and Drift Detection

How to keep specifications honest—detecting and resolving divergence between specs and implementation.

## Detection Approaches

1. **PR-time analysis**: AI reads spec + code diff, flags inconsistencies. Non-blocking warning. Cheapest moment to fix.
2. **Periodic audit**: Scheduled scan producing drift reports. Noisier but catches gradual drift.
3. **Test-spec linkage**: When tests change without spec changes, flag potential drift.
4. **Structured assertion checking**: Behavior statements precise enough for mechanical verification.

## Backlinks Enable Automation

Implementation annotates which spec it implements:

```python
# spec: docs/specs/auth.md
# spec-section: Behavior/Login
def login(credentials):
    ...
```

Tools can then:
1. Find all files referencing a spec
2. Compare spec intent with implementation
3. Flag divergence

Without backlinks, detection requires guessing which code relates to which spec.

## Annotation Format

The standard annotation format for spec references:

```
# spec: <path-to-spec>
# spec-section: <section-name>
```

Languages use their comment syntax:
- Python: `# spec: ...`
- JavaScript/TypeScript: `// spec: ...`
- Go: `// spec: ...`
- Ruby: `# spec: ...`

Place annotations at:
- Function/method level for behavioral code
- Module level for entire files implementing a spec
- Test file level to connect tests to specs

## Design Principles

- **High confidence first** — false positives kill trust in tooling
- **Detection over prevention** — suggestions, not blockers
- **Human decision authority** — AI flags, humans decide; no auto-correction
- **Targeted analysis** — only analyze files referencing specs, only when changed

## Related

- [Principles](principles.md) — #13 (references flow upstream)
- [Artifact relationships](relationships.md) — backlink patterns

## Sources

- Personal experience: CI-based linting, spec-code drift in production systems
