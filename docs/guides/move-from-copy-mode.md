---
status: working
---
# Move from copy mode

Move a repository that grafts with graft v2.12.0 or earlier (committed copies in `scions/`, `flake = false`
provider inputs) to v2.13.0: git submodules at release tags, and a `flake.nix` that names no provider. This
is a one-time change, by hand or by an agent following these steps.

## Before you begin

- Nix 2.27 or later: `nix --version`.
- Each provider you use from Nix (its tools, a `nix/package.nix`, a `src/` on `PYTHONPATH`) has a release with
  a `graft.nix`. Its CHANGELOG says which release, and which settings moved from Nix arguments to a file.
- graft v2.12's weekly upgrade job cannot make this move. When it tries to take v2.13.0, the job fails, with
  `graft v2.13.0 replaced copied scions with git submodules: see …CHANGELOG.md…` in its log, and opens no
  pull request. Do these steps instead.
- The example has a flake at the top and scions in `scions/`. For a flake in a subdirectory, see the notes
  in each step.

## Steps

1. On a branch, write down each scion's version, before you change graft:

   ```
   git switch -c graft-submodules
   nix run .#graft outdated
   ```

   The `PINNED` column is the tag to graft in step 4.

2. Delete the copies:

   ```
   git rm -r -q scions
   ```

   In a subdirectory flake (`flakeDir = "nix/agent-tools"`, scions at the top in `.scions`), the scions move
   under the flake's directory: delete the old one at the top.

3. Edit `flake.nix`:

   - Delete every `flake = false` provider input, and its wiring in `outputs` (`import "${inputs.X}/…"`,
     `callPackage "${inputs.X}/package.nix"`).
   - Delete `mkFlake`'s `scions` and `flakeDir` arguments. In a subdirectory flake, set `dir = ".scions";`.
   - Set `graft.url` to `github:danielnaab/graft/v2.13.0`.
   - For a flake at the repository's top, add `inputs.self.submodules = true;`.
   - Tools of your own that were in `tools` (a compiler your scripts need, say) go in `mkFlake`'s
     `perSystem` ([Commands](../reference/commands.md#mkflake)). Consumer settings that were Nix arguments
     move to the files that each provider's CHANGELOG names.

   For a repository with one provider, the whole flake is:

   ```nix
   {
     description = "What this repo is";
     inputs.self.submodules = true;
     inputs.graft.url = "github:danielnaab/graft/v2.13.0";
     outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
   }
   ```

4. Graft each provider at the version from step 1, or at the first release with a `graft.nix`:

   ```
   git add flake.nix
   nix run github:danielnaab/graft/v2.13.0 -- add https://github.com/danielnaab/knowledge-base-kit@v2.2.0
   ```

   In a subdirectory flake, add `--scions nix/agent-tools/.scions` to the first `add`.

5. Lock, write graft's files, and check:

   ```
   nix flake lock
   nix run .#graft sync
   git add -A
   git commit -m "graft v2.13.0: scions are git submodules"
   nix flake check
   nix run .#graft check
   ```

   `sync` writes the marker README and graft's v2.13.0 workflows. For a subdirectory flake:
   `nix flake lock "path:$PWD/nix/agent-tools"`, `nix run 'path:nix/agent-tools#graft' sync`, and so on.

6. Fix paths in your own files that named the old scions directory: links, `just` imports (use
   `import? "scions/…"`), hooks, source imports, tools' settings, and lint or link-checker excludes. Find them:

   ```
   git grep -n -e 'scions/' -e '\.scions/' -- ':!scions' ':!*/.scions'
   ```

7. Replace the agent rule "never edit `scions/`" in `AGENTS.md` with the section in
   [Graft your first pattern](graft-a-repository.md#steps), step 6. If your agent tool denies edits to the
   scions directory, change that to ask.

8. Commit, run `nix flake check` and `nix run .#graft check` again, and merge. The next weekly upgrade runs
   v2.13.0's workflows.

## Check it worked

From the repository's top, paste this. It prints `ok` only if no provider input is left, every scion is a
clean submodule at a release, everything is committed, and the checks pass:

```sh
! grep -q 'flake = false' flake.nix && nix run '.#graft' check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## If something goes wrong

- **`graft: scions/README.md is graft's copy-mode README (v2.12 or earlier): …`.** The old scions directory
  is still there: step 2.
- **`graft: scions/NAME exists`, from `add`.** The copy is still on disk. `git rm -r -q scions/NAME` (and
  `rm -rf scions/NAME` for untracked leftovers), then `add` again.
- **`error: attribute 'X' missing` in your own outputs.** Your flake still uses a provider input deleted in
  step 3. Use the provider's tool from `packages.<system>.tools`, or `scions.X` in `perSystem`.
- **`graft: .gitmodules registers scions/X but the flake's source lacks it: …`.** Step 3's
  `inputs.self.submodules = true;` is missing, or Nix is older than 2.27.

## Sources

- [CHANGELOG](../../CHANGELOG.md), v2.13.0: the same steps, as Upgrading.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the copy-mode README error.
- [tests/fixtures.sh](../../tests/fixtures.sh) `copy-mode`: a v2.12 consumer that these steps were read
  against.
