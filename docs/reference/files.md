---
status: working
---
# Files

The files graft reads and writes, and the lines in them that graft recognizes.

## In a consumer

| File | Read or written | Purpose |
|---|---|---|
| `flake.nix` | Read as data. `upgrade` changes the tags. | Inputs with `flake = false` are scions. The input named `graft` is graft's pin. |
| `flake.lock` | Read as data. Written by Nix, by `upgrade` for `git+` inputs, and by `sync` when it re-pins an annotated tag. | The commit and hash of each pin. Must be committed: `graft check` fails when git ignores it, `checks.<system>.graft` when git does not track it, and `graft ci check` when it is not in `HEAD`. |
| `<dir>/<name>/` | Written by `sync` and `upgrade`. | Each scion: the provider's export. Empty directories are left out, because git cannot hold them. The checks compare each file's contents and each symbolic link's target, not file modes. |
| `<dir>/README.md` | Written by `sync` and `upgrade`. | The marker, and the table of scions and commits. |
| `.forgejo/workflows/graft-*.yaml` | Restored by `sync`, replaced by `upgrade`, compared by the checks. | graft's workflows, if they carry the marker. |

## The scions README

graft writes it. Its format is what graft's commands read, so do not edit it.

```markdown
<!-- managed by graft: https://github.com/danielnaab/graft -->
<!-- graft flake: nix/workspace -->
# scions/
…
| Scion | Commit |
|---|---|
| [knowledge-base-kit](knowledge-base-kit/) | `f95fa544dbd92d7ebba36ef7d4eb6c8bfb8a28f8` |
```

- **Line 1, the marker:** graft writes it on line 1, and looks for it on any line. graft finds a
  repository's scions directories by this line, in any `README.md` that git tracks, except one inside
  another scions directory (a provider's leaked scions README). graft deletes only a directory whose
  `README.md` has it.
- **Line 2, only for a flake in a subdirectory:** the `flakeDir`. graft's commands and workflows find the
  flake from it, by the same rules: only a `README.md` with the marker counts, not one inside another
  scions directory, and every scions directory in a repository must name the same flake.
- **The table:** one row per scion: a link to its folder, and its commit. graft's commands find the
  scions from these rows. A local override shows the commit with `-dirty`.

## In a provider

| File | Purpose |
|---|---|
| `.gitattributes` | Paths marked `export-ignore` stay out of the export. |
| `CHANGELOG.md` | One `## vX.Y.Z` section per release. A major version's section has an `### Upgrading` subsection. graft quotes the sections between two tags. |
| Tags | Semver tags (`v1.2.3` or `1.2.3`) are versions. Other tags are ignored. |

## graft's workflows

A file in `.forgejo/workflows/` is graft's when it has the name of a file in graft's
`template/.forgejo/workflows/` and contains the marker line:

```yaml
# managed by graft: https://github.com/danielnaab/graft (delete this line to keep your own edits)
```

graft renders its own copy with the lines that are yours: `branches:`, `paths-ignore` and `# graft ssh:`
(with the step that it implies). The file must equal that rendering byte for byte.

- `upgrade` replaces the file when graft's pin moves.
- `sync` restores the pinned version's copy.
- The checks fail if the file differs.
- To keep your own edits, delete the marker line. graft then never reads or changes the file.
- graft never adds a workflow that you did not copy in.

### The branches line

One line under the `push:` trigger of `graft-check.yaml`: the branches whose pushes run the check.
graft's copy has `branches: [main]`. If your default branch has another name, write it there:

```yaml
  push:
    branches: [v2]
```

- A list of branch names on one line, with no comment after it. Names can contain letters, digits, `.`,
  `_`, `/` and `-`. graft writes them without quotes.
- graft 2.11.0 or later.

### The paths-ignore line

One line under each of the `pull_request:` and `push:` triggers, the same under both:

```yaml
    paths-ignore: ["notebook/**", "drafts/**"]
```

- A list of quoted globs on one line, indented four spaces, with no comment after it. Globs can contain
  letters, digits, `.`, `_`, `/`, `*`, `?` and `-`. Actions' syntax applies: `**` crosses directories. A run is skipped
  only when every changed file matches a glob.
- A block list, or two lines that differ, fail the check.
- If the line is under one trigger only, `sync` writes it under the other.
- graft refuses a glob that could match a file that graft-check reads: the scions directory,
  `flake.nix`, `flake.lock` (for a flake in a subdirectory, that whole directory) and `.forgejo/`. It
  also refuses a glob that starts with `*` or `/`. graft reads globs as shell patterns, where `*` and
  `?` also match `/`, so it refuses in case of doubt. Name a directory.
- graft 2.8.0 or later.

### The graft ssh line

One comment line, in `graft-check.yaml` only. graft keeps it above `name:`.

```yaml
# graft ssh: host=[platform-vm]:2222 address=192.168.1.51 hostkey=ssh-rsa AAAAB3NzaC1yc2E... secret=GRAFT_SSH_KEY
```

| Field | Required | Meaning |
|---|---|---|
| `host` | yes | The host as the scion's URL names it; `[host]:port` when the port is not 22. |
| `address` | no | The address that the host name resolves to, for a runner that cannot resolve it. |
| `hostkey` | yes | The host's public key, `TYPE KEY`, where `TYPE` is `ssh-ed25519`, `ssh-rsa` or `ecdsa-sha2-nistp256` (or `384`, `521`). Pinned: `StrictHostKeyChecking yes`. For Forgejo's built-in ssh server, `data/ssh/gitea.rsa.pub`. |
| `secret` | no | The Actions secret that holds the private key. Default `GRAFT_SSH_KEY`. |

The step writes the key to `~/.ssh/graft_scions`, the host key to `~/.ssh/known_hosts` (under the host
name and the address), and a `Host` entry to `~/.ssh/config`. On an image with no ssh client, it takes
`openssh` from Nix. graft 2.9.0 or later.

## Sources

- [lib/assemble.sh](../../lib/assemble.sh): the scions README
- [lib/common.sh](../../lib/common.sh): `scion_dirs`, `scions_in`, the flake line
- [lib/workflows.sh](../../lib/workflows.sh): workflow ownership, the paths-ignore and ssh lines
- [lib/tools.sh](../../lib/tools.sh): semver tags
