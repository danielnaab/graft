---
deps:
  - docs/github-integration/00-sources/design-philosophy.md
  - docs/how-it-works.md
  - docs/configuration.md
  - bin/graft
  - Dockerfile
---
# GitHub Actions Implementation Strategy

Based on the design philosophy and Graft's technical architecture, produce a detailed implementation strategy for GitHub Actions workflows.

## Required Analysis

### 1. Docker Integration
Analyze how Graft runs in Docker (from bin/graft and Dockerfile):
- Image building vs pulling strategy
- Workspace mounting requirements
- Credential passing mechanisms
- Performance optimization opportunities

### 2. AWS Authentication
Design secure credential management:
- GitHub Secrets approach (simplest)
- OIDC federation (most secure)
- Environment configuration options
- Regional considerations

### 3. Workflow Triggers
Specify when workflows should run:
- Pull request events (opened, synchronized, reopened)
- Push to specific branches
- Manual dispatch options
- Label-based triggers (optional)
- Comment-based triggers (optional)

### 4. Job Structure
Design the workflow job architecture:
- Validation job (fast, always runs)
- Generation job (slower, runs on demand or automatically?)
- Caching strategy for Docker images
- Parallelization opportunities
- Dependency between jobs

### 5. Validation Logic
Specify what validation means:
- Check dvc.yaml is current
- Verify all dependencies exist
- Ensure no circular dependencies
- Confirm outputs match sources/prompts
- How to determine if docs are stale

### 6. Generation Strategy
Decide on regeneration approach:
- Auto-commit regenerated docs to PR branch?
- Or just validate and require manual regeneration?
- If auto-commit: who is the committer? How to attribute?
- How to handle conflicts with concurrent pushes?

### 7. Performance Optimization
Identify optimization strategies:
- Cache Docker images between runs
- Use DVC remote cache?
- Parallel job execution
- Smart detection of what needs regeneration
- Timeout handling

### 8. Error Handling
Design failure scenarios and recovery:
- LLM API failures (rate limits, timeouts)
- Missing AWS credentials
- Dependency resolution failures
- Docker issues
- Clear, actionable error messages

### 9. Status Reporting
Determine how to communicate results:
- GitHub Check Runs API
- PR comments with summary
- Commit statuses
- Annotations on specific files

### 10. Integration Points
Specify how this connects to:
- Claude Code (slash commands trigger workflows?)
- Branch protection rules
- Required status checks
- PR merge requirements

## Deliverables

Provide:

1. **Workflow YAML Structure**: Conceptual structure (not full implementation)
2. **Decision Matrix**: For each design question, the chosen approach and rationale
3. **Security Considerations**: Threat model and mitigations
4. **Performance Profile**: Expected execution time and cost per PR
5. **Error Catalog**: Common failures with resolution steps
6. **Testing Strategy**: How to test the workflows before production use

Be specific about technical details. This will directly inform the GitHub Actions workflow implementation.
