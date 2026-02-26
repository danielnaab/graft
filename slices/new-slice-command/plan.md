---
status: draft
created: 2026-02-26
depends_on: [sequence-declarations]
---

# Draft a new slice plan from a feature description

## Story

Creating a new slice plan requires manually writing a `slices/<slug>/plan.md` file that
follows the template, injects current project context, and fits into the existing
dependency order. The manual effort means ideas sit undocumented or get drafted
hastily. A `new-slice` command takes a feature description and uses Claude to produce
a complete draft plan populated with project context, so it avoids duplicating
existing work and slots into the backlog immediately.

## Approach

New `scripts/new-slice.sh` accepts a feature description, reads current project context
(list-slices.sh output, recent git log), and pipes an augmented prompt through
`claude -p --dangerously-skip-permissions` to produce the plan file.

The prompt instructs Claude to:
- Derive a kebab-case slug from the description
- Output the full `slices/<slug>/plan.md` content in the standard template format
- Reference existing slices to avoid duplication and inform `depends_on`
- Use `status: draft` and `created: <today>` in the frontmatter

The prompt instructs Claude to output `slug: <value>` as the **very first non-empty
line** of its response, followed by the full plan file content. The script extracts
the slug from that marker line and exits 1 with a clear error if the marker is absent
or malformed. If `slices/<slug>/plan.md` already exists, it exits non-zero with a
clear error.

Add a `new-slice` command to `graft.yaml` with a required `description` string arg.

## Acceptance Criteria

- `graft run software-factory:new-slice "description"` produces `slices/<slug>/plan.md`
  with correct frontmatter (`status: draft`, `created: <date>`)
- Generated plan includes Story, Approach, Acceptance Criteria, and at least one Step
  with **Delivers**, **Done when**, and **Files** sub-bullets
- The slug is kebab-case derived from the description
- If `slices/<slug>/plan.md` already exists, command exits non-zero with
  `"slice already exists: slices/<slug>"`
- The plan does not duplicate existing slices (context injection includes current
  slice names and statuses)
- `cargo test` passes with no regressions (this is a script-only change)

## Steps

- [ ] **Write `scripts/new-slice.sh` and add `new-slice` command to `graft.yaml`**
  - **Delivers** — `graft run software-factory:new-slice "description"` produces a
    drafted slice plan file
  - **Done when** — `new-slice.sh` reads context from `list-slices.sh` and
    `git log --oneline -10`; constructs a prompt combining the description, existing
    slices summary, and the plan.md template format (Story, Approach, Acceptance
    Criteria, Steps with sub-bullets); pipes to `claude -p --dangerously-skip-permissions`;
    requires Claude to output `slug: <value>` as the very first non-empty line of its
    response; extracts the slug from that marker; exits 1 with
    `"error: missing slug: marker on first line"` if the marker is absent or not
    kebab-case; writes to `slices/<slug>/plan.md` with the current date; exits 1 if
    the slug already exists; `graft.yaml` gains a `new-slice` command with a required
    positional `description` string arg; end-to-end manual test: run the command,
    inspect the generated plan file, confirm it compiles with the template format
  - **Files** — `.graft/software-factory/scripts/new-slice.sh`,
    `.graft/software-factory/graft.yaml`
