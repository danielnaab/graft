---
status: draft
---
# Sandbox

`sb`, graft's sandbox: its model, contexts, kits, requests, grants, command line, messages and limits. For the
steps, see [Run an agent or a command in a sandbox](../guides/run-in-a-sandbox.md). For a move from v2.14.0's
`sandbox` launcher, see [Move from v2.14.0's sandbox](../guides/move-from-the-v2-14-sandbox.md). For why, see
[Decisions](../decisions.md#roots-grant-reach-kits-only-ask).

`sb` is experimental in v2.15.0. It runs on Linux, with bubblewrap. On macOS it refuses to start until a later
release ([On macOS](#on-macos)).

## The model

`sb CMD` runs a command as a **run**: one process tree, under one policy, enforced by one **backend**. Plain
`claude` stays unrestricted: `sb` is opt-in.

A policy has three kinds of item:

- **see:** tools on `PATH`, fixed variables, fixed files;
- **limits:** hidden paths, protected paths, read-only access, no network;
- **reach:** host paths, network hosts, passed variables, sockets, persistent state, and named **capabilities**
  such as `git.fetch` or `credential`.

Any kit may add limits. A kit's see items configure its own commands, and only a repo kit's packages reach every
run. Reach comes only from the person: a **grant** recorded in their **root**, or a flag typed for one run.

Eight concepts:

| Concept | One line |
|---|---|
| **Run** | One command, under one policy, by one backend. |
| **Root** | The host-side authority: contexts, root kits, grants, bindings, backends. |
| **Context** | A root's named starting point: mode, workspace access, backend, the kits it trusts and includes. It carries no reach of its own, except `inspect`'s view. |
| **Kit** | A bundle with an identity that the host establishes: see items, limits, requests, commands. |
| **Request** and **grant** | Approval. A kit asks for one reach item, and the root approves the kit's set. |
| **Capability** and **binding** | Mechanism. A capability is a request named by its effect (`git.fetch`), and a binding is the root's way of carrying it out. |
| **Backend** | Enforcement on one platform, with probed [guarantees](#guarantees). |

A capability is a kind of request. A grant says *whether*; a binding says *how*.

Terms built from these:

| Term | Meaning |
|---|---|
| **workspace** | The repositories a run works on. |
| **mode** | `in-place` or `clone`. |
| **access** | `write`, `read` or `none`: what the run may do to its workspace. |
| **state** | A reach request for a private persistent directory. |
| **command** | A program that a kit ships. |
| **command kit**, **repo kit**, **scion kit**, **root kit**, **agent kit** | A kit by its place: the kit whose command runs; a repository's own needs; a grafted provider's kit; a kit the root defines; a kit that describes an agent, such as Claude Code. |
| **allowance** | The bounds on a `delegate` grant (not in v2.15.0). |
| **digest**, **source** | What a grant is fixed to: a hash of the kit's request set, and for powerful items the code it came from. |

graft's other terms keep their meaning. A provider ships a kit in `graft.nix`. A consumer's own kit is its repo
kit. A scion is how a provider's kit enters a workspace. A pin is still only the gitlink.

**Reach follows the command.** `sb claude` gets Claude's grant, the repo kit's, and the standing reach of the
root kits that its context includes. **Output is untrusted**: after an in-place run, read the tree as you would
read a stranger's pull request.

## Roots

A root is where the person's authority lives. A run never writes a root in place.

| Root | Where | For |
|---|---|---|
| A home-manager root | `programs.sb` in your home-manager configuration, from `inputs.graft.homeManagerModules.sb` | Your own machine. Its source repository is a [root repository](#root-repositories). |
| graft's default root | `nix run github:danielnaab/graft/v2.15.0#sb -- CMD` | Someone without a root: graft's contexts, hidden list and standard tools. |
| A minimal root of your own | `sb init` writes one, built with `graft.lib.mkRoot` | A start for a root that you then edit. |

<!-- unverified: the home-manager module's attribute (homeManagerModules.sb), graft.lib.mkRoot, the default
root's flake attribute (#sb), its bindings, and what sb init writes and where. -->

How a call finds its `sb`: `<home>/.config/graft/sb/launcher`, a link that the root writes. `<home>` comes from
the password database, never from `$HOME`. If the link is missing, graft's default root runs. There is no
host-wide `/etc` pointer.

### Home-manager options

`programs.sb.<option>`:

| Option | Meaning |
|---|---|
| `enable` | Installs `sb` and writes the launcher link. |
| `grantsFile` | The [grants file](#the-grants-file), such as `"~/.workstation/sandbox/grants.json"`. Tracked in the root's repository, hidden from every run. |
| `stateDir` | `sb`'s state: agent homes, caches, run directories. Such as `"~/.local/state/graft/sb"`. Hidden from every run. |
| `rootRepos` | The root's source repositories. Default: the home-manager flake's repository. See [Root repositories](#root-repositories). |
| `hidden` | Paths hidden from every run, added to [graft's default list](#hidden-paths). |
| `inspect.home` | The paths under `~` that `inspect` can read. Replaces graft's default, `~/src` and home-manager's generated files. |
| `tools` | Packages on every run's `PATH`, after the standard set. |
| `caBundle` | A CA bundle for every run, such as a corporate proxy's. |
| `kits.<name>` | Root kits: agent kits and standing reach ([Kits](#kits)). |
| `repoKits."<origin>"` | A repo kit for a repository that is not a graft consumer, keyed by its [normalised origin](#identity). |
| `contexts.<name>` | Changes to the contexts, and new ones ([Contexts](#contexts)). |
| `bindings.<capability>."<selector>"` | How each capability is carried out ([Bindings](#bindings)). |
| `skillDirs` | Real directories in the agent's skills directory to bind read-only. Others are absent. |

The home-manager module asserts `workmux.sandbox.enable = false` while `programs.sb.enable` is on: workmux's
Docker sandbox is replaced for agent panes, never stacked with `sb`.

<!-- unverified: every programs.sb option name and default above, and the workmux assertion, against the
module. -->

## Contexts

The first word after `sb` names the context. With none, the root's default, `work`.

| Context | Mode | Access | Note |
|---|---|---|---|
| `host` | none | — | `sb host CMD`: no sandbox, said out loud. |
| `inspect` | in-place | read | Read the machine except secrets; change nothing. |
| `work` (default) | in-place | write | Write this repository in place. |
| `dispatch` | clone | write | Not in v2.15.0. Work in a throwaway clone and hand back commits. |

A root adds contexts with the same fields:

```nix
programs.sb.contexts.readonly = { extends = "work"; access = "read"; };
```

| Field | Values | Meaning |
|---|---|---|
| `extends` | a context name | Start from that context's fields. |
| `mode` | `in-place`, `clone`, `none` | `none` is `host`. `clone` is not in v2.15.0. |
| `access` | `write`, `read`, `none` | What the run may do to its workspace. |
| `backend` | a backend name, default `auto` | The backend that enforces the run. |
| `requires` | guarantee names | A run for which the backend cannot prove these is refused ([Guarantees](#guarantees)). |
| `trust` | `workspace`, `root` | Whether workspace kits are evaluated. `root` is not in v2.15.0. |
| `include` | root kit names | Root kits in every run of the context: their reach is standing reach. |
| `commands` | root kit names | Root kits whose commands a run may start as children, without their reach. Not in v2.15.0. |
| `network` | `granted`, `none` | `none` drops every network item. |
| `nest` | boolean, default `true` | Whether the run may hold nested children. Not in v2.15.0. |

A context name may not be a verb: `explain`, `why`, `grant`, `review`, `init`, `run` or `serve`.

### host

`sb host CMD` runs `CMD` with no sandbox, and says so. Use it for what only the host may do: `sudo`,
`systemctl start|stop|restart`, `docker run|exec`, `home-manager switch`, `darwin-rebuild`, and anything over the
system bus or the docker socket. `inspect` diagnoses; `host` repairs.

### inspect

`inspect` is the one context that carries reach: its view is `host` made read-only, with secrets hidden.

- **Outside `~`:** readable, except `/run`, `/tmp` and `$XDG_RUNTIME_DIR`. `/run/current-system`, `/etc` and the
  whole Nix store are bound back read-only. Sockets are masked everywhere.
- **Inside `~`:** only `inspect.home`.
- **Secret patterns** are masked inside readable trees ([Secret patterns](#secret-patterns)).
- **Processes:** its own pid namespace and a new `/proc`. `ps` shows only the run. `journalctl` reads the
  journal files your user can read. `systemctl status` and `docker ps` need the system bus or the docker
  socket, so they are host-only.
- **Tools:** the standard set plus `procps`, `iproute2`, `lsof` and `systemd`.
- **The network:** the command kit's model API only, without MCP. The model provider gets whatever the run
  reads.
- **Hidden even here:** the grants file and `sb`'s state. Root repositories are readable, so an agent can
  diagnose a failed `home-manager switch`. It can read the hidden list, and the grants file's committed
  contents through the root's `.git`.
- **Outside a git repository**, `sb inspect` runs with no workspace.

<!-- unverified: inspect's extra tools, and that sb inspect outside git runs (state key none). -->

### work

`work` writes the repositories of its workspace in place. The workspace is the repository around the current
directory, and each `--repo PATH`. The run can write its workspace, except [protected paths](#protected-paths).
The rest of the host is absent, except what the run's grants reach. Outside a git repository, `work` refuses:
`~/Downloads is not inside a git repository: sb inspect CMD, or sb host CMD`.

### Root repositories

A root's source repositories (`rootRepos`) decide what every run may do, so a run never writes one in place. In
v2.15.0, `work` refuses in a root repository, and so does `--repo` to one:

```
graft sandbox (work): ~/.workstation is a root: sb inspect, or claude (unsandboxed) until sb dispatch (1b)
```

Change a root repository with plain `claude`, or by hand, and review the change before `home-manager switch`.
`sb inspect` reads it. In a root repository's worktrees, set workmux's agent command to `claude` in the
repository's `.workmux.yaml`.

## Kits

A kit is a bundle with an identity that the host establishes:

| Kit | Declared in | Identity |
|---|---|---|
| Root kit | the root's `programs.sb.kits.<name>` | `root:<name>[@<instance>]` |
| Scion kit | a provider's `graft.nix`: `kit = { … };` | `git:<url>`, verified: the gitlink is a release tag at that URL |
| Repo kit | a consumer's mkFlake `perSystem`: `kit = { … };`; or the root's `repoKits."<origin>"` | `repo:<normalised origin>` |

graft ships agent kits as functions that a root calls, such as `graft.kits.claude-code`. Each account is an
instance: `root:claude-code@personal`, `root:claude-code@flexion`.

<!-- unverified: the attribute graft.kits.claude-code and its arguments (account, binary, files, skillsDir,
skillSources, provider, region, mcp). -->

### Schema

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
    network = [ "api.anthropic.com" ];          # "host" means port 443; "host:port"
    env = [ "WM_TOKEN" { name = "AWS_SESSION_TOKEN"; secret = true; } ];
    state = [ { name = "claude"; at = "~/.claude"; } ];
    capabilities = [ { name = "git.fetch"; remote = "origin"; } ];
  };
  # commands: programs the kit ships
  commands.meeting-kit = { program = "${pkg}/bin/meeting-kit"; access = "write"; requests = { }; };
  # recipes: data from which graft generates just recipes
  recipes.meeting-run = "meeting-kit run";
};
```

| Field | Kind | Meaning |
|---|---|---|
| `packages` | see | On `PATH`. A repo kit's packages reach every run in its workspace; a scion kit's do not. |
| `env.set` | see | Fixed values, for the kit's own commands. Names that `sb` owns are refused: `BASH_ENV`, `PATH`, `HOME`, `GRAFT_SANDBOX*`, the proxy variables, `NIX_REMOTE`, `SSL_CERT_FILE`, `TZ`. |
| `files."<path>"` | see | A file in the run. A key starts with `~/` (in the run's `HOME`) or `/` (needs the `root-files` guarantee). `mode`: `fixed`, bound read-only; `seed`, copied into state on first use, then the run's own; `reassert`, the listed `keys` written from `source` at every start. |
| `protect` | limits | Workspace paths, read-only inside. A path relative to the repository's top, or `**/NAME`. |
| `requests.*` | reach | Requests, inert until [granted](#grants). |
| `commands.<name>` | | A program the kit ships, with its own `access` and `requests`. |
| `recipes.<name>` | | A just recipe body: `"<command> <args>"`. |

- **Requests are literal.** A kit may compute them from its consumer's Nix arguments
  (`perSystem.kitArgs.<kit>`), never from a workspace file at run time. They are part of the digest.
- **Paths are normalised.** `..` is refused. A path under your home is recorded as `~/…`.
- A `program` written `{ host = "~/.local/bin/claude"; }` is resolved on the host at start. Its target must fall
  under a `read` request.
- In v2.15.0, `commands` and `recipes` are declared but not wrapped or generated: kit tools run as they did
  before ([Not in v2.15.0](#not-in-v2150)).

<!-- unverified: perSystem.kitArgs, and how v2.15.0 treats commands and recipes (accepted and unused, or an
error). -->

### Which kits a run is in

1. its **command kit**: the kit whose command runs, if any;
2. the **repo kit** of each repository in its workspace;
3. the **scion kits** of those repositories;
4. the root kits that its context **includes**.

What configures a program follows that program's command. What describes the workspace follows the workspace:

| Kit | see | limits | reach (when granted) |
|---|---|---|---|
| Command kit | all of it | `protect` | its requests, and its command's |
| Repo kit | `packages` to every run; `env.set` and `files` only to its own commands | `protect` | its requests, to every run in the workspace |
| Scion kit | none | `protect` | only when one of its commands runs |
| Included root kit | all of it | `protect` | its requests, to every run in the context |
| Root and context | the standard tools, `TZ`, the CA bundle | hidden paths, `access`, `network` | none, except `inspect`'s view |
| Flags, one run | `--package` | `--access read` | `--read`, `--write`, `--net`, `--env` |

How `sb NAME` finds its command kit:

1. Root kits first, then workspace kits.
2. Two kits of the same rank that ship `NAME` stop the run, and suggest `sb KIT:NAME`.
3. A program that no kit ships runs with no command kit.
4. A workspace kit may not ship a command named like a root kit's command, a standard tool or `sb`.

A kit with a stale grant stops only the runs its requests apply to. Other runs get a note on the startup line,
such as `meeting-kit asks for more: sb grant`.

### Merge rules

- **Union with sources, no priorities.** Every item keeps the kit or grant it came from. Nothing outside the
  root's own Nix uses `mkForce`.
- **Conflicts are errors.** Two values for one `env.set` name, or two different files at one path, stop the run.
- **Paths resolve on the host,** with links followed, at start. A reach path that resolves into a hidden path,
  or outside its requested prefix, is refused with its reason. The most specific rule wins. When rules tie, the
  narrower wins: hidden, then read, then write. Hidden wins at every depth below it.
- **Two path spaces.** Reach paths are host paths that grants and flags name. `sb` places its own mounts (state,
  run directory, policy file) from its state directory, which no request can name.
- **Sockets are reach.** In each granted directory, every socket is masked unless a `socket` request names it.

### Identity

`sb` decides who a kit is, where it runs and what it may read from facts it observes on the host. Nothing that a
repository, the caller's environment or a wrapper's arguments says about these is taken as fact.

| Question | Established by |
|---|---|
| A root kit's identity | `root:<name>[@<instance>]`, from the root's store manifest. A workspace manifest that claims a root name is an error. |
| A scion kit's identity | `git:<url>`, from `.gitmodules` at the workspace's `HEAD`, verified: the gitlink commit is a release tag at that URL. The check is cached per URL and commit. An unverified kit is listed, and its grant does not apply. |
| A repo kit's identity | `repo:<normalised origin>`, from `git remote get-url origin`. With no remote, `repo:<machine>:<git common dir>`. |
| Whether the caller is inside a run | `/run/graft-sandbox/policy.json` on a read-only mount. |
| The context | The first word typed, or the root's default. Never the environment. |
| State and cache paths | Fixed by the root, and refused inside any workspace. `XDG_*` is ignored at run time. |
| The account | `--account NAME`, or the caller's `CLAUDE_CONFIG_DIR`: the one input from the environment. It chooses only among the instances that the root declares, each granted on its own. A non-default account is named on the startup line. |

How an origin is normalised:

1. The scp form `[user@]host:path` becomes `ssh://host/path`.
2. The scheme, the user and any token are dropped.
3. The host is lowercased. The port is dropped only when it is the scheme's default (https 443, http 80, ssh 22,
   git 9418).
4. `//` is collapsed. A trailing `/`, then `.git`, then `/` is stripped.
5. The path is lowercased only on case-insensitive forges: github.com, gitlab.com, codeberg.org, bitbucket.org.
6. The query and fragment are dropped.

So `ssh://forgejo@platform-vm:2222/daniel/graft.git` is `repo:platform-vm:2222/daniel/graft`, and
`https://GitHub.com/DanielNaab/Notebook-Kit.git` is `repo:github.com/danielnaab/notebook-kit`. Different ports
stay different identities. ssh config host aliases are not resolved. Worktrees share their common directory, so
they share an identity.

## Requests

### Kinds and classes

A request is **ordinary** or **powerful**. `sb grant` approves a kit's ordinary items together, and each powerful
item on its own.

| Kind | Form | Class |
|---|---|---|
| `read` | a host path | ordinary |
| `network` | `"host"` (port 443), `"host:port"` | ordinary; a wildcard host is powerful |
| `env` | a name, or `{ name; secret = true; }` | ordinary. A secret shows `(the run holds it)`. |
| `state` | `{ name; at; }`, scoped per kit, workspace and context. `at` is in the run's `HOME`, default `~/.local/state/<name>`. | ordinary |
| `state` with `shared = true` | one scope for every workspace and context, for caches | powerful |
| `write` | a host path | powerful |
| `socket` | a socket path in a granted directory | powerful |
| `unsandboxed` | the kit's commands run on the host | powerful |
| `capabilities` | `{ name; … }` | [below](#capabilities) |

### Capabilities

A capability is a request named by its effect, and a root [binding](#bindings) carries it out. Every name is
reserved now. v2.15.0 binds those marked 1a.

| Capability | Phase, first binding | Class |
|---|---|---|
| `status.relay` | 1a: a relay and a `workmux` shim | ordinary |
| `git.fetch { remote }` | 1a: an ssh shim and a host broker that allows only `git-upload-pack` | ordinary |
| `credential { host; env }` | 1a: by `env` or `file`. Later: a proxy that replaces the auth header; `sigv4`. | powerful |
| `mcp { server }` | 1a: a fixed MCP config, with a `credential` by `env` or `file`. Later: by proxy. | powerful |
| `nix { build }` | 1a: a relay to an untrusted Nix daemon | powerful |
| `git.push { remote; refs }`, `git.sign`, `ops.read` | later | powerful |
| `delegate { … }` | later | powerful |

Also reserved: the network forms `host-loopback:PORT` and `{ host; via = "forward"; }`. A field that this version
does not know is an error.

- **`status.relay`:** a `workmux` shim inside forwards only `set-window-status` and `register-agent` to the
  launching tmux pane, which the host reads from its own `TMUX_PANE` at start. It escapes the text.
- **`git.fetch`:** `GIT_SSH_COMMAND` points at a shim. The host broker runs host `ssh`, with your agent, and
  allows only `git-upload-pack` to the bound URLs. Push from the host.
- **`credential`:** the run holds the secret, as a variable (`env`) or a file that the binding reads (`file`).
- **`mcp`:** the agent gets `--mcp-config <file> --strict-mcp-config`, so it reads no other MCP configuration.
- **`nix`:** a host relay runs `nix daemon --stdio --force-untrusted --process-ops` with the experimental
  features `daemon-trust-override` and `mounted-ssh-store`. The run gets
  `NIX_REMOTE=unix:///run/graft-sandbox/nix.sock`. The daemon reports `Trusted: 0`; builds and offline
  evaluation of locked inputs work. The run sees the whole store read-only, so it loses the `closure-store`
  guarantee. Fixed-output derivations reach the network outside the proxy. `sb explain` says both.

### Bindings

`bindings.<capability>."<selector>"`. The selector is the capability's key (a host, a remote URL pattern, an MCP
server name, or `*`), and may name one kit instance: `"api.anthropic.com kit=claude-code@flexion"`.

```nix
programs.sb.bindings = {
  "status.relay"."*" = { workmux = true; };
  "git.fetch"."ssh://forgejo@platform-vm:2222/*" = { broker = "git"; key.agent = true; };
  nix."*" = { via = "daemon-untrusted"; };
  credential."api.anthropic.com kit=claude-code@personal" = {
    via = "file"; from.file = "~/.config/graft/sb/claude-token-personal"; };
  mcp.metamcp = { url = "https://mcp.example.com/mcp"; via = "env"; env = "METAMCP_API_KEY";
                  from.file = "~/.config/metamcp/key.env"; };
};
```

- `from` is always a reference. A file that it names is hidden from every run.
- A binding may add see items, such as a shim or a config file. It never adds reach beyond its capability.
- `sb` resolves remote names in requests (`origin`) at start, from the protected `.git/config`.
- Host git never opens a repository that a run can write: the broker reads through a sandboxed `upload-pack`
  into staging that the host owns, with `transfer.fsckObjects=true`.

<!-- unverified: the binding attribute names (workmux, broker, key.agent, via, from.file, url, env). -->

## Grants

A request does nothing until you run `sb grant` at a terminal. A grant records the kit's exact request set as a
digest, and, for powerful items on code that the root does not pin itself, its source. If the kit later asks for
more, or its source changes, its runs stop with the difference until you grant again. Asking for less needs no
approval.

### The grants file

One tracked file in the root's repository, such as `~/.workstation/sandbox/grants.json`. `sb` reads it at start,
with no rebuild. Commit it with the root's next change.

```json
{
  "version": 1, "canonical": 1,
  "grants": {
    "root:claude-code@personal": {
      "digest": "sha256-3qL…",
      "requests": [ "network api.anthropic.com:443", "read ~/.local/share/claude", "state claude",
                    "capability status.relay",
                    "capability credential api.anthropic.com env=CLAUDE_CODE_OAUTH_TOKEN" ],
      "granted": "2026-11-15 platform-vm" },
    "git:https://github.com/danielnaab/meeting-kit": {
      "digest": "…", "requests": [ "…" ], "source": { "commit": "9e1c…", "tag": "v0.3.0" }, "granted": "…" },
    "repo:platform-vm:2222/daniel/flexion-assistant": { "digest": "sha256-Zk1…", "requests": [ ] }
  },
  "machines": {
    "platform-vm": { "repo:platform-vm:/home/daniel/scratch/.git": { "digest": "…", "requests": [ "…" ] } }
  }
}
```

| Field | Meaning |
|---|---|
| `version` | `1`. |
| `canonical` | `1`: the form of the request strings and paths that the digest is computed over. |
| `grants."<identity>"` | One kit's grant: `digest`, the `requests` it covers, `source` for powerful items, `granted` (when, and on which machine), and `delegate` (reserved). |
| `machines."<machine>"."<identity>"` | A grant for one machine, which overrides the shared entry there. Repositories with no remote are always here. |

- **Machine-independent.** Identities hold no store paths, no system and no absolute paths, except for
  repositories with no remote. A path under your home is recorded as `~/…`. So the file is shared across
  machines through git. `sb grant` writes the shared section when the request set matches it, and the machine's
  section otherwise.
- **`source`:** none for a root kit, whose code the root's own lock pins. The verified release commit and tag for
  a scion kit's powerful grants. The tree hash of the flake directory for a repo kit's powerful grants. Ordinary
  items carry no source: the digest fixes them.
- **Hidden, not secret.** No run can see or write the grants file: it is unmounted and unwritable. It is not
  confidential: its committed contents are readable through the root's git history wherever the root's `.git` is
  readable, `inspect` included. Grants hold no secrets.

### What sb checks at start

| Case | Result |
|---|---|
| Nothing new | The run proceeds. |
| More requested, or a changed source | Exit 125, with the difference and `sb grant KIT`. |
| Never granted, at a terminal, with no agent marker | The grant prompt appears inline, and the run continues after it. The agent markers are `CLAUDECODE`, `CODEX_SANDBOX` and `GRAFT_SANDBOX`. A workmux pane is not one. |
| Never granted, otherwise | Exit 125, with `sb grant KIT`. |
| The file changed since this machine last read it | One line, such as `3 grants arrived by git: sb grant --diff`. Nothing stops. |
| The file does not parse | Every run stops. |

<!-- unverified: the "never granted, otherwise" row (exit 125) is inferred, not stated in the design. -->

## The policy file

At start, `sb` composes the run's policy from manifests, grants, flags and host facts, and writes it as
`policy.json`, version 1. Inside the run it is `/run/graft-sandbox/policy.json`, read-only, and
`GRAFT_SANDBOX_POLICY` names it.

```json
{
  "version": 1, "run": "7f3a", "context": "work", "mode": "in-place", "access": "write",
  "backend": { "name": "bubblewrap", "requires": [], "proved": [ "closure-store", "own-netns", "…" ] },
  "workspace": [ { "id": "repo:platform-vm:2222/daniel/flexion-assistant", "path": "/home/daniel/src/flexion-assistant",
                   "common": "…/.git", "protected": [ "…" ], "clone": null } ],
  "items": [ { "kind": "network", "value": "api.anthropic.com:443", "class": "reach",
               "sources": [ { "kit": "root:claude-code@personal", "file": "…", "as": "request" },
                            { "grant": "grants.json", "digest": "sha256-3q…" } ], "effective": true } ],
  "mounts": { "state": [ { "name": "claude", "at": "~/.claude", "key": [ "root:claude-code@personal", "repo:…", "work" ] } ],
              "run": "/run/graft-sandbox" },
  "bindings": [ { "capability": "status.relay", "socket": "/run/graft-sandbox/relay.sock", "see": [ "workmux shim" ] } ],
  "children": { },
  "grants": [ { "id": "root:claude-code@personal", "digest": "sha256-…", "match": true } ]
}
```

| Field | Meaning |
|---|---|
| `run` | The run's id. |
| `context`, `mode`, `access` | The context, and what it set. |
| `backend` | Its name, the guarantees that the context requires, and those that it proved. |
| `workspace` | Each repository: identity, path, git common directory, protected paths, and `clone` (always `null` in v2.15.0). |
| `items` | Every see, limits and reach item: its kind, value, class, each source (kit, file, grant), and whether it is in effect. |
| `mounts` | State directories with their scope key, and the run directory. |
| `bindings` | Each bound capability, its socket, and the see items it adds. |
| `children` | The policies of the commands that the run may start. Empty in v2.15.0. |
| `grants` | Each kit's grant, and whether its digest matched. |

The shape is frozen: a later version adds values, not fields.

## Guarantees

A backend probes the guarantees it can prove. A context's `requires` names guarantees. A probe that fails, or an
item that removes a required guarantee, refuses the run with exit 125: `requires separate-kernel; bubblewrap
proves …`. The startup line names the guarantees that the root's usual backend has and this one lacks.

| Guarantee | Meaning |
|---|---|
| `closure-store` | Only the closure's store paths are visible. The `nix` capability removes it. |
| `own-netns` | The run has its own network namespace. |
| `own-pidns` | The run has its own pid namespace and `/proc`. |
| `no-tiocsti` | The run cannot type into the caller's terminal. |
| `protected-as-mounts` | Protected paths are read-only mount points. |
| `local-listen` | The run can listen on its own loopback. |
| `resource-limits` | Memory and task limits. |
| `separate-kernel` | The run has its own kernel. |
| `in-place` | The workspace is the host's working tree. |
| `same-path-clone` | A clone appears at the original path. |
| `nest` | A run can hold nested runs. |
| `private-proxy` | Only this run can use its network proxy. |
| `root-files` | `files` may place a file at `/…`. |

bubblewrap lacks `resource-limits` and `separate-kernel`.

<!-- unverified: which guarantees v2.15.0's bubblewrap backend probes and proves. -->

## The command line

```
sb [VERB | CONTEXT] [FLAGS] [--] [CMD [ARGS...]]
```

The verbs are `explain`, `why`, `grant`, `review` and `init`. `run` and `serve` are reserved. A first word that is
not a verb is a context if it names one, and otherwise the command. `--` forces a command. Flags after `CMD`
belong to the command; a known `sb` flag found there is warned about.

```sh
sb claude                        # work here
sb inspect claude                # look around, change nothing
claude                           # unrestricted
sb host --resume 9b0e claude     # carry a run's transcript to the host, as data
sb --net pypi.org claude         # one more host, for this run
sb --repo ~/src/other claude     # a second repository in the workspace
```

<!-- unverified: what sb runs with no CMD (a shell?). -->

### Flags

Each flag mirrors a kit field, for one run. The exit line names its permanent form.

| Flag | Mirrors | Meaning |
|---|---|---|
| `--package PKG` | `packages` | Adds a package from the root's nixpkgs, by attribute name. A flake reference is refused: it would evaluate on the host. |
| `--read PATH` | `requests.read` | Reads a host path. |
| `--write PATH` | `requests.write` | Writes a host path. |
| `--net HOST[:PORT]` | `requests.network` | Reaches a host, on port 443 by default. |
| `--env NAME` | `requests.env` | Passes a variable from the caller. |
| `--access read\|write` | `access` | Narrows the workspace's access. |
| `--mode in-place\|clone`, `--clone` | `mode` | `clone` is not in v2.15.0. |
| `--repo PATH` | | Adds a repository to the workspace, with its repo kit and protections. |
| `--from REF`, `--detach` | | Clone mode. Not in v2.15.0. |
| `--account NAME` | | Chooses an agent kit instance. |
| `--backend NAME` | | Chooses a backend that the root defines. |
| `--resume RUN` | | With `host`: passes run `RUN`'s transcript into a new host conversation as quoted data, never as history. |

A flag or `--repo` that names a hidden path, or a root repository in `work`, is refused at start.

<!-- unverified: which flags v2.15.0 accepts, and the messages for the clone-mode flags. -->

### sb explain

```
sb explain [CONTEXT] [FLAGS] [CMD] [--json] [--backend-command]
```

Prints the run's policy without running it: every item with the kit, file or grant that it came from. It lists
the `--settings` overlay's hooks, which Claude's `/hooks` does not show, and says that kit tools run on the host in
v2.15.0. `--json` prints the policy file. `--backend-command` prints the backend's command line.

```
run        claude → root:claude-code@personal
context    work: in place, bubblewrap; includes root:daniel
workspace  repo:platform-vm:2222/daniel/flexion-assistant (write)
           scions: notebook-kit v2.3.0, knowledge-base-kit v2.0.0, meeting-kit v0.3.0 (verified)
see        standard tools · uv bun nodejs (root) · repo tools · claude-code files (fixed), skills (14, read-only)
limits     protect: flake dir, host-run files, agent config (graft) · ensure.sh .local .githooks AGENTS.md (repo) ·
           notebook-kit.yaml knowledge-base.yaml meeting-kit.yaml windmill (scions) · hidden: 24 paths
reach                                                               granted
  network  api.anthropic.com claude.ai                              claude-code@personal
           pypi.org files.pythonhosted.org registry.npmjs.org       daniel (included by work)
  read     ~/.local/share/claude ~/src/agent-skills                 claude-code@personal
  state    claude → ~/.claude (flexion-assistant, work)             claude-code@personal
  cap      credential api.anthropic.com: file (the run holds it)    claude-code@personal
           status.relay                                             claude-code@personal
           git.fetch forgejo                                        daniel (included by work)
           nix (whole store visible; drops closure-store)           repo:platform-vm:2222/daniel/flexion-assistant
backend    bubblewrap proves closure-store* own-netns own-pidns no-tiocsti protected-as-mounts …; lacks separate-kernel resource-limits
           * dropped for this run by nix
```

<!-- unverified: the layout of sb explain's output. -->

### sb why

```
sb why PATH|HOST|CMD|VAR
```

Answers for one item: whether the run has it, where it came from, and, if the run lacks it, the one-run flag and
the permanent fix. It works inside a run, from the run's policy file.

### sb grant

```
sb grant [KIT...] [--revoke KIT] [--diff] [--root ROOT --workspace PATH]
```

- With no `KIT`: each kit of this workspace's runs that is not granted, or asks for more.
- `--revoke KIT` removes a kit's grant. `--diff` shows what changed, such as grants that arrived by git.
- `--root` and `--workspace` grant for another root and workspace.

`sb grant` is for people:

- It reads its confirmation from `/dev/tty`, and refuses when an agent marker is set.
- It escapes everything it prints.
- It approves ordinary items together, and each powerful item on its own.
- It marks requests whose defining file is not committed.
- It names the scope, such as "every workspace that grafts meeting-kit", and the capabilities that have no
  binding yet.

```
claude-code@personal   root kit                                       not granted
  ordinary:  + network api.anthropic.com · claude.ai
             + read    ~/.local/share/claude
             + state   claude (per workspace and context)
             + capability status.relay        bound: workmux relay
  grant these? [y/N] y
  powerful:  + capability credential api.anthropic.com (env CLAUDE_CODE_OAUTH_TOKEN)
               bound: file ~/.config/graft/sb/claude-token-personal (the run holds it)    grant? [y/N] y
wrote ~/.workstation/sandbox/grants.json. Commit it with ~/.workstation's next change.
```

It is a speed bump, not a guarantee: an unrestricted agent on the host can edit the file or script a terminal.

<!-- unverified: the default set of kits with no KIT, and the prompt's exact text. -->

### sb init

Writes a minimal root, built with `graft.lib.mkRoot`, for someone who has none.

<!-- unverified: what sb init writes, where, and what it prints. -->

### sb review

`sb review [RUN] [--fetch|--discard|--approve] [--since T] [--flags] [--root NAME] [--json]` lists past runs and
acts on one. Not in v2.15.0: it comes with `dispatch` and the run log.

<!-- unverified: what sb review does in v2.15.0 (refuses, with which message). -->

## Signals

### The startup line

Before the command starts, `sb` prints one line on stderr. It shows only what differs from the root's usual
shape: a context other than the default, a non-default account, a requested variable that is unset, another
kit's stale grant, a missing guarantee, `host`, or a resume.

```
sb: work · flexion-assistant · sb explain
sb: inspect · env METAMCP_API_KEY unset · meeting-kit asks for more: sb grant
```

<!-- unverified: the startup line's exact layout. -->

### The exit line

After the run, `sb` prints lines only when it has something to say:

- unsigned commits, with the host command that signs them:
  `git rebase --exec 'git commit --amend --no-edit -S' <base>`;
- what the run changed that the host runs or reads: new git metadata and gitlinks, changed protected files,
  `.gitmodules`, `.lfsconfig`, `.gitattributes`, CI workflows, and changes to the repo kit;
- the run's denials;
- for each one-run flag used, its one permanent fix.

The permanent fix names the command's kit when that is a workspace kit. Otherwise (an agent kit, or a program
that no kit ships) it names the repo kit, and the context's included root kit as the fix for every repository. A
fix that needs a `home-manager switch` says so.

<!-- unverified: the exit line's exact wording. -->

### Inside a run

- The prompt is `(sb:work) ~/src/flexion-assistant $`.
- `/run/graft-sandbox/README.md` explains the run. `~/.graft-sandbox-hidden` lists the hidden top-level names.
- `/run/graft-sandbox/denials` lists the run's denials ([Denials](#denials)).

## Environment

`sb` starts the command with an empty environment, then sets:

| Variable | Value |
|---|---|
| `GRAFT_SANDBOX` | The context's name, such as `work`. Test it with `[ -n "${GRAFT_SANDBOX:-}" ]`. |
| `GRAFT_SANDBOX_TOOLS` | The `bin/` of the workspace's tools, which `sb` built and cached on the host. |
| `GRAFT_SANDBOX_POLICY` | `/run/graft-sandbox/policy.json`. |
| `GRAFT_SANDBOX_CLONE` | `1` in clone mode. Never set in v2.15.0. |
| `PATH` | The standard set first, then the root's tools, the kits' packages and the workspace's tools, and the not-found shims last. |
| `HOME` | Your home's path. Only what the run's items bind is in it. |
| `TZ` | The host's `TZ`, or the zone name of `/etc/localtime` when it is unset. |
| `SSL_CERT_FILE` | The CA bundle: the root's `caBundle`, or the closure's. |
| `PS1` | `(sb:<context>) \w \$ `. |
| `BASH_ENV` | The not-found handler. |
| The proxy variables | `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and their lowercase forms, when the run has network. |
| `NIX_REMOTE` | `unix:///run/graft-sandbox/nix.sock`, with the `nix` capability. |
| `GIT_SSH_COMMAND` | The `git.fetch` shim, with that capability. |
| `TERM`, `COLORTERM` | The caller's, when set. |
| A kit's `env.set` | For the command kit, and the included root kits. |
| Granted `env` requests | Each name that the caller has set, with the caller's value. |
| A `credential` by `env` | The bound value, such as `CLAUDE_CODE_OAUTH_TOKEN`. |

<!-- unverified: the exact variable list and PATH order. -->

Inside, git uses a generated config: `user.*` resolved on the host, `commit.gpgsign=false`, and no includes or
helpers. Commits are unsigned. `/run/current-system` is bound read-only where it exists, in every context.

The standard set: bash, coreutils, findutils, grep, sed, gawk, diffutils, less, git, which, file, tree, ripgrep,
fd, jq, curl, gnutar, gzip, unzip, xz, patch, python3, just, gh, and graft's command.

## Protected paths

In place, a run can write its workspace except these, which are read-only mount points. They are defence in
depth: the tree after a run is untrusted output.

- What decides the sandbox: the flake directory that `sb` evaluates, `**/graft.nix`, `**/.gitmodules`.
- What the host runs: `**/.envrc`, `**/justfile`, `**/.pre-commit-config.yaml`, the `core.hooksPath` folder,
  `.githooks`.
- Agent configuration: `**/.claude`, `**/.mcp.json`, `**/.pi`, `**/.codex`, `**/.vscode`.
- The scions directory.
- Each kit's `protect`.
- In the common git directory: `config`, `hooks/`, `info/`, `worktrees/*/{commondir,gitdir,config.worktree}` and
  `modules/*/{config,hooks}`. The rest of it is writable, so a run can commit. It can also delete other branches'
  refs.

A protected file with a second hard link stops the run. To change a protected file, edit it on the host.

## Hidden paths

A hidden path is unmounted and unwritable in every run, `inspect` included. graft's default list:

- `~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.azure`, `~/.config/gcloud`, `~/.kube`, `~/.docker/config.json`, `~/.netrc`,
  `~/.git-credentials`, `~/.config/gh`, `~/.config/sops`;
- the agents' credential and configuration homes: `~/.claude`, `~/.claude.json`, `~/.pi/agent/auth.json`,
  `~/.codex/auth.json`, `~/.local/share/opencode/auth.json`;
- `~/.local/share/keyrings`; the browser profiles `~/.mozilla`, `~/.config/google-chrome`, `~/.config/chromium`,
  `~/.config/BraveSoftware`, `~/.config/vivaldi` and `~/snap/firefox`; `/etc/ssh/*_key`;
- `sb`'s state directory, and every grants file;
- each file that a binding's `from` names.

A root adds to it with `hidden`. A run's agent gets its own home: the claude-code kit's `state` puts a private
`~/.claude` in place of yours.

### Secret patterns

In `inspect`, these are masked inside readable trees:

- `**/.env`, `**/.env.*`, `**/*.tfstate*`, `**/.git-credentials`, `**/.netrc`;
- `**/id_rsa*`, `**/id_dsa*`, `**/id_ecdsa*`, `**/id_ed25519*`, except `*.pub`;
- `**/*.pem` and `**/*.key`, except inside `.venv`, `node_modules` and `site-packages`, where they are CA
  bundles;
- the root's credential references.

`.envrc` stays readable. `sb` finds them with a walk that ignores `.gitignore`, because secrets are gitignored by
design. For a `~/src` of 900,000 entries, the walk costs about 1.5 s cold and 0.1 s warm.

## Denials

Most clients do not show a proxy's refusal: curl and git print `CONNECT tunnel failed, response 403`, uv prints
`tunnel error: unsuccessful`, npm prints a generic 403. So `sb` sends each denial to each audience on its own
path:

| Audience | Channel |
|---|---|
| You, live | `sb` never writes to the run's terminal while it runs: a line there corrupts a full-screen program such as Claude Code. Under tmux, the host shows one `tmux display-message` on the launching pane for each new denial. Repeats are counted, not shown again. Outside tmux, nothing is live. |
| The agent | The host appends every denial to `/run/graft-sandbox/denials`, which the run can read but not write. The claude-code kit adds a `PostToolUse` hook (matchers `Bash` and `WebFetch`) that gives the model each new denial line as context. A missing program reaches the agent through the not-found shims. |
| Afterwards | The exit line lists the run's denials. |

`/run/graft-sandbox/README.md` tells a reader inside that `CONNECT … 403`, `tunnel failed` or
`Could not resolve host` mean that the sandbox refused, and to read the denials file or run `sb why HOST`.

### Messages

Every denial starts `graft sandbox (<context>):`. When the run's own traffic was refused, the message says "the
run tried", and gives the fix as a form, not as a command to paste.

| Denial | Message, after `graft sandbox (<context>): ` |
|---|---|
| Network, live (tmux) | `refused example.com:443 · sb why example.com` |
| Network | `the run tried example.com:443 (not in this run). one run: sb --net HOST …  permanent: the repo kit's requests.network (perSystem.kit), or programs.sb.kits.daniel.requests.network for every repository (home-manager switch) (sb why example.com)` |
| Requested, not granted | `api.github.com:443 is requested by meeting-kit, not granted: sb grant meeting-kit` |
| No binding | `needs git.push for ssh://…/flexion-assistant; this root binds none: programs.sb.bindings."git.push"."…" (home-manager switch)` |
| A program | `rg is not in this run. one run: sb --package ripgrep …  permanent: programs.sb.tools (home-manager switch), or the repo kit's packages` |
| Outside the workspace | `~/src/other is not in this run: sb --repo ~/src/other` |
| Hidden | `~/.ssh is hidden by your root in every run` |
| Protected | `justfile is protected (the host runs it): edit it on the host, or in a dispatch run` |
| A root repository | `~/.workstation is a root: sb inspect, or claude (unsandboxed) until sb dispatch (1b)` |
| Not a repository | `~/Downloads is not inside a git repository: sb inspect CMD, or sb host CMD` |
| Stale grant (125) | `meeting-kit (git:…) asks for more than its grant:`, then each new request as `  + network api.github.com:443`, then `  grant it: sb grant meeting-kit`. Also `… requests reach that is not granted:`, `… changed source since its grant:` with `  source: OLD -> NEW`, and `… is not a verified release, so its grant does not apply`. |
| Clone mode (1) | `clone mode (dispatch) lands in 1b; in 1a, sb works in place` |
| Missing guarantee (125) | `requires separate-kernel; bubblewrap proves …` |

On the startup line: `METAMCP_API_KEY requested and granted, unset on the host`. On macOS:
`sb: no backend for darwin in v2.15.0; Seatbelt returns in 1b (v2.14.0 has v2's sandbox)`.

`daniel` in these messages is the root kit that the context includes; yours has the name your root gives it.

<!-- unverified: every message text above against the code. The Protected message names "a dispatch run", which
v2.15.0 does not have. -->

## Exit status

- The command's exit status.
- `125` when `sb` refuses to start the run because of a grant (stale, missing or unverified) or a missing
  guarantee.
- `1` when `sb` refuses to start the run for another reason, such as a root repository, a hidden path in a flag,
  a conflict between kits, or clone mode.
- `2` for a usage error, such as an unknown context, an `--account` that the root does not declare, or two kits
  that ship the same command.

The message says why. `sb` never runs the command with a weaker policy, a weaker backend or no sandbox.

<!-- unverified: from lib/sb/compose.py's Refusal statuses while it was being written; the launcher's own
statuses (macOS, not a repository, command not found) are not confirmed. -->

## Claude Code

The claude-code kit runs Claude with its own agent home, configured by the root:

- **Settings, instructions, statusline:** `fixed` files from the root's home-manager files, plus a `--settings`
  overlay. The overlay sets `sandbox.enabled = false`, which Claude treats as managed (`/sandbox` cannot turn it
  on again), and adds the denial hook. Overlay hooks are added to your own hooks, which still run.
- **Skills, agents, commands, hooks, plugins:** `~/.claude/{skills,agents,commands,hooks,plugins}` are read-only.
  Each skill link's target is bound read-only: a store target is a see item, and a checkout target must fall
  under the kit's `read` requests. A real directory in the skills directory is absent unless the root names it
  in `skillDirs`.
- **MCP:** an `mcp` request gives `--mcp-config <file> --strict-mcp-config`.
- **Login:** a `credential` for `api.anthropic.com` with `env = "CLAUDE_CODE_OAUTH_TOKEN"`, bound to a file that
  you make once per account with `claude setup-token`. There is no `/login` inside, and the run never reaches a
  keychain.
- **Onboarding and folder trust:** `reassert` keys in `.claude.json`.
- **Transcripts:** `state claude` at `~/.claude`, scoped per account, workspace and context.
- **Accounts:** one kit instance per account, `claude-code@<account>`, each granted on its own.
- **Bedrock:** `provider = "bedrock"` requests `bedrock-runtime.<region>.amazonaws.com`, the Bedrock variables,
  and a `credential` for the `AWS_*` keys.
- **workmux status:** `status.relay`.

A sandboxed transcript is run output. `sb host --resume RUN claude` starts a new host conversation with it
attached as quoted data.

## On macOS

In v2.15.0, `sb` refuses to start on macOS:

```
sb: no backend for darwin in v2.15.0; Seatbelt returns in 1b (v2.14.0 has v2's sandbox)
```

v2.14.0 is the last release with a Seatbelt sandbox. On a Mac, stay on v2.14.0 for a sandbox until the Seatbelt
backend returns.

## Not in v2.15.0

- **Kit tools run on the host.** A kit's commands are not wrapped or sandboxed on their own: `just`, a hook or a
  scheduler runs them as before. Inside `sb claude`, they run under the run's policy. `sb explain` says so.
- **No `dispatch`:** no clone mode, no `sb review`, no run log, no fetch-back.
- **No nesting:** the children table is empty. A command started inside a run gets that run's policy.
- **No generated recipes or wrappers:** `recipes` and `commands` are not used yet.
- **No macOS** ([On macOS](#on-macos)).
- No proxy credentials, `git.push`, `git.sign`, `ops.read`, `trust = "root"`, `delegate` or resource limits.

## Residual risks

1. **Shared kernel.** A bubblewrap run shares the host's kernel. Only a VM proves `separate-kernel`.
2. **Allowed hosts carry data out.** The proxy limits where, not what. The model API carries the workspace. A run
   can use its own account on an allowed host. `inspect` gives the model provider what it reads.
3. **Ordinary grants trust a kit's future code** within their reach. Powerful grants fix the source for scion and
   repo kits. Root kits rely on the root's lock and its review.
4. **The host still runs repository and kit content:** Nix evaluation, fixed-output derivations, direnv, your own
   recipes, and in v2.15.0 kit tools and kit recipes.
5. **The working tree, transcripts and agent state are run output.** Read them as untrusted.
6. **In place, a repo kit's code is mutable.** A powerful grant fixes its flake directory's tree hash. The rest of
   its code, and ordinary grants, are not fixed.
7. **`sb grant` is a speed bump.** An unrestricted agent on the host can edit the grants file or script a
   terminal. A run cannot.
8. **Within one state scope,** a run can leave transcripts and caches that the next run of that scope reads. It
   cannot leave configuration.
9. **Sockets that a run creates** under a granted directory are reachable.
10. **The terminal is shared:** OSC 52 clipboard writes, title changes, tmux passthrough. Set tmux's
    `set-clipboard off` and `allow-passthrough off`. `sb` escapes every byte that a run chose before it prints it.
11. **Verifying a scion kit needs the network.** Offline, an uncached scion kit is unverified, and its grant does
    not apply.
12. **The nix relay depends on experimental Nix features.** Review it at each Nix upgrade.
13. **Grants are readable** through the root's git history wherever its `.git` is readable. Hidden means
    unmounted and unwritable, not confidential.
14. **Live denials need tmux.** Outside tmux, you see denials on the exit line, and the agent sees them through
    its hook and the denials file.
15. **Some denials cannot be observed:** reads of hidden paths, and lookups of variables that were not passed.

## Sources

<!-- unverified: the code paths below; make them links when the implementer's files exist. -->

- `lib/sb/`: the `sb` command, composition, grants, and the bubblewrap backend
- `lib/kits/claude-code.nix`: the claude-code kit
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `kit` from each `graft.nix` and `perSystem`, and
  `packages.<system>.graft-kits`
- [tests/run.sh](../../tests/run.sh)
