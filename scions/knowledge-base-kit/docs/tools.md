---
status: working
---

# The checker

How to run and configure the kit's checker: three tools in `src/` that check a knowledge base against the
conventions in this kit.

## Quick Start

From a clone of this repo:

```bash
uv pip install -e .
backlink-scanner /path/to/kb
kb-linter /path/to/kb
link-validator /path/to/kb
```

From a repo that grafts the kit (no install; Python 3.11+ standard library only):

```bash
PYTHONPATH=scions/knowledge-base-kit/src python3 -B -m link_validator .
PYTHONPATH=scions/knowledge-base-kit/src python3 -B -m kb_linter .
```

`-B` keeps Python from writing `__pycache__/` into the scion, which graft's drift check would report. In a
Nix check, read the fetched export instead: `PYTHONPATH=${inputs.knowledge-base-kit}/src`.

Exit codes: 0 no findings (or `--report-only`), 1 findings, 2 a configuration error. Output is JSON.

## Tools Overview

| Tool | Purpose | Checks |
|------|---------|--------|
| `backlink-scanner` | Spec traceability | `# spec:` annotations point to real specs |
| `kb-linter` | Content rules | Frontmatter status, provenance (Sources section or `sources` frontmatter), OKF conformance |
| `link-validator` | Link integrity | Internal markdown links and their heading anchors exist; OKF link classes |

## Configuring scan scopes

By default the tools check `docs/`, `policies/`, `playbooks/` (the linter) and also `notes/`, `specs/`
(the link validator). To check another layout, add `rules.checks` to `knowledge-base.yaml`:

| Key | Tool | Meaning |
|-----|------|---------|
| `rules.checks.links.include` / `exclude` | link-validator | Markdown files whose links are checked |
| `rules.checks.lifecycle.include` / `exclude` | kb-linter | Files whose frontmatter `status` is checked |
| `rules.checks.lifecycle.extraStatuses` | kb-linter | Statuses allowed beyond `rules.lifecycle.statuses` (`living` is always allowed) |
| `rules.checks.provenance.include` / `exclude` | kb-linter | Files that need a `## Sources` section or `sources` frontmatter; replaces `sources.canonical` as the scope |
| `rules.checks.links.anchors` | link-validator | `false` turns heading-anchor checks off (default `true`) |
| `rules.checks.okf.bundle` | both | A directory that is an OKF bundle (see below) |
| `rules.checks.okf.version` | kb-linter | The `okf_version` the bundle-root `index.md` must declare |

- Globs are repository-relative: `*` stays within a path segment, `**` crosses segments, `dir/**` is everything beneath `dir/`.
- Hidden directories (`.pi/`) are walked only when an include names them or reaches them with `**`. `.git/`, `node_modules/` and directories graft wrote are never walked.
- `rules.lifecycle.statuses` may be an inline list (quoted or bare) or a block list.
- If a configured value cannot be read, the tool exits 2 and names the key.
- Example: a notebook-style KB with dated notes (no status) and wiki pages (status and Sources) sets
  `lifecycle.include` and `provenance.include` to `docs/**` and `notebook/wiki/**`, and `links.include` to everything.

## Checking an OKF bundle

Declare the bundle, and optionally the OKF version it targets:

```yaml
rules:
  checks:
    okf:
      bundle: notebook
      version: "0.2"
```

- **kb-linter** checks every note in the bundle for frontmatter with a non-empty `type`, `index.md` files
  for OKF's listing shape (no frontmatter but `okf_version` at the root; headings and `* [Title](url) - description`
  entries), and `log.md` files for date headings.
- **link-validator** reports a link to a note not written yet as an `unwritten-link` notice, which does not fail
  the run. Links that leave the bundle, `[[wikilinks]]`, unencoded spaces, and a link whose note exists under the
  same name elsewhere (`misdirected-link`) fail.
- `.okfignore` at the bundle root (`.gitignore` syntax) excludes files, such as `templates/`, from conformance.
  Hidden directories (`.obsidian/`) are never part of the bundle.

Every KB, bundle or not: heading anchors in links are checked, and `sources` frontmatter whose entries each have
a `resource` counts as provenance.

## Sources

- [Specifications](https://github.com/danielnaab/knowledge-base-kit/tree/main/specs) — behavioral contracts for each tool
- [tool-cli: configuration and path scopes](https://github.com/danielnaab/knowledge-base-kit/blob/main/specs/tool-cli.md#configuration)
