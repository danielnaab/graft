---
status: working
---
# `graft check` fails: find what changed in a scion, and fix it

When `nix run .#graft check` or `nix flake check` fails on drift, find which files changed and why,
save any change someone meant, put the scions back to their pins, and stop the cause from doing it again.

## Before you begin

- graft never keeps an edit to a scion: step 5 replaces it. A change that you want to keep belongs in the
  provider.
- The commands assume the scions directory is `scions/` and `flake.nix` is at the repository's top. If
  `mkFlake` sets `dir`, read your directory for `scions/`. If your flake is in a subdirectory, run
  `nix run path:DIR#graft` for `nix run .#graft`. Then `nix flake check` does not check the scions:
  `nix run path:DIR#graft check` is the check ([Keep private files out of the Nix store](flake-in-a-subdirectory.md)).
- Commit or stash work that is not about the scions, so that `git status` shows only the drift.

## Steps

1. Run the check from the repository's top:

   ```
   nix run .#graft check
   ```

   Nix may first warn `Git tree '…' is dirty`: that only means uncommitted changes. On drift, graft
   prints a `diff -r` of the pinned copy (left) against your scions (right), then this line, and exits 1:

   ```
   graft: scions/ differs from the pins in flake.lock; run: nix run .#graft
   ```

   If it prints `graft: scions/ and graft's workflows match the pins`, the scions are fine. If it prints
   neither line, read its `graft:` lines, and see [If something goes wrong](#if-something-goes-wrong).

2. Read each part of the diff. The left side is always the pin, a path ending in `-graft-scions/`. The
   right side is your copy: under `/tmp/…/actual/scions/` from `graft check`, or
   `/nix/store/…-source/scions/` from `nix flake check`. Each `FILE` can be a whole directory.

   | The diff says | It means | Usual cause |
   |---|---|---|
   | `diff -r LEFT/NAME/FILE RIGHT/NAME/FILE` and changed lines | a grafted file was edited | you, an agent, a formatter or a linter's `--fix` |
   | `Only in RIGHT/NAME/DIR: FILE` | a file that the pin does not have | a tool wrote it (`__pycache__/`, a build), or someone added it |
   | `Only in LEFT/NAME/DIR: FILE` | a grafted file that your copy lacks | it was deleted; or git ignores it (`git check-ignore -v scions/NAME/FILE` names the rule); or, for `nix flake check` only, it is not added to git yet |
   | a changed line in `scions/README.md`'s table of commits | the pin moved, and the scions did not | see [If something goes wrong](#if-something-goes-wrong) |

   A line `graft: .forgejo/workflows/FILE differs from the copy this graft version ships` is one of graft's
   workflows. Step 5 restores it. To keep your edits instead, delete its marker line ([Files](../reference/files.md)).

   Not yet: the diff names a temporary directory and store paths, not `scions/NAME/FILE`.
   Not yet: the check does not explain a file only on the right, and for a file only on the left it
   names only `.gitignore`, even when the file was deleted.

3. Find which changes someone meant. The check shows committed and uncommitted edits alike, so ask git:

   ```
   git status --short -- scions/
   git log --oneline -- scions/
   ```

   Uncommitted changes are often a tool's output. A commit with a message of its own is usually a person's.

4. Commit each meant change that is not committed yet, naming its files, so that a tool's output stays out:

   ```
   git commit -m "Meant scion edit" -- scions/NAME/FILE
   ```

5. Put the scions back to their pins:

   ```
   nix run .#graft
   ```

   It prints one line for each scion, `graft: scions/NAME <- COMMIT`. It replaces the whole scions
   directory, so stray files go too, and stages it. It also restores graft's workflows, which it does not
   stage.

6. Save each meant change, from the staged undo of it, outside the repository:

   ```
   git diff --cached -R -- scions/NAME/FILE > /tmp/scion-change.patch
   ```

   Its headers read `b/` before `a/`, which `git apply` does not mind. In the provider's checkout,
   `git apply -p3 /tmp/scion-change.patch` applies it: add one to `-p` for each extra segment in your
   scions directory's path. Commit and release it there, then take the new version with [Upgrade](upgrade.md).
   To try it here first, see [a provider's unreleased change](upgrade.md#try-a-providers-unreleased-change).

7. Stop the cause from doing it again. Tell each tool that the scions directory is not yours:

   | Tool | Add |
   |---|---|
   | ruff | `extend-exclude = ["scions"]` and `force-exclude = true`, under `[tool.ruff]` in `pyproject.toml` |
   | black | `extend-exclude = "^/scions/"` and `force-exclude = "^/scions/"`, under `[tool.black]` in `pyproject.toml` |
   | pre-commit | `exclude: ^scions/` at the top of `.pre-commit-config.yaml` |
   | Prettier | `scions/` in `.prettierignore` |
   | treefmt | `excludes = ["scions/**"]` under `[global]` in `treefmt.toml` |
   | Python run from a scion | `__pycache__/` in `.gitignore`, or `PYTHONDONTWRITEBYTECODE=1` |
   | An agent | "Never edit `scions/`: graft replaces it" in `AGENTS.md` |

   `force-exclude` matters because pre-commit and an editor's format-on-save name each file, and without
   it ruff and black format a named file even when it is excluded. The check reads files as git would commit
   them, so a file that git ignores does not count.

8. Check again:

   ```
   nix run .#graft check
   ```

   Expected, with exit 0:

   ```
   graft: scions/ and graft's workflows match the pins
   ```

9. Commit the restored scions and each file that you changed in step 7, here `pyproject.toml` and
   `.gitignore`:

   ```
   git add -A -- scions/
   git add -- pyproject.toml .gitignore
   git commit -m "Restore scions to their pins"
   ```

   If step 5 printed `graft: refreshed .forgejo/workflows/FILE`, add that file too. Name only paths that
   exist: when one does not, `git add` stages nothing at all. Then `git status --short` should list nothing.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions match their pins, nothing is left
to commit, and the check that CI runs passes:

```sh
nix run .#graft check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## If something goes wrong

- **`graft check` and `nix flake check` disagree.** `nix flake check` sees only what git tracks.
  `graft check` also sees files that git would add. So a new file in `scions/` that git does not ignore
  fails only `graft check`, and a grafted file removed from git (`git rm --cached`, or a new scion that was
  never added) fails only `nix flake check`. `nix flake check` shows the last 25 lines of the diff; run
  `nix flake check -L` for all of it.
- **The diff shows a commit changing in `scions/README.md`.** The pin moved and the scions did not: a tag
  edited in `flake.nix`, `nix flake update`, or a bot. Every other file in the diff changed upstream, not
  here. Run `nix run .#graft` to take the new pin, or go back to the old one, and see [Upgrade](upgrade.md).
- **Step 5 prints `graft: flake.lock re-pinned; nothing synced yet: run nix run .#graft again`.** The lock
  pointed at an annotated tag's object, and graft fixed it. Run step 5 again, and commit `flake.lock`
  with the scions.
- **Step 5 refuses: `graft: scions/ exists and graft didn't write it (no marker in its README.md)`.** The
  scions README lost its first line, or was deleted. Save any uncommitted meant change first, with
  `git diff -- scions/NAME/FILE > /tmp/scion-change.patch`. Then restore the README with
  `git checkout HEAD -- scions/README.md`, and run step 5 again. Delete the directory only if graft never
  wrote it.

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the `check` case of the app (`graft-sync`) and the
  `checks.<system>.graft` derivation; the `sync` case, which replaces and stages the scions directory, and
  refuses one without the marker.
- [lib/graft.sh](../../lib/graft.sh): the re-pin before a sync.
- [lib/workflows.sh](../../lib/workflows.sh): the check and refresh of graft's workflows.
- [tests/run.sh](../../tests/run.sh): "check catches an edited scion", "check catches a scion file git
  doesn't track", "check catches a pin moved without grafting", "check catches an edited graft workflow",
  "graft won't replace a directory it didn't write", and the `subdir:` checks.
- [Commands](../reference/commands.md), [Files](../reference/files.md).
