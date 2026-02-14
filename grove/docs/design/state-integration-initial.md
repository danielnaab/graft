---
status: design
date: 2026-02-14
context: Initial Grove state query integration - UX design
---

# Grove State Integration - Initial Design

## Goals

1. **Immediate Value**: Show repository health at a glance
2. **Simple UX**: Minimal keystrokes, intuitive display
3. **Extensible**: Room for future features (dashboards, alerts, trends)
4. **Performance**: Leverage fast state queries (< 30ms)

## Design Principles

### 1. Progressive Disclosure
- Show summary by default (high-level health)
- Details available on demand (drill-down)
- Don't overwhelm with data

### 2. Actionable Information
- Highlight problems (broken links, stale notes, low coverage)
- Make it easy to refresh/invalidate
- Show what changed since last check

### 3. Extensibility Points
- Plugin architecture for new state query types
- Custom thresholds per repository
- Future: Trends, alerts, comparisons

---

## UX Design: Repository Detail View Enhancement

### Current State (Before)
```
┌─────────────────────────────────────────────────┐
│ Repository: notebook                            │
├─────────────────────────────────────────────────┤
│ Commands:                                       │
│   capture - Quick capture to daily note         │
│                                                 │
│ Dependencies:                                   │
│   meta-knowledge-base                           │
│   living-specifications                         │
│                                                 │
│ [Press 'x' for commands, 'q' to quit]          │
└─────────────────────────────────────────────────┘
```

### Proposed State (After)
```
┌─────────────────────────────────────────────────┐
│ Repository: notebook                            │
├─────────────────────────────────────────────────┤
│ Health: ● Good  (Press 's' for state details)  │
│                                                 │
│ Commands:                                       │
│   capture - Quick capture to daily note         │
│                                                 │
│ Dependencies:                                   │
│   meta-knowledge-base                           │
│   living-specifications                         │
│                                                 │
│ [x: commands, s: state, q: quit]               │
└─────────────────────────────────────────────────┘
```

**Health Indicator**:
- ● Green "Good" - All metrics healthy
- ● Yellow "Check" - Some issues (broken links, stale notes)
- ● Red "Issues" - Critical problems
- ○ Gray "Unknown" - No state queries defined or cache empty

---

## State Detail Panel (Press 's')

### Compact View (Default)
```
┌─────────────────────────────────────────────────┐
│ State Queries - notebook                       │
├─────────────────────────────────────────────────┤
│ writing-today  ● 2,450 words today (cached 5m) │
│ tasks          ● 59 open, 49 done   (cached 5m)│
│ graph          ⚠ 463 orphans, 2K broken        │
│ recent         ● Modified today: 12            │
│                                                 │
│ [r: refresh, d: details, ESC: back]            │
└─────────────────────────────────────────────────┘
```

**Status Indicators**:
- ● Green: Metric is good
- ⚠ Yellow: Attention needed
- ✗ Red: Critical issue
- ○ Gray: No data / stale cache

### Detailed View (Press 'd')
```
┌─────────────────────────────────────────────────┐
│ State Query: graph (knowledge graph health)    │
├─────────────────────────────────────────────────┤
│ Total Notes:    2,019                          │
│ Total Links:    4,910                          │
│ Broken Links:   2,223  ⚠ (45%)                 │
│ Orphaned:       463    ⚠ (23%)                 │
│ Avg Links:      2.4 per note                   │
│                                                 │
│ Top Hubs:                                       │
│   1. meta-knowledge-base (145 backlinks)       │
│   2. zettelkasten-principles (89 backlinks)    │
│   3. knowledge-management (67 backlinks)       │
│                                                 │
│ Cached: 5 minutes ago (commit: abc123f)        │
│ [r: refresh, ESC: back to summary]             │
└─────────────────────────────────────────────────┘
```

---

## Key Bindings

### Repository Detail View
- `s` - Show state summary (new)
- `x` - Show command picker (existing)
- `d` - Show dependencies (existing)
- `q` - Quit

### State Summary View
- `r` - Refresh selected query (invalidate + re-run)
- `R` - Refresh all queries
- `d` - Show detailed view of selected query
- `↑/↓` - Navigate between queries
- `ESC` - Back to repository detail

### State Detail View
- `r` - Refresh this query
- `j/k` or `↑/↓` - Scroll (if content is long)
- `ESC` - Back to summary

---

## Implementation Phases

### Phase 1: Basic Display (1-2 hours) ✅ RECOMMENDED

**Goal**: Show state query results in Grove

**Deliverables**:
1. Parse `graft.yaml` to discover state queries
2. Read cached state results (if available)
3. Add state summary panel to detail view
4. Add 's' keybinding to toggle state panel
5. Display query names + simple status

**Files to Modify**:
- `src/tui.rs` - Add state panel rendering
- `src/detail_provider.rs` - Add state query discovery
- `src/cache.rs` (new) - Read graft cache files

**Testing**:
- Manual: Run grove on notebook repository
- Verify state queries are discovered
- Verify cached results are displayed

---

### Phase 2: Health Indicators (1 hour)

**Goal**: Smart health assessment

**Deliverables**:
1. Define health thresholds per query type
2. Compute overall repository health
3. Color-code status indicators
4. Show health in repository detail view (before pressing 's')

**Example Thresholds**:
```rust
struct HealthThresholds {
    graph_broken_links_warn: f64,    // 0.10 (10%)
    graph_broken_links_critical: f64, // 0.30 (30%)
    graph_orphans_warn: f64,          // 0.20 (20%)
    graph_orphans_critical: f64,      // 0.40 (40%)
    tasks_open_warn: u32,             // 100
    tasks_open_critical: u32,         // 200
}
```

**Files to Modify**:
- `src/health.rs` (new) - Health assessment logic
- `src/tui.rs` - Show health indicator

---

### Phase 3: Refresh Actions (1 hour)

**Goal**: Allow manual refresh from Grove

**Deliverables**:
1. Add 'r' keybinding to refresh selected query
2. Execute `graft state query <name> --refresh`
3. Show spinner during refresh
4. Update display with new results

**Files to Modify**:
- `src/tui.rs` - Handle refresh keybinding
- `src/command.rs` - Execute graft state refresh

---

### Phase 4: Detailed View (1 hour)

**Goal**: Drill down into specific query results

**Deliverables**:
1. Add 'd' keybinding to show details
2. Parse JSON and format for display
3. Show metadata (cached time, commit)
4. Handle long content (scrolling)

**Files to Modify**:
- `src/tui.rs` - Add detail view rendering
- `src/format.rs` (new) - Format JSON for display

---

## Data Structures

### StateQuery
```rust
#[derive(Debug, Clone)]
pub struct StateQuery {
    pub name: String,
    pub description: Option<String>,
    pub deterministic: bool,
    pub timeout: Option<u64>,
}
```

### StateResult
```rust
#[derive(Debug, Clone)]
pub struct StateResult {
    pub query_name: String,
    pub data: serde_json::Value,
    pub metadata: StateMetadata,
}

#[derive(Debug, Clone)]
pub struct StateMetadata {
    pub commit_hash: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub deterministic: bool,
}
```

### HealthStatus
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Good,      // All metrics healthy
    Warning,   // Some issues
    Critical,  // Major problems
    Unknown,   // No data or stale
}
```

---

## Extensibility Points

### 1. Custom Renderers per Query Type
```rust
trait StateRenderer {
    fn render_summary(&self, data: &Value) -> String;
    fn render_detail(&self, data: &Value) -> Vec<String>;
    fn assess_health(&self, data: &Value) -> HealthStatus;
}

// Built-in renderers
struct GraphMetricsRenderer;
struct TaskMetricsRenderer;
struct WritingMetricsRenderer;

// Future: Plugin system
// struct CustomRenderer { ... }
```

### 2. Threshold Configuration
```rust
// Future: Read from .grove/config.toml
struct StateConfig {
    thresholds: HashMap<String, QueryThresholds>,
}

// For now: Hardcoded reasonable defaults
```

### 3. Historical Trends
```rust
// Future Phase: Track changes over time
struct TrendData {
    query_name: String,
    history: Vec<(DateTime<Utc>, Value)>,
}

// Show sparklines, deltas, etc.
```

### 4. Alerts & Notifications
```rust
// Future Phase: Notify on threshold breaches
struct Alert {
    query_name: String,
    severity: HealthStatus,
    message: String,
}
```

---

## Files to Create

**Phase 1**:
- `src/state/mod.rs` - State module entry point
- `src/state/query.rs` - StateQuery, StateResult types
- `src/state/cache.rs` - Read graft cache files
- `src/state/discovery.rs` - Parse graft.yaml for state queries

**Phase 2**:
- `src/state/health.rs` - Health assessment logic
- `src/state/thresholds.rs` - Default thresholds

**Phase 3**:
- `src/state/refresh.rs` - Refresh actions

**Phase 4**:
- `src/state/render.rs` - Rendering utilities
- `src/state/detail.rs` - Detailed view logic

---

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_parse_state_query_from_yaml() { ... }

#[test]
fn test_read_cached_state_result() { ... }

#[test]
fn test_health_assessment_graph_metrics() { ... }

#[test]
fn test_format_graph_metrics_summary() { ... }
```

### Integration Tests
```rust
#[test]
fn test_state_panel_displays_cached_results() {
    // Setup: Create repo with graft.yaml + cached state
    // Execute: Open grove, press 's'
    // Verify: State queries displayed correctly
}

#[test]
fn test_refresh_invalidates_and_reruns() {
    // Setup: Repository with state queries
    // Execute: Press 's', then 'r' on a query
    // Verify: Cache invalidated, query re-executed
}
```

---

## MVP Definition (Recommended Initial Scope)

**Deliver in Phase 1**:
- ✅ Discover state queries from graft.yaml
- ✅ Read cached state results
- ✅ Display state summary panel (press 's')
- ✅ Show query names + cached data
- ✅ Basic formatting (JSON pretty-print)

**Defer to Future**:
- Health indicators (Phase 2)
- Refresh actions (Phase 3)
- Detailed view (Phase 4)
- Trends, alerts, dashboards

**Why**:
- Phase 1 provides immediate value (see state in Grove)
- Simple to implement (1-2 hours)
- Validates architecture before adding complexity
- Room for feedback before building more

---

## Success Criteria

**Phase 1**:
- ✅ User can press 's' in repository detail view
- ✅ State queries are discovered from graft.yaml
- ✅ Cached results are displayed (if available)
- ✅ UI is responsive (no blocking operations)
- ✅ Tests pass

**Future Phases**:
- Health assessment is accurate and useful
- Refresh workflow is smooth (spinner, no hangs)
- Detailed view is readable and scrollable
- Extensibility points work (easy to add new renderers)

---

## Open Questions

1. **Cache Location**: Where does graft store cache files?
   - Answer: `~/.cache/graft/{workspace-hash}/{repo-name}/state/{query-name}/{commit}.json`

2. **State Query Discovery**: Parse graft.yaml or use `graft state list`?
   - Recommendation: Parse graft.yaml (faster, no subprocess)
   - Alternative: `graft state list --json` (if available)

3. **Workspace Hash**: How to compute workspace hash to locate cache?
   - Answer: Read from graft implementation (SHA256 of workspace name)

4. **JSON Schema**: How to know structure of each query type?
   - Phase 1: Generic JSON display
   - Future: Type-specific renderers with known schemas

---

## Next Steps

1. **Implement Phase 1** (1-2 hours)
   - Create state module structure
   - Implement cache reading
   - Add state panel to UI
   - Test with notebook repository

2. **Gather Feedback**
   - Is the display useful?
   - What's missing?
   - What thresholds make sense?

3. **Plan Phase 2+** based on feedback
