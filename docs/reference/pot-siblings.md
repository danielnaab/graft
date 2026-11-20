---
status: working
---
# pot: siblings, not nesting (the v2.20.0 design)

The design behind [Siblings and the children table](pot.md#siblings-and-the-children-table) (design §10, revised
in v2.20.0). It replaces nesting on both backends: a narrower child is never started inside its caller; the host
starts it beside the caller, as a **sibling**, under its own policy only. Daniel's decisions 11-22 (2026-11-19)
fix the rules; this note fixes the mechanism, and lists what a review has to check.

## Why

- **One trust rule (decision 11).** Every kit command runs only under its own grant, never with a caller's
  authority. Under nesting a parent held everything its children got (residual 23): their closures, state and
  credentials. A sibling holds nothing of its caller's, and its caller holds nothing of the sibling's.
- **One mechanism on every backend (13).** Seatbelt cannot nest, so on macOS a narrower child was refused, and
  `git commit` inside `pot claude` stopped at a wrapped pre-commit hook. A sibling needs only a host process that
  starts runs, which both backends have.
- **No user namespaces in any run (13).** Nesting kept user namespaces open in every agent run (residual 1).
  Every run now gets `--disable-userns`.

## The launch seam

```
in the run                          on the host
----------                          -----------
wrapper -> pot run --command C      the run's broker (siblings.py, started by the backend with the run's other
  (the in-run pot: the client)        bindings): listens on <run dir>/children.sock
  1. looks C up in the table        3. checks the request (size, time, fd types, the row), takes a tree slot
  2. connects to children.sock,     4. starts the host entry in a new session: <root's pot> run --sibling,
     sends the request and fds 0-2     with fds 0-2 the caller's and the request on fd 3
  5. blocks; ^C -> INT              5. the entry composes the sibling now, admits it against the caller's policy,
  6. exits with the sibling's code     and runs it as any wrapped start does (prepare, run, after, cleanup)
```

- **The client** is the in-run `pot run --command NAME --manifest M -- ARGS`, the wrappers' entry (pot-kits/1,
  unchanged). It finds its row by the manifest's slot and command in `/run/pot/policy.json`'s `children`:
  - `exec`: the row's policy is identical to the run's (decision 17): it execs the program in place, as before.
  - `sibling`: it connects to `children.sock` beside `POT_POLICY` (`/run/pot/children.sock` under bubblewrap,
    `<run dir>/children.sock` under Seatbelt), sends one JSON line (`v`, `key`, `args`, `cwd`, and for a hook row
    the caller's `GIT_INDEX_FILE` and `GIT_PREFIX`), with fds 0, 1 and 2 by `SCM_RIGHTS`, and blocks. The answer is
    `EXIT n` (the sibling's status), or `ERR n TEXT` (a refusal: TEXT on stderr, exit n).
  - `refused`: the row's text, exit 1 (125 for a grant), as before.
  - **No row: exit 126** (decision 16): `pot (work): C: not in this run's children table (a wrapper from another
    workspace?): run it on the host`. A run never runs a command under its own policy because it has no entry.
- **fds 0-2 cross only as what a sibling cannot reopen into more.** A terminal, a socket or `/dev/null` crosses as
  it is. The spike showed why: a directory fd passed as stdin let the child read a file its own view hides
  (`/proc/self/fd/0/…`); and the review found that a pipe, reopened the other way round through `/proc`, let a
  sibling write into its caller's input. The client therefore puts a pipe of its own in front of any other kind (a
  pipe or FIFO, a regular file, as in `check-kb < notes.md` or `> out.txt`) and copies, so the caller's authority
  over it stays the caller's; the broker takes a pipe too (the client's). The cost: a sibling's piped input is read
  ahead, so what it leaves unread is not left for its caller. A directory or a closed fd is `/dev/null`.
- **The terminal is passed, not given.** The sibling is in a session of its own: the caller's terminal is not its
  controlling terminal, so it has no job control (decision 14: non-interactive), and `^C` reaches only the caller's
  foreground group, where the client turns it into `INT` on the socket. The broker sends SIGINT to the sibling's
  session; the entry and the backend ignore it, and the command's SIGINT is the default again (`env
  --default-signal=INT`, as in every run). Under Seatbelt, whose command is its own process group, the backend
  forwards it. A sibling cannot type into the terminal: bubblewrap's seccomp filter and Seatbelt's `no-tiocsti`
  apply to it as to any run.
- **Cancellation is the socket closing** (decision 14): the client killed, the agent's tool timeout (it kills the
  client's process tree), or the client's SIGTERM or SIGHUP (it closes the socket and exits 143 or 129). The broker
  then sends SIGTERM to the sibling's session, again each second (so one that lands while the sibling is starting
  is not lost), for up to 10 s (longer than the sibling's own cleanup), and then SIGKILL. Under Seatbelt the command
  is its own process group, so the backend passes SIGTERM on to it (replaying one that came before the command
  started) and sends it SIGKILL 3 s later itself. A sibling still being composed is not started. **When the run ends** the backend stops
  its broker (waiting 12 s for it), which stops every sibling the same way first and starts none after. An entry
  whose broker died stops its own session the same way. The tree's slot is held by the broker and by the entry, so
  it stays taken while either lives. A sibling never outlives the request that started it. **A caller's host pot
  killed** (outside a sibling, say by SIGTERM) runs no cleanup of its own: its backend runs on until the command
  ends and stops the broker then, and the next start removes the run's `children.sock` and `<state>/trees/<id>`
  (v2.20.1; [Protected paths](pot.md#protected-paths) on a lost run).
- **The host entry** is `pot run --sibling`, the root's own pot (`launch.json`'s `potBin`, fixed at the run's
  start). It reads the request from fd 3, never from a file a run could reach. Inside a run it is refused.

## Admission

The entry composes the sibling **at its start**, from host facts, as `pot <context> KIT:C` would start there, with
the caller's context and the caller's workspace (from the top of the caller's repository that holds the cwd, with
its other repositories as `--repo`, never from the cwd itself; in a clone, the caller's clone, at the original
path), and then
admits it only if every check below holds against the caller's policy, read from the caller's `policy.json` in its
run folder and checked against the digest the host recorded when the caller started.

1. **The row** is the caller's `sibling` row for that key (the table the host wrote at the caller's start).
2. **Its grant matches now.** A stale kit exits 125 with the usual message; a sibling never prompts.
3. **`narrower(sibling, caller)`** has no lack (decision 13; no union: the sibling's policy is its own row's):
   access, network, read, write, socket, env, capability (bindings covered: each of the sibling's
   `(capability, selector, request)` is one of the caller's), `unsandboxed` never.
4. **A write sibling keeps the caller's limits:** with `write` access, each `protect` pattern in effect for the
   caller is in effect for the sibling too, else the lack is `protect PATTERN`. (Under nesting the child stood in
   its parent's read-only mounts; a sibling gets only its own.)
5. **The same mode and workspace:** the sibling's mode is the caller's, and its repositories are among the
   caller's.
6. **Depth:** the caller's depth plus one is at most **8** (the safety cap, decision 18): `pot (work): C: this
   run is 8 starts deep (pot's cap): run it on the host`.
7. **Concurrency:** a slot is free among the tree's **16** (decision 18, tree-wide): `pot (work): C: 16 commands
   already run in this run's tree (pot's cap): wait for one to end, or run it on the host`. The slots are
   `flock`ed files in `<state>/trees/<top run>/`, outside every run folder; a slot frees when its holder ends,
   however it ends.

**Strict narrowing per hop (18).** An identical policy never becomes a hop (it execs in place, 17), so a hop is
narrower in reach than its caller or equal in reach and different elsewhere (state, which stays outside the ⊆
test, or more `protect`). Chains of the second kind are finite only by the depth cap, which is why the cap is a
safety cap and not a tuning knob.

**The table is composed per run, at its start (18).** The caller's table lists the rows it may start, composed by
the host at the caller's start; a sibling's own table is composed at the sibling's start, against the sibling's
policy. There are no sub-tables, and nothing about a row is decided inside a run.

**A tool's reach (15)** is its own grant plus the repo kit's, never the context's `include` (standing reach, such as
a personal kit's `git.fetch`). This holds for every wrapped start, on the host too. It changes no grant digest
(grants are per kit), so nothing re-asks.

## The review list: what nesting covered implicitly

Nesting gave a child its parent's limits for free, because it stood inside them. A sibling stands in its own
policy alone, so each dimension below needs its own reason, and a test.

| Dimension | Under nesting | As a sibling | Test |
|---|---|---|---|
| Filesystem view | the parent's view, narrowed again | its own view, composed from host paths; reads ⊆ the caller's (narrower) | a read-only sibling cannot write; its reads are its own |
| Write access | the parent's read-only mounts held | access ≤ the caller's; a write sibling keeps every caller `protect` (check 4) | write lacks refused; protect lack refused |
| Hidden paths | inherited | the same root and context: the same list | — |
| Network | a proxy inside the parent, upstream the parent's | its own proxy, started by the host, allow list ⊆ the caller's | a host only the caller reaches: 403 |
| Bindings | equal to the parent's; the parent's sockets | covered by the caller's; its own broker and sockets | a binding the caller lacks: refused |
| Credentials | the parent held them (in its environment) | set by the host from the binding, only for a covered binding | the agent's token is not in the sibling |
| Sockets | the parent's | its own `socket` items ⊆ the caller's | — |
| Processes | the parent's pid namespace | its own pid namespace (bubblewrap); Seatbelt shares the user's processes (residual 22) | the caller's processes are not visible |
| Environment | the parent's run env, its own items | its own run env; from the caller only the hook's two variables, checked as paths | the caller's variables do not cross |
| fds | inherited | 0-2 only, as a terminal, pipe, socket or `/dev/null` | a directory fd is refused |
| Terminal | the parent's | passed, not controlling; no TIOCSTI | — |
| State | bound into the parent (residual 23) | the sibling's alone | the caller cannot see the sibling's state |
| Closure | bound into the parent | the sibling's alone | — |
| User namespaces | kept in a parent with a `nest` row | `--disable-userns` everywhere | `unshare -U` fails in every run |
| The cwd | the parent's, inside its view | the caller's, checked under a workspace top (on the host, under the clone for a clone), else the top | a cwd outside: the top |
| Lifetime | the parent's pid namespace | the request: the socket closing stops it | `^C`, kill, timeout, the run's end |
| Floods | — | header 256 KiB and 5 s; 16 slots per tree; depth 8 | 17 at once: one refused |
| Records | parent-written claims (`by: parent`) | the host writes every line (`by: host`); `caller` names the run that asked | — |

## The deletions

- **Nested bubblewrap** and the `nest` row; `nest_child`, `inner_clone`, the table folder (`runs/<id>/table/`,
  `/run/pot/table/`), the children folder (`runs/<id>/children/`, `/run/pot/children/`), the parent-bound closures
  and state (`write_table`), the nested proxy (`proxy.py --upstream`, `--nested`), and the nested branches of the
  backends.
- **User namespaces:** `--disable-userns` in every run; no backend proves `nest` (the name stays a known
  guarantee, so a root that names it still parses, and a context that requires it is refused as for any missing
  guarantee).
- **Nested dispatch and nested `--detach`** (decision 19): `pot dispatch`, `--clone` and `--detach` inside a run are
  refused: `pot (work): pot dispatch is the host's: a run starts no clone inside it; run it on the host`. Dispatch
  rows go from the table.
- **The parent-claims child records** (decision 20): `read_claims`, `child_records`, `children_after`,
  `finish_children`, `prune_children`, `PARENT/CHILD` ids in `pot runs` and `pot review` on the host, and
  `pot review CHILD --json` inside a run. runs/1's `parent` and `children` stay, `null` and `[ ]`. Folders an older
  pot left under `runs/<id>/children/` are not read (no legacy path); they go with their parent's folder.
- **Residual 23** (a parent holds everything its children get), and residual 24's `by: parent`.

## Contracts and docs

- `kit/1`, `pot-kits/1`, `grants/1`: unchanged (the split gate keeps counting).
- `run-env/1` stays version 1, as runs/1 does (decision 20): `children` keeps its key; `how` adds `sibling` and no
  longer has `nest`; `/run/pot/children.sock` is added; `/run/pot/table/` and `/run/pot/children/` are no longer
  bound (only pot's own in-run pot read them); `mounts.parent` is no longer written.
- `runs/1` stays version 1: `via` adds `"sibling"`, `invoker` is `"run"` for one, and its `start` event adds
  `caller` (the run that asked, host-written) and `depth`; its `exit` event's `killed` adds `"caller"` (the request
  went away before the sibling started). A sibling stopped while it ran exits with its command's status (143 for
  SIGTERM).
- Behaviour and docs change in the same commit: pot.md (§10 becomes *Siblings and the children table*; Nested
  dispatch goes; Wrapped commands, Hook commands, Inside a run, Guarantees, On macOS, Not yet, Residual risks), the
  dispatch, kit-commands and run-in-a-sandbox guides, decisions.md, README.

## Residuals

- **Decision 22 (the worker):** the Windmill worker runs kit commands unconfined until it can sandbox (phase 2),
  as decision 7 said.
- **No access to the caller's TMPDIR** (decision 14): a sibling's `/tmp` is its own, so a tool handed a path under
  the caller's `/tmp` does not find it. A shared request folder comes only when a real tool needs one.
- **The per-hop cost:** a sibling is a host start plus about 45 ms (measured on platform-vm: 0.34 s against 0.30 s
  for the same command started on the host, in tests/run.sh's fixture; a host start of flexion-assistant's
  `check-tasks` takes 0.50 s), against about 0.2 s for a nested child. A commit whose hook runs two checks pays it
  twice.
- **macOS (residual 22):** the caller shares the user's processes with its sibling, as with every process: it can
  read a sibling's arguments and environment. Admission makes that nothing new (the sibling's variables and
  credentials are covered by the caller's); a sibling's state is the one thing of its own the caller could reach
  through it there.

## Sources

- Decisions 11-22: the flexion-assistant task `pot-1b`, "Decisions for v2.20.0" (Daniel, 2026-11-19).
- The spike (bubblewrap, 2026-11-19, throwaway): fds 0-2 by `SCM_RIGHTS` into a child in its own session; its exit
  code; `^C` at a pty forwarded; the child stopped when the client was killed, when the caller's sandbox was killed
  and when the server ended; a directory fd's escape (`/proc/self/fd/0/…`). Recorded in the flexion-assistant
  session record `2026-11-19-pot-v2-20-siblings`.
- [pot/core/siblings.py](../../pot/core/siblings.py), [pot/core/pot.py](../../pot/core/pot.py) (`inner_run`,
  `sibling_client`, `sibling_entry`), [pot/core/compose.py](../../pot/core/compose.py) (`children_table`,
  `narrower`, `admit`).
