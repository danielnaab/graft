---
title: "Work Log: One-Level Dependency Model Exploration"
date: 2026-01-31
status: completed
---

# One-Level Dependency Model Exploration

## Objective

Explore whether Graft could simplify its architecture by supporting only one level of dependencies (no transitive resolution).

## Background

Graft "grafts" material from dependencies—once migrations run and content is transformed, results are committed to git. This raises a question: if downstream consumers get YOUR committed content rather than the original dependency chain, why resolve transitives at all?

Current model:
```
project/
├── graft.yaml
├── graft.lock          # Full dependency graph
└── .graft/
    ├── meta-kb/        # Direct dependency
    ├── standards-kb/   # Transitive dependency
    └── templates-kb/   # Transitive dependency
```

Proposed one-level model:
```
project/
├── graft.yaml
├── graft.lock          # Direct deps only
└── .graft/
    └── meta-kb/        # Only direct dependencies cloned
```

## Model Comparison

| Aspect | Current (Full Transitive) | Proposed (One-Level) |
|--------|---------------------------|----------------------|
| What's cloned | Direct + all transitive deps | Direct deps only |
| Lock file | Complete graph with requires/required_by | Simple list of direct deps |
| Where results end up | `.graft/` (resolved on demand) | Project files (committed to git) |
| Downstream experience | Must re-resolve full graph | Gets committed results directly |

## Trade-off Analysis

### What Simplifies

1. **No recursive resolution algorithm.** Dependency resolution becomes a simple loop over direct dependencies.

2. **No conflict detection for transitive deps.** Version conflicts are impossible—each project manages only its direct dependencies.

3. **Smaller `.graft/` directories.** Only direct dependencies are cloned, reducing disk usage.

4. **Simpler lock file format.** No need for `requires`/`required_by` relationships.

5. **Clearer mental model.** "My deps are my problem; their deps are their problem."

### What Breaks

1. **Exported commands referencing transitives.** If `meta-kb:validate` shells out to `../.graft/standards-kb/bin/checker`, that path doesn't exist.

2. **Cross-dependency markdown links.** Links in grafted content pointing to transitive deps break.

3. **Decision 0005 (No Partial Resolution).** Would need to be retired or amended.

4. **Debugging/tracing.** Cannot inspect transitive dependency sources locally.

5. **Live collaboration.** Cannot easily fix bugs in transitive deps in place.

## Use Case Analysis

| Use Case | Works? | Notes |
|----------|--------|-------|
| Graft content from direct dep | YES | Identical to current |
| Run `graft upgrade` for migrations | YES | For direct deps |
| Reference files in direct dep | YES | `.graft/meta-kb/` exists |
| Migration script uses transitive dep | NO | Transitive not available |
| Downstream clones your project | BETTER | Grafted content already committed |
| Debug where content came from | WORSE | Only direct sources available |

## Critical Constraint: Migration Script Self-Containment

The one-level model works if and only if dependencies never reference their transitive deps in commands or migrations.

Self-contained migration (works):
```yaml
commands:
  migrate-v2:
    run: "npx jscodeshift -t codemods/v2.js src/"
```

Migration referencing transitive (breaks):
```yaml
commands:
  migrate-v2:
    run: |
      cp ${DEP_ROOT}/../standards-kb/templates/* ./templates/
```

This constraint is difficult to enforce and limits what migrations can do.

## Possible Hybrid: Build-Time Transitives

Clone transitive deps temporarily during `graft upgrade`, run migrations with full graph access, then delete transitives. Committed results remain.

This preserves:
- Simple `.graft/` (only direct deps persist)
- Full migration capability (transitives available during execution)
- Committed grafts for downstream

But adds:
- Complexity of temporary cloning
- Non-obvious behavior (deps appear and disappear)
- Harder to debug migration failures

## Recommendation

**The one-level model is viable IF:**
1. Migration scripts are constrained to be self-contained
2. Graftable content avoids cross-dependency links
3. The project accepts reduced traceability

**Otherwise**, keep full transitive resolution but consider adding a "graft commit" phase that commits migration results to git. This gives downstream the committed content benefit while preserving full resolution capability during development.

The current architecture, validated in the 2026-01-12 exploration, correctly prioritizes development workflow (traceability, collaboration, stable paths) over simplicity. The one-level model optimizes for a different use case (artifact distribution) at the expense of development experience.

## Changelog

- 2026-01-31: Explored one-level dependency model as alternative to full transitive resolution. Identified migration script self-containment as critical constraint. Concluded current architecture remains appropriate for development workflow; one-level model better suited to artifact distribution scenarios.
