---
status: working
---
# Graft a private repository

Graft a provider that only an ssh key can read, on your machine and in graft-check.

## Before you begin

- You can read the provider over ssh. This prints the tag's commit:

  ```
  git ls-remote ssh://git@git.example.com:2222/team/notes refs/tags/v1.0.0
  ```

  Nix fetches the provider with your own ssh setup: your key, your agent and `~/.ssh/config`.
- The provider's export becomes readable by more people than the provider is. Nix copies it into the Nix
  store, which every user of your machine can read, and graft commits it into your scions directory, which
  everyone who can read your repository can read. Keep your own private files out of the store with
  [Keep private files out of the Nix store](flake-in-a-subdirectory.md).
- The provider's `.gitattributes` writes its directory rules without a trailing slash (`tests export-ignore`,
  not `tests/ export-ignore`). Over ssh, Nix fetches from a bare clone and ignores the slash form
  ([What to export](../reference/files.md#what-to-export)).
- Your repository already grafts with graft ([Graft your first pattern](graft-a-repository.md)), and keeps
  graft's `graft-check.yaml` in `.forgejo/workflows/`. If your flake is in a subdirectory, write
  `nix run 'path:DIR#graft'` for `nix run .#graft`.
- You need admin rights on the provider's repository, to add a deploy key, and a trusted copy of the
  server's host key: a shell on the server, or its admin. Without one, compare the key that `ssh-keyscan`
  prints with the one in your own `~/.ssh/known_hosts`, from your first connection.
- The example's host is `git.example.com`, its ssh port `2222`, and the provider `team/notes`. Use yours.
  Read your scions directory for `scions/`.

## Steps

1. Fetch the tag once, by its full name. Nix 2.24 cannot make the first lock of `ref=v1.0.0` over ssh, but
   it can when its git cache has the tag:

   ```
   nix flake prefetch 'git+ssh://git@git.example.com:2222/team/notes?ref=refs/tags/v1.0.0&exportIgnore=1'
   ```

   It prints `Downloaded 'git+ssh://…&rev=COMMIT' to '/nix/store/…-source' …`.

2. Add the provider to `flake.nix`'s inputs, with `exportIgnore=1`:

   ```nix
   notes = { url = "git+ssh://git@git.example.com:2222/team/notes?ref=v1.0.0&exportIgnore=1"; flake = false; };
   ```

   Without `exportIgnore=1`, Nix ignores the provider's export rules, and the scion gets its whole tree.

3. Graft:

   ```
   nix run .#graft
   ```

   Nix may first print `fatal: couldn't find remote ref refs/heads/v1.0.0` and a warning that it
   `could not update mtime`: ignore them when the lock is written. Then it prints
   `warning: updating lock file` and `• Added input 'notes'`. graft prints a line for each scion, among
   them `graft: scions/notes <- COMMIT`, with the commit from step 1. It stages the scion's files itself.

4. Make a key pair for CI, in a new directory outside the repository, so that no commit can take it:

   ```
   keys=$(mktemp -d) && ssh-keygen -t ed25519 -N '' -C "graft-check@YOUR-REPO" -f "$keys/graft_ssh_key" && echo "$keys"
   ```

   Do step 5 in the same shell, or write the printed directory for `$keys`.

5. Give the key pair to the forge, then delete it:

   - On the provider's repository, add the content of `"$keys/graft_ssh_key.pub"` as a read-only deploy
     key (Forgejo: Settings, Deploy keys).
   - On your repository, add the content of `"$keys/graft_ssh_key"` as the Actions secret `GRAFT_SSH_KEY`
     (Forgejo: Settings, Actions, Secrets).
   - Delete both key files: `rm -r "$keys"`.

6. Get the host's public keys:

   ```
   ssh-keyscan -p 2222 git.example.com 2>/dev/null
   ```

   It prints one line per key, such as `[git.example.com]:2222 ssh-rsa AAAA…`. Take the line whose type
   matches a key file on the server, and compare the key with that file: for Forgejo's built-in ssh server,
   `data/ssh/gitea.rsa.pub` (`ssh-rsa`); for the system's OpenSSH, a file such as
   `/etc/ssh/ssh_host_ed25519_key.pub`.

7. Add one line to `.forgejo/workflows/graft-check.yaml`, anywhere. `host=` is the first field of step 6's
   line, and `hostkey=` the other two:

   ```yaml
   # graft ssh: host=[git.example.com]:2222 hostkey=ssh-rsa AAAA…
   ```

   Add `address=IP` if CI's runner cannot resolve the host's name, and `secret=NAME` if the secret is not
   `GRAFT_SSH_KEY` ([Files](../reference/files.md#the-graft-ssh-line)).

8. Delete graft's upgrade workflow. With a private scion, it fails every week, because it has no key (see
   [How it works](#how-it-works)). graft never adds it back:

   ```
   git rm .forgejo/workflows/graft-upgrade.yaml
   ```

9. Let graft write the step that the line asks for:

   ```
   nix run .#graft
   ```

   After the scions' lines, it prints `graft: refreshed .forgejo/workflows/graft-check.yaml`. The file now
   has a step, `SSH key for private scions`, before the check. It writes the secret to
   `~/.ssh/graft_scions`, the host key to `~/.ssh/known_hosts`, and a `Host` entry with
   `StrictHostKeyChecking yes` to `~/.ssh/config`. graft moved the line above `name:`.

10. Commit the flake, the scions directory and the workflows, and nothing else. Step 8 already staged the
    removal of `graft-upgrade.yaml`:

    ```
    git add flake.nix flake.lock scions .forgejo/workflows/graft-check.yaml
    git commit -m "Graft notes v1.0.0, a private provider"
    ```

    For a flake in a subdirectory, name that subdirectory for `flake.nix flake.lock`.

## Check it worked

From the repository's top, paste this, with your scions directory for `scions`. It prints `ok` only if the
scions match their pins, graft-check has the ssh step, graft-upgrade is gone, the flake, the scions and the workflows are
committed, and `nix flake check` passes:

```sh
nix run .#graft check && grep -q 'name: SSH key for private scions' .forgejo/workflows/graft-check.yaml && test ! -e .forgejo/workflows/graft-upgrade.yaml && git diff --quiet HEAD -- flake.nix flake.lock scions .forgejo && nix flake check && echo ok
```

For a flake in `nix/workspace`:

```sh
nix run 'path:nix/workspace#graft' check && grep -q 'name: SSH key for private scions' .forgejo/workflows/graft-check.yaml && test ! -e .forgejo/workflows/graft-upgrade.yaml && git diff --quiet HEAD -- nix/workspace scions .forgejo && nix flake check 'path:nix/workspace' && echo ok
```

Then push a branch, and see graft-check pass on Forgejo.

## If something goes wrong

- **`error: resolving Git reference 'v1.0.0': revspec 'v1.0.0' not found`.** Read the lines above it. If
  ssh printed an error, such as `ssh: Could not resolve hostname …`, Nix could not reach the provider: make
  the `git ls-remote` in [Before you begin](#before-you-begin) work. If ssh printed nothing, Nix's git cache
  has not seen the tag: do step 1.
- **`graft: .forgejo/workflows/graft-check.yaml: its "# graft ssh:" line: can't read "…"`.** The line has a
  word that is not a field: often the key's comment (`root@server`), or the host pasted again before the
  key. Write `hostkey=TYPE KEY` only. `host= must be the host as known_hosts names it` means a port without
  brackets: write `[git.example.com]:2222`.
- **`graft: flake.lock re-pinned; nothing synced yet: run nix run .#graft again`.** The provider's tag is
  annotated, and graft moved the lock to the tag's commit
  ([Nix behaviours](../reference/nix-behaviours.md#annotated-tags-and-exportignore)). Run step 3 again.
- **In CI, `::error::the GRAFT_SSH_KEY secret is empty or not set`.** Add the secret of step 5 to this repository, with the
  name that `secret=` gives, if any.

## How it works

- graft-check runs in a new container with no ssh key. The ssh line is the only thing that gives it one.
  In CI, Nix fetches the provider by the commit in `flake.lock`, so step 1 is not needed there.
- The upgrade workflow gets no key, because it runs code that nobody has reviewed yet
  ([CI security](../ci-security.md#limits)).

  Not yet: graft-upgrade fails for a repository with a private scion, also for its other scions and for
  graft's own pin. Its jobs build graft's app, which fetches every scion.

  Until then, step 8 deletes graft-upgrade, and you upgrade on your machine, graft's own pin too: `nix run .#graft outdated`, then `nix run .#graft upgrade`. `upgrade` locks each new
  tag's commit itself, so step 1 is not needed again.

## Related

- [Keep private files out of the Nix store](flake-in-a-subdirectory.md), [Run graft in CI and review its upgrade pull requests](ci.md),
  [Files](../reference/files.md#the-graft-ssh-line).

## Sources

- [lib/workflows.sh](../../lib/workflows.sh): `ssh_parse`, `ssh_line`, `ssh_step` and `render_ssh`: the
  line's fields, its messages, and the step.
- [lib/common.sh](../../lib/common.sh): `relock_git`, which locks a tag's commit for `upgrade`.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): the app depends on every scion's fetch.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-check.yaml): the check step.
- [tests/run.sh](../../tests/run.sh): the `ssh:` checks, and "Annotated tags", a provider over HTTP.
- [tests/fixtures.sh](../../tests/fixtures.sh) `private`: `consumer/`, `notes-provider/` and
  `server-host-key.pub`, the repositories this page is read against.
