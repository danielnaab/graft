---
status: done
created: 2026-02-22
---

# Add an iterate command that loads a slice plan and generates a prompt for implementing the next unchecked step

## Story

The iterate command takes a slice path, reads its plan.md, identifies the next unchecked step, and generates a focused implementation prompt with project state -- closing the plan-iterate loop so agents can work through slices step by step.

## Approach

Follow the same pattern as the plan command: a Tera template in software-factory that receives state context, plus a new template variable for the slice content. The iterate command is defined in graft.yaml with `stdin:` pointing to `.graft/software-factory/templates/iterate.md`. The template reads the slice plan from the CLI args (the slice path), extracts the next `- [ ]` step, and renders a prompt that includes the step details, verification status, and recent changes. The slice content is injected via a new `scripts/read-slice.sh` state query that takes the slice path as an argument, or more simply, the template instructions tell the agent what to do with the step -- the slice file content itself is passed as a CLI arg that the template can reference.

Since the template engine already has `args` for CLI arguments, the simplest approach is: `graft run iterate slices/my-slice` passes the slice path as args, and a state query `slice` reads the plan.md from that path and returns its content plus parsed metadata (next step, progress). The template then renders a focused prompt around the next step.

## Acceptance Criteria

- `graft run iterate slices/<slug>` renders a prompt focused on the next unchecked step from the slice's plan.md
- The prompt includes: the step's name, delivers, done-when, and files sections
- The prompt includes: the full story and approach for context
- The prompt includes: verification status and recent changes (same as plan)
- The prompt includes: overall progress (e.g. "Step 3 of 5")
- When all steps are checked, the prompt says the slice is complete and suggests marking status as `done`
- When the slice path doesn't exist, the command fails with a clear error
- `graft run iterate --dry-run slices/<slug>` works for previewing the prompt

## Steps

- [x] **Create `scripts/read-slice.sh` state query**
  - **Delivers** -- a script that reads a slice's plan.md, extracts frontmatter status, finds the next unchecked step, and returns structured JSON
  - **Done when** -- `bash scripts/read-slice.sh slices/iterate-command` returns JSON with `{status, story, approach, steps_total, steps_done, next_step: {name, delivers, done_when, files}, content}`
  - **Files** -- `scripts/read-slice.sh`

- [x] **Create iterate template**
  - **Delivers** -- a Tera template that renders a focused implementation prompt for the next unchecked step
  - **Done when** -- template renders with step details, story context, verification status, progress indicator, and implementation instructions
  - **Files** -- `.graft/software-factory/templates/iterate.md`

- [x] **Wire iterate command in graft.yaml**
  - **Delivers** -- `graft run iterate slices/<slug>` works end-to-end
  - **Done when** -- `graft run iterate --dry-run slices/iterate-command` renders the prompt with the first unchecked step from this plan
  - **Files** -- `graft.yaml`
