---
status: working
---
# Move to v2.16.0

graft v2.16.0 puts a one-way boundary between graft and pot, graft's sandbox
([Decisions](../decisions.md#one-repository-one-way-arrows-between-graft-and-pot)). A repository now presents its
kits to pot as `packages.<system>.pot-kits`, and pot finds that flake from `.gitmodules` alone. pot's core no
longer knows any one agent: an agent kit says how pot recognises it and its instances (kit 1.1), a root chooses an
instance with `CMD@NAME`, and the claude-code kit takes an `api` value. Most repositories and roots need no edit
now: the old forms still work, with a warning, until v2.17.0. This page says what changes for a consumer, a root
and a provider.

## When to use this

- You have a consumer (a repository that grafts scions) on graft v2.15.0, and a pot root.
- Or you have a root (`homeManagerModules.pot` or `graft.lib.mkRoot`) on graft v2.15.0.

You can move the consumer and the root in either order: each side reads the other's v2.15.0 form until v2.17.0.
That holds while providers keep their kits in v2.15.0's form, and while the consumer's own kit uses no kit 1.1
field (`dirs`, `agent`): a v2.16.0 consumer's `graft-kits` translates its own kit for a v2.15.0 root, but leaves
out a kit that uses a 1.1-only field, so under a v2.15.0 root that repository has no repo kit. And a v2.15.0
consumer cannot build a provider's kit that uses `tools` ([Move a provider](#move-a-provider)).

## What still works until v2.17.0

These are deprecated in v2.16.0 and removed in v2.17.0:

| v2.15.0 | v2.16.0 | Until v2.17.0 |
|---|---|---|
| `packages.<system>.graft-kits` | `packages.<system>.pot-kits` | mkFlake writes both, so a v2.15.0 root reads a v2.16.0 consumer: `graft-kits` gives each kit as v2.15.0 reads it, and leaves out one that uses kit 1.1 (`graft: graft-kits: FILE uses kit 1.1 (dirs or agent), which a v2.15.0 pot cannot read: left out (pot-kits has it)`). A v2.16.0 pot reads `graft-kits` when a flake has no `pot-kits`, so it reads a v2.15.0 consumer, and says so at start: `~/DIR has graft-kits only: its graft predates v2.16.0 (pot reads graft-kits until v2.17.0)`. |
| `kit.packages` | `kit.tools` | Read as `tools`. A consumer's own `perSystem.kit` gets the evaluation warning `flake.nix (perSystem): packages is tools' old name; packages goes in v2.17.0`; a provider's `graft.nix` gets none. Both together are an error: `pot: FILE: tools and packages are one field (packages is its old name): give tools`. A provider keeps `packages` until v2.17.0, with no warning ([Move a provider](#move-a-provider)). |
| the claude-code kit's `account = "flexion";` | (nothing: the root's `kits` key names the instance) | Ignored, with the warning `graft.kits.claude-code: account is not used: the instance is the root's kits key; account goes in v2.17.0`. |
| `pot --account NAME CMD` | `pot CMD@NAME`, or `pot --instance NAME CMD` | Works as `--instance`, after one line: `pot: --account is --instance; --account goes in v2.17.0`. |
| `defaultAccount = "personal";` | `defaultInstance.claude-code = "personal";` | Applied to every kit with instances, with the evaluation warning `graft.lib.mkRoot: defaultAccount is defaultInstance.<kit>; defaultAccount goes in v2.17.0` (from the module: `programs.pot.defaultAccount is programs.pot.defaultInstance.<kit>; defaultAccount goes in v2.17.0`). Both together are an error. |
| the claude-code kit's `provider = "bedrock"; region = "us-east-1";` | `api = graft.apis.bedrock { region = "us-east-1"; };` | With the warning `graft.kits.claude-code: provider and region are api = graft.apis.bedrock { region = …; } (or graft.apis.anthropic); they go in v2.17.0`. Both together are an error. |
| a 1.0 command's `fixedDirs`, `realDirs`, `skillDirs`, `memory` | kit 1.1's `dirs` | Kit 1.0 is still read, and its fields with it: only a kit that declares `version = "1.1"` must use `dirs`. |

The old forms and the new give the same canonical requests, so no grant asks again for them.

## What changes without an edit

- **One new grant for a repo kit with powerful requests.** Taking v2.16.0 changes your flake's `flake.lock`, so
  the tree of your flake directory changes, and a powerful grant is fixed to that tree. The next run stops with
  exit 125 (or prompts, at a terminal) with `changed source`, and the same requests:

  ```
  pot grant flexion-assistant
  ```

  The source is the flake directory as committed and staged, so the prompt comes after you commit (or stage) the
  new `flake.lock`, not before: commit first, then grant.

  Use your repository's short name. No canonical request changes in v2.16.0, so no other grant asks again.
- **graft's command left a run's standard tools.** In a graft consumer, a run still has the repository's own
  `graft`, from its bundle (`POT_TOOLS`). In any other repository, a run has no `graft` unless your root adds it
  ([below](#move-a-root)).
- **Protected paths.** `**/graft.nix` and the scions folder are now protected because the consumer's bundle names
  them, not by pot's own list. `pot explain` labels them `(repo)`; pot's own defaults are labelled `(pot)`, not
  `(graft)`. In `policy.json`, pot's defaults are `{ "root": "default" }`, not `{ "root": "graft" }`.
- **Messages name a kit's file as the bundle reports it:** `the repo kit's requests.network (flake.nix (perSystem))`,
  not `(perSystem.kit)`. Kit errors from the build start `pot:`, not `graft:`:
  `pot: <file>: <field> is not a kit field (…)`.
- **The instance follows the caller's variables, most specific first.** v2.15.0 picked the instance named like the
  caller's `CLAUDE_CONFIG_DIR` folder. v2.16.0 compares the caller's variables with each instance's
  `agent.select.env`: for the claude-code kit, `CLAUDE_CONFIG_DIR = "~/<configDir>"`, and for Bedrock also
  `CLAUDE_CODE_USE_BEDROCK = "1"`. The instance that matches the most variables wins. So a shell set up for
  Bedrock now picks the Bedrock instance, not the plain one that shares its config folder. `CMD@NAME` always wins.
- **The startup line says `instance NAME`,** not `account NAME`, and the resume line names the instance:
  `pot: resume this session with: pot claude@flexion --resume ID`.
- **`pot host --resume` does not resume a run from before v2.16.0.** Such a run has no `resume.json`, which now
  says where its transcripts are. Read its transcript in its state folder instead.
- **Your consumer's lock gets shorter.** graft's flake no longer has a `knowledge-base-kit` input, so it leaves
  your `flake.lock` at the next update.

## Move a consumer

1. Take v2.16.0, from the repository's top:

   ```
   nix run .#graft upgrade
   ```

   ```
   graft: graft v2.15.0 -> v2.16.0
   …
   ```

   For a flake in a subdirectory, run `nix run 'path:DIR#graft' upgrade`.
2. If the repository's own `outputs` defines `packages.<system>.pot-kits` or `apps.<system>.pot-kits`, rename it:
   the name is graft's now. The build says so:
   `graft: mkFlake's outputs has packages.<s>.pot-kits: that name is graft's; rename the repo's own`.
3. If the repository has a `perSystem.kit` but no submodules, pot no longer finds its flake: a repository with no
   `.gitmodules` has no repo kit from its flake, and pot never evaluates it. Move the kit into your root's
   `repoKits."<origin>"`, or graft a scion.
4. If a script or hook inside a run reads `/run/pot/policy.json` to decide whether it may fetch, ask pot instead:

   ```sh
   if pot why "$url" --json >/dev/null; then git ls-remote --tags "$url"; fi
   ```

   It exits 0 when the run reaches the URL, and 1 when it does not ([run-env/1](../../pot/contracts/run-env-1.md)).
5. In `perSystem.kit`, rename `packages` to `tools`. Your own kit only: your graft, now v2.16.0, builds both
   bundles, and its `graft-kits` gives `tools` as `packages` to a v2.15.0 root. A scion's kit is the provider's to
   rename:

   ```nix
   # v2.15.0
   kit = { packages = [ pkgs.nodejs_22 ]; requests.network = [ "registry.npmjs.org" ]; };
   # v2.16.0
   kit = { tools = [ pkgs.nodejs_22 ]; requests.network = [ "registry.npmjs.org" ]; };
   ```

6. Run `nix run .#graft check` and `nix flake check`, and commit.
7. Run `pot claude` (or any run) at a terminal, and grant the repo kit's changed source when it asks. The prompt
   follows the commit of step 6.

## Move a root

1. Move the root's `graft` input to v2.16.0, and update its lock. For example, in a home-manager flake:

   ```nix
   inputs.graft.url = "github:danielnaab/graft/v2.16.0";
   ```

   ```
   nix flake update graft
   ```

2. If your runs need `graft` outside graft consumers, add it to the root's tools:

   ```nix
   programs.pot.tools = [ inputs.graft.packages.${pkgs.stdenv.hostPlatform.system}.default ];
   ```

   With mkRoot, pass the same list as `tools`.
3. Replace the deprecated forms, before v2.17.0 removes them. Before:

   ```nix
   programs.pot = {
     kits."claude-code@personal" = inputs.graft.kits.claude-code { inherit pkgs; };
     kits."claude-code@flexion-bedrock" = inputs.graft.kits.claude-code {
       inherit pkgs; account = "flexion"; configDir = ".claude-accounts/flexion";
       provider = "bedrock"; region = "us-east-2";
     };
     defaultAccount = "personal";
   };
   ```

   After:

   ```nix
   programs.pot = {
     kits."claude-code@personal" = inputs.graft.kits.claude-code { inherit pkgs; };
     kits."claude-code@flexion-bedrock" = inputs.graft.kits.claude-code {
       inherit pkgs; configDir = ".claude-accounts/flexion";
       api = inputs.graft.apis.bedrock { region = "us-east-2"; };
     };
     defaultInstance.claude-code = "personal";
   };
   ```

   `account` goes too: the root's `kits` key names the instance. With mkRoot, the same arguments go to
   `graft.lib.mkRoot`. An Anthropic instance needs no `api`: the default is
   `graft.apis.anthropic`.
4. If you named an instance with `--account` in scripts or a workmux agent command, use `CMD@NAME`:
   `pot --account flexion claude` becomes `pot claude@flexion`.
5. If a tool besides Claude Code or Codex should count as an agent (so that `pot grant` refuses under it), add its
   variable to `agentMarkers`. pot's floor, `CODEX_SANDBOX` and `CLAUDECODE`, and `POT` always apply, whatever
   kits the root has: `agentMarkers` adds to them, and never removes one.
6. Build the root (`home-manager switch`, or rebuild your mkRoot flake). Build it with no evaluation warning from
   graft, and the deprecated forms are gone. The grants file needs no edit.

## Move a provider

`graft.nix` is still [provider/2](../reference/provider.md). A `kit` with an unknown field now fails the consumer's
`pot-kits` build with pot's message, `pot: <dir>/<name>/graft.nix: <field> is not a kit field (…)`.

**Keep `packages`, and give no `version`, until v2.17.0.** Each consumer's own graft builds your `graft.nix`. A
consumer still on graft v2.15.0 refuses `tools` and `version` in your kit, and its whole bundle with it, so every
run in that consumer stops. graft v2.16.0 gives no deprecation warning for a provider's `packages`, for that reason.
Rename `packages` to `tools` in the release you make for graft v2.17.0, and say in its
CHANGELOG that it needs graft v2.16.0 or later. A scion kit may not declare `agent`, which is a root's.

## Check it worked

In the consumer, from its top, paste this. It prints `ok` when the flake has both bundles and pot reads it with
no stale grant:

```sh
F=.   # or F='path:nix/agent-tools' for a flake in a subdirectory
nix eval --raw "$F#packages.x86_64-linux.pot-kits.name" | grep -qx pot-kits &&
  nix eval --raw "$F#packages.x86_64-linux.graft-kits.name" | grep -qx graft-kits &&
  pot bash -c 'test -n "$POT_TOOLS" && command -v graft >/dev/null' && echo ok
```

Use your machine's system for `x86_64-linux`.

## If something goes wrong

- **The run stops with exit 125 and `changed source`, listing `flake.lock`.** That is the one expected new grant:
  run `pot grant NAME` at a terminal.
- **`pot explain` shows no `(repo)` items in a repository that has a repo kit.** The repository has no
  submodules, so pot found no flake. Use the root's `repoKits` (step 3).
- **`graft: command not found` inside a run, in a repository that is not a graft consumer.** Add graft to the
  root's `tools` (root step 2).
- **`pot: claude: the caller's variables match claude-code@a and claude-code@b equally: choose one with
  claude@NAME or --instance NAME`.** Two instances have the same `select.env` (the same `configDir` and API).
  Name one with `CMD@NAME`, or set `defaultInstance` to one of them.
- **`graft.lib.mkRoot: defaultInstance.claude-code = "X": the root declares claude-code@a, claude-code@b`.** The
  default names no declared instance. Use one of the names it lists. A kit name that the root lacks gives
  `graft.lib.mkRoot: defaultInstance.KIT: the root has no kit KIT@NAME (kits: …)`.
- **A run picks a different instance than in v2.15.0.** The Bedrock instance now wins in a Bedrock shell. Name the
  instance with `CMD@NAME` when you want the other.
- **`pot (work): ~/DIR#pot-kits: kit.json is version 2, and this pot reads version 1: …`.** The consumer's graft
  is newer than your root's pot. Move the root first.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): `pot-kits`, `graft-kits` and the reserved names
- [pot/lib/kits-for.nix](../../pot/lib/kits-for.nix): the bundle
- [pot/core/pot.py](../../pot/core/pot.py): finding the flake, the `graft-kits` fallback, `pot why --json`
- [pot's contracts](../../pot/contracts/README.md)
- [pot/core/compose.py](../../pot/core/compose.py): kit versions and the deprecated forms
- [pot/kits/claude-code.nix](../../pot/kits/claude-code.nix): `api`, `provider` and `region`
- [CHANGELOG](../../CHANGELOG.md)
