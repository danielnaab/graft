---
deps:
  - docs/project-overview.md
  - docs/how-it-works.md
  - docs/getting-started.md
  - docs/configuration.md
  - docs/command-reference.md
  - docs/claude-skills.md
---

Generate a practical guide to Graft use cases that shows concrete applications across different domains.

## Document Purpose

Create a document that helps users understand how to apply Graft to their own documentation challenges. The goal is to:
- Show realistic applications across diverse domains
- Demonstrate design process from vague goals to complete information architectures
- Provide concrete, actionable examples users can adapt
- Let the ideas speak for themselves through clear examples

## Structure

### Introduction
- Brief overview of what Graft does (git-native change detection, multi-level DAGs, intelligent updates)
- Mention Claude Skills with link to docs/claude-skills.md for AI-assisted documentation management
- How to read this document and apply patterns to your work
- Keep this section concise (2-3 paragraphs)

### Core Use Cases

Provide exactly 5 use cases (see list below). For cases 1-3, use standard structure:
- **Scenario** (2-3 sentences): What problem does this solve?
- **Implementation** (YAML frontmatter + 2-3 sentences): How to set it up
- **Workflow** (4-6 bash commands with brief comments): Concrete example
- **What This Enables** (2-3 sentences): What becomes possible

For cases 4-5, show the DESIGN PROCESS:
- Start with a qualitative goal or vague requirement
- Show the thinking process of identifying information needs
- Demonstrate iterating on information architecture
- Show how exploration reveals better structures
- Include dead ends and adjustments (realistic design process)
- End with complete, working architecture

#### The 5 Use Cases

1. **PR-to-Release Notes Pipeline**: Automated release notes that synthesize merged PR summaries into user-facing changelogs

2. **Living API Documentation**: API docs that stay in sync with OpenAPI specs and code examples

3. **Incident Post-Mortem Synthesis**: Comprehensive post-mortems from logs, timelines, runbooks, and team retrospectives

4. **Strategic Planning Hierarchy** (DESIGN PROCESS): Starting from "we need better planning docs", explore and design a multi-level information architecture (company strategy → engineering analysis → team plans)

5. **Research Synthesis to Product Brief** (DESIGN PROCESS): Starting from "turn user research into actionable insights", design an information architecture that explores from raw data through analysis to recommendations

### Cross-Cutting Patterns

After use cases, provide a BRIEF patterns section. For each of these 3 patterns, write exactly 1 paragraph (3-4 sentences):

#### Pattern: Cascade Synthesis
Building multi-level documentation hierarchies where abstractions flow from detailed sources to executive summaries.

#### Pattern: Living Documentation
Keeping documentation in sync with rapidly changing sources using intelligent UPDATE actions.

#### Pattern: Git-Native Workflows
Integrating documentation generation into existing git workflows (branches, PRs, CI/CD).

### Getting Started with Your Use Case

Provide a BRIEF (3-4 paragraphs) practical guide helping readers apply these patterns to their own work. Include a simple framework for getting started.

## Writing Guidelines

**Tone & Style:**
- Plain language - clear and direct, no hype or overselling
- Modest - let the ideas speak for themselves without superlatives
- Concrete over abstract - every use case needs specific examples
- Respectful of reader's time - comprehensive but concise
- Professional but accessible - avoid jargon, explain technical concepts

**Technical Accuracy:**
- Use only actual Graft commands and syntax from the documentation
- Reference real frontmatter fields and configurations
- Show accurate `.prompt.md` structures
- Include realistic dependency graphs

**Examples Quality:**
- Each use case should feel "real" - like something a team would actually do
- Include enough detail to be actionable (not just conceptual)
- Show both simple and sophisticated applications
- For design process examples: show realistic iteration, including missteps and corrections

**Emphasis:**
- Connect use cases to Graft's core capabilities (change detection, DAGs, git-native)
- Focus on what becomes possible, not on persuading the reader
- For design process examples: emphasize the thinking and exploration, not just the outcome

**Structure:**
- Use clear headings and subheadings for scannability
- Include code blocks with proper syntax highlighting
- Use callouts or emphasis for key insights
- Keep sections focused - each use case should be 1-2 pages max

## Output Format

Generate the complete use cases document as clean markdown suitable for docs/use-cases.md. The document should:
- Cover exactly 5 use cases (3 standard, 2 showing design process)
- Use cases 4-5 should be longer and show the design thinking process in detail
- Include 3 cross-cutting patterns (1 paragraph each)
- Include getting started section (3-4 paragraphs)
- Use plain language throughout - no hype, no overselling
- Total length: around 500-700 lines to accommodate design process examples

**IMPORTANT**: Output ONLY the markdown content. Do NOT wrap in code fences. Start with level-1 heading.

**CRITICAL**: COMPLETE THE ENTIRE DOCUMENT. Do not stop mid-section. All 5 use cases (including both full design process examples), all 3 patterns, and the getting started section MUST be included in full. If you're running short on space, make the workflow examples in cases 1-3 more concise, but ensure use cases 4-5 fully show the design process with iteration and thinking.
