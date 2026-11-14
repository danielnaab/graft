---
status: working
---
# Sandbox

The `sandbox` launcher, its options, and what a run sees. For the steps, see
[Run an agent or a command in a sandbox](../guides/run-in-a-sandbox.md). For why, see
[Decisions](../decisions.md#enforce-the-boundary-in-a-sandbox-not-in-rules). Linux only.

## The launcher

mkFlake builds `packages.<system>.sandbox` for every repository: a program named `sandbox`, with the
repository's [sandbox policy](../concepts.md#what-runs-in-a-sandbox) built in.

```
usage: sandbox [--with PKG]... [--allow HOST[:PORT]]... [--explain [--json]] [--dry-run] [--] [CMD [ARGS...]]
```

| Argument | Meaning |
|---|---|
| `CMD [ARGS...]` | The command to run. A name must be on the sandbox's `PATH`, which holds only the closure: the launcher checks it before the run (a name with a `/` is not checked). With no command: `bash`, an interactive shell with the prompt `(sandbox) DIR $`. Your `$SHELL` is not used. |
| `--` | Optional. The first word that is not an option starts the command, as with `env`. Write `--` before a command that starts with `-`. |
| `--with PKG` | Adds the package `PKG` to this run: the launcher builds it with `nix build`, makes its closure visible, and puts its `bin/` on `PATH`. One package for each `--with`; repeat it for more. See [--with](#--with). |
| `--allow HOST[:PORT]` | This run can also reach `HOST`, with the syntax of `network.allow`. Repeat it for more hosts. It works when the policy has no network. |
| `--explain` | Prints the policy, with the file that set each value, and each request that the consumer has not granted. Runs nothing. See [--explain](#--explain). |
| `--explain --json` | The same, as JSON. |
| `--dry-run` | Prints the startup line and the bubblewrap command, and runs nothing. No proxy starts, and the launcher creates no folder in the repository. |
| `-h`, `--help` | Prints the usage. |

Where to run it:

| Where | Command |
|---|---|
| A flake at the repository's top | `nix run .#sandbox -- CMD` |
| A flake in a subdirectory | `nix run 'path:nix/agent-tools#sandbox' -- CMD`, from the repository's top |
| The default dev shell (`nix develop`, direnv) | `sandbox CMD`: the launcher is on its `PATH`, with no flake evaluation for each run. It is the launcher of the moment you entered the shell: after you change the policy, leave the shell and enter it again (direnv reloads it for you). |
| A deployed service | `nix build .#sandbox -o result-sandbox` once, then `result-sandbox/bin/sandbox CMD`. The out-link is a garbage-collector root. |
| A `justfile` recipe | `nix run 'path:nix/agent-tools#sandbox' -- claude`: the one `--` is for Nix |

The launcher works from any directory in a git repository. The repository is the outermost git repository
around the current directory: from inside a scion, it is the repository that grafts the scion. The command
starts in the current directory. `--explain` works in any directory.

### --with

- `--with jq` builds `jq` from the flake's `inputs.nixpkgs`, or from graft's nixpkgs when the flake has none.
  When that nixpkgs is no longer in the Nix store (a launcher deployed elsewhere), it builds from the
  registry's `nixpkgs`.
- A word with a `#` is a flake installable, built as it is: `--with github:owner/repo#tool`. Nix evaluates
  that flake on the host, outside the sandbox: never name a flake that a run created.
- `NIXPKGS_ALLOW_UNFREE=1` in the caller's environment allows an unfree package (the launcher adds
  `--impure`).
- The launcher says what it added, and from where, before the startup line:

  ```
  graft: sandbox: with jq 1.7.1 (graft's nixpkgs), for this run
  ```

  The words in parentheses are `the flake's nixpkgs`, `graft's nixpkgs`, `the registry's nixpkgs`, or
  `from FLAKE`.
- `--with` needs Nix on the host, and the network or a binary cache for a package that is not built yet.

## Options

A `sandbox` attribute set, returned by a provider's `graft.nix`, by mkFlake's `perSystem`, or both:

```nix
sandbox = {
  packages = [ ];                 # closure roots, on PATH
  env.set = { };                  # fixed environment values
  protect = [ ];                  # repository paths, read-only inside
  repository.readOnly = false;    # true: the whole repository is read-only
  paths.readOnly = [ ];           # host paths, read-only
  paths.writable = [ ];           # host paths, writable
  network.allow = [ ];            # hosts the run can reach
  env.pass = [ ];                 # names copied from the caller's environment
  grant = [ ];                    # scions whose requests apply
};
```

| Option | Type | A provider | The consumer | Meaning |
|---|---|---|---|---|
| `packages` | list of packages | adds | adds | Each package and its closure are visible read-only, and each `bin/` is on `PATH`. |
| `env.set` | attribute set of strings | adds | adds | Fixed values. They are in the launcher's store path, which every user of the machine can read: never put a secret here. Use `env.pass`. |
| `protect` | list of repository paths | adds | adds | Read-only inside. A path relative to the repository's top, a file or a folder (`"AGENTS.md"`, `"nix/agent-tools"`), or `"**/NAME"`: every file or folder named `NAME` anywhere in the working tree, in scions and untracked files too. No other pattern. A path that does not exist is skipped. |
| `repository.readOnly` | boolean, default `false` | no | sets | `true`: the whole repository is read-only, for a command that only reads (a review agent). |
| `paths.readOnly` | list of host paths | requests | sets | Host paths, read-only. Each starts with `~/` or `/`. |
| `paths.writable` | list of host paths | requests | sets | Host paths, writable. Each starts with `~/` or `/`. |
| `network.allow` | list of `"host"` or `"host:port"` | requests | sets | Hosts the run can reach through the proxy. `"host"` means ports 443 and 80; `"host:port"` means that port only. The host can use `*` and `?`: `"*.anthropic.com"` (which does not match `anthropic.com`: list both). `"*"` allows every host, on ports 443 and 80. `[ ]`: no network. |
| `env.pass` | list of names | requests | sets | Variables copied from the caller's environment when they are set, such as `CLAUDE_CODE_OAUTH_TOKEN`. |
| `grant` | list of scion names | no | sets | The scions whose requests apply. |

- **Adds:** a provider's value applies, merged with the others. Lists are joined, without duplicates.
  `env.set` is merged by name; two files that set one name to different values fail evaluation, and name
  both files. The consumer can win with `lib.mkForce`.
- **Requests:** a provider's value applies only when the consumer names the scion in `grant`. The startup
  line counts the requests that are not granted, and `--explain` lists them. A provider may add to what the
  run can see, never to what it can reach.
- **No:** only the consumer may set it. A provider that sets it fails evaluation.
- A provider cannot override the consumer. `lib.mkForce` (or `lib.mkOverride` with a priority below 100) on
  an option fails evaluation. Other priorities have no effect: graft takes a provider's values as plain
  values.
- `~/` is the home of the user who runs the launcher. A `paths.*` entry that does not exist is skipped, and
  the startup line and `--explain` say so.

graft's base adds, before every other value:

- `packages`: `bashInteractive`, `coreutils`, `findutils`, `gnugrep`, `gnused`, `diffutils`, `less`,
  `gitMinimal`, and the repository's `packages.<system>.tools`.
- `protect`: `**/flake.nix`, `**/flake.lock`, `**/graft.nix`, `**/.gitmodules`, `**/.envrc`, `**/justfile`
  and `**/.pre-commit-config.yaml`.

## What a run sees

| Path | Inside the sandbox |
|---|---|
| The closure of `packages` (and of `--with`) | Read-only, at its `/nix/store` path. No other store path is visible. |
| The repository | Writable, at the same path (read-only with `repository.readOnly = true`). |
| [Protected paths](#protected-paths) | Read-only, in the repository. |
| `HOME` | An empty, temporary folder at your home's path. Nothing in it is kept after the run. If the repository is in your home, the folders on the way to it are there too. |
| `paths.readOnly`, `paths.writable` | Read-only or writable, at the same path. |
| `/tmp` | An empty, temporary folder. |
| `/proc`, `/dev` | New, for the sandbox. There is no `/sys`. |
| `/etc` | Only `passwd` and `group` (your user and group), `hosts` (`localhost`), and `ssl/certs/ca-certificates.crt`. |
| `/bin/sh`, `/usr/bin/env` | bash and coreutils' `env`, so that scripts' first lines work. |
| The Nix daemon's socket | Never. A Nix trusted user can use the daemon to act as root. |

The sandbox has its own process, network, user, IPC and hostname namespaces, and stops when the launcher
stops.

### Protected paths

The sandbox never writes what decides the sandbox, or what the host runs. Each of these is read-only, and a
mount point, so it cannot be renamed, removed or replaced either:

- Each match of `protect`: graft's base (`flake.nix`, `flake.lock`, `graft.nix`, `.gitmodules`, `.envrc`,
  `justfile` and `.pre-commit-config.yaml`, at any depth), and the providers' and consumer's entries.
- The git metadata that tells host git where a repository is or what to run. In each git directory (the top
  `.git`, each scion's under `.git/modules/`, each worktree's, and that of any repository in the working
  tree): `config`, `config.worktree`, `hooks/`, `info/`, `commondir` and `gitdir`. Every `.git` file (each
  scion's). The `core.hooksPath` folder, when it is in the repository.
- Each folder between the repository's top and a protected path, so that the folder cannot be renamed
  either. Such a folder stays writable.

Objects, refs, logs, the index and `HEAD` stay writable, so the command can commit.

The startup line counts the protected paths, the git metadata included. To change a protected file, edit it
outside the sandbox.

When a protected path, or a folder on the way to it, is a symbolic link, the launcher stops before the run:
it cannot make a link read-only. Replace the link with what it points to, or remove it. When a protected
file has another hard link, the launcher stops too, because a run could write the file through that link.
Replace it with a copy of its own: `cp -p F F.copy && mv F.copy F`.

The launcher can create empty folders in the host's git metadata before the run, so that the run cannot
create them: `hooks/` in a git directory that has a `config` and no `hooks/`, and the `core.hooksPath`
folder when it is missing.

### What the sandbox does not protect

- **The repository's history.** `rm -rf .git/*` inside deletes objects and refs. Only the protected files
  stay. Push what you need to keep.
- **A planted repository.** A command can make a new repository in any writable folder (`dir/.git/`), or a new
  gitlink with a `.git` file. Host git that runs in that folder later (a shell prompt's `git status`, an
  editor) runs what its config names. Before the run, the launcher records every `.git` entry, git directory,
  gitlink in the index, and match of `protect`. After the run, it prints a warning on stderr for each one that
  is new, with paths relative to the repository's top. The exit status does not change:

  ```
  graft: sandbox: WARNING: the run created what host git, a shell prompt, an editor or direnv reads, and may run code from (in ~/team-kb):
    .envrc
    evil/.git
    evil/.git/config
    evil/.git/hooks
    evil/.git/info
  graft: sandbox: read a new repository's config before anything runs git in it: git --no-pager config --file DIR/.git/config --list | cat -v (git -C DIR would obey that config); remove one you didn't expect: git clean -ffd DIR (plain -fd keeps a nested repository)
  ```

- **A new file with a protected name.** A new `.envrc` or `justfile` is reported after the run, not
  prevented. So is a `config.worktree` that the run creates.
- **A script in the working tree that git config names.** A filter, a `textconv`, or `core.fsmonitor = ./x`
  runs what the run wrote there, when host git runs. Add the script to `protect`.
- **The command line.** A process in the run can read the command and its arguments in `/proc/1/cmdline`.
  The environment is not there.

## Network

- `network.allow = [ ]` (the default): the sandbox has no network, only its own loopback.
- Otherwise the launcher starts a proxy on the host. It allows a `CONNECT` (HTTPS), or a plain HTTP request,
  only to a host and port that `network.allow` (or `--allow`) matches. It refuses the rest with
  `HTTP/1.1 403 Forbidden (graft sandbox: not in network.allow)`, and a host that it cannot reach with
  `502 Bad Gateway`. A request whose host is not a plain host name or IP address gets `400 Bad Request`.
  Inside, the proxy is at `http://127.0.0.1:3128`.
- `HTTPS_PROXY`, `https_proxy`, `HTTP_PROXY`, `http_proxy`, `ALL_PROXY` and `all_proxy` point at the proxy;
  `NO_PROXY` and `no_proxy` are `localhost,127.0.0.1,::1`. The proxy stops when the run ends.
- Only programs that use these variables reach the network. Name resolution is the proxy's: the sandbox has
  no DNS. ssh, `git push` over ssh, and other protocols fail.
- After the run, the launcher prints the refused connections, with a count for each:

  ```
  graft: sandbox: the proxy refused: github.com:443 (1); network.allow lists the hosts a run may reach
  ```

  A refused request with a host that is not valid is counted as `(malformed)`.

## Environment

The launcher starts the command with an empty environment, then sets:

| Variable | Value |
|---|---|
| `PATH` | Each `bin/` of the closure's roots (and of `--with`). |
| `HOME` | Your home's path (an empty folder). |
| `USER`, `LOGNAME` | Your user name. |
| `SHELL` | The sandbox's bash. |
| `TERM`, `COLORTERM` | The caller's, when set. |
| `LANG` | `C.UTF-8`. |
| `SSL_CERT_FILE`, `NIX_SSL_CERT_FILE` | `/etc/ssl/certs/ca-certificates.crt`. |
| `TERMINFO_DIRS` | The closure's terminfo. |
| `GRAFT_SANDBOX` | `1`. |
| `PS1` | `(sandbox) \w \$ `. |
| `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL` | See [git identity](#git-identity). |
| `GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_*`, `GIT_CONFIG_VALUE_*` | `commit.gpgsign` and `tag.gpgsign` set to `false`. |
| The proxy variables | When the run has network. |
| `env.set` | Each value. |
| `env.pass` | Each name that the caller has set, with the caller's value. |

The launcher gives bubblewrap its arguments and these values through file descriptors, not its command
line, so `ps` does not show a passed value. bubblewrap itself starts with an empty environment, so
`/proc/1/environ` in the run is empty.

### git identity

Your global git config is not visible inside. The launcher reads `user.name` and `user.email` with
`git config` in the repository (your global config included) before the run, and sets them as the four
`GIT_AUTHOR_*` and `GIT_COMMITTER_*` variables. Commits and tags made inside are not signed: the signing key
is not visible. When your git config sets `commit.gpgsign`, the startup line says `commits unsigned`.

## Startup and the terminal

Before the command starts, the launcher prints one line on stderr. `GRAFT_SANDBOX_QUIET=1` in the caller's
environment stops it. For example:

```
graft: sandbox: ~/team-kb, 11 paths protected; network: none; 1 request not granted: sandbox --explain
```

```
graft: sandbox: ~/team-kb, 11 paths protected; also writable: ~/.claude; network: *.anthropic.com, api.example.com; env: MY_TOKEN; absent, skipped: ~/.claude; commits unsigned
```

The parts, in this order. The repository and `network:` are always there; the others only when they apply:

| Part | Meaning |
|---|---|
| `~/team-kb, N paths protected` | The repository (`~` for your home), and the number of [protected paths](#protected-paths). With `repository.readOnly = true`: `~/team-kb read-only`. |
| `also writable: …` | `paths.writable`. |
| `network: …` | `network.allow` and `--allow`, or `none`. |
| `env: …` | The `env.pass` names that the caller has set. |
| `absent, skipped: …` | `paths.readOnly` and `paths.writable` entries that do not exist on this host. |
| `commits unsigned` | Your git config signs commits, and the run cannot. |
| `N requests not granted: sandbox --explain` | Requests from providers that `grant` does not name. |

The run keeps your terminal: job control works, and Ctrl-C reaches the command alone. The command's exit
status is the launcher's, and the checks after the run still print. A seccomp filter refuses the `TIOCSTI`
and `TIOCLINUX` requests, so the command cannot type into the shell that started it, whatever the kernel's
`dev.tty.legacy_tiocsti` says. The launcher prints nothing about it.

## --explain

`sandbox --explain` prints each option's effective value, and for each value the file that set it:

| Source | Named as |
|---|---|
| graft's base | `graft` |
| A provider | `<dir>/<name>/graft.nix` |
| The consumer | `flake.nix (perSystem)` |
| `--with`, `--allow` | `command line`, in a separate list at the end |

For example, in a repository with one provider that requests a host:

```
sandbox policy: each value with the file that sets it (graft is graft's base)

packages
  ripgrep-14.1.1                          scions/notebook-kit/graft.nix
  bash-interactive-5.2p37                 graft
  coreutils-9.7                           graft
  findutils-4.10.0                        graft
  gnugrep-3.11                            graft
  gnused-4.9                              graft
  diffutils-3.12                          graft
  less-668                                graft
  git-minimal-2.50.1                      graft
  tools                                   graft

env.set
  (none)

protect
  notebook/tasks/index.md                 scions/notebook-kit/graft.nix
  **/flake.nix                            graft
  **/flake.lock                           graft
  **/graft.nix                            graft
  **/.gitmodules                          graft
  **/.envrc                               graft
  **/justfile                             graft
  **/.pre-commit-config.yaml              graft

repository.readOnly
  false                                   graft

paths.readOnly
  (none)

paths.writable
  (none)

network.allow
  (none: no network)

env.pass
  (none)

grant
  (none)

requested, not granted (inert until perSystem's sandbox.grant names the scion)
  network.allow api.example.com           scions/notebook-kit/graft.nix

Always (bubblewrap): the closure read-only, and no other store path; the repository around the
current directory (read-only with repository.readOnly = true); the git metadata that says where
the repository is and what host git runs, read-only; HOME empty; /tmp empty; no Nix daemon; the
caller's terminal, but no typing into it (TIOCSTI); the environment, only what the policy names.
```

With `--with` or `--allow`, a list `command line (this run only)` follows. With a `paths.*` entry that does
not exist, a list `absent on this host (skipped)` follows. Add the scion's name to `grant` to apply its
requests.

`--explain --json` prints the same as one JSON object: `options`, with a list of `{ "value", "file" }` for
each option (a package also has its store `path`); `notGranted`, a list of
`{ "option", "value", "file", "scion" }`; and `absent`. Without the command-line entries and `absent`, it is
also `nix eval --json .#packages.<system>.sandbox.explain`.

`packages.<system>.sandbox.policy` is the policy as a renderer reads it: `packages`, `env.set`, `env.pass`,
`protect`, `repository.readOnly`, `paths.readOnly`, `paths.writable`, and `network.allow` as hosts and ports
(`[ { host = "api.example.com"; ports = [ 443 80 ]; } ]`).

## Exit status

- The command's exit status, also after Ctrl-C.
- `127` when the command is not on the sandbox's `PATH`.
- `1` when the launcher cannot start the sandbox (no user namespaces, outside a git repository, a symbolic
  link or a hard link on a protected path, macOS), with a message on stderr. It never runs the command without the sandbox.
- `2` for a usage error: the message, then the usage line.
- `130` for Ctrl-C before the run starts (while `--with` builds, for example).

## Messages

Evaluation, from mkFlake, each after Nix's `error:`:

- `graft: packages sandbox (from NAME): that name is graft's; the provider must rename it`, and the same for
  `apps sandbox`. For `perSystem`: `graft: packages sandbox (from self): that name is graft's; rename the repo's own`.
- `graft: mkFlake's outputs has apps.<system>.sandbox: that name is graft's; rename the repo's own`, and the
  same for `packages.<system>.sandbox`: `nix run .#sandbox` and `nix build .#sandbox` are always graft's
  launcher.
- `graft: sandbox: scions/NAME/graft.nix uses mkForce (or mkOverride below 100) on packages: a provider adds to the policy, never overrides it`
  (each option that it forces, separated by commas).
- `graft: sandbox: scions/NAME/graft.nix sets grant: only the consumer's perSystem may`, and the same for
  `repository`.
- `graft: sandbox: grant names NAME, which this repository does not graft`
- A value of the wrong type is the module system's error, which names the rule and the file:

  ```
  A definition for option `paths.readOnly."[definition 1-entry 1]"' is not of type `host path starting with "~/" or "/" (a path in the repository goes in protect)'. Definition values:
  - In `flake.nix (perSystem)': "docs"
  ```

  The rules for the other options: `path in the repository, relative to its top (no leading "/" or "~", no "..", no * ? [ but in "**/NAME")`
  for `protect`, `"host" or "host:port" (the host may use * and ?, as in "*.example.com")` for
  `network.allow`, `environment variable name` for `env.pass`, and `boolean` for `repository.readOnly`.
- A provider's `sandbox` that fails evaluation fails `packages.<system>.sandbox` only: `nix run .#graft`
  still runs, so you can repair the scion.

From the launcher, on stderr:

| Message | Exit |
|---|---|
| `graft: sandbox: unknown option --bogus`, then the usage line | 2 |
| `graft: sandbox: --with needs a package name`; `--allow needs a host`; `--json goes with --explain` | 2 |
| `graft: sandbox: --allow X: give "host" or "host:port" (the host may use * and ?, as in "*.example.com")` | 2 |
| `graft: sandbox: claude is not on the sandbox's PATH: add its package to sandbox.packages, or run with --with PKG` | 127 |
| `graft: sandbox: DIR is not in a git repository: a run writes to the repository around the current directory` | 1 |
| `graft: sandbox: justfile is a symbolic link, so the sandbox can't make it read-only (a run must not write what decides the sandbox or what the host runs): replace the link with what it points to, or remove it, then run again` | 1 |
| `graft: sandbox: .envrc has another hard link, so a run could write it through that link (a run must not write what decides the sandbox or what the host runs): replace it with a copy of its own (cp -p F F.copy && mv F.copy F), then run again` | 1 |
| `graft: sandbox: nix is a symbolic link, so the sandbox can't pin nix/agent-tools, below it (…)`: a folder on the way to a protected path | 1 |
| `graft: sandbox: bubblewrap can't create a user namespace here (above). In Docker, run the container with --security-opt seccomp=unconfined --security-opt systempaths=unconfined, or run on the host or a VM; elsewhere, allow unprivileged user namespaces (sysctl user.max_user_namespaces). graft never runs it unsandboxed.` Both Docker options weaken the container. | bubblewrap's |
| `graft: sandbox: --with PKG needs nix on PATH`; `graft: sandbox: --with PKG: nix build REF failed (above)` | 1 |
| `graft: sandbox: git rev-parse --path-format=absolute failed in REPO (above): the launcher needs git 2.31 or later` | 1 |
| `graft: sandbox: HOME is not set: a run gets an empty HOME, at the same path` | 1 |
| `graft: sandbox: the network proxy did not start: LOG`; inside, `graft: sandbox: the network relay (socat) did not start` | 1 |
| `graft: sandbox: not yet on macOS (Linux bubblewrap only)` | 1 |

After the run: the [proxy's refusals](#network) and the [warning about new git metadata](#what-the-sandbox-does-not-protect).

In the startup line, the messages and the lines after the run, the launcher shows each control character
(and each byte from 0x80 to 0x9f) as `\xNN`, so that a file name that a run made cannot change your
terminal. A UTF-8 character that holds such a byte, such as `€`, is shown escaped too.

## Deployed use

- Build the launcher once, and run its out-link: `nix build .#sandbox -o result-sandbox`, then
  `ExecStart=…/result-sandbox/bin/sandbox CMD …` in a systemd unit, or the same in a job. Nothing is evaluated
  at run time.
- `~/` is the home of the user that runs the service. A service with `DynamicUser` has none.
- In Docker, the sandbox needs user namespaces: see [Messages](#messages). The launcher fails, and never falls
  back to an unsandboxed run.
- `--with` needs Nix on the host: use it while you develop, not in a service.
- `--dry-run` prints the bubblewrap command that a run would use. It shows each `env.pass` value as
  `"$NAME" (env.pass)`, never the value.

## Not yet

macOS (Seatbelt), a systemd renderer, profiles (`sandbox.profiles.<name>`), and the rest of the list in
[Decisions](../decisions.md#enforce-the-boundary-in-a-sandbox-not-in-rules).

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the `sandbox` option from each `graft.nix` and `perSystem`,
  reserved names, `packages.<system>.sandbox` and `apps.<system>.sandbox`, the dev shell
- [lib/sandbox/options.nix](../../lib/sandbox/options.nix): the options and their types
- [lib/sandbox/policy.nix](../../lib/sandbox/policy.nix): merging, requests and grants, `--explain`
- [lib/sandbox/default.nix](../../lib/sandbox/default.nix): graft's base, the macOS stub
- [lib/sandbox/bwrap.nix](../../lib/sandbox/bwrap.nix) and [lib/sandbox/launcher.sh](../../lib/sandbox/launcher.sh):
  the bubblewrap launcher, protected paths, the startup line and the warning after the run
- [lib/sandbox/proxy.py](../../lib/sandbox/proxy.py): the network proxy
- [lib/sandbox/tiocsti.py](../../lib/sandbox/tiocsti.py): the seccomp filter
- [tests/run.sh](../../tests/run.sh): the `sandbox` section
