---
status: reference
date: 2026-02-14
context: Architectural documentation for state query system and domain integration
---

# State Queries: Layered Architecture

## Overview

The state query system follows a clean three-layer architecture that separates infrastructure (graft) from domain logic (analytics libraries like notecap).

## Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Layer 1: Graft State Infrastructure (Generic)          │
│ - Query orchestration and execution                     │
│ - Commit-based caching                                  │
│ - Temporal queries (git worktree)                       │
│ - CLI: graft state query <name>                         │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼ (executes commands from graft.yaml)
┌─────────────────────────────────────────────────────────┐
│ Layer 3: Domain State Commands (Integration)           │
│ - Thin CLI wrappers around analytics                   │
│ - JSON output formatting                               │
│ - Error handling and validation                        │
│ - CLI: notecap state <command>                         │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼ (calls analytics functions)
┌─────────────────────────────────────────────────────────┐
│ Layer 2: Domain Analytics (Pure Functions)             │
│ - Pure functions with no side effects                  │
│ - Dataclass return types                               │
│ - No CLI, no caching, no I/O                           │
│ - Reusable across projects                             │
│ - Functions: calculate_writing_metrics(), etc.         │
└─────────────────────────────────────────────────────────┘
```

## Responsibilities by Layer

### Layer 1: Graft State Infrastructure

**Location**: `/home/coder/src/graft/src/graft/`

**Purpose**: Generic state query infrastructure that works with ANY domain

**Responsibilities**:
- Parse state query definitions from `graft.yaml`
- Execute commands and capture JSON output
- Cache results keyed by git commit hash
- Handle temporal queries using git worktree
- Invalidate caches on demand
- Provide CLI interface for state management

**Does NOT**:
- Know anything about specific domains (notebooks, code coverage, etc.)
- Parse or understand the JSON data structure
- Implement domain-specific analytics

**Key Files**:
- `src/graft/domain/state.py` - Domain models (StateQuery, StateResult, StateCache)
- `src/graft/services/state_service.py` - Service functions (get_state, execute_temporal_query)
- `src/graft/cli/commands/state.py` - CLI commands (query, list, invalidate)

**Example Usage**:
```bash
# Execute state query (caches result)
graft state query coverage

# Query historical state
graft state query coverage --commit v1.0.0

# Force refresh
graft state query coverage --refresh

# List all state queries
graft state list
```

---

### Layer 2: Domain Analytics Libraries

**Location**: Domain-specific repositories (e.g., `/home/coder/src/notebook/src/notecap/analytics.py`)

**Purpose**: Pure analytics functions that compute metrics from domain data

**Responsibilities**:
- Define dataclasses for structured results
- Implement pure functions that analyze domain data
- Return structured results (not JSON strings)
- Be testable in isolation (no side effects)
- Be reusable across different integration points

**Does NOT**:
- Perform I/O (except reading data files)
- Cache results
- Format output as JSON
- Depend on graft or any orchestration framework

**Key Files** (notecap example):
- `src/notecap/analytics.py` - Pure analytics functions
  - `calculate_writing_metrics(notes_dir: Path) -> WritingMetrics`
  - `calculate_task_metrics(notes_dir: Path) -> TaskMetrics`
  - `calculate_graph_metrics(notes_dir: Path) -> GraphMetrics`
  - `calculate_recent_metrics(notes_dir: Path) -> RecentMetrics`

**Example Usage** (programmatic):
```python
from notecap.analytics import calculate_writing_metrics
from pathlib import Path

# Use analytics directly
metrics = calculate_writing_metrics(Path('notes'))
print(f"Total words: {metrics.total_words}")
print(f"Modified today: {metrics.notes_modified}")
```

---

### Layer 3: Domain State Commands

**Location**: Domain-specific repositories (e.g., `/home/coder/src/notebook/src/notecap/state.py`)

**Purpose**: CLI integration layer that bridges Layer 2 (analytics) and Layer 1 (graft)

**Responsibilities**:
- Provide CLI commands that wrap analytics functions
- Convert dataclass results to JSON
- Handle errors gracefully with user-friendly messages
- Validate input (e.g., check if notes directory exists)
- Format output for consumption by graft

**Does NOT**:
- Implement caching (that's Layer 1's job)
- Implement temporal queries (that's Layer 1's job)
- Duplicate analytics logic (calls Layer 2)

**Key Files** (notecap example):
- `src/notecap/state.py` - State query CLI commands
  - `notecap state writing` - Wraps `calculate_writing_metrics()`
  - `notecap state tasks` - Wraps `calculate_task_metrics()`
  - `notecap state graph` - Wraps `calculate_graph_metrics()`
  - `notecap state recent` - Wraps `calculate_recent_metrics()`

**Example Usage** (CLI):
```bash
# Called directly by users
notecap state writing

# Called by graft (via graft.yaml)
graft state query writing-today
# → executes: uv run notecap state writing
# → graft caches the result
```

---

## Integration Pattern: graft + notecap

### Configuration (graft.yaml)

```yaml
state:
  writing-today:
    run: "uv run notecap state writing"
    cache:
      deterministic: false  # writing metrics change as files are modified
    timeout: 15

  tasks:
    run: "uv run notecap state tasks"
    cache:
      deterministic: true  # task list only changes with commits
    timeout: 20
```

### Execution Flow

1. **User runs**: `graft state query writing-today`

2. **Layer 1 (Graft)**:
   - Reads `graft.yaml` and finds the `writing-today` query
   - Checks cache for current commit hash
   - If not cached (or `--refresh` flag):
     - Executes: `uv run notecap state writing`

3. **Layer 3 (Notecap CLI)**:
   - `state.py` receives the command
   - Calls: `get_notes_dir()` to locate notes
   - Calls Layer 2: `calculate_writing_metrics(notes_dir)`

4. **Layer 2 (Analytics)**:
   - `analytics.py` scans notes directory
   - Counts words, notes created/modified
   - Returns `WritingMetrics` dataclass

5. **Layer 3 (Notecap CLI)**:
   - Converts dataclass to dict: `asdict(metrics)`
   - Outputs JSON to stdout: `print(json.dumps(output, indent=2))`

6. **Layer 1 (Graft)**:
   - Captures JSON output
   - Parses and validates JSON
   - Creates `StateResult` with metadata
   - Writes to cache: `~/.cache/graft/{workspace}/{repo}/state/writing-today/{commit}.json`
   - Returns cached result to user

### Data Flow Diagram

```
User
  │
  ▼
graft CLI (Layer 1)
  │ reads graft.yaml
  │ checks cache
  │ executes command
  ▼
subprocess: "uv run notecap state writing"
  │
  ▼
notecap CLI (Layer 3)
  │ validates input
  │ calls analytics
  ▼
calculate_writing_metrics() (Layer 2)
  │ reads files
  │ computes metrics
  │ returns WritingMetrics
  ▼
notecap CLI (Layer 3)
  │ converts to JSON
  │ prints to stdout
  ▼
graft CLI (Layer 1)
  │ parses JSON
  │ caches result
  │ displays to user
  ▼
User sees cached JSON result
```

---

## Why This Architecture?

### Separation of Concerns

- **Infrastructure (graft)**: Handles generic orchestration, caching, temporal queries
- **Domain Logic (analytics)**: Pure, testable, reusable functions
- **Integration (state CLI)**: Thin glue layer with minimal logic

### No Duplication

- Caching logic lives ONLY in graft
- Analytics logic lives ONLY in domain libraries
- Integration logic is minimal (just wrapping + JSON conversion)

### Flexibility

- Analytics functions can be used:
  - Via graft (with caching)
  - Via standalone CLI (no caching)
  - Programmatically (Python imports)
  - In notebooks, scripts, tests

### Testability

- Layer 1 tests: Mock subprocess calls, verify caching behavior
- Layer 2 tests: Pure function tests with temp directories
- Layer 3 tests: CLI integration tests via CliRunner

---

## Type Safety

### Current State

Layer 1 (graft) treats all state query results as `Dict[str, Any]`:

```python
result: StateResult = get_state(...)
result.data: Dict[str, Any]  # No type safety!
```

### Improvement: Typed Wrappers (Optional)

Domain-specific code can define TypedDicts for known schemas:

```python
from typing import TypedDict

class WritingMetrics(TypedDict):
    notes_created: int
    notes_modified: int
    words_today: int
    total_words: int
    date: str

# Layer 2 already returns dataclass (type-safe)
metrics: WritingMetrics = calculate_writing_metrics(notes_dir)

# Layer 3 converts to JSON (runtime validation)
output = asdict(metrics)
assert "total_words" in output  # Validated

# Layer 1 receives untyped JSON (but validated structure exists)
```

This provides:
- **Static type safety** in Layer 2 (dataclasses)
- **Runtime validation** in Layer 3 (dataclass serialization)
- **Documentation** of expected schema

---

## Common Patterns

### Adding a New State Query

**Step 1: Implement Analytics (Layer 2)**

```python
# src/notecap/analytics.py

@dataclass
class LinkHealthMetrics:
    broken_links: int
    orphaned_notes: int
    total_backlinks: int

def calculate_link_health(notes_dir: Path) -> LinkHealthMetrics:
    """Calculate link health metrics."""
    # Implementation...
    return LinkHealthMetrics(
        broken_links=broken,
        orphaned_notes=orphaned,
        total_backlinks=backlinks,
    )
```

**Step 2: Add CLI Command (Layer 3)**

```python
# src/notecap/state.py

@state_app.command(name="link-health")
def link_health_command(
    pretty: bool = typer.Option(True, "--pretty", "-p"),
) -> None:
    """Show link health metrics."""
    try:
        notes_dir = get_notes_dir()
        metrics = calculate_link_health(notes_dir)
        output = asdict(metrics)
        print(json.dumps(output, indent=2 if pretty else None))
    except Exception as e:
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from e
```

**Step 3: Configure State Query (Layer 1)**

```yaml
# graft.yaml

state:
  link-health:
    run: "uv run notecap state link-health"
    cache:
      deterministic: true
    timeout: 30
```

**Step 4: Use It**

```bash
# Via graft (with caching)
graft state query link-health

# Standalone (no caching)
notecap state link-health

# Programmatic (Python)
from notecap.analytics import calculate_link_health
metrics = calculate_link_health(Path('notes'))
```

---

## Anti-Patterns to Avoid

### ❌ Don't: Duplicate Caching Logic

```python
# BAD: Layer 3 implementing its own cache
@state_app.command("writing")
def writing_command():
    cache_file = Path(".cache/writing.json")
    if cache_file.exists():  # ❌ Duplicates Layer 1
        return json.loads(cache_file.read_text())
    # ...
```

**Why**: Caching is Layer 1's responsibility. Duplication leads to:
- Inconsistent cache behavior
- No temporal query support
- No commit-based invalidation

**Fix**: Remove caching from Layer 3, let graft handle it.

---

### ❌ Don't: Mix Analytics with I/O in Layer 2

```python
# BAD: Analytics function doing JSON output
def calculate_writing_metrics(notes_dir: Path) -> None:
    # ... calculation ...
    output = {"total_words": total}
    print(json.dumps(output))  # ❌ Side effect in pure function
```

**Why**: Layer 2 should be pure functions that return data, not perform I/O.

**Fix**: Return dataclass, let Layer 3 handle JSON conversion.

---

### ❌ Don't: Add Domain Knowledge to Layer 1

```python
# BAD: Graft implementing notebook-specific logic
def get_state(...):
    result = execute_query(...)
    if query_name == "writing":  # ❌ Domain knowledge in infrastructure
        # Special handling for writing metrics
```

**Why**: Layer 1 must remain generic to work with any domain.

**Fix**: Keep Layer 1 generic, move domain logic to Layer 2/3.

---

## Testing Strategy

### Layer 1 Tests (Graft)

```python
# tests/services/test_state_service.py

def test_get_state_uses_cache(tmp_path):
    """Verify caching works across commits."""
    # Mock subprocess to return JSON
    # Verify cache file created
    # Verify second call uses cache
```

### Layer 2 Tests (Analytics)

```python
# tests/unit/test_analytics.py

def test_calculate_writing_metrics():
    """Test pure analytics function."""
    with tempfile.TemporaryDirectory() as tmpdir:
        notes_dir = Path(tmpdir)
        (notes_dir / "note1.md").write_text("Hello world")
        metrics = calculate_writing_metrics(notes_dir)
        assert metrics.total_words == 2
```

### Layer 3 Tests (Integration)

```python
# tests/unit/test_state.py

def test_writing_command(tmp_path, monkeypatch):
    """Test CLI command outputs correct JSON."""
    notes_dir = tmp_path / "notes"
    notes_dir.mkdir()
    (notes_dir / "note.md").write_text("Test")

    monkeypatch.chdir(tmp_path)
    result = runner.invoke(app, ["state", "writing"])

    assert result.exit_code == 0
    output = json.loads(result.stdout)
    assert "total_words" in output
```

---

## Conclusion

The three-layer architecture provides:

- ✅ **Clear separation of concerns**: Infrastructure vs domain vs integration
- ✅ **No duplication**: Each layer has distinct responsibilities
- ✅ **Flexibility**: Analytics reusable across contexts
- ✅ **Testability**: Each layer testable in isolation
- ✅ **Type safety**: Dataclasses in Layer 2, JSON in Layers 1/3
- ✅ **Maintainability**: Changes localized to appropriate layer

This architecture scales well as more domains (code coverage, test results, etc.) integrate with graft's state query system.
