---
status: working
---
# run-env/1: what a program sees inside a run

| | |
|---|---|
| Version | 1: `policy.json`'s `version` |
| Producer | pot's backends (bubblewrap), from the policy that pot composed |
| Reader | graft (`graft check`), kit hooks (the claude-code kit's denial hook), and any program in a run |
| Owner | pot |

## Shape

### Variables

| Variable | Value |
|---|---|
| `POT` | The context's name, such as `work`. Set in every run: test it with `[ -n "${POT:-}" ]`. |
| `POT_POLICY` | `/run/pot/policy.json` |
| `POT_TOOLS` | The `bin/` of the workspace's [pot-kits](pot-kits-1.md) bundle, when the workspace has a repo kit. |
| `POT_CLONE` | `1` in clone mode (since v2.17.0); unset in place. |

The rest of a run's environment is described in pot's reference
([Environment](../../docs/reference/pot.md#environment)); it is not part of this contract.

### Files

| Path | Contents |
|---|---|
| `/run/pot/policy.json` | The run's policy ([below](#policyjson-version-1)), read-only. |
| `/run/pot/launcher` | The root's in-run pot, read-only; a wrapper inside the run execs it (under Seatbelt, `launcher` beside `POT_POLICY`). `~/.config/pot/launcher` is a link to it, in a read-only folder. Since v2.18.0. |
| `/run/pot/table/<digest>/` | Each child policy in the table (`policy.json`, its closure), read-only; a nested row's state folders under `state/`. Since v2.18.0. |
| `/run/pot/children/` | The parent's children folder, read-write in that parent only. Since v2.18.0. |
| `/run/pot/denials` | One entry for each refused host and port, appended as it happens, read-only to the run: a line `pot (<context>): the run tried HOST:PORT (not in this run).`, then indented lines `  one run:   pot --net HOST …` and `  permanent: <fix>` (one more `             or <fix>` line for each other fix). Since v2.16.1; v2.16.0 wrote one line per entry. Read it as text: its layout is not part of this contract's shape. |

### policy.json, version 1

The shape is frozen. Its top-level fields never change. Within version 1, a release adds values, and may add an
optional key to an item, never remove or rename one.

```json
{
  "version": 1, "run": "7f3a", "context": "work", "mode": "in-place", "access": "write",
  "backend": { "name": "bubblewrap", "requires": [], "proved": [ "closure-store", "own-netns", "…" ] },
  "workspace": [ { "id": "repo:…", "path": "/home/…", "common": "…/.git", "protected": [ "…" ], "clone": null } ],
  "items": [ { "kind": "network", "value": "api.anthropic.com:443", "class": "reach",
               "sources": [ { "kit": "root:claude-code@personal", "file": "…", "as": "request" },
                            { "grant": "grants.json", "digest": "sha256-…" } ], "effective": true } ],
  "mounts": { "state": [ { "name": "claude", "at": "~/.claude", "key": [ "…" ] } ], "run": "/run/pot" },
  "bindings": [ { "capability": "status.relay", "socket": "/run/pot/relay.sock", "see": [ "workmux shim" ] } ],
  "children": { },
  "grants": [ { "id": "root:claude-code@personal", "digest": "sha256-…", "match": true } ]
}
```

| Field | Type | Meaning |
|---|---|---|
| `version` | number | `1` |
| `run` | string | the run's id, four hex digits |
| `context`, `mode`, `access` | strings | the context; `in-place` or `clone`; `read` or `write` |
| `backend` | object | its `name`, the guarantees the context `requires`, and those it `proved` for this run (`resource-limits` only for a run that gets the resource scope; `nest` where the backend can nest) |
| `workspace` | list | each repository: `id` (a [kit identity](kit-1.md#identity)), `path` (the original path, where the run sees it), `common`, `protected`, `sealed` (since v2.19.1: the protected patterns whose folders are sealed under bubblewrap), `clone` (`null` in place; in clone mode, the clone's host path in pot's state, and `common` is the clone's `.git`) |
| `items` | list | every see, limits and reach item: `kind`, `value`, `class` (`see`, `limits` or `reach`), `sources`, `effective`; a file a mirror dir binds also has `memory: true` (as in v2.15.0) and, from v2.16.0, `mirror` (the dir's label, such as `"memory"`) |
| `mounts` | object | the state directories, each with its scope `key`, the run directory, and, since v2.18.0, `launcher` (`"/run/pot/launcher"`, the run's own pot, bound read-only, by which a wrapper tells a v2.18.0 run) and, in a nested child, `parent` (the parent's id) |
| `bindings` | list | each bound capability, its socket, and the see items it adds |
| `children` | object | the run's children table: for each command the run may start, keyed `<kit id>:<command>` (or `dispatch <kit id>:<command>`), `{ slot, command, kit, kitName, context, hook?, how: "exec" \| "nest", policy: "sha256-…", argv, env, table, children }` (a `nest` row's `children` is its sub-table), or `{ …, refused: "<why>", text?, grant? }`. Empty in v2.17.x |
| `grants` | list | each kit's grant `id`, `digest`, and whether it `match`ed |

An item's `sources` say where it came from. Each is one of these objects:

| Source | Meaning |
|---|---|
| `{ "kit": ID, "file": F, "as": "see" \| "limit" \| "request" }` | a kit, as its `file` names where it is written |
| `{ "grant": "grants.json", "digest": D }` | the grant that made a request effective |
| `{ "flag": "--net" }` (and `--read`, `--write`, `--env`, `--package`) | a one-run flag |
| `{ "context": NAME }` | the context: its access, network, included kits, `inspect`'s view |
| `{ "binding": "mcp.<selector>" }` | a binding that adds reach (an MCP server's host) |
| `{ "root": "default" }` | pot's own defaults: hidden paths, protected paths; with `"why": "the flake directory pot evaluates"` for that folder |
| `{ "root": "bundle", "why": "the repository's pot-kits" }` | a path the bundle's `protect` names |
| `{ "root": R }`, R one of `standard`, `tools`, `inspect`, `base`, `caBundle`, `hidden`, `pot`, `binding` | the root: its standard set, its tools, `inspect`'s tools, `TZ`, its CA bundle, its hidden paths, pot's own state and grants file, a binding's key file |

An item may carry keys for its kind, such as `mode`, `source`, `paths`, `via` or `why`. v2.16.0 adds one: a file
that a [`dirs` mirror](kit-1.md#dirs) puts in the run has `"mirror": "<label>"`, such as `"mirror": "memory"`. It
keeps v2.15.0's `"memory": true` on the same items, so a v2.15.0 reader still finds them.

v2.16.0 changes one value: pot's defaults were `{ "root": "graft" }` in v2.15.0, and are `{ "root": "default" }`.
The source `{ "root": "graft", "why": "the scions directory" }` is gone: the scions folder now comes from the
bundle (`{ "root": "bundle", … }`).

### pot why TARGET --json

The supported question, so that a program need not read `policy.json`'s items. Inside a run it answers for that
run. On the host it answers from the policy a run of the default context would have there, never from the host's
own view: a path the host has but such a run would not see (hidden, or outside the workspace and its reach) is not
reached. `--json` may come before or after the target.

```
pot why TARGET --json      # TARGET: HOST, HOST:PORT, a git URL, or a path
```

```json
{ "version": 1, "target": "https://github.com/danielnaab/meeting-kit", "kind": "network",
  "reach": "github.com:443", "reached": true, "context": "work", "text": "…" }
```

| Field | Type | Meaning |
|---|---|---|
| `version` | number | `1` |
| `target` | string | the target, as given |
| `kind` | string | `network`, `git.fetch` or `file` |
| `reach` | string | what was asked: `host:port`, a normalised remote key (`platform-vm:2222/daniel/graft`), or a path |
| `reached` | boolean | whether the run reaches it |
| `context` | string | the context answered for |
| `text` | string | `pot why`'s answer in words, as it prints it: a refusal has several lines, joined with `\n` (`pot (work): pot would refuse example.com:443 in a run here.\n  one run:   pot --net example.com …\n  permanent: …`); a file-like word that names no path, `flake.nix: no such path here (pot why PATH answers paths; for a host, pot why flake.nix:443)`, with `kind` `file` and `reached` false |

| Target | `kind` | Reached when |
|---|---|---|
| `HOST` (port 443), `HOST:PORT`; an `https://` (443), `http://` (80) or `git://` (9418) URL | `network` | an effective network reach item matches it, wildcards included |
| an `ssh://` or `git+ssh://` URL, or the scp form `user@host:path` | `git.fetch` | a `git.fetch` binding in the policy has a remote pattern that covers it |
| a `file://` URL, or a path (absolute, `~/…`, relative with a `/`, or a name that exists here) | `file` | the run sees it: it is in a workspace (protected paths included: they are readable), granted, generated by pot, in the store or under `/run/pot`. Not reached when it does not exist, is hidden, is not in the run, or is requested but not granted. `reach` is the path as given (`/etc/hosts`, `~/.ssh/id`); `reached` comes from the path it resolves to. |

| Exit | Meaning |
|---|---|
| 0 | reached |
| 1 | not reached |
| 2 | not a target this asks about (a command, a root kit's command even when a path has its name, a variable): `pot why --json answers a host, HOST:PORT, a git URL or a path; X is none (pot why X explains it)` |

`graft check` asks this, inside a run, before it lists a scion's tags, so that a fetch the run cannot make leaves no
refusal behind. When the answer has no boolean `reached` (an older pot, or no `pot` on `PATH`), it tries the
fetch.

## Compatibility

- **Readers ignore what they do not know:** a new guarantee, item kind, source or capability name, and an item
  key that a later release adds. They may rely on the top-level fields, which never change in version 1.
- **Variables are added, never renamed,** within version 1. v2.14.0's `GRAFT_SANDBOX*` names are not set, and no
  kit may set them.
- A program that wants to know whether it is in a run tests `POT`. One that wants to know whether something is
  reachable asks `pot why --json`. Neither reads pot's state folder.

## Sources

- [pot/core/pot.py](../core/pot.py): the run's environment, `why_json`, `reach_of`
- [pot/core/compose.py](../core/compose.py): the policy and its sources
- [lib/common.sh](../../lib/common.sh): `pot_reaches`, `graft check`'s question
- [pot's reference: policy.json](../../docs/reference/pot.md#policyjson)
