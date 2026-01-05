---
title: "Plan: Testing graft-knowledge v2 Upgrade"
date: 2026-01-05
status: draft
version: 1.0
---

# Plan: Testing graft-knowledge v2 Upgrade

## Purpose

Test the complete v2 upgrade implementation by actually resolving the graft-knowledge dependency. This serves as both validation of the implementation and a real-world test of the upgrade process.

## Objectives

1. **Validate Implementation**: Ensure all v2 features work correctly
2. **Test Real Workflow**: Use graft as intended (resolve dependencies)
3. **Identify Issues**: Discover any bugs or edge cases
4. **Document Experience**: Record what works well and what doesn't
5. **Generate Insights**: Collect data for post-implementation analysis

## Test Scenarios

### Scenario 1: Fresh Resolve

**Goal**: Test from clean state

**Steps**:
1. Ensure no existing `.graft/` directory or `graft.lock`
2. Run `graft resolve`
3. Verify graft-knowledge cloned to `.graft/graft-knowledge/`
4. Verify lock file created with v2 format
5. Check if graft-knowledge has any transitive dependencies

**Expected Behavior**:
- graft-knowledge should be marked as `direct: true`
- Lock file should contain all transitive deps (if any)
- Dependencies should be in `.graft/` directory
- All v2 fields should be populated correctly

**Success Criteria**:
- [ ] Dependency clones successfully
- [ ] Lock file created with `apiVersion: graft/v0`
- [ ] All v2 fields present and correct
- [ ] No errors or warnings

### Scenario 2: Tree Visualization

**Goal**: Test tree command with real data

**Steps**:
1. After Scenario 1 completes
2. Run `graft tree`
3. Run `graft tree --show-all`

**Expected Behavior**:
- Tree shows graft-knowledge as direct dependency
- If transitive deps exist, they appear in tree
- Output is readable and informative

**Success Criteria**:
- [ ] Tree displays correctly
- [ ] Direct deps clearly marked
- [ ] Transitive deps (if any) shown with relationships
- [ ] Detailed view shows all metadata

### Scenario 3: Lock File Validation

**Goal**: Verify lock file structure

**Steps**:
1. After Scenario 1 completes
2. Manually inspect `graft.lock`
3. Verify format matches specification
4. Check all required fields present

**Expected Behavior**:
- `apiVersion: graft/v0` at top
- Dependencies section with all deps
- Each entry has all v2 fields
- Direct/transitive properly marked

**Success Criteria**:
- [ ] Lock file is valid YAML
- [ ] Format matches v2 specification exactly
- [ ] All commit hashes are 40-character SHA-1
- [ ] Timestamps are valid ISO 8601

### Scenario 4: Re-resolve (Idempotent)

**Goal**: Test that resolve is idempotent

**Steps**:
1. After Scenario 1 completes
2. Run `graft resolve` again
3. Verify no changes to existing state

**Expected Behavior**:
- Should fetch but not re-clone
- Lock file should not change (same commits)
- No errors or warnings

**Success Criteria**:
- [ ] Second resolve completes successfully
- [ ] Lock file unchanged (or only timestamp updated)
- [ ] No duplicate work performed

### Scenario 5: Inspect Cloned Repository

**Goal**: Verify dependency is usable

**Steps**:
1. After Scenario 1 completes
2. Navigate to `.graft/graft-knowledge/`
3. Check git status
4. Read documentation files
5. Verify correct ref checked out

**Expected Behavior**:
- Repository is valid git repo
- Correct ref/branch checked out
- All files accessible
- Can read documentation

**Success Criteria**:
- [ ] Git repository is clean
- [ ] Correct commit hash matches lock file
- [ ] Documentation files are readable
- [ ] No corruption or missing files

### Scenario 6: Conflict Detection (Optional)

**Goal**: Test conflict detection if possible

**Setup**: This requires creating a test scenario with conflicts

**Steps**:
1. Manually create a test graft.yaml with conflicting deps
2. Run `graft resolve`
3. Verify clear error message

**Expected Behavior**:
- Clear error about conflict
- Shows which deps conflict
- Provides actionable guidance

**Success Criteria**:
- [ ] Conflict detected correctly
- [ ] Error message is clear and helpful
- [ ] Process fails gracefully

## Data Collection

### Metrics to Record

**Performance**:
- Time to clone graft-knowledge
- Time to parse its graft.yaml
- Total resolution time
- Lock file write time

**Quality**:
- Number of dependencies resolved
- Direct vs transitive count
- Lock file size
- Any warnings or errors

**User Experience**:
- Clarity of output messages
- Usefulness of tree visualization
- Intuitiveness of directory layout
- Documentation clarity

### Observations to Note

**What Worked Well**:
- Which features felt natural
- Clear and helpful output
- Smooth workflows

**Pain Points**:
- Confusing messages
- Unexpected behavior
- Missing features
- Unclear documentation

**Surprises**:
- Unexpected dependencies
- Performance issues
- Undocumented behavior

## Execution Plan

### Phase 1: Clean Setup (5 min)

1. Navigate to graft directory
2. Remove any existing `.graft/` directory
3. Remove any existing `graft.lock`
4. Verify `graft.yaml` has graft-knowledge dependency
5. Note starting state

### Phase 2: Execute Scenarios (15 min)

Execute each scenario sequentially, documenting results:

1. Run Scenario 1 (Fresh Resolve)
2. Run Scenario 2 (Tree Visualization)
3. Run Scenario 3 (Lock File Validation)
4. Run Scenario 4 (Re-resolve)
5. Run Scenario 5 (Inspect Repository)
6. Run Scenario 6 (Conflict Detection) - if time permits

### Phase 3: Documentation (10 min)

1. Record all observations
2. Screenshot interesting output
3. Note any issues encountered
4. Collect metrics

### Phase 4: Analysis (10 min)

1. Review what worked vs what didn't
2. Identify patterns in issues
3. Categorize findings
4. Prepare insights for analysis document

## Success Definition

**Minimum Success**:
- Scenario 1 completes without errors
- Lock file created correctly
- Dependency usable

**Full Success**:
- All 6 scenarios pass
- No bugs discovered
- Excellent user experience
- Complete data for analysis

**Exceptional Success**:
- Identifies opportunities for improvement
- Discovers edge cases to handle
- Generates actionable recommendations

## Risk Assessment

### Risk 1: Network Issues

**Risk**: Git clone might fail due to network

**Mitigation**:
- Test from stable environment
- Have offline fallback
- Document any network-related issues

### Risk 2: graft-knowledge Format Changes

**Risk**: graft-knowledge might have format we don't support

**Mitigation**:
- Check graft-knowledge repository first
- Be prepared to handle edge cases
- Document any specification gaps

### Risk 3: Performance Issues

**Risk**: Resolution might be slow

**Mitigation**:
- Note performance metrics
- Identify bottlenecks
- Document for optimization recommendations

## Related Documents

- [Implementation Plan](./upgrade-to-graft-knowledge-v2.md)
- [Upgrade Analysis](./upgrade-analysis.md) - will be updated with findings
- [Improvement Recommendations](./graft-improvements-recommendations.md) - will be informed by testing

## Changelog

- **2026-01-05**: Initial testing plan created
  - Defined 6 test scenarios
  - Established metrics and observations
  - Outlined execution plan
