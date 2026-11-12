# Changelog

## [Unreleased]

## v2.1.0 - 2026-11-12

Findings from rewriting graft's docs to these conventions. No checker change.

### Fixed
- **`docs/tools.md`:** run the checker from a scion with `python3 -B`. Without it Python writes `__pycache__/`
  into the scion, and graft's drift check fails (seen in graft, 2026-11-12).
- **Manage KB dependencies playbook, Extend policy:** upgrade with `nix run .#graft upgrade`. The old
  `nix flake update <name>` re-locked the same tag and never moved to a new one.

### Changed
- **Style policy:** brevity means saying each thing once, never cutting a reason or a limit; bullets for
  parallel items, prose for reasoning; rules for procedures (ASD-STE100: imperative numbered steps, one
  action each, conditions first) and for terms (one term per concept); the kit's document types mapped to
  Diátaxis, with lengths for each, reference and explanation included.
- **Linking policy:** link text says where it goes; backticks for any literal text the reader types or sees
  (commands, flags, keys, identifiers), not only paths; the rule for exported files moved here from the
  dependencies playbook, with links by URL at the pinned tag.
- **Lifecycle policy:** a log of decisions that still hold is `living`; a dated decision record is not.

## v2.0.0 - 2026-11-11

meta-knowledge-base (the conventions, v1.x) and knowledge-base-integrity (the checker, v0.x) are one repo,
**knowledge-base-kit**, so a rule and its enforcement change together. This continues meta-knowledge-base's
versions; integrity's history is merged in below, its tags not imported.

### Changed
- **Renamed** meta-knowledge-base → knowledge-base-kit. Old GitHub and Forgejo URLs redirect.
- **The export includes the checker** (`src/`, Python standard library only), beside README, CHANGELOG, `docs/`,
  `adapters/` and `examples/`. Specs, tests and build files are not exported.
- **Lifecycle policy:** `living` is always a valid status, whatever a KB declares. The checker already allowed it
  (integrity 0.4.0); now the policy states it.
- **Manage KB dependencies playbook:** examples use graft's `mkFlake` and the new name; two new rules: exported
  files link only to exported files, and dependencies are between exports (conformance and upkeep edges are not
  design dependencies).
- **`composition.precedence`:** `domain-over-imports` (was `domain-over-pack`, which named nothing).
- **`apiVersion`:** `kb/v1` is the only value (was `meta-kb/v1` in this repo and its README example).
- README and AGENTS.md cover both halves; `docs/tools.md` is the checker's usage (integrity's `docs/README.md`).

### Migration

In a repo that grafts meta-knowledge-base and/or takes knowledge-base-integrity as a tool input:

1. `flake.nix`: replace the `meta-knowledge-base` input with
   `knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.0.0"; flake = false; };`, and delete the
   `knowledge-base-integrity` input. Read the checker as `${inputs.knowledge-base-kit}/src` (was
   `${inputs.knowledge-base-integrity}/src`). Use one input: a second one for this repo (`git+https`, say) shares
   Nix's fetch cache with the first, and one of them gets the wrong tree.
2. Run `nix run .#graft`: `scions/meta-knowledge-base/` becomes `scions/knowledge-base-kit/` (or under `.scions/`).
3. `knowledge-base.yaml`: import path `scions/knowledge-base-kit`; `apiVersion: kb/v1`;
   `precedence: domain-over-imports`.
4. Links into `scions/meta-knowledge-base/…`: the same paths under `scions/knowledge-base-kit/…`.
5. `nix run github:danielnaab/knowledge-base-integrity#lint` has no successor (the flake isn't exported): run the
   tools through your workspace (notebook-kit's `just check-kb`) or from a clone (`nix run .#lint`).

## v1.1.0 - 2026-11-06

Additive: existing knowledge bases need no changes.

### Added
- **Provenance policy**: structured provenance. A `sources` frontmatter list, with footnotes keyed
  to its `id`s, counts the same as a `## Sources` section. It follows OKF v0.2 (§5.1).
- **Temporal layers policy**: two optional layers. Raw sources are kept as captured and never
  edited; historical records are dated, written once, and corrected with a dated correction.
  Neither carries a lifecycle `status`; the path marks them.

### Changed
- **Lifecycle policy**: raw sources and historical records carry no status.
- **Temporal layers policy**: the layers are one table; the content of the existing three is unchanged.
- **README**: the temporal layers summary names the two optional layers.

## v1.0.1 - 2026-11-03

### Changed
- **Manage KB dependencies playbook**: grafted copies live in `scions/` (graft 2.1's default),
  not `vendor/`.

## v1.0.0 - 2026-11-02

The first tagged release: what repos graft (https://github.com/danielnaab/graft/tree/v2).

### Changed
- **Exports**: `.gitattributes` defines what this repo exports: README, CHANGELOG, `docs/`,
  `adapters/` and `examples/`. `AGENTS.md`, `knowledge-base.yaml` and `notes/` are this repo's own.
- **Manage KB dependencies playbook**: take a dependency by grafting its export at a pinned
  commit into `vendor/`, link into the grafted copy, and provide one by declaring `export-ignore`
  rules and tagging releases.

### Removed
- `graft.yaml` here and in `examples/meal-planner/`: graft v1 is retired.

### Added
- **Intent-revealing structure policy** - File and directory naming that communicates purpose
  - Names reveal content without opening files
  - Hybrid names (number + intent) are acceptable
  - Addresses navigation pain at structural level
  - Policy condensed to 44 lines; detailed examples in exploration doc
  - Motivation and examples: `notes/2026-01-11-intent-revealing-structure.md` in this repo (not exported)
- **Track spec-impl relationship playbook** - Concrete patterns for specification-to-implementation tracking
  - CHANGELOG.md pattern for surfacing specification changes
  - Three pinning options (config file, build metadata, text file)
  - Reference format standard (`repo@commit`, `file.md#section`)
  - Sync workflow and anti-patterns

### Upgrading

For a repo that linked into this one before v1.0.0, through graft v1's `.graft/meta-knowledge-base/`
or GitHub `main`, as of the 2026-01-05 entry below. These moves happened in January 2026, before
the tag, and were not listed above.

1. Take it as a graft v2 scion pinned to a tag: in the repo's graft `flake.nix`,
   `inputs.meta-knowledge-base = { url = "github:danielnaab/meta-knowledge-base/v1.0.1"; flake = false; };`
   and `"meta-knowledge-base"` in `deps`; `nix run .#graft`; commit `flake.nix`, `flake.lock`
   and the grafted directory (`scions/` from graft 2.1, `vendor/` before). Remove the
   `meta-knowledge-base` entry from `graft.yaml` and its `.graft/meta-knowledge-base`.
2. Point links at the grafted copy, with these moves applied:
   - `policies/<name>.md` is `docs/policies/<name>.md` (all of them, `README.md` included).
   - `playbooks/<name>.md` is `docs/playbooks/<name>.md` (all of them, `README.md` included).
   - `docs/meta.md` is gone: its content is in `README.md`. `docs/README.md` is gone too; use
     `README.md`.
   - `meta.yaml` is `knowledge-base.yaml`, and it is not exported: drop imports of it.
   - `examples/meal-planner/docs/agents.md` is `examples/meal-planner/AGENTS.md`.
   - Removed with no replacement: `starter-kits/`, `examples/graft-project/`, `examples/minimal/`.
     The examples are now `api-catalog`, `meal-planner`, `research-notes` and `team-handbook`.
   - `AGENTS.md`, `knowledge-base.yaml` and `notes/` are not exported: link them on GitHub if you
     need them.

Done when `grep -rnE --exclude-dir=scions 'meta-knowledge-base/(policies|playbooks|starter-kits)/|docs/meta\.md|meta\.yaml|\.graft/meta-knowledge-base' .`
finds nothing and `nix flake check` passes.

## 2026-01-05 - Evolution for evolving knowledge bases

Based on analysis of Graft project evolution (53 commits, 84 files, 15k+ lines), added guidance for knowledge bases that evolve significantly.

### New policies

**Temporal layers** - three-tier organization:
- Ephemeral (notes/): scratch work, days to weeks
- Tracking (status/): implementation progress, weeks to months
- Durable (docs/): architecture and guides, indefinite

**Generated content** - when to automate:
- Pain triggers: synchronization, scale, consistency
- Three levels: indexes, extracted summaries, fully generated
- Mark generated content, separate concerns

### Enhanced playbooks

**Evolve KB** - evidence-based evolution:
- Five pain types with thresholds
- Minimal interventions before adding complexity
- Evolution moves ordered by cost
- Require 3+ examples before infrastructure

**Manage KB dependencies** - cross-repository coordination:
- Four relationship types: foundational, canonical-source, peer, generated-from
- Three sync policies: interpret, extend, reference
- Cross-repository provenance patterns

**Multi-agent coordination** (experimental):
- Three coordination patterns: sequential, concurrent, collaborative
- Communication protocols
- Best practices and anti-patterns

### Updated core

**meta.md** - new dimensions (2026):
- Temporal layers
- KB dependencies
- Evolution triggers
- Generated content
- Multi-agent coordination

**style.md** - enhanced:
- Conciseness targets (policies: 20-40 lines, playbooks: 50-100 lines)
- Context size management for LLM consumption

### Philosophy maintained

All additions:
- Optional, not mandatory
- Evidence-driven (from real project pain)
- Minimal intervention approach preserved
- Stable entrypoint unchanged (meta.md)

### Evidence base

- Graft project: 53 commits, 84 files, 15,768 lines
- Observed gaps: status/ lifecycle, cross-repo coordination, evolution triggers
- Real pain points, not theoretical needs

## History: knowledge-base-integrity

The checker's changelog before the merge, as released from github.com/danielnaab/knowledge-base-integrity.

### [0.4.0] - 2026-11-06

#### Added

- OKF bundles: `rules.checks.okf.bundle` declares a directory as an OKF v0.2 bundle, and `.okfignore` at its
  root (`.gitignore` syntax) excludes files from it.
  - kb-linter checks conformance: every note has frontmatter with a non-empty `type`; `index.md` has no
    frontmatter (the root's may hold `okf_version`) and lists headings and entries; `log.md` headings are dates.
    `rules.checks.okf.version` pins the root's `okf_version`.
  - link-validator classifies links in the bundle: a missing note is an `unwritten-link` or `likely-typo`
    notice and does not fail; `escapes-bundle`, `malformed-link`, `misdirected-link` and `wikilink` fail.
    `/` links resolve from the bundle root. Output gains a `notices` array and `unwritten`/`likely_typos` counts.
- Structured provenance (meta-knowledge-base v1.1.0): a `sources` frontmatter list whose entries each have a
  `resource` counts as provenance in every KB; a malformed one is `invalid-sources`.
- link-validator checks heading anchors in every KB (`missing-anchor`), accepting GitHub and dashed slugs,
  Obsidian heading text and block ids, and HTML ids.
- The configuration reader reads inline mappings (`{ by: x, at: y }`) and nested inline lists.

#### Changed

- meta-knowledge-base v1.1.0 (structured provenance; raw-sources and historical layers).

#### Fixed

- Link titles (`[x](path.md "Title")`) are no longer read as part of the path.

#### Upgrading

- A KB without `rules.checks.okf` sees two changes: links to missing headings fail (`missing-anchor`), and
  same-file `#heading` links count toward `links_checked`. Fix the anchor or the heading, or set
  `rules.checks.links.anchors: false` until you can.
- Anything parsing link-validator's JSON should read the new `notices` array; notices never fail a run.

### [0.3.0] - 2026-11-06

#### Added

- The tools are flake apps: `nix run .#lint`, `.#check-links`, `.#scan-backlinks` and `.#validate`,
  run from a knowledge base's root. A repo takes this flake as an input and re-exports the ones it
  wants, in place of wrapping the modules itself.

#### Changed

- graft v2.6.0.

### [0.2.0] - 2026-11-06

#### Added

- Scan scopes are configurable in `knowledge-base.yaml` under `rules.checks` (`links`, `lifecycle`,
  `provenance`, each with `include`/`exclude` globs; `lifecycle.extraStatuses`). Defaults are the old
  directories, so KBs without the block behave as before. A KB with content under `notebook/` or at
  the root can now be checked in full, including hidden directories an include names (`.pi/skills/**`).
- Shared `tool_cli.config` (YAML-subset reader) and `tool_cli.scope` (globs and file collection).

#### Fixed

- `rules.lifecycle.statuses` written as a bare YAML list (`[draft, working]`) or block list was read as
  empty, so the invalid-status check silently never ran. It is read correctly, and a value that cannot be
  read as a list now exits 2.
- `status: living` is accepted (the meta-knowledge-base temporal-layers policy prescribes it).
- `## Sources and verification` and similar headings count as a Sources section.

#### Changed

- When `rules.checks.provenance.include` is set it replaces `sources.canonical` as the provenance scope.
- All three tools skip directories graft wrote (`scions/`, or wherever graft's `dir` puts
  them), recognized by the marker in their README.md. The link validator still checks links
  into them.

### [0.1.0] - 2026-01-31

#### Added

- Initial extraction from delivery-practices repository
- Backlink Scanner (`backlink-scanner`):
  - Scan for `# spec:` and `// spec:` annotations
  - Detect dangling references and orphan specs
  - Section-level traceability via `spec-section:` annotations
- KB Linter (`kb-linter`):
  - Validate frontmatter status fields
  - Check provenance requirements for canonical content
  - Read rules from knowledge-base.yaml
- Link Validator (`link-validator`):
  - Detect broken internal markdown links
  - Skip code blocks and inline code
  - Handle relative paths and fragments
- Shared CLI infrastructure (`tool_cli`):
  - Consistent JSON output format
  - Exit codes: 0 (clean), 1 (issues), 2 (config error)
  - `--report-only` flag for informational runs
- Behavioral specifications for all tools
- Comprehensive test suite
