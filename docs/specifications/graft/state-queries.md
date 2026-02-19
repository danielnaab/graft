---
status: working
last-verified: 2026-02-14
last-updated: 2026-02-15
owners: [human, agent]
---

# State Queries

## Intent

Define how graft queries, caches, and exposes structured state about repositories. State queries enable observability by running commands that output JSON and caching results tied to git commits.

State is a primitive building block for:
- Repository health dashboards
- Historical analysis (how did coverage change over time?)
- Workspace-level aggregation (show all repos with failing tests)
- Smart caching (avoid re-running expensive operations)

## Non-goals

- **Not Stage 1**: Command integration (`source: command:test`)
- **Not Stage 1**: Composed state (`compose:` dependencies)
- **Not Stage 1**: Non-deterministic state with TTL caching
- **Not Stage 1**: Workspace aggregation
- **Not Stage 1**: Schema validation (JSON Schema)

## Behavior

### State Definition [Stage 1]

```gherkin
Given a repository has a graft.yaml file
When the file contains a `state:` section
Then graft parses state query definitions
And each state query has a name and a run command
```

```gherkin
Given a state query is defined
When the run command outputs valid JSON to stdout
Then graft captures and caches the JSON output
```

```gherkin
Given a state query is defined
When the run command outputs invalid JSON to stdout
Then graft returns an error
And does not cache the output
And shows the stderr/stdout for debugging
```

**Example:**
```yaml
# graft.yaml
state:
  coverage:
    run: "pytest --cov --cov-report=json --quiet | jq '.totals.percent_covered'"
    cache:
      deterministic: true

  test-results:
    run: "cat test-results.json"
    cache:
      deterministic: true
```

### State Execution [Stage 1]

```gherkin
Given a state query named "coverage" is defined
When the user runs `graft state query coverage`
Then graft executes the run command in the repository directory
And captures stdout as JSON
And caches the result keyed by commit hash
And outputs the JSON to stdout
```

```gherkin
Given a state query has been executed and cached
When the user runs `graft state query coverage` again
And the commit hash has not changed
And cache is deterministic
Then graft returns the cached result
And does not re-run the command
```

```gherkin
Given a state query has been cached for commit A
When the user runs `graft state query coverage` on commit B
Then graft detects the commit change
And re-runs the command
And caches the result for commit B
```

```gherkin
Given a state query defines a timeout value
When the command exceeds the timeout
Then graft terminates the command
And returns an error
And does not cache the result
```

```gherkin
Given a state query has no timeout specified
When the command is executed
Then graft uses the default timeout of 300 seconds (5 minutes)
```

**Implementation note (2026-02-19)**: Timeout enforcement was declared in this spec from the
start but was not implemented in the initial graft-engine `state.rs` code (the timeout was
computed but ignored). This is now fixed — `execute_state_query()` passes the timeout to
`run_to_completion_with_timeout()` via `ProcessConfig`, which enforces it with process kill.

### Cache Invalidation [Stage 1]

```gherkin
Given a state query has cached results
When the user runs `graft state query coverage --refresh`
Then graft invalidates the cache
And re-runs the command
And caches the new result
```

```gherkin
Given a state query has cached results
When the user runs `graft state invalidate --all`
Then graft clears all cached state for the repository
```

```gherkin
Given a state query has cached results
When the user runs `graft state invalidate coverage`
Then graft clears cached state for the specific query
```

### Temporal Queries [Stage 1]

```gherkin
Given a repository has historical commits
When the user runs `graft state query coverage --commit HEAD~5`
And the working tree is clean
Then graft creates a temporary git worktree at the specified commit
And executes the state query in the worktree
And caches the result for that commit hash
And cleans up the worktree
```

```gherkin
Given a repository has historical commits
When the user runs `graft state query coverage --commit HEAD~5`
And the working tree has uncommitted changes
Then graft returns an error
And tells the user to commit or stash changes first
```

```gherkin
Given a state query has been cached for commit A
When the user runs `graft state query coverage --commit A`
Then graft returns the cached result
And does not re-run the command
```

**Note:** Temporal queries are read-only. They don't modify the working tree permanently.

### Cache Storage [Stage 1]

```gherkin
Given graft executes a state query
When caching the result
Then the cache is stored at:
  ~/.cache/graft/{workspace-hash}/{repo-name}/state/{state-name}/{commit-hash}.json
```

```gherkin
Given a cache file exists
When the cache file contains metadata
Then metadata includes:
  - query_name: the name of the state query
  - commit_hash: the git commit
  - timestamp: when the query was executed
  - command: the run command that was executed
  - deterministic: whether the cache is commit-bound
```

**Cache file format:**
```json
{
  "metadata": {
    "query_name": "coverage",
    "commit_hash": "abc123...",
    "timestamp": "2026-02-13T10:30:00Z",
    "command": "pytest --cov --cov-report=json --quiet | jq '.totals.percent_covered'",
    "deterministic": true
  },
  "data": {
    "percent_covered": 87.5
  }
}
```

### List State [Stage 1]

```gherkin
Given a repository has state queries defined
When the user runs `graft state list`
Then graft shows all defined state queries
And indicates which have cached results for current commit
```

**Example output:**
```
State queries in graft.yaml:
  coverage        (cached for abc123)
  test-results    (not cached)
  lint-report     (cached for abc123)
```

## Edge Cases

### No graft.yaml

```gherkin
Given a repository has no graft.yaml
When the user runs `graft state query coverage`
Then graft returns an error
And suggests creating graft.yaml with state definitions
```

### No state section

```gherkin
Given a repository's graft.yaml has no `state:` section
When the user runs `graft state query coverage`
Then graft returns an error
And suggests adding state queries to graft.yaml
```

### State query not found

```gherkin
Given a repository's graft.yaml defines state queries
When the user runs `graft state query nonexistent`
And "nonexistent" is not defined
Then graft returns an error
And lists available state queries
```

### Command fails (non-zero exit)

```gherkin
Given a state query's run command fails
When graft executes the command
And the exit code is non-zero
Then graft does not cache the result
And returns an error with stderr/stdout
```

### Invalid JSON output

```gherkin
Given a state query's run command outputs invalid JSON
When graft attempts to parse the output
Then graft returns an error
And shows the raw output for debugging
And does not cache the result
```

### Dirty working tree with temporal query

```gherkin
Given the user runs `graft state query coverage --commit HEAD~5`
When the working tree has uncommitted changes
Then graft returns an error
And tells the user to commit or stash changes first
```

Temporal queries require a clean working tree. This is a safety measure to prevent worktree creation from interfering with in-progress work. When the tree is clean, graft uses `git worktree add` to create an isolated checkout at the target commit.

## Constraints

- **JSON output only**: State queries must output valid JSON to stdout
- **Deterministic caching**: Cache key is commit hash (assumes same commit → same output)
- **Single repository**: State queries operate on one repo at a time (workspace aggregation is later)
- **Read-only**: State queries should not modify repository state (not enforced, just convention)

## CLI Interface

### `graft state query <name>`

**Usage:**
```bash
graft state query <name> [OPTIONS]
```

**Options:**
- `--commit <ref>`, `-c`: Query state for a specific commit (default: HEAD)
- `--refresh`, `-r`: Invalidate cache and re-run query
- `--raw`: Output only the data (no metadata)
- `--pretty`, `-p`: Pretty-print JSON output (default: True)

**Examples:**
```bash
# Execute query and cache result
graft state query coverage

# Query historical state
graft state query coverage --commit v1.0.0

# Force re-run
graft state query coverage --refresh

# Output just the data
graft state query coverage --raw

# Compact output (for piping)
graft state query coverage --no-pretty
```

**Note:** The CLI uses a subcommand-based design (`query`, `list`, `invalidate`) rather than flags, providing clearer separation of concerns and better discoverability.

### `graft state list`

**Usage:**
```bash
graft state list [OPTIONS]
```

**Options:**
- `--cache`, `-c`: Show cache status for current commit (default: True)

**Output:**
```
State queries defined in graft.yaml:

coverage
  Command: pytest --cov --cov-report=json --quiet | jq '.totals.percent_covered'
  Cached:  Yes (commit abc123, 5 minutes ago)

test-results
  Command: cat test-results.json
  Cached:  No
```

### `graft state invalidate`

**Usage:**
```bash
# Invalidate specific query
graft state invalidate <name>

# Invalidate all queries
graft state invalidate --all
```

**Options:**
- `--all`, `-a`: Invalidate all state caches for the repository

**Examples:**
```bash
# Invalidate one query
graft state invalidate coverage

# Invalidate all
graft state invalidate --all
```

## File Structure

### Updated graft.yaml Schema

```yaml
# graft.yaml
name: my-repo
version: "1.0"

dependencies:
  # ... existing dependencies

commands:
  # ... existing commands

state:  # NEW
  coverage:
    run: "pytest --cov --cov-report=json --quiet | jq '.totals.percent_covered'"
    cache:
      deterministic: true  # cache by commit hash
    timeout: 300  # optional: command timeout in seconds (default: 300)

  test-results:
    run: "pytest --json-report --quiet"
    cache:
      deterministic: true
    timeout: 120  # faster test suite

  dependency-health:
    run: "uv pip list --outdated --format json"
    cache:
      deterministic: false  # Stage 1: treat as deterministic anyway
      # Stage 2+: ttl: 86400
    timeout: 60  # network operations can be faster with good cache
```

### Cache Directory Structure

```
~/.cache/graft/
  {workspace-hash}/           # e.g., "a3f2b1c9..."
    {repo-name}/              # e.g., "my-repo"
      state/
        coverage/
          abc123def456.json   # commit hash
          789ghi012jkl.json
        test-results/
          abc123def456.json
        .metadata.json        # last updated times, etc.
```

## Open Questions

**Stage 1:** (All resolved - see Decisions below)
- [x] Should `--commit` use git worktree or fail if working tree is dirty? → **RESOLVED: Uses git worktree, but fails fast if working tree is dirty for safety**
- [x] Should we validate JSON structure (basic parse) or defer schema validation to Stage 2? → **RESOLVED: Basic JSON parse for Stage 1, schema validation deferred to Stage 2+**
- [x] Should cache include command stdout/stderr even on success? → **RESOLVED: No - cache only includes JSON output. Stderr/stdout shown only on error**

**Future Stages:**
- [ ] How to handle non-deterministic state (TTL, external dependencies)?
- [ ] How to compose state (state depending on other state)?
- [ ] How to aggregate state across workspace?
- [ ] How to integrate with commands (`source: command:test`)?

## Decisions

- **2026-02-13**: Stage 1 implementation focuses on deterministic state only
  - Simplest case: same commit → same output
  - Non-deterministic state (TTL, external APIs) deferred to Stage 2
  - Composition deferred to Stage 4

- **2026-02-13**: Cache format includes metadata wrapper
  - Allows evolution (add schema validation, TTL, etc.)
  - Metadata and data are separate for clean extraction
  - `--raw` flag returns just the data

- **2026-02-13**: Temporal queries use git worktree with safety check
  - Uses `git worktree add` to create isolated environment for historical queries
  - Requires clean working tree before executing (fail-fast safety measure)
  - User must commit or stash changes before querying historical commits
  - Worktree is automatically cleaned up after query execution

- **2026-02-13**: JSON validation is basic parse only
  - Validates output is valid JSON
  - JSON Schema validation deferred to Stage 2+
  - Enough to catch most errors (malformed JSON, plain text output)

- **2026-02-14**: CLI uses subcommand design
  - `graft state query <name>` (not `graft state <name>`)
  - `graft state list` and `graft state invalidate` as explicit subcommands
  - Clearer separation of concerns and better discoverability
  - Consistent with other CLI tools (git, docker, etc.)

- **2026-02-14**: Timeout field added to state queries
  - Optional `timeout` field in state query definition (seconds)
  - Default: 300 seconds (5 minutes)
  - Prevents runaway queries from hanging indefinitely
  - Documented in domain model (`StateQuery.timeout`)

## Sources

- [Graft Core Operations](./core-operations.md) - Base command execution patterns
- [Grove Vertical Slices Evolution](../../../notes/2026-02-13-grove-vertical-slices-evolution.md) - Slice 8 (Workspace Health Dashboard) motivates state queries
