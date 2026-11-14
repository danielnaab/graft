---
status: working
last-verified: 2026-11-06
owners: [daniel]
---

# KB Linter

## Intent

Enforce the content rules declared in [`knowledge-base.yaml`](../knowledge-base.yaml) — validating that content files have proper frontmatter status fields and provenance (a Sources section or `sources` frontmatter) where required, and that an OKF bundle the KB declares conforms to OKF v0.2.

This gives Principle #2 ("source-of-truth requires enforcement") its mechanical backing: rules declared in configuration are checked by tooling, not just convention.

## Non-goals

- Prose quality or completeness checking
- Write boundary enforcement (agent entrypoint handles this)
- Markdown link validation (separate concern, handled by [link-validator](link-validator.md))
- Spec-to-implementation traceability (handled by [backlink scanner](backlink-scanner.md))
- Semantic analysis of whether content qualifies as "practice-recommendations" vs. other types

## Behavior

### Configuration

```gherkin
Given a knowledge-base.yaml file in the root directory
When the linter starts
Then it reads rules.lifecycle.statuses for valid status values
  And it reads sources.canonical paths for provenance-required directories
```

```gherkin
Given a knowledge-base.yaml with rules.lifecycle.statuses written as a YAML list
  (inline, quoted or bare: [draft, working] or ["draft", "working"], or a block list)
When the linter starts
Then it reads every status in the list
  And a statuses value that is not a list of strings exits 2 (a configuration error), never an empty set
```

```gherkin
Given a knowledge-base.yaml with rules.checks.lifecycle, rules.checks.provenance or extraStatuses
When the linter starts
Then it reads them as described in [tool-cli](tool-cli.md#path-scopes)
  And a value of the wrong shape exits 2
```

```gherkin
Given a knowledge-base.yaml where sources has sibling blocks after canonical
  (e.g. interpretation: and tracking: at the same indentation as canonical:)
When the linter reads sources.canonical paths
Then it reads only the "- path:" entries inside the canonical block
  And the block ends at the next key at the same or lesser indentation as canonical:
  And nested lines such as note: are skipped
```

### Frontmatter validation

```gherkin
Given a markdown file in a content directory (docs/)
When the file has no YAML frontmatter
Then it reports a "missing frontmatter" violation
```

```gherkin
Given a markdown file with frontmatter
When the frontmatter has no status field
Then it reports a "missing status" violation
```

```gherkin
Given a frontmatter status field
When the value is not in rules.lifecycle.statuses
Then it reports an "invalid status" violation with the invalid value and allowed values
```

```gherkin
Given a frontmatter with a valid status field
When the status value is in the allowed set
Then no violation is reported for that file's frontmatter
```

```gherkin
Given a decision record: a file under a decisions/ directory (docs/decisions/0003-x.md)
  or a file named adr-*.md or ADR-*.md anywhere
When its status is proposed, accepted, rejected, superseded, or deprecated
Then no invalid-status violation is reported
  And a status in rules.lifecycle.statuses is also accepted
  And a missing status is still reported as "missing status"
```

```gherkin
Given a frontmatter status of living
When the file is checked
Then no invalid-status violation is reported, whatever rules.lifecycle.statuses lists
  (living marks an always-current document; the lifecycle policy allows it in every KB)
  And statuses in rules.checks.lifecycle.extraStatuses are accepted the same way
```

```gherkin
Given a file that is not a decision record (docs/guide.md, docs/decisions.md)
When its status is a decision status not in rules.lifecycle.statuses (accepted)
Then it reports an "invalid status" violation
```

### Provenance validation

```gherkin
Given a file in the provenance scope
When the file has neither a ## Sources heading nor sources frontmatter
Then it reports a "missing provenance" violation
```

```gherkin
Given a file whose frontmatter has a sources list in which every entry is a mapping with a non-empty resource
When the file is checked for provenance
Then the list counts as provenance, the same as a ## Sources section
  (structured provenance: meta-knowledge-base v1.1.0 provenance policy, OKF v0.2 §5.1)
```

```gherkin
Given any checked file whose frontmatter has a sources key
When sources is not a non-empty list, or an entry is not a mapping or has no resource
Then it reports an "invalid-sources" violation naming the first bad entry
  And no "missing provenance" violation is added for the same file
```

```gherkin
Given a heading "## Sources" optionally followed by a space and more words ("## Sources and verification")
When the file is checked for provenance
Then the heading is accepted
  And "## Sourcesx" and "### Sources" are not
```

```gherkin
Given no rules.checks.provenance.include
When the provenance scope is computed
Then it is the files in the lifecycle scope that match sources.canonical (as before)
  And canonical paths outside the lifecycle scope (AGENTS.md, flake.nix) are not scanned
```

```gherkin
Given rules.checks.provenance.include in knowledge-base.yaml
When the provenance scope is computed
Then it is the .md files matching those globs, minus rules.checks.provenance.exclude
  And sources.canonical no longer decides which files need Sources
  And those files are scanned even when outside the lifecycle scope, but their status is not checked
```

```gherkin
Given a file in a canonical path with a ## Sources heading
When the section exists (regardless of content)
Then no provenance violation is reported
```

```gherkin
Given a file in a non-canonical path (notes/)
When the file lacks a Sources section
Then no provenance violation is reported (provenance is optional for these paths)
```

```gherkin
Given a README.md in a path matching sources.canonical entries (docs/decisions/README.md)
When the file lacks a Sources section
Then no provenance violation is reported (a README indexes canonical docs; it is not one)
  And its frontmatter status is still checked
  And other files in the same canonical path still need a Sources section
```

### OKF conformance

Runs only when `rules.checks.okf.bundle` declares a bundle (see [tool-cli](tool-cli.md#okf-bundle)). Every note in the bundle is checked, whatever the lifecycle and provenance scopes say.

```gherkin
Given a note in the bundle that is not index.md or log.md
When it has no frontmatter block
Then it reports "okf-missing-frontmatter"
When its frontmatter has no type, or an empty or non-string type
Then it reports "okf-missing-type"
```

```gherkin
Given an index.md in the bundle
When it has frontmatter
Then it reports "okf-index-frontmatter", unless it is the bundle-root index.md and okf_version is its only key
When a body line outside code fences is not a heading, an entry (* [Title](url) or * [Title](url) - description,
  with * or -), or a line that is one HTML comment
Then it reports "okf-index-entry" with the line number
```

```gherkin
Given a log.md in the bundle
When a level-2 heading is not an ISO date (## YYYY-MM-DD)
Then it reports "okf-log-date"
```

```gherkin
Given rules.checks.okf.version
When the bundle-root index.md is missing or its okf_version differs
Then it reports "okf-version" on that index.md
```

```gherkin
Given index.md or log.md in the bundle, inside the lifecycle or provenance scope
When the file is checked
Then status and Sources are not required (OKF forbids frontmatter there; they list, they do not claim)
```

### Scanned paths

- Lifecycle scope (status checked): `rules.checks.lifecycle.include` minus `.exclude`; default `docs/`, `policies/`, `playbooks/`
- Files scanned: the lifecycle scope, plus the provenance scope when `rules.checks.provenance.include` is set, plus the bundle's notes when `rules.checks.okf.bundle` is set
- Only `.md` files are checked
- README.md files are included (they carry status; they are exempt from provenance)
- Files in subdirectories are included recursively
- Skips: .git/, .graft/, .venv/, node_modules/, __pycache__/
- Skips directories graft wrote (see [tool-cli](tool-cli.md#graft-directories))
- By default does not scan: specs/, src/, tests/, notes/ (specs have their own lifecycle; code and tests don't need frontmatter; notes are ephemeral). A KB with a different layout declares its scopes instead (see [tool-cli](tool-cli.md#path-scopes))

### Output structure

```gherkin
Given completed linting
When results are reported
Then output is a JSON object with a flat violations array and a summary object
```

Rules: `missing-frontmatter`, `missing-status`, `invalid-status`, `missing-provenance`, `invalid-sources`, and with a bundle `okf-missing-frontmatter`, `okf-missing-type`, `okf-index-frontmatter`, `okf-index-entry`, `okf-log-date`, `okf-version`.

Example output:
```json
{
  "violations": [
    {
      "file": "docs/example.md",
      "rule": "missing-status",
      "message": "No status field in frontmatter"
    },
    {
      "file": "docs/living-specifications/example.md",
      "rule": "missing-provenance",
      "message": "No Sources section found (required for canonical content)"
    }
  ],
  "summary": {
    "files_checked": 12,
    "files_passing": 10,
    "violations": 2
  }
}
```

### Exit codes

- **Exit 0**: no violations found
- **Exit 1**: one or more violations found
- **`--report-only` flag**: always exit 0

### Edge cases

- Files with empty frontmatter (`---\n---`): reports "missing status"
- Files with malformed frontmatter: reports "missing status" (regex extraction, not full YAML parsing)
- Binary files: skipped (UnicodeDecodeError caught)
- File symlinks: resolved normally; directory symlinks: not followed (os.walk default)
- knowledge-base.yaml missing: exit with error message (not a violation, a misconfiguration)
- A configured value of the wrong shape (statuses not a list, include not a list of strings): exit 2 naming the key

## Constraints

- Runs in Python (no runtime dependencies beyond standard library)
- Reads configuration from `knowledge-base.yaml` (not hardcoded) through [tool-cli](tool-cli.md#configuration)
- Completes in under 1 second for knowledge bases up to 1,000 content files

## Open Questions

- [ ] Should notes/ files be checked for frontmatter status? (Currently excluded as ephemeral, but they do use status fields)
- [ ] Should the linter support a `# kb-lint: ignore` annotation for intentional exceptions?

## Decisions

- 2026-01-24: Read rules from knowledge-base.yaml rather than hardcoding. Keeps the tool adaptable to different KBs and avoids drift between declared rules and enforcement.
- 2026-01-24: Provenance checking uses `sources.canonical` paths to determine which files need Sources sections. This is more precise than checking all files (notes are ephemeral).
- 2026-01-24: Notes excluded from linting. They're ephemeral explorations — enforcing structure on them contradicts their purpose.
- 2026-10-30: End the sources.canonical block at the next sibling key, not only at a top-level key. Paths in sibling blocks such as interpretation: were read as canonical, so the docs listed there got false missing-provenance findings (daniel/digital-strategy: docs/README.md, docs/agents.md). Parsing stays targeted extraction, not a general YAML parser.
- 2026-10-30: Decision records keep decision statuses. An ADR's status records the decision (proposed, accepted, rejected, superseded, deprecated), a separate axis from document lifecycle, so decision records may use those values as well as rules.lifecycle.statuses. Relabelling an accepted ADR as "stable" to satisfy the linter loses that information; Daniel rejected doing so. Evidence: graft's 9 accepted ADRs (docs/decisions/ and a grove adr-001 under docs/specifications/grove/) were flagged invalid-status. A decision record is a file under a decisions/ directory or named adr-*.md; a status is still required.
- 2026-10-30: README.md files are exempt from provenance even when a sources.canonical glob (docs/decisions/**) covers them. A README indexes the canonical docs beside it; it is not a source of truth, and a Sources section on an index has nothing to cite. Status is still checked.

- 2026-11-05: Skip directories graft wrote, found by the marker in their README.md (`tool_cli.graft.is_graft_dir`), wherever they sit: `scions/`, a hidden `.scions/`, or `docs/Grafted/`. A grafted directory is another repo's export, checked by that repo, and graft's `nix flake check` fails if it is edited here, so a finding inside one can't be acted on. Before this, only steward's check-kb-integrity duty filtered them, so a direct run reported upstream content as this repo's.
- 2026-11-05: Scan roots, status scope and provenance scope are configurable (`rules.checks`), with the old directories as defaults. A notebook-style KB (content under `notebook/`, root README/AGENTS) had 6 of ~75 files checked. When `rules.checks.provenance.include` is set it replaces `sources.canonical` as the provenance scope: canonical paths name sources of truth (config files), which are not necessarily documents that cite sources.
- 2026-11-05: Statuses are read with the shared subset parser. The old regex accepted only JSON-style lists, so `statuses: [draft, working]` produced an empty set and the status check silently never ran. An unreadable statuses value now exits 2.
- 2026-11-05: `living` is always an allowed status. The meta-knowledge-base temporal-layers policy prescribes `status: living` for always-current documents, while its lifecycle policy lists only draft/working/stable/deprecated, so a KB following both was flagged. The tool follows the policy that names the value instead of making each KB list it. Other extra statuses go in `rules.checks.lifecycle.extraStatuses`.
- 2026-11-05: The Sources heading matches `## Sources` with optional trailing words. `## Sources and verification` is a real Sources section; the exact-match rule failed it. A different heading level or a longer word (`## Sourcesx`) still fails.
- 2026-11-11: The lifecycle policy now states that `living` is always valid, so the check above implements a
  rule the policy owns instead of settling a conflict between two policies. Checker and conventions are one repo
  (knowledge-base-kit), so a rule like this changes in the policy and the code together.
- 2026-11-06: `sources` frontmatter counts as provenance in every KB, not only OKF bundles. The kit's provenance policy (v1.1.0) makes it equivalent to a Sources section. A present but malformed list is reported as invalid rather than missing, so the finding points at the entry to fix.
- 2026-11-06: OKF conformance is checked here, not by OKF Wiki's `okf` CLI: this tool already gates every workspace, honours `.okfignore`, and keeps one finding format. The checks are OKF v0.2 §11: frontmatter with a non-empty `type` on every note, and the §8 and §9 shapes for `index.md` and `log.md`. Optional families (`generated`, `verified`, `stale_after`) are not validated, since OKF forbids rejecting a note for them.
- 2026-11-06: An index body is headings and entries only. Workspace indexes are generated, so prose in one means a hand edit the generator will drop; HTML comment lines are allowed for a generator's marker.

## Sources

- Living Specifications Principles — #2 (enforcement), #14 (fight tribal knowledge)
- Living Specifications Format — "Status is non-negotiable"
- [OKF v0.2 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) — §4.1 frontmatter, §5.1 sources, §8 index files, §9 log files, §11 conformance
- [Provenance policy](../docs/policies/provenance.md) — structured provenance (v1.1.0)
- [Backlink Scanner](backlink-scanner.md) — sibling tool, structural patterns to follow
