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
  scion*: a new file is invisible until `git -C scions/NAME add`. So the same command can pass, then fail,
  with no change to the scion ([Nix behaviours](../reference/nix-behaviours.md#what-a-build-sees-of-a-scion)).
  The provider's own tests (step 2) are the reliable test; step 3 says how to read a consumer build.

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

3. Try the fix in the consumer, from the consumer's top. First add new files in the scion, so that Nix can see
   them:

   ```
   cd ../..
   git -C scions/knowledge-base-kit add -A
   git status --short --untracked-files=no
   ```

   A consumer build uses your edit only if that lists a modified file **other than** `scions/…`, such as
   ` M docs/outage.md`. If it lists only the scion, the build tests the release, not your fix. Then:

   ```
   nix shell .#tools -c kb-lint .
   nix flake check
   ```

   For a flake in a subdirectory, `nix shell 'path:DIR#tools' -c kb-lint .` reads the working tree every
   time. Never build `path:.` at a repository's top: it copies every file there, ignored ones included, into
   the world-readable Nix store. To test the committed state exactly, push the branch and record it on a
   consumer branch ([Use the fix before its release](#use-the-fix-before-its-release)).

4. While the fix is not released, `graft check` says so. That is expected:

   ```
   nix run .#graft check
   ```

   ```
   graft: uncommitted edits in scions/knowledge-base-kit: src/kb_linter/status.py (Nix builds ignore them while the rest of the tree is clean)
   ```

5. Commit the fix in the scion, and push the branch to the provider:

   ```
   cd scions/knowledge-base-kit
   git commit -am "kb-lint: accept every status that knowledge-base.yaml lists"
   git push origin fix-status-values
   ```

   `origin` is the URL in `.gitmodules`. If it is an `https` URL and you push over ssh, set a push URL once:
   `git remote set-url --push origin git@github.com:danielnaab/knowledge-base-kit.git`. Without push rights
   (an agent sandbox, say), write a patch instead, and hand it on:
   `git format-patch -1 -o /tmp/kb-fix`.

6. Open a pull request on the provider from `fix-status-values`. Say which consumer showed the bug, and how
   to reproduce it.

7. Put the scion back to its pin, so that the consumer stays clean while you wait:

   ```
   cd ../..
   git submodule update -- scions/knowledge-base-kit
   ```

   The scion is at its gitlink again. Your branch stays in the scion's git directory.

8. When the provider releases the fix, take it:

   ```
   nix run .#graft upgrade knowledge-base-kit
   ```

   ```
   graft: knowledge-base-kit v2.2.0 -> v2.2.1
   ```

   Then commit and check, as in [Take a newer version by hand](upgrade.md#steps).

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
`graft: scions/knowledge-base-kit at abc1234 is not a release tag of …: release it, or graft upgrade knowledge-base-kit`, so the
pull request cannot pass graft-check until the scion is at a release. `graft upgrade` refuses a scion that is
not at a release, so move it by hand when the release is out:
`git -C scions/knowledge-base-kit fetch --tags -q && git -C scions/knowledge-base-kit checkout -q --detach v2.2.1 && git add scions/knowledge-base-kit`.
Until the commit is pushed, `nix run .#graft` itself cannot evaluate; run
`nix run github:danielnaab/graft/v2.13.0 -- check -C .` instead. With
`push.recurseSubmodules=check` ([Clones, worktrees, sandboxes and CI](clones-and-worktrees.md)), git refuses
to push the consumer while the scion's commit is on no remote.

## If something goes wrong

- **`nix flake check` passes, and the fix is not in it; or it fails on an edit you already reverted.** A git
  flake build read the scion at its gitlink, or the working tree, depending on whether another tracked file
  was modified. Read step 3's `git status` again.
- **`opening file '/nix/store/…-scion-knowledge-base-kit/NEW-FILE': No such file or directory`.** A file that
  you created in the scion is not tracked there. `git -C scions/knowledge-base-kit add NEW-FILE`.
- **`Cannot find Git revision '<sha>' in ref 'refs/heads/main' of repository '<url>'`.** The consumer records
  a scion commit that the provider does not have: you committed the gitlink before pushing the scion's
  branch. Push it (step 5), or go back with step 7 and `git add scions/NAME`.
- **`graft upgrade` refuses: the scion has uncommitted edits, or is checked out away from its gitlink.** Do
  step 7 first. Commit or stash anything in the scion that you want to keep.
- **`graft: scions/knowledge-base-kit is checked out at X but the repository records Y: …`.** The scion is
  on your branch. `git add scions/knowledge-base-kit` records it (only for [Use the fix before its
  release](#use-the-fix-before-its-release)); step 7 goes back. `graft sync` does not: it never moves a
  filled scion, so that it cannot lose your branch.

## How it works

- `graft check` fails on uncommitted edits, a scion away from its gitlink, and a gitlink that is not a
  version tag's commit. So a fix can live in a consumer's working tree and in a pushed branch, and still not
  merge unreleased.
- The provider's release is what every other consumer takes, through its weekly upgrade. Fixing it in the
  provider, not in a consumer's file, is the point.

## Sources

- [lib/tools.sh](../../lib/tools.sh): `upgrade` refuses a dirty or moved scion.
- [lib/graft.sh](../../lib/graft.sh): the `check` rules.
- [tests/run.sh](../../tests/run.sh): the Nix semantics tests (clean parent ignores a scion edit; dirty parent
  uses it; an unpushed gitlink fails; a branch commit is fetched).
- [tests/fixtures.sh](../../tests/fixtures.sh) `fix-a-provider`: the repositories this page was read against.
