# Changelog

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

### Changed
- `graft-upgrade.yaml` runs as two jobs. `upgrade` (upgrade, check, push `graft/upgrade`) holds
  only the job token, since moving graft runs the new version's code before review. `pr` opens
  the PR with `GRAFT_PR_TOKEN` (falling back to the job token, as before) from a fresh checkout
  of the default branch, never evaluating the upgraded flake. It opens nothing if the branch is
  already in the default branch. The token reaches `curl` on a pipe, not in its command line.
- The PR and commit are titled `graft: upgrade` (was `graft: upgrade scions`).
- `graft-outdated`'s first column is `NAME` (was `SCION`), and `--json` has a row for graft
  with `dir` `-`.

Upgrading from 2.2.x (once per repo, by hand: 2.2 can't move its own pin):

```
nix run github:danielnaab/graft/v2.3.0#upgrade -- graft@v2.3.0
src=$(nix flake prefetch --json github:danielnaab/graft/v2.3.0 | jq -r .storePath)
cp --no-preserve=mode "$src"/examples/forgejo/graft-*.yaml .forgejo/workflows/
nix flake check
```

then commit `flake.nix`, `flake.lock` and `.forgejo/workflows/`. The scions directory doesn't
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
