---
status: working
---
# Fix a provider from a consumer

You, or an agent, find a bug in a provider while working in a repository that grafts it. Fix it in the scion,
test the fix in the consumer, push it to the provider, and take it back as a release.

## When to use this

| You want | Do |
|---|---|
| A provider's rule, tool or check is wrong for everyone | This page. |
| A provider's file does not fit your repository only | Keep your own file, outside `scions/`, and point your agents and tools at it. A provider cannot leave a file out for one consumer. |
| A provider's tool should behave differently here | Change its settings file in your repository, such as `knowledge-base.yaml`. The provider's README names it. |
| To try a provider's branch that someone else pushed | [Steps](#steps) 3 and 4 only, with their branch. |

A scion is a real clone of the provider, so the fix can start where you found the bug. What it cannot do is
merge into the consumer before the provider releases it: `graft check` fails on a scion that is not at a
release tag.

## Before you begin

- The example consumer is `team-kb`, which grafts `knowledge-base-kit` in `scions/`. The bug: `kb-lint`
  rejects a valid status. Read your scions directory for `scions/`. For a flake in a subdirectory, write
  `nix run 'path:DIR#graft'` for `nix run .#graft`.
- You can push a branch to the provider, or you can hand a patch to someone who can.
- The scion is filled: `ls scions/knowledge-base-kit` lists files. If not:
  `git submodule update --init --recursive -- scions/knowledge-base-kit`.
- Know the Nix behaviour that makes this confusing: a git flake build (`nix build .#tools`,
  `nix flake check`) **ignores your uncommitted edit in a scion while every other tracked file is clean, and
  uses it as soon as any tracked file is modified**. Even then it sees only files that git tracks *in the
  scion*: a new file is invisible. So the same command can pass, then fail, with no change to the scion
  ([Nix behaviours](../reference/nix-behaviours.md#what-a-build-sees-of-a-scion)). The provider's own tests
  (step 2) are the reliable test; `graft try` (step 3) makes every consumer build use the fix, until you turn
  it off.
- **Do not switch the consumer's branch while you edit a scion.** A checkout, switch or pull of the parent
  that moves the scion's gitlink can move the scion under your edits, and with `submodule.recurse=true` it
  discards uncommitted ones without a word. Commit or stash in the scion first.

## Steps

1. Make a branch in the scion. A scion is checked out at a tag, with no branch:

   ```
   cd scions/knowledge-base-kit
   git switch -c fix-status-values
   ```

2. Fix the bug, and run the provider's own tests there, as its README or `AGENTS.md` says. For
   knowledge-base-kit:

   ```
   PYTHONPATH=src python3 -m pytest tests
   ```

3. Build the consumer with the fix. From the consumer's top, turn on `graft try`:

   ```
   cd ../..
   nix run .#graft -- try knowledge-base-kit
   ```

   ```
   graft: try is on: Nix builds of this repository read its working tree, and its scions' (.graft-try marks it)
   graft: scions/knowledge-base-kit: branch fix-status-values at v2.2.0 (19392ed), as recorded: 1 modified, 1 new Nix doesn't see
   graft: scions/knowledge-base-kit: Nix sees a new file only once git tracks it: git -C scions/knowledge-base-kit add -- tests/test_statuses.py
   graft: build from the repository's top: nix build .#tools, nix shell .#tools -c TOOL, or nix flake check
   graft: graft try --stop turns try off (before git stash or a pull with --autostash here), and graft check fails until then
   ```

   It marks `.graft-try` at the top with intent to add, so every Nix build reads the working tree. It does
   not touch the scion. Check its line for the scion: the branch, the edits you expect. A new file in the
   scion stays invisible to Nix until git tracks it there; run the `git -C … add` that `try` prints (it stages
   the file for real, in the provider's branch, where you will commit it anyway). Then build and check with
   the fix:

   ```
   git -C scions/knowledge-base-kit add -- tests/test_statuses.py
   nix shell .#tools -c kb-lint .
   nix flake check
   ```

   Created another file in the scion? `git -C scions/knowledge-base-kit add` it, or run `try` again to see
   the command. While try is on, `git stash` in the consumer fails, and so does `git pull --autostash`: turn
   try off first (step 4).

   When `nix run .#graft` itself fails (the fix broke `graft.nix`, or Nix cannot fetch a scion's gitlink),
   run graft from its own flake: `nix run github:danielnaab/graft/v2.16.1 -- try -C .`.

   For a flake in a subdirectory, `try` changes nothing and prints `nix shell 'path:DIR#tools' -c TOOL`: a
   `path:` flake reads its working tree, new files included, on every build. Never build `path:.` at a
   repository's top instead: it copies every file there, untracked and ignored ones included, into the
   world-readable Nix store.

4. While try is on, `graft check` fails first on that, then on the scion's uncommitted edits. That is
   expected:

   ```
   nix run .#graft check
   ```

   ```
   graft: graft try is on (.graft-try): Nix builds read the working tree, scions included: graft try --stop turns it off
   graft: uncommitted edits in scions/knowledge-base-kit: src/kb_linter/status.py tests/test_statuses.py (graft try is on: Nix builds use them)
   ```

   When you are done building, turn try off. It removes `.graft-try`; the scion stays as it is:

   ```
   nix run .#graft -- try --stop
   ```

5. Commit the fix in the scion (before you switch the consumer's branch), and push the branch to the
   provider:

   ```
   cd scions/knowledge-base-kit
   git add -A
   git commit -m "kb-lint: accept every status that knowledge-base.yaml lists"
   git push origin fix-status-values
   ```

   `origin` is the URL in `.gitmodules`. If it is an `https` URL and you push over ssh, set a push URL once:
   `git remote set-url --push origin git@github.com:danielnaab/knowledge-base-kit.git`. Without push rights
   (an agent sandbox, say), write a patch instead, and hand it on:
   `git format-patch -1 -o /tmp/kb-fix`.

6. Open a pull request on the provider from `fix-status-values`. Say which consumer showed the bug, and how
   to reproduce it.

7. Put the scion back to its pin, so that the consumer stays clean while you wait. If try is still on, turn
   it off first (`graft upgrade` refuses while it is on):

   ```
   cd ../..
   nix run .#graft -- try --stop
   git submodule update -- scions/knowledge-base-kit
   ```

   The scion is at its gitlink again. Your branch stays in the scion's git directory.

8. The provider merges the fix, tags a release, and pushes the tag:

   ```
   git -C ~/src/knowledge-base-kit tag v2.2.1 && git -C ~/src/knowledge-base-kit push origin main v2.2.1
   ```

   (Or its maintainers do.) Then take it in the consumer, naming the tag:

   ```
   nix run .#graft upgrade knowledge-base-kit@v2.2.1
   ```

   ```
   graft: knowledge-base-kit v2.2.0 -> v2.2.1
   ```

   `NAME@TAG` works also when the consumer recorded the unreleased fix
   ([Use the fix before its release](#use-the-fix-before-its-release)). Then commit and check, as in
   [Take a newer version by hand](upgrade.md#steps).

## Check it worked

After step 8, from the consumer's top, paste this. It prints `ok` only if the scion is clean and at a
release, the move is committed, and the checks pass on the committed tree:

```sh
nix run '.#graft' check && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

## Use the fix before its release

Sometimes the consumer's branch needs the fix before the provider releases it, to show it in a pull request.
Record the pushed commit:

```
git add scions/knowledge-base-kit
git commit -m "knowledge-base-kit: fix-status-values, until v2.2.1"
```

Nix fetches a commit that is on any branch of the provider, so builds work. `graft check` fails with
`graft: scions/knowledge-base-kit at abc1234 is not a release tag of …: release it, then graft upgrade knowledge-base-kit@TAG …`, so the
pull request cannot pass graft-check until the scion is at a release: step 8's
`nix run .#graft upgrade knowledge-base-kit@v2.2.1` moves it there. Until the commit is pushed, Nix cannot
fetch it, so `nix run .#graft` itself cannot evaluate on a clean tree; run
`nix run github:danielnaab/graft/v2.16.1 -- check -C .` instead. With
`push.recurseSubmodules=check` ([Clones, worktrees, sandboxes and CI](clones-and-worktrees.md)), git refuses
to push the consumer while the scion's commit is on no remote.

## If something goes wrong

- **`nix flake check` passes, and the fix is not in it; or it fails on an edit you already reverted.** A git
  flake build read the scion at its gitlink, or the working tree, depending on whether another tracked file
  was modified. Turn on `graft try` (step 3) and read its line for the scion.
- **`opening file '/nix/store/…-scion-knowledge-base-kit/NEW-FILE': No such file or directory`.** git does
  not track the file in the scion, so Nix does not see it:
  `git -C scions/knowledge-base-kit add -- NEW-FILE` (step 3).
- **`graft: .graft-try is committed …`.** `git commit -a`, or `git add -A` then a commit, took try's marker.
  `nix run .#graft -- try --stop` stages its removal; commit that ([`graft check` fails](check-fails.md)).
- **`error: Entry '.graft-try' not uptodate. Cannot merge.`, from `git stash` or a pull with `--autostash`.**
  Try is on. `nix run .#graft -- try --stop`, then stash.
- **`Cannot find Git revision '<sha>' in ref 'refs/heads/main' of repository '<url>'`.** The consumer records
  a scion commit that the provider does not have: you committed the gitlink before pushing the scion's
  branch. `nix run .#graft` fails the same way, before graft runs; use graft's own flake meanwhile
  (`nix run github:danielnaab/graft/v2.16.1 -- try -C .`, or `-- check -C .`). Push the branch (step 5), or
  record the release again: `git -C scions/NAME checkout --detach v2.2.0` (the tag it was at), then
  `git add scions/NAME` and commit. Step 7 does not help here: it checks out the recorded commit.
- **`graft upgrade` refuses: the scion has uncommitted edits, or try is on.** Do step 7 first. Commit or stash
  anything in the scion that you want to keep.
- **`graft: scions/knowledge-base-kit is checked out at X but the repository records Y: …`.** The scion is
  on your branch. `git add scions/knowledge-base-kit` records it (only for [Use the fix before its
  release](#use-the-fix-before-its-release)); step 7 goes back. `graft sync` does not: it never moves a
  filled scion, so that it cannot lose your branch.

## How it works

- `graft try` marks `.graft-try` with intent to add (`git add -N`): it differs from `HEAD`, so Nix reads the
  working tree, and nothing is staged for real, so a plain `git commit` leaves it out. It never touches a
  scion's index: a file marked with intent to add counts as tracked with empty content, so
  `git checkout -- .` would empty it and `git reset --hard` delete it. `graft try --stop` removes the marker.
- `graft check` fails while try is on, and on uncommitted edits, a scion away from its gitlink, and a
  gitlink that is not a version tag's commit. So a fix can live in a consumer's working tree and in a pushed branch, and still not
  merge unreleased.
- The provider's release is what every other consumer takes, through its weekly upgrade. Fixing it in the
  provider, not in a consumer's file, is the point.

## Sources

- [lib/tools.sh](../../lib/tools.sh): `upgrade` refuses a dirty scion, and refuses while try is on.
- [lib/scions.sh](../../lib/scions.sh): `try`, `try --stop` and the `check` rules.
- [tests/run.sh](../../tests/run.sh): the Nix semantics tests (clean parent ignores a scion edit; dirty parent
  uses it; an unpushed gitlink fails; a branch commit is fetched), and `try` (a build sees the edit and a new
  file only while it is on; `check` refuses while it is on).
- [tests/fixtures.sh](../../tests/fixtures.sh) `fix-a-provider`: the repositories this page was read against.
