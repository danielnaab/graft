---
status: working
---
# Move from copy mode

Move a repository that grafts with graft v2.12.0 or earlier (committed copies in `scions/`, `flake = false`
provider inputs) to v2.13.0: git submodules at release tags, and a `flake.nix` that names no provider. This
is a one-time change, by hand or by an agent following these steps.

## Before you begin

- Nix 2.27 or later: `nix --version`.
- **Which version of each provider to graft:** the first release with a `graft.nix` in the major version that
  you pin today, if the provider has one; otherwise the version you pin today (`PINNED` in step 1). A
  provider's CHANGELOG says which release adds `graft.nix`, and which settings moved from Nix arguments to a
  file. Graft a new major only on purpose ([Ship and take an incompatible change](incompatible-change.md)).
- **Every submodule in the repository must be fetchable.** With `inputs.self.submodules = true;`, Nix fetches
  every submodule that `.gitmodules` registers when it evaluates the flake, not only scions. An old or private
  submodule elsewhere (a `.graft/…` left from graft v1, say) breaks every build: remove it
  (`git rm PATH`, which also edits `.gitmodules`) or make it readable, in its own commit first.
- **A provider that is a Copier template** (python-kit, say): do its `copier update` to the version that
  supports v2.13.0 first, on a clean tree, as its CHANGELOG's Upgrading says, before step 2. Otherwise its
  update bot keeps editing a flake input that no longer exists.
- **Until the providers' releases are pushed**, Nix cannot fetch their tags, and a clean-tree
  `nix run .#graft …` or `nix flake check` fails. Check with a `path:` copy instead, and only in a repository
  with no private files (it copies the whole working tree into the store):
  `nix flake check 'path:.' --override-input graft git+file:///ABSOLUTE/PATH/TO/graft`. Run "Check it
  worked" after the push.
- graft v2.12's weekly upgrade job cannot make this move. When it tries to take v2.13.0, the job fails, with
  `graft: graft v2.13.0 replaced copied scions with git submodules; this upgrade can't be automatic: follow the Upgrading steps of v2.13.0 in graft's CHANGELOG.md (…)`
  in its log, and pushes nothing. Do these steps instead.
- The example has a flake at the top and scions in `scions/`. For a flake in a subdirectory, see the notes
  in each step.

## Steps

1. On a branch, write down each scion's version, before you change graft:

   ```
   git switch -c graft-submodules
   nix run .#graft outdated
   ```

   Pick each version to graft by the rule in [Before you begin](#before-you-begin).

2. Delete the copies, and commit that alone:

   ```
   git rm -r -q scions
   git commit -q -m "graft: remove copied scions"
   ```

   In a subdirectory flake (`flakeDir = "nix/agent-tools"`, scions at the top in `.scions`), the scions move
   under the flake's directory: delete the old one at the top. Committing first makes sure no copied file is
   left in git's index where the submodule goes.

3. Edit `flake.nix`. Keep your own inputs (`nixpkgs`, `follows`) and outputs:

   - Delete every `flake = false` provider input, and its wiring in `outputs` (`import "${inputs.X}/…"`,
     `callPackage "${inputs.X}/package.nix"`).
   - Delete `mkFlake`'s `scions` and `flakeDir` arguments. In a subdirectory flake, set `dir = ".scions";`.
   - Set `graft.url` to `github:danielnaab/graft/v2.13.0`.
   - For a flake at the repository's top, add `self.submodules = true;` to the inputs.
   - Tools of your own that were in `tools` go in `mkFlake`'s `perSystem`; other outputs stay in `outputs`
     ([Commands](../reference/commands.md#mkflake)). Consumer settings that were Nix arguments move to the
     files that each provider's CHANGELOG names.

   A typical result:

   ```nix
   {
     description = "What this repo is";
     inputs = {
       self.submodules = true;
       nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
       graft = { url = "github:danielnaab/graft/v2.13.0"; inputs.nixpkgs.follows = "nixpkgs"; };
     };
     outputs = { self, nixpkgs, graft, ... }@inputs: graft.lib.mkFlake {
       inherit inputs;
       perSystem = { pkgs, ... }: { tools = { inherit (pkgs) jq; }; };     # your own tools, if any
       outputs.templates.default = { path = ./template; description = "…"; };   # your other outputs
     };
   }
   ```

   With one provider and nothing of your own, the whole flake is three lines: see
   [Graft your first pattern](graft-a-repository.md#steps), step 1.

4. Graft each provider at the version you picked. `git add flake.nix` first, so that `add` sees the new flake:

   ```
   git add flake.nix
   nix run github:danielnaab/graft/v2.13.0 -- add -C . https://github.com/danielnaab/knowledge-base-kit@v2.2.0
   ```

   In a subdirectory flake, add `--scions nix/agent-tools/.scions` to the first `add`. A provider that grafts
   others prints `graft: note: scions/NAME has its own submodules (…)`: `add` fills them, and Nix fetches them
   too, so they must be readable from here.

5. Lock, write graft's files, commit, and check:

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
   A provider's new check can fail on content that was never checked before (a knowledge base's broken links,
   say): fix what it reports, or `nix flake check` stays red.

6. Fix paths in your own files that named the old scions directory: links, `just` imports (use
   `import? "scions/…"`), hooks, source imports, tools' settings, and lint or link-checker excludes (keep
   excluding the scions directory). Find them, leaving out the scions, `.gitmodules` and dated history:

   ```
   git grep -n -e 'scions/' -e '\.scions/' -- ':!scions' ':!*/.scions' ':!.gitmodules' ':!CHANGELOG.md'
   ```

   Lines in dated notes or logs describe the past: leave them.

7. Other CI that reads the scions needs them filled. On GitHub Actions, `actions/checkout` with
   `submodules: recursive` (and an `ssh-key` for a private scion). graft's own Forgejo workflows handle it.

8. Update your agent instructions: the file your agents read first (`AGENTS.md`, `CLAUDE.md`, or the entry
   point that `knowledge-base.yaml` names). Replace a rule like "never edit `scions/`" with the section in
   [Graft your first pattern](graft-a-repository.md#steps), step 6. If an agent tool or `knowledge-base.yaml`
   denies writes to the scions directory, decide whether to keep that or to ask instead: a scion edit is now a
   change to the provider, made on a branch and upstreamed.

9. Commit, run `nix flake check` and `nix run .#graft check` again, push, and merge. The next weekly upgrade
   runs v2.13.0's workflows.

## Check it worked

After the providers' releases are pushed, from the repository's top, paste this. It prints `ok` only if no
provider input is left, every scion is a clean submodule at a release, everything is committed, and the checks
pass:

```sh
! grep -q 'flake = false' flake.nix && nix run '.#graft' check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

On a clean tree, `nix run .#graft` makes Nix fetch every scion before graft runs, so an unpushed or
unreachable pin fails here with `Cannot find Git revision …` or `Failed to fetch …`. graft's own flake does
not evaluate yours: `nix run github:danielnaab/graft/v2.13.0 -- check -C .` names the scion.

## If something goes wrong

- **`graft: scions/README.md is graft's copy-mode README (v2.12 or earlier): …`.** The old scions directory
  is still there: step 2.
- **`graft: scions/NAME exists`, from `add`.** The copy is still on disk or in git's index. Do step 2 (commit
  the removal), and `rm -rf scions/NAME` for untracked leftovers, then `add` again.
- **`error: attribute 'X' missing` in your own outputs.** Your flake still uses a provider input deleted in
  step 3. Use the provider's tool from `packages.<system>.tools`, or `scions.X` in `perSystem`.
- **`graft: .gitmodules registers scions/X but the flake's source lacks it: …`.** Step 3's
  `self.submodules = true;` is missing, or Nix is older than 2.27.
- **`Failed to fetch git repository '…'` naming a path outside the scions directory.** Another submodule that
  Nix cannot fetch: see [Before you begin](#before-you-begin).

## Sources

- [CHANGELOG](../../CHANGELOG.md), v2.13.0: the same steps, as Upgrading.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the copy-mode README error.
- [tests/fixtures.sh](../../tests/fixtures.sh) `copy-mode`: a v2.12 consumer that these steps were read
  against.
