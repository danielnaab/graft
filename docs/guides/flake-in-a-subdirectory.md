---
status: working
---
# Keep private files out of the Nix store

Put the repository's flake, and its scions, in a subdirectory, and use it as a `path:` flake, so that Nix
copies only that directory, and never your private files, into the Nix store.

## Before you begin

- Why: Nix copies a flake's source into the Nix store, and every user of the machine can read the store. For
  a flake at the repository's top, the source is every file that git tracks, with uncommitted edits, and
  every `nix run .#graft`, `nix build` and `nix flake check` copies them again. A `path:` flake in a
  subdirectory copies only that directory.
- What changes:
  - The scions live **under** the flake's directory, such as `nix/agent-tools/.scions/`: a `path:` flake
    cannot see anything outside its directory.
  - A `path:` flake reads the working tree, so scions must be filled. In a fresh clone, `tools` builds
    without them, with a warning, until you run `git submodule update --init --recursive`.
  - Your flake's checks cannot read files outside the directory: a provider's check of your pages skips
    ([Make a pattern's checks gate your merges](gate-merges.md#a-flake-in-a-subdirectory)).
- The steps use `nix/agent-tools` for the subdirectory and `.scions` for the scions directory. Use your own
  names. Keep no other files in the subdirectory: Nix copies all of them.
- The commands quote `'path:nix/agent-tools#graft'`, as [The graft command](../reference/commands.md#the-graft-command)
  says.

## Steps

Run every command from the repository's top. Never run `nix run .#graft`, `nix flake check`, `nix develop`
or direnv's `use flake` inside `nix/agent-tools/`, and never use a `git+file:` reference to the repository:
those are git flakes, which copy the whole repository.

1. Make the subdirectory, and write a flake in it:

   ```
   mkdir -p nix/agent-tools
   ```

   `nix/agent-tools/flake.nix`:

   ```nix
   {
     description = "This repository's agent tools";
     inputs.graft.url = "github:danielnaab/graft/v2.13.1";
     outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; dir = ".scions"; };
   }
   ```

   No `inputs.self.submodules`: a `path:` flake copies the working tree, scions included. If the repository
   had a flake at the top, move it (`git mv flake.nix flake.lock nix/agent-tools/`) and edit it the same way.

2. Lock it, by its absolute path:

   ```
   git add nix/agent-tools/flake.nix
   nix flake lock "path:$PWD/nix/agent-tools"
   ```

   It prints `warning: creating lock file '…/nix/agent-tools/flake.lock'`.

3. Graft a provider into the subdirectory's scions directory:

   ```
   nix run 'path:nix/agent-tools#graft' -- add --scions nix/agent-tools/.scions https://github.com/danielnaab/knowledge-base-kit
   ```

   ```
   graft: nix/agent-tools/.scions/knowledge-base-kit <- v2.2.0 (abc1234)
   ```

   It writes `nix/agent-tools/.scions/README.md`, the marker that tells graft where the flake is. Later `add`
   commands find the directory by it, and need no `--scions`.

4. Put graft's workflows at the top, where Forgejo reads them:

   ```
   (cd nix/agent-tools && nix flake init -t github:danielnaab/graft/v2.13.1)
   mkdir -p .forgejo/workflows && mv -n nix/agent-tools/.forgejo/workflows/*.yaml .forgejo/workflows/ && rm -r nix/agent-tools/.forgejo
   ```

   `nix flake init` refuses to overwrite your `flake.nix` and exits 1: expected. Skip this step on GitHub.

5. Check it, and build the tools:

   ```
   nix run 'path:nix/agent-tools#graft' check
   nix build 'path:nix/agent-tools#tools'
   ```

   `graft check` prints `graft: 1 scion clean and at release tags`.

6. Commit the flake, the scions and the workflows, and nothing else:

   ```
   git add nix/agent-tools .gitmodules .forgejo
   git commit -m "Put the flake and its scions in nix/agent-tools"
   ```

7. Point paths at the new place: `AGENTS.md`, `just` imports (`import? "nix/agent-tools/.scions/…"`),
   hooks, and tools' settings that named a scion.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions are clean and at releases, no flake
is left at the top, everything is committed, and the store copy of the flake is exactly `nix/agent-tools/`,
with nothing from the rest of the repository:

```sh
nix run 'path:nix/agent-tools#graft' check && test ! -e flake.nix && test -z "$(git status --porcelain -- nix/agent-tools .gitmodules)" && src=$(nix eval --raw --impure --expr "(builtins.getFlake \"path:$PWD/nix/agent-tools\").outPath") && diff -r "$src" nix/agent-tools && echo ok
```

## If something goes wrong

- **`warning: graft: .scions/knowledge-base-kit, … are empty in the flake's source, so the tools leave out what they give: run git submodule update --init --recursive (or graft sync)`.**
  A fresh clone or worktree: the tools build without those providers. Run `git submodule update --init --recursive`. To fill scions on every clone, see
  [Clones, worktrees, sandboxes and CI](clones-and-worktrees.md).
- **`error: cannot fetch input 'path:nix/agent-tools' because it uses a relative path`.** Nix 2.24 needs
  the absolute path to write the lock: do step 2.
- **`error: path '/…/REPO' is not part of a flake`.** You ran `nix run .#graft` or `nix flake check` at the
  top. Write `'path:nix/agent-tools'` for `.`.
- **`graft: two scions directories: A, B; graft supports one per repository`.** An old scions directory at
  the top still has the marker README. Remove it (`git rm -r .scions`), and commit.

## How it works

| To | Run |
|---|---|
| Fill the scions | `git submodule update --init --recursive`, or `nix run 'path:nix/agent-tools#graft' sync` |
| Check | `nix run 'path:nix/agent-tools#graft' check` |
| Build the tools | `nix build 'path:nix/agent-tools#tools'` |
| Upgrade | `nix run 'path:nix/agent-tools#graft' upgrade` |
| Lock again after an edit to `flake.nix` | `nix flake lock "path:$PWD/nix/agent-tools"` |
| Run a command from graft's own flake | `nix run github:danielnaab/graft/v2.13.1 -- outdated -C .` |

- **Always write `path:`.** `.` inside the subdirectory, `./nix/agent-tools` and `git+file:` references are
  git flakes, which copy the whole repository.
- **graft finds the flake from the marker README**: the scions directory's parent. It finds the top with git,
  so the app runs from anywhere in the checkout.
- **A `path:` build always reads the working tree**, scion edits included. That makes trying a provider fix
  simple ([Fix a provider from a consumer](fix-a-provider.md)), and means `graft check` is the only thing
  that stops an unreleased scion from being committed.
- **A tool rebuild after an upgrade.** A script that caches the tools' build must include the scions'
  gitlinks in its cache key, for example
  `(cat nix/agent-tools/flake.nix nix/agent-tools/flake.lock; git submodule status -- nix/agent-tools/.scions) | cksum`.
- **In CI**, graft's workflows fill the scions before they evaluate a flake in a subdirectory.

### Remove old copies

1. Find them. Search for a line that only your private files hold, in the store paths that a flake
   (`-source`) or a `src = ./.` (named after the repository's folder) makes. It can take minutes:

   ```
   find /nix/store -mindepth 1 -maxdepth 1 -type d \( -name '*-source' -o -name "*-$(basename "$PWD")" \) -exec grep -rlF 'THE LINE' {} + 2>/dev/null
   ```

2. For each store path that it names, `nix-store --query --roots PATH` prints what keeps it. Nothing
   printed means that nothing does.
3. Delete each with `nix store delete PATH`. `nix store gc` deletes every unused path on the machine
   instead.

## Related

- [Graft a private repository](private-providers.md): a provider that only an ssh key can read.
- [Run an agent or a command in a sandbox](run-in-a-sandbox.md): `nix run 'path:nix/agent-tools#sandbox' -- CMD`.
- [Nix behaviours](../reference/nix-behaviours.md#path-flakes-in-a-subdirectory), [Commands](../reference/commands.md#mkflake).

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): `dir`, the empty-scion error.
- [lib/common.sh](../../lib/common.sh): finding the flake from the marker README.
- [lib/tools.sh](../../lib/tools.sh): `add --scions`.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-check.yaml): how CI finds the flake
  and fills the scions.
- [tests/run.sh](../../tests/run.sh): the `path:` subdirectory checks, and "the top's content never entered
  the store".
- [tests/fixtures.sh](../../tests/fixtures.sh) `private`: the repositories this page is read against.
