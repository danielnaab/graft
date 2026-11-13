---
status: working
---
# Nix behaviours

The Nix behaviours that graft works around, and the Nix versions where they were seen. Check this page
again when you change Nix versions.

| Behaviour | Seen in | What graft or its docs do |
|---|---|---|
| A flake at a repository's root copies every tracked file to the world-readable store. No lazy trees. | 2.24 | [A flake in a subdirectory](../guides/flake-in-a-subdirectory.md) |
| Nix does not write the lock of a relative `path:` flake (`cannot fetch input … because it uses a relative path`). | 2.24 | Lock with the absolute path: `nix flake lock "path:$PWD/DIR"`. `upgrade` uses `--flake path:<absolute path>`. |
| `nix flake update NAME --flake REF` needs 2.19 or later. | 2.19 | `upgrade` for a flake in a subdirectory needs 2.19. |
| `?flake=false` in a URL is ignored, and `inputs` is read without evaluation. | 2.24 | Write `flake = false` on each input, and `inputs` as a literal. |
| Two inputs for one repository at one commit (`github:` and `git+https`, for example) share one fetch-cache entry. The first fetch decides the tree for both. | 2.24, 2.33 | Take one repository through one input. |
| [Trailing-slash rules](#trailing-slash-rules) | 2.24 to 2.35 | Providers write directory rules without the slash too. |
| [Annotated tags and exportIgnore](#annotated-tags-and-exportignore) | 2.24 | graft keeps such locks at the tag's commit. |
| A first lock of `git+…?ref=v1.0.0` from a bare clone, with a cache that has not seen the tag, asks for `refs/heads/v1.0.0` and fails: `revspec 'v1.0.0' not found`. | 2.24 | Fetch the tag once by its full name: `nix flake prefetch "git+URL?ref=refs/tags/v1.0.0&exportIgnore=1"`. `graft upgrade` locks with the commit, which avoids it. |

## Trailing-slash rules

For a `git+…?exportIgnore=1` input fetched from a bare clone (`git+https`, `git+ssh`, or `git+file` to a
bare repository), Nix applies `.gitattributes` where it cannot tell a directory from a file. A rule such as
`tests/ export-ignore` then matches nothing. `tests export-ignore` works, and `/tests export-ignore`
matches the top-level directory only.

`git+file` to a checkout with a working tree applies both forms, so a provider that tries its export that
way does not see the problem.

`github:` inputs come from GitHub's `git archive`, which applies both forms.

## Annotated tags and exportIgnore

Nix 2.24 locks `?ref=v1.0.0` at an annotated tag (`git tag -a`) to the tag object's hash, not the
commit's. Its export filter then looks for `.gitattributes` at an object that is not a commit, finds
none, and ignores nothing. The scion gets the provider's whole tree, and the drift check agrees with it,
because the check's expected tree comes from the same fetch.

A `git+file` checkout hides this: Nix reads the working tree's `.gitattributes`. `github:` inputs are not
affected.

What graft does:

- `graft upgrade` locks a moved `git+` input itself. It reads the tag's commit with `git ls-remote`
  (`v1.0.0^{}`), and writes the lock for `?ref=v1.0.0&rev=<commit>`. This also avoids `nix flake update`,
  which in 2.24 cannot fetch `ref=TAG` over HTTP or ssh into a cache that has not seen the tag.
- `nix run .#graft` changes a lock at a tag object to the commit, then stops, because that run was built
  from the old lock. A tag that moved upstream is not changed: that is an upgrade, not a repair.
- `nix flake check`, `graft check` and `graft ci check` fail if a fetched export still holds a path that
  its own `.gitattributes` excludes, and name the path. `nix run .#graft` refuses to write that export.

The lock holds the commit, because every tool reads the lock: Nix's check, graft, and CI, with any Nix
version. Newer Nix locks a tag at its commit itself, and keeps the lock that graft wrote.

## Sources

- [CHANGELOG v2.9.0](../../CHANGELOG.md#v290---2026-11-10): annotated tags
- [lib/export-ignored.sh](../../lib/export-ignored.sh), [lib/tools.sh](../../lib/tools.sh) (`peel`)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `flakeDir`
