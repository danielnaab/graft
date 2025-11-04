---
model: bedrock-claude-v4.5-sonnet-us
deps:
  - Makefile
  - bin/graft
  - scripts/entrypoint.sh
  - .env.example
---

# Getting Started Guide

Create a practical, step-by-step getting started guide for Graft that helps new users go from zero to generating their first document.

## Required Sections

1. **Introduction**
   - Brief one-sentence description of what Graft is
   - Position it as an LLM-powered documentation pipeline

2. **Prerequisites**
   - List required tools (Docker, AWS credentials)
   - Be specific about AWS requirements (Bedrock access, Claude Sonnet 4.5)

3. **Installation**
   - **Build Graft**: Show the complete sequence from git clone through make build
   - **Set up your project**: Explain the two AWS credential methods (mounted ~/.aws vs .env file)
   - Show actual commands with proper context
   - **Initialize Graft**: Document the `bin/graft init` command and what it does

4. **Create Your First Document**
   - **Scaffold a new prompt**: Show `bin/graft new` command with practical example
   - **Add dependencies and instructions**: Show how to edit the prompt file with a realistic example
   - **Generate the document**: Show `bin/graft rebuild` and explain what happens
   - Use a concrete, relatable example throughout (not abstract placeholders)

5. **Review and Iterate**
   - Show how to view the generated document
   - Explain the two types of refinement:
     - Editing source files (triggers UPDATE)
     - Editing prompt instructions (triggers full regeneration)
   - Emphasize that the system automatically detects what changed

6. **Next Steps**
   - Link to other documentation (use-cases, how-it-works, configuration, command-reference)
   - Provide context for when to read each

## Style Guidelines

- Write in second person ("you") to directly address the reader
- Use complete, copy-pasteable commands with proper working directory context
- Every code block should be runnable as-is
- Include brief explanations of what each step accomplishes
- Use consistent terminology throughout
- Keep the example scenario simple but realistic
- Progressive disclosure: start simple, mention advanced features at the end
- Use encouraging, confidence-building language

## Technical Accuracy

- Show exact command syntax from entrypoint.sh
- Reference the correct paths and directory structure
- Explain what files get created where
- Document actual system behavior (what triggers what)

Generate a guide that gives new users a smooth onboarding experience and builds their confidence to explore further.
