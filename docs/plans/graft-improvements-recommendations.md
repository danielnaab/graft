---
title: "Recommendations: Graft Dependency Management Improvements"
date: 2026-01-05
status: draft
version: 0.1
---

# Recommendations: Graft Dependency Management Improvements

## Purpose

This document proposes specific improvements to Graft based on the experience of upgrading to graft-knowledge v2 specifications. The recommendations focus on making dependency management smoother, more intuitive, and better aligned with Graft's philosophy.

## Status

✅ **Complete** - Based on v2 upgrade implementation and testing

See:
- [Implementation Plan](./upgrade-to-graft-knowledge-v2.md) - ✅ All 7 phases completed
- [Upgrade Analysis](./upgrade-analysis.md) - ✅ Comprehensive analysis with 18 lessons
- [Testing Results](./testing-v2-upgrade.md) - ✅ 5/5 scenarios passed

## Recommendation Framework

Each recommendation follows this structure:

1. **Problem Statement**: What pain point does this address?
2. **Current State**: How does Graft currently handle this?
3. **Proposed Solution**: What should change?
4. **Design Considerations**: What options exist? What are the trade-offs?
5. **Implementation Notes**: How would this be built?
6. **Success Criteria**: How would we measure improvement?
7. **Priority**: High/Medium/Low based on impact and effort

## Recommendations

### CLI & User Experience

#### Recommendation 1: Add `graft status` Command

**Problem Statement:**
Users have no way to know if their dependencies are up-to-date without manually checking each repository. There's no equivalent to `npm outdated` or `cargo outdated` for Graft.

**Current State:**
- Users must manually compare lock file commits to upstream
- No visibility into available updates
- No way to know if resolve is needed

**Proposed Solution:**
Add `graft status` command that shows:
- Which dependencies have newer commits available
- Current commit vs latest commit on tracked ref
- Whether local .graft/ directory is in sync with lock file
- Whether lock file is in sync with graft.yaml

**Design Considerations:**

- **Option A: Simple commit comparison**
  - Pros: Fast, easy to implement
  - Cons: Doesn't account for version tags, just raw commits

- **Option B: Semantic version aware** (if deps use tags)
  - Pros: More meaningful "v1.2.0 → v1.3.0" vs commit hashes
  - Cons: Requires parsing git tags, more complex

- **Recommended: Start with Option A, extend to B later**

**Implementation Notes:**
```python
# New file: src/graft/cli/commands/status.py
def status_command():
    # 1. Read graft.yaml and graft.lock
    # 2. For each dependency:
    #    - git ls-remote to get latest commit on ref
    #    - Compare with lock file commit
    #    - Check if local .graft/<name> exists and is at right commit
    # 3. Display results in table format
```

Files to modify:
- `src/graft/cli/commands/status.py` (new)
- `src/graft/cli/main.py` (register command)
- `src/graft/services/git_operations.py` (add remote_ref_commit() method)

**Success Criteria:**
- [ ] Can detect when dependencies have updates available
- [ ] Shows clear comparison: current vs latest
- [ ] Runs in <5 seconds for typical project (5-10 deps)
- [ ] Handles network errors gracefully
- [ ] Exit code 0 if up-to-date, 1 if updates available

**Priority:** High - Common user need, clear value

---

#### Recommendation 2: Add `graft validate` Command

**Problem Statement:**
No way to validate graft.yaml or graft.lock without actually running resolve. Need pre-commit hook friendly validation.

**Current State:**
- Errors only surface during resolve
- Can't catch syntax errors early
- No CI-friendly validation

**Proposed Solution:**
Add `graft validate` command with multiple modes:
- `graft validate config` - Check graft.yaml syntax and semantics
- `graft validate lock` - Check graft.lock format and consistency
- `graft validate all` - Both + check lock matches config

**Design Considerations:**

- **Validation levels:**
  - Syntax: Valid YAML, required fields present
  - Semantics: Valid git URLs, reasonable refs
  - Consistency: Lock file matches config dependencies
  - Integrity: Commits exist, no cycles in dep graph

- **Error reporting:**
  - Clear messages for each validation failure
  - Multiple errors reported, not just first one
  - Suggestions for fixes

**Implementation Notes:**
```python
# New file: src/graft/cli/commands/validate.py
def validate_command(mode: str = "all"):
    validators = {
        "config": [validate_yaml_syntax, validate_graft_config],
        "lock": [validate_lock_syntax, validate_lock_semantics],
        "all": [all_above + validate_consistency]
    }
    # Run validators, collect errors, report
```

**Success Criteria:**
- [ ] Catches all common config errors
- [ ] Runs in <2 seconds (no network calls)
- [ ] Clear error messages with line numbers
- [ ] Exit code 0 if valid, 1 if invalid
- [ ] Works as pre-commit hook

**Priority:** High - Enables CI/CD integration, prevents bad commits

---

#### Recommendation 3: Add `graft upgrade` Command

**Problem Statement:**
No automated way to update dependencies to latest versions. Users must manually edit graft.yaml.

**Current State:**
- Manual process: check what's new, edit config, resolve
- Error-prone (typos, wrong commit hashes)
- Time-consuming for projects with many deps

**Proposed Solution:**
Add `graft upgrade` command with modes:
- `graft upgrade --all` - Update all dependencies to latest on their refs
- `graft upgrade <dep-name>` - Update specific dependency
- `graft upgrade --interactive` - Choose which deps to update

**Design Considerations:**

- **Update strategy:**
  - Option A: Update refs (main → latest commit on main)
  - Option B: Smart version bumping (v1.2 → v1.3 if available)
  - Recommended: A for v1, B for future enhancement

- **Safety:**
  - Show what will change before applying
  - Require confirmation (unless --yes flag)
  - Create backup of graft.lock
  - Rollback on failure

**Implementation Notes:**
```python
# New file: src/graft/cli/commands/upgrade.py
def upgrade_command(deps: list[str], interactive: bool, dry_run: bool):
    # 1. For each dependency (or --all):
    #    - Fetch latest commit on tracked ref
    #    - Show current → new
    # 2. If interactive: prompt for each
    # 3. Update graft.yaml (or suggest user do it)
    # 4. Run resolve to update lock file
```

**Success Criteria:**
- [ ] Successfully updates dependencies
- [ ] Clear preview of changes
- [ ] Safe rollback if resolve fails
- [ ] Handles network errors gracefully
- [ ] Preserves comments in graft.yaml

**Priority:** Medium - High value but complex implementation

---

### Visualization & Introspection

#### Recommendation 4: Enhance `graft tree` Command

**Problem Statement:**
Current tree command is good but could be more powerful for large dependency graphs and programmatic use.

**Current State:**
- Basic tree view works well
- `--show-all` provides details
- No filtering, depth limiting, or export options

**Proposed Solution:**
Enhance tree command with:
- `--depth N` - Limit tree depth
- `--json` - JSON output for tooling
- `--filter <pattern>` - Show only matching deps
- `--format dot` - Graphviz DOT format for visualization

**Design Considerations:**

- **Output formats:**
  - Keep current text format as default
  - Add JSON for programmatic use
  - Add DOT for visual diagrams
  - Consider adding Mermaid format

- **Filtering:**
  - By name pattern (glob or regex)
  - By depth (only direct, or max 2 levels)
  - By status (outdated only)

**Implementation Notes:**
```python
# Modify: src/graft/cli/commands/tree.py
def tree_command(
    show_all: bool = False,
    depth: int | None = None,
    output_format: str = "text",
    filter_pattern: str | None = None
):
    # Existing logic + new formatting/filtering
```

**Success Criteria:**
- [ ] `--depth` limits output correctly
- [ ] `--json` produces valid JSON
- [ ] `--format dot` works with graphviz
- [ ] `--filter` matches expected deps
- [ ] Backward compatible (no breaking changes)

**Priority:** Medium - Nice enhancement, not critical

---

### Validation & Integrity

#### Recommendation 5: Integration Test Suite

**Problem Statement:**
Two bugs found during testing were integration issues that unit tests didn't catch. Need better integration testing.

**Current State:**
- Unit tests for individual components
- Manual integration testing
- No CI integration tests

**Proposed Solution:**
Add comprehensive integration test suite:
- Test full workflows (resolve, tree, validate)
- Test with real git repositories (or fixtures)
- Run in CI on every commit
- Cover error paths and edge cases

**Design Considerations:**

- **Test data approach:**
  - Option A: Mock git repos in tests/ directory
  - Option B: Use real public repos (graft-knowledge)
  - Option C: Hybrid - mocks for speed, real for validation
  - Recommended: C

- **CI Integration:**
  - Run on PRs and main branch
  - Fast feedback (<2 min)
  - Clear failure messages

**Implementation Notes:**
```python
# New: tests/integration/test_full_workflow.py
def test_resolve_with_transitive_deps():
    # Set up test environment
    # Run graft resolve
    # Verify lock file, .graft/ directory
    # Verify tree output

def test_idempotent_resolve():
    # Resolve twice, verify no changes
```

**Success Criteria:**
- [ ] Integration tests cover all commands
- [ ] CI runs integration tests automatically
- [ ] Tests run in <2 minutes
- [ ] Tests catch the types of bugs we found
- [ ] Clear documentation for writing integration tests

**Priority:** High - Prevents bugs, improves confidence

---

### Migration & Upgrades

#### Recommendation 6: CI/CD Integration Guide

**Problem Statement:**
No documentation or examples for using Graft in CI/CD pipelines.

**Current State:**
- Users must figure out CI integration themselves
- No best practices documented
- No example workflows

**Proposed Solution:**
Create comprehensive CI/CD integration guide with:
- GitHub Actions workflow example
- GitLab CI template
- Generic shell script approach
- Best practices and tips

**Design Considerations:**

- **Common patterns to document:**
  - Validate on PR (graft validate)
  - Check for outdated deps (graft status)
  - Fail if lock file out of sync
  - Cache .graft/ directory for speed

- **Security considerations:**
  - SSH key management
  - Private repository access
  - Credential handling

**Implementation Notes:**
```yaml
# New: examples/ci/github-actions.yml
name: Graft CI
on: [pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Validate graft config
        run: graft validate
      - name: Check dependencies status
        run: graft status
```

Files to add:
- `examples/ci/github-actions.yml`
- `examples/ci/gitlab-ci.yml`
- `examples/ci/generic-script.sh`
- `docs/ci-integration.md`

**Success Criteria:**
- [ ] Examples work out-of-box for simple cases
- [ ] Documentation covers common scenarios
- [ ] Security best practices documented
- [ ] Examples for public and private repos

**Priority:** Medium - Helps adoption, not critical for core functionality

---

### Specification Evolution

These recommendations are for the graft-knowledge repository and the specification evolution process:

#### Recommendation 7: Specification Enhancements for graft-knowledge

**Problem Statement:**
During implementation, several minor gaps in the specification were discovered that would help future implementers.

**Current State:**
- Lock file v2 spec is good but missing some details
- No migration guide from v1 to v2
- Some conventions emerged organically (ordering)
- Version semantics unclear

**Proposed Solution:**
Enhance graft-knowledge specifications with:

1. **Lock file ordering convention**
   - Specify that direct deps should come before transitive
   - Rationale: Improves readability
   - Example showing both ordered and unordered

2. **API version semantics**
   - Clarify what `graft/v0`, `graft/v1` mean
   - Document when to bump versions
   - Add compatibility matrix

3. **Conflict detection examples**
   - Show scenario where conflicts occur
   - Document expected error message format
   - Provide resolution strategies

4. **Migration guide section**
   - "Upgrading from v1 to v2" document
   - Explain automatic migration strategy
   - Timeline for v1 deprecation

5. **Extended examples**
   - 3-level dependency chain
   - Conflict scenarios
   - Cycle detection (if relevant)

6. **Decision log**
   - Why flat layout?
   - Why tuples for requires/required_by?
   - Design rationale helps implementers

**Design Considerations:**

- **Balance detail vs clarity:**
  - Don't over-specify every detail
  - Trust implementers for minor decisions
  - Focus on critical semantics

- **Living specification:**
  - Link to reference implementation (graft repo)
  - Code and spec co-evolve
  - Examples from real usage

**Implementation Notes:**
Files to modify in graft-knowledge:
- `docs/specification/lock-file-format.md`
- `docs/specification/dependency-layout.md`
- `docs/guides/migration-v1-to-v2.md` (new)
- `docs/decisions/` (new directory for ADRs)

**Success Criteria:**
- [ ] All identified gaps addressed
- [ ] Migration guide complete and tested
- [ ] Examples cover common scenarios
- [ ] Version semantics documented
- [ ] Decision rationale captured

**Priority:** High - Benefits all future implementations

---

## Prioritization

### High Priority
*(Recommendations with immediate impact and clear implementation path)*

- **Rec #2: `graft validate` command** - Enables CI/CD, prevents bad commits, relatively simple to implement
- **Rec #5: Integration test suite** - Prevents bugs like we found, improves reliability, essential for quality
- **Rec #7: Specification enhancements** - Benefits all future implementations, small effort, high impact
- **Rec #1: `graft status` command** - Common user need ("are my deps outdated?"), clear value proposition

### Medium Priority
*(Valuable improvements that can wait for next phase)*

- **Rec #4: Enhance `graft tree`** - Nice features but current tree is functional
- **Rec #6: CI/CD integration guide** - Documentation work, helps adoption but not blocking
- **Rec #3: `graft upgrade` command** - High value but complex, can be manual workflow for now

### Low Priority
*(Nice-to-haves or future enhancements)*

- Graph visualization (DOT/Mermaid export) - Cool but niche use case
- Semantic versioning support - Needs more design work
- Plugin architecture - Interesting but premature

## Implementation Roadmap

*(Suggested order of implementation based on dependencies and priorities)*

### Phase 1: Quality & Validation (1-2 weeks)
*Foundation for reliability and CI/CD integration*

**Week 1:**
- **Rec #5: Integration test suite**
  - Set up test framework
  - Add workflow tests (resolve, tree)
  - Configure CI to run tests
  - Impact: Prevents future bugs

- **Rec #7: Specification enhancements**
  - Update graft-knowledge docs
  - Add migration guide
  - Document conventions
  - Impact: Helps future implementers

**Week 2:**
- **Rec #2: `graft validate` command**
  - Implement validation logic
  - Add to CLI
  - Write tests
  - Document usage
  - Impact: Enables pre-commit hooks, CI checks

**Deliverables:**
- Integration tests running in CI
- Enhanced graft-knowledge spec
- Working `graft validate` command

### Phase 2: User Experience (2-3 weeks)
*Make dependency management more intuitive*

**Week 3:**
- **Rec #1: `graft status` command**
  - Implement remote ref checking
  - Add status display logic
  - Handle network errors gracefully
  - Write tests and docs
  - Impact: Users can see outdated deps

**Week 4-5:**
- **Rec #6: CI/CD integration guide**
  - Write GitHub Actions example
  - Write GitLab CI example
  - Document best practices
  - Test examples in real repos
  - Impact: Easier CI adoption

- **Rec #4: Enhance `graft tree`** (partial)
  - Add `--depth` flag
  - Add `--json` output
  - Impact: Better tooling integration

**Deliverables:**
- `graft status` command
- Complete CI/CD guide with examples
- Enhanced tree command

### Phase 3: Advanced Features (4-6 weeks)
*Powerful features for complex workflows*

**Week 6-8:**
- **Rec #3: `graft upgrade` command**
  - Design safe upgrade workflow
  - Implement interactive mode
  - Add rollback capability
  - Extensive testing
  - Impact: Automated dependency updates

**Week 9-11:**
- **Tree visualization enhancements**
  - DOT format export
  - Mermaid format
  - Diagram generation scripts
  - Impact: Better documentation, presentations

- **Semantic versioning support**
  - Design version-aware resolution
  - Tag-based version detection
  - Upgrade strategies
  - Impact: Smarter dependency management

**Deliverables:**
- `graft upgrade` command
- Graph visualization tools
- Version-aware features

### Phase 4: Polish & Ecosystem (ongoing)
*Community, documentation, and ecosystem growth*

- Plugin architecture exploration
- Additional CI platform examples
- Video tutorials and guides
- Community feedback incorporation
- Performance optimizations

## Alignment with Graft Philosophy

Each recommendation should align with Graft's core principles:

- **Knowledge dependency management**: Focus on knowledge bases, not just code
- **Reproducibility**: Clear, auditable dependency states
- **Transparency**: No magic, inspectable structures
- **Atomicity**: All-or-nothing operations with rollback
- **Simplicity**: Minimize complexity, maximize clarity

## Exploration Areas

*(Ideas that need more research before becoming recommendations)*

### Idea 1: Workspace Support (Monorepo)

**Context:**
Projects may have multiple knowledge bases in one repository, each with its own graft.yaml. How should Graft handle workspace-style setups?

**Open Questions:**
- Should there be a workspace root graft.yaml?
- How to avoid duplicate dependency resolution?
- Shared .graft/ directory or per-workspace?
- How does this interact with transitive deps?

**Next Steps:**
- [ ] Research how npm/cargo handle workspaces
- [ ] Design workspace-aware dependency resolution
- [ ] Prototype implementation
- [ ] Gather user feedback on use cases

---

### Idea 2: Content Addressing / Integrity Verification

**Context:**
Currently we trust git commit hashes. Should we add content integrity verification (checksums of resolved content)?

**Open Questions:**
- Is git commit hash sufficient for integrity?
- What attack vectors exist?
- Performance impact of checksumming?
- How to handle when content changes without commit hash change?

**Next Steps:**
- [ ] Threat modeling for dependency integrity
- [ ] Research similar tools (npm package-lock, cargo Cargo.lock)
- [ ] Evaluate cost/benefit
- [ ] Design verification mechanism if needed

---

### Idea 3: Partial Dependency Resolution

**Context:**
For large projects, resolving all dependencies might be slow. Could we support partial resolution (only what's needed)?

**Open Questions:**
- How to determine "what's needed"?
- Impact on reproducibility?
- Complexity vs benefit trade-off?
- Use cases where this matters?

**Next Steps:**
- [ ] Identify real-world projects with slow resolution
- [ ] Profile current resolution performance
- [ ] Design lazy/partial resolution strategy
- [ ] Benchmark potential improvements

---

### Idea 4: Dependency Caching / Mirror Support

**Context:**
Corporate environments might need dependency caching or mirrors for reliability/security.

**Open Questions:**
- Should Graft support custom git mirrors?
- How to configure mirror fallback?
- Cache invalidation strategy?
- Authentication handling?

**Next Steps:**
- [ ] Survey enterprise user needs
- [ ] Research git mirror protocols
- [ ] Design cache/mirror configuration
- [ ] Prototype mirror support

---

## Related Documents

- [Implementation Plan](./upgrade-to-graft-knowledge-v2.md)
- [Upgrade Analysis](./upgrade-analysis.md)
- [graft: Architecture Overview](../README.md)
- [graft-knowledge: Architecture Decisions](../../../graft-knowledge/docs/decisions/)

## Changelog

- **2026-01-05**: Initial placeholder document created
  - Defined recommendation framework
  - Set up structure for systematic proposals
  - Outlined prioritization and roadmap sections
- **2026-01-05**: Recommendations completed
  - **7 concrete recommendations** across all categories:
    - Rec #1: `graft status` command
    - Rec #2: `graft validate` command
    - Rec #3: `graft upgrade` command
    - Rec #4: Enhanced `graft tree` command
    - Rec #5: Integration test suite
    - Rec #6: CI/CD integration guide
    - Rec #7: Specification enhancements for graft-knowledge
  - **Prioritization**: Categorized as High/Medium/Low priority
  - **4-phase roadmap**: Quality→UX→Advanced→Ecosystem
  - **4 exploration areas**: Workspaces, integrity, partial resolution, caching
  - All recommendations follow systematic framework
  - Based on real implementation experience and testing
