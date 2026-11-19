# Changelog

## Unreleased

- **pot: the `pi` kit.** `graft.kits.pi { pkgs; providers; settings; endpoints; files; … }` runs the pi coding
  agent in pot, named `pi` in a root (or `pi@NAME` for several) ([pot's reference: pi](docs/reference/pot.md#pi)).
  - Its reach is computed from the providers: each `baseUrl` (provider and model) and endpoint is a `network`
    request, each `"${NAME}"` key a `credential` for its host, and nothing else. A provider on the host's loopback,
    one with no `baseUrl`, or a command key (`"!…"`) fails evaluation, naming it.
  - The run's pi home is its own state, with pi's configuration fixed by the root: `settings.json`, `models.json`,
    the home-manager files, and `auth.json` and `mcp-auth.json` as `{}` (OAuth providers and `/login` work only
    on the host). `/model` switches within a session and does not persist. Extensions, skills, prompts, themes and
    installed packages are read-only. pi starts with `PI_OFFLINE=1`.
  - A key named by a variable (an `apiKey`, a header, an endpoint's `key`) travels only over https; header values
    follow `apiKey`'s rules. A pi run's denial names the root's pi kit as its first fix.
- **pot never reaches the host's loopback.** The proxy resolves each host once and refuses it when an address is
  loopback or unspecified, granted or not, so the host's own name and a name that rebinds to `127.0.0.1` are
  refused like `127.0.0.1`; it connects to the checked address. Before, a granted loopback host or a name that
  resolved there was reached (every kit and `--net`).
- `~/.pi/agent/mcp-auth.json` joins pot's hidden floor.
- `kit/1` and `pot-kits/1` are unchanged (the second quiet release), and no existing grant asks again.

### Upgrading

- Nothing is required. To run pi in pot: add `kits.pi` to the root, bind each `credential` that
  `pot explain pi` lists (`credential."HOST kit=pi"`), rebuild the root, and run `pot grant pi`
  ([Set up a root](docs/guides/run-in-a-sandbox.md#set-up-a-root)).

## v2.18.0 - 2026-11-18

pot, graft's sandbox (experimental), in three parts:

- **pot runs on macOS again.** A Seatbelt backend replaces the refusal that v2.15.0 to v2.17.1 printed there. It is
  weaker than bubblewrap, and says so: no process or network isolation, no resource limits, and no nesting.
- **Kit commands run under their own policy.** Each command a kit declares gets a wrapper in the consumer's
  `packages.<system>.tools`, which runs it in pot: quietly on the host, nested inside an agent's run, so a kit tool,
  a `just` recipe or a git hook never gets the agent's token or MCP. graft generates `<scions>/kits.just` from the
  kits' recipes, and a hook command (`commands.<name>.hook`) is what `.githooks/<hook>` execs.
- **Runs nest (Linux).** A run's children table, composed by the host, says for each command it may start whether
  it execs, nests or is refused. `pot dispatch` works inside a run: its clone is kept in host state, and you fetch
  it on the host with `pot review PARENT/CHILD --fetch`. Seatbelt cannot nest, so on macOS a row that would nest is
  refused, never flattened.

`kit/1` and `pot-kits/1` are unchanged (the first of the two quiet releases pot's split waits for). Linux runs keep
their canonical grant items and digests (`pot-golden` holds them). **No grant asks again** when a root takes
v2.18.0; a consumer that takes it asks once for its own repo kit, and a kit whose commands request nothing needs no
grant ([Move to v2.18.0](docs/guides/move-to-v2-18.md)).

### Upgrading
- **Roots first:** take v2.18.0 and `home-manager switch` (on a Mac, rebuild), so `~/.config/pot/launcher` names a
  pot that runs wrappers. A consumer that moves first still works: its wrappers run unwrapped, with one stderr line
  naming the fix. On a Mac, then `pot doctor`, and `pot grant` as on Linux; grants carry over.
- **Consumers:** `graft sync` writes `kits.just`; commit it. Import it after any provider's own `.just` file (just keeps the
  first of two recipes with one name; both run the same wrapper), and
  define `tools` as the tools package's root (`kits.just` runs `{{tools}}/bin/<command>`). Make `.githooks/<hook>`
  exec the hook command's wrapper, and set `core.hooksPath` relative (`git config core.hooksPath .githooks`) so
  dispatch clones carry it.
- **Anything that must not run pot** (CI, Nix checks, containers, a scheduler's worker) takes
  `packages.<system>.tools.passthru.unwrapped`.
- **Providers:** declare `commands`, `recipes` and `hook` ([Declare kit commands](docs/guides/kit-commands.md)).
- **On macOS, `git commit` inside `pot claude` stops at a wrapped pre-commit hook**, because the hook command would
  nest: commit on the host, or `git commit --no-verify`. With pot still off on a Mac (no launcher), wrappers run
  unwrapped, and commits there are unaffected.

### Added
- **The Seatbelt backend** (`pot/core/seatbelt.py`, `/usr/bin/sandbox-exec`): a Darwin root's backend is `seatbelt`.
  Its profile denies by default and is written at each start from `policy.json`, every host path a parameter.
  It proves `closure-store`, `no-tiocsti`, `in-place` and `private-proxy`; the startup line says
  `seatbelt lacks own-netns own-pidns`, and `pot explain` lists every lack. HOME is the run's own folder, where links
  stand in for state, fixed files and fixed directories; protected paths are path rules (no write, no new hard
  link, no rename or removal); `inspect` reads allow-then-deny; the network is the proxy's port on the loopback with
  a per-run token (`407` without it); the run is its own process group, stopped with what left it when the command
  exits ([On macOS](docs/reference/pot.md#on-macos)). A run can read the arguments and environment of this user's
  other processes (Seatbelt does not check `KERN_PROCARGS2`): [residual 22](docs/reference/pot.md#residual-risks).
- **The proxy's token** (`POT_PROXY_TOKEN`, from the backend): every request must carry it as its
  `Proxy-Authorization` password. bubblewrap's proxy, on a socket only its run sees, has none.
- **Wrappers** for every declared kit command (any kit minor), in `tools` and `devShells.default`; `apps` and
  `scions.<name>.tools` stay unwrapped. `tools.passthru.unwrapped` (also `tools.unwrapped`). A wrapper runs the
  launcher's pot (bubblewrap or Seatbelt). With no launcher (pot is still off on the Macs), or under a root older
  than v2.18.0, it runs its program unwrapped, with `pot: NAME runs without pot (…)` on stderr. Inside a v2.18.0
  run, policy.json decides first: the wrapper execs the run's own pot beside it (`/run/pot/launcher`; under
  Seatbelt `<run dir>/launcher`), or exits 126 (`pot: NAME: this run's launcher (/run/pot/launcher) is missing, so
  its children table can't be reached: run NAME on the host`); an older pot's run gets the program under its policy
  ([Wrapped commands](docs/reference/pot.md#wrapped-commands)).
- `pot run --command NAME --manifest M -- ARGS`, the wrappers' entry, no longer reserved. It runs in the root's
  `defaultContext` with the command's `access`; a `read` command runs in a root repository too.
- `<scions>/kits.just`, written by `graft add`, `sync` and `upgrade` (through a temporary file; a link is refused)
  and compared by `graft check`, where the repository uses it: the file exists, or the top `justfile` names it.
  Arguments are single-quoted. A kit recipe with the name of one of the justfile's own fails `graft check`
  ([Files](docs/reference/files.md#kitsjust)).
- Recipe checks in every kit version: a name `[A-Za-z][A-Za-z0-9_-]*`, literal words of
  `[A-Za-z0-9._/=:,@+%-]`, one space apart (`pot: FILE: recipes.N: literal words only (letters, digits and . _ / =
  : , @ + % -, one space apart)`).
- Hook commands: `GIT_INDEX_FILE` passes only when it names this repository's index
  ([Hook commands](docs/reference/pot.md#hook-commands)). Dispatch clones carry a relative, tracked
  `core.hooksPath`, so their commits run the hook command too.
- The children table in `policy.json` and `pot explain`'s `children` row; nested children in bubblewrap, with a
  proxy that forwards to the parent's; the `nest` guarantee. A child that needs write access under a read run is
  refused (`needs write access`); a nested child's denials name its kit's `requests.network`, with no `one run:`
  line. On macOS, every row that would nest is refused (`pot (work): check-kb runs under its own, narrower policy,
  and Seatbelt cannot nest (a run on macOS starts no runs inside it): run it on the host (just check-kb)`); a row
  whose policy equals the run's still execs in place ([Nesting](docs/reference/pot.md#nesting-and-the-children-table)).
- Nested `pot dispatch` and `--detach` (Linux), through the run's `dispatch` rows: the child's clone in
  `runs/<parent>/children/`, kept after the parent, staged by the host itself once the parent has ended;
  `pot review PARENT/CHILD --fetch` into `refs/remotes/pot-PARENT-CHILD/*`; inside the parent, `pot runs` lists
  children and `pot review CHILD --json` shows their commits (a claim); children killed with their parent, logged
  by the host (`killed: "parent"`). Limits: only the child's top repository is staged (not its scions),
  `clones.maxSize` does not count children, and a grandchild gets no children table
  ([Nested dispatch](docs/reference/pot.md#nested-dispatch)).
- `pot doctor`: `nest` among the proved guarantees; with no launcher, `~/.config/pot/launcher is missing: a kit
  command's wrapper here runs its program without pot, and says so`.
- policy.json's `mounts.launcher`: every v2.18.0 run has its own pot beside its policy, read-only.

### Changed
- A Linux run keeps user namespaces when its children table has a `nest` row (every agent run); other runs keep
  `--disable-userns`. The backend is in every run's closure.
- **A dispatch on a backend without `same-path-clone` runs the clone at its own path**, said on the startup line
  (`clone at its own path`), where v2.17.0 refused it. A context that `requires` `same-path-clone` still exits 125.
  bubblewrap proves it, so Linux dispatch is unchanged.
- `pot explain`'s note is now `a kit command started in this run (its wrapper) runs under its own policy, from the
  children table`; on macOS its row reads `children   none (Seatbelt cannot nest: a run on macOS starts no runs
  inside it)` when its table is empty.
- Messages inside a run: `pot dispatch` and `pot run` work (Linux); `pot review --fetch`/`--discard` say they are
  the host's. On macOS, `pot` inside a run refuses: `pot (work): Seatbelt cannot nest: a run on macOS starts no
  runs inside it (pot dispatch, pot CONTEXT CMD); run it on the host`.
- `pot runs` hides wrapped runs that exited 0 (`--all` shows them).
- Every root's drvPath changes: its pot package gains `share/pot/run-1`, the marker wrappers look for.
- **Cost:** a wrapped command on the host takes 0.30–0.45 s on platform-vm, against 8 ms unwrapped; a nested child
  adds about 0.2 s. Each root byte-compiles pot and caches closures and the scope probe.
- **The claude-code kit, on macOS only:** its denial hooks read the denials file in the run's folder
  (`${POT_POLICY%/*}`), and `CLAUDE_CODE_TMPDIR=~/.tmp` keeps Claude out of the host's `/tmp/claude-UID`. On Linux
  the kit is byte for byte v2.17.1's.
- The run's shims find their sockets beside `POT_POLICY` (`/run/pot` under bubblewrap, as before).

### Fixed
- **A host `core.fsmonitor` could run in a dispatch clone on the host.** The repo kit's grant source, read from the
  clone's index with `git write-tree`, ran without `core.fsmonitor=false`. Every host git call on a clone now runs
  with `core.fsmonitor=false` and no hooks, so no hook or filter of the clone's runs on the host (tested with a
  global `filter`, `textconv` and `fsmonitor` that each write a marker).
- **Seatbelt, before release:** the host's look at a clone runs git under the run profile's last denies too
  (`TIOCSTI`, the fcntls), in a session of its own with stdin `/dev/null`; leftover processes are found by
  `sandbox_check` (sandboxed, and able to read this run's `policy.json`) as well as by the run's `TMPDIR`, which is
  matched only in a process's environment, and stopped with TERM, then KILL, until a pass finds none; the folders
  pot makes on the way to a link in the run's HOME are kept like the link, so a run cannot rename them.
- pot's unit tests pass on macOS: their `/home/u` fixtures were rewritten by `realpath`, as `/home` is an automount
  there.


## v2.17.1 - 2026-11-18

pot, graft's sandbox (experimental): the claude-code kit can use an Anthropic-compatible gateway, and a run checks
its binding files before it asks for grants or makes a clone. Nothing else changes: no grant asks again, and the
anthropic and Bedrock instances' canonical grant items and digests are unchanged (`pot-golden` holds them).

### Added
- **`graft.apis.gateway { url; }`**, a third API for the claude-code kit: a gateway at an `https://HOST[/PATH]` url
  that takes its key as a bearer token, such as a Bifrost or LiteLLM proxy. The run reaches only the gateway's host,
  on port 443 (no `claude.ai`). Claude gets `ANTHROPIC_BASE_URL=<url>/anthropic` and
  `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY=1`, so `/model` lists the gateway's models. The credential carries
  `ANTHROPIC_AUTH_TOKEN`: bind it to the key's file. `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is not set, since it
  would stop the model discovery too; `DISABLE_TELEMETRY`, `DISABLE_ERROR_REPORTING` and `DISABLE_BUG_COMMAND` are set
  in its place. `agent.select.env` adds the `ANTHROPIC_BASE_URL`, so a shell set up for the gateway picks the gateway
  instance over one that shares its config folder. A url that is not https, or names a port other than 443, fails
  evaluation: `graft.apis.gateway: url = "http://…": give https://HOST[/PATH] (pot reaches HOST on port 443)`
  ([the claude-code kit](docs/reference/pot.md#claude-code)).

### Fixed
- **A missing token file was found only after the grant prompts, and a dispatch left its clone behind.** A run now
  checks each `from.file` of the credential and mcp bindings its kits use (granted or not yet) after it composes,
  before the grant prompts and before a dispatch uses its clone. A missing or unreadable file stops the run (exit 1),
  with the binding's `refresh` as the fix:
  `pot (work): ~/.config/pot/claude-token-flexion is missing (the credential binding api.anthropic.com
  kit=claude-code@flexion): claude setup-token, saved with mode 600, then start again`, else `create it, with mode
  600`. A dispatch refused there leaves no clone and no run folder (v2.17.0 kept a stopped run with its clone,
  about 190 MB for flexion-assistant). The file is checked by path and mode alone, never opened. `pot explain` and
  `pot why` do not check ([the binding check](docs/reference/pot.md#key-files)).

### Upgrading
- Nothing to change. To use a gateway, add an instance with `api = graft.apis.gateway { url = "…"; }` and a
  credential binding for its host, then grant it once: `pot grant claude-code@NAME`
  ([Run in a sandbox](docs/guides/run-in-a-sandbox.md)).

## v2.17.0 - 2026-11-18

pot, graft's sandbox (experimental), in three parts:

- **Dispatch: unattended work in a clone.** `pot dispatch CMD` runs a command in a throwaway clone in pot's state.
  Its commits reach your repository only when you review and fetch them (`pot review ID --fetch`, into
  `refs/remotes/pot-<id>/*`). Every run leaves a record that `pot runs` reads (contract
  [runs/1](pot/contracts/runs-1.md)). A 125 is recorded as `stopped` and turns its workmux pane `waiting`, and runs
  without a terminal get memory and task limits.
- **Grants and doctor.** `pot grant` with no argument offers what stopped runs wait for, and `--commit` commits the
  grants file. A repo kit's grant now holds across changes outside its defining files. `pot doctor` checks what
  runs depend on. An unverified scion no longer stops a run.
- **The removals and the contract additions.** The six forms that v2.16.0 deprecated are gone, each with an error
  that names its replacement. kit 1.2, pot-kits/1's new bundle files and pot-lib/1's new argument fix every shape
  that v2.18.0's wrapped kit commands need, so v2.18.0 and v2.19.0 can leave `kit/1` and `pot-kits/1` unchanged:
  pot's split waits for two such releases.

**No grant asks again** when a root takes v2.17.0 (one caveat below), and a consumer still on v2.16.x keeps working
under it ([Move to v2.17.0](docs/guides/move-to-v2-17.md)).

### Upgrading
- **Roots: replace the removed forms first.** `defaultAccount`, the claude-code kit's `account`, `provider` and
  `region`, and `--account` now fail, one at a time, with an error that names the replacement
  ([Removed](#removed)). A root that builds on v2.16.x with no evaluation warning from graft needs no edit.
- **No re-grant.** grants/1 does not change: pot still writes a repo kit's source as `{ "tree": … }`, so a v2.16.x pot
  sharing the file reads what v2.17.0 writes, with no prompt. Only the comparison narrows ([Changed](#changed)). The
  caveat: pot learns a repo kit as built at the first v2.17.0 run whose tree equals its grant's, so a repository
  that has moved past its granted tree before that first run asks once.
- **A consumer that takes v2.17.0** changes its `flake.lock`, so its repo kit asks once: `pot grant NAME`. No other
  kit asks.
- **Providers and consumers: `kit.packages` is refused where a kit is built** (kitsFor, mkRoot): rename it to
  `tools`. A bundle that graft v2.16.x built still reads, `packages` included, so nothing that works today stops.
- **`graft-kits` is gone.** A repository whose graft is older than v2.16.0 (it has `graft-kits` and no `pot-kits`)
  is refused, not run without its protections: take graft v2.16.0 or later there, or run `pot host CMD`.
- **Dispatch runs have their own context.** It does not include `work`'s standing kit: give it with
  `contexts.dispatch.include = [ "NAME" ];` (the kit's existing grant applies).
- **An unverified scion no longer stops a run.** A scion whose gitlink is not a release tag at its URL gets one
  startup note, and its kit's reach does not apply; its command runs with its see items. It was exit 1 for its
  command, and 125 where its grant applied.
- Every root's drvPath changes (new `limits` and `clones` data); no grant does.

### Removed
- **`packages.<system>.graft-kits`:** mkFlake no longer writes it, and pot no longer reads it:
  `pot (work): ~/DIR has graft-kits only: its graft predates v2.16.0, and this pot reads pot-kits: take graft v2.16.0
  or later there (nix flake update graft), or run pot host CMD`. The name stays reserved.
- **`kit.packages`,** at the source: kitsFor refuses it in a provider's or a consumer's kit, `pot: FILE: packages is
  tools' old name, gone in v2.17.0: give tools`, and mkRoot in a root's, `pot: the root (programs.pot.kits.NAME):
  packages is tools' old name, gone in v2.17.0: give tools`. A bundle that graft v2.16.x built is still read, with
  `packages` as `tools` and no warning.
- **`--account NAME`:** `pot: --account is gone (v2.17.0): --instance NAME, or CMD@NAME` (exit 2).
- **`defaultAccount`:** `graft.lib.mkRoot: defaultAccount is gone (v2.17.0): defaultInstance.<kit> = "NAME"`.
- **`programs.pot.defaultAccount`:** a removed option: home-manager stops with ``The option definition
  `programs.pot.defaultAccount' in `FILE' no longer has any effect; please remove it.`` and `Use
  programs.pot.defaultInstance.<kit> = "NAME" (v2.17.0).`
- **The claude-code kit's `account`:** `graft.kits.claude-code: account is gone (v2.17.0): the instance is the root's
  kits key ("claude-code@NAME")`.
- **The claude-code kit's `provider` and `region`:** `graft.kits.claude-code: provider and region are gone (v2.17.0):
  api = graft.apis.bedrock { region = "…"; } (or graft.apis.anthropic)`.
- The flake check `pot-aliases` is replaced by `pot-removed`: each removed form fails with its message.

### Added: dispatch
- **`pot dispatch [--from REF] [--detach] CMD`,** and `--clone` (`--mode clone`) for another context
  ([pot: dispatch](docs/reference/pot.md#dispatch), [Work in a clone](docs/guides/dispatch.md)):
  - the clone is in pot's state (`runs/<id>/clone/`), at the base (`--from`, default `HEAD`), keeping only the base's
    branch; the first level of submodules is cloned too, each at its gitlink only. It carries `user.name`,
    `user.email` and the origin URL, and none of your other config or hooks;
  - it is mounted at the repository's own path (`same-path-clone`), with `POT_CLONE=1`; kits come from the base
    commit, never the working tree; uncommitted changes stay behind, with a notice;
  - `--detach` runs in the background after every visible check, and prints the run's id on stdout;
  - at the run's end, the host lists the clone's refs and fetches each branch alone through a sandboxed
    `git upload-pack` with fsck into its own staging, and runs no hook, config or filter of the clone's. The exit
    line counts each moved branch's and each scion's commits: `pot: run cdf8 · 1 commit on main, 2 on side, 1 in
    nix/agent-tools/.scions/meeting-kit: pot review cdf8, then pot review cdf8 --fetch`. A ref that fails to stage
    is an ATTENTION item quoting git (`fetch failed: refs/heads/bad: error: bogus commit object …`), the exit line
    says so (`pot: run 5d5b · 1 commit on main · fetch failed: …: pot review 5d5b, then pot review 5d5b --fetch`),
    and the run stays in `review`; so does a run whose host look itself fails (`fetch failed: the host's look
    failed (…)`), listed and kept. Host git reads names as bytes, splits at newlines only, and takes every pathspec
    literally, so a branch or path that is not UTF-8 or holds U+0085 works (`1 on b\xfe`), and a file named
    `:(exclude)*` hides nothing;
  - a run whose pot was killed gets that look from the next `pot runs` or `pot review`, one pot at a time
    (`recovering · pot runs ID again` while another pot completes it);
  - allowed in a root repository, where its branch is the way to change the root (D13); `pot review` names a changed
    file that holds `programs.pot` (`root: modules/pot.nix (programs.pot)`) and the grants file
    (`root: pot/grants.json`). `--repo` is refused in clone mode (a later release), a `--repo` that is a root first
    with the root refusal;
  - one-run hints name the context: `pot dispatch --net HOST …`, `pot dispatch --read PATH …`.
- **`pot review [RUN] [--fetch|--discard]`:** the runs that need a person (`review` and `stopped`, and ended runs with
  an ATTENTION item), with `--since`, `--until`, `--command`, `--flags` and `--json`; empty, `pot: no runs need
  review`. ATTENTION is the host's findings, never a bare commit count: `N unsigned` (when your git config signs,
  until fetched), `root: …`, `host-run: …`, `repo kit: …` (only the repo kit's own files), `gitlinks: …`,
  `N new git metadata paths`, `N denied: …`, `fetch failed: …`. `--fetch` fetches only the refs the run moved, names
  the base's branch first, and prints `review:`, then, when your git config signs, `sign:    git switch -c pot-ID pot-ID/main && git rebase
  --exec 'git commit --amend --no-edit -S' BASE`, then `take it: git merge pot-ID` (`git merge pot-ID/main` without
  signing). `--discard` removes the clone and keeps its log. `--approve` (phase 3) and `--root` (phase 2) are
  refused with their phase.
- **`clones.keepDays`** (default 14) and **`clones.maxSize`** (`"20G"`): they remove only clones that were fetched,
  discarded or have no commits, and never follow a link. pot never deletes unfetched commits on its own; over the
  cap, the startup line and `pot doctor` name the runs waiting for review. A dispatch stopped with 125 keeps its
  clone, and says so (`next       pot grant NAME · its clone is kept until 2026-12-02: pot review ID --discard`).
- **`pot runs [ID] [--json] [--follow] [--all]`** and contract **runs/1**: every run's record, computed from its log,
  with the states `running`, `exited`, `stopped`, `review` (and `pending`, reserved). The command is shown as you
  typed it (`command.typed`; `argv` stays pot's). New log events `stop`, `fetch` and `discard`; new record keys
  include `clone.nested`, `clone.fetchFailed`, `stop.grantedSince` and `recovering`.
- **A 125 is recorded** as a `stopped` run. When a kit that applies to the run requests `status.relay`, the root
  binds it with `workmux = true`, and the caller is in tmux with `workmux` on its `PATH`, it also sets the launching
  pane's workmux status to `waiting`. The 125 text is unchanged. Once its kits are granted, the run reads
  `stopped · granted since: start it again` and leaves `pot review`'s list.
- **Resource limits:** a run without a terminal, every `--detach` run, and a context that requires
  `resource-limits` start in `systemd-run --user --scope` with `MemoryMax` and `TasksMax` from the root's
  `limits.memoryMax` (default `"50%"`) and `limits.tasksMax` (`2048`). bubblewrap proves `resource-limits` for such
  a run only. `pot explain` gains a `resources` row. Without user systemd, the run starts with the note
  `no resource limits (no user systemd: pot doctor)`, and a context that requires the limits stops with 125, ending
  ` (resource-limits: a user systemd scope, for runs without a terminal: pot doctor)`.
- Every run (and `inspect`) first cleans up a killed in-place run: its sockets, and v2.16.2's placeholders that
  still hold what pot wrote. `inspect` starts when pot's state lies under `inspect.home` and holds sockets.
- v2.16.2's protections cover the clone: its `.git` and modules get the read-only placeholders, and write access to
  a root is refused in `dispatch` too (a dispatch's `--write` is a host path).

### Added: grants and doctor
- **`pot grant` with no argument** prompts for every kit that a `stopped` run waits for (in any workspace, its status
  computed there), then for this workspace's stale grants (asks for more, changed source), showing only the
  difference. A stopped run's kit is headed `(stopped run 8b3f)`, or `(stopped run 9427, in
  ~/src/flexion-assistant)`. A kit never granted that no run waits for is not prompted; one line names those:
  `pot: also not granted: claude-code@flexion, claude-code@personal (pot grant NAME)`. Nothing to prompt:
  `pot: nothing to grant`.
- **`pot grant --commit`** (opt-in on each call) commits the grants file's path alone in its repository, with that
  repository's hooks, and never pushes: `committed ~/.workstation/pot/grants.json (8982a53); push it with
  ~/.workstation's next push`. The body is `Granted on MACHINE, DATE (pot grant --commit).`; `--revoke KIT --commit`
  commits the revocation (`pot grant --revoke: KIT`, `Revoked on MACHINE, DATE (pot grant --revoke --commit).`).
  It is refused before any prompt when the path has uncommitted changes, is ignored, or is not in a git repository;
  a commit that fails unstages a file pot added for it. Flags may come in any position. Under an agent marker,
  `pot grant` is refused before it reads any argument; `pot grant --diff`, which writes nothing, is allowed.
- **`pot doctor`:** the backend and its guarantees (`resource-limits: for runs without a terminal` where user systemd
  answers; `PATH` named when its probe finds no `true` there), user systemd, the grants file, each binding's file by
  path and mode (never its contents), the instances and which one the shell selects, the workspace, pot's state and
  the launcher. A key file that others can read is marked (`chmod 600 it`); a missing one names its fix (the
  binding's new `refresh`, such as `claude setup-token, saved with mode 600`, else `create it, with mode 600`); a
  missing grants file reads `missing: pot grant writes it`; the launcher's fix fits the root's kind. Exit 1 when a
  line starts `!`. A run that meets the same probe failure says `… user namespaces were not tested, so no run
  starts`.

### Added: contracts
- **Kit 1.2** ([kit/1](pot/contracts/kit-1.md)), read now and used from v2.18.0: `commands.<n>.hook` (one of
  `pre-commit`, `commit-msg`, `prepare-commit-msg`, `pre-push`, `post-checkout`, `post-merge`) and the `recipes`
  attrs form `{ run; description; }`, whose first word must be one of the kit's commands. A 1.3 kit gets
  `pot: FILE: kit version 1.3 is newer than this pot reads (1.0, 1.1, 1.2): take a newer pot`.
- **pot-kits/1** fixes three bundle files for v2.18.0 (`kits.just`, `wrappers/bin/`, `commands/*.json`) and the rule
  that a reader ignores bundle files it does not know; it stays version 1, and a v2.16.x pot reads such a bundle.
- **pot-lib/1** gains `recipes ? { }` and `passthru.wrappers` (an empty package until v2.18.0).

### Changed
- **A repo kit's grant holds across changes outside its defining files.** The grant holds when its tree is equal, or
  when only files outside `flake.nix` and `flake.lock` changed and the kit as built is the one this machine recorded
  for the old tree (`cache/sources/` in pot's state). So a scion bump or a test file no longer asks again; a new
  `flake.lock` does. A changed source shows the defining files' diffstat when they changed, else the tree's, and the
  line `the kit as built changed` when the build differs.
- **Root repositories:** `pot (work): ~/.workstation is a root: pot dispatch CMD (its commits come back for review), or
  pot inspect CMD` (was `… pot inspect, or claude (unsandboxed) until pot dispatch (a later release)`).
- **The unverified-scion note:** `meeting-kit: gitlink is not a release tag at its URL; its kit is unverified and its
  grant does not apply`, or `meeting-kit: its URL did not answer, …`, in place of the refusal.
- **Exit status:** 125 no longer covers an unverified scion. `pot review`, `pot runs` and `pot doctor` are verbs, not
  usage errors; `--from` and `--detach` without clone mode, and `--account`, are.
- Inside a run, `pot runs` shows the run itself; `pot review` and `pot doctor` are the host's (exit 2); `pot
  dispatch` waits for nested dispatch, in v2.18.0 (exit 1).
- `--read` or `--write` of a folder that holds pot's state says `which holds pot's own state (~/.local/state/pot)`
  (was `which is pot's own state (launcher mounts are pot's alone)`).
- `nix flake init -t github:danielnaab/graft/v2.17.0` and the docs pin v2.17.0.

### Fixed
- **A repository or state folder whose path has a space, `#` or `?`** works: pot percent-encodes the repo kit's flake
  reference (it was `path:<dir>` as written, so such a checkout failed `the flake at … did not evaluate`). Messages
  show the path as before.
- **Unicode format characters** (bidi overrides and isolates, zero-width characters, the BOM) that a run chose are
  escaped as `\uNNNN` wherever pot prints them, so v2.16.2's known limit (a U+202E name on the exit line) is gone. A
  byte that is not UTF-8 in a path shows as `\xNN`.

## v2.16.2 - 2026-11-18

A security patch for pot, graft's sandbox (experimental). It closes two ways a run could change what the host does
after it, both present in v2.16.1. Nothing else changes: no grant asks again, no canonical grant item or digest
changes (`pot-golden` holds them), and the only new output is the refusal below.

### Fixed
- **A run could point host git at a config it wrote.** In a git directory that pot protects (the top `.git`, each
  worktree's, each scion's), the run could create an absent `commondir`, `config.worktree` or `gitdir`, and host git
  then read run-chosen config after the run (a `core.fsmonitor` command, say), or broke. pot now puts a neutral
  placeholder at each absent name before the run (`commondir` holding `.`, the others empty), binds it read-only
  like the names that exist, and removes it after the run if it still holds what pot wrote. The exit line's
  `WARNING: the run created …` no longer lists these names, since the run cannot create them. A pot killed
  mid-run leaves its placeholders behind: host git works with them, and the next run binds them read-only, so the
  protection holds; remove them by hand if you like.
- **A run could be given write access to a root repository.** `work` refused in a root, but a `--write` path, a
  granted `requests.write` item, or a `--repo` with write access that is, is inside or holds a root repository was
  allowed. Each is now refused at start (exit 1), naming the flag or the kit, and the root:
  `pot (work): --write ~/src/w holds the root ~/src/w/rootrepo: a run never writes a root (pot inspect reads it; pot
  host CMD runs unsandboxed)`, likewise `--write ~/.workstation is the root ~/.workstation`, `--write
  ~/.workstation/modules is inside the root ~/.workstation`, `KIT's write …` and `--repo …` (which said
  `… is a root: pot inspect, or pot host CMD until pot dispatch (a later release)`). A write item that holds a root
  is refused, not narrowed. Reading a root, `--repo` with `--access read` and `work`'s own refusal in a root are
  unchanged.

### Known limit
- A name that a run creates with a Unicode format character, such as U+202E (right-to-left override), still reaches
  the in-place exit line raw, so the terminal can show it reversed (`a…exe.jpg` for `a<U+202E>gpj.exe/.git`). pot
  escapes only control characters there. This is the shared terminal, residual 10 in the pot reference; v2.17.0 escapes these characters too.

### Upgrading
- Nothing to change. A run that relied on writing a folder that holds a root repository now stops: narrow its
  `--write` or `requests.write` to a folder beside the root, and change the root itself on the host.

## v2.16.1 - 2026-11-17

pot's output, made readable. **Text and layout only:** nothing changes in what a run can reach, in grants, in the
canonical grant items or their digests (`pot-golden` holds them), or in any `--json` shape (`policy.json` v1 and
`pot why --json` under run-env/1). A script that reads pot's `--json` needs no change; one that parses pot's text
output, or `/run/pot/denials`, may.

### Changed: pot's output
- **`pot grant --diff`** starts with a summary, `3 kits granted · 2 not granted` (zero counts left out), then one
  line per kit, the names aligned: a mark, the short name, the status and the command to run, such as
  `✗ claude-code@flexion          not granted (9 requests) · pot grant claude-code@flexion`. The marks are `✓`, `✗`
  and `~`; `ok`, `no` and `~` when the output is not a terminal or the locale lacks UTF-8. A kit's identity is shown
  only when its name does not imply it. The requests are listed only for a kit that asks for more (`     + …`) or
  changed source (`   source: A -> B` and the diffstat). The new `--verbose` adds every identity and a
  never-granted kit's requests.
- **`pot explain`** has six sections with a blank line between them (the run, see, limits, reach, not in run,
  backend), and wraps to the terminal's width (100 columns otherwise):
  - the run section adds `next`: `ready`, or the grant to make, such as
    `next       pot grant claude-code@personal (9 requests)`;
  - the overlay's hooks are one row, `hooks      PreToolUse, PostToolUse, PostToolUseFailure (Bash|WebFetch); SessionStart`,
    then `(pot's --settings overlay; /hooks does not list them)`;
  - `env.set`, `file` and `dir` rows are grouped under their kit's short name, and a `dir` row's left-out entries say
    `left out until KIT is granted:` when the kit is the cause;
  - each protected path appears once, with its labels joined: `.githooks (pot, repo)`;
  - the `not in run` items are filled under their row, not at column 72, one per line.
- **`pot why`, the denials file, the exit line's denials and the 127 line** say what is missing, then `one run:` and
  `permanent:` lines, and name a kit's home by the flake directory pot evaluated:

  ```
  pot (work): pot would refuse files.example.org:443 in a run here.
    one run:   pot --net files.example.org …
    permanent: requests.network in the repo kit (nix/agent-tools/flake.nix, perSystem)
               or programs.pot.kits.daniel.requests.network, for every repository (home-manager switch)
  ```

  A repeated denial says `(not in this run, 3x).`. The exit line's `pot: --net X was for this run.` gives its
  `permanent:` lines below.
- **A word that reads as a file name** and names no path gets
  `flake.nix: no such path here (pot why PATH answers paths; for a host, pot why flake.nix:443)`, never a `--net`
  fix. In `--json` it is `"kind": "file"` with `"reached": false`: values, not a new field.
- **A program the host lacks too** gets, from the start check and `pot why` on the host,
  `pot (work): tsc is not in this run, nor on the host's PATH.` and
  `  install it (for a repository's own tools, its package manager), then:` before the two lines.
- **An unknown first word that the host lacks too** (`pot tsc`, `pot work tsc`, `pot -- tsc`) gets the not-found
  form, then `  (tsc is not a context either: dispatch, host, inspect, work)`, in place of
  `pot: unknown context or command NAME; contexts: …`. Exit 2, as before.
- **Long startup and exit lines** wrap at the terminal's width (100 columns otherwise), with their continuations
  indented.

### Fixed
- `pot why PATH --json` for a path that does not exist (`nope/x.nix`, `./x`) says `"reached": false` and exits 1;
  v2.16.0 said `true` and exited 0. The text answer, `… no such path here …`, is unchanged.

## v2.16.0 - 2026-11-17

A one-way boundary between graft and pot, graft's sandbox (experimental): graft calls one function of pot's,
`graft.lib.pot.kitsFor`, and pot finds a repository's kits from git facts alone, never from graft's files. graft's
command is built from graft's sources only, so a change to pot no longer changes it. The formats the two share are
written down as versioned contracts ([pot's contracts](pot/contracts/README.md), and graft's
[provider/2](docs/reference/provider.md)). pot's core loses its knowledge of any one agent: an agent kit says, in
kit 1.1, how pot recognises it and its instances; a run names an instance with `CMD@NAME`; and the claude-code kit
takes an `api` value. Most repositories and roots need no edit: the old forms work, with a warning, until v2.17.0,
and each side reads the other's v2.15.0 form, so you can move them in either order, as long as providers keep
their kits in v2.15.0's form until v2.17.0 and a consumer's own kit uses no kit 1.1 field (`dirs`, `agent`) ([Move to v2.16.0](docs/guides/move-to-v2-16.md)).

### Upgrading
- **A repo kit with powerful requests asks for one new grant.** Taking v2.16.0 changes the flake's `flake.lock`,
  so the tree its grant is fixed to changes: the next run stops with exit 125 (or prompts, at a terminal) with
  `changed source` and the same requests. Run `pot grant NAME` once. No canonical request item and no digest
  changes (a new flake check, `pot-golden`, holds them). The source is the committed and staged tree, so the prompt
  follows the commit of the new `flake.lock`: commit first, then grant.
- **Providers: keep `packages`, and give no `version`, until v2.17.0.** Each consumer's own graft builds a provider's
  `graft.nix`, and graft v2.15.0 refuses `tools` and `version`, stopping every run in that consumer. A consumer may
  rename its own `perSystem.kit`'s `packages` to `tools` once it is on v2.16.0: its `graft-kits` gives `tools` as
  `packages` and drops `version` for a v2.15.0 root, and leaves out a kit that uses `agent` or `dirs`, with a build
  evaluation warning, `graft-kits: FILE uses kit 1.1 (dirs or agent): a v2.15.0 pot cannot read it, so graft-kits leaves it out (pot-kits has it)`. Otherwise its `kit.json` is byte-identical to v2.15.0's.
- **pot finds a repository's flake from `.gitmodules` at `HEAD` only:** the nearest `flake.nix` at or above the
  folder that holds the submodules (the deepest, when they differ). **A repository with no submodules has no repo
  kit from its flake,** and pot never evaluates it or copies it into the store. Move such a repository's
  `perSystem.kit` into the root's `repoKits."<origin>"`, or graft a scion. graft's marker README and the
  `mkFlake {` text no longer decide anything for pot.
- **graft's command left a run's standard tools** (now 24). A run in a graft consumer still has the repository's
  own `graft`, from its bundle (`POT_TOOLS`). For other repositories, add graft to the root's `tools`:
  `programs.pot.tools = [ inputs.graft.packages.${pkgs.stdenv.hostPlatform.system}.default ];`.
- **`pot-kits` is a reserved name** in `apps` and `packages`, beside `sandbox`, `pot` and `graft-kits`: a
  repository's own `outputs`, `perSystem` or a provider that defines it fails with `… that name is graft's; …`.
- **Kit errors from the build start `pot:`,** not `graft:`: `pot: <file>: <field> is not a kit field (…)`, and
  `pot: mkRoot: a context may not be named like a pot verb: …`.
- **Messages name a kit's file as its bundle reports it:** `the repo kit's requests.network (flake.nix (perSystem))`
  (was `(perSystem.kit)`); a scion command kit's fix names `(<its file>: a kit release)`; an unverified scion's
  command names only `pot host CMD`; the grant prompt says `every workspace that pins X` (was `grafts`).
- **`policy.json` values:** pot's own defaults are `{ "root": "default" }` (was `{ "root": "graft" }`). The bundle's
  protected paths are `{ "root": "bundle", "why": "the repository's pot-kits" }`; the scions-directory item is
  gone. `pot explain` labels pot's defaults `(pot)` (was `(graft)`) and the bundle's protected paths `(repo)`.
  A reader that matched on `"graft"` must match `"default"` ([run-env/1](pot/contracts/run-env-1.md)).
- **graft's flake has no `knowledge-base-kit` input:** it leaves consumers' locks at their next update. graft's
  own docs check is `nix flake check ./dev`.
- **The instance follows the caller's variables, most specific first.** v2.15.0 picked the claude-code instance
  named like the caller's `CLAUDE_CONFIG_DIR` folder. v2.16.0 picks the instance whose `agent.select.env` the
  caller matches with the most variables (claude-code: `CLAUDE_CONFIG_DIR = "~/<configDir>"`, and
  `CLAUDE_CODE_USE_BEDROCK = "1"` for Bedrock), so a Bedrock shell now picks the Bedrock instance. A tie goes to the
  default; otherwise pot stops: `pot: claude: the caller's variables match claude-code@a and claude-code@b equally:
  choose one with claude@NAME or --instance NAME`. `CMD@NAME` and `--instance` still decide first. Values other
  than paths compare as exact strings: `CLAUDE_CODE_USE_BEDROCK=true` does not match `"1"`.
- **`runs` and `doctor` are reserved:** `pot: runs comes in a later release` (and `doctor`; exit 2), and a root may not name a
  context so.
- **`defaultInstance` is checked when the root evaluates:** `graft.lib.mkRoot: defaultInstance.KIT = "X": the root
  declares KIT@a, KIT@b`, or `graft.lib.mkRoot: defaultInstance.KIT: the root has no kit KIT@NAME (kits: …)`; the
  home-manager module names its options (`programs.pot.defaultInstance.KIT = "X": programs.pot.kits declares …`).
- **`pot host --resume` does not resume runs from before v2.16.0:** they have no `resume.json`, which now records
  where the agent's transcripts are and how to read them. Read such a run's transcript in its state folder.
- **Messages say instance, not account:** the startup note `instance NAME`; the resume line
  `pot: resume this session with: pot [CONTEXT] claude@NAME --resume ID`; and the usage errors
  `pot: claude@NAME: this root declares claude-code@a, claude-code@b`,
  `pot: claude-code has the instances a, b: choose one with claude@NAME or --instance NAME`,
  `pot: --instance names an instance, and KIT has none`. `--resume` outside `pot host` names
  `(pot host --resume RUN CMD)`.
- **`pot explain` lists pot's agent-configuration protections in a new order:**
  `… .githooks **/.mcp.json **/.vscode **/.claude **/.pi **/.codex (pot)`.
- `nix flake init -t github:danielnaab/graft/v2.16.0` and the docs pin v2.16.0.

### Deprecated (removed in v2.17.0)
- **`packages.<system>.graft-kits`.** mkFlake still writes it in v2.15.0's exact shape, so a v2.15.0 root reads a
  v2.16.0 consumer; pot reads it when a flake has no `pot-kits`, so a v2.16.0 root reads a v2.15.0 consumer,
  protecting what v2.15.0 protected for it (`**/graft.nix` and its scions folder).
- **`--account NAME`** works as `--instance NAME`, after one line: `pot: --account is --instance; --account goes in
  v2.17.0`.
- **`defaultAccount`** (mkRoot and `programs.pot`) works as `defaultInstance.<kit>` for every kit with instances,
  with the evaluation warning `graft.lib.mkRoot: defaultAccount is defaultInstance.<kit>; defaultAccount goes in
  v2.17.0` (`programs.pot.defaultAccount is programs.pot.defaultInstance.<kit>; defaultAccount goes in v2.17.0`).
  Giving both is an error.
- **The claude-code kit's `account`** is not used, and warns: `graft.kits.claude-code: account is not used: the
  instance is the root's kits key; account goes in v2.17.0`.
- **The claude-code kit's `provider` and `region`** work as `api`, with the warning `graft.kits.claude-code: provider
  and region are api = graft.apis.bedrock { region = …; } (or graft.apis.anthropic); they go in v2.17.0`. Giving
  both is an error.
- **`kit.packages`** is read as `tools` in kit 1.0 and 1.1. A consumer's own `perSystem.kit` gets the evaluation
  warning `flake.nix (perSystem): packages is tools' old name; packages goes in v2.17.0`; a provider's `graft.nix` gets none, since providers keep `packages` until v2.17.0. Giving both is an error: `pot: FILE: tools and packages
  are one field (packages is its old name): give tools`.

The old and new forms give byte-identical canonical requests (the checks `pot-golden` and `pot-aliases`), so no
grant asks again.

### Added
- **`graft.lib.pot.kitsFor { pkgs; self; scions; tools; files; }`** (contract [pot-lib/1](pot/contracts/pot-lib-1.md)):
  pot's library, the one Nix edge between graft and pot. It validates each kit and writes
  **`packages.<system>.pot-kits`** (contract [pot-kits/1](pot/contracts/pot-kits-1.md)): `kit.json`
  (`{ version = 1; self; scions; protect; }`, each kit with its `file`), `bin/` and `closure`. mkFlake calls it and
  reads no kit field.
- **`pot why HOST|HOST:PORT|URL|PATH --json`** (contract [run-env/1](pot/contracts/run-env-1.md)), on the host and inside
  a run: `{ version, target, kind, reach, reached, context, text }`, exit 0 when reached and 1 when not. `kind` is
  `network`, `git.fetch` (an ssh URL, reached when a `git.fetch` binding covers it) or `file` (a `file://` URL or a
  path, reached when the run sees it). On the host it answers from the policy a run of the default context would
  have. pot classifies the target once, so its text and `--json` agree, and a git URL never gets a path's answer.
- **pot's contracts**, one page each, versioned: `kit/1`, `pot-lib/1`, `pot-kits/1`, `grants/1`, `run-env/1` (with
  the frozen `policy.json` v1) and `runs/1` (reserved), plus graft's `provider/2`.
- **A bundle check at start:** a `pot-kits` whose `version` or fields this pot does not know is refused:
  `pot (work): ~/DIR#pot-kits: kit.json is version 2, and this pot reads version 1: take a pot that reads it, or
  run pot host CMD`. A flake that does not evaluate: `pot (work): the flake at ~/DIR did not evaluate (nix eval,
  above): fix it, or run pot host CMD`.
- **A negative cache:** a flake with neither `pot-kits` nor `graft-kits` is remembered as "no repo kit" by its
  tree, so pot evaluates it again only when it changes. Start time is unchanged (cold about 2 s, warm 0.12 s, for
  flexion-assistant).
- **The flake check `pot-golden`:** the canonical grant items and digests of the claude-code instances, a standing
  kit and a repo kit, as v2.15.0 wrote them.
- **Instances:** `pot CMD@NAME` and `--instance NAME` choose an instance of an agent kit; mkRoot's and
  `programs.pot`'s `defaultInstance.<kit>` gives each kit's default. With none of them, the caller's variables
  choose ([kit/1: Instances](pot/contracts/kit-1.md#instances)).
- **Kit 1.1** ([kit/1](pot/contracts/kit-1.md)): `version` (`"1.0"` when missing; a newer minor gets one refusal,
  `pot: FILE: kit version 1.2 is newer than this pot reads (1.0, 1.1): take a newer pot`); `tools`; `agent` (a
  root's kit only: `markers`, `select.env`, `resume` and `hooks`); and the command field `dirs`, with the modes
  `links` and `mirror`, in place of 1.0's `fixedDirs`, `realDirs`, `skillDirs` and `memory`. A mirror shows a host
  folder read-only, with writable copies of the files it names, and copies what the run adds out to
  `<label>-out/` for review. The claude-code kit writes 1.1; 1.0 kits are read as before.
- **`graft.apis.anthropic` and `graft.apis.bedrock { region; }`,** for the claude-code kit's new `api` argument.
- **`agentMarkers`** (mkRoot and `programs.pot`, default `[ ]`): variables that mean an agent is the caller. They add to a floor that
  no root removes, `CODEX_SANDBOX` and `CLAUDECODE`, and to each root kit's `agent.markers` and `POT`, so `pot grant`
  refuses under an agent whatever kits the root has.
- **`policy.json`:** a file that a mirror puts in the run has `"mirror": "<label>"`, beside v2.15.0's
  `"memory": true`, which stays ([run-env/1](pot/contracts/run-env-1.md)).
- **The flake check `pot-aliases`:** the deprecated forms give the same canonical requests as the new.

- **`pot host CMD`** runs the workspace bundle's command when `CMD` is not on the host's `PATH`, so the
  unverified-scion refusal's `pot host CMD` works. That refusal still exits 1.
- **The graft-kits fallback is noted at start:** `~/DIR has graft-kits only: its graft predates v2.16.0 (pot reads
  graft-kits until v2.17.0)`.

### Changed
- **`graft check` inside a pot run asks `pot why URL --json`** whether the run reaches a provider's URL, instead of
  reading `POT_POLICY`. With no usable answer (an older pot), it tries the fetch. Its messages are unchanged.
- **Layout:** pot moved from `lib/pot/` to `pot/` (`pot/core/`, `pot/nix/`, `pot/lib/`, `pot/kits/`, `pot/tests/`,
  `pot/contracts/`). graft's command is built from `lib/*.sh`, `lib/*.jq`, the scions README and `template/` only.
  No flake output changed its name.
- **pot's core names no agent.** The agents' hidden and protected defaults (`~/.claude`, `**/.codex`, …) are root
  data in pot's Nix (`baseHidden`, `baseProtect`), a floor that a root's `hidden` adds to and cannot remove. Claude's
  markers, config-folder rule, transcripts, resume arguments and memory wording are the claude-code kit's data.
- **The base protected list loses `**/graft.nix` and the scions folder:** graft's mkFlake names them in its
  bundle's `protect` instead, so they stay read-only in every run of a graft consumer.

### Fixed
- A scion's protected folder is read-only throughout a run, not only its `graft.nix` (a scion's other files were
  writable).
- A scion whose URL ends in `.git` has the short name without it (`pot grant qk10`).
- `pot grant NAME` and `--revoke NAME` take the name pot displays, so the name a 125 line offers works for a
  repository with a local origin (its folder) and a scion named unlike its URL.
- An unknown first word adds where a missing program goes: `pot: unknown context or command NAME; contexts: …; a
  program every run here should have goes in programs.pot.tools (home-manager switch)` (an mkRoot root:
  `goes in tools (rebuild the root)`).

### Removed
- **v2.14.0's Seatbelt files** (`lib/sandbox/`, unwired since v2.15.0), `tests/macos-smoke.sh` and
  `tests/run.sh seatbelt`.
- pot's guesses at graft's layout: the marker README check and the `mkFlake {` regex.

## v2.15.0 - 2026-11-16

pot, graft's sandbox for agents and other commands (experimental), replaces v2.14.0's `sandbox` launcher. pot is
the sandbox of the grove tools (grove, graft and pot), and ships in graft's flake for now. A repository
or a provider declares what its work needs as a **kit**; only a person grants reach, with `pot grant`, into a
grants file in their own **root**. Linux only: on macOS, `pot` refuses until a later release, and v2.14.0 is the
last release with a Seatbelt sandbox. **This minor version breaks v2.14.0's sandbox:** read Upgrading.

### Upgrading
- **`perSystem.sandbox` becomes `perSystem.kit`, and a provider's `sandbox` in `graft.nix` becomes `kit`.**
  `paths.readOnly`, `paths.writable`, `network.allow` and `env.pass` move to `requests.read`, `requests.write`,
  `requests.network` and `requests.env`. Delete `grant` and `repository.readOnly`: grants live in your root
  (`pot grant`), and a read-only run is `pot inspect`. A leftover `sandbox` fails with
  `graft: scions/NAME/graft.nix gives \`sandbox\`: graft v2.15.0 replaced it with \`kit\` (pot; see graft's CHANGELOG)`.
- **`"host"` in `requests.network` means port 443 only** (v2.14.0: 443 and 80). Write `"host:80"` for plain HTTP.
- **`nix run .#sandbox -- CMD` is gone,** with `packages.<system>.sandbox`, `apps.<system>.sandbox` and the dev
  shell's `sandbox`. Run `pot CMD` from your root, or graft's default root
  (`nix run github:danielnaab/graft/v2.15.0#pot -- CMD`). The names `sandbox`, `pot` and `graft-kits` are reserved.
- **`--with`, `--allow`, `--explain` and `--dry-run`** are `--package`, `--net`, `pot explain` and
  `pot explain --backend-command`.
- **`GRAFT_SANDBOX=1` becomes `POT=<context>`,** such as `POT=work`. Test it with `[ -n "${POT:-}" ]`. The other
  variables are `POT_TOOLS`, `POT_POLICY` and `POT_QUIET`; paths inside a run are under `/run/pot`.
- **User paths:** `~/.config/pot/` (grants file, token files, launcher link) and `~/.local/state/pot` (state).
- **Names:** the command is `pot`; `packages.<system>.pot` and `apps.<system>.pot`; `homeManagerModules.pot` and
  `programs.pot`. The reserved output names are `sandbox`, `pot` and `graft-kits`. nixpkgs has an unrelated
  `pot` package; the two collide only on a `PATH` that has both.
- A repository whose `just check` runs `graft check` inside a run needs the `nix` capability in its kit, a
  `POT_TOOLS` branch in its `ensure.sh`, and a `graft_run` that uses `graft` from `PATH`
  ([Move from v2.14.0's sandbox](docs/guides/move-from-the-v2-14-sandbox.md)). **A `nix` request does nothing
  until your root binds it** (`nix."*" = { via = "daemon-untrusted"; };`) and you run `pot grant`.
- **A powerful repo-kit grant is fixed to the flake directory as committed and staged.** Each commit or staged
  change under it changes the source: the next run stops with exit 125, lists the changed paths, and `pot grant`
  asks again for each powerful item.
- **The home-manager module asserts `workmux.sandbox.enable = false`** while `programs.pot.enable` is on: `pot`
  replaces workmux's Docker sandbox for agent panes. Turn workmux's off, or evaluation fails.
- **A repository used on both macOS and Linux loses its Mac sandbox in v2.15.0:** `pot` refuses on Darwin. To keep
  the Seatbelt sandbox on a Mac, keep that repository on v2.14.0; the choice is per repository, because each
  pins graft in its own `flake.lock`.
- `nix flake init -t github:danielnaab/graft/v2.15.0` and the docs pin v2.15.0.
- The weekly upgrade job takes this version like any minor version. If your repository declares a `sandbox`,
  move it before you merge that pull request.

### Added
- **`pot [VERB | CONTEXT] [FLAGS] [--] [CMD [ARGS...]]`:** runs a command as a run, under a policy composed at
  each start from the run's kits (the command's, the repository's, its scions', and the root kits its context
  includes) and the person's grants. Contexts: `work` (the default: write the repository in place), `inspect`
  (read the machine except secrets, change nothing), `host` (no sandbox, said out loud). bubblewrap, on Linux
  ([pot](docs/reference/pot.md), [Run commands in pot](docs/guides/run-in-a-sandbox.md)).
- **Kits:** `kit = { packages; env.set; files; protect; requests; commands; recipes; }` in `graft.nix` and
  mkFlake's `perSystem`, with `perSystem.kitArgs`. mkFlake exports them, with the workspace's tools and `graft`,
  as `packages.<system>.graft-kits`.
- **Requests and capabilities:** `read`, `write`, `socket`, `network`, `env`, `state`, `unsandboxed`, and the
  capabilities `status.relay`, `git.fetch`, `credential`, `mcp` and `nix` (bound in this version), with the
  later ones reserved. Each is ordinary or powerful.
- **Grants:** `pot grant [KIT...] [--revoke KIT] [--diff]` records a kit's request set by digest (and, for
  powerful items, its source) in one tracked grants file with a section per machine. A kit that asks for more
  stops with exit 125 until it is granted again. At a terminal, every stale kit (not granted, asks for more,
  changed source) is prompted inline, and the run goes on. `pot grant` refuses agents and reads from `/dev/tty`;
  ordinary items are approved together, each powerful item alone. Kits go by short names (`flexion-assistant`,
  `claude-code` for every instance). A changed grants file is noted at start; an entry whose digest does not match
  stops runs and names its repair (`pot grant --revoke KIT`, then `pot grant KIT`).
- **`pot explain`** shows every item with the kit, file or grant that it came from; `pot why PATH|HOST|CMD|VAR`
  answers for one, inside a run too.
- **Roots:** `graft.lib.mkRoot`, graft's default root (`packages.<system>.pot`, `apps.<system>.pot`; grants in
  `~/.config/pot/grants.json`), `pot init DIR`, and a home-manager module (`programs.pot`). The default root and
  `pot init`'s root ship a `claude-code@personal` instance, its token binding
  (`~/.config/pot/claude-token-personal`, from `claude setup-token`) and the `nix` relay, so `pot claude`
  works without writing a root.
- **The claude-code kit:** `graft.kits.claude-code { pkgs; account; … }`: accounts, skills and plugins read-only
  (opt out with `skillDirs = [ ]` and `plugins = false`), the host's project memory read-only, with `MEMORY.md` as a
  writable copy (new memory files and a changed `MEMORY.md` are listed on the exit line and copied out of the run;
  nothing is written back; a `SessionStart` note tells Claude),
  MCP config, Bedrock (its credentials named on the startup line when unset or expired), a `claude setup-token`
  login file, and its own agent home per account, workspace and context.
- **Denials** reach the person through `tmux display-message` on the launching pane, the agent through
  `/run/pot/denials` and `PostToolUse` and `PostToolUseFailure` hooks, and both through the exit line. Every denial starts
  `pot (<context>):`, names its source, and gives a one-run flag and one permanent fix.
- **Bindings** carry out `status.relay` (a `workmux` shim aimed at the launching pane), `git.fetch` (an ssh shim and
  a host broker that allows only `git-upload-pack`), `nix` (a relay to an untrusted daemon), and `credential` and
  `mcp` (by env or file, read on the host at start). A file a binding names is hidden.
- **Not-found handling:** bash's handler (also exported as a function), and shims for host-only programs (`nix`, `ssh`, `sudo`, `systemctl`,
  `docker`, `home-manager`, …), each with a one-run flag and a permanent fix. Exit 127.
- **`pot host --resume RUN|last CMD`** passes a run's transcript to a host command as quoted, untrusted data. The
  exit line names the run's id.
- **`nix` inside a run** with the `nix` capability: `pot work nix flake check` runs the host's nix through the
  relay.
- **`graft check` inside an `pot` run** tries only the provider URLs that the run reaches, and prints one note
  instead of leaving a denial.
- **Root repositories:** `work` refuses in a root's own repository, and names `pot inspect` or plain `claude`.
- **The decision** behind it: [Roots grant reach; kits only ask](docs/decisions.md#roots-grant-reach-kits-only-ask).

### Not yet
- Synced claude.ai skills do not load inside a run: Claude loads them only when signed in to claude.ai, and a run
  signs in with a token. Install the skills you need as ordinary skill folders (`skillSources` or `skillDirs`).
- Kit tools still run on the host: kit commands are not wrapped or sandboxed on their own.
- `graft add` and `graft upgrade` do not print each kit's request diff yet.
- `pot review`, and `pot` on Darwin.
- In `work`, `/run/current-system` is a link whose target the run does not see: a test that hard-codes
  `/run/current-system/sw/bin` fails there.
- No `dispatch` (clone mode), `pot review`, nesting, generated `kits.just`, or Seatbelt backend.

### Removed
- v2.14.0's `sandbox` launcher and its options, and the Seatbelt backend.

## v2.14.0 - 2026-11-15

A sandbox for agents and other commands. Every repository gets a launcher, `packages.<system>.sandbox`,
that runs one command so that it sees its closure, the repository, and only what the repository declares.
Nothing to change: a repository that declares no `sandbox` gets graft's base policy.

### Added
- **The `sandbox` launcher:** `nix run .#sandbox -- CMD` (`nix run 'path:DIR#sandbox' -- CMD` for a flake in
  a subdirectory; `sandbox CMD` in the default dev shell). Linux, with bubblewrap and unprivileged user
  namespaces; macOS, with Seatbelt (`sandbox-exec`). The command sees its closure read-only, the repository writable, an empty home folder, an
  empty environment but for what the policy names, and no network.
  `sandbox [--with PKG]... [--allow HOST[:PORT]]... [--explain [--json]] [--dry-run] [--] [CMD [ARGS...]]`;
  with no command, `bash`. Exit status: the command's; `127` for a command not on the sandbox's `PATH`; `2`
  for a usage error ([Sandbox](docs/reference/sandbox.md),
  [Run an agent or a command in a sandbox](docs/guides/run-in-a-sandbox.md)).
- **`sandbox` in `graft.nix` and `perSystem`:** `packages`, `env.set`, `protect`, `repository.readOnly`,
  `paths.readOnly`, `paths.writable`, `network.allow`, `env.pass` and `grant`, merged by Nix. A provider may
  add to what the run can see (`packages`, `env.set`, `protect`), never to what it can reach: its
  `network.allow`, `paths.*` and `env.pass` are requests, which apply only when the consumer names the scion
  in `sandbox.grant`.
- **Protected paths:** `flake.nix`, `flake.lock`, `graft.nix`, `.gitmodules`, `.envrc`, `justfile` and
  `.pre-commit-config.yaml` at any depth, `protect`'s paths, and the git metadata that host git reads to find
  a repository or to run something (`.git` files, config, hooks) are read-only inside, so a command cannot
  widen its next run or plant a hook. A symbolic link, or a file with another hard link, on a protected path stops the launcher. The launcher
  shows control characters in what it prints as `\xNN`. After each run
  the launcher warns about new git metadata, new gitlinks and new files with a protected name.
- **`network.allow`:** `"host"` (ports 443 and 80) or `"host:port"`, with `*` and `?`, through a proxy on the
  host that refuses the rest. Only programs that use `HTTPS_PROXY` reach the network. After the run, the
  launcher lists the connections that the proxy refused.
- **`sandbox --explain`** prints the policy with the file that set each value, and the requests that are not
  granted. `--dry-run` prints the bubblewrap command. `--with PKG` and `--allow HOST` add a package or a host
  for one run.
- **On macOS**, the same policy as a Seatbelt profile that denies by default: the closure's store paths and
  the macOS files a process needs, read-only; the repository writable but for the protected paths; `HOME`
  and `TMPDIR` new temporary folders; the network only through the proxy, on a `127.0.0.1` port
  ([On macOS](docs/reference/sandbox.md#on-macos)). A new hard link to a protected file is denied. One
  limit: Seatbelt does not check `KERN_PROCARGS2`, so a run can read the environment of the user's other
  processes. `tests/macos-smoke.sh` checks it on a Mac.
- **The decision** behind it, with its costs: the sandbox protects the host, not the repository's history
  ([Decisions](docs/decisions.md#enforce-the-boundary-in-a-sandbox-not-in-rules)). graft's scope is now Nix,
  git submodules, a tag comparison, and the sandbox policy their operations run under.

### Upgrading
- Nothing required. `apps.<system>.sandbox` and `packages.<system>.sandbox` are now graft's names, like
  `apps.graft` and `packages.tools`: a provider or a `perSystem` that defines either fails evaluation with
  `graft: packages sandbox (from NAME): that name is graft's; …`, and mkFlake's `outputs` with
  `graft: mkFlake's outputs has packages.<system>.sandbox: that name is graft's; rename the repo's own`.
  Rename yours.

## v2.13.1 - 2026-11-14

### Fixed
- **graft v2.13.0 can't be a flake input.** Its own `flake.nix` declared `inputs.self.submodules = true;`, and
  Nix applies that to the fetch of a flake taken as an input: every consumer with
  `inputs.graft.url = "github:danielnaab/graft/v2.13.0"` failed to lock or evaluate with
  `error: input attribute 'submodules' not supported by scheme 'github'` (`'path'` for a `path:` checkout).
  graft's flake no longer declares it, and no longer calls `mkFlake` on itself; its docs check reads
  knowledge-base-kit from a `flake = false` input at its scion's commit. `nix run github:…/v2.13.0 -- …`
  (graft's command, not an input) was not affected.
- The docs said `github:` inputs were not affected by a flake's `inputs.self.submodules`: they are. A flake
  that others take as an input must not declare it ([Nix behaviours](docs/reference/nix-behaviours.md)).

### Upgrading
- On v2.13.0: set `inputs.graft.url` to `github:danielnaab/graft/v2.13.1`, run `nix flake update graft`, and
  commit. `nix run .#graft upgrade` can't do it, because v2.13.0 doesn't evaluate as an input. Nothing else
  changes. Moving from v2.12 or earlier: follow v2.13.0's Upgrading steps below with v2.13.1 for v2.13.0.

## v2.13.0 - 2026-11-14

graft now pins providers with git submodules. A scion is a submodule at a provider's release tag; its gitlink
is the pin. A provider's `graft.nix` wires its tools, apps and checks into your flake, so your `flake.nix` names
no provider. Copy mode is gone: every consumer moves once, by hand or by an agent following the Upgrading
steps. Needs Nix 2.27 or later.

### Upgrading

These are also a guide: [Move from copy mode](docs/guides/move-from-copy-mode.md).

1. On a branch, note each scion's tag: `nix run .#graft outdated` (before you upgrade graft).
2. `git rm -r -q scions` (your scions directory), so the copied files leave the index and the working tree.
   Never only `rm -rf` it: `git add` over a path the index still holds as files records the new clone as
   files, not as a submodule (`graft add` refuses to, and `graft check` fails on it:
   `… is tracked as files, not as a submodule`). For a flake in a subdirectory, the scions directory moves
   under the flake's directory (`nix/agent-tools/.scions`): delete the old one at the top.
3. In `flake.nix`: delete every `flake = false` provider input and its wiring in `outputs`
   (`import "${inputs.X}/…"`); delete `mkFlake`'s `scions` and `flakeDir` (in a subdirectory flake, set
   `dir = ".scions";`); set `graft.url` to `github:danielnaab/graft/v2.13.0`; for a flake at the repository's
   top, add `inputs.self.submodules = true;`. Tools of your own that were in `tools` go in `mkFlake`'s new
   `perSystem`. Consumer settings that were Nix arguments move to the files that each provider's CHANGELOG
   names.
4. For each provider: `nix run github:danielnaab/graft/v2.13.0 -- add -C . URL@TAG`, at a release that ships a
   `graft.nix` if you used its Nix code. In a subdirectory flake, add `--scions nix/agent-tools/.scions` to the
   first.
5. `nix flake lock`, then `nix run .#graft sync` (it writes the marker README and graft's v2.13.0 workflows).
   Commit, then run `nix flake check` and `nix run .#graft check`.
6. Fix paths that named the old scions directory: links, `just` imports (use `import?`), hooks, source imports,
   lint excludes. Replace the "never edit scions" agent rule with the section in
   [Graft your first pattern](docs/guides/graft-a-repository.md#steps), step 6, and make an agent tool's rule
   that denies edits to the scions directory ask instead (Claude Code: move `Edit(scions/**)` from
   `permissions.deny` to `permissions.ask`): a scion edit is now a provider fix that a person approves.
7. Commit and push.

What a weekly upgrade job still on v2.12 does: it moves graft's pin to v2.13.0, then runs v2.13.0's copy step,
which is now a stub. The stub prints
`graft: graft v2.13.0 replaced copied scions with git submodules; this upgrade can't be automatic: follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md (https://github.com/danielnaab/graft/blob/v2/CHANGELOG.md)`
and exits 1, so the job fails before it commits: no branch, no pull request, the message in the job's log.
Run by hand (`nix run .#graft upgrade` on v2.12), it leaves `flake.nix` and `flake.lock` moved to v2.13.0 and
the scions untouched; every command then says the scions README is copy mode. Follow the steps above.

### Added
- **`graft add [-C REPO] [--scions PATH] URL[@TAG] [NAME]`:** a new scion, as a submodule at the newest
  release (or `TAG`).
- **`graft try [-C REPO] [NAME...]` and `graft try --stop`:** build your repository with a scion's
  uncommitted edits, for a provider fix before its release. `try` marks `.graft-try` with intent to add (never
  a commit, never a scion's index), so a git flake reads the working tree, and prints what each build will
  see, the `git -C … add` that makes a scion's new files visible, and the command to run; `--stop` removes
  the marker. `graft check` (and
  `ci check`) fails while it is on, and `upgrade` refuses. A `path:` flake in a subdirectory needs nothing:
  `try` only reports. Replaces the scratch-file steps of the guides.
- **`graft.nix`:** a provider's tools (`packages.<system>.tools`, one environment for every provider), apps,
  packages and checks (`checks.<system>.<scion>-<name>`), with `requires` for providers it needs. Arguments
  `pkgs`, `lib`, `system`, `scion`, `scions`, `consumer` ([Files](docs/reference/files.md#graftnix)).
- **mkFlake's `perSystem`:** a consumer's own tools, apps, packages, checks and dev shells, merged like a
  provider's.
- **`graft check` rules:** every scion registered as a gitlink (not as files), filled (with its own
  submodules), clean, checked out at its gitlink, at a version tag's commit (a tag that exists only in the
  scion's clone says "push it"), and its `graft.nix` parses; absolute URLs; `inputs.self.submodules` in a
  top-level flake (opt out with the comment `# graft: no self.submodules`); from `nix run .#graft`, mkFlake's
  `dir` is the scions directory, and the tools and checks evaluate. `check --committed` checks only what is
  committed.
- **A provider that fails is never dropped:** its error fails `packages.<system>.tools`, the checks,
  `nix flake check` and `graft check`, under `… while evaluating scions/NAME/graft.nix`; `nix run .#graft` and
  your own outputs still evaluate. A `graft.nix` that does not parse breaks every output: run
  `nix run github:danielnaab/graft/v2.13.0 -- check -C .` to name it.
- **mkFlake names a wrong `dir`:** `graft: graft's README is in scions/ but mkFlake's dir is "DIR": set dir = "scions"; …`.
- **`add`** notes each sibling the new scion's `graft.nix` requires that you do not graft, with its
  `graft add` line.
- Guides: [Clones, worktrees, sandboxes and CI](docs/guides/clones-and-worktrees.md) (empty scions, the
  post-checkout hook, git settings), [Fix a provider from a consumer](docs/guides/fix-a-provider.md),
  [Write a provider's graft.nix](docs/guides/write-graft-nix.md), and
  [Move from copy mode](docs/guides/move-from-copy-mode.md).

### Changed
- Every fill graft does or advises is recursive: `git submodule update --init --recursive`.
- **`sync`** fills empty scions and their own submodules, writes the marker README and refreshes graft's
  workflows. It never moves a pin or a filled scion. `add` and `upgrade` fill a scion's own submodules too.
- **`upgrade`** moves gitlinks (with no clone, in CI) and graft's own pin; it never changes `.gitmodules`.
  It moves an untagged scion (a fix recorded before its release) when named, `NAME@TAG`, or `NAME` for the
  newest release in the major of its nearest tag; it refuses only a scion with uncommitted edits (a fix
  branch in the scion stays).
  **`ci upgrade`** commits, then runs `nix flake check` on the clean committed tree, so Nix builds the new
  scions.
- **`ci check`** runs `graft check --committed`, then `nix flake check --no-build`: the whole flake must
  evaluate.
- **`ci pr`** opens a pull request for gitlink moves, and for graft's own move with the marker README; it
  refuses a `.gitmodules` change, and (as before) a change to graft's workflows.
- **`outdated`, `changelog`, `migrate`** derive each scion's version from its gitlink. `migrate` lists what
  moved from the provider's history between the two commits.
- **The scions README** is a fixed marker and explanation, with the right run command for the flake's kind; no
  table, no `<!-- graft flake: -->` line. `check` reads only its marker line.
- **The `# graft ssh:` line** may be in either workflow, and installs the key before every step that runs
  graft: the check, the upgrade and the pr step. `paths-ignore` may not match `.gitmodules`.
- [Concepts](docs/concepts.md), [Decisions](docs/decisions.md) (Pin with git submodules; let providers wire
  themselves), and every guide and reference page are rewritten for submodules.
  [Ship a tool or seed files](docs/guides/ship-tools.md) is now
  [Write a provider's graft.nix](docs/guides/write-graft-nix.md);
  [Change a grafted file](docs/guides/change-a-grafted-file.md) is now
  [Fix a provider from a consumer](docs/guides/fix-a-provider.md).

### Removed
- Copy mode: committed copies of exports, the drift check and report, `checks.<system>.graft`, export rules
  (`export-ignore` is no longer read), the annotated-tag re-pin, mkFlake's `scions` and `flakeDir`, and the
  scions README's table.

### Fixed
- **On Nix 2.34, `graft ci upgrade`'s commit and pull request no longer show Nix's `For full logs, run:`,
  `nix log /nix/store/…` and `Output paths:` lines** of a failing check. Nix 2.34 words a failed build's
  error differently from 2.24; graft now drops both forms.

## v2.12.0 - 2026-11-13

Every guide is now a task page read against a fixture: drift, first graft, gating merges, changing a grafted
file, private files and providers, CI, upgrades, incompatible changes and publishing. graft's checks now catch
an uncommitted or stale `flake.lock`, a repository's own `checks.graft`, and bad arguments; the drift report
names files as your repository does; and `ci upgrade` refuses a dirty tree. Read **Upgrading** first: the
weekly upgrade onto this version opens no pull request, because graft's workflows change.

### Upgrading
- **A `.gitignore` that drops `flake.lock` now fails `graft check` and `nix flake check`, and
  `graft ci check` fails when `flake.lock` is not in `HEAD`.** Remove the rule (`git check-ignore -v flake.lock` names it), and commit
  `flake.lock`.
- **A repository's own `checks.<system>.graft` or `apps.<system>.graft`, in `mkFlake`'s `outputs`, is now an
  evaluation error** of graft's app and check. Rename the repository's own.
- **`outdated NAME`, `upgrade --json` and an unknown option now exit 2.** Remove them from scripts that
  call graft.
- **graft's workflow files change, so the first weekly upgrade onto this release opens no pull request.**
  It moves graft's pin and refreshes the workflows, and the **pr** job refuses with
  `this upgrade changes CI workflows: .forgejo/workflows/graft-check.yaml .forgejo/workflows/graft-upgrade.yaml. … so none is opened: …`.
  Read the workflow diff at the link in its log, then open the pull request from `graft/upgrade` yourself.
- **The new workflows fail CI where the old ones guessed.** With scions READMEs that name different flake
  directories, or one that graft won't use, the old workflows took the first line they found; the new ones
  fail with `::error::the scions READMEs name different flake directories: …` or
  `::error::the scions README names a flake directory graft won't use: …`. `git grep -n 'graft flake:'` lists
  the lines: each must name the flake's directory, so delete a scions directory that your flake no longer
  writes, and commit.
- **The drift report is new text.** A script that read `diff -r`'s output from `graft check` or
  `nix flake check` (`Only in …`) reads the `graft: scions/NAME/FILE: …` lines instead.
- **A workflow of your own that runs `graft ci upgrade` must check out `BRANCH` first**, for example
  `git switch -c "$BRANCH"`: `ci upgrade` now refuses any other branch. graft's own `graft-upgrade.yaml`
  already does.
- **`graft ci check` fails when `flake.lock` does not cover `flake.nix`** (an input added without
  re-locking). Run `nix run .#graft`, and commit `flake.lock`.

### Added
- **[`graft check` fails](docs/guides/check-fails.md):** a guide to finding what changed in a scion
  (an edit, a formatter, a tool's `__pycache__/`), saving a change someone meant, restoring the pins, and
  keeping tools out of the scions directory. It replaces the list of causes in Graft a repository.
- **[Change a grafted file](docs/guides/change-a-grafted-file.md):** a guide to changing what a scion
  gives you without editing it: keep your own version of a file, change a setting that a provider's tool
  reads, fix it in the provider and take the release, or stop grafting a provider (`mkFlake`'s `scions`).
- **[Make a pattern's checks gate your merges](docs/guides/gate-merges.md):** a guide to running a
  provider's checker from its fetched export in `nix flake check`, a workflow that runs it on each pull
  request, and the Forgejo branch rule that requires it. graft's `graft-check.yaml` stays graft's drift
  check: your checks are a status of their own. It says why graft's weekly upgrade pull request gets no
  status from your workflow until a person reopens it.
- **[Ship a tool or seed files with a pattern](docs/guides/ship-tools.md):** a tool that consumers build
  from the fetched export with the provider's `package.nix`, seed files as a flake template, and a recipe
  for a step after each upgrade.
- **[Ship and take an incompatible change](docs/guides/incompatible-change.md):** a guide in two parts. The
  provider writes Upgrading steps, tries them on a throwaway worktree of a real consumer (`nix flake lock
  --override-input`, then `graft migrate`), and tags the new major version. The consumer takes it on a branch
  with `upgrade NAME@TAG`, works from `graft migrate`'s brief, and opens a pull request. It says which base
  `migrate` compares with, what to do when a provider renames its repository, and which Upgrading
  headings graft finds. It replaces "Cross a major version" in Upgrade and "Make an incompatible change"
  in Publish, which now link to it.
- **[Nix behaviours](docs/reference/nix-behaviours.md)**: trailing-slash rules fail only for fetches from
  a bare clone, not through `git+file` to a checkout; and Nix 2.24 reuses a consumer's `github:` fetch of a
  commit for `.` at that commit, so a repository whose export rules exclude its own `flake.nix` fails there.
- **`tests/fixtures.sh`** builds the repositories that a guide is read against ([Development](docs/development.md)).
- **`nix flake init -t` says what to run for a flake in a subdirectory**: `nix run 'path:DIR#graft'` from
  the repository's top, since `nix run .#graft` there copies the whole repository into the Nix store.

### Changed
- **Guides renamed, each now one goal with numbered steps, a check, and its real failures.** The old URLs
  stay.
  - Graft a repository is now [Graft your first pattern](docs/guides/graft-a-repository.md), a tutorial
    for a new or an existing repository (an existing flake's outputs, its own `flake = false` inputs, a
    `.gitignore` that drops `flake.lock`). Its section "Try a provider's unreleased change" moved to
    [Take a newer version by hand](docs/guides/upgrade.md#try-a-providers-unreleased-change).
  - Upgrade is now [Take a newer version by hand](docs/guides/upgrade.md): `outdated`, `changelog`,
    `upgrade`, the scions diff, the check, and which files to commit.
  - Publish is now [Publish a pattern](docs/guides/publish.md), the steps of one release: export rules,
    the CHANGELOG section, the export listed with `git archive` and checked for trailing-slash rules, the
    tag (prefer lightweight; signed or annotated works, with one re-pin for `git+` consumers), the push, and
    the consumer's upgrade.
  - CI is now [Run graft in CI and review its upgrade pull requests](docs/guides/ci.md), a guide to the
    weekly upgrade pull request: what its title and description say, the newer major versions it holds, what
    to read in the scions diff, and what to do when its checks fail or the pull request is not opened. It
    shows how to run the steps by hand (on GitHub, say) and how to check many repositories with `outdated -C`.
  - A flake in a subdirectory is now [Keep private files out of the Nix store](docs/guides/flake-in-a-subdirectory.md):
    numbered steps from a flake at the top or from no flake, a check that proves the store copy is only the
    subdirectory, and what to do about copies already in the store. It says that a repository's own check
    named `graft` is dropped from `checks` in a subdirectory flake, and that graft's app then fails to
    evaluate.
  - Private providers is now [Graft a private repository](docs/guides/private-providers.md), with the step
    that Nix 2.24 needs to make the first lock of a `git+ssh` input at a tag (`nix flake prefetch` at
    `refs/tags/TAG`): without it, the old page's pin failed with `revspec 'v1.0.0' not found`. Its section
    "Check in CI" is now its steps 4 to 10. It makes the CI key outside the repository, and says why
    graft-upgrade cannot run with a private scion, by design, and how to upgrade instead.
- **Moved to [Files](docs/reference/files.md)**: what to export and how to name a provider (from Publish),
  graft's workflows' `paths-ignore` line, and how to keep your own edits to a workflow (from CI).
- **One drift report for `graft check` and `nix flake check`** (lib/drift.sh). It names each file as the
  repository does (`scions/NAME/FILE`), not by a temporary directory or a store path: `edited`, with a
  unified diff; `only in your copy, not in the pin`; or `only in the pin, not in your copy`. A symbolic link
  that points elsewhere, or that became a file, is `edited`, with both targets. File modes are not compared.
  Then it says what each kind usually is: a tool's output or someone's addition; or a deleted file, a file
  git ignores (with the `git check-ignore -v` command that names the rule) or, for `nix flake check`, one not
  added to git. It replaces `diff -r`'s output and the "a file listed only on the left" hints.

### Fixed
- **The checks fail when `flake.lock` is not committed.** A `.gitignore` that dropped `flake.lock` let
  every check pass while Nix resolved the tags again on every run. `graft check` now fails when git
  ignores it: `graft: git ignores flake.lock, so the pins are not committed: git check-ignore -v flake.lock names the rule; …`
  (`FLAKEDIR/flake.lock` for a flake in a subdirectory). `checks.<system>.graft` fails when git does not
  track it: `graft: git does not track flake.lock, so the pins are not committed: …`. A lock that is only
  uncommitted passes both, because Nix git-adds a lock that it writes; `graft ci check`, which CI runs on a
  fresh checkout, fails unless the lock is in `HEAD`:
  `graft: flake.lock is not committed, so nothing is pinned: run nix run .#graft, then commit flake.lock …`.
- **A repository's own `checks.<system>.graft` or `apps.<system>.graft` is an error.** Passed in
  `outputs`, it was merged under graft's and never ran, without a word (in a subdirectory flake, a
  `checks.<system>.graft` was dropped). graft's app and check now fail to evaluate with
  `graft: mkFlake's outputs has checks.SYSTEM.graft: that name is graft's; rename the repo's own`.
  The repository's other outputs still evaluate.
- **`outdated`, `changelog` and `upgrade` refuse arguments they do not take**, with exit 2 and the
  command's usage line. `-C` with no repository failed with `unbound variable`; `outdated NAME` and
  `upgrade --json` were accepted and ignored; an unknown option was read as a scion's name.
- **graft's workflows find the flake by the commands' rules.** Their step read the first
  `<!-- graft flake: … -->` line in any `README.md`, so a provider's leaked scions README (one that names
  its own flake) could send a top-level repository's CI to a flake that does not exist. The step now reads
  only READMEs with graft's marker, skips one inside another scions directory, and fails on two that
  differ or on a path graft won't use, as `lib/common.sh` does. The new workflows arrive when the graft pin
  moves to this version: a sync restores the pinned version's copy.
- **A missing or mistyped `flakeDir` is named.** graft said only `graft: no flake.lock in TOP`; it now says
  `… set mkFlake's flakeDir to it` or `… check mkFlake's flakeDir, "DIR" …`.
- **`graft ci upgrade` by hand no longer commits and pushes files of yours.** It runs `git add -A`, so in a
  working copy it pushed untracked files (an `.env.local`, say) to `origin`, and on `main` it committed the
  upgrade there. It now refuses a working tree with changes of its own
  (`graft: ci upgrade commits every change in the working tree, and this one has the changes of its own above …`), after listing them, and
  a branch other than `BRANCH` (`graft: ci upgrade commits on the branch checked out, main, …`). Without
  `BRANCH`, it says `graft: BRANCH is not set: …`, not bash's `BRANCH: BRANCH is not set`. `graft ci check`
  ends with `graft: ci check passed: …`. A `flake.lock` that Nix re-locked while evaluating graft (an input
  added to `flake.nix` without re-locking) is still taken into the upgrade commit, with a line that says so.
- **`graft ci check` fails on a lock that does not cover `flake.nix`**:
  `graft: flake.lock in HEAD does not match flake.nix: Nix re-locked it to evaluate graft …`. It passed on
  the lock Nix wrote in the runner, which nothing commits.
- **The upgrade pull request's notes read like `graft changelog`'s.** `graft ci pr` had its own copy of the
  notes: it now prints each provider's **Upgrading** sections first, with control characters made plain, as
  `changelog` and `migrate` do (lib/upstream.jq). In all three, the second part is now labelled
  `The rest of its CHANGELOG.md for this range, without the Upgrading sections above:`, so a version's
  heading shown twice reads as one version split in two, and a provider that cannot be fetched gets its
  compare link.
- **The failing pull request's warning says to fix your own files first**:
  `Fix your own files on a branch made from this one: the next run force-pushes graft/upgrade. If the provider broke something, tell its maintainers.`
- **The logs of failing checks leave out Nix's noise.** In the pull request's **Failing checks** and in
  `migrate`'s brief, the dirty-tree warning, the lines about what will be built, the builder's log repeated,
  `For full logs, run 'nix log …'` and the checkout's path are gone, and store paths read `/nix/store/…-NAME`.
- **`migrate` reads a provider's branch locked with `--override-input`.** The brief said
  `page-rules v1.1.0 (from v1.1.0)` and listed commits. It now names the locked ref and commit
  (`page-rules lifecycle at COMMIT (from v1.1.0)`) and quotes the branch's CHANGELOG sections above the old
  tag, Upgrading first: the provider's unreleased major. Its header names the repository from its origin,
  else its main checkout's folder, not a worktree's (`handbook`, not `handbook-try`, and without `.git` for
  a bare clone's worktree), and says `flake.nix, flake.lock and scions/` when a changed line of `flake.nix`
  is an input's url.
- **`migrate` names a git input that has no ref in its URL by its commits.** It said
  `NAME refs/heads/main (from refs/heads/main)`, the ref Nix records for such an input; it now shows the
  old and new commits, and `REF at COMMIT` for a ref that is not a version tag.
- **Upstream notes past 50 commits say so.** They ended `... and 1 more`, a wrong count; they now end
  `... and more; see the compare link` (or the provider's git log when there is no compare link), in
  `changelog`, `migrate` and the upgrade pull request.
- **The scions README and graft's messages quote their commands for zsh.** For a flake in a subdirectory
  they said `nix run path:DIR#graft`, which zsh with `extendedglob` reads as a glob; they now say
  `nix run 'path:DIR#graft'` (and `migrate`'s brief `nix flake check 'path:DIR'`). `graft upgrade` rewrites the README when it takes this version.
- **A `### Migration` subsection counts as Upgrading steps**, as knowledge-base-kit v2.0.0 heads them:
  `changelog`, `migrate` and the upgrade pull request show it first.
- **A workflow line graft cannot read names every line it keeps.** The hint after it listed one, two or
  three of them, depending on the line; it now always says
  `graft: (paths-ignore, branches and the ssh line are the only edits graft keeps in FILE; …)`.

### graft's own checks
- **graft's own docs check fails on a broken link.** `checks.<system>.docs` ran
  `link_validator && kb_linter`, so a link failure never failed the build. Only graft's own
  `nix flake check` changes; nothing consumers run does.
- **graft's own shellcheck check also runs `bash -n`** over `lib/*.sh` and `tests/*.sh`, so a script bash
  cannot parse fails it even where shellcheck's parser differs. Only graft's own `nix flake check` changes.

## v2.11.1 - 2026-11-12

Docs only: nothing graft runs changes.

### Changed
- **The README says what graft is for.** It opens with the problem (a copied set of rules stops improving)
  and defines pattern, provider and scion. It then follows one knowledge-base example: while pinned, the
  pattern's checks govern each change, and taking a new version is a reviewed pull request, weekly for
  minor versions, by a person for a major one. It says what graft does and does not do, and compares it
  with copying, submodules, templates and dependency bots.

## v2.11.0 - 2026-11-12

Docs, graft grafts its own scions, and two fixes to what graft reads.

### Added
- **graft-check.yaml keeps your `branches:` line.** A repository whose default branch isn't `main` writes
  its name under `push:` (`branches: [v2]`), and sync, the check and `graft upgrade` keep it, as they
  keep `paths-ignore`. Before, the only way was to delete the marker line and give up graft's copy.

### Fixed
- **A provider's leaked scions README is no longer read as the consumer's.** A provider that forgets to
  export-ignore its scions directory puts `scions/<name>/scions/README.md`, with graft's marker, in the
  consumer. `outdated`, `upgrade`, `changelog` and `migrate` took it for one of the consumer's scions
  directories. They now skip a scions directory inside another.

### Changed
- **The README is now a set of short pages.** `README.md` says what graft is, explains the scion metaphor
  and gives a quick start. `docs/` has [concepts](docs/concepts.md), six guides, three reference pages
  (commands, files, Nix behaviours), [decisions](docs/decisions.md) and [development](docs/development.md).
  Each section of the v2.10.0 README is on one of these pages, and [CI security](docs/ci-security.md)
  explains the upgrade workflow's two jobs. The v1 table's `commands:` row now says what
  consumers do: load a provider's code from the same input.
- **graft grafts knowledge-base-kit** (v2.1.0) with this checkout's own `mkFlake` (not a pinned graft), into
  `scions/`, and its docs follow the kit's conventions. `nix flake check` runs the kit's link and lifecycle
  checks on them (`checks.<system>.docs`). graft's flake has a new input, `knowledge-base-kit`; a
  consumer's `flake.lock` gains a node for it on upgrade, which nothing in the consumer evaluates.
- **graft runs its own workflows:** `.forgejo/workflows/` has graft-check (pushes to `v2`, through the new
  `branches:` line) and graft-upgrade, managed by this checkout's graft like any consumer's.
- **`.gitattributes`:** graft's export leaves out `scions`, `.gitignore`, `AGENTS.md` and
  `knowledge-base.yaml`. A new `.gitignore` drops `__pycache__/`, which a local run of the kit's checker
  writes into the scion.
- A pointer in an error message (`lib/export-ignored.sh`) and in `template/flake.nix` now names the page
  instead of a README section.

## v2.10.0 - 2026-11-11

Docs only: nothing graft runs changes.

### Changed
- **README and template:** examples use the providers' new names, knowledge-base-kit (was
  meta-knowledge-base) and python-kit (was python-starter).
- **README, "As a provider":** naming a repo you graft (a suffix for what the content is: `-kit`,
  `-conventions`, `-knowledge`), a glossary (provider, consumer, scion, export), the
  `graft-provider` topic, and why one repo shouldn't be taken through two inputs at one commit.

## v2.9.0 - 2026-11-10

### Fixed
- **Annotated tags defeated `exportIgnore`.** Nix 2.24 locks a `git+…?ref=v1.0.0&exportIgnore=1`
  input at an annotated tag to the tag object's sha, and its export-ignore filter then finds no
  commit there and silently ignores nothing: the scion got the provider's whole tree (405 files
  for a 197-file export), and `nix flake check` passed, since the check's expected tree was
  fetched the same way. graft now keeps such a lock at the tag's commit (README, "Annotated
  tags"):
  - `graft upgrade` locks a moved `git+` input itself, at the tag's peeled commit (`git
    ls-remote`, `^{}`), with the tag as its ref. This also gets past Nix 2.24 failing to fetch
    `ref=TAG` over HTTP or ssh into a cache that never saw the tag, which stopped `nix flake
    update` in a fresh CI container.
  - `nix run .#graft` first re-pins a lock that holds an annotated tag's object to its commit,
    then stops and asks to be run again (that run was built from the old lock); the regraft
    inside `graft upgrade` re-pins too.
  - `nix flake check`, `graft check` and `graft ci check` fail when a fetched `exportIgnore`
    export still holds a path its own `.gitattributes` marks `export-ignore`, and say why;
    `nix run .#graft` refuses to write one.

### Added
- **Private scions in CI: `# graft ssh:` in graft-check.yaml.** One comment line,
  `# graft ssh: host=[platform-vm]:2222 address=192.168.1.51 hostkey=ssh-rsa AAAA…` (and
  `secret=NAME` when the secret isn't `GRAFT_SSH_KEY`), makes graft write a step before the check
  that installs an ssh key from that Actions secret, the host's pinned key in `known_hosts`, and
  an ssh config for the host, so the job can fetch a private `git+ssh` scion. Like `paths-ignore`,
  the line is read from the repo's copy and kept by `nix run .#graft`, the check and `graft
  upgrade`; a line graft can't read, or two that differ, fail the check, and it's refused in
  graft-upgrade.yaml (README, "Private scions over ssh").
- README, "As a provider": for `git+` consumers, write `tests export-ignore`, not `tests/`; Nix
  applies the rules from a bare clone, where a pattern only a directory matches matches nothing.

### Changed
- Nothing in graft's workflows for a repo without `# graft ssh:`: they are byte for byte 2.8.0's,
  so the upgrade PR changes nothing in `.forgejo/`.
- `nix run .#graft` lists the tags of each `git+…&exportIgnore=1` input (`git ls-remote`) before it
  syncs. When the upstream can't be reached it says so and goes on; it then fails only if the
  export Nix fetched is unfiltered.

### Upgrading
- `nix run .#graft upgrade graft` (or the weekly PR) takes it. Then run `nix run .#graft`: if a
  `git+` scion's lock held an annotated tag's object, it re-pins the commit and stops; run it again
  to rebuild the scion without the files its provider export-ignores. Review that diff of `scions/` and
  `flake.lock` and commit it; `nix flake check` fails until you do.
- A provider whose `.gitattributes` has `dir/ export-ignore` and is grafted as `git+`: change it to
  `dir export-ignore` (or `/dir export-ignore`) and tag a release; nothing else ignores it.
- To fetch a private `git+ssh` scion in graft-check, after this version is pinned:
  1. Make an ssh key (`ssh-keygen -t ed25519 -N '' -f graft_ssh_key`), add `graft_ssh_key.pub` as a
     read-only deploy key on the scion's repo, and `graft_ssh_key` as the Actions secret
     `GRAFT_SSH_KEY` on the repo that grafts it; delete both files.
  2. Add one line to `.forgejo/workflows/graft-check.yaml`, with the host as the scion's url names
     it, its address if the runner can't resolve it, and the host's public key, e.g.
     `# graft ssh: host=[platform-vm]:2222 address=192.168.1.51 hostkey=ssh-rsa AAAA…`
     (add `secret=NAME` for another secret's name).
  3. `nix run .#graft` (it writes the step), `nix run .#graft check`, commit.
  Add it only after graft 2.9.0 is pinned: 2.8.x reports the edited file as differing.

## v2.8.0 - 2026-11-09

### Added
- **`paths-ignore` in graft-check.yaml.** A repo where something pushes often to files graft's
  check never reads (a notes vault synced by a bot, say) can keep graft-check from running for
  those pushes and PRs: one line, `paths-ignore: ["notebook/**"]`, under both its `push:` and
  `pull_request:` triggers. It is the one edit graft keeps in its workflows: `nix run .#graft`, the
  check and `graft upgrade` read it from the repo's copy and write it under both triggers of the
  new one, so it survives upgrades. A glob that could match the scions directory, the flake or
  graft's workflows is refused, in any spelling (`scions/**`, `flake.*`, `?forgejo/**`), as is
  one that starts with `*` or `/`, a block list, a comment after the line, and two lines that
  differ. The line is kept only in graft-check.yaml: in a workflow of graft's with no `push:` or
  `pull_request:` trigger (graft-upgrade.yaml) it fails the check rather than being dropped. graft-check has no `schedule:` trigger and needs none; checks of the ignored files
  are the repo's own (README, "Pushes that touch only some paths").

### Changed
- Nothing for a repo without `paths-ignore`: graft's workflows are byte for byte 2.7.2's, so the
  upgrade that brings this version changes nothing in `.forgejo/` and its PR opens as usual.

### Upgrading
- No step is needed: `nix run .#graft upgrade graft` (or the weekly PR) takes it.
- To skip pushes that touch only some paths, after the upgrade is merged: in
  `.forgejo/workflows/graft-check.yaml`, add `    paths-ignore: ["notebook/**"]` (four spaces, your
  own globs) as the last line under `  pull_request:` and under `  push:` (after
  `    branches: [main]`). Run `nix run .#graft check` (`nix run path:<flakeDir>#graft check` for a
  flake in a subdirectory): it passes when the line is right, and says what's wrong when it isn't.
  Add it only after graft 2.8.0 is pinned: 2.7.x reports the edited file as differing.
- If branch protection requires the graft-check status, a PR that changes only ignored files never
  gets one; stop requiring it before adding the line.

## v2.7.2 - 2026-11-09

### Fixed
- graft's workflows failed in a repo whose flake is in a subdirectory (`flakeDir`): the step that
  reads the flake's place from the scions README used `sed`, which the `nixos/nix` image doesn't
  have, so it fell back to the top and found no `flake.nix`. It reads it with bash alone now, and
  the test runs that step with only `nix` and `git` on PATH, as CI does.

### Upgrading
- `nix run path:<flakeDir>#graft upgrade graft` replaces both workflows with the fixed copies (as
  long as their `# managed by graft:` line is there). The failing workflow can't do this upgrade
  itself, so run it by hand.

## v2.7.1 - 2026-11-07

### Fixed
- `graft check`, and `checks.<system>.graft`, print what a file listed only on the left means only
  when the diff lists one, not after every diff (a single edited file, say).
- `graft changelog` and `graft migrate`: in the rest of a CHANGELOG.md, after its Upgrading
  subsections, a version whose only content was an Upgrading subsection printed its heading with
  nothing under it. It's left out.

## v2.7.0 - 2026-11-07

### Added
- **A flake in a subdirectory.** `mkFlake`'s `flakeDir` (default `.`) says where `flake.nix` sits
  under the repo's top, for a repo with content that must never be copied into the world-readable
  Nix store: run as `nix run path:nix/workspace#graft`, Nix copies only that directory. `dir` stays
  relative to the top, and the app finds the top with git, from anywhere in the checkout. The scions
  README records the flake (`<!-- graft flake: nix/workspace -->`) and names its commands; that line
  is how `outdated`, `upgrade` and `migrate` (also with `-C`), `graft ci` and graft's workflows find
  it. `upgrade` re-locks it with `nix flake update --flake path:…` (Nix 2.19 or later). Such a flake
  has no `checks.<system>.graft`, which couldn't see the scions directory.
- **`graft check`**: the working tree's scions directory, as git would commit it, and graft's
  workflows against the pins. In either layout; in a subdirectory it is the check, and
  `graft ci check`, `graft ci upgrade`'s full check and `graft migrate` run it as `graft`.
- `graft changelog [-C REPO] [--json] [NAME[@TAG]...]`: what an upgrade would take, read before
  taking it. For each input `outdated` reports behind (or each one named), the provider's
  CHANGELOG.md sections between the pinned tag and the target (`LATEST`, or the named tag, which
  may cross a major), Upgrading subsections first, or its commits when there's no CHANGELOG.md.
  Quoted and labelled as the provider's text, as `graft migrate` does; one line for an input with
  nothing to show. Fetched with plain git; reads the repo as data, so it runs from graft's own
  flake with `-C`, and changes nothing.

### Changed
- graft's workflows find the flake from the scions README before running `graft ci`. A repo whose
  flake is at its top runs exactly what it did, but the files differ, so the upgrade that brings
  this version changes `.forgejo/workflows/` and `graft ci pr` won't open its PR: review the diff
  and open it yourself, once.
- `graft migrate`'s brief names the flake's `flake.lock` and check commands when it's in a
  subdirectory.
- The upstream notes are one implementation: `upstream_range` (lib/common.sh) fetches and extracts
  for `ci pr`, `migrate` and `changelog`, and `lib/upstream.jq` renders them for `migrate` and
  `changelog`.

## v2.6.0 - 2026-11-06

### Changed
- **`lib.mkGraft` is now `lib.mkFlake`**, the conventional name for a function that returns a
  flake's outputs, and `deps` is now `scions`. `mkGraft` is gone: graft has one user, who moved
  every consumer in this release, so this breaking change stays in v2.
- **`scions` is optional.** It defaults to every `flake = false` input, so a repo declares a scion
  once. It's read from `flake.nix` as data, so nothing extra is fetched. Name them when a non-flake
  input isn't one.
- **One command.** The apps `graft-outdated`, `graft-upgrade`, `graft-migrate` and `graft-ci` are
  now subcommands of the one app: `nix run .#graft upgrade name@v1.2.0`. A subcommand needs no
  `--`; a flag still does (`nix run .#graft -- outdated --json`). `nix run .#graft` alone still
  rebuilds the scions directory, also spelled `nix run .#graft sync`. From graft's own flake it's
  `nix run github:danielnaab/graft -- outdated -C REPO`, replacing `#outdated`, `#upgrade`,
  `#migrate` and `#ci`. The one app is built from the pins, so every subcommand now needs each
  scion's pin fetchable; if one has vanished, use that outside form.
- graft's workflows moved from `examples/forgejo/` to `template/.forgejo/workflows/`, and call
  `nix run .#graft ci STEP`.

### Added
- `mkFlake`'s `outputs` argument takes the repo's own flake outputs and merges them with graft's
  at every level. This replaces wrapping the call in `nixpkgs.lib.recursiveUpdate`, and the `//`
  that silently dropped one side's `checks`. A repo can't replace `checks.<system>.graft`.
- `nix flake init -t github:danielnaab/graft` scaffolds a repo: `flake.nix` and graft's workflows.
- The `graft` app has a `meta.description`, which Nix's `lacks attribute 'meta'` warning asks for.

### Fixed
- A new scion failed `nix flake check` until its files were staged, with a message saying to run
  the command that had just been run. `graft sync` and `graft upgrade` now stage the scions
  directory, and the check's message says what a file only on the left means.
- A repo's own outputs no longer evaluate graft's checks on its arguments: `nix develop` never
  looks at the inputs on graft's behalf.

### Upgrading

In `flake.nix`: move graft to `v2.6.0`; replace `mkGraft` with `mkFlake`; delete `deps` (or rename
it `scions`, if some `flake = false` input isn't a scion); and replace any
`recursiveUpdate (mkGraft { … }) { … }` with `mkFlake { …; outputs = { … }; }`. Then
`nix run .#graft`, which refreshes the scions README and graft's workflows, and commit. Done when
`nix flake check` passes. Search the repo's own docs and scripts for `#graft-` and replace each
with the subcommand. `graft upgrade` can't do this one: 2.5's upgrade only moves the tag, and
`mkGraft` no longer exists at the new one.

## v2.5.1 - 2026-11-04

### Fixed
- `graft-upgrade` found a `git+…` input's url in `flake.nix` by the bare string `ref=<old>`, so a
  run that moved one input to the tag another was pinned at (mp v1.0.0 -> v2.0.0 while provider
  moved on from v2.0.0) found `ref=v2.0.0` twice at the second edit and stopped with `flake.nix`
  half-edited. Each input's url is now found by its whole spelling from `flake.lock`'s
  `original` (`git+URL?` with `ref=<old>` among its parameters, `github:OWNER/REPO` with `/<old>`
  or `?ref=<old>`), from a quote or space to the next quote, space or `;`, so another
  input's url, or a longer one that starts with it, never matches (`lib/retarget.jq`). And every
  edit is now made in memory, in the order the run makes them, before `flake.nix` is written: an
  edit that can't be made (the same url twice at one ref) stops the run with nothing changed.
  `github:` urls written with `?ref=` can now be moved too.
- `graft-upgrade NAME@TAG` with a TAG that isn't upstream wrote the ref, then failed at
  `nix flake update`, leaving `flake.nix` and `flake.lock` out of step (and any input moved before
  it already moved). The tag is now looked for upstream first, and the run refused with nothing
  changed. A `flake.nix` that isn't valid UTF-8 is refused too, rather than rewritten.

### Upgrading

`nix run .#graft-upgrade -- graft@v2.5.1` (the weekly PR does it too), `nix flake check`, then
commit `flake.nix` and `flake.lock`. The workflows don't change.

## v2.5.0 - 2026-11-04

### Added
- `graft-migrate` (`nix run .#graft-migrate`, or `nix run github:danielnaab/graft#migrate -- -C
  REPO`): on a branch where an upgrade is already applied (the bot's `graft/upgrade`, say), prints
  a migration brief for an agent or a person: what moved, from which tag to which; the rules (fix
  this repo's own files; never the scions directory, graft's workflows or the pins); each moved
  provider's CHANGELOG.md sections between the tags, **Upgrading** subsections first, or its
  commits, quoted and labelled as the provider's text, not instructions; the scions diff, renames
  and deletions first, then modified and added files with line counts; the lines of the repo's own
  files that name a renamed or deleted scion path (or a directory that's gone), which may need
  updating; each of the repo's checks by name, with the failing ones' log tails; and what done
  means. The base is the merge-base with `origin/HEAD` (else `origin/main`, `origin/master`,
  `main`, `master`), or `--base REF`; uncommitted changes count. `--no-check` skips the checks
  (run from graft's flake with `-C`, it then only reads the branch); `--json` prints the same data as JSON. It never runs
  an agent and never changes the repo; it exits 0 whenever it prints a brief, failing checks or
  not. README: **graft-migrate**, under **Breaking changes and migrations**.
- The upgrade PR's body ends with a hint: check out `graft/upgrade` and run
  `nix run github:danielnaab/graft/<the default branch's graft tag>#migrate -- -C .` to hand the
  migration to an agent (the tag read from the default branch's `flake.lock`, so the hint never
  names code from the branch).

### Fixed
- Upstream text in the PR body (CHANGELOG sections, commit subjects) has its control characters
  replaced: a lone CR could end the block quote and put unquoted text in the body, and ESC
  sequences could rewrite a terminal. CRLF CHANGELOGs now match their version headings, a
  CHANGELOG over 1 MB is cut rather than dropped, and a cut at a line's end keeps that line.

### Changed
- `lib/common.sh`: what `tools.sh`, `ci.sh` and `migrate.sh` share (the scions directories, the
  lock reading, the full check, the upstream fetch and the changelog extraction), so the PR body
  and the brief read upstream the same way. The `pr` job's behaviour is unchanged: it still runs
  the default branch's graft, never evaluates the upgraded flake, and fetches upstream with plain
  git from the default branch's lock, without `PR_TOKEN` in git's environment.

### Upgrading

`nix run .#graft-upgrade -- graft@v2.5.0` (the weekly PR does it too), `nix flake check`, then
commit `flake.nix` and `flake.lock`. The workflows don't change.

## v2.4.1 - 2026-11-04

### Fixed
- `graft-ci pr` failed in CI with `awk: command not found`: the `nixos/nix` image has no awk,
  and graft's tools didn't bring one. They now carry gawk and findutils too, and `tests/run.sh`
  runs `graft-ci` with nothing but `nix` on PATH, as the image does, so a missing tool fails there.

### Upgrading

From 2.4.0, by hand (2.4.0's `pr` job is the broken one, so its weekly PR can't open):
`nix run .#graft-upgrade -- graft@v2.4.1`, `nix flake check`, then commit `flake.nix` and
`flake.lock`. The workflows don't change. From earlier versions, follow 2.4.0's steps with
`v2.4.1` in place of `v2.4.0`.

## v2.4.0 - 2026-11-04

### Added
- The upgrade PR is gated by the repo's whole `nix flake check`, not only graft's check: the
  repo's own checks (link checks into the scions, lint, tests) run on every upgrade, in the
  `upgrade` job, which still holds only the job token. Each of `checks.<system>` is built on its
  own, so a failure has a name. A failing check doesn't hide the upgrade: the branch is pushed,
  the PR is titled `[checks failing] graft: upgrade` and carries a **Failing checks** section (each
  failing check's name and the tail of its log), and the run fails.
- The PR body says what changed upstream: for each input the branch moved, the provider's
  CHANGELOG.md sections between the old and new tag (`## vX.Y.Z`, `## X.Y.Z` or `## [X.Y.Z]`
  headings), or, without them, its `git log --oneline` for the range (up to 50 commits), with a
  compare link on GitHub. Capped at 6000 characters an input. The `pr` job computes it, from the
  default branch's graft: see Security below.
- `graft-outdated` has a `MAJOR` column (`major` in `--json`, `null` if none): the newest tag of a
  later major version, which is never taken unless named.
- README: **Breaking changes and migrations**, the convention for providers (a major is the
  declaration; CHANGELOG.md's **Upgrading** section says what to do, written so an agent can apply
  it; optional redirect stubs at renamed paths for one major).

### Changed
- Scions no longer cross a major version by themselves, the rule graft already applied to its own
  pin. `graft-upgrade` with no arguments, or `graft-upgrade name`, moves each to the newest tag of
  its pinned major and prints `name: vN.0.0 is a new major version; not taken`; `name@vN.0.0`
  crosses. A `0.y` series counts as a major (semver's 0.x: anything may change), so `v0.3.0` moves
  to `v0.3.4` but not to `v0.4.0`. `LATEST` in `graft-outdated` is now the newest of the pinned
  major, so a repo whose only newer tag is a new major reads `current`.
- `graft-upgrade.yaml`: the `pr` job runs when the `upgrade` job pushed even if its checks
  failed (`if: ${{ !cancelled() && needs.upgrade.outputs.changed == 'true' }}`); the `upgrade`
  job's timeout is 60 minutes (the repo's own checks run there now).
- When a PR for `graft/upgrade` is already open (from this repo's own branch, not a fork's),
  `graft-ci pr` updates its title and body to describe the new push (it used to leave them as
  first written). A new major that's held back is a `::warning::` in the upgrade job's log, even
  when nothing else moved and no PR opens.

### Security
- The PR body is untrusted upstream text, and it's written where `GRAFT_PR_TOKEN` is, so it stays
  data: the `pr` job (the default branch's graft, never the upgraded flake) reads both
  `flake.lock`s with `jq`; fetches each moved provider with `git fetch` into a bare repo (no
  checkout, so no hooks or filters run) from the upstream named by the *default branch's* lock,
  never the branch's (an input the branch points elsewhere is listed, not fetched); accepts the
  branch's revs only as full commit hashes and its refs only as version tags; runs git without
  `PR_TOKEN` in its environment and without `http.extraHeader`; and hands the text to `jq
  --rawfile` for the request, each provider's text in a block quote so a code fence it leaves
  open can't swallow or fake the rest of the body. Failing-check logs from the `upgrade` job go in as an indented code
  block, which no log line can close. The `upgrade` job writes nothing for the `pr` job beyond
  the branch and its commit message, both of which it already wrote.

### Upgrading

From 2.3.x: the weekly PR moves graft to 2.4.0, but its workflow changes, so the `pr` job refuses
to open it and links to the diff. Read the diff of `.forgejo/workflows/graft-upgrade.yaml` (the
`pr` job's `if:` and the timeout), then open the PR from that link yourself, or run the steps for
2.2.x below. That one upgrade is still made by 2.3's code: it may move a scion across a major,
it runs only graft's check, and its body has no upstream changes, so read its scion lines.

From 2.2.x (once per repo, by hand: 2.2 can't move its own pin), from the repo root:

```
nix run github:danielnaab/graft/v2.4.0#upgrade -- graft@v2.4.0
src=$(nix eval --raw --impure --expr '(builtins.getFlake "github:danielnaab/graft/v2.4.0").outPath')
cp -f --no-preserve=mode "$src"/examples/forgejo/graft-*.yaml .forgejo/workflows/
nix flake check
```

then commit `flake.nix`, `flake.lock` and `.forgejo/workflows/`. The first line moves only graft
(name it alone; the scions wait for the weekly PR), and leaves the scions directory as it was:
`assemble.sh` is unchanged since 2.2.2. `nix flake check` is now what every upgrade PR must pass,
so a check of the repo's own that fails today will mark every PR failing: fix it first.

## v2.3.0 - 2026-11-03

### Added
- graft upgrades itself. `graft-outdated` lists the `graft` input beside the scions, and
  `graft-upgrade` moves it like one (`graft-upgrade graft@v2.3.0`, or with no arguments when
  it's outdated). It moves first; then every scions directory is rebuilt with the new version's
  `assemble.sh`, and graft's workflows are replaced with the new version's. A graft pinned to a
  branch or a path is `untagged` and left alone.
- `nix run .#graft-ci -- check|upgrade|pr` (`lib/ci.sh`): the steps of the two Forgejo
  workflows, which now just call it, so they run the graft version the repo pins.
- graft's workflows carry a marker line (`# managed by graft: …`). A marked file in
  `.forgejo/workflows/` named like one in `examples/forgejo/` is graft's: `graft-upgrade`
  replaces it when graft moves, `nix run .#graft` restores it, `nix flake check` fails if it
  differs. Without the marker a workflow is yours and never read.
- graft stays within its pinned major version unless one is named (`graft@v3.0.0`).

### Changed
- `graft-upgrade.yaml` runs as two jobs. `upgrade` (upgrade, check, push `graft/upgrade`) holds
  only the job token, since moving graft runs the new version's code before review. `pr` opens
  the PR with `GRAFT_PR_TOKEN` (falling back to the job token, as before) from a fresh checkout
  of the default branch, never evaluating the upgraded flake. A PR runs the repo's
  `pull_request` workflows from its branch with the repo's secrets, so `pr` opens one only when
  the branch changes nothing but `flake.nix`, `flake.lock` and the scions directories; a change
  to `.forgejo/` or anything else fails with a link to the diff, to review and open by hand. It
  opens nothing if the branch is already in the default branch, and takes a 409 as "already
  open". The token reaches `curl` on a pipe, not in its command line.
- `graft-upgrade` checks every ref it will rewrite before changing any file, refuses to move graft
  below 2.3.0, and, in a CI job that isn't `graft-ci` (2.2's single-job workflow), leaves graft
  itself alone so the new version's code never runs beside the PR token.
- The PR and commit are titled `graft: upgrade` (was `graft: upgrade scions`).
- `graft-outdated`'s first column is `NAME` (was `SCION`), and `--json` has a row for graft
  with `dir` `-`.

Upgrading from 2.2.x (once per repo, by hand: 2.2 can't move its own pin), from the repo root:

```
nix run github:danielnaab/graft/v2.3.0#upgrade -- graft@v2.3.0
src=$(nix eval --raw --impure --expr '(builtins.getFlake "github:danielnaab/graft/v2.3.0").outPath')
cp --no-preserve=mode "$src"/examples/forgejo/graft-*.yaml .forgejo/workflows/
nix flake check
```

then commit `flake.nix`, `flake.lock` and `.forgejo/workflows/`. If graft is pinned to the
branch (`github:danielnaab/graft/v2`), the first line refuses it: change the URL to
`github:danielnaab/graft/v2.3.0` by hand and run `nix flake update graft` instead. The scions directory doesn't
change (`assemble.sh` is the same as 2.2.2's). From then on the weekly `graft/upgrade` PR brings
each new graft version, its workflows included.

## v2.2.2 - 2026-11-03

### Fixed
- `graft-upgrade.yaml` couldn't open its PR. Forgejo refuses the job token for that call on a
  private repo (404 from `CreatePullRequest`), so the PR is now opened with the Actions secret
  `GRAFT_PR_TOKEN`, falling back to the job token. On failure the branch is still pushed and
  the error links to the compare page.
- The PR step used `sed`, which the `nixos/nix` image doesn't have, and ran its API calls in
  a nested `bash -c`. It now fetches `jq` by store path, and does the rest in plain bash.

## v2.2.1 - 2026-11-03

### Fixed
- `examples/forgejo/graft-upgrade.yaml` saved its checkout token as an unscoped
  `http.extraheader`, so `graft-upgrade`'s `git ls-remote` sent it to every provider's server.
  GitHub refused it, and the scion read as unknown. The header is now scoped to the repo's own
  server, and graft clears `http.extraHeader` for its own `ls-remote` calls.
- An upstream that can't be read made `graft-upgrade` report "nothing to upgrade". It now prints
  git's error and fails without changing anything. A `git grep` failure (a repo git won't trust)
  is an error too, not "no scions here".
- The upgrade workflow's log is written outside the checkout: `git add` failed when the repo's
  `.gitignore` ignored it.

## v2.2.0 - 2026-11-03

### Added
- `nix run .#graft-outdated`: each scion's pinned tag against the newest semver tag upstream
  (`git ls-remote`); `--json` for machines.
- `nix run .#graft-upgrade [name[@tag]...]`: moves scions to newer tags, re-locks them and
  rebuilds the scions directory.
- Both read the repo as data and never call its flake's outputs, so graft's own flake runs them
  against any checkout: `nix run github:danielnaab/graft#outdated -- -C <repo>`.
- `examples/forgejo/`: a check workflow and a weekly upgrade workflow that opens a PR.

### Changed
- `lib/assemble.sh` builds the scions directory for both the check and graft-upgrade. The
  directory's README now names `graft-upgrade`; repos moving to 2.2 re-run `nix run .#graft`.

## v2.1.0 - 2026-11-03

### Changed
- Scions: grafted exports go to `scions/` by default (was `vendor/`, which Go, PHP and Ruby use
  for their own dependencies). The README graft writes there now starts with a marker line.
- `dir` takes any relative path inside the repo: nested, hidden, with spaces or quotes. It's
  quoted everywhere it reaches a shell, and refused if absolute, empty, or with `.`/`..` segments.

### Added
- graft refuses to replace an existing directory it didn't write (no marker in its `README.md`),
  so a mistyped `dir` can't wipe your code. A `vendor/` written by 2.0.0 is recognised.

Upgrading from 2.0.0: bump the input, `git mv vendor scions` (or set `dir = "vendor"`), then
`nix run .#graft` and update links that pointed into `vendor/`.

## v2.0.0 - 2026-11-02

A new design, sharing no history with graft v1 (branch `v1`): pinned, provider-defined exports
via Nix flake inputs, copied by `nix run .#graft` and checked by `nix flake check`.
