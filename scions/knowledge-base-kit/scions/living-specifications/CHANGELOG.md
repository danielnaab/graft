# Changelog

All notable changes to this project will be documented in this file.

## [1.0.2] - 2026-11-11

### Changed

- README, "Using This in Your Repository": pin `v1.0.2`, and linking by URL as the alternative to grafting.
- README, "Architecture": shows the one design edge (knowledge-base-integrity implements the `# spec:`
  annotation format) and says the graft of meta-knowledge-base is for this repo's upkeep only.
- `knowledge-base.yaml` `version` matches the release (was `0.1.0`).
- `.forgejo/` (this repo's CI) is no longer exported; it had been since graft CI was added after 1.0.1.
- `docs/` is unchanged. Checked against the pre-extraction copy in delivery-practices
  (`docs/living-specifications/`): this repo's text is newer in every file that differs, so nothing came across.

## [1.0.1] - 2026-11-03

### Changed

- README, "Using This in Your Repository": the grafted copy is `scions/living-specifications/`
  (was `vendor/living-specifications/`), graft 2.1's default directory.
- This repo's own graft setup: graft `v2.1.0` (was `v2.0.0`), meta-knowledge-base grafted into
  `scions/` (was `vendor/`), and AGENTS.md, `.gitattributes` and the Claude Code edit deny rule
  point at `scions/`.
- `docs/` is unchanged.

## [1.0.0] - 2026-11-02

### Changed

- Exports for graft (https://github.com/danielnaab/graft): `.gitattributes` defines what repos
  that graft this get: README, CHANGELOG and `docs/`. `AGENTS.md`, `knowledge-base.yaml` and
  `scions/` are this repo's own.
- meta-knowledge-base is grafted at `v1.0.0` into `scions/`; AGENTS.md links into that copy.

### Upgrading

`docs/` is unchanged from 0.1.0; what changed is how a repo takes it. 0.1.0's README said to
import it with graft v0 (`graft.yaml`, `living-specifications: ...living-specifications.git#main`)
and reference `.graft/living-specifications/docs/`. From 1.0.0 it is a graft v2 flake input,
pinned to a tag. In a repo that imported it the old way:

1. Remove the `living-specifications` entry from `graft.yaml` (and `graft.yaml` itself, with
   `.graft/`, if nothing else is in it).
2. Add the input to the repo's graft `flake.nix` (see graft's README for `mkGraft`), and list
   `"living-specifications"` in its `deps`:
   `inputs.living-specifications = { url = "github:danielnaab/living-specifications/v1.0.1"; flake = false; };`
3. `nix run .#graft`, then commit `flake.nix`, `flake.lock` and `scions/`.
4. Replace `.graft/living-specifications/` with `scions/living-specifications/` in links (paths
   below `docs/` are the same). Drop links to this repo's `knowledge-base.yaml`, which is no
   longer exported.

Done when `grep -rn '\.graft/living-specifications' .` finds nothing and `nix flake check` passes.

## [0.1.0] - 2026-01-31

### Added

- Initial extraction from delivery-practices repository
- Core methodology documentation:
  - `docs/format.md` - Specification template structure
  - `docs/principles.md` - 16 governing principles for spec-driven development
  - `docs/relationships.md` - How specs relate to tests and code
  - `docs/verification.md` - Drift detection approaches
  - `docs/comprehension.md` - Making the system legible
- Guides for spec authoring:
  - `docs/guides/writing-specs.md` - How to write specifications
  - `docs/guides/ensemble-with-specs.md` - Mob programming with specs
  - `docs/guides/reviewing-against-specs.md` - PR review patterns
- Repository configuration:
  - `knowledge-base.yaml` - KB configuration importing meta-knowledge-base
  - `AGENTS.md` - Agent entrypoint
