# Testing Strategy

Graft uses **black-box subprocess testing**: all tests invoke the `graft` CLI via Python's `subprocess` module, asserting on JSON output and exit codes.

This document explains the testing approach, why it was chosen, and how to write effective tests.

## Testing Philosophy

### CLI as Contract

The CLI is Graft's public API. Users (humans and agents) interact via commands like `graft run`, `graft finalize`, etc.

**Testing the CLI directly means:**
- We test what users actually use
- Tests verify the contract, not implementation
- Refactoring internals doesn't break tests (as long as CLI behavior unchanged)
- Forces CLI stability (breaking tests = breaking users)

### Black-Box, No Internal Imports

Tests don't import Graft's internal modules. They execute `graft` as a subprocess and examine:
- Exit codes (0 = success, 1 = user error, 2 = system error)
- Stdout/stderr output
- JSON output (`--json` flag)
- File system state (what files were created/modified)

**Benefits:**
- Tests are resilient to refactoring
- We can rewrite internals without touching tests
- Tests verify user experience, not implementation details

**Trade-offs:**
- Slower than unit tests (subprocess overhead)
- Can't test internal functions directly (but that's intentional)
- Debugging failures requires inspecting subprocess output

## Test Structure

### Location

All tests live in `tests/`:

```
tests/
├── conftest.py           # Pytest fixtures
├── test_explain.py       # graft explain tests
├── test_run.py           # graft run tests
├── test_status_finalize.py  # graft status/finalize tests
├── test_impact_simulate.py  # graft impact/simulate tests
└── test_dvc_orchestrator.py # DVC integration tests
```

### Fixtures

`conftest.py` provides reusable fixtures:

```python
@pytest.fixture
def agile_ops_example(tmp_path):
    """Copy agile-ops example to tmp_path for testing."""
    src = Path(__file__).parent.parent / "examples" / "agile-ops"
    dest = tmp_path / "agile-ops"
    shutil.copytree(src, dest)

    # Initialize git (required for Graft)
    subprocess.run(["git", "init"], cwd=dest, check=True)
    subprocess.run(["git", "add", "."], cwd=dest, check=True)
    subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=dest, check=True)

    return dest
```

**Pattern:** Copy fixtures to `tmp_path`, initialize git, return path.

### Test Pattern

Standard test structure:

```python
def test_run_sprint_brief(agile_ops_example):
    # Arrange
    artifact_path = agile_ops_example / "artifacts" / "sprint-brief"

    # Act: Run graft CLI
    result = subprocess.run(
        ["graft", "run", str(artifact_path), "--json"],
        cwd=agile_ops_example,
        capture_output=True,
        text=True
    )

    # Assert: Exit code
    assert result.returncode == 0

    # Assert: JSON output
    data = json.loads(result.stdout)
    assert data["artifact"] == "sprint-brief"
    assert "derivations" in data

    # Assert: File system state
    output_file = artifact_path / "brief.md"
    assert output_file.exists()
    content = output_file.read_text()
    assert "Sprint Brief" in content
```

**Key elements:**
1. **Arrange** — Set up test environment (use fixtures)
2. **Act** — Run `graft` command via subprocess
3. **Assert** — Check exit code, JSON output, file state

### JSON Output Assertions

Most commands support `--json` for structured output:

```python
result = subprocess.run(
    ["graft", "status", str(artifact_path), "--json"],
    cwd=repo_path,
    capture_output=True,
    text=True
)

data = json.loads(result.stdout)
assert data["artifact"] == "sprint-brief"
assert data["status"] == "dirty"
assert len(data["downstream"]) > 0
```

**Benefits:**
- Machine-readable, stable format
- Easy to assert on specific fields
- Simulates how agents will use Graft

### Exit Code Assertions

Exit codes verify error handling:

```python
# Success case
result = subprocess.run(["graft", "run", str(artifact_path)])
assert result.returncode == 0

# User error (missing graft.yaml)
result = subprocess.run(["graft", "run", "/nonexistent/path"])
assert result.returncode == 1

# System error (Docker not available, etc.)
# returncode == 2
```

### File System Assertions

Verify outputs were created:

```python
# Check output exists
assert (artifact_path / "brief.md").exists()

# Check provenance created
provenance_file = artifact_path / ".graft" / "provenance" / "brief.json"
assert provenance_file.exists()

# Verify provenance content
provenance = json.loads(provenance_file.read_text())
assert provenance["artifact"] == "sprint-brief"
assert "materials" in provenance
```

## Test Categories

### Command Tests

One test file per command:

**`test_explain.py`** — Verify `graft explain`:
- Returns artifact configuration
- JSON output matches schema
- Handles missing graft.yaml
- Shows materials, derivations, policy

**`test_run.py`** — Verify `graft run`:
- Template-based derivations
- Container-based derivations
- Multiple derivations
- Error handling (missing materials, build failures)

**`test_status_finalize.py`** — Verify `graft status` and `graft finalize`:
- Detects dirty artifacts
- Shows downstream impact
- Finalize creates provenance
- Attestation captured correctly

**`test_impact_simulate.py`** — Verify `graft impact` and `graft simulate`:
- Impact shows downstream dependencies
- Simulate doesn't modify files
- Cascade mode works

**`test_dvc_orchestrator.py`** — Verify DVC integration:
- Autosync generates dvc.yaml
- Drift detection works
- Sync policies respected
- Non-managed stages preserved

### Integration Tests

Tests that exercise multiple commands together:

```python
def test_full_workflow(agile_ops_example):
    """Test run → edit → finalize → status workflow."""
    artifact = agile_ops_example / "artifacts" / "sprint-brief"

    # Run
    result = subprocess.run(["graft", "run", str(artifact)])
    assert result.returncode == 0

    # Edit output
    output = artifact / "brief.md"
    content = output.read_text()
    modified = content + "\n## Added Section\n"
    output.write_text(modified)

    # Finalize
    result = subprocess.run([
        "graft", "finalize", str(artifact),
        "--agent", "Test User"
    ])
    assert result.returncode == 0

    # Status should show clean
    result = subprocess.run(
        ["graft", "status", str(artifact), "--json"],
        capture_output=True,
        text=True
    )
    data = json.loads(result.stdout)
    assert data["status"] == "clean"
```

### Error Handling Tests

Verify graceful failures:

```python
def test_run_missing_material(tmp_path):
    """Test that missing materials produce user error."""
    # Create graft.yaml with non-existent material
    artifact = tmp_path / "artifact"
    artifact.mkdir()
    graft_yaml = artifact / "graft.yaml"
    graft_yaml.write_text("""
graft: test
inputs:
  materials:
    - { path: "/nonexistent/file.yaml", rev: HEAD }
derivations:
  - id: test
    outputs:
      - { path: "./output.md" }
""")

    result = subprocess.run(["graft", "run", str(artifact)])
    assert result.returncode == 1  # User error
```

## Running Tests

### Run All Tests

```bash
pytest
```

### Run Specific Test File

```bash
pytest tests/test_run.py
```

### Run Specific Test

```bash
pytest tests/test_run.py::test_run_sprint_brief
```

### Run with Verbose Output

```bash
pytest -v
```

### Run with Coverage

```bash
pytest --cov=graft --cov-report=html
```

### Run in Parallel

```bash
pytest -n auto
```

## Writing New Tests

### Checklist

When adding a new feature:

1. **Write test first** (TDD approach)
2. **Test via CLI** (no internal imports)
3. **Use fixtures** (agile_ops_example, tmp_path)
4. **Assert on:**
   - Exit code
   - JSON output (use `--json`)
   - File system state
5. **Test error cases** (not just happy path)
6. **Clean up** (fixtures should use tmp_path)

### Example: Adding a New Command

Suppose we add `graft validate`:

```python
# tests/test_validate.py

def test_validate_valid_artifact(agile_ops_example):
    """Test validate on valid artifact."""
    artifact = agile_ops_example / "artifacts" / "sprint-brief"

    result = subprocess.run(
        ["graft", "validate", str(artifact), "--json"],
        cwd=agile_ops_example,
        capture_output=True,
        text=True
    )

    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert data["valid"] == True

def test_validate_invalid_artifact(tmp_path):
    """Test validate on invalid artifact."""
    artifact = tmp_path / "artifact"
    artifact.mkdir()
    graft_yaml = artifact / "graft.yaml"
    graft_yaml.write_text("invalid: yaml: content")

    result = subprocess.run(
        ["graft", "validate", str(artifact)],
        capture_output=True,
        text=True
    )

    assert result.returncode == 1
    assert "invalid" in result.stderr.lower()
```

### Example: Testing Provenance

```python
def test_finalize_creates_provenance(agile_ops_example):
    """Test that finalize creates complete provenance."""
    artifact = agile_ops_example / "artifacts" / "sprint-brief"

    # Run and finalize
    subprocess.run(["graft", "run", str(artifact)], check=True)
    subprocess.run([
        "graft", "finalize", str(artifact),
        "--agent", "Test User"
    ], check=True)

    # Check provenance file
    prov_file = artifact / ".graft" / "provenance" / "brief.json"
    assert prov_file.exists()

    provenance = json.loads(prov_file.read_text())

    # Assert structure
    assert provenance["artifact"] == "sprint-brief"
    assert provenance["derivation_id"] == "brief"
    assert "materials" in provenance
    assert "outputs" in provenance
    assert "attestation" in provenance

    # Assert attestation
    assert provenance["attestation"]["agent"] == "Test User"
    assert "finalized_at" in provenance
```

## Testing Best Practices

**Use tmp_path** — Never modify examples/ directly. Copy to tmp_path for isolation.

**Initialize git** — Graft requires git. Fixtures should `git init`, `git add`, `git commit`.

**Test JSON output** — Most assertions should be on `--json` output, not parsing stderr.

**Test error codes** — Verify 0/1/2 exit codes distinguish success/user error/system error.

**Keep tests fast** — Use minimal fixtures. Don't test exhaustively, focus on important workflows.

**Name tests clearly** — `test_run_sprint_brief`, not `test_1`.

**One assertion theme per test** — Don't combine unrelated assertions. Split into multiple tests.

**Don't mock subprocess** — We're testing subprocess invocation. Mocking defeats the purpose.

## Continuous Integration

CI should run:

```bash
# Run all tests
pytest -v

# Check coverage
pytest --cov=graft --cov-report=term-missing

# Lint
ruff check src/

# Type check
mypy src/
```

Fail the build if:
- Any test fails
- Coverage drops below threshold (e.g., 80%)
- Linting errors
- Type errors

## Debugging Test Failures

**Inspect subprocess output:**
```python
result = subprocess.run(..., capture_output=True, text=True)
print("STDOUT:", result.stdout)
print("STDERR:", result.stderr)
print("EXIT CODE:", result.returncode)
```

**Run with pytest verbose:**
```bash
pytest -vv tests/test_run.py::test_run_sprint_brief
```

**Inspect tmp_path manually:**
```python
def test_debug(tmp_path):
    print("TMP PATH:", tmp_path)
    # Add breakpoint or sleep to inspect directory
    import pdb; pdb.set_trace()
```

**Check git state:**
```bash
cd /tmp/pytest-XXX/test_name/
git log --oneline
git status
```

## Future Testing Enhancements

**Property-based testing** — Use Hypothesis to generate random graft.yaml configs, verify invariants.

**Performance benchmarks** — Track command execution time, alert on regressions.

**Contract testing** — Verify JSON schemas stay stable across versions.

**Cross-platform testing** — Run tests on Linux, macOS, Windows in CI.

---

This testing strategy ensures Graft's CLI contract remains stable while allowing internal refactoring and evolution.

Next: See [Implementation Strategy](implementation-strategy.md) for development workflow.
