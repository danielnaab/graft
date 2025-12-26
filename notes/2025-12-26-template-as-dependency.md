---
date: 2025-12-26
status: stable
tags: [copier, template, documentation, knowledge-base, refactoring]
---

# Template as Dependency Pattern Implementation

## Summary

Successfully refactored both python-starter template and graft repository to implement the "template as dependency" pattern, where generated projects import template documentation via knowledge base imports rather than duplicating it.

## Problem

After initial Copier template application, graft contained ~11,000 lines of duplicated template documentation:
- Generic architecture patterns in docs/architecture/
- Template-specific ADRs in docs/decisions/
- How-to guides in docs/guides/
- Technical reference in docs/reference/
- Template KB config in docs/knowledge-base.yaml

This created:
- Duplication between template and project
- Maintenance burden (fix docs in N places)
- Noise in project (template patterns vs domain logic)
- Stale docs in projects when template improves

## Solution Implemented

### Phase 1: Template Changes (python-starter)

**Updated copier.yml**:
- Excluded template docs from generation (_exclude)
- Protected project-specific docs (_skip_if_exists)
- Template docs stay in python-starter only

**Created parameterized docs**:
- `docs/README.md.jinja` - Minimal project docs with template links
- `docs/agents.md.jinja` - Agent entrypoint with template references
- `TEMPLATE_STATUS.md.jinja` - Project-specific template metadata

**Commit**: `145552b` - "Refactor template to treat docs as dependency"

### Phase 2: Project Cleanup (graft)

**Removed duplicated docs**:
- Deleted docs/architecture/ (5 files)
- Deleted docs/decisions/ (7 ADRs)
- Deleted docs/guides/ (5 files)
- Deleted docs/reference/ (5 files)
- Deleted docs/knowledge-base.yaml

**Updated configuration**:
- Added python-starter import to knowledge-base.yaml
- Created minimal docs/README.md linking to template
- Kept custom docs/agents.md (graft-specific)

**Results**:
- Reduced from ~11,000 lines to ~100 lines of project docs
- Clear separation: template = patterns, graft = domain

**Commit**: `7052b52` - "Remove duplicated template docs, import python-starter KB instead"

### Phase 3: Testing & Documentation

**Tested Copier update lifecycle**:
- Updated .copier-answers.yml to latest template commit
- Ran `copier update --trust --defaults`
- Verified merge conflict resolution (TEMPLATE_STATUS.md)
- Confirmed protected files preserved (docs/agents.md)
- Update completed successfully

**Documented the pattern**:
- Created docs/copier/dependency-pattern.md in python-starter
- Comprehensive guide for using this pattern
- Includes problem/solution, implementation, benefits, migration

**Commit** (python-starter): `a7e1c3f` - "Document template-as-dependency pattern"

## How It Works

### Agent Discovery Flow

1. Agent reads `graft/knowledge-base.yaml`
2. Sees import: `path: ../python-starter`
3. Reads `../python-starter/knowledge-base.yaml`
4. Discovers template docs:
   - `../python-starter/docs/architecture/`
   - `../python-starter/docs/decisions/`
   - `../python-starter/docs/guides/`
   - `../python-starter/docs/reference/`
5. Uses template patterns while working on graft

No duplication needed - template docs accessible via KB imports.

### Update Lifecycle

```bash
# Template improvement
cd python-starter
# Update docs/architecture/functional-services.md
git commit -m "Improve functional services guide"

# Project gets update
cd graft
copier update --trust
# New docs/README.md references improved template docs
# Custom docs/agents.md preserved
# Template docs immediately available
```

## Benefits Realized

1. **No Duplication**: Single source of truth for patterns
2. **Living Dependency**: Template improvements benefit graft automatically
3. **Clean Separation**: Template focuses on patterns, graft on domain
4. **Agent Discovery**: KB imports make template docs discoverable
5. **Update Safety**: Copier lifecycle preserves customizations
6. **Reduced Noise**: Project contains only project-specific docs

## File Organization

**Template (python-starter)**:
```
python-starter/
├── docs/
│   ├── README.md.jinja           # Generated minimal docs
│   ├── agents.md.jinja            # Generated agent entrypoint
│   ├── architecture/              # NOT generated (referenced)
│   ├── decisions/                 # NOT generated (referenced)
│   ├── guides/                    # NOT generated (referenced)
│   ├── reference/                 # NOT generated (referenced)
│   └── copier/
│       └── dependency-pattern.md  # This pattern documented
├── knowledge-base.yaml            # Template KB (NOT generated)
└── copier.yml                     # Template config
```

**Project (graft)**:
```
graft/
├── docs/
│   ├── README.md       # Minimal docs, links to ../python-starter/docs/
│   └── agents.md       # Graft-specific agent guidance
├── knowledge-base.yaml # Imports ../python-starter KB
└── notes/
    └── 2025-12-26-*.md # Implementation notes
```

## Copier Update Test Results

✅ **Update successful**: No errors
✅ **Conflict resolution**: TEMPLATE_STATUS.md merged cleanly
✅ **Protected files**: docs/agents.md preserved
✅ **Template refs**: Documentation links work
✅ **KB imports**: Agent can discover template docs

## Next Steps

Future work can now:
1. Improve template docs → benefits graft automatically
2. Add new patterns to template → graft gets them via update
3. Focus graft docs on domain-specific content
4. Use template patterns for other projects
5. Version template → projects can update selectively

## Pattern Applicability

This pattern works well when:
- ✅ Template provides reusable patterns
- ✅ Projects need access to template repository
- ✅ KB import system available
- ✅ Template is stable and maintained

Don't use when:
- ❌ Projects deploy without template access
- ❌ Template changes frequently (instability)
- ❌ Projects fork patterns significantly

## Sources

- Template changes: python-starter commits 145552b, a7e1c3f
- Project cleanup: graft commits 7052b52, 3b0fe7b, b54bdc2
- Pattern docs: python-starter/docs/copier/dependency-pattern.md
- Meta-KB methodology: ../meta-knowledge-base/docs/meta.md
- Copier documentation: https://copier.readthedocs.io/

## Impact

**Lines of documentation**:
- Before: ~11,387 lines (duplicated)
- After: ~80 lines (project-specific) + template reference
- Reduction: ~99% duplication eliminated

**Maintainability**:
- Before: Fix docs in template + every project
- After: Fix docs in template only

**Clarity**:
- Before: Template patterns mixed with domain logic
- After: Clear separation via imports

This pattern is now documented in the template and ready for use in future projects.
