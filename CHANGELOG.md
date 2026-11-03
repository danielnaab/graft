# Changelog

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
