---
status: draft
---
# runs/1: runs, for other programs

A run's record and its log, as pot reads them back. `pot runs` lists runs and follows one, and `pot review` uses the
same records for the runs that need a person. workmux, Windmill jobs and, later, grove read this contract instead
of pot's state folder.

| | |
|---|---|
| Version | 1: the `version` of `pot runs --json` |
| Producer | pot, which writes each run's log on the host |
| Reader | `pot runs` and `pot review`; workmux, Windmill jobs, later grove |
| Owner | pot |

## Shape

```
pot runs [ID] [--json] [--follow] [--all]
```

### The run log

Each run's log, `log.jsonl`, is written by the host, one JSON object per line, and hidden from every run. **A record
is computed from the log alone:** there is no second record file, so a record always agrees with its events.

Every event has `ev` and `by`:

| `by` | Meaning |
|---|---|
| `host` | a fact the host observed |
| `parent` | v2.18.0 to v2.19.2: written by a nested parent's in-run pot. Not written since v2.20.0, and not read. |
| `run` | a claim the run reported: data, never evidence |

| `ev` | When | Keys |
|---|---|---|
| `start` | the run starts (also `pot host`) | `run`, `context`, `backend`, `argv`, `kits`, `invoker`, `time`; `mode`, `workspace`, `command`, `pid`, `boot`, `pidStart`, `pane`, `detached`, `parent`, `oneRun`, in clone mode `clone` (`{ base, branch, path, nested }`), and for a sibling (since v2.20.0) `caller` (the id of the run that asked) and `depth` (1 for a sibling of a top-level run) |
| `stop` | pot refused to start the run with exit 125 | `reason` (`grant` or `guarantee`), `kits` (each `{ id, status, digest, source }`: what was wanted), `fix` (such as `pot grant flexion-assistant`), `text` (the 125 lines), `workspace`, `time` |
| `net` | a network decision | `host`, `port`, `verdict`, `count`, `bytes_out`; a request the proxy could not read is `{ "host": null, "port": null, "verdict": "malformed" }` |
| `cap`, `notfound` | in the shape; not written yet | |
| `exit` | the command ended | `status`, `time` |
| `after` | the host's look at what the run left | `commits`, `unsigned`, `touches_host_run`, `repo_kit_change`, `root_repo_change`, `base`; `sealed_left` (since v2.19.1: what the run left in its sealed folders, `{ path, type, size }`, discarded) and `hidden_blanked` (since v2.19.1: hidden tracked paths a new commit holds empty); in clone mode also `agent_config` (since v2.19.1: the proposed agent configuration its commits change, links' targets and scions' paths included), `refs` (the clone's branches that moved: `{ "main": "e91d2c7…" }`), `scions` (`{ "<path>": { "commits": [ … ] } }`), `uncommitted`, `size` (bytes), `gitlinks`, `new_git_metadata`, `root_attention` and, when the host's staging fetch failed, `fetch_failed` (for each ref that failed, git's first `error:` line, else its first line; a scion's prefixed with its path), `refCommits` (`{ "<ref>": N }`, the commits of each moved ref); always `signs` (whether the host's git signs, which decides whether unsigned commits are shown); `recovered: true` when `pot runs` or `pot review` completed it after pot had ended |
| `fetch` | `pot review ID --fetch` | `refs` (`refs/remotes/pot-<id>/…`), `time` |
| `discard` | a clone was removed | `why`: `person` (`--discard`), `keepDays` or `maxSize`. `time`. `keepDays` and `maxSize` remove only a clone that was fetched, discarded, or has no commits: pot never removes unfetched commits on its own. |

The shapes of `start`, `net`, `cap`, `notfound`, `exit` and `after` are frozen (pot's design, Appendix A). The other
kinds, and the keys added to the frozen ones, are values that this version adds.

### States

Each record has one state:

| State | When |
|---|---|
| `running` | a `start`, no `exit` or `stop`, and its process alive: `pid` exists with the start time `pidStart`, on the boot `boot`. For a run started with `--detach`, the pid is the background pot's. |
| `exited` | an `exit`. Also a `start` whose process is gone with no `exit` (a crash, a reboot, `kill -9`): then `exit` is `null` and `lost` is `true`. A clone-mode run that was fetched or discarded is `exited` too, with `fetched` or `discarded` set. |
| `stopped` | a `stop`: exit 125, for a grant or a missing guarantee. The command never started; `stop.fix` says what unblocks it. When every kit it waited for now has a grant of that digest and source on this machine, `stop.grantedSince` is `true`: start it again. A `stop` written before v2.17.0's fix round (no `digest`) is never granted since. |
| `review` | clone mode, ended, with commits (its own or a scion's), neither fetched nor discarded; or ended with a failed staging fetch (`fetch_failed`), which `keepDays` and `maxSize` never remove. |
| `pending` | reserved for a request waiting for approval (phase 3). Not written. |

When pot finds a clone-mode run that ended without its `after` event (its pot was killed), the next `pot runs` or
`pot review` that reads the run completes the host's look first, as the run's end does, records it with
`recovered: true`, then shows it: a clone with commits is `review`, as for any run that ended. One pot recovers a run at
a time (`runs/<id>/recover.lock`): another reader waits for it, up to 15 seconds, then reads the log again; while the
lock is still held, the record is `running` with `recovering: true`.

### The record

`pot runs ID --json` prints one record. `pot runs --json` prints `{ "version": 1, "runs": [ record, … ] }`.

```json
{ "version": 1, "id": "9b0e", "state": "review", "context": "dispatch", "mode": "clone",
  "command": { "name": "claude", "kit": "root:claude-code@flexion", "instance": "flexion",
               "typed": [ "claude@flexion", "-p", "migrate tasks to schema v3" ] },
  "argv": [ "claude", "-p", "migrate tasks to schema v3" ],
  "workspace": [ { "id": "repo:platform-vm:2222/daniel/flexion-assistant", "path": "/home/daniel/src/flexion-assistant", "name": "flexion-assistant" } ],
  "invoker": "person", "detached": true, "pane": null, "parent": null, "children": [ ],
  "started": "2026-11-17T14:02:11", "ended": "2026-11-17T14:31:40", "exit": 0, "lost": false,
  "stop": null,
  "clone": { "base": "c9996c5…", "branch": "main", "commits": 3, "unsigned": 1, "refs": { "main": "e91d2c7…" },
             "uncommitted": 0, "size": 201326592, "fetched": null, "discarded": null, "keptUntil": "2026-12-01" },
  "attention": [ "1 unsigned", "host-run: justfile", "2 denied: registry.npmjs.org" ] }
```

| Field | Type | Meaning |
|---|---|---|
| `version` | number | `1` |
| `id` | string | the run's id, four hex digits |
| `state` | string | one of the [states](#states) |
| `context`, `mode` | strings | the context; `in-place`, `clone`, or `none` for `pot host` |
| `command` | object | `name`, the command kit's identity `kit` (or `null`), `instance` (or `null`), and `typed`, the command as the person typed it. Text shows `typed`. (A log from before the field: `command` is `null`.) |
| `argv` | list of strings | the command as pot started it: the kit's program and arguments |
| `workspace` | list | each repository's identity `id`, `path`, and `name`, the short name the startup line uses: the normalised origin's last path part, else the checkout's folder |
| `invoker` | string | `person` (at a terminal), `script`, or `run` for a sibling (a nested child until v2.19.2) |
| `via` | string, or absent | `"wrapper"` for a start through a [wrapper](pot-kits-1.md#shape) on the host (`pot run`), `"sibling"` for one a run asked for (since v2.20.0); `"dispatch"` was a nested dispatch (v2.18.0 to v2.19.2). Its `start` event has `via`, and `hook` when the command declares one. `pot runs` hides such runs that exited 0 unless `--all` |
| `detached` | boolean | started with `--detach` |
| `pane` | string or `null` | the tmux pane the run held |
| `parent`, `children` | `null`; `[ ]` | since v2.20.0 always: a run starts no child inside it (decision 20). v2.18.0 to v2.19.2 listed nested children here, as `PARENT/CHILD`; their folders are not read any more. A sibling names the run that asked in its `start` event's `caller`. |
| `started`, `ended` | strings or `null` | local times |
| `exit`, `lost` | number or `null`; boolean | the exit status; whether pot ended without one |
| `killed` | `"caller"`, or absent | since v2.20.0: a sibling whose request went away before it started (its `exit` is 143). `"parent"` (v2.18.0 to v2.19.2) is not written any more |
| `stop` | object or `null` | for `stopped`: `{ reason, kits, fix, grantedSince }` |
| `recovering` | `true`, or absent | another pot is completing the run's staging (its `recover.lock`), still after a reader's 15-second wait: the state is `running` until it ends |
| `clone` | object or `null` | in clone mode: the `base` commit and `branch`, `commits` and `unsigned`, `scionCommits`, `nested` (the scions whose own submodules were not cloned), `fetchFailed` (the after event's `fetch_failed`, else `null`), the moved `refs`, `uncommitted` files left in the clone, its `size`, when it was `fetched` or `discarded`, `keptUntil`, and, since v2.19.1, `agentConfig` (the after event's `agent_config`; `[ ]` when none) |
| `attention` | list of strings | what the host's look found, as `pot review` shows it: `N unsigned` (when the host signs, until fetched), `root: …` or `root changed`, `host-run: …`, `repo kit: …`, `gitlinks: …`, `N new git metadata paths`, `N denied: …`, `fetch failed: …`. Never a bare commit count. |

Every string from the run (`argv`, ref names) is JSON in `--json`, and escaped in text.

### --follow

- `pot runs ID --follow` prints the run's events as they are appended, until `exit`, `after` or `stop`, and exits
  with the run's status: 125 when it stopped, 1 when it was lost, else the command's. With `--json` it prints the log's own
  lines, verbatim.
- `pot runs --follow` prints the list, then one line for each change of any run's state, until interrupted.

Both read the logs again every half second.

### Inside a run

`pot runs` inside a run shows that run, from its policy (`id`, `state` (`running`), `context`, `mode` and
`workspace`; the other fields are `null`), alone. The host's logs stay hidden, its siblings' too.

## Compatibility

- **Values are added, never fields,** within version 1: new event kinds, new keys in an event, new `attention`
  strings. A reader ignores what it does not know, and treats a state it does not know as one it does not act on.
- **`by` decides trust.** Only `by: host` lines are evidence. `by: run` lines are claims (and v2.18.0 to v2.19.2's `by: parent` lines).
- **The state folder is not the contract.** A reader uses `pot runs --json`. The layout of `runs/<id>/` is pot's
  own, and may change in any release.

## Sources

- [pot/core/runs.py](../core/runs.py): a record from a run's log, the states
- [pot/core/pot.py](../core/pot.py): the run log, `pot runs`, `pot review`
- [pot's reference: State](../../docs/reference/pot.md#state)
