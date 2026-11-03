# graft

**This is graft v2, a new design. It does not read `graft.yaml`, `graft.lock` or `.graft/`;
those belong to graft v1, which stays on `main` until v2 replaces it and is then tagged `legacy`.**

graft copies other repositories' content into yours at a pinned commit, so agents and people
read exactly the same text on every machine, with nothing to fetch or set up. The provider
decides what it exports; the consumer decides which version it takes.

It is a Nix flake library of about 100 lines. Nix does the real work:

| Job | Done by |
|---|---|
| What a repo exports | the provider's `.gitattributes`: paths marked `export-ignore` stay out |
| Fetching and pinning | flake inputs: `flake.lock` records the commit and a content hash |
| Applying the export rules | Nix, for `github:` inputs and for `git+…?exportIgnore=1` inputs |
| One copy of a shared dependency | `follows`, if two inputs share one |
| Upgrading | `nix flake update <name>` |
| Copying into the repo | graft: `nix run .#graft` |
| Checking the copy | graft: `nix flake check` |

## Use it

```nix
# flake.nix
{
  inputs.graft.url = "github:danielnaab/graft/v2";
  inputs.meta-knowledge-base = {
    url = "github:danielnaab/meta-knowledge-base/v1.0.0";   # github: applies export-ignore
    flake = false;
  };
  # a private Forgejo repo: git+ssh, and exportIgnore=1 or its export rules are skipped
  # inputs.notes = { url = "git+ssh://forgejo@platform-vm:2222/daniel/notes?ref=v1&exportIgnore=1"; flake = false; };

  outputs = inputs: inputs.graft.lib.mkGraft {
    inherit inputs;
    deps = [ "meta-knowledge-base" ];   # each becomes vendor/<name>/
    # dir = "vendor";                   # the default
  };
}
```

Then `nix run .#graft` and commit `vendor/`. `vendor/README.md` lists each folder's pinned commit.
`nix flake check` fails if `vendor/` differs from what the pins produce: an edited vendored file,
a pin moved without re-running graft, or a vendored file your `.gitignore` drops.

To upgrade one dependency: change its ref if you pinned a tag, then `nix flake update <name>`,
`nix run .#graft`, and commit. The diff of `vendor/` is the change you are taking.

A flake with outputs of its own merges them with `nixpkgs.lib.recursiveUpdate`, not `//`, which
would drop one side's `checks`.

## As a provider

Mark what isn't part of your export in `.gitattributes`, and tag releases:

```
tests/   export-ignore
notes/   export-ignore
vendor/  export-ignore   # always: consumers take your content, never your dependencies
```

Your own dependencies stay yours. A consumer that needs one of them grafts it directly, so every
repo has one level of `vendor/` and one copy of each dependency.

## Scope

graft stays this small. It has no config file, no lock of its own, no CLI beyond `nix run`, and
no update bot. A feature that can't be expressed as Nix plus a copy and a diff does not belong here.

## Develop

`tests/run.sh` runs the whole cycle against a throwaway provider repo: graft, check, drift, upgrade.
`nix flake check` runs shellcheck on it.
