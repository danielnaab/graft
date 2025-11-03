---
name: steward
description: AI-assisted documentation stewardship - manage documentation structure, dependencies, and content
---

# Documentation Stewardship Skill

Use this skill when the user requests documentation changes that require:
- Adding new documentation
- Restructuring existing docs
- Managing dependencies between prompts
- Propagating requirement changes across sources
- Ensuring documentation goals are met

## Core Principles

1. **Edit sources and prompts, never generated outputs** - `.md` files next to `.prompt.md` are generated
2. **Maintain clean git diffs** - Make targeted, minimal edits
3. **Validate before building** - Check dependencies exist, no cycles
4. **Show impact analysis** - Tell user what will regenerate
5. **Iterate until goals met** - Review outputs, refine sources

## Documentation Structure

```
docs/
├── 00-sources/                  # Manual strategic documents (edit these)
│   └── source-file.md
├── 01-explorations/            # Deep dives (prompts + generated)
│   ├── topic.prompt.md         # Prompt with frontmatter (edit these)
│   └── topic.md                # Generated output (NEVER edit)
├── 02-frameworks/              # Synthesized guidance (prompts + generated)
│   ├── framework.prompt.md
│   └── framework.md
├── 03-integration/             # Cross-domain integration (prompts + generated)
│   ├── overview.prompt.md
│   └── overview.md
└── 04-artifacts/               # Final deliverables (prompts + generated)
    ├── index.prompt.md
    └── index.md
```

**Source files** (`00-sources/*.md`): Manually maintained strategic inputs
**Prompt files** (`*.prompt.md`): Frontmatter + instructions for generation
**Generated files** (`.md` next to `.prompt.md`): DVC outputs, git-tracked but AI-generated

**Structure principle**: Numbered prefixes (00-04) show the DAG flow - sources → explorations → frameworks → integration → artifacts

## Stewardship Workflow

### 1. UNDERSTAND THE GOAL

When user requests documentation changes:
- Clarify the documentation goal
- Ask about target audience, scope, dependencies
- Identify if this is new content, restructuring, or refinement

### 2. ANALYZE CURRENT STATE

Use these commands to understand structure:

```bash
# List all source files
find docs -name '*.md' ! -name '*.prompt.md'

# List all prompts
find docs -name '*.prompt.md'

# See what a prompt generates
bin/docflow diff <stage_name>
# Opens build/<stage_name>.promptpack.txt showing full context

# Check which prompts use a file (reverse dependencies)
bin/docflow uses <file>
# Example: bin/docflow uses docs/strategy/messaging-framework.md
```

Read relevant files to understand content and dependencies.

### 3. PLAN OPERATIONS

Determine what needs to change. Common operations:

#### Create New Source File
```bash
# Use Write tool for manual source documents
# File: docs/00-sources/<name>.md
```

#### Create New Prompt
```bash
# Scaffold with proper frontmatter
bin/docflow new <name> <directory>
# Example: bin/docflow new architecture-exploration 01-explorations
# Generates docs/01-explorations/<name>.prompt.md

# Then edit the frontmatter and prompt
```

#### Edit Source Content
```bash
# Use Read to view current content
# Use Edit to make targeted changes
# Keep edits minimal and semantic
```

#### Add Dependency to Prompt
```bash
# Use Read to view frontmatter
# Use Edit to add to deps: list
```

Example:
```yaml
---
deps:
  - docs/00-sources/flexion-solutions-messaging.md
  - docs/01-explorations/case-management-architecture.md  # <- Add this
---
```

#### Remove Dependency from Prompt
```bash
# Use Edit to remove from deps: list
```

#### Change Prompt Instructions
```bash
# Use Edit to modify prompt body
# NOTE: This triggers RESTYLE - full regeneration
```

### 4. IMPACT ANALYSIS

Before making changes, analyze impact:

```bash
# Check which prompts depend on a file you're editing
bin/docflow uses docs/architecture/components.md

# Output shows all prompts that will regenerate
```

Tell user:
- Which files you'll create/edit
- Which prompts will regenerate (cascade)
- Estimated DVC rebuild time

### 5. VALIDATE

Before rebuilding:

```bash
# Validate all dependencies exist, no circular deps
bin/docflow check

# Or validate specific prompt
bin/docflow check docs/architecture/high-level.prompt.md
```

Fix any errors before proceeding.

### 6. EXECUTE CHANGES

Make the changes using standard tools:
- `Write` - Create new source files
- `Edit` - Modify existing sources or frontmatter
- `bin/docflow new` - Scaffold new prompts

Keep a todo list of operations and mark them complete.

### 7. REBUILD PIPELINE

```bash
# Regenerate dvc.yaml and run DVC
bin/docflow rebuild
```

DVC will:
- Detect which sources changed
- Detect which prompts changed
- Regenerate only affected outputs
- Apply appropriate action (GENERATE, UPDATE, RESTYLE, REFRESH)

### 8. REVIEW OUTPUTS

```bash
# Show git diff to see what changed
git diff

# Read generated files to check quality
# Use Read tool on docs/<topic>/<name>.md
```

### 9. ITERATE IF NEEDED

If outputs don't meet goals:
- Edit source files to add missing content
- Adjust prompt instructions for better synthesis
- Add/remove dependencies
- Run `bin/docflow rebuild` again

Repeat until goals met.

### 10. FINALIZE

Once satisfied:
```bash
# Show clean git diff
git diff

# Create commit with stewardship context
git add <files>
git commit -m "Add deployment documentation

- Created docs/deployment/aws.md (source)
- Created docs/deployment/overview.prompt.md
- Added deployment context to architecture/components.md
- Generates comprehensive deployment guide"
```

## Common Patterns

### Pattern: Add New Documentation Topic

```
User: "We need deployment documentation"

1. Create source files in 00-sources:
   - docs/00-sources/deployment-aws.md
   - docs/00-sources/deployment-local.md
   - docs/00-sources/deployment-cicd.md

2. Create exploration prompt:
   bin/docflow new deployment-overview 01-explorations

3. Edit deployment-overview.prompt.md frontmatter:
   deps:
     - docs/00-sources/deployment-aws.md
     - docs/00-sources/deployment-local.md
     - docs/00-sources/deployment-cicd.md

4. Validate:
   bin/docflow check

5. Build:
   bin/docflow rebuild

6. Review:
   Read docs/01-explorations/deployment-overview.md

7. Iterate if needed
```

### Pattern: Enhance Existing Documentation

```
User: "Technical framework needs deployment info"

1. Check impact:
   bin/docflow uses docs/01-explorations/case-management-architecture.md
   # Shows: 02-frameworks/technical-framework.prompt.md depends on it

2. Edit source:
   # Add "Deployment" section to case-management-architecture.md

3. Build:
   bin/docflow rebuild
   # Triggers: technical-framework.md regeneration (UPDATE mode)

4. Review:
   git diff docs/02-frameworks/technical-framework.md
   # Should show only deployment-related additions
```

### Pattern: Restructure Dependencies

```
User: "Technical framework should also consider deployment"

1. Read current deps:
   Read docs/02-frameworks/technical-framework.prompt.md

2. Check what's available:
   find docs/00-sources docs/01-explorations -name '*.md' ! -name '*.prompt.md'

3. Edit frontmatter to add dependency:
   deps:
     - docs/00-sources/flexion-solutions-messaging.md
     - docs/01-explorations/case-management-architecture.md
     - docs/00-sources/deployment-overview.md  # <- Add this

4. Validate:
   bin/docflow check docs/02-frameworks/technical-framework.prompt.md

5. Build:
   bin/docflow rebuild
   # Triggers: REFRESH (dep changed)

6. Review cascade:
   git diff
```

### Pattern: Fix Broken Dependencies

```
bin/docflow check
# Error: docs/old-file.md doesn't exist

1. Find affected prompts:
   grep -r "old-file.md" docs/**/*.prompt.md

2. Either:
   a) Create the missing file, OR
   b) Remove the dependency (Edit frontmatter)

3. Validate again:
   bin/docflow check
```

## Key Commands Reference

### Existing Commands (Use Freely)

```bash
bin/docflow new <name> <topic>     # Scaffold new prompt
bin/docflow diff <stage>           # See prompt context
bin/docflow check [prompt]         # Validate dependencies
bin/docflow rebuild                # Regenerate + build
bin/docflow sync                   # Just regenerate dvc.yaml
bin/docflow status                 # DVC pipeline status
```

### New Commands (To Be Implemented)

```bash
bin/docflow uses <file>            # Show reverse dependencies
```

### Standard Tools (Always Available)

```bash
find docs -name '*.md'             # Find markdown files
grep -r "pattern" docs/            # Search in docs
Read <file>                        # Read file contents
Edit <file>                        # Edit file (YAML-safe)
Write <file>                       # Create new file
```

## Change Detection Logic

The system automatically determines actions based on what changed:

- **GENERATE**: No previous output exists → create from scratch
- **UPDATE**: Sources changed, prompt unchanged → apply semantic changes only
- **RESTYLE**: Prompt changed, sources unchanged → full rewrite with new style
- **REFRESH**: Both changed → apply updates AND new style
- **MAINTAIN**: Nothing changed → no updates needed

This is handled automatically by `scripts/pack_prompt.py` - you don't control it directly.

## Anti-Patterns (Avoid These)

❌ **Don't edit generated `.md` files** - They're outputs, will be overwritten
❌ **Don't edit `dvc.yaml` manually** - Auto-generated from frontmatter
❌ **Don't make unnecessary changes** - Triggers rebuilds, pollutes git diffs
❌ **Don't skip validation** - Catches errors before expensive rebuilds
❌ **Don't ignore impact analysis** - User needs to know what will regenerate

## Best Practices

✅ **Make targeted edits** - Change only what's needed for the goal
✅ **Check reverse deps** - Know what will cascade before editing sources
✅ **Validate early** - Run `check` before `rebuild`
✅ **Show clean diffs** - Present git diff to user before committing
✅ **Explain cascades** - Tell user why certain docs regenerated
✅ **Iterate confidently** - Rebuild is safe, outputs are git-tracked
✅ **Document rationale** - Explain why you made specific changes

## Example Session

```
User: "We need comprehensive deployment documentation"

Steward (you):
I'll help create deployment documentation. Let me analyze the current structure.

[Uses find to see docs/ structure]
[Uses bin/docflow uses to check dependencies]

I'll create:
1. Source files in 00-sources: deployment-aws.md, deployment-local.md, deployment-cicd.md
2. Exploration prompt: deployment-overview in 01-explorations
3. Framework synthesis: deployment-framework in 02-frameworks

This will cause these docs to regenerate:
- 01-explorations/deployment-overview.md (new)
- 02-frameworks/deployment-framework.md (new)
- 03-integration/solution-overview.md (will include deployment context)

Estimated rebuild: 3 stages, ~90 seconds

Proceed? [User confirms]

[Creates source files using Write in 00-sources]
[Scaffolds prompts with bin/docflow new]
[Edits frontmatter with Edit]
[Validates with bin/docflow check]
[Builds with bin/docflow rebuild]
[Shows git diff]

Review the generated docs. Need any refinements?

[User: "Overview needs more detail on CI/CD"]

[Edits docs/00-sources/deployment-cicd.md with more detail]
[Rebuilds - cascades through exploration and framework]
[Shows updated git diff]

Looks good! Ready to commit?
```

## Tips for Success

1. **Start with analysis** - Understand before acting
2. **Plan operations** - Think through cascades
3. **Validate frequently** - Catch errors early
4. **Review outputs** - Quality check generated docs
5. **Iterate confidently** - Rebuilds are cheap and safe
6. **Show your work** - Use TodoWrite to track operations
7. **Clean git history** - Meaningful commits with context

---

**Remember**: You're managing the *source layer* of a documentation DAG. The system handles generation. Your job is strategic: structure, dependencies, and content quality.
