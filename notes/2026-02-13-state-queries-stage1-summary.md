---
status: deprecated
date: 2026-02-13
archived-reason: "Session complete, implementation merged"
---

# State Queries Stage 1 - Summary

Consolidated from session tracking documents. Canonical source: [state-queries spec](../docs/specifications/graft/state-queries.md).

## What Was Delivered

- **Domain models** (`src/graft/domain/state.py`): `StateCache`, `StateQuery` (with timeout), `StateResult` - frozen dataclasses, 100% tested
- **Config parsing** (`src/graft/services/config_service.py`): `state:` section in graft.yaml
- **State service** (`src/graft/services/state_service.py`): execute, cache, invalidate, temporal queries via git worktree
- **CLI** (`src/graft/cli/commands/state.py`): `graft state query <name>`, `graft state list`, `graft state invalidate`
- **Spec** (`docs/specifications/graft/state-queries.md`): behavioral specification with Gherkin scenarios

## Key Design Choices

- **JSON-only output**: structured data enables tooling and composition
- **Deterministic caching**: cache key = commit hash, no TTL (Stage 1)
- **Temporal queries**: git worktree for isolation, requires clean working tree
- **Cache format**: metadata wrapper (`metadata` + `data` sections) for evolution
- **Security model**: `shell=True` is safe because commands come from user's own graft.yaml (same trust model as Makefile)

## Improvements Applied (from critique)

1. **Temporal queries**: implemented with git worktree (was broken - only resolved hash without checkout)
2. **Timezone handling**: `datetime.now(UTC)` everywhere (was mixing naive/aware)
3. **Dirty tree check**: fail fast if working tree dirty when using `--commit`
4. **Command timeout**: configurable per query, default 300s
5. **Security docs**: docstring explaining `shell=True` trust model
6. **Cache corruption**: warnings to stderr, corrupted files deleted

## Test Coverage

- Domain: 100% (12 tests)
- Config parsing: 9 tests
- State service: 17 unit tests
- All 459 project tests passing

## Remaining Work

- End-to-end integration tests (`tests/integration/test_state_queries_e2e.py`) - not blocking merge
- Non-deterministic state (TTL caching) - Stage 2+
- Workspace aggregation - future stage

## Cache Structure

```
~/.cache/graft/{workspace-hash}/{repo-name}/state/{query-name}/{commit-hash}.json
```

## Sources

- [State queries spec](../docs/specifications/graft/state-queries.md)
- [Domain models](../src/graft/domain/state.py)
- [State service](../src/graft/services/state_service.py)
- [CLI commands](../src/graft/cli/commands/state.py)
