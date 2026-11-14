---
status: working
---
# Files

The files graft reads and writes, and the lines in them that graft recognizes.

## In a consumer

| File | Read or written | Purpose |
|---|---|---|
| `flake.nix` | Read as data. `upgrade` changes graft's tag. | The input named `graft` is graft's pin. A top-level flake declares `inputs.self.submodules = true;`. Names no provider. |
| `flake.lock` | Written by Nix, and by `upgrade` for graft's input. | graft's commit and hash, and nixpkgs if you have one. No provider entries. Must be committed: `graft ci check` fails when it is not in `HEAD`. |
| `.gitmodules` | Written by `git submodule add` (through `graft add`) and `git rm`. Read by every command. | Each scion's path and URL. `upgrade` never changes it. |
| The gitlinks | Written by `add` and `upgrade`, or by `git add scions/NAME`. | The pins. Each must be a gitlink (mode 160000 in `git ls-files -s`), never the scion's files: `check` fails on a registered path tracked as files, and `add` refuses to graft over one. |
| `<dir>/<name>/` | A submodule. Filled by `git submodule update --init --recursive`, `graft sync`, a recursive clone, or the post-checkout hook. | The provider's whole repository at a release tag. |
| `<dir>/README.md` | Written by `add` and `sync`; `check` looks for its marker line. | The marker README. |
| `.graft-try` | Written by `try`, at the repository's top, and marked with intent to add; removed by `try --stop`. `check` fails while it exists. Never committed. | Makes every Nix build of a git flake read the working tree. Two `#` comment lines. See [try](commands.md#try). |
| `.forgejo/workflows/graft-*.yaml` | Restored by `sync`, replaced by `upgrade`, compared by `check`. | graft's workflows, if they carry the marker. |
| Settings files, such as `knowledge-base.yaml` | Read by providers' tools and checks, never by graft. | What a provider lets you configure. Each provider's README names its file. |

Inside a [sandbox](sandbox.md#protected-paths), `flake.nix`, `flake.lock`, `.gitmodules`, each `graft.nix`,
`.envrc`, `justfile` and `.pre-commit-config.yaml` (at any depth), and the git metadata, are read-only.

## The marker README

`<dir>/README.md`. `add` writes it when it is missing; `sync`, and `upgrade` when graft's pin moves, write
it again with this graft version's text (`graft: wrote scions/README.md`). For a flake at the repository's
top:

```markdown
<!-- managed by graft: https://github.com/danielnaab/graft -->
# Scions

Each folder here is a git submodule: another repository, grafted at a release tag. Empty folders?
Run `git submodule update --init --recursive` (or `nix run .#graft sync`).

- Read them freely. Ask before you edit one: a fix belongs in the provider. See graft's "Fix a provider from
  a consumer" guide: edit on a branch, test, build with it (`nix run .#graft -- try NAME`, then `nix run .#graft -- try --stop`),
  push to the provider, release, then `nix run .#graft upgrade NAME`.
- `nix run .#graft check` fails until every scion is clean and at a release tag, and while try is on.
- Versions: `nix run .#graft outdated`.
```

graft's text is `lib/scions-readme.md`, with `@RUN@` replaced by the flake's run command. For a flake in a
subdirectory, that is `nix run 'path:nix/agent-tools#graft'`. `check` reads only the marker line.

- **The marker line** is how graft's commands and workflows find the scions directory, and the flake (its
  parent directory). Exactly one per repository.
- **Other tools skip the directory by it.** knowledge-base-kit's checkers (`kb_linter`, `link_validator`)
  do not walk a directory whose `README.md` has the marker, so they do not check providers' tests and
  fixtures.
- v2.12.0 and earlier wrote a table of scions and a second line, `<!-- graft flake: DIR -->`. Both are gone;
  `sync` rewrites the README.

## `just` imports from a scion

A justfile that imports a provider's recipes uses `import?` (with `?`), so a fresh clone with empty scions
still parses: `import? "scions/notebook-kit/tools/kit.just"`. That holds only while no recipe of your own
depends on an imported one. A recipe `check: kit-check` makes `just` fail to parse when the import is missing
(`error: Recipe 'check' has unknown dependency 'kit-check'`), whatever recipe you ask for. Call the imported
recipe from the body instead, which `just` resolves only when it runs:

```just
import? "scions/notebook-kit/tools/kit.just"

check:
    just kit-check
```

## Nested scions: the One-level cost

A provider's own submodules are filled recursively by `add`, `sync` and `upgrade` (git leaves them empty
otherwise, and Nix fails on an empty one while a tracked file is modified), and Nix fetches them on every
evaluation of a git flake. So each level costs every consumer a fetch, and a private or unpushed nested
submodule breaks consumers who cannot read it: a consumer of notebook-kit fetches knowledge-base-kit, then
living-specifications, then meta-knowledge-base. Keep providers to one level
([One level](../decisions.md#one-level)).

## In a provider

| File | Purpose |
|---|---|
| Tags | Semver tags (`v1.2.3` or `1.2.3`) are versions. Other tags, such as `v1.3.0-rc.1`, are ignored. A tag may be lightweight, annotated or signed. Never move a released tag: consumers' gitlinks point at its commit. |
| `CHANGELOG.md` | One `## vX.Y.Z` (or `## X.Y.Z`) section per release; anything can follow the version, such as the date. A major version's section has an `### Upgrading` subsection (graft also takes `### Migration`). graft reads it from the provider's repository at the target tag. |
| `graft.nix` | Optional. What the provider contributes to a consumer's flake ([below](#graftnix)). |
| `README.md` | Say what to point agents at, which tools the provider ships, and which settings file they read. |
| `.gitattributes` | No longer read by graft. A consumer gets the whole repository. `export-ignore` lines still apply to `git archive`, if you use it. |

A consumer gets every committed file, so link only to files that exist in the repository, and keep a
provider's own agent instructions (`AGENTS.md`, `CLAUDE.md`) for work on the provider: a consumer's agent
reads the files that its own `AGENTS.md` points at, not the scion's.

### graft.nix

A function, called once per system by the consumer's mkFlake:

```nix
{ pkgs, lib, system, scion, scions, consumer, ... }:
{
  requires = { knowledge-base-kit = "https://github.com/danielnaab/knowledge-base-kit"; };
  tools    = { nb-index = pkgs.writeShellApplication { … }; };
  apps     = { check-kb = { type = "app"; program = "…"; }; };
  checks   = { tasks = pkgs.runCommand "tasks" { } "…"; };
  packages = { obsidian-plugins = …; };
  sandbox  = { packages = [ pkgs.ripgrep ]; network.allow = [ "api.example.com" ]; };
}
```

**Arguments.** Always passed. Take `...`, so that a later graft version can add one.

| Argument | Value |
|---|---|
| `pkgs`, `lib`, `system` | The consumer's `inputs.nixpkgs` if it has one, else graft's pinned nixpkgs. |
| `scion` | This scion, copied alone into the store, without its `.git` file. Relative paths (`./bin/tool`) resolve in it too. |
| `scions` | Every scion of the consumer. `scions.NAME` is the scion's store path (`"${scions.NAME}/src"` works) merged with its `graft.nix` result (`scions.NAME.tools.TOOL`). |
| `consumer` | The consumer flake's source: the repository for a top-level flake, only the flake's directory for a `path:` flake. Checks read the consumer's settings from it, such as `${consumer}/knowledge-base.yaml`, and skip, not fail, when the file is absent. |

**Returned attributes.** All optional.

| Attribute | Becomes | Collision |
|---|---|---|
| `tools` | Part of `packages.<system>.tools`, on the consumer's `PATH` | An error, unless both are the same store path. |
| `apps` | `apps.<system>.<name>` | An error; `graft` and `sandbox` are reserved. |
| `packages` | `packages.<system>.<name>` | An error; `tools` and `sandbox` are reserved. |
| `checks` | `checks.<system>.<scion>-<name>` | None: prefixed. |
| `sandbox` | Part of the [sandbox policy](sandbox.md#options). `packages`, `env.set` and `protect` apply. `network.allow`, `paths.readOnly`, `paths.writable` and `env.pass` are requests: they apply only when the consumer grants them. | None: merged. `repository.readOnly`, `grant`, and `mkForce` (or `mkOverride` below 100) are an error. |
| `requires` | Nothing. Checked first: each named scion must exist, or evaluation fails with `graft add URL`. | |
| Anything else | Visible to other providers as `scions.NAME.ATTR`, not a flake output. | |

- `requires` names other providers by their folder name, which is the URL's last segment, with the URL that
  the error tells the consumer to `graft add`. It has no version. A provider that needs a newer release of a
  sibling (one that first ships a tool it uses) releases a new major version of its own, and says so in its
  Upgrading steps. A consumer that renames a folder breaks `requires`, by design.
- Providers ship no dev shells. A consumer's `devShells.default` has every tool on `PATH`.
- Never read the provider's own scions from `graft.nix`: a git flake consumer has them, a `path:` flake
  consumer has them empty.
- A provider's own `flake.nix` and `flake.lock` are for work on the provider. Consumers never evaluate them.
  To pin something other than the consumer's nixpkgs, fetch it in `graft.nix` with a fixed hash
  ([Write a provider's graft.nix](../guides/write-graft-nix.md#pin-your-own-inputs)).

### Naming

A provider's name ends with what the content is, not the role that the repository plays:

| Suffix | Content | A consumer should |
|---|---|---|
| `-kit` | Something to adopt and keep upgrading: tools, skills, structure, often with the rules that they enforce | run or adopt it |
| `-conventions` | Rules only | comply with it |
| `-knowledge` | A knowledge base about a topic | read it |

- A repository that defines a named method takes that name, such as `living-specifications`.
- Do not use role words (`-provider`, `-scion`), words that other tools own (`-pack`, `-bundle`,
  `-template`), or words that suggest a one-time copy (`-seed`, `-starter`).
- Give the repository the topic `graft-provider` on its forge (Forgejo or GitHub).
- Tool names share one `PATH` across every provider a consumer grafts: start them with something of yours
  (`kb-lint`, not `lint`). App names share one namespace too. Check names are prefixed for you.

## graft's workflows

A file in `.forgejo/workflows/` is graft's when it has the name of a file in graft's
`template/.forgejo/workflows/` and contains the marker line:

```yaml
# managed by graft: https://github.com/danielnaab/graft (delete this line to keep your own edits)
```

graft renders its own copy with the lines that are yours: `branches:`, `paths-ignore` and `# graft ssh:`
(with the steps that it implies). The file must equal that rendering byte for byte.

- `upgrade` replaces the file when graft's pin moves.
- `sync` restores the pinned version's copy.
- `check` fails if the file differs.
- To keep your own edits, delete the marker line. graft then never reads or changes the file
  ([Keep your own edits to a workflow](#keep-your-own-edits-to-a-workflow)).
- graft never adds a workflow that you did not copy in.

The workflows find the flake from the marker README: its directory's parent; two marker READMEs fail with
`::error::two scions directories: … graft supports one per repository`. A flake in a subdirectory runs as
`path:$PWD/DIR`, and the check and upgrade jobs first fill its scions (`git submodule update -q --init -- DIR`),
because a `path:` flake reads the working tree. A top-level git flake does not need that: Nix fetches the
scions itself. graft-check's step is named `Scions registered and at release tags`.

### Keep your own edits to a workflow

1. Delete the marker line. Do this before you run `nix run .#graft sync`: while the marker is there, sync puts
   graft's copy back.
2. Make your edits. The comment below the old marker still says that graft replaces the file: change it
   too.
3. Run `nix run .#graft check`. It no longer reads the file.
4. Commit the workflow.

Without the marker, `upgrade` no longer replaces the file, so you get none of graft's later changes to it.
To take graft's copy again, put the marker line back as the first line and run `nix run .#graft sync`. It
prints `graft: refreshed .forgejo/workflows/FILE`, keeps the `branches:`, `paths-ignore:` and `# graft ssh:`
lines, and drops your other edits. graft does not stage the workflows, so `git restore` gets back an edit that
git had.

### The branches line

One line under the `push:` trigger of `graft-check.yaml`: the branches whose pushes run the check. graft's
copy has `branches: [main]`. If your default branch has another name, write it there:

```yaml
  push:
    branches: [v2]
```

A list of branch names on one line, with no comment after it. Names can contain letters, digits, `.`, `_`,
`/` and `-`.

### The paths-ignore line

One line under each of the `pull_request:` and `push:` triggers of `graft-check.yaml`, the same under both.
It skips graft-check for pushes that change only files that graft's check never reads, such as a notes
directory that a bot pushes to often:

```yaml
on:
  pull_request:
    paths-ignore: ["notebook/**", "drafts/**"]
  push:
    branches: [main]
    paths-ignore: ["notebook/**", "drafts/**"]
```

- A list of quoted globs on one line, indented four spaces, with no comment after it. Globs can contain
  letters, digits, `.`, `_`, `/`, `*`, `?` and `-`. `**` crosses directories. A run is skipped only when
  every changed file matches a glob.
- A block list, or two lines that differ, fail the check. If the line is under one trigger only, `sync`
  writes it under the other.
- graft refuses a glob that could match a file that graft-check reads: the scions directory, `.gitmodules`,
  `flake.nix`, `flake.lock` (for a flake in a subdirectory, that whole directory) and `.forgejo/`. It also
  refuses a glob that starts with `*` or `/`. Name a directory.
- A pull request that changes only ignored files gets no graft-check status. Do not make that status
  required in branch protection.

### The graft ssh line

One comment line, for a private scion that only an ssh key can read
([Graft a private repository](../guides/private-providers.md)), in either workflow file. graft keeps it above
`name:`, and renders a step that installs the key before each step that runs graft: graft-check's check, and
graft-upgrade's upgrade and pr steps. In a file whose copy has no such step, the check fails with
`graft: X: its "# graft ssh:" line is kept only in a workflow with a step that runs graft, and this graft version's copy has none; delete the line`.

```yaml
# graft ssh: host=[git.example.com]:2222 address=192.168.1.51 hostkey=ssh-rsa AAAAB3NzaC1yc2E... secret=GRAFT_SSH_KEY
```

| Field | Required | Meaning |
|---|---|---|
| `host` | yes | The host as the scion's URL names it; `[host]:port` when the port is not 22. |
| `address` | no | The address that the host name resolves to, for a runner that cannot resolve it. |
| `hostkey` | yes | The host's public key, `TYPE KEY`, where `TYPE` is `ssh-ed25519`, `ssh-rsa` or `ecdsa-sha2-nistp256` (or `384`, `521`). Pinned: `StrictHostKeyChecking yes`. For Forgejo's built-in ssh server, `data/ssh/gitea.rsa.pub`. |
| `secret` | no | The Actions secret that holds the private key. Default `GRAFT_SSH_KEY`. |

The step writes the key to `~/.ssh/graft_scions`, the host key to `~/.ssh/known_hosts` (under the host name
and the address), and a `Host` entry to `~/.ssh/config`, which both git and Nix's git fetcher read. On an
image with no ssh client, it takes `openssh` from Nix.

## Sources

- [lib/common.sh](../../lib/common.sh): finding the marker README; semver tags
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the `graft.nix` arguments, merging and collisions, `sandbox`
- [lib/workflows.sh](../../lib/workflows.sh): workflow ownership, the branches, paths-ignore and ssh lines
- [lib/tools.sh](../../lib/tools.sh): `add`, `upgrade` and `.gitmodules`
- [lib/common.sh](../../lib/common.sh) `upstream_range`, `changelog_between`: `CHANGELOG.md` read from the
  provider, and its headings
