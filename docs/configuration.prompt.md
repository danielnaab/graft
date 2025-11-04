---
model: bedrock-claude-v4.5-sonnet-us
deps:
  - Dockerfile
  - .env.example
  - scripts/render_llm.sh
  - bin/graft
---

# Configuration Documentation

Create comprehensive configuration documentation for Graft that covers:

## Required Sections

1. **Environment Variables**
   - AWS credential configuration (multiple methods: directory mount, environment variables, container environment)
   - Region configuration
   - Other environment variables (GRAFT_DIR, etc.)
   - Show practical examples with code blocks

2. **AWS Authentication Methods**
   - Explain the preferred approach (AWS directory mount) and why it's recommended
   - Document alternative approaches for CI/CD and different use cases
   - Include troubleshooting tips for common authentication issues

3. **Prompt Frontmatter Schema**
   - Document the YAML frontmatter structure for .prompt.md files
   - Explain the `model` parameter with available options
   - Detail the `deps` parameter with examples showing:
     - Manual source files
     - Generated docs as inputs (DAG creation)
     - Files outside docs/ directory
   - Explain the rules and behaviors

4. **Model Configuration**
   - List available AWS Bedrock models with their IDs
   - Explain the default model and when to use alternatives
   - Document the US inference profiles and their benefits
   - Note: Graft doesn't currently expose temperature, top_p, or max_tokens - explain this and suggest workarounds via prompt instructions

5. **DVC Configuration**
   - Document the .dvc/config structure
   - Explain the local cache approach
   - Note that users rarely need to modify DVC configuration

6. **Docker Configuration**
   - List what's included in the Dockerfile
   - Explain how to customize the Docker image
   - Document the rebuild process

7. **Git Hooks**
   - Document the pre-commit hook that auto-regenerates dvc.yaml
   - Explain why this synchronization is important

## Style Guidelines

- Use clear, scannable headings
- Provide practical code examples for each configuration option
- Include comments in code blocks explaining what each setting does
- Use admonitions (recommended/warning) where appropriate
- Cross-reference related documentation
- Assume the reader is a developer setting up Graft for the first time
- Be concise but comprehensive - every configuration option should be documented

## Technical Accuracy

- Extract exact model IDs, command flags, and configuration keys from the source files
- Show the actual directory structure and file locations
- Reference the correct environment variable names as they appear in the code
- Document both the recommended approach and alternatives

Generate documentation that helps users understand all configuration options and make informed choices for their setup.
