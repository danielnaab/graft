---
deps:
  - docs/github-integration/00-sources/design-philosophy.md
  - docs/how-it-works.md
  - scripts/pack_prompt.py
  - scripts/generate_dvc.py
---
# Validation Strategy Design

Based on the design philosophy and Graft's change detection mechanisms, produce a comprehensive validation strategy for ensuring documentation consistency.

## Core Validation Principles

Design validation that:
1. **Prevents stale documentation** from being merged
2. **Catches missing dependencies** before they cause failures
3. **Detects circular dependencies** that would break the DAG
4. **Verifies outputs match their sources** and prompts
5. **Provides actionable error messages** that guide fixes
6. **Runs fast enough** for tight feedback loops
7. **Integrates seamlessly** with both local and CI workflows

## Validation Layers

### Layer 1: Structural Validation (Fastest)

Check basic structure without LLM calls:

#### dvc.yaml Synchronization
- Verify `dvc.yaml` matches current `*.prompt.md` files
- Detect added prompts not in pipeline
- Detect removed prompts still in pipeline
- Error message: Specific prompts that need `bin/graft sync`

#### Dependency Existence
- For each prompt's `deps:` list, verify files exist
- Check glob patterns expand to at least one file
- Report missing dependencies with file and line number
- Suggest: Create file, or remove from deps

#### Circular Dependency Detection
- Build dependency graph from all prompts
- Check for cycles using topological sort
- Report cycle with file names: A → B → C → A
- Suggest: Remove one dependency to break cycle

#### Frontmatter Validity
- Parse YAML frontmatter in all prompts
- Verify required fields (deps, out)
- Check model names are valid
- Report syntax errors with line numbers

**Performance target**: <5 seconds for entire codebase

### Layer 2: Change Detection (Medium Speed)

Detect what changed without regenerating:

#### Source Change Detection
- For each prompt, run git diff on its dependencies
- Report which prompts have changed sources
- Determine action type (GENERATE/UPDATE/REFINE/REFRESH/MAINTAIN)
- Show cascade effect through DAG

#### Prompt Change Detection
- For each prompt, check if instructions changed since last commit
- Diff against HEAD version
- Report which prompts have new instructions

#### Output Staleness
- Use DVC status to detect stale outputs
- Cross-reference with git status for uncommitted changes
- Report which docs need regeneration
- Distinguish: never generated vs stale vs current

**Performance target**: <10 seconds for entire codebase

### Layer 3: Content Validation (Requires Regeneration)

Verify outputs match their inputs:

#### Regeneration Verification
- Regenerate all docs (or subset if specified)
- Compare regenerated content to committed content
- Report byte-for-byte differences
- This is the ultimate truth check

#### Signature Verification
- Check Graft signatures in generated docs
- Verify signature matches: sources + prompt + model + revision
- Detect hand-edited generated files (signature mismatch)
- Warn: You edited a generated file, use the prompt instead

**Performance target**: Depends on doc count, ~30s per doc

## Validation Modes

### Mode 1: Quick Validation (Local, Pre-Commit)

**When**: Before committing, many times per session
**What**: Layers 1 & 2 only (no regeneration)
**Exit code**: 0 if no issues, 1 if problems found
**Command**: `bin/graft check` (fast mode)

**Reports**:
- Missing dependencies
- Circular deps
- dvc.yaml out of sync
- Which docs are stale (don't regenerate, just detect)

**Use case**: Developer checking if commit is safe

### Mode 2: Full Validation (Local, Pre-Push)

**When**: Before pushing, less frequently
**What**: All layers including regeneration
**Exit code**: 0 only if regeneration produces identical output
**Command**: `bin/graft check --full` (slow mode)

**Reports**:
- Everything from Quick Validation
- Regeneration differences (show diffs)
- Attribution issues

**Use case**: Final check before opening PR

### Mode 3: CI Validation (GitHub Actions)

**When**: On PR open/update automatically
**What**: Layers 1 & 2, plus conditional Layer 3
**Strategy**:
  - Always run Layer 1 & 2 (required check)
  - Optionally run Layer 3 if label "validate:full" is added
  - Or run Layer 3 if sources/prompts changed

**Reports**:
- Status check: Pass/Fail
- PR comment with summary
- Annotations on specific files
- Suggestion: Run `bin/graft rebuild` locally

**Use case**: Prevent merging stale docs

### Mode 4: Pre-Commit Hook (Automated)

**When**: Every commit attempt
**What**: Layer 1 only (very fast)
**Behavior**: Block commit if issues found
**Command**: Installed by `bin/graft init`

**Reports**:
- Blocking message with specific issue
- Command to fix (usually `bin/graft sync` or `bin/graft rebuild`)

**Use case**: Prevent committing broken state

## Error Taxonomy

Design clear error messages for each scenario:

### E001: dvc.yaml Out of Sync
```
Error: dvc.yaml is not synchronized with prompt files

Found 2 new prompts not in pipeline:
  - docs/features/new-feature.prompt.md
  - docs/api/endpoints.prompt.md

Fix: Run `bin/graft sync` to regenerate dvc.yaml
```

### E002: Missing Dependency
```
Error: Dependency does not exist

In: docs/overview.prompt.md (line 4)
Missing: docs/architecture/diagram.md

Fix: Either create the missing file, or remove it from the deps list
```

### E003: Circular Dependency
```
Error: Circular dependency detected

Cycle: docs/a.prompt.md → docs/b.md → docs/c.prompt.md → docs/a.md

Fix: Remove one dependency to break the cycle
```

### E004: Stale Documentation
```
Error: Generated documentation is stale

3 docs need regeneration:
  - docs/how-it-works.md (sources changed)
  - docs/getting-started.md (prompt changed)
  - docs/overview.md (both changed)

Fix: Run `bin/graft rebuild` to regenerate
```

### E005: Output Mismatch
```
Error: Generated output doesn't match committed version

File: docs/api-reference.md
Regeneration produced different content (247 lines changed)

This means either:
1. You edited the generated file directly (edit the .prompt.md instead)
2. Sources/prompts changed but output wasn't regenerated
3. LLM non-determinism (rare, re-run to verify)

Fix: Run `bin/graft rebuild` and commit the regenerated file
```

### E006: Invalid Frontmatter
```
Error: Invalid YAML frontmatter

In: docs/features/new-feature.prompt.md (line 2)
Parse error: mapping values are not allowed here

Fix: Check YAML syntax in the frontmatter section
```

### E007: Invalid Model Name
```
Error: Unknown model specified

In: docs/overview.prompt.md (line 3)
Model: gpt-4 (not supported)

Supported models: bedrock-claude-v4.5-sonnet-us

Fix: Use a valid model name or remove the line to use default
```

## Integration with Git Hooks

Design hook behavior:

### Pre-Commit Hook
```bash
# In .git/hooks/pre-commit
# Generated by bin/graft init

# Quick validation only
bin/graft check

if [ $? -ne 0 ]; then
  echo ""
  echo "Commit blocked: Graft validation failed"
  echo "Fix the issues above and try again"
  exit 1
fi
```

### Pre-Push Hook (Optional)
```bash
# In .git/hooks/pre-push
# User can optionally enable for full validation

bin/graft check --full

if [ $? -ne 0 ]; then
  echo ""
  echo "Push blocked: Generated docs don't match sources/prompts"
  echo "Run 'bin/graft rebuild' and commit the changes"
  exit 1
fi
```

## CI/CD Integration Points

Specify how validation connects to GitHub Actions:

### Required Status Check
- Name: "Graft Validation"
- Always runs on PR
- Must pass before merge (if branch protection enabled)
- Fast mode (Layer 1 & 2)

### Optional Full Validation
- Triggered by label "validate:full"
- Runs Layer 3 (regeneration check)
- Slower but comprehensive
- Posts diff if mismatches found

### Status Reporting
- Use GitHub Checks API
- Show summary in PR checks section
- Post comment with details if failed
- Link to workflow logs

### Merge Blocking
- Configure as required status check
- Prevent merge if validation fails
- Clear error message in PR
- Actionable instructions for fix

## Performance Optimization

Design for speed:

### Caching Strategies
- Cache DVC pipeline state between runs
- Cache Docker images
- Cache git history for diffs
- Skip unchanged branches of DAG

### Parallelization
- Run independent validations concurrently
- Check multiple prompts in parallel
- Use DVC's built-in parallelism

### Smart Scoping
- Option to validate only changed files
- Detect affected prompts from git diff
- Skip validation of unchanged subtrees
- Full validation only on demand

### Progressive Disclosure
- Start with fastest checks
- Fail fast on structural issues
- Only proceed to expensive checks if basic checks pass
- Report results as they complete, not all at end

## Testing the Validator

Design test scenarios:

### Test Cases
1. **Happy path**: Everything valid and current
2. **New prompt**: Added but dvc.yaml not regenerated
3. **Missing dep**: Prompt references non-existent file
4. **Circular dep**: A → B → A cycle
5. **Stale output**: Sources changed, output not regenerated
6. **Hand-edited output**: User modified generated file
7. **Invalid frontmatter**: Syntax error in YAML
8. **Unknown model**: Invalid model specified

### Test Implementation
- Create test fixtures for each scenario
- Verify correct error code
- Verify error message content
- Verify suggested fix is accurate
- Test in both local and CI contexts

## Deliverables

Provide:

1. **Validation Algorithm**: Step-by-step logic for each layer
2. **Error Messages**: Complete catalog with examples
3. **Performance Targets**: Expected times for each mode
4. **Hook Templates**: Exact code for git hooks
5. **CI Integration Spec**: How to connect to GitHub Actions
6. **Test Plan**: Scenarios to validate the validator

Be specific about detection logic, exit codes, and error message formatting. This will guide implementation of the validation system.
