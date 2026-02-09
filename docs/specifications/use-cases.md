---
title: "Graft Use Cases"
status: draft
---

# Graft Use Cases

This document describes the core use cases that Graft enables. It focuses on **what users want to accomplish** rather than specific implementation details.

## Overview

Graft serves developers and teams who need to:
1. Manage dependencies from git repositories
2. Execute repeatable tasks across projects
3. Keep software current as dependencies evolve
4. Collaborate with AI coding agents on maintenance tasks

## Primary Use Cases

### 1. Share Code Across Projects

**Context**: A team has common code (libraries, configs, tooling) that multiple projects need to use. They want to share this code without publishing to package registries.

**Goal**: Reference and version-pin shared code from git repositories.

**Workflow**:
1. Define dependency in `graft.yaml` with git URL and ref
2. Run dependency resolution to fetch specific version
3. Lock to exact commit for reproducibility
4. Update to newer versions when ready

**Value**:
- No package registry infrastructure needed
- Direct dependency on source of truth (git repo)
- Fine-grained version control (any commit, branch, or tag)
- Works for any file type or language

**Example scenarios**:
- Shared configuration templates
- Internal libraries not ready for public release
- Documentation that multiple projects reference
- Tooling scripts used across team

---

### 2. Execute Consistent Tasks

**Context**: Different projects need to run similar tasks (build, test, deploy, lint) but with project-specific configurations.

**Goal**: Define executable tasks in a simple, portable format.

**Workflow**:
1. Define tasks in `graft.yaml`
2. Run tasks by name: `graft run <task>`
3. Tasks can depend on other tasks
4. Tasks can use environment variables and configuration

**Value**:
- Consistent interface across projects
- Self-documenting (tasks defined in code)
- Composable (tasks can call other tasks)
- Portable (not tied to specific tools)

**Example scenarios**:
- Build steps for different environments
- Test suites with different configurations
- Code generation tasks
- Deployment workflows

---

### 3. Stay Current with Upstream Changes

**Context**: A project depends on code from other repositories. Those dependencies evolve (bug fixes, new features, breaking changes). The project needs to stay reasonably current.

**Goal**: Understand what changed upstream and safely adopt updates.

**Workflow**:
1. Check for available updates
2. Review what changed (changelog, diff)
3. Understand impact on current project
4. Apply updates incrementally
5. Verify changes work correctly

**Value**:
- Visibility into available updates
- Context for understanding changes
- Controlled update process
- Reduced risk of breaking changes

**Example scenarios**:
- Security patches in dependencies
- New features to adopt
- Breaking changes that require code updates
- Bug fixes that affect workarounds

**Key challenges this addresses**:
- "What changed since last update?"
- "Will this break my code?"
- "How do I migrate to new API?"
- "How do I verify update worked?"

---

### 4. Migrate Through Breaking Changes

**Context**: A dependency has introduced breaking changes (API renamed, config format changed, behavior modified). The consuming project needs to adapt.

**Goal**: Update code to remain compatible with new dependency version.

**Workflow**:
1. Discover that update contains breaking changes
2. Understand what specifically broke and why
3. Get guidance on how to fix
4. Apply fixes (automated or manual)
5. Verify compatibility

**Value**:
- Clear migration path
- Automation of repetitive changes
- Reduced manual effort
- Confidence that migration is complete

**Example scenarios**:
- Function/class renamed
- Configuration format changed
- API signature modified
- Deprecated feature removed

**What dependencies can provide**:
- Structured changelog explaining changes
- Migration scripts for common changes
- Verification steps to confirm success
- Rationale for breaking changes

---

### 5. Collaborate with AI on Upgrades

**Context**: An AI coding agent assists with software maintenance. Dependency updates require understanding changes and modifying code accordingly.

**Goal**: Enable AI to help with upgrade process while keeping human in control.

**Workflow**:
1. AI detects available updates
2. AI reads structured changelog
3. AI analyzes impact on current codebase
4. AI proposes specific changes
5. Human reviews and approves
6. AI or human applies changes
7. Verify changes work

**Value**:
- Faster upgrade cycles
- Reduced tedious manual work
- AI handles repetitive transformations
- Human provides judgment and oversight

**What makes this work**:
- Structured, machine-readable metadata
- Clear migration guidance in changelogs
- Explicit verification strategies
- Atomic, reviewable changes

**Example scenarios**:
- Renaming function calls across many files
- Updating configuration files to new schema
- Adopting new API patterns
- Removing deprecated feature usage

---

### 6. Maintain Reproducible Environments

**Context**: Multiple developers and CI systems work on the same project. They need identical dependency versions.

**Goal**: Ensure everyone uses exactly the same version of each dependency.

**Workflow**:
1. Dependencies locked to specific commits
2. Lock file committed to version control
3. Anyone running `graft resolve` gets identical versions
4. Updates happen explicitly via `graft upgrade`

**Value**:
- Reproducible builds
- No "works on my machine" issues
- Controlled, intentional updates
- Audit trail of dependency changes

**Example scenarios**:
- CI/CD pipelines
- Onboarding new developers
- Reproducing historical builds
- Security audits

---

## Secondary Use Cases

### 7. Understand Dependency Health

**Context**: A project has multiple dependencies, some might be outdated or have security issues.

**Goal**: Get visibility into dependency status.

**Workflow**:
1. Check status of all dependencies
2. See which are outdated
3. Understand severity (patch vs. breaking)
4. Prioritize updates

**Value**: Proactive maintenance, security awareness

---

### 8. Test Pre-Release Versions

**Context**: A dependency is developing new features. Consumer wants to test before official release.

**Goal**: Use specific branch or commit from dependency.

**Workflow**:
1. Point dependency at feature branch
2. Test integration
3. Provide feedback to maintainers
4. Switch to release when available

**Value**: Earlier feedback, smoother adoption

---

### 9. Contribute Migration Guides

**Context**: A dependency released breaking changes but lacks migration documentation. Consumer figured out how to migrate.

**Goal**: Share migration knowledge to help others.

**Workflow**:
1. Perform migration manually
2. Document changes made
3. Contribute back to dependency
4. Future users benefit from guide

**Value**: Community knowledge sharing, improved upgrade experience

---

## Cross-Cutting Concerns

### Human + AI Collaboration

All workflows should support both:
- **Human execution**: Clear commands, readable output
- **AI assistance**: Structured metadata, actionable guidance

The interface should be semantically clear and useful to both.

### Evolvability

Software isn't static. Graft should make evolution easier:
- Discovery of changes
- Understanding of impact
- Automated application where safe
- Human oversight where needed

### Simplicity

Use cases should be achievable with minimal concepts:
- Dependencies (what you depend on)
- Versions (which version you use)
- Tasks (what you run)
- Changes (what's new)

Avoid unnecessary complexity or special cases.

---

## Status

This document is **draft** status. Use cases will be refined as Graft's design evolves.

## Related

- [Architecture](architecture.md) - How Graft implements these use cases
- [Decision 0001: Initial Scope](decisions/decision-0001-initial-scope.md) - What's in/out of scope
- [Brainstorming: Upgrade Mechanisms](../../notes/2026-01-01-upgrade-mechanisms.md) - Exploring solution approaches
