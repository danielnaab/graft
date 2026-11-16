---
status: draft
---
# Move from v2.14.0's sandbox

graft v2.15.0 replaces v2.14.0's `sandbox` launcher with `sb`. A repository no longer builds its own launcher:
your root runs `sb`, and grants come from your root's grants file, not from `flake.nix`. This page moves a
consumer, a provider and a person's habits. The new model is in [Sandbox](../reference/sandbox.md#the-model).

## What changed

| v2.14.0 | v2.15.0 |
|---|---|
| `perSystem.sandbox`; a provider's `sandbox` in `graft.nix` | `perSystem.kit`; a provider's `kit` |
| `paths.readOnly`, `paths.writable`, `network.allow`, `env.pass` | `requests.read`, `requests.write`, `requests.network`, `requests.env` |
| `grant = [ "notebook-kit" ];` in `flake.nix` | `sb grant`, into the root's grants file, by the kit's identity |
| `repository.readOnly = true` | `sb inspect`, or a context with `access = "read"` |
| `"host"` means ports 443 and 80 | `"host"` means port 443 only; write `"host:80"` for plain HTTP |
| `nix run .#sandbox -- CMD`, `packages`/`apps.<system>.sandbox`, the dev shell's `sandbox` | the root's `sb CMD`; graft's default root; repositories export `packages.<system>.graft-kits` |
| `--with PKG`, `--allow HOST` | `--package PKG`, `--net HOST` |
| `--explain`, `--dry-run` | `sb explain`, `sb explain --backend-command` |
| `GRAFT_SANDBOX=1` | `GRAFT_SANDBOX=<context>`. Test it with `[ -n "${GRAFT_SANDBOX:-}" ]` only. |
| A policy built with the launcher | Kit manifests, composed at each start with your grants |
| Seatbelt on macOS | None in v2.15.0: `sb` refuses on macOS. Stay on v2.14.0 there. |

The names `sandbox`, `sb` and `graft-kits` are graft's: a provider, `perSystem` or mkFlake's `outputs` cannot
define them as packages or apps. There is no `packages.<system>.sandbox` or `apps.<system>.sandbox` any more.

A `sandbox` left in `perSystem` or a provider's `graft.nix` fails the tools and `graft-kits` with:

```
graft: the consumer's perSystem gives `sandbox`: graft v2.15.0 replaced it with `kit` (graft sb; see graft's CHANGELOG)
graft: scions/notebook-kit/graft.nix gives `sandbox`: graft v2.15.0 replaced it with `kit` (graft sb; see graft's CHANGELOG)
```

## Move a consumer

1. Take graft v2.15.0: `nix run .#graft upgrade graft`.
2. In `flake.nix`, rename `perSystem`'s `sandbox` to `kit`, and move each option to its new name:

   ```nix
   # v2.14.0
   sandbox = {
     packages = [ pkgs.ripgrep ];
     protect = [ "AGENTS.md" ];
     network.allow = [ "registry.npmjs.org" ];
     env.pass = [ "NPM_TOKEN" ];
     grant = [ "notebook-kit" ];
   };

   # v2.15.0
   kit = {
     packages = [ pkgs.ripgrep ];
     protect = [ "AGENTS.md" ];
     requests.network = [ "registry.npmjs.org" ];
     requests.env = [ "NPM_TOKEN" ];
   };
   ```

3. Delete `grant` and `repository.readOnly`. Grants now live in your root; a read-only run is `sb inspect`.
4. If a `"host"` entry needs plain HTTP, add `"host:80"`.
5. If a run inside runs `graft check` (`just check`, say), add the `nix` capability: `graft check` evaluates each
   scion's `graft.nix`.

   ```nix
   kit.requests.capabilities = [ { name = "nix"; } ];
   ```

6. Make the repository's tools work inside ([below](#run-the-repositorys-tools-inside)).
7. Replace `nix run .#sandbox -- CMD` in recipes and notes with `sb CMD`. Replace a test of
   `GRAFT_SANDBOX = 1` with `[ -n "${GRAFT_SANDBOX:-}" ]`.
8. Run `nix flake check`, and commit.
9. On your machine, set up a root ([Set up a root](run-in-a-sandbox.md#set-up-a-root)), run `sb claude` in the
   repository, and grant the kits it asks for.

## Run the repository's tools inside

`sb` builds the workspace's tools on the host, from `packages.<system>.graft-kits`, caches them, and passes their
`bin/` to the run as `GRAFT_SANDBOX_TOOLS`. Those tools include `graft`. A script or recipe that builds tools with
`nix` should use them instead, so that it works without `nix` and without the network.

graft ships no `ensure.sh`. If your repository has one that prints the tools' `bin/`, add this branch as its first
step after `set -eu`, before any other logic:

```sh
set -eu
# In a graft sandbox run, sb passes the workspace tools it built and cached on the host.
if [ -n "${GRAFT_SANDBOX_TOOLS:-}" ]; then printf '%s\n' "$GRAFT_SANDBOX_TOOLS"; exit 0; fi
```

Trusting the variable is safe: inside a run, it only chooses which tools the run's own commands use.

If your `justfile` runs graft through `nix run`, make it prefer `graft` from `PATH` inside a run:

```just
graft_run := if env("GRAFT_SANDBOX_TOOLS", "") != "" { "graft" } else { "nix run 'path:nix/agent-tools#graft' --" }

graft-check:
    {{graft_run}} check
```

Use your flake's reference for `path:nix/agent-tools`. `graft check` still evaluates `graft.nix` inside, so the
run needs the `nix` capability (step 5).

## Move a provider

1. In `graft.nix`, rename `sandbox` to `kit`, and move `network.allow`, `paths.readOnly`, `paths.writable` and
   `env.pass` under `requests`, as for a consumer.
2. If a request depends on the consumer, take it from the `kitArgs` argument of `graft.nix`, which the consumer
   sets in `perSystem.kitArgs.<your kit>`. Never read it from a file in the workspace: requests are literal.
3. Release it. A new request is a change that every consumer must grant: say so in your CHANGELOG. A consumer's
   `sb` stops the runs that the request applies to until the consumer runs `sb grant`.

## Move your own habits

| You ran | Run |
|---|---|
| `nix run .#sandbox -- claude` | `sb claude` |
| `nix run .#sandbox -- --explain` | `sb explain` |
| `nix run .#sandbox -- --with ripgrep rg TODO` | `sb --package ripgrep rg TODO` |
| `nix run .#sandbox -- --allow pypi.org claude` | `sb --net pypi.org claude` |
| `sandbox claude`, in the dev shell | `sb claude` |
| a run with `repository.readOnly = true` | `sb inspect claude` |

## Check it worked

From the consumer's top, paste this. It prints `ok` only if the flake has a repo kit output, has no `sandbox`
launcher, and a run sees the tools:

```sh
nix eval --raw '.#packages.x86_64-linux.graft-kits.name' >/dev/null &&
  ! nix eval '.#packages.x86_64-linux' --apply 'p: p ? sandbox' | grep -q true &&
  sb bash -c 'test -n "$GRAFT_SANDBOX_TOOLS" && command -v graft >/dev/null' && echo ok
```

Use your machine's system for `x86_64-linux`.

## If something goes wrong

- **`… gives \`sandbox\`: graft v2.15.0 replaced it with \`kit\``.** A `perSystem` or a provider still declares
  `sandbox`. Rename it to `kit` (step 2), or take the provider's release that does.
- **A test fails because `/run/current-system/sw/bin/…` is missing.** In `work`, the run sees only its closure, so
  that link does not resolve. Make the test use the `PATH` it was given.
- **`just check` inside a run fails with `nix: command not found`.** The repo kit lacks the `nix` capability, or
  `graft_run` still calls `nix run`. Add both changes, then `sb grant`.
- **`ensure.sh: … nix: command not found` inside a run.** `ensure.sh` lacks the `GRAFT_SANDBOX_TOOLS` branch, or
  it is not the first step.
- **On macOS, `sb: no backend for darwin in v2.15.0`.** Keep graft at v2.14.0 on that machine's repositories
  until the Seatbelt backend returns.

## Sources

- [Sandbox](../reference/sandbox.md)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `kit`, `kitArgs` and `packages.<system>.graft-kits`
- [CHANGELOG](../../CHANGELOG.md)
