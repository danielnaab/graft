---
status: working
purpose: "Append-only progress log for graft Rust rewrite Ralph loop"
---

# Progress Log

## Consolidated Patterns

### Domain Type Patterns (from Task 1)
- **Newtype pattern for validated strings**: Wrap primitives (String) in struct, validate in constructor
  - Example: `GitRef(String)` with validation in `::new()`
  - Use `#[serde(try_from = "String", into = "String")]` for serde support
- **Builder pattern with #[must_use]**: Chain methods return Self, all marked with `#[must_use]`
  - Example: `Change::new("v1.0.0")?.with_type("breaking").with_migration("migrate")`
  - Clippy enforces `#[must_use]` on all builder methods
- **Cross-field validation**: Use a separate `validate()` method called after construction
  - Example: `GraftConfig` validates that migration commands exist in commands section
- **Regex without lookahead**: Rust regex doesn't support lookahead/lookbehind
  - Replace `(?!pattern)` with manual checks after matching
  - Example: `if !path.starts_with("//")` instead of `(?!//)`

### Module Organization (from graft-core, graft-engine)
- **graft-core**: Only domain types, error types, traits. No I/O, no business logic
- **graft-engine**: Business logic, config parsing, adapters. Can read files, execute commands

### Testing Patterns
- Unit tests inline in domain.rs with `#[cfg(test)] mod tests`
- Integration tests in `tests/` directory for end-to-end scenarios
- Integration test successfully parsed repo's own graft.yaml

---

### Iteration 1 — graft.yaml parsing and domain model
**Status**: completed
**Commit**: `07cefea`
**Files changed**:
- `crates/graft-core/src/lib.rs`
- `crates/graft-core/src/domain.rs` (new, 648 lines)
- `crates/graft-core/src/error.rs` (new, 41 lines)
- `crates/graft-core/Cargo.toml`
- `crates/graft-engine/src/lib.rs`
- `crates/graft-engine/src/config.rs` (new, 519 lines)
- `crates/graft-engine/tests/integration_test.rs` (new)

**What was done**:
- Implemented complete domain model in graft-core:
  - `GitRef`, `GitUrl`: validated newtypes for git references and URLs
  - `DependencySpec`: dependency declaration with name validation
  - `Change`: semantic change with type, description, migration, verify fields
  - `Command`: executable command (validates no colons in name per spec)
  - `Metadata`, `GraftConfig`: top-level config structure
- Implemented config parsing in graft-engine:
  - `parse_graft_yaml()`: parse from file path
  - `parse_graft_yaml_str()`: parse from string
  - Supports both "deps: {name: url#ref}" and "dependencies" formats
  - Cross-field validation (migration commands must exist)
- URL normalization: SCP-style `git@host:path` → `ssh://git@host/path`
- 23 tests passing (14 unit, 8 config, 1 integration)

**Critique findings**:
1. ✅ Spec compliance: All acceptance criteria met
2. ✅ Validation: Matches Python implementation plus spec requirements
3. ✅ Code quality: Idiomatic Rust, follows grove-core patterns
4. ✅ Error messages: Clear, structured errors with field paths
5. ✅ Test coverage: Unit tests for validation, integration test for real file

**Improvements made**:
None needed — implementation complete and clean on first pass.

**Learnings for future iterations**:
1. Rust regex doesn't support lookahead — use manual post-match checks instead
2. Clippy enforces `#[must_use]` on builder methods — add proactively
3. Use `&str` parameters instead of `String` where possible to avoid clones
4. The `#[allow(clippy::too_many_lines)]` is acceptable for parsing functions
5. Integration tests in `tests/` dir can access repo files via `CARGO_MANIFEST_DIR`
6. Newtype pattern + validation in constructor = compile-time safety

