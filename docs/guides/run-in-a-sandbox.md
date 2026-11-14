---
status: working
---
# Run an agent or a command in a sandbox

Run an agent, or any command, so that it can change your repository but cannot read your home folder, reach
hosts that you did not name, or change the files that decide what it may do.

## Before you begin

- Linux, with unprivileged user namespaces (the default on NixOS, Debian, Ubuntu and Fedora). In Docker, see
  [If something goes wrong](#if-something-goes-wrong). Or macOS, where the launcher uses Seatbelt
  (untested on a Mac yet; see [On macOS](../reference/sandbox.md#on-macos) for what differs).
- graft v2.14.0 or later: `nix run .#graft outdated` prints graft's pinned version. Until v2.14.0 is
  released, use a graft checkout that has the sandbox: add `--override-input graft path:/path/to/graft` to
  each `nix run` and `nix flake lock` below.
- The steps use `team-kb`, a repository in your home folder with a flake at its top, and Claude Code as the
  agent. For a [flake in a subdirectory](flake-in-a-subdirectory.md), write
  `nix run 'path:nix/agent-tools#sandbox'` for `nix run .#sandbox`, from the repository's top.
- A sandbox protects your machine, not the repository's history: a command inside can delete commits that
  you have not pushed. Push first ([What the sandbox does not protect](../reference/sandbox.md#what-the-sandbox-does-not-protect)).

## Steps

1. Start a shell in the sandbox, from the repository's top:

   ```
   nix run .#sandbox
   ```

   The launcher prints one line, then starts `bash` with the prompt `(sandbox)`:

   ```
   graft: sandbox: ~/team-kb, 11 paths protected; network: none
   ```

   The number of protected paths depends on your repository. If a provider requests something, the line
   ends with `1 request not granted: sandbox --explain` (step 5).

2. Look around, then leave:

   ```
   ls -A ~
   git status --short
   exit
   ```

   `ls -A ~` prints only `team-kb`: your home folder is empty inside, except for the way to the repository.
   On macOS it prints nothing: `HOME` is a new, empty folder. `git status` works on the repository.

3. Add the agent's package, and what it needs, to `perSystem` in `flake.nix`. Claude Code is unfree, so the
   example takes it from a nixpkgs that allows unfree packages. The flake needs an `inputs.nixpkgs`:

   ```nix
   {
     inputs.self.submodules = true;
     inputs.graft.url = "github:danielnaab/graft/v2.14.0";
     inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
     outputs = inputs: inputs.graft.lib.mkFlake {
       inherit inputs;
       perSystem = { system, ... }: {
         sandbox = {
           packages = [ (import inputs.nixpkgs { inherit system; config.allowUnfree = true; }).claude-code ];
           paths.writable = [ "~/.claude" "~/.claude.json" ];
           network.allow = [ "*.anthropic.com" ];
         };
       };
     };
   }
   ```

   - `packages`: the commands that the sandbox can run. Only the closure is visible, so a program installed
     on the host outside Nix is not.
   - `paths.writable`: Claude Code keeps its login and settings in `~/.claude` and `~/.claude.json`. These
     also hold the transcripts of every project you use Claude Code in, so the agent can read them all. To
     give it none of that, leave out `paths.writable`, log in with a token
     (`env.pass = [ "CLAUDE_CODE_OAUTH_TOKEN" ];`), and accept that its settings are lost after each run.
   - `network.allow`: the hosts the agent may reach, through a proxy. `"*.anthropic.com"` means ports 443 and
     80 of every host in that domain.

4. Lock the new input, and stage `flake.nix`, so that the flake sees it:

   ```
   nix flake lock
   git add flake.nix flake.lock
   ```

5. Read the policy:

   ```
   nix run .#sandbox -- --explain
   ```

   Each value is listed with the file that set it: `graft` for graft's base, `flake.nix (perSystem)` for
   yours, `scions/NAME/graft.nix` for a provider's. If a provider requests something that you have not
   granted, a list near the end names it:

   ```
   requested, not granted (inert until perSystem's sandbox.grant names the scion)
     network.allow api.example.com           scions/notebook-kit/graft.nix
   ```

6. Grant a provider's requests only if you accept them. A provider can add packages to the sandbox, but its
   network, host paths and environment variables apply only when you name the scion in `grant`:

   ```nix
         sandbox = {
           # …the values of step 3…
           grant = [ "notebook-kit" ];
         };
   ```

   Stage `flake.nix`, and run `nix run .#sandbox -- --explain` again. The request is now in the policy, and
   no longer in the list of requests not granted. When an upgrade pull request moves a granted scion, its new
   requests apply too: read `--explain` on that branch.

7. Run the agent:

   ```
   nix run .#sandbox -- claude
   ```

   The line before the agent starts now names the writable paths and the network:

   ```
   graft: sandbox: ~/team-kb, 11 paths protected; also writable: ~/.claude, ~/.claude.json; network: *.anthropic.com
   ```

   A path that does not exist on your machine is skipped, and the line says so: `absent, skipped: ~/.claude.json`.

   In the dev shell (`nix develop`, or direnv), the launcher is on `PATH`: `sandbox claude`. The dev shell
   keeps the launcher that it had when you entered it: after you change `flake.nix`, leave the shell and enter
   it again.

8. After the run, read the launcher's last lines. If it warns that the run created git metadata (a line that
   starts with `graft: sandbox: WARNING:`), read each path that it names before you run git there
   ([What the sandbox does not protect](../reference/sandbox.md#what-the-sandbox-does-not-protect)).

9. To try a command for one run, add its nixpkgs package with `--with`:

   ```
   nix run .#sandbox -- --with ripgrep rg TODO
   ```

   The launcher builds `ripgrep` from the flake's nixpkgs, says that it added it, and runs `rg`. The version
   is your nixpkgs':

   ```
   graft: sandbox: with ripgrep 14.1.1 (the flake's nixpkgs), for this run
   ```

   For a command that you keep using, add it to `packages` in step 3. To let one run reach one more host,
   use `--allow HOST`.

## Check it worked

From the repository's top, paste this. It prints `ok` only if the sandbox can write the repository, cannot
write `flake.nix`, and sees an empty home folder (at most the folder on the way to the repository):

```sh
GRAFT_SANDBOX_QUIET=1 nix run .#sandbox -- bash -c \
  'touch .sandbox-check && ! touch flake.nix 2>/dev/null && test "$(ls -A ~ | wc -l)" -le 1' &&
  test -e .sandbox-check && rm -f .sandbox-check && echo ok
```

## If something goes wrong

- **In Docker, the launcher fails before the command starts,** with
  `graft: sandbox: bubblewrap can't create a user namespace here (above). …`. Docker's default seccomp profile
  blocks user namespaces. Run the command on the host or in a virtual machine. Or run the container with
  `--security-opt seccomp=unconfined --security-opt systempaths=unconfined`; both weaken the container. The
  launcher never runs the command without the sandbox.
- **The agent cannot connect, and after the run the launcher prints**
  `graft: sandbox: the proxy refused: HOST:PORT (N); network.allow lists the hosts a run may reach`. Add the
  host to `network.allow`, or, if a provider requested it, grant that provider. Only programs that use
  `HTTPS_PROXY` reach the network: `git push` over ssh fails in the sandbox. Push from outside.
- **`graft: sandbox: claude is not on the sandbox's PATH: add its package to sandbox.packages, or run with --with PKG`.**
  The command is not in the closure. Add its package to `packages`, stage `flake.nix`, and run again. A
  program installed on the host outside Nix is never visible.
- **`touch: cannot touch 'flake.nix': Read-only file system`** (or `Device or resource busy`, or the agent
  says that it cannot edit `flake.nix`, a `graft.nix`, `justfile` or `.git/config`). These are protected: the
  sandbox never writes what decides the sandbox or what the host runs. Edit them outside the sandbox.
- **`graft: sandbox: justfile is a symbolic link, so the sandbox can't make it read-only …`.** A protected
  path is a symbolic link, and the launcher stops. Replace the link with the file that it points to, or
  remove it. For `… has another hard link …`, give the file a copy of its own:
  `cp -p F F.copy && mv F.copy F`.

## Declare what your tools need (provider)

A provider's `graft.nix` can return `sandbox`, next to the tools that need it:

```nix
{ pkgs, ... }:
{
  tools   = { nb-index = …; };
  sandbox = {
    packages = [ pkgs.ripgrep ];                  # applies
    protect = [ "notebook/tasks/index.md" ];      # applies: read-only inside
    network.allow = [ "api.example.com" ];        # a request
  };
}
```

`packages`, `env.set` and `protect` apply to every consumer. `network.allow`, `paths.readOnly`,
`paths.writable` and `env.pass` are requests, and apply only for a consumer that grants them. Say in your
README which requests your tools need and why, so that consumers can decide. A new request in a release is a
change to what consumers must grant: say so in the CHANGELOG.

## How it works

The launcher is a program that Nix builds from the merged policy, so the closure, `PATH` and the `protect`
entries are fixed when it builds. At each run it finds the repository's git metadata and the paths that
`protect` matches, and starts the network proxy if the run has network. On Linux it starts bubblewrap with a
new set of namespaces, binds the closure read-only, the repository writable, and each protected path
read-only over it. On macOS it writes a Seatbelt profile that allows the same paths and denies writing the
protected ones, and starts the command with `sandbox-exec` ([Sandbox](../reference/sandbox.md)).

## Sources

- [Sandbox](../reference/sandbox.md): options, what a run sees, messages
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `sandbox` from `graft.nix` and `perSystem`, requests and grants
- [lib/sandbox/launcher.sh](../../lib/sandbox/launcher.sh): the launcher, its startup line and messages
- [tests/run.sh](../../tests/run.sh): the `sandbox` section
- [Decisions](../decisions.md#enforce-the-boundary-in-a-sandbox-not-in-rules): why a sandbox, and its costs
