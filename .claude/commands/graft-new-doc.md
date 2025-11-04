# Create New Graft Documentation

Interactively scaffold a new documentation prompt file with proper structure.

This command guides you through creating a new `.prompt.md` file with:
- Proper YAML frontmatter
- Appropriate dependencies
- Clear instructions template
- Automatic DVC pipeline integration

## Workflow

### Step 1: Gather Information

Ask the user for:

1. **Document name** (required)
   - Format: kebab-case (e.g., "api-reference", "getting-started")
   - Will become the filename: `<name>.prompt.md`
   - Suggest based on existing docs if user is unsure

2. **Topic/directory** (required)
   - Where to create the file (e.g., "docs", "docs/api", "docs/guides")
   - Suggest common locations:
     - `docs/` - Top-level documentation
     - `docs/github-integration/01-explorations/` - Design explorations
     - `docs/github-integration/02-frameworks/` - Synthesis frameworks
   - Show existing directories with `find docs -type d`

3. **Dependencies** (required)
   - Which files should this document depend on?
   - Suggest recently modified files: `git diff --name-only HEAD~5..HEAD`
   - Suggest files in related directories
   - Allow user to enter custom paths or glob patterns
   - Examples:
     - Source files: `src/**/*.py`
     - Other docs: `docs/how-it-works.md`
     - Multiple: `[file1.md, file2.md, src/**/*.ts]`

4. **Purpose/Instructions** (optional but recommended)
   - Brief description of what this document should cover
   - Will be used to generate the initial prompt instructions
   - Examples:
     - "API reference for all public endpoints"
     - "Step-by-step guide for deploying to production"
     - "Comprehensive security best practices"

### Step 2: Validate Inputs

Before creating the file:

- Check if file already exists at the target path
- Verify directory exists (create if needed)
- Validate dependencies exist (warn if missing, don't block)
- Show a preview of what will be created

### Step 3: Create the Prompt File

Generate a `.prompt.md` file with this structure:

```markdown
---
deps:
  - <dependency 1>
  - <dependency 2>
  # Add model override if user requested (optional)
  # model: bedrock-claude-v4.5-sonnet-us
---

# <Document Name> - Prompt

<Purpose/instructions from user, or default template>

## Required Sections

<Suggest appropriate sections based on document name>

## Style Guidelines

- Write in clear, concise prose
- Use active voice
- Include examples where helpful
- Keep paragraphs focused

## Change Handling

When sources change: [Describe how to incorporate updates]
When prompt changes: [Describe regeneration expectations]
```

### Step 4: Update DVC Pipeline

After creating the file:

```bash
# Regenerate dvc.yaml to include new prompt
GRAFT_DIR=. bin/graft sync
```

Show the user what stage was added to the pipeline.

### Step 5: Guidance

Provide next steps:

```markdown
✅ Created: <file-path>

Next steps:
1. Review and customize the prompt instructions in <file-path>
2. Add any additional dependencies to the frontmatter
3. Generate the initial documentation:
   bin/graft rebuild

The new document will be generated at: <output-path>

To preview what will be generated:
/graft-preview

To see which docs will cascade from this:
/graft-impact <output-path>
```

## Example Session

```
User: /graft-new-doc

Claude: I'll help you create a new Graft documentation prompt. Let me gather some information.

**Document name** (kebab-case, e.g., 'api-reference'):
User: deployment-guide

Claude: Great! Where should I create this document?

**Topic/directory** (suggestions: docs/, docs/guides/, docs/api/):
User: docs/guides

Claude: Perfect! Now let's set up the dependencies. This document should depend on which files?

I can suggest:
- docs/configuration.md
- docs/getting-started.md
- bin/graft
- Dockerfile

Enter dependencies (comma-separated, or 'all' for suggestions):
User: docs/configuration.md, Dockerfile, bin/graft

Claude: Excellent! Finally, what should this document cover?

**Purpose/description**:
User: Complete guide for deploying Graft to production environments

Claude: Perfect! Here's what I'll create:

File: docs/guides/deployment-guide.prompt.md
Output: docs/guides/deployment-guide.md
Dependencies:
  - docs/configuration.md
  - Dockerfile
  - bin/graft

Proceed? (yes/no)
User: yes

Claude: ✅ Created docs/guides/deployment-guide.prompt.md

Added to DVC pipeline as stage: deployment_guide

Next steps:
1. Review and customize the prompt at docs/guides/deployment-guide.prompt.md
2. Generate the documentation: bin/graft rebuild
3. The output will be at: docs/guides/deployment-guide.md

Would you like me to open the file for editing?
```

## Template Variations

Adjust the template based on document type:

### For Exploratory Documents (01-explorations/)
- Emphasize deep analysis
- Request comprehensive coverage
- Encourage multiple perspectives
- Template includes "Analysis", "Tradeoffs", "Recommendations" sections

### For Framework Documents (02-frameworks/)
- Focus on synthesis and actionability
- Request clear specifications
- Emphasize implementation guidance
- Template includes "Overview", "Specifications", "Integration Points" sections

### For Reference Documents
- Emphasize completeness and accuracy
- Request structured, scannable format
- Include examples and code samples
- Template includes "API", "Examples", "Parameters" sections

### For Guides
- Focus on step-by-step instructions
- Request clear prerequisites
- Include troubleshooting
- Template includes "Prerequisites", "Steps", "Troubleshooting" sections

## Smart Defaults

When suggesting dependencies:

1. **For docs in same directory**: Suggest sibling documents
2. **For guides**: Suggest configuration and getting-started docs
3. **For API references**: Suggest source code files (src/**, scripts/**)
4. **For explorations**: Suggest related source documents (00-sources/)
5. **For frameworks**: Suggest related explorations

When suggesting structure:

1. **Check similar documents**: Look at docs in the same directory
2. **Parse existing prompts**: Extract common section patterns
3. **Match naming conventions**: deployment-guide → Deployment Guide

## Error Handling

- **File exists**: Offer to open existing file or choose different name
- **Invalid directory**: Create directory or suggest valid path
- **Missing dependencies**: Warn but allow (user might create them later)
- **Permission errors**: Explain and suggest fix

## Integration

After creating the file:

1. Run `bin/graft sync` to update dvc.yaml
2. Validate the new prompt: `python3 scripts/validate.py`
3. Offer to regenerate immediately or defer to user

This command makes it easy to scaffold new documentation with proper structure and dependencies