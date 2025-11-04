---
model: bedrock-claude-v4.5-sonnet-us
deps:
  - scripts/render_llm.sh
  - scripts/entrypoint.sh
  - bin/graft
---

# Troubleshooting Guide

Create a comprehensive troubleshooting guide that helps users diagnose and fix common issues with Graft.

## Required Sections

1. **Nothing Regenerated**
   - How to check pipeline status
   - How to verify dependencies
   - How to force regeneration
   - Explain why this happens (no changes in deps:)

2. **AWS Authentication Errors**
   - How to verify credentials
   - How to test Bedrock access
   - Common IAM permission issues
   - Model availability by region
   - Temporary credentials / AWS SSO
   - Reference the actual error handling from render_llm.sh

3. **Docker Image Not Found**
   - How to build the image
   - How to verify it exists
   - Custom Graft location (GRAFT_DIR)
   - Reference the checks in bin/graft

4. **Large Diffs in Generated Docs**
   - Why this might happen
   - How to ensure UPDATE directives work properly
   - How to review packed prompts
   - How to adjust prompt instructions for better preservation
   - Show example of good preservation instructions

5. **Dependency Errors**
   - Dependency not found errors
   - Circular dependencies
   - How to check and fix
   - Show examples of good vs bad dependency structures

6. **DVC Errors**
   - Corrupted cache
   - Stage failures
   - How to view logs
   - Common causes (credential expiration, rate limiting, invalid syntax)

7. **Performance Issues**
   - Slow generation (explain expected timing)
   - How to optimize prompt and source structure
   - Note that DVC parallelizes when possible
   - Many unnecessary regenerations (how to diagnose with `bin/graft uses`)

8. **Getting Help**
   - What to check before asking for help
   - How to gather diagnostic information
   - Where to file issues

## Style Guidelines

- Start each section with the symptom/error message users see
- Provide diagnostic commands first, then solutions
- Use complete, copy-pasteable commands
- Include expected output to confirm success
- Organize by user impact (most common issues first)
- Use clear, actionable headings
- Be empathetic - acknowledge frustration
- Provide multiple solutions where applicable
- Cross-reference relevant documentation

## Technical Accuracy

- Reference actual error messages from the code
- Show real command output
- Document actual system behavior
- Reference the specific checks and error handling in the source code
- Show correct file paths and directory structure

## Problem-Solution Format

For each issue:
1. Describe the symptom clearly
2. Provide diagnostic commands
3. Explain the root cause
4. Give step-by-step solution
5. Show how to verify the fix worked

Generate documentation that reduces user frustration and builds troubleshooting confidence.
