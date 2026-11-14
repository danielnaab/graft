---
status: working
last-verified: 2026-11-06
owners: [daniel]
---

# Link Validator

## Intent

Detect broken internal links in markdown content — files that reference paths or headings which don't exist. This catches link rot caused by file renames, moves, or deletions without updating references. Inside an OKF bundle, it tells a note not written yet from a broken link.

This complements the [backlink scanner](backlink-scanner.md) (code→spec traceability) and [KB linter](kb-linter.md) (frontmatter/provenance rules) to form a structural integrity trio for the knowledge base.

## Non-goals

- External URL validation (network-dependent, slow, flaky — separate concern)
- Link content quality (whether link text is descriptive)
- Fixing broken links (detection only)
- Checking links inside code blocks or inline code spans (those are examples, not navigational)

## Behavior

### Link extraction

```gherkin
Given a markdown file
When the validator scans it
Then it extracts all [text](target) links where target is a relative path
  And it skips links inside fenced code blocks, inline code spans and HTML comments
  And it still extracts links whose text is or contains a code span ([`name.py`](path))
  And it skips external links (any target that begins with a URI scheme)
```

```gherkin
Given a link whose target begins with a URI scheme
  (a letter, then letters, digits, "+", "-" or ".", then ":", per RFC 3986)
When the validator scans it
Then it treats the link as external and does not check or count it
  Examples: http:, https:, mailto:, tel:, file:, data:, airmail:, app:
```

```gherkin
Given a link inside an HTML comment (<!-- ... -->), on one line or spanning several
When the validator scans it
Then it does not check or count it, and a code fence inside the comment does not open a block
```

```gherkin
Given a link whose first ":" comes after a "/", "?" or "#" (e.g., sub/a:b.md)
When the validator scans it
Then it treats the link as a relative path and checks it
```

```gherkin
Given a link with a CommonMark title [text](path.md "Title") or [text](path.md 'Title')
When the validator scans it
Then the title is not part of the path: it checks path.md
```

### Path resolution

```gherkin
Given a link [text](../format.md) in file docs/living-specifications/guides/writing-specs.md
When the validator resolves the path
Then it resolves relative to the linking file's directory: docs/living-specifications/format.md
```

```gherkin
Given a link [text](living-specifications/format.md) in file docs/README.md
When the validator resolves the path
Then it resolves to docs/living-specifications/format.md
```

```gherkin
Given a link with a fragment [text](../docs/living-specifications/principles.md#principle-2)
When the validator resolves the path
Then it strips the fragment and checks only the file path: docs/living-specifications/principles.md
```

```gherkin
Given a link with a percent-encoded target [text](../transcripts/Weekly%20Sync%2C%20Notes.md)
When the validator resolves the path
Then it strips the fragment and query, then decodes the percent-encoding
  And it checks the decoded path: transcripts/Weekly Sync, Notes.md
```

```gherkin
Given a link with an angle-bracket destination [text](<Name with spaces.md>)
When the validator resolves the path
Then it removes the angle brackets and resolves the path inside them: Name with spaces.md
```

### Anchors

```gherkin
Given a link with a fragment to an existing .md file, or a same-file link (#fragment)
When the fragment is decoded (%20 to a space)
Then it passes if the target file has a matching anchor:
  a heading's GitHub slug, compared in any case (## Done when → done-when; a repeat takes the next
    free -1, -2, skipping slugs already taken: Foo, Foo, Foo-1 → foo, foo-1, foo-1-1)
  a heading's dashed slug, where each run of punctuation and space is one "-" (What's open → what-s-open)
  a heading's text as Obsidian writes it (Done%20when), or the last part of a nested Parent#Child
  an Obsidian block id (^block-1) on a line ending in ^block-1
  an HTML id or name attribute (<a id="x">)
  And otherwise it reports "missing-anchor"
```

```gherkin
Given headings in a target file
When its anchors are collected
Then ATX (## Heading) and setext (Heading over === or ---) headings count, including ATX headings
    in blockquotes, callouts and list items (> ## A, - ## B)
  And slugs are made from the rendered text: images and HTML tags dropped, link text kept,
    entities decoded (Run <code>make</code> → run-make, A &amp; B → a--b)
  And frontmatter and fenced code blocks are not read, and a fence closes only on its own
    character repeated at least as long (a ``` line inside a ```` block does not close it)
```

- `rules.checks.links.anchors: false` turns anchor checks off, for a KB that needs time to fix them after upgrading; any value but `true` or `false` exits 2.
- Fragments on links to directories or non-markdown files (`file.pdf#page=2`) are not checked, nor on links with a query (`file.md?plain=1#L3` is a source view's line).
- Same-file links now count toward `links_checked`.

### Links in an OKF bundle

Applies to links written in files inside the bundle `rules.checks.okf.bundle` declares (see [tool-cli](tool-cli.md#okf-bundle)). Links in files outside the bundle are checked as before, including links into it.

```gherkin
Given a link in the bundle whose target is a missing .md file inside the bundle
When no file with that name (any case) exists elsewhere in the bundle
Then it is a notice, not a violation: "unwritten-link" (OKF §6.1: a note not written yet)
  And an unwritten index.md, README.md or log.md is always "unwritten-link": many folders share those names
  Or "likely-typo" when a note's filename is close to it (difflib ratio at least 0.85), naming the note
When a note with the same filename exists elsewhere in the bundle
Then it reports "misdirected-link" naming that note (a wrong relative path, typically a wrong depth)
```

```gherkin
Given a link in the bundle
When it resolves outside the bundle (../docs/guide.md), even to an existing file
Then it reports "escapes-bundle" (links out of the bundle are absolute URLs or backticked paths)
When its destination has an unencoded space and no angle brackets ([x](Pat Lee.md))
Then it reports "malformed-link"
When it starts with "/"
Then it resolves from the bundle root, as OKF §6.1 defines
```

```gherkin
Given a [[wikilink]] in a file in the bundle, outside code and HTML comments, not followed by "("
  ([[1]](#ref) is a citation-style link)
When the validator scans it
Then it reports "wikilink": a bundle uses Markdown links only
```

- A missing target that is not a `.md` file (an image) is a `broken-link`, as before.
- Hidden directories and `.okfignore` do not change which files' links are checked; `rules.checks.links` decides that.

### Validation

```gherkin
Given a resolved path pointing to an existing file
When validation completes
Then no violation is reported
```

```gherkin
Given a resolved path pointing to an existing directory (e.g., ../docs/)
When validation completes
Then no violation is reported
```

```gherkin
Given a resolved path pointing to a file that does not exist
When validation completes
Then it reports a "broken-link" violation with the source file, link target, and resolved path
```

### Scanned paths

- Scope: `rules.checks.links.include` minus `.exclude` in knowledge-base.yaml (see [tool-cli](tool-cli.md#path-scopes)); default `docs/`, `policies/`, `playbooks/`, `notes/`, `specs/`
- knowledge-base.yaml is optional here: without it (or without `rules.checks.links`) the default scope applies
- A configured value of the wrong shape exits 2
- Root-level files (README.md, AGENTS.md) and hidden directories (`.pi/`) are scanned when an include glob names them
- Only `.md` files are scanned for links
- Files in subdirectories are included recursively
- Skips: .git/, .graft/, .venv/, node_modules/, __pycache__/
- Skips directories graft wrote (see [tool-cli](tool-cli.md#graft-directories)); links *into* them are still checked, since the target exists

### Output structure

```gherkin
Given completed validation
When results are reported
Then output is a JSON object with a flat violations array, a notices array, and a summary object
  And notices never affect the exit code
```

Rules: `broken-link`, `missing-anchor`, and in a bundle `escapes-bundle`, `malformed-link`, `misdirected-link`, `wikilink` (violations) and `unwritten-link`, `likely-typo` (notices).

Example output:
```json
{
  "violations": [
    {
      "file": "docs/guide.md",
      "target": "../old-reference.md",
      "resolved": "old-reference.md",
      "rule": "broken-link",
      "message": "Link target does not exist"
    }
  ],
  "notices": [],
  "summary": {
    "files_checked": 15,
    "links_checked": 42,
    "broken": 1,
    "unwritten": 0,
    "likely_typos": 0
  }
}
```

### Exit codes

- **Exit 0**: no broken links found
- **Exit 1**: one or more broken links found
- **`--report-only` flag**: always exit 0

### Edge cases

- Links with query strings (e.g., `file.md?raw=true`): strip query, check file path
- Links to paths outside the repository root (e.g., `../../../etc/passwd`): report as broken (resolved path escapes root)
- Empty link targets `[text]()`: skip (not a path reference)
- Image links `![alt](path)`: check the path (images can rot too)
- Multiple links to the same broken target: report each occurrence separately
- Absolute paths starting with `/`: resolve from repository root

## Constraints

- Runs in Python (no runtime dependencies beyond standard library)
- Completes in under 1 second for knowledge bases up to 1,000 content files
- Does not access the network

## Open Questions

- [x] Should links into graft's directories be validated? Yes: they are this repo's links, and a target an upgrade removed is exactly the breakage to catch. Links *inside* graft's directories are not (2026-11-05).

## Decisions

- 2026-01-24: Scan all content directories including notes/ and specs/. Unlike the KB linter (which skips notes/specs), link rot affects navigability regardless of content type.
- 2026-01-24: Skip links inside code blocks and inline code spans. Specs and playbooks contain example link syntax that is illustrative, not navigational.
- 2026-01-24: Check image links too. A broken image reference is as bad as a broken text link for content integrity.
- 2026-01-24: Strip fragments before checking. Anchor validation is a harder problem (requires parsing headings) and is explicitly a non-goal for v1. Superseded 2026-11-06: anchors are checked.
- 2026-10-30: Treat any URI scheme as external, not just http:, https: and mailto:. App-specific schemes (Airmail's `airmail://message?...`, `tel:`, `data:`, `app:`) were resolved as relative paths and reported as broken: 264 false findings across 18 knowledge bases. `file:` links are external too, since they point outside the repository.
- 2026-10-30: Decode percent-encoding after stripping the fragment and query. CommonMark and Obsidian decode `%20` and `%2C`, so `Meeting%20Notes.md` names `Meeting Notes.md`. Decoding last keeps an encoded `%23` in the path from being mistaken for a fragment. Violations report the target as written and the decoded resolved path.
- 2026-10-30: Accept CommonMark angle-bracket destinations (`<Name with spaces.md>`). Obsidian writes them for filenames with spaces.
- 2026-10-30: Replace inline code spans with placeholder text, not nothing, before matching links. Removing them outright also removed code-span link text, so `` [`gate.py`](path) `` was never checked and a real broken link went unreported. A link written entirely inside a code span is still ignored.
- 2026-10-30: Skip links inside HTML comments. They aren't rendered, and templates use them for commented-out example entries: henry-papa-workbench's notes/README.md had four example links in a `<!-- ... -->` block reported as broken.
- 2026-11-05: Skip directories graft wrote, found by the marker in their README.md (`tool_cli.graft.is_graft_dir`), wherever they sit: `scions/`, a hidden `.scions/`, or `docs/Grafted/`. A grafted directory is another repo's export, checked by that repo, and graft's `nix flake check` fails if it is edited here, so a finding inside one can't be acted on. Before this, only steward's check-kb-integrity duty filtered them, so a direct run reported upstream content as this repo's.

- 2026-11-05: Scan scope is configurable (`rules.checks.links`) through the shared [tool-cli](tool-cli.md#configuration) reader, defaults unchanged. A notebook-style KB keeps its content under `notebook/` and at the root, which the hardcoded directories never reached. The configuration file stays optional for this tool, as before.
- 2026-11-06: Check heading anchors. The 2026-01-24 non-goal deferred it as the harder problem; the notebook-conventions OKF proposal makes a missing anchor a failure, and across 19 local KBs the check found one real broken anchor once nested fences and dashed slugs were handled. Several slug styles are accepted because these KBs are read on GitHub, Forgejo and in Obsidian, which write different ones.
- 2026-11-06: In an OKF bundle a missing note is a notice, not a failure. OKF §6.1 treats a link to a missing concept as not-yet-written knowledge, and the notebook links people and topics before their notes exist. A same-name note elsewhere is a failure instead (`misdirected-link`): a wrong relative depth is the common mistake, and an unwritten note with exactly an existing note's filename is rare. A merely close name stays a notice (`likely-typo`) because there is no way to mark a deliberate near-duplicate.
- 2026-11-06: Bundle rules apply only inside the bundle. Escaping links, wikilinks and unencoded spaces stay legal elsewhere, so KBs without a bundle see no change beyond anchors and link titles.
- 2026-11-06: A review of the branch found false positives this spec now rules out: headings in blockquotes and lists, HTML and entities in headings, taken suffixed slugs, line anchors on source views, citation brackets, and common filenames read as misdirected links.
- 2026-11-06: Strip CommonMark link titles, after an angle-bracket destination too. `[x](path.md "Title")` was checked as the path `path.md "Title"` and reported broken.

## Sources

- [KB Linter](kb-linter.md) — sibling tool, structural patterns to follow
- [Backlink Scanner](backlink-scanner.md) — sibling tool, completes the integrity trio
- Living Specifications: Verification and Drift Detection — link validation as detection approach
- [OKF v0.2 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) — §6.1 links between concepts
