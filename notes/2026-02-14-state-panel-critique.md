---
status: working
date: 2026-02-14
purpose: "Identify Phase 2/3 improvements for state panel UX"
note: "Phase 1 complete (A-). Defer Phase 2/3 until user demand observed."
---

# Grove State Panel - Critical Critique & Improvement Plan

**Current Grade**: B+ (85%) → A- (90%) after Phase 1
**Target Grade**: A (95%)

---

## Executive Summary

The state panel is now **production ready** with comprehensive test coverage, error handling, and documentation. However, several architectural and UX improvements would elevate it from "good enough to ship" to "excellent user experience."

**Key Findings**:
- ✓ **Strengths**: Solid foundation, well-tested, error handling works
- ✗ **Weaknesses**: Limited interactivity, no refresh capability, minimal metadata shown
- ⚠ **Risks**: User confusion about cache staleness, no way to update data

---

## Critical Issues (Must Fix for A Grade)

### 1. **No Cache Freshness Indicators** 🔴 HIGH PRIORITY

**Problem**: Users can't tell if cached data is stale.

```
Current Display:
  ▶ writing      5000 words total, 250 today
    tasks        59 open, 49 done
    graph        2223 broken links, 463 orphans
```

Users see numbers but have **no idea**:
- When was this data generated?
- Is this from today or last week?
- Do I need to refresh it?

**Impact**:
- Users might make decisions based on outdated data
- No visibility into cache age without inspecting files manually
- Undermines trust in the data shown

**Solution**: Add timestamp display

```
Better Display:
  ▶ writing      5000 words total, 250 today        (5m ago)
    tasks        59 open, 49 done                    (2h ago)
    graph        2223 broken links, 463 orphans      (3d ago)
```

**Implementation**:
- Already have `metadata.timestamp` in `StateResult`
- Just need to render it with `time_ago()` helper (already exists!)
- Update rendering in `render_state_panel_overlay()` (~line 1230)

**Effort**: 30 minutes
**Value**: High - Critical for data trustworthiness

---

### 2. **No Refresh Action** 🔴 HIGH PRIORITY

**Problem**: Users can't update stale cache from within Grove.

**Current UX**:
1. User sees "59 open tasks (3d ago)"
2. Thinks "That's probably outdated"
3. Has to:
   - Exit Grove (q)
   - Run `graft state query tasks --refresh`
   - Restart Grove
   - Navigate back to repo
   - Press 's' again

**Impact**:
- Breaks flow for checking fresh state
- Makes state panel feel like a "view-only report" not a live tool
- Users will just not use it if it's too stale

**Solution**: Add 'r' keybinding to refresh selected query

```
State Panel (with refresh):
  ▶ writing      5000 words total, 250 today        (5m ago)  [Press 'r' to refresh]
    tasks        59 open, 49 done                    (3d ago)
    graph        2223 broken links, 463 orphans      (1w ago)
```

Press 'r' → Executes `graft state query tasks --refresh` → Updates display

**Implementation Approach**:
```rust
// In handle_key_state_panel()
KeyCode::Char('r') => {
    if let Some(selected) = self.state_panel_list_state.selected() {
        if let Some(query) = self.state_queries.get(selected) {
            self.refresh_state_query(&query.name);
        }
    }
}

fn refresh_state_query(&mut self, query_name: &str) {
    // Show "Refreshing..." status message
    self.status_message = Some(StatusMessage::info(
        format!("Refreshing {}...", query_name)
    ));

    // Execute: graft state query <name> --refresh
    // (Similar to command execution flow, but simpler)
    // Update cache
    // Reload state_results
}
```

**Challenges**:
- Need to spawn subprocess for `graft state query`
- Need to handle async execution (running vs completed)
- Need to show progress/spinner while refreshing

**Effort**: 2-3 hours
**Value**: High - Makes state panel actually useful for live monitoring

---

### 3. **No Detail View for Query Results** 🟡 MEDIUM PRIORITY

**Problem**: Users can only see summary, not full data.

**Current**: Show "2223 broken links, 463 orphans"
**Desired**: Press Enter → See full JSON with metadata

```
State Query Detail View
════════════════════════════════════════════════════════════════
Query: graph
Command: python scripts/analyze-graph.py
Commit: abc123def (main branch)
Timestamp: 2026-02-14 10:30:15 UTC (2h ago)
Deterministic: true
────────────────────────────────────────────────────────────────
{
  "broken_links": 2223,
  "orphaned": 463,
  "total_nodes": 15432,
  "total_edges": 28901,
  "clusters": 42
}
────────────────────────────────────────────────────────────────
[q/Esc: back  r: refresh]
```

**Impact**:
- Users can't investigate *why* summary shows certain values
- No access to extra fields that aren't in summary format
- Can't see metadata (commit hash, command run, etc.)

**Solution**: Add detail pane for selected query

**Implementation**:
- Add `ActivePane::StateQueryDetail` enum variant
- Press Enter on selected query → transition to detail view
- Render full JSON with pretty formatting
- Show all metadata fields
- 'q' returns to state panel list

**Effort**: 2-3 hours
**Value**: Medium - Power users want this, but summary is enough for most

---

## Architectural Issues

### 4. **Module Location Confusion** 🟡 MEDIUM PRIORITY

**Problem**: State module code is duplicated between `lib.rs` and `main.rs`

**Current Structure**:
```
grove/src/
├── lib.rs         # Re-exports state module for tests
├── main.rs        # Contains state module implementation
└── state/         # Actual module files
    ├── mod.rs
    ├── cache.rs
    ├── discovery.rs
    └── query.rs
```

**Why This Is Weird**:
- `main.rs` contains `pub mod state;` but state is only for lib/TUI
- Integration tests import `grove::state::*` but it's in the binary crate
- Violates single-source-of-truth principle

**Impact**:
- Confusing for new contributors
- Binary bloat (state code compiled into CLI even if never used)
- Testing architecture is backwards

**Solution**: Move state module to `lib.rs` only

```
grove/src/
├── lib.rs         # pub mod state; pub mod tui;
├── main.rs        # Just CLI entry point
└── state/         # Pure library code
```

**Implementation**:
1. Remove `pub mod state;` from `main.rs`
2. Keep `pub mod state;` in `lib.rs`
3. Import in `main.rs` via `use grove::state::*;`
4. Verify tests still work

**Effort**: 30 minutes
**Value**: Low impact now, but good hygiene

---

### 5. **Tight Coupling to `graft state query` CLI** 🟡 MEDIUM PRIORITY

**Problem**: State panel assumes `graft state query` exists and works.

**Current Flow**:
```
Grove TUI → Reads cache files directly from ~/.cache/graft/
          ↓
     Assumes graft CLI populated them
```

**What Breaks**:
- If user doesn't have `graft` installed → Silent failure
- If `graft state query` command changes format → Parse errors
- If cache structure changes → Incompatible

**Impact**:
- Fragile integration between Grove (Rust) and graft (Python)
- No version compatibility checks
- Difficult to test in isolation

**Better Architecture**:
```
grove/src/state/
├── provider.rs   # Trait: StateQueryProvider
├── graft.rs      # Implementation: GraftStateProvider
└── mock.rs       # For testing
```

**Interface**:
```rust
trait StateQueryProvider {
    fn discover_queries(&self, repo_path: &Path) -> Result<Vec<StateQuery>>;
    fn read_cached(&self, query: &str) -> Result<StateResult>;
    fn refresh(&self, query: &str) -> Result<StateResult>;
}
```

**Benefits**:
- Can swap providers (future: native Rust queries?)
- Easy to mock for testing
- Explicit about dependency boundaries

**Effort**: 3-4 hours
**Value**: Medium - More important if we expand beyond graft CLI

---

## UX Polish Issues

### 6. **Empty State Could Be More Helpful** 🟢 LOW PRIORITY

**Current**: Panel shows nothing when no queries defined.

**Better**:
```
┌─ State Queries ─────────────────────────────────────────┐
│                                                          │
│  No state queries defined in graft.yaml                 │
│                                                          │
│  State queries let you track project metrics like:      │
│  - Code coverage, test counts, lint warnings            │
│  - Task/issue counts, PR status                         │
│  - Documentation health, broken links                   │
│                                                          │
│  Example graft.yaml:                                     │
│    state:                                                │
│      coverage:                                           │
│        run: "pytest --cov --cov-report=json"            │
│        cache:                                            │
│          deterministic: true                             │
│                                                          │
│  Learn more: https://graft.dev/docs/state-queries       │
└──────────────────────────────────────────────────────────┘
```

**Effort**: 15 minutes
**Value**: Low - Nice for new users, but not critical

---

### 7. **No Visual Distinction for Deterministic vs Non-Deterministic** 🟢 LOW PRIORITY

**Problem**: Users can't tell which queries will give consistent results.

**Current**:
```
  ▶ coverage     85% lines, 72% branches
    tasks        59 open, 49 done
```

**Better**:
```
  ▶ coverage     85% lines, 72% branches       [D] (5m ago)
    tasks        59 open, 49 done               [N] (2h ago)
```

Legend: `[D] = Deterministic` `[N] = Non-deterministic`

**Why It Matters**:
- Deterministic queries (coverage) are safe to cache long-term
- Non-deterministic queries (tasks) might change even with same commit

**Effort**: 20 minutes
**Value**: Low - Mostly educational, doesn't affect functionality

---

## Performance & Scalability

### 8. **No Pagination for Large Query Lists** 🟢 LOW PRIORITY

**Problem**: What if a repo has 50+ state queries?

**Current**: All queries rendered in one list, scrolling is clunky.

**Scenarios to Handle**:
- 100+ queries defined
- Very long query names
- Very long summaries

**Impact**:
- Unlikely (most repos have <10 queries)
- But UI would be unusable if it happens

**Solution Options**:
1. Add pagination (10 queries per page)
2. Add search/filter (type to narrow list)
3. Add grouping (by category tag)

**Effort**: Varies (2-6 hours depending on approach)
**Value**: Very Low - YAGNI until we see real usage

---

### 9. **Cache Read Performance** 🟢 LOW PRIORITY

**Current**: Reads all cache files on panel open (one file per query).

**Potential Issue**:
- If 50 queries × 100 commits cached = 5000 files to scan
- `read_all_cached_for_query()` does `fs::read_dir()` per query

**Impact**:
- Likely negligible (SSD read is fast)
- Would need profiling to confirm

**Optimization Ideas**:
- Lazy load (only read when selected)
- Cache metadata in index file
- Stream results instead of loading all

**Effort**: 2-3 hours
**Value**: Low - Premature optimization

---

## Testing Gaps

### 10. **No End-to-End Tests with Real graft.yaml** 🟡 MEDIUM PRIORITY

**Current Testing**:
- ✓ Unit tests with mocked queries
- ✓ Integration tests with temp files
- ✗ No tests with actual graft repositories

**Missing Coverage**:
- Real graft.yaml parsing with complex state definitions
- Cache population → reading flow
- Error scenarios (graft not installed, command fails)

**What Could Break**:
- graft YAML schema changes
- Cache format changes
- Path resolution issues

**Solution**: Add fixture-based integration tests

```rust
#[test]
fn test_state_panel_with_real_notebook_repo() {
    // Use actual notebook graft.yaml as test fixture
    let fixture = include_str!("../../fixtures/notebook-graft.yaml");
    // ... test discovery and rendering
}
```

**Effort**: 1-2 hours
**Value**: Medium - Catches real-world breakage

---

### 11. **No Accessibility Testing** 🟢 LOW PRIORITY

**Questions Not Answered**:
- Does state panel work with screen readers?
- Are colors distinguishable for colorblind users?
- Can keyboard-only users navigate effectively?

**Current State**:
- Keyboard navigation works (j/k/q/Esc)
- But no accessibility audit done

**Effort**: 2-4 hours (requires specialized testing)
**Value**: Low now, High later (when aiming for production polish)

---

## Documentation Gaps

### 12. **No User Guide for State Queries** 🟡 MEDIUM PRIORITY

**What Exists**:
- ✓ TUI spec documents behavior
- ✓ Help overlay shows 's' key
- ✗ No guide on *why* or *when* to use state panel

**Missing Documentation**:
- "Getting Started with State Queries" tutorial
- Examples of useful queries (coverage, tasks, docs health)
- Best practices (when to use deterministic vs non-deterministic)
- Troubleshooting guide (cache not populating, stale data)

**Target Users**:
- Developers setting up state queries for their repo
- Users trying to understand what state panel shows

**Effort**: 3-4 hours (write comprehensive guide)
**Value**: Medium - Improves feature adoption

---

## Recommended Implementation Plan

### **Phase 1: Critical UX (Week 1)** - Required for A Grade

**Effort**: 3-4 hours
**Value**: HIGH

1. **Add cache age display** (30 min)
   - Show "(5m ago)" next to each query
   - Already have timestamp, just need to render it

2. **Add refresh action** (2-3 hours)
   - 'r' key to refresh selected query
   - Execute `graft state query <name> --refresh`
   - Show progress feedback

3. **Improve empty state** (15 min)
   - Show helpful example when no queries defined

**Deliverable**: State panel that users can actually rely on for live monitoring.

---

### **Phase 2: Power User Features (Week 2)** - Optional

**Effort**: 5-7 hours
**Value**: MEDIUM

1. **Add detail view** (2-3 hours)
   - Press Enter → See full JSON + metadata
   - 'q' to return to list

2. **Add E2E tests** (1-2 hours)
   - Test with real graft.yaml fixtures
   - Verify integration doesn't break

3. **Write user guide** (3-4 hours)
   - Tutorial with examples
   - Best practices
   - Troubleshooting

**Deliverable**: Complete, well-documented feature for power users.

---

### **Phase 3: Architecture Polish (Week 3)** - Nice to Have

**Effort**: 4-6 hours
**Value**: LOW (but good hygiene)

1. **Refactor module location** (30 min)
   - Move state to lib.rs only

2. **Add provider abstraction** (3-4 hours)
   - Decouple from graft CLI
   - Enable future extensibility

3. **Add accessibility audit** (2-4 hours)
   - Test with screen readers
   - Check colorblind accessibility

**Deliverable**: Clean, maintainable codebase ready for future expansion.

---

## Success Metrics

**Current State (B+ / 85%)**:
- ✓ 22 tests covering core functionality
- ✓ Error messages shown to users
- ✓ Feature documented in spec
- ✗ No cache freshness indicators
- ✗ No refresh capability

**Target State (A / 95%)**:
- ✓ Cache age displayed for all queries
- ✓ Refresh action works reliably
- ✓ Detail view for investigating results
- ✓ User guide with examples
- ✓ E2E tests with real fixtures
- ✓ Provider abstraction for clean architecture

**Grade Breakdown**:
- B+ (85%): "Production ready but limited"
- A- (90%): Phase 1 complete (cache age + refresh)
- A  (95%): Phase 1 + Phase 2 complete (detail view + docs)
- A+ (100%): All phases complete (architecture polish)

---

## Decision: What to Implement Now?

**Recommendation**: **Implement Phase 1 only** (3-4 hours).

**Rationale**:
1. Cache age + refresh are **critical for usability**
   - Users need to know if data is stale
   - Users need to update data without leaving Grove

2. Phase 2/3 are **nice-to-haves**
   - Detail view: only needed by power users
   - Architecture polish: no user-facing value yet

3. **Diminishing returns** after Phase 1
   - Going from 70% → 85% was critical
   - Going from 85% → 90% is high value (cache age)
   - Going from 90% → 95% is lower value (detail view)
   - Going from 95% → 100% is polish (diminishing returns)

**Suggested Action**:
- Ship current version (B+) to get user feedback
- Implement Phase 1 if users complain about staleness
- Defer Phase 2/3 until there's demonstrated demand

---

## Conclusion

The state panel is **production ready** as-is (B+ grade). The critical gaps from launch are now filled:
- ✓ Tests ensure reliability
- ✓ Errors are surfaced to users
- ✓ Feature is discoverable

The main remaining limitation is **cache freshness visibility**, which could be addressed with a focused 3-4 hour sprint on Phase 1.

**Ship it now, iterate based on user feedback.**
