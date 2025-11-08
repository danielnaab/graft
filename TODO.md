# TODO

## Things to consider

- refactor graft implementation - instead of handling LLM patches natively, switch graft to a model where it pipelines output through external processes. think deeply about the advantages of this model and the qualities the solution should have to be consistent with the goals of this system, while remaining flexible enough to support non-llm transformation.
- "graft" can be used as a noun and a verb
  - noun: a graft on a git tree
  - verb: graft _X_ onto the tree, update/grow/prune/feed the graft
- grafts can grow multiple files
  - artifacts produced are defined in `graft.md` or `<name>.graft.md`
- `<name>.graft.md`
  - can produce any `name.*` outputs (including subdirectories)
- `<name>/graft.md`
  - defines `<name>` as a graft
  - has a primary output: `REPORT.md`, `GRAFT.md`, `<name>.md`, or similar
  - can produce outputs in the directory
  - maybe dependencies should only be pulled from other directories, but we could if the user opts in, maybe not. let's think deeply about this and consider the tradeoffs, so we can make a well-reasoned decision.
- grafts can have `lock: true` to prevent expensive regenerations. this would be useful for point in time explorations that you want to keep around.
