---
status: working
---
# Clones, worktrees, sandboxes and CI

A new clone or worktree of a consumer has empty scion folders until something fills them. Know what still
works when they are empty, fill them where it matters, and set up your machine so that it happens by itself.

## When to use this

- `ls scions/NAME` prints nothing, after `git clone`, `git worktree add`, or in an agent sandbox or CI job.
- `just` does not list a provider's recipes, or a tool that reads `scions/…` in the working tree fails.
- `nix build 'path:…'` warns `graft: … are empty in the flake's source`, or a git flake build fails
  with `error: Failed to fetch git repository '…/scions/NAME'`.
- `git worktree remove` fails with `fatal: working trees containing submodules cannot be moved or removed`.

## What works with empty scions

| Use | Empty scions |
|---|---|
| A top-level git flake: `nix build .#tools`, `nix flake check`, `nix run .#graft …`, with no tracked file modified | Works, if every scion's commit is on its provider's remote: Nix fetches each scion at its gitlink from the URL in `.gitmodules`, before graft runs. Otherwise `nix run github:danielnaab/graft/v2.21.0 -- check -C .` still runs. |
| The same, after any tracked file is modified (an agent's first edit, a CI step that writes a file) | Fails inside Nix: `error: Failed to fetch git repository '…/scions/NAME'`. Nix reads the working tree, and the scion is empty. `nix run .#graft sync` cannot run either, because the flake does not evaluate: run `git submodule update --init --recursive` first. |
| `nix run github:danielnaab/graft/v2.21.0 -- outdated -C .` and the other `-C` commands | Works: graft reads `.gitmodules` and the gitlinks. |
| A `path:` flake in a subdirectory | `tools` builds without the empty scions, with a warning; a provider that needs one fails. Fill the scions first. |
| Reading a scion's files, agents following them, `PYTHONPATH=scions/…` | Nothing to read. Fill the scions first. |
| `just` with `import? "scions/…"` | Lists your own recipes, without the provider's. With `import` (no `?`), `just` fails to parse. |
| `graft check` | Fails: `scions/NAME is empty: …`. `graft ci check` does not need filled scions. |

## Fill the scions

Any one of these, from the repository's top:

```
git submodule update --init --recursive   # git alone
nix run .#graft sync                      # the same, and rewrites the marker README and graft's workflows
git clone --recurse-submodules URL        # at clone time
```

They fill the scions at their gitlinks, and the scions' own submodules too. Fill those as well: without
`--recursive`, git leaves them empty, and a build with any tracked file modified then fails on them
(`graft check` says `scions/NAME's own submodules are empty`). For a shallow clone, add `--depth 1` to `git submodule update`. To fill only some
scions, name them: `git submodule update --init --recursive -- scions/notebook-kit`.

A private scion needs a key that can read it ([Graft a private repository](private-providers.md)). Where
there is none, name only the public scions.

## Set up your machine once

Git can fill scions in new clones and worktrees by itself, and refuse a push that would break builds. With
Home Manager, in `programs.git.extraConfig`:

```nix
{
  push.recurseSubmodules = "check";    # refuse to push a gitlink whose commit is on no remote
  status.submoduleSummary = true;      # git status shows scion moves
  diff.submodule = "log";              # diffs list the provider commits that a scion move takes
  hook.graft-populate = { event = "post-checkout"; command = "${graftPopulate}"; };
}
```

with the hook, for example in a `let`:

```nix
graftPopulate = pkgs.writeShellScript "graft-populate" ''
  # post-checkout: fill the submodules of a new worktree or clone (previous HEAD all zeros).
  # Never fails the checkout: git would report the clone or worktree add as failed.
  case $1 in 0000000000000000000000000000000000000000) ;; *) exit 0 ;; esac
  [ -f .gitmodules ] || exit 0
  [ -z "$(git rev-parse --show-superproject-working-tree)" ] || exit 0   # not for a submodule's own checkout
  git submodule update --init --recursive --quiet ||
    echo "graft-populate: could not fill submodules; run git submodule update --init --recursive" >&2
  exit 0
'';
```

Without Home Manager, the same with `git config --global`:

```
git config --global push.recurseSubmodules check
git config --global status.submoduleSummary true
git config --global diff.submodule log
git config --global hook.graft-populate.event post-checkout
git config --global hook.graft-populate.command ~/.local/bin/graft-populate
```

with the script above saved as `~/.local/bin/graft-populate` and made executable.

- The hook runs on `git clone` and `git worktree add`, beside any repository's own `core.hooksPath` hook,
  in every repository with a `.gitmodules`, graft's or not. When a fetch fails (offline, a private scion
  without its key), it prints a warning and the checkout still succeeds: run `git submodule update --init --recursive`
  later.
- **Do not set `submodule.recurse = true`.** With it, a checkout, switch or pull of the parent that moves a
  scion's gitlink also checks out the scion, and discards uncommitted edits in it without a word: an agent
  fixing a provider in place loses its work. Without it, the scion stays where it is after a pull, and
  `graft check` reports it (`scions/NAME is checked out at X but the repository records Y`); move it on
  purpose with `git submodule update --init --recursive -- scions/NAME`.
- Even without it, commit (or stash) in a scion before you switch the parent's branch, and run
  `git submodule update --init --recursive` after.
- Config-based hooks (`hook.<name>.event`) need git 2.54 or later. On older git, run `nix run .#graft sync`
  after a clone or `worktree add`.
- `push.recurseSubmodules=check` stops you pushing a consumer whose gitlink points at a scion commit that you
  have not pushed to the provider: Nix could not fetch it, and every build would fail. It also refuses a
  gitlink at a commit that only a tag reaches (no branch of the provider contains it), which Nix fetches
  fine; `git push --recurse-submodules=no` then pushes. Providers that tag on their default branch never
  hit this.

## Worktrees

- `git worktree add ../repo-topic -b topic` gives empty scions, which the hook fills.
- `git worktree remove ../repo-topic` refuses a worktree with filled scions. Use
  `git worktree remove --force ../repo-topic`. It deletes uncommitted work in the worktree, the scions'
  included: commit or push what you need first.
- Tools that manage worktrees for you (workmux, for example) call `git worktree remove`: set them to force, or
  remove the worktree yourself.

## Agent sandboxes and cloud environments

Add the fill to the environment's setup script, after the checkout:

```
git submodule update --init --recursive --depth 1
```

Without it, agents see empty `scions/` folders, and the rules in them are missing. pot, graft's sandbox,
runs in your checkout, with the scions that it has: fill them before the run
([Run commands in pot](run-in-a-sandbox.md)). An environment with no network
after setup must fill the scions during setup.

## CI

- **graft's Forgejo workflows** fill the scions where it is needed (a flake in a subdirectory), and leave it
  to Nix for a top-level flake.
- **GitHub Actions:** `actions/checkout` with `submodules: recursive`. For a private scion, also pass an `ssh-key`
  that can read it.
- **Automation that clones or pulls** (a systemd pull timer, a workflow engine's job, a bootstrap script for
  a new machine): clone with `--recurse-submodules`, and run `git submodule update --init --recursive --quiet` after each
  pull or fast-forward. Do not rely on the machine's hook: a service may not read the user's git config, and
  a new machine has none yet. Until the scions are filled, a `path:` flake builds
  `tools` without them, and a git flake build fails once the job modifies a tracked file.

## Check it worked

From the top of a new clone or worktree, paste this. It prints `ok` only if every scion is filled and at
its gitlink:

```sh
test -z "$(git submodule status | grep -v '^ ')" && nix run '.#graft' check && echo ok
```

`git submodule status` starts a line with `-` for an empty scion and `+` for one away from its gitlink.

## Sources

- [lib/graft.sh](../../lib/graft.sh): `sync`.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-check.yaml): the checkout step.
- [tests/run.sh](../../tests/run.sh): `sync` fills scions and not nested ones.
- [tests/fixtures.sh](../../tests/fixtures.sh) `worktrees`: the repositories this page was read against.
- [Nix behaviours](../reference/nix-behaviours.md): a fresh clone builds; `path:` reads the working tree; nested
  scions.
