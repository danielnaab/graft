# Path Display Improvements Plan

## Context

Recent work added adaptive path compaction that:
1. Calculates overhead dynamically based on status width
2. Drops branch name when path would be severely compacted
3. Uses fish-style abbreviation and prefix truncation

Code review identified several issues to address.

## Issues & Proposed Fixes

### P0: Fix width threshold bug

**Issue:** Line 556 uses `.len()` instead of `.width()` for display width check
```rust
let use_branch = !compacted_path_with_branch.starts_with("[..]")
    && compacted_path_with_branch.len() >= 8;  // BUG: len != width
```

**Fix:**
```rust
let use_branch = !compacted_path_with_branch.starts_with("[..]")
    && compacted_path_with_branch.width() >= 8;
```

**Test:** Add unicode path test to verify width-based threshold

---

### P1: Show repository basename in extremely tight spaces

**Issue:** In very tight spaces (pane_width < 15), showing `~/s ●` doesn't help identify the repo

**Proposed strategy:**
1. If pane_width < 15 (very tight), show just basename: `graft ●`
2. If pane_width 15-25 (tight), show compacted path without branch: `~/src/graft ●`
3. If pane_width > 25 (normal), show with branch: `~/src/graft [main] ●`

**Implementation:**
- Add helper: `fn extract_basename(path: &str) -> &str`
- Add tier: very tight (basename only), tight (no branch), normal (with branch)

**Spec update:** Add scenario for extremely tight spaces showing basename only

---

### P1: Verify and fix margin overhead calculation

**Issue:** `let overhead = 2 + status_width + 3;` - unclear what `+ 3` represents

**Investigation needed:**
- Measure actual List widget overhead
- Check if highlight symbol is already in pane_width or separate
- Document what the margin actually accounts for

**Action:** Add test that verifies actual rendered width matches calculation

---

### P2: Add test coverage for adaptive branch display

**Missing tests:**
- Branch shown when path width allows (>= threshold)
- Branch dropped when path < threshold
- Branch dropped when path uses [..]
- Unicode path triggers width-based (not len-based) threshold

**Implementation:** Add tests in tui_tests.rs under "Adaptive display tests"

---

### P3: Consider caching formatted lines (performance)

**Issue:** For large repo lists (50+ repos), formatting on every render could impact performance

**Proposed approach:**
- Cache: `HashMap<(RepoPath, Option<RepoStatus>, u16), Line<'static>>`
- Invalidate when: status changes, pane width changes
- Benchmark: measure impact with 100 repos

**Note:** Might be premature optimization - defer unless user reports lag

---

### P4: Make threshold configurable or remove from decision

**Issue:** Spec decision mentions "8 chars" but it's an implementation detail

**Options:**
1. Add to Constraints: `MIN_PATH_WIDTH_WITH_BRANCH: 8`
2. Remove from decision, keep as internal implementation detail
3. Make configurable in workspace.yaml

**Recommendation:** Option 2 (remove from decision) - it's an internal heuristic, not a user-facing constraint

## Implementation Order

1. **Fix P0 bug** (`.len()` → `.width()`) - 5 min
2. **Add P2 tests** (adaptive display coverage) - 20 min
3. **Implement P1** (basename extraction for very tight spaces) - 30 min
4. **Investigate P1** (verify margin overhead) - 15 min
5. **Defer P3** (caching) - only if performance issue reported
6. **Update P4** (remove threshold from spec decision) - 5 min

Total: ~75 min of work

## Success Criteria

- [ ] No byte-count bugs (all width checks use `.width()`)
- [ ] Adaptive display has test coverage
- [ ] Very tight spaces show useful repo identifier (basename)
- [ ] Overhead calculation is verified and documented
- [ ] Spec reflects observable behavior, not implementation details
