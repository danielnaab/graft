---
deps:
  - docs/github-integration/00-sources/design-philosophy.md
  - docs/how-it-works.md
  - docs/use-cases.md
---
# GitHub Integration Workflow Patterns

Based on the design philosophy and Graft's core capabilities, produce a comprehensive specification of workflow patterns for the GitHub + Graft integration.

For each workflow pattern, provide:

1. **Pattern Name**: Clear, descriptive name
2. **Actor**: Who performs this workflow (developer, reviewer, CI system, Claude Code)
3. **Trigger**: What initiates this workflow
4. **Preconditions**: Required state before workflow begins
5. **Steps**: Detailed step-by-step process with commands and expected outputs
6. **Artifacts**: What gets created/modified
7. **Success Criteria**: How to know the workflow succeeded
8. **Error Scenarios**: Common failures and how to handle them
9. **Performance Characteristics**: Time, cost, resource usage
10. **Integration Points**: Where this workflow connects to others

Cover these critical workflow patterns:

## Developer Workflows
- **Local Iteration**: Edit sources/prompts → regenerate → review cycle
- **PR Preparation**: Validate locally before pushing
- **PR Response**: Responding to review feedback with doc updates

## CI/CD Workflows
- **Validation on Push**: Verify docs match sources/prompts
- **Preview Generation**: Regenerate docs to show impact
- **Merge Enforcement**: Block merge if validation fails

## Claude Code Workflows
- **Assisted Authoring**: Using Claude Code to refine prompts/sources
- **Impact Analysis**: Understanding which docs will regenerate
- **Quick Validation**: Check consistency without full regeneration

## Review Workflows
- **Context Building**: Reviewer understanding changes
- **Feedback Provision**: Requesting changes to prompts/sources
- **Approval Decision**: Criteria for accepting PRs

## Error Recovery Workflows
- **Stale Docs**: Fixing documentation that's out of sync
- **Failed Generation**: Handling LLM API failures
- **Dependency Issues**: Resolving missing or circular dependencies

For each workflow, think deeply about:
- How it demonstrates Graft's value
- What makes it better than manual documentation
- Where automation helps and where human judgment is essential
- How it fits into the GitHub PR lifecycle
- What Claude Code adds to the experience

Be specific about commands, expected outputs, and decision points. This document will guide implementation, so precision matters.
