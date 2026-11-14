---
status: working
last-verified: 2026-11-06
---

# Spec: tool-cli

Shared CLI runner that enforces consistent interface conventions across all of the knowledge-base-kit's tools.

## Intent

Three tools share identical CLI behavior: argument parsing, JSON output, exit codes, and `--report-only` mode. Rather than duplicating this logic, `tool_cli` provides a single entry point that accepts tool-specific functions and handles the rest.

## Non-goals

- **Argument parsing library** — no argparse/click; the interface is intentionally minimal
- **Output formatting options** — always JSON, always indented
- **Tool-specific logic** — the runner is generic; tools provide their own runner/serializer/has_failures

## Behavior

### Argument handling

- `--report-only` flag is extracted from argv regardless of position
- First non-flag argument is the root directory (defaults to `.`)
- Unknown flags are passed through as positional arguments (no validation)

### Execution

- Calls `runner(root_dir)` with the resolved directory path
- If `runner` raises `FileNotFoundError` or `ConfigError`, prints error to stderr and exits 2
- Other exceptions propagate (intentional — tool bugs should be visible)

### Output

- Calls `serializer(result)` and prints as JSON with 2-space indent to stdout
- Output is always valid JSON (one object, no streaming)

### Exit codes

- **0**: No failures found (or `--report-only` is set)
- **1**: `has_failures(result)` returns True (and `--report-only` is not set)
- **2**: Configuration error (`FileNotFoundError` or `ConfigError` from runner)

### `--report-only` mode

- When set: always exit 0 after printing output, regardless of failures
- Purpose: informational runs in contexts where non-zero exit would abort a pipeline

### Graft directories

- `tool_cli.graft.is_graft_dir(path)` is true when `path/README.md` carries one of graft's markers in its first 2 KB: `<!-- managed by graft: https://github.com/danielnaab/graft -->` (graft 2.1 and later, line 1) or `Managed by [graft]` (2.0)
- Every tool prunes such directories from its walk, at any depth
- The markers match steward's `GRAFT_MARKERS` (duties/check-kb-integrity); change both together

### Configuration

`tool_cli.config` reads `knowledge-base.yaml` for every tool, so layouts and rules are declared once.

```gherkin
Given a knowledge-base.yaml using nested mappings, block lists (including lists of mappings),
  inline lists with quoted or bare items ([draft, working] or ["draft", "working"]),
  inline mappings ({ by: agent/1.0, at: 2026-06-01T00:00:00Z }), nested in either kind of list,
  quoted or bare scalars, and # comments
When a tool reads it
Then values are available by key path (rules.lifecycle.statuses)
  And YAML outside this subset (anchors, block scalars) is ignored, not an error
```

A braced value becomes a mapping only when every item is `key: value`; `{draft}` and a template's `{{type}}` stay strings, as before.

The same reader parses note frontmatter (`type`, `sources`, `okf_version`); an inline mapping's key ends at its first `: `, so `by: human:dan` reads as `{by: "human:dan"}`.

```gherkin
Given a requested key that is present but is not a list of strings
When a tool asks for it as a list
Then it raises ConfigError naming the key path, and the tool exits 2
  And a lone string is read as a one-item list
  And an absent key is not an error (the tool uses its default)
```

Never silently check nothing: a value the tool is meant to enforce that cannot be read is a configuration error.

### Path scopes

A scope is a list of `include` globs and an optional list of `exclude` globs over repository-relative POSIX paths.

```gherkin
Given the glob syntax: * matches within one path segment, ** matches across segments, ? matches one character
When a path is matched
Then docs/** matches every path beneath docs/ (and not docs itself as a file)
  And **/x.md matches x.md at any depth including the root
  And README.md matches only the root README.md
```

```gherkin
Given a scope with include globs
When a tool collects files
Then it returns the .md files matching an include and no exclude, sorted by path
  And it never descends into SKIP_DIRS or directories graft wrote
  And it descends only into directories an include glob can reach, so hidden directories
    (.pi/, .obsidian/) are walked only when an include names them or reaches them with **
```

`rules.checks` in `knowledge-base.yaml` declares scopes. Every key is optional; without it each tool keeps its default scope.

| Key | Used by | Default |
|-----|---------|---------|
| `rules.checks.links.include` / `.exclude` | link-validator | `docs/** policies/** playbooks/** notes/** specs/**` |
| `rules.checks.lifecycle.include` / `.exclude` | kb-linter (status) | `docs/** policies/** playbooks/**` |
| `rules.checks.lifecycle.extraStatuses` | kb-linter | none (`living` is always allowed) |
| `rules.checks.provenance.include` / `.exclude` | kb-linter (Sources) | `sources.canonical` paths within the lifecycle scope |
| `rules.checks.links.anchors` | link-validator | `true`: heading anchors are checked |
| `rules.checks.okf.bundle` | kb-linter, link-validator | none: no OKF checks (see [OKF bundle](#okf-bundle)) |
| `rules.checks.okf.version` | kb-linter | none: `okf_version` not checked |

### OKF bundle

A KB may declare a directory as an [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) bundle. `tool_cli.okf` reads the declaration and `.okfignore`, and splits frontmatter for both tools.

```gherkin
Given rules.checks.okf.bundle names a directory in the repository ("notebook", or "." for the root)
When a tool starts
Then that directory, normalized (notes/.. is the root), is the bundle
  And rules.checks.okf that is not a mapping, a bundle that is not a directory in the repository,
    or a version that is not a string exits 2
```

```gherkin
Given a .okfignore file at the bundle root
When the bundle's notes are collected
Then its patterns follow .gitignore rules: # comments, ! negation, a trailing / for directories,
  a / at the start or middle anchors the pattern to the bundle root, otherwise it matches at any depth,
  *, ?, [...], and ** as a whole segment (**/x, x/**, a/**/b; elsewhere ** is *)
  And trailing spaces are stripped unless escaped
  And the last matching pattern decides, and a file under an excluded directory stays excluded
  And excluded files are not notes: they get no OKF conformance checks
```

```gherkin
Given a bundle
When its notes are collected
Then they are the .md files beneath it, minus .okfignore matches
  And hidden directories inside the bundle (.obsidian/, .trash/) are not part of it
```

- Frontmatter is a block delimited by `---` lines at the very start of a file, read with the configuration reader.
- `.okfignore` is option B of the open [OKF issue #58](https://github.com/GoogleCloudPlatform/open-knowledge-format/issues/58); a later release follows OKF if it adopts another mechanism.

## Constraints

- No runtime dependencies beyond Python stdlib
- Generic over result type (uses TypeVar)
- The configuration reader is a targeted subset parser, not a general YAML parser (stdlib only)
- Each tool provides three callables: `runner`, `serializer`, `has_failures`

## Decisions

- 2026-01-24: Extracted after 3 tools shared identical __main__.py patterns.
- 2026-01-24: No argparse — the minimal interface (one flag, one positional) doesn't justify the dependency. If more flags are added, reconsider.
- 2026-01-24: FileNotFoundError specifically (not general OSError) because tools raise it for missing config files (knowledge-base.yaml, spec directories).
- 2026-11-05: Skip directories graft wrote, found by the marker in their README.md (`tool_cli.graft.is_graft_dir`), wherever they sit: `scions/`, a hidden `.scions/`, or `docs/Grafted/`. A grafted directory is another repo's export, checked by that repo, and graft's `nix flake check` fails if it is edited here, so a finding inside one can't be acted on. Before this, only steward's check-kb-integrity duty filtered them, so a direct run reported upstream content as this repo's.
- 2026-11-05: One config reader and path-scope helper in `tool_cli`, shared by the linter and link validator, so scan roots, exclusions and list syntax mean the same thing in both. Motivated by a notebook-style KB (content under `notebook/`, root `README.md`/`AGENTS.md`) where hardcoded roots checked 6 of ~75 files.
- 2026-11-05: A subset parser replaces regex extraction of single keys. The old statuses regex read only JSON-style lists, so valid YAML (`[draft, working]`) produced an empty list and the status check silently never ran. A requested value that cannot be read now exits 2.
- 2026-11-05: Unsupported YAML constructs are ignored rather than rejected, so a KB whose configuration uses other features (and never asked for these checks) keeps working. Only a requested key with the wrong shape is an error.
- 2026-11-05: Hidden directories are walked only when an include glob can reach them, so `.pi/skills/**` can be checked while `.obsidian/` and `.direnv/` are not scanned by accident.
- 2026-11-06: An OKF bundle is declared in `rules.checks.okf` and checked by the existing tools, not a new one: conformance is a content rule (kb-linter) and link classes are link rules (link-validator), and workspaces that already run both get the checks from configuration alone. Motivated by the notebook-conventions proposal to make `notebook/` an OKF v0.2 bundle.
- 2026-11-06: The reader gains inline mappings and nested inline lists. OKF frontmatter writes `generated: { by: …, at: … }` and `verified: [{ … }]`, and agents write `sources` entries the same way; reading them as strings would hide a missing `resource`.
- 2026-11-06: Hidden directories inside a bundle are not part of it. `**` reaches hidden directories in an ordinary scope (a tested behavior KBs may rely on), but Obsidian keeps `.obsidian/` and `.trash/` inside the vault, and neither holds notes.

## Related

- [backlink-scanner](backlink-scanner.md) — consumer
- [kb-linter](kb-linter.md) — consumer
- [link-validator](link-validator.md) — consumer

## Sources

- Extracted from: identical patterns in three __main__.py files
- Living Specifications Principles — #5 (low ceremony, high signal)
