---
status: working
---
# Declare kit commands, recipes and hook commands

A provider's kit declares the programs it ships as `commands`. From graft v2.18.0, each consumer's tools hold a
**wrapper** of the same name, so `pot` runs your command under its own policy: on the host, from `just`, from a git
hook, and nested inside an agent's run. The wrapped command gets only what it requests, never the agent's token or
MCP. This page is for a provider; the consumer's side is [Move to v2.18.0](move-to-v2-18.md).

## Before you begin

- Your provider has a `graft.nix` with `tools` ([Write a provider's graft.nix](write-graft-nix.md)).
- `recipes` in the attrs form and `hook` are kit 1.2. pot reads 1.2 from v2.17.0, so write it once your consumers'
  roots are on v2.17.0 or later. Wrappers appear once a consumer takes graft v2.18.0; under an older root they run
  the program unwrapped, with one line on stderr ([Wrapped commands](../reference/pot.md#wrapped-commands)).

## Steps

1. Declare each program as a command, with the least `access` it needs and no `requests` unless it reaches
   something outside the workspace and the store. A kit with no requests needs no grant: it runs with no prompt.

   ```nix
   # graft.nix (notebook-kit's, trimmed)
   kit = {
     version = "1.2";
     commands = {
       check-kb = { program = lib.getExe scripts.check-kb; access = "read";
                    description = "Check links, lifecycle status, and Sources"; };
       nb-index = { program = lib.getExe scripts.nb-index; access = "write"; };
       nb-pre-commit = { program = lib.getExe preCommit; access = "read"; hook = "pre-commit";
                         description = "markdownlint on staged Markdown, then check-kb"; };
     };
     recipes = {
       check-kb = { run = "check-kb"; description = "Check links, lifecycle status, and Sources"; };
       index = { run = "nb-index"; description = "Rebuild the indexes"; };
     };
   };
   ```

   `program` is a file in the store (`lib.getExe drv`), not a package. A recipe's `run` starts with one of the
   kit's own commands; the rest are literal words (letters, digits and `. _ / = : , @ + % -`, one space apart). A
   recipe's name is letters, digits, `_` and `-`, starting with a letter.

2. Make a hook command call other programs by store path, not through `just` or another wrapper, so one commit
   is one sandbox start: put them in the hook script's `runtimeInputs`.

3. Try it in a consumer: `graft try` in the consumer, then `nix run .#graft sync` there. Check:
   - `head -3 "$(nix build --no-link --print-out-paths .#tools)/bin/check-kb"` is a wrapper (a comment names pot-kits/1);
   - `just --list` shows the generated recipes, from `<scions>/kits.just`;
   - `pot explain claude` lists your commands on its `children` row as `nest`.

4. Release a minor version: a new field, nothing removed.

## Keep older consumers working

- Keep your hand-written `.just` file and hook script while consumers on older graft use them. A consumer on
  v2.18.0 imports it first and `kits.just` after; with `set allow-duplicate-recipes`, the generated recipes win.
- Another provider's Nix uses your unwrapped tools, `scions.<name>.tools.<tool>`, as before. Never call a
  wrapper from a Nix build or a check.

## Check it worked

- On the host, `check-kb` in the consumer runs quietly; `pot runs --all` lists it (plain `pot runs` hides wrapped
  runs that exited 0).
- Inside `pot claude`, the same command nests: it cannot reach the agent's hosts.
- `git commit` in the consumer runs your hook command.

## If something goes wrong

- **`pot: FILE: recipes.N: runs X, which is not one of this kit's commands (A, B)`.** Declare `X` as a command,
  or point the recipe at one.
- **`pot: FILE: recipes.N: literal words only (letters, digits and . _ / = : , @ + % -, one space apart)`.** A recipe passes the caller's
  arguments itself (`"$@"`); write only fixed words.
- **`pot: FILE: recipes.N: a recipe's name is letters, digits, _ and - (starting with a letter)`.** Rename it.
- **`graft: scions/kits.just's recipe X has the name of the justfile's own X: …`** (in a consumer). A kit's recipe
  never replaces the consumer's own; one of the two is renamed.
- **`… needs X, which this run (claude) lacks: run it on the host (just RECIPE)`.** Your command requests reach
  the agent's run lacks. Drop the request if it is not needed; otherwise that command runs on the host only.
- **`pot: kitsFor: two recipes are named X (FILE and FILE)`,** or `two kits ship a command named X (self and
  scions.a): a wrapper has one name`. Two kits in the consumer use one name. Rename yours.
- **`pot: kitsFor: a command name is letters, digits, . _ and - (X)`.** Rename the command.

## Sources

- [kit/1](../../pot/contracts/kit-1.md): `commands`, `recipes`, `hook`
- [pot-kits/1](../../pot/contracts/pot-kits-1.md): the wrapper, `kits.just`, the manifests
- [pot/lib/kits-for.nix](../../pot/lib/kits-for.nix): what kitsFor builds
- [pot's reference: Wrapped commands](../reference/pot.md#wrapped-commands)
