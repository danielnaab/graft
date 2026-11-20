---
status: working
---
# Write a provider's graft.nix

Give every consumer your tools on their `PATH`, your checks in their `nix flake check`, and your apps in
their flake, with no Nix code of theirs: one `graft.nix` at the top of your repository.

## When to use this

- **A tool**, such as a checker or a CLI that agents run: `tools`.
- **A check** that runs on the consumer's files in `nix flake check`: `checks`, reading the consumer through
  `consumer`, and its settings from a file there.
- **Something to run once**, such as a setup command: `apps`.
- **A package that is not a command**, such as a bundle of plugins: `packages`.
- **Your tool uses another provider's tool or files**: `requires` and `scions`.
- A provider of rules and docs only needs no `graft.nix`.
- **Seed files**, which a consumer copies once and then owns, such as a starter configuration: no `graft.nix`.
  Tell consumers to copy them out of the scion without overwriting a copy they have:
  `cp -n scions/note-kit/seed/note-kit.conf .`. After an upgrade, they compare:
  `diff note-kit.conf scions/note-kit/seed/note-kit.conf`.

## Before you begin

- A provider: see [Publish a pattern](publish.md). The example is `note-kit`: a tool, `bin/note-lint`, and a
  check that lints a consumer's `notes/`, configured by `note-kit.yaml` in the consumer.
- A consumer to try it in, such as `team-notes`, that grafts note-kit
  ([Graft your first pattern](graft-a-repository.md)).
- The full contract (every argument and attribute, the collision rules): [Files](../reference/files.md#graftnix).

## Steps

1. At the top of your repository, write `graft.nix`:

   ```nix
   # graft.nix: called once per system by the consumer's graft mkFlake
   { pkgs, lib, scion, consumer, ... }:
   let
     note-lint = pkgs.writeShellApplication {
       name = "note-lint";
       runtimeInputs = [ pkgs.gnugrep pkgs.findutils pkgs.yq-go ];
       text = builtins.readFile ./bin/note-lint;
     };
   in
   {
     tools = { inherit note-lint; };

     checks.notes =
       if builtins.pathExists "${consumer}/note-kit.yaml"
       then pkgs.runCommand "notes" { nativeBuildInputs = [ note-lint ]; } ''
         cd ${consumer}
         note-lint --config note-kit.yaml notes
         touch $out
       ''
       else pkgs.runCommand "notes-skipped" { } "echo 'note-kit: no note-kit.yaml; skipped' > $out";
   }
   ```

   - `./bin/note-lint` resolves inside your scion's own store copy (`scion`), not in the consumer.
   - `writeShellApplication` puts `runtimeInputs` on the tool's `PATH`, and runs shellcheck on it when it
     builds.
   - The check reads the consumer through `consumer`, and **skips, not fails, when its settings file is
     absent**: a consumer that grafts note-kit for its tool alone keeps a passing `nix flake check`.
   - Take `...` in the arguments, so that a later graft version can add one.

2. Make the tool read its settings at run time, from the consumer's top:

   ```sh
   config="${NOTE_KIT_CONFIG:-$(git rev-parse --show-toplevel)/note-kit.yaml}"
   ```

   Consumer settings are files in the consumer, never Nix arguments: a consumer's `flake.nix` names no
   provider, so it has nowhere to pass them. Name the file and its fields in your README.

3. Name things for a shared namespace. Every provider's tools land on one `PATH`, and apps share one name
   space: `note-lint`, not `lint`. Checks are prefixed for you (`checks.<system>.note-kit-notes`). A clash is
   an evaluation error in the consumer that names both providers.

4. Try it in the consumer before you release. In the consumer, check out your working branch in the scion,
   and turn on `graft try`, so that Nix builds the scion's working tree:

   ```
   git -C scions/note-kit fetch origin my-branch
   git -C scions/note-kit checkout --detach FETCH_HEAD
   nix run .#graft -- try note-kit
   ```

   ```
   graft: try is on: Nix builds of this repository read its working tree, and its scions' (.graft-try marks it)
   graft: scions/note-kit: detached at 4c5d6e7, 2 commits after v1.0.0, not the recorded v1.0.0 (19392ed): no edits
   graft: build from the repository's top: nix build .#tools, nix shell .#tools -c TOOL, or nix flake check
   …
   ```

   A file you create in the scion is invisible to Nix until git tracks it there: `try` prints the
   `git -C scions/note-kit add -- FILE` to run. Then build with the command it prints:

   ```
   nix shell .#tools -c note-lint --help
   nix flake check
   ```

   `nix eval .#checks.x86_64-linux --apply builtins.attrNames` lists `"note-kit-notes"`. Why `try` is needed:
   [Fix a provider from a consumer](fix-a-provider.md). When done, turn it off and put the scion back:

   ```
   nix run .#graft -- try --stop
   git submodule update --recursive -- scions/note-kit
   ```

   For a flake in a subdirectory, `try` prints `'path:DIR#…'` commands and changes nothing: a `path:` flake
   reads its working tree on every build. If `nix run .#graft` fails (your `graft.nix` does not parse, say),
   run `nix run github:danielnaab/graft/v2.20.2 -- try -C .` instead.

5. Check the contract in your own flake ([below](#check-the-contract-before-you-tag)), and run
   `nix flake check` there.

6. Commit `graft.nix`, the tool and your README, then release
   ([Publish a pattern](publish.md)). Adding a `graft.nix` is a minor version. Removing a tool, renaming one,
   or changing a settings file's shape is a major version.

## Check the contract before you tag

A mistake in `graft.nix` reaches every consumer at their next upgrade, and it is loud:

- An evaluation error (a `throw`, a missing attribute, a package that does not exist) fails the consumer's
  `packages.<system>.tools`, its checks and `graft check`, naming your provider. The weekly upgrade's pull
  request then shows `[checks failing]`.
- A **parse error** (a missing `;` or brace) breaks the consumer's whole flake, its own outputs included: Nix
  cannot contain it. The consumer has to move your scion back by hand.

So evaluate `graft.nix` in your own flake, the way a consumer does, and assert what it gives. Use graft's
`mkFlake` with `perSystem`, and import your `graft.nix` with your repository as both `scion` and `consumer`:

```nix
{
  inputs.self.submodules = true;
  inputs.graft.url = "github:danielnaab/graft/v2.20.2";
  outputs = { self, graft, ... }@inputs: graft.lib.mkFlake {
    inherit inputs;
    perSystem = { pkgs, lib, system, scions, ... }:
      let own = import ./graft.nix { inherit pkgs lib system scions; scion = self; consumer = self; }; in
      {
        inherit (own) tools;
        checks = own.checks // {
          graft-nix =
            assert lib.assertMsg (lib.attrNames own.tools == [ "note-lint" ]) "graft.nix: tools are ${toString (lib.attrNames own.tools)}";
            pkgs.runCommand "graft-nix" { nativeBuildInputs = lib.attrValues own.tools; } ''
              note-lint --help > /dev/null
              touch $out
            '';
        };
      };
  };
}
```

`nix flake check` then parses `graft.nix`, builds every tool, runs your checks on your own repository, and
fails if the contract changed. knowledge-base-kit's `flake.nix` does this with its `graft-nix` check. Then
try the release candidate in one real consumer (step 4) before you tag.

## Use another provider

When your tool needs another provider (a checker that your tool calls, or its files), say so in `requires`,
and use it through `scions`:

```nix
{ pkgs, scions, ... }:
{
  requires = { knowledge-base-kit = "https://github.com/danielnaab/knowledge-base-kit"; };
  tools.nb-index = pkgs.writeShellApplication {
    name = "nb-index";
    runtimeInputs = [ scions.knowledge-base-kit.tools.kb-lint pkgs.python3 ];
    text = ''kb-lint "$@" && python3 ${scions.knowledge-base-kit}/src/index.py "$@"'';
  };
}
```

- `scions.NAME` is the other scion's store path (`"${scions.NAME}/src"`) merged with its `graft.nix` result
  (`scions.NAME.tools.kb-lint`).
- A consumer that does not graft it gets
  `graft: note-kit needs knowledge-base-kit, which this repository does not graft: graft add https://github.com/danielnaab/knowledge-base-kit`.
- `requires` has no version. If you start using something that only a newer release of the other provider
  has, release a new major version of yours, and say in its Upgrading steps which release of the other to
  take.
- Never read your own repository's scions from `graft.nix`: consumers' Nix sees them only for a git flake, and
  your consumers graft what they need themselves ([One level](../decisions.md#one-level)).

## Pin your own inputs

`pkgs` is the consumer's nixpkgs (or graft's). When your tool needs another version of something, pin it in
`graft.nix` with a fixed hash. Pure evaluation allows it, and the pin is part of your commit, so consumers
review it when they take your version:

```nix
{ system, ... }:
let
  nixpkgs-bun = import (builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/<rev>.tar.gz";
    sha256 = "<hash>";
  }) { inherit system; };
in
{ tools.bun = nixpkgs-bun.bun; }
```

`nix-prefetch-url --unpack URL` prints the hash. Your own `flake.nix` and `flake.lock` are for work on your
repository: consumers never evaluate them.

A tool that needs the network, a host path or an environment variable in a consumer's sandbox requests it in
`kit` ([pot](../reference/pot.md#schema), [Move a provider](move-from-the-v2-14-sandbox.md#move-a-provider)).

## Check it worked

In a consumer that grafts your release, from its top, with your tool and check names, paste this. It prints
`ok` only if the tool is in `tools`, the check is in the consumer's flake, and the checks pass:

```sh
nix shell '.#tools' -c sh -c 'command -v note-lint' >/dev/null && nix eval --json '.#checks.x86_64-linux' --apply builtins.attrNames | grep -q '"note-kit-notes"' && nix flake check && echo ok
```

Use your machine's system for `x86_64-linux`.

## If something goes wrong

- **`opening file '/nix/store/…-scion-note-kit/bin/note-lint': No such file or directory`.** The file is not
  committed, and, while you try it, git does not track it in the scion: `git -C scions/note-kit add -- FILE`
  (`graft try` prints it). A git flake sees only files that git tracks in the scion.
- **The consumer's check fails with `cannot execute: required file not found`.** It runs your script
  directly, and its `#!/usr/bin/env bash` names a path that Nix's build sandbox lacks. Run the built tool
  (`note-lint` from `nativeBuildInputs`), not `bin/note-lint`.
- **`graft: tools note-lint (from note-kit and other-kit): two scions provide the same name`.** Rename yours,
  in a new major version.
- **A consumer with no `note-kit.yaml` fails your check.** Skip when the file is absent (step 1).

## How it works

- The consumer's mkFlake calls `graft.nix` once per system, with `scion` copied alone into the store, so your
  tool's store path does not change when the consumer commits something unrelated.
- `tools` from every provider become one `packages.<system>.tools`, which consumers put on `PATH` with
  `nix shell .#tools`, `nix develop` or direnv. Providers ship no dev shells.
- **Kit commands.** A program your kit declares in `commands` is wrapped in consumers' tools, so pot runs it under
  its own policy ([Declare kit commands, recipes and hook commands](kit-commands.md)). Other providers' Nix uses
  the unwrapped `scions.<name>.tools.<tool>`.
- **A step after each upgrade.** If consumers must run something after an upgrade (regenerate files, say),
  give them a `just` recipe that runs `nix run .#graft upgrade` and then your app, and a check that fails when
  the generated files differ from what the new version renders. graft has no post-upgrade hook
  ([Scope](../decisions.md#scope-nix-git-submodules-a-tag-comparison-and-a-sandbox-policy)).

## Sources

- [lib/mk-flake.nix](../../lib/mk-flake.nix): the arguments, `requires`, the merge and collision rules, and
  prefixed checks.
- [tests/run.sh](../../tests/run.sh): the providers `kb` (a tool, a check reading a settings file),
  `nk` (requires kb), `mk` (a colliding tool), `same` (a deduplicated tool) and `lk` (a fetchTarball pin).
- [tests/fixtures.sh](../../tests/fixtures.sh) `publish`: the repositories this page was read against.
- [Files](../reference/files.md#graftnix).
