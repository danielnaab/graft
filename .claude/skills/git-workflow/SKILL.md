# Skill: Git Workflow

## Purpose
Guide through git operations following Graft project conventions, ensuring work logs are updated and commits follow project standards.

## When to Use
Activate when the user mentions:
- Making a commit
- Committing changes
- Git workflow
- Creating a PR
- Push to remote
- Git operations

## Pre-Commit Checklist

Before any commit, ensure:
- [ ] All tests pass (`pytest -q`)
- [ ] Code is formatted (`pre-commit run --all-files` or `/format`)
- [ ] Work log is updated with session notes
- [ ] Changes follow layered architecture
- [ ] No files in `.venv/` or unintended files included

## Commit Message Guidelines

### Format
```
<type>: <brief description>

<optional detailed explanation>

<optional references>
```

### Types
- **feat**: New feature (e.g., "feat: implement Slice 1 run command")
- **fix**: Bug fix (e.g., "fix: correct exit code for missing graft.yaml")
- **refactor**: Code restructuring without behavior change
- **test**: Adding or updating tests
- **docs**: Documentation updates
- **chore**: Maintenance tasks (dependencies, tooling)

### Examples from Graft
```
feat: implement explain command with layered architecture

- Add Domain entities for Artifact, GraftConfig, Derivation
- Create FileSystem and Config adapters
- Implement ExplainService with dependency injection
- Wire up CLI command with proper error handling
- All tests passing

Completes Slice 0.
```

```
fix: ensure explain command returns exit code 1 for user errors

Changed exception handling in CLI layer to map FileNotFoundError
and yaml.YAMLError to exit code 1 instead of 2.

Fixes test_explain_missing_graft_yaml and test_explain_malformed_yaml.
```

## Commit Process

### 1. Review Changes
```bash
git status
git diff
```

Check:
- Only intended files are modified
- No debug code, console.log, or temporary changes
- No sensitive data (.env, credentials, etc.)

### 2. Verify Tests
```bash
pytest -q
```

All tests must pass before committing.

### 3. Update Work Log
Use `/work-log` to document:
- What was accomplished
- Files modified/created
- Test results
- Architectural decisions

### 4. Format Code
```bash
pre-commit run --all-files
# Or use /format command
```

Fix any issues reported by formatters/linters.

### 5. Stage Changes
```bash
git add <files>
# Or for all changes:
git add .
```

Be explicit about what you're staging.

### 6. Create Commit
```bash
git commit -m "$(cat <<'EOF'
type: brief description

Detailed explanation of what changed and why.

References to issues, slices, or tests if relevant.
EOF
)"
```

Pre-commit hooks will run automatically. If they fail:
- Review the errors
- Fix the issues
- Stage the fixes
- Commit again

### 7. Review Commit
```bash
git log -1 --stat
git show HEAD
```

Verify the commit looks correct.

## Pull Request Process

### 1. Ensure Branch is Up to Date
```bash
git status
# Check if ahead of remote
```

### 2. Push to Remote
```bash
git push origin <branch-name>
# Or if tracking is set:
git push
```

### 3. Create PR
Use GitHub CLI or web interface:
```bash
gh pr create --title "Implement Slice N: <description>" --body "$(cat <<'EOF'
## Summary
- Bullet points of what was implemented
- Reference to slice documentation

## Test Plan
- All tests passing (pytest -q)
- Manual testing performed: ...

## Architectural Notes
- Follows layered architecture (ADR-0002)
- No breaking changes to CLI contract

## Checklist
- [x] Tests pass
- [x] Code formatted
- [x] Work log updated
- [x] Documentation updated
EOF
)"
```

## Common Scenarios

### Scenario: Need to amend last commit
```bash
# Make additional changes
git add <files>
git commit --amend
```

Use when:
- Forgot to include a file
- Typo in commit message
- Minor fix to last commit

Don't use if commit has been pushed to remote (unless you're the only one on the branch).

### Scenario: Work in progress, want to save state
```bash
git add .
git commit -m "WIP: working on <feature>"
```

Use when:
- Switching context temporarily
- End of day, incomplete work
- Before trying risky refactoring

Can squash/amend these commits later before PR.

### Scenario: Made changes to wrong files
```bash
# Unstage specific files
git reset HEAD <file>

# Or unstage everything
git reset HEAD

# Discard changes to file (CAREFUL!)
git checkout -- <file>
```

### Scenario: Tests failing but want to commit anyway
**Don't do this.** Fix the tests first. Green tests are a requirement.

Exception: Creating a WIP commit on a personal branch, with clear "WIP" marker.

## Integration with Graft Workflow

### After Completing a Slice
1. Ensure slice acceptance criteria are met
2. Run `/check-contract` to verify CLI compliance
3. Update work log with `/work-log`
4. Run `/test` to verify all tests pass
5. Format with `/format` or pre-commit
6. Commit with descriptive message referencing slice
7. Create PR with slice summary

### During Development
- Commit frequently with descriptive messages
- Keep commits focused (one logical change per commit)
- Update work log at session boundaries
- Push regularly to backup work

## Anti-Patterns to Avoid

### ❌ Committing without testing
```bash
git add . && git commit -m "fix stuff"  # NO!
```

Always run tests first.

### ❌ Vague commit messages
```bash
git commit -m "update files"  # NO!
git commit -m "fix"           # NO!
git commit -m "wip"           # Acceptable for personal branches only
```

### ❌ Committing broken code
Never commit code that doesn't compile or has failing tests (except WIP on personal branches).

### ❌ Large, unfocused commits
Break up commits into logical units. If you can't describe the commit succinctly, it's too large.

### ❌ Committing sensitive data
Never commit:
- Passwords, API keys, tokens
- `.env` files with secrets
- Credentials or certificates
- Personal data

Use `.gitignore` and be vigilant.

## Quality Checklist

Before pushing:
- [ ] All commits have descriptive messages
- [ ] No WIP commits (or squashed if on feature branch)
- [ ] Tests pass
- [ ] Code formatted
- [ ] Work log updated
- [ ] No sensitive data committed
- [ ] Commit history is clean and logical

## Outputs
- Clean git history with descriptive commits
- All tests passing
- Updated work logs
- Ready-to-review pull requests
