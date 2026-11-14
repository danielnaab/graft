---
status: working
---
# Graft a private repository

Graft a provider that only an ssh key can read, on your machine and in graft's CI workflows.

## Before you begin

- You can read the provider over ssh. This prints a hash and `refs/tags/v1.0.0`:

  ```
  git ls-remote ssh://git@git.example.com:2222/team/notes refs/tags/v1.0.0
  ```

  git and Nix fetch the provider with your own ssh setup: your key, your agent and `~/.ssh/config`.
- Who can read what:
  - Your repository records only the provider's URL and a commit. People who can read your repository but not
    the provider see neither its files nor its history.
  - Nix copies the provider into the Nix store, which every user of your machine can read. Keep your own
    private files out of the store with [Keep private files out of the Nix store](flake-in-a-subdirectory.md).
- Use an ssh URL. In CI, graft's workflows give git and Nix an ssh key; an `https` URL to a private repository
  gets no credentials there.
- [One level](../decisions.md#one-level): a provider that others graft must not graft a private repository
  that its consumers cannot read. Nix fetches nested scions, and their evaluation would fail.
- Your repository grafts with graft ([Graft your first pattern](graft-a-repository.md)), and keeps graft's
  workflows in `.forgejo/workflows/`. For a flake in a subdirectory, write `nix run 'path:DIR#graft'` for
  `nix run .#graft`.
- You need admin rights on the provider's repository, to add a deploy key, and a trusted copy of the
  server's host key: a shell on the server, or its admin.
- The example's host is `git.example.com`, its ssh port `2222`, and the provider `team/notes`. Use yours.

## Steps

1. Graft it with its ssh URL:

   ```
   nix run .#graft add ssh://git@git.example.com:2222/team/notes.git
   ```

   ```
   graft: scions/notes <- v1.0.0 (abc1234)
   ```

   Write the absolute `ssh://` form. A relative URL (`../notes.git`) is refused: Nix cannot resolve it.

2. Commit, and build:

   ```
   git commit -m "Graft notes v1.0.0, a private provider"
   nix flake check
   ```

   Nix fetches the scion over ssh at the committed gitlink.

3. Make a key pair for CI, in a new directory outside the repository, so that no commit can take it:

   ```
   keys=$(mktemp -d) && ssh-keygen -t ed25519 -N '' -C "graft@YOUR-REPO" -f "$keys/graft_ssh_key" && echo "$keys"
   ```

   Do step 4 in the same shell, or write the printed directory for `$keys`.

4. Give the key pair to the forge, then delete it:

   - On the provider's repository, add the content of `"$keys/graft_ssh_key.pub"` as a **read-only** deploy
     key (Forgejo: Settings, Deploy keys).
   - On your repository, add the content of `"$keys/graft_ssh_key"` as the Actions secret `GRAFT_SSH_KEY`
     (Forgejo: Settings, Actions, Secrets).
   - Delete both key files: `rm -r "$keys"`.

5. Get the host's public keys:

   ```
   ssh-keyscan -p 2222 git.example.com 2>/dev/null
   ```

   It prints one line per key, such as `[git.example.com]:2222 ssh-rsa AAAA…`, and comment lines starting
   with `#`, which you skip. Compare the key with the server's key file: for Forgejo's built-in ssh server,
   `data/ssh/gitea.rsa.pub`; for OpenSSH, a file such as `/etc/ssh/ssh_host_ed25519_key.pub`.

6. Add one line to `.forgejo/workflows/graft-check.yaml` and to `.forgejo/workflows/graft-upgrade.yaml`,
   anywhere. `host=` is the first field of step 5's line, and `hostkey=` the other two:

   ```yaml
   # graft ssh: host=[git.example.com]:2222 hostkey=ssh-rsa AAAA…
   ```

   Add `address=IP` if CI's runner cannot resolve the host's name, and `secret=NAME` if the secret is not
   `GRAFT_SSH_KEY` ([Files](../reference/files.md#the-graft-ssh-line)).

7. Let graft write the steps that the line asks for:

   ```
   nix run .#graft sync
   ```

   It prints `graft: refreshed .forgejo/workflows/graft-check.yaml` and the same for `graft-upgrade.yaml`.
   Each now has a step, `SSH key for private scions`, before each step that runs graft. It writes the secret to
   `~/.ssh/graft_scions`, the host key to `~/.ssh/known_hosts`, and a `Host` entry with
   `StrictHostKeyChecking yes` to `~/.ssh/config`, which git and Nix both use.

8. Commit the workflows:

   ```
   git add .forgejo/workflows
   git commit -m "CI: ssh key for the private scion"
   ```

## Check it worked

From the repository's top, paste this. It prints `ok` only if the scion is clean and at a release, both
workflows have the ssh step, everything is committed, and `nix flake check` passes:

```sh
nix run '.#graft' check && grep -q 'name: SSH key for private scions' .forgejo/workflows/graft-check.yaml && grep -q 'name: SSH key for private scions' .forgejo/workflows/graft-upgrade.yaml && test -z "$(git status --porcelain)" && nix flake check && echo ok
```

Then push a branch, and see graft-check pass on Forgejo.

Not yet: fetching a private ssh scion in CI, by git and by Nix inside the `nixos/nix` container, is untested
against a live forge.

## If something goes wrong

- **`error: Failed to fetch git repository 'ssh://…'`.** Read the lines above it. If ssh printed an error,
  such as `ssh: Could not resolve hostname …` or `Permission denied (publickey)`, make the `git ls-remote` in
  [Before you begin](#before-you-begin) work. In CI: the deploy key or the `# graft ssh:` line.
- **`graft: .forgejo/workflows/graft-check.yaml: its "# graft ssh:" line: can't read "…"`.** The line has a
  word that is not a field: often the key's comment (`root@server`). Write `hostkey=TYPE KEY` only.
  `host= must be the host as known_hosts names it` means a port without brackets: write
  `[git.example.com]:2222`.
- **In CI, `::error::the GRAFT_SSH_KEY secret is empty or not set`.** Add the secret of step 4 to this
  repository, with the name that `secret=` gives, if any.
- **A fresh clone's `git submodule update --init --recursive` fails for this scion only.** The machine has no key for it.
  Fill the others by name (`git submodule update --init --recursive -- scions/OTHER`); builds of a clean tree still fetch
  it through Nix, with your ssh setup.

## How it works

- The upgrade workflow's upgrade and pr steps hold the deploy key too: the upgrade's `nix flake check` must
  fetch the private scion at its new tag, and the pr step's graft app fetches it at the default branch's commit. The key is read-only, for the provider only, and the code that job runs unreviewed is that
  provider's own new version, which the key can already read ([CI security](../ci-security.md#limits)).
- Nix fetches scions with the system's git and ssh, so `~/.ssh/config` and your agent apply.

## Related

- [Keep private files out of the Nix store](flake-in-a-subdirectory.md), [Run graft in CI and review its upgrade pull requests](ci.md),
  [Files](../reference/files.md#the-graft-ssh-line).

## Sources

- [lib/workflows.sh](../../lib/workflows.sh): `ssh_parse`, `ssh_line`, `ssh_step` and `render_ssh`: the
  line's fields, its messages, and the steps.
- [lib/tools.sh](../../lib/tools.sh): `add` refuses a relative URL.
- [template/.forgejo/workflows/](../../template/.forgejo/workflows/graft-check.yaml): the checkout and ssh steps.
- [tests/run.sh](../../tests/run.sh): the `ssh:` checks.
- [tests/fixtures.sh](../../tests/fixtures.sh) `private`: the repositories this page is read against.
