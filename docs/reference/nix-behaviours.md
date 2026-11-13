---
status: working
---
# Nix behaviours

The Nix behaviours that graft works around, and the Nix versions where they were seen. Check this page
again when you change Nix versions.

| Behaviour | Seen in | What graft or its docs do |
|---|---|---|
| A flake at a repository's root copies every tracked file to the world-readable store. No lazy trees. | 2.24, 2.34 | [Keep private files out of the Nix store](../guides/flake-in-a-subdirectory.md) |
| Nix does not write the lock of a relative `path:` flake (`cannot fetch input … because it uses a relative path`). | 2.24. Not 2.34. | Lock with the absolute path: `nix flake lock "path:$PWD/DIR"`. `upgrade` uses `--flake path:<absolute path>`. |
| A fetch of a `git+ssh` or `git+https` input (or `git+file` to a bare repository) at `?ref=TAG`, without `rev=`, asks for `refs/heads/TAG`, and fails. In 2.24 (`revspec 'TAG' not found`), only until a fetch at `?ref=refs/tags/TAG` or with `rev=` puts the tag in Nix's git cache. In 2.34 (`couldn't find remote ref refs/heads/TAG`), every time. A fetch at `?ref=refs/tags/TAG`, or one with `rev=`, works. So does a lock already in `flake.lock`. | 2.24, 2.34 | [Graft a private repository](../guides/private-providers.md) locks the first tag with `nix flake lock --override-input` and `rev=`. `upgrade` locks a tag's commit itself. |
| Nix fetches a locked `git+` input's commit with a shallow fetch, which a git server on dumb HTTP refuses (`fatal: dumb http transport does not support shallow capabilities`). Forges serve smart HTTP and ssh, which allow it. | 2.34. Not 2.24. | graft's tests serve their HTTP provider with `git http-backend`. |
| `nix flake update NAME --flake REF` needs 2.19 or later. | 2.19 | `upgrade` for a flake in a subdirectory needs 2.19. |
| `?flake=false` in a URL is ignored, and `inputs` is read without evaluation. | 2.24, 2.34 | Write `flake = false` on each input, and `inputs` as a literal. |
| Two inputs for one repository at one commit (`github:` and `git+https`, for example) share one fetch-cache entry. The first fetch decides the tree for both. On a provider's machine, where `.` fetched the commit first, a consumer locked there gets the hash of the whole commit, not of the export, and fails elsewhere with `NAR hash mismatch`. | 2.24, 2.33, 2.34 | Take one repository through one input. Lock consumers with an empty cache ([Publish](../guides/publish.md#if-something-goes-wrong)). |
| A case of the row above: a repository whose export rules exclude its own `flake.nix`, used as `.` with a clean tree at a commit that a consumer fetched through `github:`, fails with `error: path '/nix/store/…-source/flake.nix' does not exist` (`nix flake metadata .`, and any command on `.`). Nix reuses that fetch, keyed by the commit, and GitHub's archive applied the export rules. A `git+…?exportIgnore=1` fetch of the commit did not do this in a test. | 2.24 | Use `path:$PWD` for `.`, which also copies untracked and ignored files into the store; or make an uncommitted change first. |
| [Trailing-slash rules](#trailing-slash-rules) | 2.24 to 2.35 | Providers write directory rules without the slash too. |
| [Annotated tags and exportIgnore](#annotated-tags-and-exportignore) | 2.24. In 2.34, only a lock that 2.24 wrote. | graft keeps such locks at the tag's commit. |

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

Nix 2.34 locks a tag at its commit (`?ref=refs/tags/v1.0.0`, or a `git+file` checkout), and cannot lock
`?ref=v1.0.0` over HTTP or ssh at all (see the table). But it fetches a lock at the tag object, such as
one that Nix 2.24 wrote, as 2.24 does: the export ignores nothing.

A `git+file` checkout hides this: Nix reads the working tree's `.gitattributes`. `github:` inputs are not
affected.

What graft does:

- `graft upgrade` locks a moved `git+` input itself. It reads the tag's commit with `git ls-remote`
  (`v1.0.0^{}`), and writes the lock for `?ref=v1.0.0&rev=<commit>`. This also avoids `nix flake update`,
  which cannot fetch `ref=TAG` over HTTP or ssh: Nix 2.24 into a cache that has not seen the tag, and
  Nix 2.34 at all.
- `nix run .#graft` changes a lock at a tag object to the commit, then stops, because that run was built
  from the old lock. A tag that moved upstream is not changed: that is an upgrade, not a repair.
- `nix flake check`, `graft check` and `graft ci check` fail if a fetched export still holds a path that
  its own `.gitattributes` excludes, and name the path. `nix run .#graft` refuses to write that export.

The lock holds the commit, because every tool reads the lock: Nix's check, graft, and CI, with any Nix
version. Nix 2.34 locks a tag at its commit itself, and keeps the lock that graft wrote.

## Sources

- [CHANGELOG v2.9.0](../../CHANGELOG.md#v290---2026-11-10): annotated tags
- [lib/export-ignored.sh](../../lib/export-ignored.sh), [lib/tools.sh](../../lib/tools.sh) (`peel`)
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `flakeDir`
- [tests/run.sh](../../tests/run.sh): "Annotated tags", a provider over HTTP, which runs on Nix 2.24 and 2.34
- The `flake.nix` that a `github:` fetch left out: seen on a clean checkout of notebook-kit v1.1.1, whose
  `.gitattributes` has `/flake.nix export-ignore`; the store path that `.` used had the `narHash` of a
  consumer's `github:` lock of that commit.
