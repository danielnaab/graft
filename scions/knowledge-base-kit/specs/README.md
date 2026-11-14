# Specifications

Living specifications for the knowledge-base-kit's checker. Each spec is a behavioral contract that code in [`src/`](../src/) implements.

## Specs

| Spec | Status | Purpose |
|------|--------|---------|
| [backlink-scanner.md](backlink-scanner.md) | working | Scan for `spec:` annotations, report traceability |
| [kb-linter.md](kb-linter.md) | working | Validate content files against knowledge-base.yaml rules |
| [link-validator.md](link-validator.md) | working | Detect broken internal markdown links |
| [tool-cli.md](tool-cli.md) | working | Shared CLI runner conventions (args, JSON output, exit codes) |

## Lifecycle

Specs use the same status vocabulary as all KB content:

- **draft** — Design intent captured, implementation may not be complete
- **working** — Implemented and tested, expected to evolve
- **stable** — Behavior is settled, changes only via explicit decision
- **deprecated** — Superseded or no longer relevant

## Backlink Verification

Run the scanner to check spec-implementation traceability:

```bash
uv run backlink-scanner        # Exit 1 if dangling refs or orphan specs
uv run backlink-scanner --report-only  # Always exit 0 (informational)
```
