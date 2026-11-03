# graft

**This is graft v2, a new design. It does not read `graft.yaml`, `graft.lock` or `.graft/`;
those belong to graft v1, kept on the `legacy` tag.**

graft copies other repositories' content into yours at a pinned commit, so agents and people
read exactly the same text on every machine, with nothing to fetch or set up. The provider
decides what it exports; the consumer decides which version it takes.

It is a Nix flake library and two small scripts. Nix does the real work:

| Job | Done by |
|---|---|
| What a repo exports | the provider's `.gitattributes`: paths marked `export-ignore` stay out |
| Fetching and pinning | flake inputs: `flake.lock` records the commit and a content hash |
| Applying the export rules | Nix, for `github:` inputs and for `git+…?exportIgnore=1` inputs |
| One copy of a shared dependency | `follows`, if two inputs share one |
| Finding newer versions | graft: `nix run .#graft-outdated` (newest semver tag upstream) |
| Upgrading | graft: `nix run .#graft-upgrade` (moves the tag, `nix flake update`, re-grafts) |
| Copying into the repo | graft: `nix run .#graft` |
| Checking the copy | graft: `nix flake check` |

## Use it

```nix
# flake.nix
{
  inputs.graft.url = "github:danielnaab/graft/v2";
  inputs.meta-knowledge-base = {
    url = "github:danielnaab/meta-knowledge-base/v1.0.0";   # github: applies export-ignore
    flake = false;
  };
  # a private Forgejo repo: git+ssh, and exportIgnore=1 or its export rules are skipped
  # inputs.notes = { url = "git+ssh://forgejo@platform-vm:2222/daniel/notes?ref=v1&exportIgnore=1"; flake = false; };

  outputs = inputs: inputs.graft.lib.mkGraft {
    inherit inputs;
    deps = [ "meta-knowledge-base" ];   # each becomes scions/<name>/
    # dir = "scions";                   # the default; any relative path
  };
}
```

Then `nix run .#graft` and commit `scions/`. A **scion** is the horticultural word for what's
grafted on: another repository's export, joined to yours while yours keeps its own roots.
`scions/README.md` lists each one's pinned commit. `nix flake check` fails if `scions/` differs
from what the pins produce: an edited scion, a pin moved without re-running graft, or a grafted
file your `.gitignore` drops.

`dir` can be any relative path inside the repo: `.scions` keeps scions out of tools that skip
hidden folders (an Obsidian vault, say), `docs/Grafted` puts them beside your docs. graft owns
that directory and replaces it on every run, so it refuses to touch an existing directory it
didn't write.

## Upgrade

```
nix run .#graft-outdated                 # each scion: pinned tag, newest tag upstream, status
nix run .#graft-upgrade                  # every outdated scion to its newest tag
nix run .#graft-upgrade -- name@v1.2.0   # one scion to a named tag
```

`graft-upgrade` rewrites the tag in `flake.nix`, runs `nix flake update <name>`, and rebuilds
the scions directory; then commit. The diff of `scions/` is the change you are taking. Only
semver tags (`v1.2.3` or `1.2.3`) count as versions; a scion pinned to a branch or a commit is
reported as `untagged` and left alone. By hand: change the ref, `nix flake update <name>`,
`nix run .#graft`.

Both read the repo as data (graft's README table says which inputs are scions; `flake.lock`
says what each is pinned to) and never call the flake's `outputs`, so they also run from graft's
own flake against any checkout, for a bot that watches many repos:

```
nix run github:danielnaab/graft#outdated -- -C path/to/repo [--json]
nix run github:danielnaab/graft#upgrade  -- -C path/to/repo
```

An upgrade rebuilds the directory with the `assemble.sh` of the graft version the repo pins, so
it writes exactly what the repo's own `nix flake check` expects.

## Working on a provider

To try a provider's unreleased change in a repo that grafts it, override the input for one run:

```
nix run .#graft --override-input meta-knowledge-base "git+file://$HOME/src/meta-knowledge-base?exportIgnore=1"
```

This grafts the local checkout, uncommitted edits included, with its `export-ignore` rules applied
(`path:` would skip them). `flake.lock` isn't touched, and the scions README records the commit as
`…-dirty`, so `nix flake check` fails if this is committed by mistake. `nix run .#graft` puts the
pinned version back. When the change is right, tag the provider and `nix run .#graft-upgrade`.

## In CI

[`examples/forgejo/`](examples/forgejo/) has two workflows to copy into `.forgejo/workflows/`:

- **graft-check.yaml**: on PRs and pushes to `main`, the scions match `flake.lock`.
- **graft-upgrade.yaml**: weekly (and on demand), `graft-upgrade`; if anything moved, it checks
  the result, force-pushes the branch `graft/upgrade` and opens a PR from it (or updates the one
  already open). It never pushes to the default branch.

Both want a runner with Nix: `runs-on: nixos-latest` means a docker label for
`docker://nixos/nix`. On GitHub, use the same steps after a Nix install action.

A flake with outputs of its own merges them with `nixpkgs.lib.recursiveUpdate`, not `//`, which
would drop one side's `checks`.

## As a provider

Mark what isn't part of your export in `.gitattributes`, and tag releases:

```
tests/   export-ignore
notes/   export-ignore
scions/  export-ignore   # always: consumers take your content, never your dependencies
```

Your own dependencies stay yours. A consumer that needs one of them grafts it directly, so every
repo has one level of scions and one copy of each dependency.

## Scope

graft stays this small. It has no config file, no lock of its own and no CLI beyond `nix run`.
It provides commands; any bot is a workflow in the repo that runs them (or a central job that runs
them from graft's flake). A feature that can't be expressed as Nix plus a copy, a diff and a tag
comparison does not belong here.

## Develop

`tests/run.sh` runs the whole cycle against a throwaway provider repo: graft, check, drift,
upgrade by hand, `graft-outdated` and `graft-upgrade` (in the repo and from outside), and the
`dir` cases. `nix flake check` runs shellcheck on it and on `lib/*.sh`.
