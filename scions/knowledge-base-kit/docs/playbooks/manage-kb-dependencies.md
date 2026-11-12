---
status: stable
---
# Managing knowledge base dependencies

Guidance for knowledge bases that import or depend on others.

## Take a dependency

Graft it: the dependency's export, at a pinned commit, committed under `scions/<name>/` (a *scion* is what's grafted on). Every
clone, worktree, agent and the forge's web view then reads the same text with nothing to set up.
See [graft](https://github.com/danielnaab/graft/tree/v2) for the flake inputs; in short:

```nix
inputs.knowledge-base-kit = { url = "github:danielnaab/knowledge-base-kit/v2.1.0"; flake = false; };
outputs = inputs: inputs.graft.lib.mkFlake { inherit inputs; };
```

`nix run .#graft` writes `scions/`; `nix flake check` fails if it drifts from `flake.lock`.
Never edit `scions/`: it is overwritten on the next graft.

Then say what the dependency is to you, in knowledge-base.yaml:

```yaml
imports:
  - kind: local
    path: scions/knowledge-base-kit
    relationship: "foundational"

  - kind: local
    path: scions/project-specs
    relationship: "canonical-source"
    sync_policy: "interpret"
```

## Provide one

What you export is your decision, made in `.gitattributes`: paths marked `export-ignore` stay
out of what others graft. Always exclude your own `scions/`: consumers take your content, never
your dependencies, so every repo has one level of scions and one copy of each dependency.
Tag releases (`v1.0.0`) so consumers pin something readable, and say what changed in a CHANGELOG.

Exported files link only to exported files ([Linking policy](../policies/linking.md#exported-files)).

### Dependencies are between exports

A dependency is a design edge only when what you export depends on what another repo exports: your docs build on
its content, or your code implements its format. Keep those edges free of cycles. Two other kinds of edge are not
design dependencies, and may point back at a repo that depends on you:

- **Conformance:** every knowledge base follows this kit's conventions for how it is written, this kit included.
  When the conventions change you may edit your docs to comply, but nothing you export changes in substance.
- **Upkeep:** a scion you graft only to maintain your own repo (its AGENTS.md links into it), never re-exported.

A cycle of design edges, or a dependency that starts defining what the other repo owns, is the smell to act on.
Each rule has one owner; a repo that enforces another's rule cites it rather than restating it.

## Relationship types

**Foundational** - provides patterns you build on
- Example: knowledge-base-kit
- Sync: review when major updates occur

**Canonical-source** - source of truth you interpret/implement
- Example: specification KB for implementation KB
- Sync: interpret changes, maintain provenance
- Track drift explicitly

**Peer** - related project, occasional references
- Sync: standard links, no special maintenance

**Generated-from** - your KB derived from another source
- Sync: automated regeneration, mark generated content

## Sync policies

**Interpret policy** (for canonical-source):
- Add Sources sections referencing canonical KB
- Mark interpretation docs with authority notes
- Review when canonical KB changes
- Track drift in sync status section

**Extend policy** (for foundational):
- Mark inherited vs added content
- The pinned version is in `flake.lock` and `scions/README.md`
- Update when foundation changes: `nix run .#graft changelog`, then `nix run .#graft upgrade`, then review
  the diff of `scions/`

**Reference policy** (for peer):
- Standard markdown links
- No special synchronization needed

## Cross-repository provenance

Link into the grafted copy with relative paths, so links resolve at the pinned version:
```markdown
[canonical architecture](../scions/canonical-kb/docs/architecture.md)
```

Declare in knowledge-base.yaml:
```yaml
sources:
  canonical:
    - path: "scions/canonical-kb/docs/spec.md"
      note: "Specifications are canonical"
  interpretation:
    - path: "docs/guides/**"
      note: "Interprets canonical specs"
```

## Anti-patterns

Avoid:
- Silent imports (not declared in knowledge-base.yaml)
- Copy-paste without attribution
- Unclear authority (can't tell which KB is authoritative)
- No sync tracking
- Links to a sibling checkout (`../other-kb/`) or to another repo's `main`: they resolve to
  whatever is there today, or nothing
- Editing a grafted copy in `scions/`
- Premature separation (split before clear boundary)

Related:
- [Authority policy](../policies/authority.md)
- [Provenance policy](../policies/provenance.md)
