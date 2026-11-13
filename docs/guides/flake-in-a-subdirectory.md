---
status: working
---
# Keep private files out of the Nix store

Put the repository's flake in a subdirectory, so that Nix copies only that directory, and never your private
files, into the Nix store.

## Before you begin

- Why: Nix copies a flake's source into the Nix store, and every user of the machine can read the store. For
  a flake at the repository's top, the source is every file that git tracks, with uncommitted edits. Every
  `nix run .#graft`, `nix flake check` and `nix flake update` copies them again. Nix 2.24 has no lazy trees.
- What changes: the flake's source is only its subdirectory. graft's drift check is then
  `nix run 'path:DIR#graft' check`, not a check in `nix flake check`. A check of your own cannot read files
  outside the subdirectory. Rename any check of your own that is named `graft`: in a subdirectory flake,
  graft drops it without a word.
- graft 2.7.0 or later has `flakeDir`. graft's workflows find the flake from 2.7.2. Your graft version is
  the tag in your `graft` input's URL (`github:danielnaab/graft/v2.11.1`), also the `PINNED` column of the
  `graft` row that `nix run .#graft outdated` prints. `graft upgrade` in a
  subdirectory flake needs Nix 2.19 or later.
- Copies that Nix made before stay in the store. See [Remove old copies](#remove-old-copies).
- The steps use `nix/workspace` for the subdirectory and `.scions` for the scions directory. Use your own
  names: `flakeDir` can contain only letters, digits, `.`, `_`, `-` and `/`.
- The commands quote `'path:nix/workspace#graft'`, because in zsh with `extendedglob` set, `#` is a glob.

## Steps

Run every command from the repository's top. Never run `nix run .#graft`, `nix flake check` or direnv's
`use flake` inside `nix/workspace/`: there, `.` is the git checkout, and Nix copies the whole repository.

1. Make the subdirectory. Keep no other files in it: Nix copies all of them.

   ```
   mkdir -p nix/workspace
   ```

2. Put a flake in it.

   - If the flake is at the top, move it, and its lock:

     ```
     git mv flake.nix flake.lock nix/workspace/
     ```

     If the repository has no `flake.lock`, move `flake.nix` only.

   - If the repository has no flake yet, write graft's template there:

     ```
     (cd nix/workspace && nix flake init -t github:danielnaab/graft/v2.11.1)
     ```

     It writes `flake.nix` and `.forgejo/workflows/` in `nix/workspace/`, and does not add them to git.
     Its message says `then nix run .#graft and commit`: for a subdirectory, follow this page instead. Edit
     the inputs as in [Graft your first pattern](graft-a-repository.md), step 2.

3. If `nix/workspace/.forgejo/` exists, move its workflows to the top, where Forgejo reads them:

   ```
   mkdir -p .forgejo/workflows && mv -n nix/workspace/.forgejo/workflows/*.yaml .forgejo/workflows/ && rmdir nix/workspace/.forgejo/workflows nix/workspace/.forgejo
   ```

   `mv -n` keeps a workflow that the top already has. Then `rmdir` fails with `Directory not empty`: compare
   the file that is left with yours, and delete one of them.

4. In `nix/workspace/flake.nix`, give `mkFlake` the subdirectory as `flakeDir`. `dir` stays relative to the
   repository's top:

   ```nix
   outputs = inputs: inputs.graft.lib.mkFlake {
     inherit inputs;
     flakeDir = "nix/workspace";   # where this flake.nix is, from the repository's top
     dir = ".scions";              # the scions directory, from the repository's top
   };
   ```

   Keep the other arguments that your `mkFlake` already has. Without `dir`, as in the template, the scions
   directory is `scions/`. If an input has a relative path (`path:./tools`), make it relative to the
   subdirectory.

5. Lock the flake, by its absolute path. Nix does not write the lock of a relative `path:` flake:

   ```
   nix flake lock "path:$PWD/nix/workspace"
   ```

   With a moved `flake.lock`, it prints nothing. For a new flake, it prints
   `warning: creating lock file '…/nix/workspace/flake.lock'` and an `• Added input` line for each input,
   graft's own inputs (`graft/nixpkgs`, for example) included.

6. Graft:

   ```
   nix run 'path:nix/workspace#graft'
   ```

   It prints one line for each scion, `graft: .scions/NAME <- COMMIT`, and stages `.scions/`. It writes the
   subdirectory into the second line of `.scions/README.md`, `<!-- graft flake: nix/workspace -->`, and
   the README's commands now say `nix run path:nix/workspace#graft`. In zsh, quote them when you type them.

7. Check it:

   ```
   nix run 'path:nix/workspace#graft' check
   ```

   Expected, with exit 0:

   ```
   graft: .scions/ and graft's workflows match the pins
   ```

8. Commit the flake, the scions and the workflows, and nothing else:

   ```
   git add nix/workspace .scions .forgejo
   git commit -m "Put the flake in nix/workspace"
   ```

   After a move, `git mv` in step 2 already staged the removal of `flake.nix` from the top. Your private
   files keep their own state: this commit does not take them.

9. Run your own checks:

   ```
   nix flake check 'path:nix/workspace'
   ```

   It runs every check of yours, and exits 0. It does not run graft's check: step 7 does.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions match their pins, no flake is left
at the top, the flake is committed, and the store copy of the flake is exactly `nix/workspace/`, with
nothing from the rest of the repository:

```sh
nix run 'path:nix/workspace#graft' check && test ! -e flake.nix && git ls-files --error-unmatch nix/workspace/flake.nix nix/workspace/flake.lock >/dev/null && git diff --quiet HEAD -- nix/workspace .scions && src=$(nix eval --raw --impure --expr "(builtins.getFlake \"path:$PWD/nix/workspace\").outPath") && diff -r "$src" nix/workspace && echo ok
```

## If something goes wrong

- **`error: cannot fetch input 'path:nix/workspace' because it uses a relative path`.** The flake has no
  lock yet, or its inputs changed. Do step 5, with the absolute path.
- **`graft: no flake.nix in /…/REPO: run from the repo's top; for a flake in a subdirectory, set mkFlake's flakeDir to it`.**
  `mkFlake` has no `flakeDir`, so graft looks for the flake at the top. Do step 4.
- **`graft: no DIR/flake.nix in /…/REPO: check mkFlake's flakeDir, "DIR" (the scions README records it)`.**
  `flakeDir` names another directory than the flake's. Correct it, as in step 4.
- **`error: path '//…/REPO/notebook/nix/workspace' does not exist`.** A `path:` reference is relative to
  the current directory. Run from the top, or write the path from where you are:
  `nix run 'path:../nix/workspace#graft'`.
- **`error: path '/…/REPO' is not part of a flake (neither it nor its parent directories contain a 'flake.nix' file)`.**
  You ran `nix run .#graft` or `nix flake check` at the top. Write `path:nix/workspace` for `.`, as in
  [How it works](#how-it-works).

## How it works

The commands, from the repository's top:

| To | Run |
|---|---|
| Sync | `nix run 'path:nix/workspace#graft'` |
| Check | `nix run 'path:nix/workspace#graft' check` |
| Upgrade | `nix run 'path:nix/workspace#graft' upgrade` |
| Lock again after an edit to `flake.nix` | `nix flake lock "path:$PWD/nix/workspace"` |
| Run a command from graft's own flake | `nix run github:danielnaab/graft -- outdated -C .` (`-C` takes the repository's top) |

- **Always write `path:`.** `.` inside the subdirectory, `./nix/workspace` and `git+file:` references are
  git flakes, which copy the whole repository. `nix/workspace` alone is read as a flake registry name.
- **graft finds the top with git** (`git rev-parse --show-toplevel`), so the app runs from anywhere in the
  checkout.
- **The scions README records the flake** on its second line. graft's other commands and its CI workflows
  find the flake from that line, so they need no change. Every scions directory in the repository must
  name the same flake ([Files](../reference/files.md#the-scions-readme)). The workflows read it by the
  commands' rules, so a scion whose provider did not exclude its own scions README does not count.
- **In CI**, `graft ci check` runs `graft check`. The upgrade workflow and `graft migrate` run it, and then
  the flake's own checks.

### Remove old copies

1. Find them. Search for a line that only your private files hold, in the store paths that a flake
   (`-source`) or a `src = ./.` (named after the repository's folder) makes. It can take minutes:

   ```
   find /nix/store -mindepth 1 -maxdepth 1 -type d \( -name '*-source' -o -name "*-$(basename "$PWD")" \) -exec grep -rlF 'THE LINE' {} + 2>/dev/null
   ```

   It misses a copy of a file that did not hold that line yet, a copy named after another folder, and a
   compressed file. Search for a line from each private file that matters.
2. For each store path that it names, `nix-store --query --roots PATH` prints what keeps it. Nothing
   printed means that nothing does. A line `removing stale link from '/nix/var/nix/gcroots/auto/…'` is Nix
   tidying up, not a root.
3. Delete each with `nix store delete PATH`. Nix refuses a path that something still uses
   (`Cannot delete path … since it is still alive`). `nix store gc` deletes every unused path on the
   machine instead, every user's cache too.

## Related

- [Graft a private repository](private-providers.md): a provider that only an ssh key can read.
- [Nix behaviours](../reference/nix-behaviours.md), [Commands](../reference/commands.md#mkflake).

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): `flakeDir` and its check, the app's search for the top,
  no `checks.<system>.graft` in a subdirectory, and `unclashed`, which makes a repository's own check named `graft` an error.
- [lib/common.sh](../../lib/common.sh): `locate` and `recorded_flake_dir`. [lib/assemble.sh](../../lib/assemble.sh):
  the README's second line and its commands.
- [flake.nix](../../flake.nix): the template's message. [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-check.yaml):
  how CI finds the flake.
- [CHANGELOG](../../CHANGELOG.md): `flakeDir` in v2.7.0, its workflows fixed in v2.7.2.
- [tests/run.sh](../../tests/run.sh): the `subdir:` checks, and "the top's content never entered the store".
- [tests/fixtures.sh](../../tests/fixtures.sh) `private`: `moving/` and `new/`, the repositories this page is
  read against.
