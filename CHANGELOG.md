# Changelog

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
