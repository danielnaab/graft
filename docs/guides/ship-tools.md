---
status: working
---
# Ship a tool or seed files with a pattern

Give consumers a tool that they build from your export, and files that they copy once and then own.

## When to use this

- **A tool**, such as a checker that consumers run in their own checks: steps 1 to 3, then 6 and 8. Its source is part of
  your export, and consumers build it from the same input, so they need no second input for your
  repository.
- **Seed files**, which a consumer copies once and then owns, such as a starter configuration. A seed file
  is not a scion: graft replaces a scion, and would remove the consumer's edits.
  - One or two files: export them, and tell consumers to copy them out of the scion once, without
    overwriting a copy they have: `cp -n scions/note-kit/seed/note-kit.conf .`. The scion keeps your current
    version. After an upgrade, a consumer compares it with their own copy:
    `diff note-kit.conf scions/note-kit/seed/note-kit.conf`.
  - A starter that `nix flake init` writes: steps 4 to 8.
  - Files that consumers edit, and that should still take your later changes: use a template tool such as
    Copier, which merges your changes into their edits. See the README's
    [comparison](../../README.md#compared-with-copying-submodules-templates-and-bots).

## Before you begin

- A provider: see [Publish a pattern](publish.md). The example is `note-kit`: a tool, `bin/note-lint`, and a
  seed file, `seed/note-kit.conf`. `team-notes` grafts it and runs the tool in its check.
- Nix with flakes. Step 2 fetches nixpkgs from your flake registry.

## Steps

1. Beside the tool, write a Nix expression that builds it with what it needs. note-kit's `package.nix`:

   ```nix
   { writeShellApplication, gnused, gnugrep, findutils }:
   writeShellApplication {
     name = "note-lint";
     runtimeInputs = [ gnused gnugrep findutils ];
     text = builtins.readFile ./bin/note-lint;
   }
   ```

   `writeShellApplication` puts `runtimeInputs` on the tool's `PATH`, starts the script with
   `set -o errexit -o nounset -o pipefail`, and runs shellcheck on it when it builds. For another language,
   write any expression that `callPackage` can build.

2. Build it from your repository's top:

   ```
   nix build --no-link --print-out-paths --impure --expr '(builtins.getFlake "nixpkgs").legacyPackages.${builtins.currentSystem}.callPackage ./package.nix { }'
   ```

   It prints the tool's store path, such as `/nix/store/…-note-lint`.

3. In your README, say how consumers build the tool. note-kit's README says:

   ```nix
   note-lint = pkgs.callPackage "${inputs.note-kit}/package.nix" { };
   ```

   `${inputs.note-kit}` is the consumer's fetched export of your repository. `pkgs` is the consumer's
   nixpkgs, such as `nixpkgs.legacyPackages.${system}`, or graft's:
   `inputs.graft.inputs.nixpkgs.legacyPackages.${system}`. The consumer then runs
   `${note-lint}/bin/note-lint` in a check. [Make a pattern's checks gate your merges](gate-merges.md) shows
   a whole check, and says why it reads `${inputs.<name>}` and not `scions/<name>`.

4. Put the files that consumers copy once in a directory of their own, such as `seed/`.

5. Give your `flake.nix` a template for that directory:

   ```nix
   {
     outputs = { self }: {
       templates.default = { path = ./seed; description = "note-kit's settings, to copy once"; };
     };
   }
   ```

   Keep `flake.nix` and the directory in your export: a `github:` URL fetches your export.

6. Commit what steps 1 to 5 added, with your README:

   ```
   git add package.nix flake.nix seed/ README.md
   git commit -m "Ship note-lint's package.nix, and seed/ as a flake template"
   ```

   In note-kit they are committed already, so `git status --short` shows only the next release's work.

7. Try the template. In an empty git repository, run this with your checkout's absolute path:

   ```
   nix flake init -t git+file:///ABSOLUTE/PATH/TO/note-kit
   ```

   It prints `wrote: …/note-kit.conf` and marks the file to add: `git status --short` shows
   ` A note-kit.conf`. In your README, give consumers the same URL that they graft you with, at a tag:
   `nix flake init -t github:OWNER/note-kit/v1.2.0`, or for a `git+` URL, the tag's full name:
   `nix flake init -t "git+https://forge.example/team/note-kit?ref=refs/tags/v1.2.0"`.

8. Release, as in [Publish a pattern](publish.md).

## Check it worked

From your repository's top, with your tool's name for `note-lint`, paste this. It copies the export at
`HEAD` (what you committed, not your working tree) to a temporary directory. It prints `ok` only if the tool
builds from that copy, and the copy holds `flake.nix` and a non-empty `seed/`, and its flake has the template:

```sh
d=$(mktemp -d) && git archive HEAD | tar -x -C "$d" && out=$(nix build --no-link --print-out-paths --impure --argstr dir "$d" --expr '{ dir }: (builtins.getFlake "nixpkgs").legacyPackages.${builtins.currentSystem}.callPackage (/. + dir + "/package.nix") { }') && test -x "$out/bin/note-lint" && test -f "$d/flake.nix" && test -n "$(ls -A "$d/seed")" && nix eval --raw "path:$d#templates.default.path" >/dev/null && echo ok
```

## If something goes wrong

- **A consumer's check fails with `…/bin/note-lint: cannot execute: required file not found`.** The check
  runs your script directly, and its `#!/usr/bin/env bash` line names a path that Nix's build sandbox does
  not have. Tell consumers to build the tool with your `package.nix` (step 3), not to run `bin/note-lint`.
- **The build fails with a shellcheck message, such as `SC2086 (info): Double quote to prevent globbing and
  word splitting.`** `writeShellApplication` refuses a script that shellcheck warns about. Fix the script, or
  add a `# shellcheck disable=SC2086` line above the line it names.
- **`nix flake init` prints `refusing to overwrite existing file '…/note-kit.conf'` and
  `error: Encountered 1 conflicts - see above`.** The repository already has the file, which is its own.
  `nix flake init` changes nothing. To take your newer version, the consumer merges it by hand with the store
  path that the message names.
- **`nix flake init -t "git+…?ref=v1.2.0"` fails with `revspec 'v1.2.0' not found`.** On Nix 2.24, a `git+`
  URL to a remote or bare repository needs the tag's full name, `ref=refs/tags/v1.2.0`, until Nix's git cache
  has seen the tag ([Nix behaviours](../reference/nix-behaviours.md)).

## How it works

- Consumers graft you with `flake = false`. Your flake's `packages` and `apps` do not reach them through
  that input, so they build your tool from your export with their own nixpkgs.
- If your tool reads a setting from the consumer's repository, name the file and the setting in your README
  ([Change a grafted file](change-a-grafted-file.md)).
- **A step after each upgrade.** If consumers must run a step after each upgrade, such as regenerating files
  from your templates, give them a recipe that runs both, for example in their `justfile`, where `render` is
  their own app that runs your tool:

  ```just
  # Take new releases, then regenerate what note-kit renders
  upgrade:
      nix run .#graft upgrade
      nix run .#render
  ```

  graft has no post-upgrade hook ([Scope](../decisions.md#scope-nix-a-copy-a-diff-and-a-tag-comparison)),
  and the weekly upgrade in CI does not run the recipe. So also give consumers a check that fails when the
  regenerated files differ from what the new version renders: it renders into the build directory and
  compares, for example `render --out rendered && diff -r rendered ${self}/generated && touch $out` in a
  `runCommand`. The upgrade pull request then fails that check, and a person runs the recipe on its branch.

## Sources

- [tests/fixtures.sh](../../tests/fixtures.sh) `publish`: note-kit's `package.nix`, seed file and template,
  and team-notes' check that builds the tool from the fetched export.
- [lib/mk-flake.nix](../../lib/mk-flake.nix): `scions` (which inputs are scions: `flake = false`), and the
  merge of a repository's own `outputs`, such as its checks.
- [Concepts](../concepts.md) (the fetched export), [Decisions](../decisions.md) (where graft v1's
  `commands:` went).
