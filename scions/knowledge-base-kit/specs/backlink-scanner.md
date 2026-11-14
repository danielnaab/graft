---
status: working
last-verified: 2026-01-24
owners: [daniel]
---

# Backlink Scanner

## Intent

Enable spec-to-implementation traceability by scanning source files for backlink annotations (`// spec:` or `# spec:`) and producing a structured report of which files reference which specs.

This supports the living specifications practice: references flow upstream (code points to specs), and views are computed from those references.

## Non-goals

- Semantic analysis of whether implementations match spec intent
- Modifying source files or specs
- Watching for file changes (runs on demand)

## Behavior

### Scanning for backlinks

```gherkin
Given a directory path
When the scanner runs
Then it finds all files containing backlink annotations matching // spec: or # spec: followed by a path
```

### Annotation format

Annotations must be standalone lines (the annotation is the entire line content, ignoring leading/trailing whitespace).

```gherkin
Given a line // spec: specs/backlink-scanner.md
When the scanner parses it
Then it records that file as an implementor of specs/backlink-scanner.md
```

```gherkin
Given a line # spec: specs/backlink-scanner.md
When the scanner parses it
Then it records that file as an implementor of specs/backlink-scanner.md
```

```gherkin
Given a line containing an annotation embedded in other content (e.g. inside a string literal)
When the scanner parses it
Then it does not match
```

### Section annotations

```gherkin
Given a line // spec-section: Behavior/Scanning for backlinks
  And a spec: annotation has previously appeared in the same file
When the scanner parses it
Then it records the section reference against the most recent spec: path
```

```gherkin
Given a spec-section: annotation with no preceding spec: in the same file
When the scanner parses it
Then it is ignored
```

### Output structure

```gherkin
Given completed scanning
When results are reported
Then output is a JSON object with specs, dangling, and orphans keys
```

Example output:
```json
{
  "specs": {
    "specs/backlink-scanner.md": {
      "implementors": [
        "src/backlink_scanner/scanner.py",
        "tests/test_scanner.py"
      ],
      "sections": {
        "Behavior/Scanning for backlinks": ["src/backlink_scanner/scanner.py"]
      }
    }
  },
  "dangling": [],
  "orphans": []
}
```

### Exit codes

- **Exit 0**: no dangling references AND no orphan specs
- **Exit 1**: dangling references OR orphan specs found
- **`--report-only` flag**: always exit 0 (for informational use without failing)

### Dangling references

```gherkin
Given a backlink annotation referencing specs/nonexistent.md
When the referenced spec file does not exist
Then the scanner includes it in a dangling array in the output
```

### Orphan specs

```gherkin
Given spec files exist in the specs directory
When no source files reference a given spec
Then the scanner includes that spec path in an orphans array in the output
```

### Edge cases

- Binary files: skipped
- Markdown code fences: annotations inside ``` blocks are skipped
- Standalone lines only: annotations embedded in other content (string literals, inline code) are ignored
- Multiple annotations in one file: all recorded
- Self-referencing specs: allowed (a spec can reference another spec)
- README.md files in specs/: excluded from orphan detection (navigational, not behavioral)
- Directories graft wrote: not scanned (see [tool-cli](tool-cli.md#graft-directories)); a provider's own `spec:` annotations would otherwise read as this repo's dangling references

## Constraints

- Runs in Python (no runtime dependencies beyond standard library)
- Completes in under 1 second for repositories up to 10,000 files
- Exit code 0 when clean; exit code 1 when dangling refs or orphans found
- `--report-only` flag overrides to always exit 0

## Open Questions

- [ ] Should the scanner support configurable annotation patterns beyond `// spec:` and `# spec:`?
- [ ] Should output include line numbers for each annotation?

## Decisions

- 2026-01-24: Use JSON output for machine readability. Human-readable summaries can be built on top.
- 2026-01-24: No external dependencies. Keeps the tool simple and the repo self-contained.
- 2026-01-24: Fail by default on dangling references or orphan specs. These are broken links and dead weight respectively; failing early catches both. `--report-only` restores informational mode.
- 2026-11-05: Skip directories graft wrote, found by the marker in their README.md (`tool_cli.graft.is_graft_dir`), wherever they sit: `scions/`, a hidden `.scions/`, or `docs/Grafted/`. A grafted directory is another repo's export, checked by that repo, and graft's `nix flake check` fails if it is edited here, so a finding inside one can't be acted on. Before this, only steward's check-kb-integrity duty filtered them, so a direct run reported upstream content as this repo's.

## Sources

- Living Specifications: Verification and Drift Detection — backlink annotation patterns
- Living Specifications: Artifact Relationships — three-layer model, upstream references
- Personal experience: backlink-based navigation in knowledge bases
