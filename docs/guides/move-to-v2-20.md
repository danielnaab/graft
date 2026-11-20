---
status: working
---
# Move to v2.20.0

graft v2.20.0 changes how pot starts a kit command inside a run: no longer nested in the run, but **beside it**, as
a sibling the host starts under the command's own policy, on Linux and macOS alike
([Siblings and the children table](../reference/pot.md#siblings-and-the-children-table)). It removes nested
`pot dispatch` and nested `--detach`, and the records of nested children. This page says what changes for you and
who changes nothing.

## When to use this

- You have a root (`homeManagerModules.pot` or `graft.lib.mkRoot`) on graft v2.19.x, or a consumer whose kit
  commands run inside `pot claude`.

## Who changes nothing

- **Your grants.** Grants are per kit, so nothing asks again.
- **Your consumers and providers.** `kit/1` and `pot-kits/1` are unchanged: the wrappers a consumer's tools hold
  call the root's pot as before, and the root's pot decides what happens. A consumer moves its graft when it
  chooses.
- **Wrapped commands that request what they need.** They run as before, on the host and inside a run.

## What changes

| Before (v2.19.x) | v2.20.0 |
|---|---|
| Inside `pot claude`, a narrower kit command nested in the run (Linux), or was refused (macOS: `Seatbelt cannot nest`) | It is a sibling, on both backends: the agent waits for it, gets its exit code, and `^C` reaches it. On a Mac, `git commit` inside `pot claude` runs the pre-commit hook |
| A wrapped command got the context's included kits (`work`'s standing reach, such as a personal kit's `git.fetch`) | It gets its own kit's grant and the repo kit's reach only, on the host too. A kit command that relied on standing reach now needs that reach in its own kit's requests |
| A wrapper with no row in the run's table ran under the run's policy | Refused, exit 126: `pot (work): C: not in this run's children table (a wrapper from another workspace?): run it on the host` |
| `pot dispatch`, `--clone` and `--detach` inside a run started a nested child | Refused: `pot (work): pot dispatch is the host's: a run starts no clone inside it; run it on the host`. Run `pot dispatch` on the host |
| `pot runs` and `pot review` listed nested children as `PARENT/CHILD` | Not listed. Folders under `runs/<id>/children/` that an older pot left are not read, and go with their parent's folder |
| An agent run kept user namespaces | Every run gets `--disable-userns` |

A sibling is non-interactive (no job control), stops when the command that started it goes, and has a `/tmp` of its
own: don't hand a kit command a temporary file of the agent's.

## Check

- `pot explain claude` in a consumer: the `children` row lists the kit's commands as `sibling` (or `exec`).
- `git commit` inside `pot claude` runs the hook command; a failing one aborts the commit.
- `pot runs --all` lists each sibling with its command, like any wrapped start.

## Sources

- [siblings, not nesting](../reference/pot-siblings.md): the design, its review list and its residuals.
- [CHANGELOG.md](../../CHANGELOG.md): v2.20.0.
