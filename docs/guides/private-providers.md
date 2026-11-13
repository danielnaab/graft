---
status: draft
---
# Graft a private repository

Graft a provider that only an ssh key can read, on your machine and in graft-check.

## Before you begin

- You can read the provider over ssh. This prints the tag's commit:

  ```
  git ls-remote ssh://git@git.example.com:2222/team/notes refs/tags/v1.0.0
  ```

  Nix fetches the provider with your own ssh setup: your key, your agent and `~/.ssh/config`.
- The provider's `.gitattributes` writes its directory rules without a trailing slash (`tests export-ignore`,
  not `tests/ export-ignore`). Over git, Nix ignores the slash form ([Publish](publish.md#decide-what-you-export)).
- Your repository already grafts with graft ([Graft your first pattern](graft-a-repository.md)), and keeps
  graft's `graft-check.yaml` in `.forgejo/workflows/`. If your flake is in a subdirectory, write
  `nix run path:DIR#graft` for `nix run .#graft`, and skip `nix flake check`
  ([Keep private files out of the Nix store](flake-in-a-subdirectory.md)).
- The example's host is `git.example.com`, its ssh port `2222`, and the provider `team/notes`. Use yours.

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

   Nix prints `warning: updating lock file` and `• Added input 'notes'`. graft prints a line for each
   scion, among them `graft: scions/notes <- COMMIT`, with the commit from step 1.

4. Make a key pair for CI:

   ```
   ssh-keygen -t ed25519 -N '' -C "graft-check@YOUR-REPO" -f graft_ssh_key
   ```

5. On the provider's repository, add the content of `graft_ssh_key.pub` as a read-only deploy key
   (Forgejo: Settings, Deploy keys).

6. On your repository, add the content of `graft_ssh_key` as the Actions secret `GRAFT_SSH_KEY`
   (Forgejo: Settings, Actions, Secrets). Then delete `graft_ssh_key` and `graft_ssh_key.pub`.

7. Get the host's public key:

   ```
   ssh-keyscan -p 2222 git.example.com 2>/dev/null
   ```

   It prints one line per key, such as `[git.example.com]:2222 ssh-rsa AAAA…`. Compare the key with one
   that you trust: for Forgejo's built-in ssh server, `data/ssh/gitea.rsa.pub` on the server.

8. Add one line to `.forgejo/workflows/graft-check.yaml`, anywhere. `host=` is the first field of step 7's
   line, and `hostkey=` the other two:

   ```yaml
   # graft ssh: host=[git.example.com]:2222 hostkey=ssh-rsa AAAA…
   ```

   Add `address=IP` if CI's runner cannot resolve the host's name, and `secret=NAME` if the secret is not
   `GRAFT_SSH_KEY` ([Files](../reference/files.md#the-graft-ssh-line)).

9. Let graft write the step that the line asks for:

   ```
   nix run .#graft
   ```

   After the scions' lines, it prints `graft: refreshed .forgejo/workflows/graft-check.yaml`. The file now
   has a step, `SSH key for private scions`, before the check. It writes the secret to
   `~/.ssh/graft_scions`, the host key to `~/.ssh/known_hosts`, and a `Host` entry with
   `StrictHostKeyChecking yes` to `~/.ssh/config`. graft moved the line above `name:`.

10. Commit:

    ```
    git add -A
    git commit -m "Graft notes v1.0.0, a private provider"
    ```

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scions match their pins, graft-check has
the ssh step, nothing is left to commit, and `nix flake check` passes:

```sh
nix run .#graft check && grep -q 'name: SSH key for private scions' .forgejo/workflows/graft-check.yaml && test -z "$(git status --porcelain)" && nix flake check && echo ok
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
- **In CI, `::error::the GRAFT_SSH_KEY secret is empty or not set`.** Do step 6 on this repository, with the
  name that `secret=` gives, if any.

## How it works

- graft-check runs in a new container with no ssh key. The ssh line is the only thing that gives it one.
- In CI, Nix fetches the provider by the commit in `flake.lock`, so step 1 is not needed there.
- The upgrade workflow gets no key, because it runs code that nobody has reviewed yet
  ([CI security](../ci-security.md#limits)). Upgrade a private scion on your machine:
  `nix run .#graft upgrade notes`. It locks the new tag's commit itself, so step 1 is not needed again.

  Not yet: graft-upgrade fails for a repository with a private scion, also for its other scions and for
  graft's own pin. Its jobs build graft's app, which fetches every scion.

## Related

- [Keep private files out of the Nix store](flake-in-a-subdirectory.md), [CI](ci.md),
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
