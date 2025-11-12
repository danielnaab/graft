# 4. Black-box testing via CLI

Date: 2025-11-12

## Status

Accepted

## Context

As a CLI tool, Graft's primary interface is its command-line commands. Users (humans and AI agents) interact exclusively through this interface, never importing internal modules. The CLI is our public API.

Testing strategies for CLI tools typically fall into three categories:

1. **Unit testing internal modules** — Fast, but couples tests to implementation; refactoring breaks tests even when user behavior unchanged
2. **Integration testing with internal imports** — Tests service layer directly, but bypasses CLI; doesn't verify the actual user contract
3. **Black-box subprocess testing** — Invokes CLI as subprocess, tests from user perspective; slower but validates what actually matters

Given Graft's goals:
- CLI stability is critical (breaking CLI = breaking all users)
- AI agents will invoke CLI programmatically (via `--json` output)
- We want freedom to refactor internals without breaking tests
- The CLI contract is more important than implementation details
- Tests should verify user experience, not internal structure
- Once the contract is stable, we may rewrite in another language

## Decision

We will prioritize **black-box subprocess testing** as the primary testing strategy for Graft.

All black-box tests execute `graft` commands via Python's `subprocess` module and assert on:
- Exit codes (0 = success, 1 = user error, 2 = system error)
- JSON output (via `--json` flag)
- File system state (what files were created/modified)
- Human-readable output (stdout/stderr)

Black-box tests will **not** import internal Graft modules. The CLI is the tested interface.

**Unit tests will still exist** for internal utilities, adapters, and complex logic where isolation is valuable. However, the majority of testing effort focuses on black-box CLI tests because they verify the actual user contract.

### Testing Approach

**Fixtures** — Copy example artifacts to temporary directories, initialize git, provide clean test environments

**Assertions** — Verify behavior via CLI outputs, not internal state

**JSON as contract** — All commands support `--json` for structured, machine-readable output that tests can assert against

**Full workflow coverage** — Tests exercise complete user workflows (run → edit → finalize → status) rather than isolated functions

## Consequences

**Positive:**

- **Tests verify user contract** — If tests pass, users can trust the CLI works
- **Maximum refactoring freedom** — Can rewrite internals completely, even in another language; tests only care about CLI behavior
- **Simulates real usage** — Tests execute exactly how users (and AI agents) will use Graft
- **Forces CLI stability** — Breaking tests means breaking users; makes us careful about CLI changes
- **No test coupling** — Internal restructuring (moving functions, renaming classes, changing languages) doesn't break tests
- **JSON output validated** — Agents will rely on `--json`; testing this ensures agent-friendliness
- **Subprocess isolation** — Each test runs in clean environment, no shared state between tests

**Negative:**

- **Slower than unit tests** — Subprocess overhead makes tests slower (but still fast enough: seconds, not minutes)
- **Debugging is harder** — Failures require inspecting subprocess output rather than stepping through code
- **Setup overhead** — Need to copy fixtures, initialize git, manage temporary directories

**Neutral:**

- **Test coverage is CLI-focused** — Can't measure "line coverage" meaningfully; coverage is "command coverage"
- **Mocking is rare** — Can't easily mock internal adapters in black-box tests (but this encourages testing with real implementations)
- **Fixtures are required** — Every test needs realistic artifact directories and git state
- **Unit tests supplement** — Some internal logic may benefit from isolated unit tests, but they're secondary to CLI tests

## Trade-offs Accepted

**Speed vs. Contract Verification**
- Unit tests would be faster
- But subprocess tests verify what actually matters: the CLI contract
- Trade-off: slightly slower CI, but higher confidence in user-facing behavior

**Internal Testing vs. Refactoring Freedom**
- Could test services/adapters directly as primary strategy
- But that couples tests to current structure and language
- Trade-off: can't verify all internal functions through tests, but can restructure or rewrite fearlessly

**Setup Complexity vs. Real Environments**
- Could mock filesystem, git, Docker
- But subprocess tests use real implementations
- Trade-off: more setup code, but tests catch real-world issues

## Implementation Notes

- Use pytest fixtures for test artifact setup
- Copy `examples/` to `tmp_path` for isolated tests
- Initialize git in test fixtures (Graft requires git)
- Assert on JSON output for structured assertions
- Assert on exit codes to verify error handling
- Assert on file system state to verify outputs created
- Keep tests focused: one workflow per test function
- Unit tests may exist for internal utilities but are not the primary testing strategy

This approach ensures Graft's CLI remains stable and trustworthy while allowing internal evolution, including potential rewrites in other languages once the problem space is well understood.
