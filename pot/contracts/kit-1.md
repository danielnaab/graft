---
status: working
---
# kit/1: one kit

| | |
|---|---|
| Version | 1 |
| Producer | a provider's `graft.nix` (`kit`), a consumer's mkFlake `perSystem.kit`, a root's `programs.pot.kits.<name>` and `repoKits."<origin>"` |
| Reader | pot: [kitsFor](pot-lib-1.md) when it writes a bundle, mkRoot when it builds a root, and pot at run start |
| Owner | pot |

A kit is what a repository or a root ships: tools, commands, limits and requests for reach. Its identity is not
part of it: pot establishes that from host facts ([Identity](#identity)).

## Shape

```nix
kit = {
  # see
  packages = [ ];
  env.set = { };
  files."~/.claude/settings.json" = { source = <store path>; mode = "fixed"; };   # fixed | seed | reassert
  # limits
  protect = [ ];
  # reach: literal requests, inert until granted
  requests = {
    read = [ ]; write = [ ]; socket = [ ];
    network = [ "api.anthropic.com" ];
    env = [ "WM_TOKEN" { name = "AWS_SESSION_TOKEN"; secret = true; } ];
    state = [ { name = "claude"; at = "~/.claude"; } ];
    capabilities = [ { name = "git.fetch"; remote = "origin"; } ];
    unsandboxed = false;
  };
  commands.<name> = { program; args; access; requests; description; settings; … };
  recipes.<name> = "<command> <args>";
};
```

| Field | Type | Meaning |
|---|---|---|
| `packages` | list of packages | On `PATH`. A repo kit's packages reach every run in its workspace; a scion kit's do not. |
| `env.set` | attrs of strings | Fixed values, for the kit's own commands. Names that pot owns are refused. |
| `files."<path>"` | `{ source; mode; keys; }` | A file in the run, at `~/…` or `/…`; `/…` needs the `root-files` guarantee. |
| `protect` | list of strings | Workspace paths, read-only inside: relative to the repository's top, or `**/NAME`. |
| `requests.<kind>` | per kind | Requests, inert until granted ([Requests](#requests)). |
| `commands.<name>` | attrs | A program the kit ships ([Commands](#commands)). |
| `recipes.<name>` | string | A just recipe body. Checked; not yet generated. |

The `files` modes: `fixed`, bound read-only; `seed`, copied into state on first use, then the run's own; and
`reassert`, the listed `keys` copied in from `source` at each start.

### Requests

Requests are literal. A kit may compute them from its consumer's Nix arguments (`kitArgs`), never from a file in
the workspace at run time.

| Kind | Form | Class |
|---|---|---|
| `read` | a host path | ordinary |
| `network` | `"host"` (port 443), `"host:port"` | ordinary; a wildcard host is powerful |
| `env` | a name, or `{ name; secret = true; }` | ordinary |
| `state` | `{ name; at; shared; }` | ordinary; `shared = true` is powerful |
| `write`, `socket` | a host path | powerful |
| `unsandboxed` | a boolean | powerful |
| `capabilities` | `{ name; … }` | `status.relay` and `git.fetch` ordinary; `credential`, `mcp`, `nix`, `git.push`, `git.sign`, `ops.read` and `delegate` powerful |

Each request has one canonical string, which the [grants file](grants-1.md#canonical-requests) stores and digests.

### Commands

| Field | Type | Meaning |
|---|---|---|
| `program` | a store path, or `{ host = "~/…"; }` | resolved on the host at each start |
| `args` | list of strings or store paths | put before the caller's arguments |
| `access` | `"read"` or `"write"` | can only narrow the context's |
| `requests` | as the kit's | the command's own reach |
| `description`, `settings` | string, attrs | shown by `pot explain` |
| `fixedDirs`, `realDirs`, `skillDirs`, `memory` | attrs, list, list, attrs | host folders the command sees ([pot's reference](../../docs/reference/pot.md#schema)) |

## Identity

pot decides who a kit is from facts it observes on the host. Nothing that a repository, a caller's environment or
a wrapper's arguments says is taken as fact. These forms are frozen:

| Kit | Identity | Established by |
|---|---|---|
| Root kit | `root:<name>[@<instance>]` | the root's store manifest. A workspace kit that claims a `root:` name is an error. |
| Scion kit | `git:<url>` | `.gitmodules` at the workspace's `HEAD`, verified: the gitlink commit is a release tag (`vX.Y.Z`) at that URL, cached per URL and commit. An unverified kit is listed, and its grant does not apply. |
| Repo kit | `repo:<normalised origin>`; with no remote, `repo:<machine>:<git common dir>` | `git remote get-url origin`, so `insteadOf` applies |

The origin normalisation is frozen:

1. The scp form `[user@]host:path` becomes `ssh://host/path`.
2. The scheme, the user and any token are dropped.
3. The host is lowercased. The port is dropped only when it is the scheme's default (https 443, http 80, ssh 22,
   git 9418).
4. `//` is collapsed. A trailing `/`, then `.git`, then `/` is stripped.
5. The path is lowercased only on case-insensitive forges: github.com, gitlab.com, codeberg.org, bitbucket.org.
6. The query and fragment are dropped.

So `ssh://forgejo@platform-vm:2222/daniel/graft.git` is `repo:platform-vm:2222/daniel/graft`. Different ports stay
different identities. ssh host aliases are not resolved. Worktrees share their common directory, so they share an
identity.

## Compatibility

- **Unknown fields are errors:** `pot: <file>: <field> is not a kit field (commands, env, files, packages, protect,
  recipes, requests)`. So are unknown request kinds, capability names, capability fields and command fields. A
  provider learns of a mistake when its consumer builds, not when a run starts.
- **Values are added, not fields.** A release may add a capability name or a files mode. A new field is a new
  version of this contract, which pot reads before any producer writes it.

<!-- Milestone 2 (not built yet): the field `version` (missing means 1.0), kit 1.1 (`tools`, `agent`, `dirs`, the
     files mode `mirror`), the refusal of a newer minor, and `packages` as a deprecated alias. -->

## Sources

- [pot/core/compose.py](../core/compose.py): `validate_kit`, `parse_requests`, `normalise_origin`,
  `repo_identity`, `scion_identity`
- [pot/lib/kit.nix](../lib/kit.nix): a kit as a manifest
- [pot's reference: Kits](../../docs/reference/pot.md#kits)
