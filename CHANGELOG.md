# Changelog

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

A new design, sharing no history with graft v1 (tag `legacy`): pinned, provider-defined exports
via Nix flake inputs, copied by `nix run .#graft` and checked by `nix flake check`.
