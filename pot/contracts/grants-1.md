---
status: stable
---
# grants/1: the grants file


| | |
|---|---|
| Version | 1, canonical 1. Frozen. |
| Producer | `pot grant`, and the inline grant prompt |
| Reader | pot, at every run start, on each machine that shares the file |
| Owner | pot |

One file per root, the root's `grantsFile` (default `~/.config/pot/grants.json`). A home-manager root tracks it in
its repository and shares it across machines through git, so machines on different pots read one file.

## Shape

```json
{
  "version": 1, "canonical": 1,
  "grants": {
    "<identity>": { "digest": "sha256-…", "requests": [ "<canonical request>", … ],
                    "source": { … }, "granted": "YYYY-MM-DD <machine>", "delegate": { … } }
  },
  "machines": { "<machine>": { "<identity>": { … } } }
}
```

| Field | Type | Meaning |
|---|---|---|
| `version` | number | `1` |
| `canonical` | number | `1`: the form of the request strings and paths that digests are computed over |
| `grants."<identity>"` | object | one kit's grant, shared by every machine. The identity is a [kit/1 identity](kit-1.md#identity). |
| `machines."<machine>"."<identity>"` | object | a grant for one machine (its short host name), which overrides the shared entry there. A repository with no remote is always here. |
| `…digest` | string | `sha256-` and the base64 of the SHA-256 of the sorted, unique canonical lines, each ended by `\n` |
| `…requests` | list of strings | the canonical requests the grant covers |
| `…source` | object, optional | for powerful items only: a scion kit's `{ "commit": "…", "tag": "vX.Y.Z" }`; a repo kit's `{ "tree": "<git tree id>" }` (`git write-tree --prefix=<flake dir>/` from a copy of the index). None for a root kit. |
| `…granted` | string | `YYYY-MM-DD <machine>` |
| `…delegate` | object, optional | reserved for delegation (later) |

The file is written sorted, with a 2-space indent.

### Canonical requests

`read ~/x`, `write ~/x`, `socket ~/x`, `network host:port` (a bare host is `:443`, lowercased), `env NAME`,
`state NAME` or `state NAME shared`, `unsandboxed`, and `capability NAME [KEY] [field=value …]`. The key is the
capability's remote, host or server; the other fields are sorted, lists joined with commas:
`capability credential api.anthropic.com env=CLAUDE_CODE_OAUTH_TOKEN`. A path under the user's home is `~/…`.
Identities and requests hold no store paths, no system and no absolute home paths, so the file is
machine-independent.

## Compatibility

- **Every reader refuses what it does not know.** A file that does not parse, a `version` or `canonical` other than
  1, an unknown top-level field, an entry field other than the five above, or a digest that does not match its
  requests: every run stops, with a message ending `every run stops until it is fixed`. `pot grant` alone reads
  digest mismatches leniently, setting such entries aside, so it can repair the file.
- **Frozen.** v2.16.0 changes no canonical string and no digest. The flake check `pot-golden` holds the
  canonical items and digests of the claude-code instances, a standing kit and a repo kit, captured from v2.15.0,
  and fails when one changes.
- **A change to the canonical form is canonical 2**, which every older pot refuses, stopping its runs. So the
  machines sharing a file upgrade pot together, before any of them writes canonical 2.
- **A changed source is not a changed request.** A repo kit's source is the tree of its flake directory, so any
  committed or staged change there (a new `flake.lock`, say) asks for one new grant of its powerful items, with
  the same requests. Taking graft v2.16.0 is such a change: flexion-assistant asks for one
  `pot grant flexion-assistant`.

## Sources

- [pot/core/compose.py](../core/compose.py): `parse_grants`, `digest`, `write_grant`
- [pot/tests/golden.json](../tests/golden.json): the golden canonical items and digests
- [pot's reference: The grants file](../../docs/reference/pot.md#the-grants-file)
