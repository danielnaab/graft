---
status: working
---
# Private providers

Graft a private repository over ssh, on your machine and in CI.

## Pin it

Use a `git+ssh` URL with `exportIgnore=1`:

```nix
notes = { url = "git+ssh://forgejo@platform-vm:2222/daniel/notes?ref=v1.0.0&exportIgnore=1"; flake = false; };
```

- Without `exportIgnore=1`, Nix does not apply the provider's export rules, and the scion gets the whole
  tree.
- The provider must write its directory rules without a trailing slash too. See
  [Publish](publish.md#decide-what-you-export).
- If the provider uses annotated tags, graft keeps the lock at the tag's commit. If `nix run .#graft`
  says that it re-pinned `flake.lock`, run it again. The reason is in
  [Nix behaviours](../reference/nix-behaviours.md#annotated-tags-and-exportignore).

## Check in CI

graft-check runs in a new container with no ssh key, so it cannot fetch a private scion. One comment line
in `graft-check.yaml` makes graft write a step that installs a key.

1. Make a key pair: `ssh-keygen -t ed25519 -N '' -C "graft-check@<repo>" -f graft_ssh_key`.
2. On the provider's repository, add `graft_ssh_key.pub` as a read-only deploy key (Settings, Deploy
   keys).
3. On the consumer's repository, add the content of `graft_ssh_key` as the Actions secret
   `GRAFT_SSH_KEY` (Settings, Actions, Secrets).
4. Delete both key files.
5. Get the host's public key. For Forgejo's built-in ssh server, it is in `data/ssh/gitea.rsa.pub`. You
   can also run `ssh-keyscan -p 2222 platform-vm`, and compare the result with that file.
6. Add the line to `.forgejo/workflows/graft-check.yaml`:

   ```yaml
   # graft ssh: host=[platform-vm]:2222 address=192.168.1.51 hostkey=ssh-rsa AAAAB3NzaC1yc2E...
   ```

7. Run `nix run .#graft`. This writes the step.
8. Commit.

The fields of the line are in [Files](../reference/files.md#the-graft-ssh-line). The upgrade workflow
does not get the key ([CI security](../ci-security.md#limits)). Upgrade a private scion with
`nix run .#graft upgrade`, or from a runner that has the key.

Related: [Run graft in CI and review its upgrade pull requests](ci.md), [Graft your first pattern](graft-a-repository.md).
