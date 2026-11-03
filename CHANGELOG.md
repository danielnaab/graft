# Changelog

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

A new design, sharing no history with graft v1 (tag `legacy`): pinned, provider-defined exports
via Nix flake inputs, copied by `nix run .#graft` and checked by `nix flake check`.
