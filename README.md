# graft

**This is graft v2, a new design. It does not read `graft.yaml`, `graft.lock` or `.graft/`;
those belong to graft v1, on the `v1` branch.** Earlier lines of work are read-only branches too:
`v1` (graft v1, Python then Rust, 2025-12 to 2026-10), `v0.2` (a Python experiment, 2025-11)
and `v0.1` (a DVC experiment, 2025-11).

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
| Finding newer versions | graft: `nix run .#graft-outdated` (newest semver tag upstream, and any new major) |
| Upgrading, graft itself too | graft: `nix run .#graft-upgrade` (moves the tag, `nix flake update`, re-grafts) |
| Copying into the repo | graft: `nix run .#graft` |
| Checking the copy | graft: `nix flake check` |

## Use it

```nix
# flake.nix
{
  inputs.graft.url = "github:danielnaab/graft/v2.4.0";        # a tag: graft-upgrade moves it
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
nix run .#graft-outdated                 # graft and each scion: pinned, newest in its major, status, newer major
nix run .#graft-upgrade                  # graft and every scion that's outdated, to the newest tag of its major
nix run .#graft-upgrade -- name@v1.2.0   # one scion to a named tag (the only way across a major)
nix run .#graft-upgrade -- graft@v2.4.0  # graft itself to a named tag
```

`graft-upgrade` rewrites the tag in `flake.nix`, runs `nix flake update <name>`, and rebuilds
the scions directory; then commit. The diff of `scions/` is the change you are taking. Only
semver tags (`v1.2.3` or `1.2.3`) count as versions; an input pinned to a branch, a commit or a
path is reported as `untagged` and left alone. By hand: change the ref, `nix flake update <name>`,
`nix run .#graft`.

Nothing crosses a major version by itself. `LATEST` is the newest tag of the major you pin (for
`0.y.z`, of `0.y`: semver's convention that anything may change before 1.0; unlike Cargo, `0.0.z`
is one series too), and
a later major shows in the `MAJOR` column (`major` in `--json`, `null` when there's none). The
no-argument upgrade, and `name` without a tag, stay within the pinned major and say what they left;
`name@v2.0.0` crosses, once you've read what it breaks (below).

graft's own pin (the input named `graft`) is listed and moved the same way. It moves first, and
everything after it uses the new version: every scions directory is rebuilt by its `assemble.sh`,
and graft's workflows (below) are replaced with the ones it ships. Pin graft to a tag for this;
`github:danielnaab/graft/v2` follows the branch and is never moved. graft stays within its major
version unless you name one (`graft@v3.0.0`): a new major may change `mkGraft`.

Both read the repo as data (graft's README table says which inputs are scions; `flake.lock`
says what each is pinned to) and never call the flake's `outputs`, so they also run from graft's
own flake against any checkout, for a bot that watches many repos:

```
nix run github:danielnaab/graft#outdated -- -C path/to/repo [--json]
nix run github:danielnaab/graft#upgrade  -- -C path/to/repo
```

An upgrade rebuilds the directory with the `assemble.sh` of the graft version the repo pins (after
moving it, if graft is being upgraded), so it writes exactly what the repo's own `nix flake check`
expects.

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

- **graft-check.yaml**: on PRs and pushes to `main`, the scions (and graft's workflows) match
  `flake.lock`.
- **graft-upgrade.yaml**: weekly (and on demand), `graft-upgrade`; if anything moved, it runs the
  repo's whole `nix flake check` on the result (graft's check and the repo's own: link checks into
  the scions, lint, tests), force-pushes the branch `graft/upgrade` and opens a PR from it (or
  retitles the one already open, which the push updated). The PR says what moved, what changed
  upstream (each provider's CHANGELOG.md sections between the two tags, or its commits when it
  has none) and which new majors were left. If the check fails the branch is pushed anyway, the PR
  is titled `[checks failing] graft: upgrade` with the failing checks and their logs' tails, and
  the run fails: the breakage is there to read and fix. It never pushes to the default branch. Forgejo's job
  token can push but can't open a PR on a private repo, so the PR is opened with the Actions
  secret `GRAFT_PR_TOKEN`: a token with `write:repository` for a user who can write to the repo
  (a bot account is best: it becomes the PR's author). A user-level secret covers all that
  user's repos.

Their steps are `nix run .#graft-ci -- check|upgrade|pr` ([`lib/ci.sh`](lib/ci.sh)), so they run
the graft version the repo pins and change with it. The files themselves start with a marker line,
`# managed by graft: …`. A file in `.forgejo/workflows/` with the name of one in
`examples/forgejo/` and that line is graft's: `graft-upgrade` replaces it when it moves graft,
`nix run .#graft` puts back the pinned version's copy, and `nix flake check` fails if it differs.
Delete the line to keep your own edits; graft then never touches the file. graft never adds a
workflow you didn't copy in.

Moving graft runs the new version's code (its `assemble.sh` and its check) before anyone has
reviewed it, so the upgrade runs in two jobs. The first upgrades, checks and pushes, and holds
nothing but the job token; protect the default branch so that token can't push to it. The second,
checked out at the default branch, runs the graft that branch pins and never evaluates the
upgraded flake, and opens the PR with `GRAFT_PR_TOKEN`. Opening a PR runs the repo's
`pull_request` workflows from the branch, with the repo's secrets, so it opens one only when the
branch changes nothing but `flake.nix`, `flake.lock` and the scions directories. The second job
also writes the PR body: it reads the two `flake.lock`s as data, and for each input that moved
fetches the provider with plain `git` (no checkout, nothing run) from the upstream the *default
branch's* lock names, at the commit the branch's lock pins, without `GRAFT_PR_TOKEN` in git's
environment. The upstream text goes into the body through `jq`, never a shell command line; it
is the provider's text, quoted, so read it as theirs.

This split assumes each job gets a fresh environment (a container per job, as `docker://nixos/nix`
gives): the `upgrade` job runs unreviewed code without Nix's sandbox (graft's new version, and now
the repo's own checks over new upstream content), so on a runner that shares `$HOME` or a writable
`/nix/store` between jobs it could leave something for the `pr` job to run. A failing check's log
tail goes into the commit message and the PR body, which are as public as the repo; checks that
print secrets would publish them there. When a new graft
version changes its workflows (rare: their logic is in `graft-ci`), or anything else changed, the
job fails with a link to the diff: read it, then open the PR yourself. The limit: `flake.nix` and
`flake.lock` are code (a moved graft pin is the new version), so a `pull_request` workflow of your
own that evaluates or builds the flake with secrets would run it before review. graft's own
workflows don't; keep yours that way, or don't let graft move itself in CI.

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

## Breaking changes and migrations

graft's convention for a provider that breaks its consumers:

- **A new major version is the declaration.** Anything a consumer may rely on (a path, a heading
  linked to, a file's shape) changing incompatibly means a new major tag. graft never crosses one
  by itself, so the weekly PR keeps taking fixes and additions while the break waits for a person.
- **CHANGELOG.md says what to do.** Each release gets a `## vX.Y.Z` section (graft quotes those
  between the pinned and new tags into the upgrade PR). A major's section has an **Upgrading**
  subsection written as steps someone who knows only the consumer repo can apply, an agent
  included: what moved where, what to search for, what to replace it with, and how to tell it's done
  (`nix flake check` passing, say).
- **Renames can leave a redirect.** For one major, a moved file may keep a stub at its old path
  that names the new one, so links into the scion still land somewhere useful. Drop it at the next
  major.

Applying those steps for the consumer (an agent that reads the **Upgrading** section, edits the
repo and opens the PR) is a steward's job, not graft's: graft reports the new major and quotes
the changelog, and that's where it stops.

## Scope

graft stays this small. It has no config file, no lock of its own and no CLI beyond `nix run`.
It provides commands; any bot is a workflow in the repo that runs them (or a central job that runs
them from graft's flake). A feature that can't be expressed as Nix plus a copy, a diff and a tag
comparison does not belong here.

## Develop

`tests/run.sh` runs the whole cycle against a throwaway provider repo: graft, check, drift,
upgrade by hand, `graft-outdated` and `graft-upgrade` (in the repo and from outside), majors
(reported, crossed only when named, 0.x), graft upgrading its own pin (from a throwaway copy of
this tree, tagged twice) and its workflows, `graft-ci` against a local origin (the full-check gate
with a failing check of the consumer's own) and a stub API (the PR body: changelog sections, the
commit-list fallback, an open PR retitled), and the `dir` cases. `nix flake check` runs shellcheck on it and on `lib/*.sh`.
