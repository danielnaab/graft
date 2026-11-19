---
status: working
---
# kit/1: one kit

| | |
|---|---|
| Version | 1.x: the field `version`, `"1.0"`, `"1.1"` or `"1.2"`. A missing `version` means `"1.0"`. |
| Producer | a provider's `graft.nix` (`kit`), a consumer's mkFlake `perSystem.kit`, a root's `programs.pot.kits.<name>` and `repoKits."<origin>"` |
| Reader | pot: [kitsFor](pot-lib-1.md) when it writes a bundle, mkRoot when it builds a root, and pot at run start |
| Owner | pot |

A kit is what a repository or a root ships: tools, commands, limits and requests for reach. Its identity is not
part of it: pot establishes that from host facts ([Identity](#identity)).

## Shape

```nix
kit = {
  version = "1.2";                       # optional; "1.0" when missing
  # see
  tools = [ ];                           # packages on PATH
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
  commands.<name> = { program; args; access; requests; description; settings; dirs; hook; };
  recipes.<name> = "<command> <args>";   # or, in 1.2, { run = "<command> <args>"; description = "…"; }
  agent = { markers; select; resume; hooks; };   # 1.1, a root's kit only
};
```

| Field | Since | Type | Meaning |
|---|---|---|---|
| `version` | 1.0 (v2.16.0 readers) | string | the minor the kit is written against |
| `tools` | 1.0 (v2.16.0 readers) | list of packages | On `PATH`. A repo kit's tools reach every run in its workspace; a scion kit's do not. |
| `packages` | 1.0 | list of packages | `tools`' old name. Deprecated in v2.16.0. Since v2.17.0 kitsFor and mkRoot refuse it; pot still reads it in a bundle that an older graft built ([Compatibility](#compatibility)). |
| `env.set` | 1.0 | attrs of strings | Fixed values, for the kit's own commands. Names that pot owns are refused. |
| `files."<path>"` | 1.0 | `{ source; mode; keys; }` | A file in the run, at `~/…` or `/…`; `/…` needs the `root-files` guarantee. |
| `protect` | 1.0 | list of strings | Workspace paths, read-only inside: relative to the repository's top, or `**/NAME`. |
| `requests.<kind>` | 1.0 | per kind | Requests, inert until granted ([Requests](#requests)). |
| `commands.<name>` | 1.0 | attrs | A program the kit ships ([Commands](#commands)). |
| `recipes.<name>` | 1.0; attrs form 1.2 | string, or `{ run; description; }` | A just recipe that runs one of the kit's commands ([Recipes](#recipes)), generated into `kits.just`. |
| `agent` | 1.1 | attrs | How pot recognises an agent and its instances ([Agent kits](#agent-kits)). Only a root's kit may declare it. |

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
`version`, `tools`, `recipes`, `agent`, `dirs` and `hook` are not requests: they change no canonical item and no digest.

### Commands

| Field | Since | Type | Meaning |
|---|---|---|---|
| `program` | 1.0 | a store path, or `{ host = "~/…"; }` | resolved on the host at each start |
| `args` | 1.0 | list of strings or store paths | put before the caller's arguments |
| `access` | 1.0 | `"read"` or `"write"` | can only narrow the context's |
| `requests` | 1.0 | as the kit's | the command's own reach |
| `description`, `settings` | 1.0 | string, attrs | shown by `pot explain` |
| `fixedDirs`, `realDirs`, `skillDirs`, `memory` | 1.0 only | | host folders the command sees; pot reads them as `dirs`. A 1.1 or 1.2 kit that gives one is refused: `… commands.NAME.fixedDirs is not a command field (kit 1.1: dirs)`. |
| `dirs` | 1.1 | attrs | host folders the command sees, read-only ([below](#dirs)) |
| `hook` | 1.2 | one of `pre-commit`, `commit-msg`, `prepare-commit-msg`, `pre-push`, `post-checkout`, `post-merge` | the git hook this command serves ([Hook commands](#hook-commands)) |

### Recipes

A recipe is data from which graft generates a just recipe; kits ship no hand-written `.just` files. The string form
(1.0) is `"<command> <args>"`; the attrs form (1.2) is `{ run = "<command> <args>"; description = "…"; }`. The first
word of `run` must be a command of the same kit, and the rest are literal words. pot checks this in every kit
version, when kitsFor builds, at run start, and inside the generated text itself (so `nix eval` refuses too):

- the name is `[A-Za-z][A-Za-z0-9_-]*`: `pot: FILE: recipes.N: a recipe's name is letters, digits, _ and -
  (starting with a letter)`;
- the words are `[A-Za-z0-9._/=:,@+%-]+`, one space apart: `pot: FILE: recipes.N: literal words only (letters,
  digits and . _ / = : , @ + % -, one space apart)`;
- the first word is one of the kit's commands: `pot: FILE: recipes.N: runs X, which is not one of this kit's
  commands (A, B)`;
- the attrs form in a 1.0 or 1.1 kit: `pot: FILE: recipes.N: { run; description; } (kit 1.2), or one line`.

Kits 1.0 and 1.1 take the string form only. graft generates one file, `kits.just`, from every kit's recipes
([pot-kits/1](pot-kits-1.md#shape)); before v2.18.0 it checked them and generated nothing.

### Hook commands

`commands.<name>.hook` names the git hook that the command serves. The consumer's `.githooks/<hook>` execs the
command's wrapper, and `git commit` inside a run gets that command's own policy from the children table (since
v2.18.0; v2.17.x checks the field and does nothing with it). Another value:
`pot: FILE: commands.NAME.hook: one of pre-commit, commit-msg, prepare-commit-msg, pre-push, post-checkout,
post-merge`; in a 1.1 kit, `commands.NAME.hook is not a command field (…)`.

There is no field that runs a command unsandboxed: running kit code outside the sandbox stays a granted
`unsandboxed` request.

### dirs

```nix
commands.claude.dirs = {
  "~/.claude/skills"  = { from = "~/.claude-accounts/flexion/skills"; real = [ "synced" ]; };
  "~/.claude/plugins" = { from = "~/.claude-accounts/flexion/plugins"; real = true; };
  "~/.claude/projects/{workspace}/memory" = {
    mode = "mirror"; from = "~/.claude-accounts/flexion/projects/{workspace}/memory";
    copy = [ "MEMORY.md" ]; label = "memory";
  };
};
```

The key is a path in the run's `HOME` (`~/…`). Each entry is `{ from; mode; real; copy; label; }`:

| Field | Mode | Meaning |
|---|---|---|
| `from` | both | the host folder: `~/…`, `/…`, or `null` for an empty read-only folder. A folder the host lacks is an empty read-only one too. |
| `mode` | | `links` (the default) or `mirror` |
| `real` | `links` | the real entries of `from` to bind (a list of names), or `true` for all |
| `copy` | `mirror` | the files that are writable copies |
| `label` | `mirror` | one lowercase word, default `mirror`, that names the mirror in messages and its out folder |

- **`links`:** the run path is a read-only folder that pot builds in the run directory. Each host entry that is a
  link is there by its target, one level deep: a store path, or a path under the run's granted `read` requests. A
  real entry is there only when `real` names it. Others are left out: the startup line counts them, and
  `pot explain` names them.
- **`mirror`:** the host folder's files, read-only, file by file. Each file that `copy` names is a writable copy
  instead, copied in at each start. When the run ends, new files and changed copies are copied out to
  `<run dir>/<label>-out/<key>/`, for a person to review. Nothing is written back to the host. `{workspace}`, in
  the key and in `from`, stands for each workspace (the working directory and the repository's top), written as
  its absolute path with every character outside `[A-Za-z0-9]` as `-`. Only a mirror may use it.

The architecture named `mirror` a files mode. v2.16.0 makes it a `dirs` mode instead: a kit's `files` come from
the store, while a mirror is a host folder, as `links` is.

### Agent kits

```nix
agent = {
  markers = [ "CLAUDECODE" ];
  select.env = { CLAUDE_CONFIG_DIR = "~/.claude-accounts/flexion"; CLAUDE_CODE_USE_BEDROCK = "1"; };
  resume = { transcripts = "~/.claude/projects/**/*.jsonl"; args = [ "--resume" "{id}" ]; render = <program>; };
  hooks = [ "PreToolUse Bash|WebFetch (pot's --settings overlay; /hooks does not list it)" ];
};
```

Every field is optional. Only a root's kit may declare `agent`; anywhere else it is refused:
`pot: FILE: agent: only a root's kit may declare it (an agent kit is the root's)`.

| Field | Type | Meaning |
|---|---|---|
| `markers` | list of variable names | A caller with any of these set is an agent: `pot grant` refuses, and a stale grant exits 125 instead of prompting. pot's markers are a floor, `CODEX_SANDBOX` and `CLAUDECODE` (root data in pot's Nix, `baseAgentMarkers`, which no root removes), plus `POT`, the root kits' `markers` and the root's `agentMarkers` (default `[ ]`), which add to the floor. So `pot grant` refuses under Claude Code even in a root with no claude-code kit. |
| `select.env` | attrs of strings | How the caller's environment names this instance ([Instances](#instances)). |
| `resume.transcripts` | a glob in the run's `HOME` | where the agent writes its transcripts, under one of its `state` requests |
| `resume.args` | list of strings | the arguments that resume a session; `{id}` is the newest transcript's name without its suffix |
| `resume.render` | a program | prints a transcript file's text, for `pot host --resume` |
| `hooks` | list of strings | rows that `pot explain` shows as `hooks` |

### Instances

A root names each instance of an agent kit: `kits."claude-code@flexion" = …`, with the identity
`root:claude-code@flexion`. A run's instance is the first of:

1. `CMD@NAME` on the command line;
2. `--instance NAME`;
3. the caller's variables: an instance matches when its `agent.select.env` is not empty and every variable in it
   equals the caller's. A value that starts with `~` or `/` is a path, and both sides are compared after `~`
   expansion and `realpath`; other values are compared as exact strings, so `CLAUDE_CODE_USE_BEDROCK=true` does
   not match `"1"`. An unset variable never matches. **The most
   specific match wins:** the instance whose `select.env` names the most variables. A tie goes to the default
   when it is among the matches; otherwise pot stops and names them;
4. the kit's default: the root's `defaultInstance.<kit>`, or the only instance. mkRoot checks each
   `defaultInstance.<kit>` when it evaluates: `graft.lib.mkRoot: defaultInstance.KIT = "X": the root declares
   KIT@a, KIT@b`, or `graft.lib.mkRoot: defaultInstance.KIT: the root has no kit KIT@NAME (kits: …)`. The home-manager
   module names its own options: `programs.pot.defaultInstance.KIT = "X": programs.pot.kits declares KIT@a, …` and `programs.pot.defaultInstance.KIT: programs.pot.kits has no KIT@NAME`.

Each instance is granted on its own.

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

- **Reader first.** pot reads a minor before any producer writes it. v2.17.0 reads 1.2; a producer writes 1.2 from
  v2.18.0, once the roots that read its kits are on v2.17.0 or later. A kit that writes 1.1 needs roots on v2.16.0
  or later. The claude-code kit writes 1.1. A kit's `commands` (any minor) are wrapped once its consumer takes
  graft v2.18.0; under a root older than that, the wrappers run their programs unwrapped
  ([pot-kits/1](pot-kits-1.md#shape)).
- **A newer minor gets one refusal, checked before any other field:**
  `pot: FILE: kit version 1.3 is newer than this pot reads (1.0, 1.1, 1.2): take a newer pot`. Another major, or a
  `version` that is not a string: `pot: FILE: kit version 'X': this pot reads kit 1.0, 1.1 and 1.2`; a number such
  as `version = 1.1`: `FILE: version is a string: "1.1"`. At run start, a bundle's refusal ends `, or run pot host
  CMD`.
- **Unknown fields within a known minor are errors:** `pot: FILE: FIELD is not a kit field (…)`, listing that
  minor's fields. So are unknown request kinds, capability names and fields, command fields, `dirs` fields,
  `agent` fields and `hook` values. A provider learns of a mistake when its consumer builds, not when a run starts.
- **Values are added, not fields,** within a minor. A new field is a new minor. (v2.16.0 was the one exception: it
  widened what a 1.0 reader accepts with `tools` and `version`.)
- **`packages`, removed at the source.** Since v2.17.0, kitsFor and mkRoot refuse a kit that gives `packages`:
  `pot: FILE: packages is tools' old name, gone in v2.17.0: give tools` (FILE is the kit's file, such as
  `scions/x/graft.nix` or `flake.nix (perSystem)`; for a root's kit, `pot: the root (programs.pot.kits.NAME): …`). pot
  still reads `packages` as `tools`, with no warning, in a bundle that graft v2.16.x built, so a consumer that has
  not moved its graft keeps working under a newer root. Giving both is an error.
- **The last planned change.** 1.2 adds every field that v2.18.0's wrapped commands, generated recipes, hook
  commands and children table read. Nesting is in [runs/1](runs-1.md) and [run-env/1](run-env-1.md), and the
  `pi` kit is written in 1.1's fields, so neither changes kit/1. This is what lets v2.18.0 and v2.19.0 be the two
  releases with `kit/1` unchanged that pot's split waits for ([pot's contracts](README.md#how-a-contract-changes)).
  v2.18.0 left it unchanged.

## Sources

- [pot/core/compose.py](../core/compose.py): `kit_version`, `validate_kit`, `check_dirs`, `check_agent`,
  `normalise_kit`, `normalise_origin`, `repo_identity`, `scion_identity`
- [pot/core/pot.py](../core/pot.py): choosing the instance, mirrors, resume
- [pot/kits/claude-code.nix](../kits/claude-code.nix): a 1.1 kit
- [pot's reference: Kits](../../docs/reference/pot.md#kits)
