---
status: working
---
# Move from v2.14.0's sandbox

graft v2.15.0 replaces v2.14.0's `sandbox` launcher with `pot`. A repository no longer builds its own launcher:
your root runs `pot`, and grants come from your root's grants file, not from `flake.nix`. This page moves a
consumer, a provider and a person's habits. The new model is in [pot](../reference/pot.md#the-model). It names
v2.16.0's bundle, `pot-kits`; from v2.15.0, see [Move to v2.16.0](move-to-v2-16.md).

## What changed

| v2.14.0 | v2.15.0 |
|---|---|
| `perSystem.sandbox`; a provider's `sandbox` in `graft.nix` | `perSystem.kit`; a provider's `kit` |
| `paths.readOnly`, `paths.writable`, `network.allow`, `env.pass` | `requests.read`, `requests.write`, `requests.network`, `requests.env` |
| `grant = [ "notebook-kit" ];` in `flake.nix` | `pot grant`, into the root's grants file, by the kit's identity |
| `repository.readOnly = true` | `pot inspect`, or a context with `access = "read"` |
| `"host"` means ports 443 and 80 | `"host"` means port 443 only; write `"host:80"` for plain HTTP |
| `nix run .#sandbox -- CMD`, `packages`/`apps.<system>.sandbox`, the dev shell's `sandbox` | the root's `pot CMD`; graft's default root; repositories export `packages.<system>.pot-kits` (`graft-kits` in v2.15.0) |
| `--with PKG`, `--allow HOST` | `--package PKG`, `--net HOST` |
| `--explain`, `--dry-run` | `pot explain`, `pot explain --backend-command` |
| `GRAFT_SANDBOX=1` | `POT=<context>`. Test it with `[ -n "${POT:-}" ]` only. |
| A policy built with the launcher | Kit manifests, composed at each start with your grants |
| Seatbelt on macOS | None in v2.15.0: `pot` refuses on macOS. A repository used on a Mac keeps its sandbox only on v2.14.0; the choice is per repository, by its graft pin. |

The names `sandbox`, `pot`, `pot-kits` and `graft-kits` are graft's: a provider, `perSystem` or mkFlake's `outputs` cannot
define them as packages or apps. There is no `packages.<system>.sandbox` or `apps.<system>.sandbox` any more.

A `sandbox` left in `perSystem` or a provider's `graft.nix` fails the tools and `pot-kits` with:

```
graft: the consumer's perSystem gives `sandbox`: graft v2.15.0 replaced it with `kit` (pot; see graft's CHANGELOG)
graft: scions/notebook-kit/graft.nix gives `sandbox`: graft v2.15.0 replaced it with `kit` (pot; see graft's CHANGELOG)
```

## Move a consumer

The steps write `F` for your flake's reference, from the repository's top: `.` for a flake at the top, or
`'path:nix/agent-tools'` for a [flake in a subdirectory](flake-in-a-subdirectory.md). A subdirectory flake is
always a `path:` flake, never `.#` or `git+file:`, which would copy the whole repository into the store.

1. Take graft v2.15.0: `nix run F#graft upgrade graft` (`nix run .#graft upgrade graft`, or
   `nix run 'path:nix/agent-tools#graft' upgrade graft`).
2. In your flake's `flake.nix`, rename `perSystem`'s `sandbox` to `kit`, and move each option to its new name:

   ```nix
   # v2.14.0
   sandbox = {
     packages = [ pkgs.ripgrep ];
     protect = [ "AGENTS.md" ];
     network.allow = [ "registry.npmjs.org" ];
     env.pass = [ "NPM_TOKEN" ];
     grant = [ "notebook-kit" ];
   };

   # v2.16.0 (v2.15.0 wrote packages, which is still read until v2.17.0)
   kit = {
     tools = [ pkgs.ripgrep ];
     protect = [ "AGENTS.md" ];
     requests.network = [ "registry.npmjs.org" ];
     requests.env = [ "NPM_TOKEN" ];
   };
   ```

3. Delete `grant` and `repository.readOnly`. Grants now live in your root; a read-only run is `pot inspect`.
4. If a `"host"` entry needs plain HTTP, add `"host:80"`.
5. If a run inside runs `graft check` (`just check`, say), add the `nix` capability: `graft check` evaluates each
   scion's `graft.nix`.

   ```nix
   kit.requests.capabilities = [ { name = "nix"; } ];
   ```

   The capability does nothing until your root binds it (`nix."*" = { via = "daemon-untrusted"; };`; graft's
   default root and `pot init`'s root do) and you run `pot grant`.

6. Make the repository's tools work inside ([below](#run-the-repositorys-tools-inside)).
7. Replace `nix run .#sandbox -- CMD` in recipes and notes with `pot CMD`. Replace a test of
   `GRAFT_SANDBOX = 1` with `[ -n "${POT:-}" ]`.
8. Run `nix flake check F` (`nix flake check` for a flake at the top), and commit. A commit under the flake
   directory changes the repo kit's source, so a powerful grant asks again at the next run.
9. On your machine, set up a root ([Set up a root](run-in-a-sandbox.md#set-up-a-root)), run `pot claude` in the
   repository, and grant the kits it asks for.

## Run the repository's tools inside

`pot` builds the workspace's tools on the host, from `packages.<system>.pot-kits`, caches them, and passes their
`bin/` to the run as `POT_TOOLS`. Those tools include `graft`. A script or recipe that builds tools with
`nix` should use them instead, so that it works without `nix` and without the network.

graft ships no `ensure.sh`. If your repository has one that prints the tools' `bin/`, add this branch as its first
step after `set -eu`, before any other logic:

```sh
set -eu
# In a pot run, pot passes the workspace tools it built and cached on the host.
if [ -n "${POT_TOOLS:-}" ]; then printf '%s\n' "$POT_TOOLS"; exit 0; fi
```

Trusting the variable is safe: inside a run, it only chooses which tools the run's own commands use.

If your `justfile` runs graft through `nix run`, make it prefer `graft` from `PATH` inside a run:

```just
graft_run := if env("POT_TOOLS", "") != "" { "graft" } else { "nix run 'path:nix/agent-tools#graft' --" }

graft-check:
    {{graft_run}} check
```

Use your flake's reference for `path:nix/agent-tools`. `graft check` still evaluates `graft.nix` inside, so the
run needs the `nix` capability (step 5).

## Move a provider

1. In `graft.nix`, rename `sandbox` to `kit`, and move `network.allow`, `paths.readOnly`, `paths.writable` and
   `env.pass` under `requests`, as for a consumer.
2. If a request depends on the consumer, take it from the `kitArgs` argument of `graft.nix`, which the consumer
   sets in `perSystem.kitArgs.<scion>`, under your scion's folder name. Never read it from a file in the workspace: requests are literal.
3. Release it. A new request is a change that every consumer must grant: say so in your CHANGELOG. A consumer's
   `pot` stops the runs that the request applies to until the consumer runs `pot grant`.

## Move your own habits

| You ran | Run |
|---|---|
| `nix run .#sandbox -- claude` | `pot claude` |
| `nix run .#sandbox -- --explain` | `pot explain` |
| `nix run .#sandbox -- --with ripgrep rg TODO` | `pot --package ripgrep rg TODO` |
| `nix run .#sandbox -- --allow pypi.org claude` | `pot --net pypi.org claude` |
| `sandbox claude`, in the dev shell | `pot claude` |
| a run with `repository.readOnly = true` | `pot inspect claude` |

## Check it worked

From the consumer's top, paste this. It prints `ok` only if the flake has a repo kit output, has no `sandbox`
launcher, and a run sees the tools:

```sh
F=.   # or F='path:nix/agent-tools' for a flake in a subdirectory
nix eval --raw "$F#packages.x86_64-linux.pot-kits.name" >/dev/null &&
  ! nix eval "$F#packages.x86_64-linux" --apply 'p: p ? sandbox' | grep -q true &&
  pot bash -c 'test -n "$POT_TOOLS" && command -v graft >/dev/null' && echo ok
```

Use your machine's system for `x86_64-linux`.

## If something goes wrong

- **`… gives \`sandbox\`: graft v2.15.0 replaced it with \`kit\``.** A `perSystem` or a provider still declares
  `sandbox`. Rename it to `kit` (step 2), or take the provider's release that does.
- **A test fails because `/run/current-system/sw/bin/…` is missing.** In `work` without the `nix` capability, the
  run sees only its closure in the store, so that link does not resolve. Make the test use the `PATH` it was given.
- **`just check` inside a run fails with `nix needs the nix capability: …`.** The repo kit lacks the `nix`
  capability, or the root binds none, or `graft_run` still calls `nix run`. Make all three changes, then
  `pot grant`.
- **`ensure.sh: … nix: command not found` inside a run.** `ensure.sh` lacks the `POT_TOOLS` branch, or
  it is not the first step.
- **On macOS, `pot: no backend for darwin in v2.16.1`.** Keep graft at v2.14.0 on that machine's repositories
  until the Seatbelt backend returns.

## Sources

- [pot](../reference/pot.md)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `kit`, `kitArgs`, `packages.<system>.pot-kits` and `graft-kits`
- [CHANGELOG](../../CHANGELOG.md)
