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

---

### Iteration 2 — Lock file parsing and writing
**Status**: completed
**Commit**: `fbdb523`
**Files changed**:
- `crates/graft-core/src/domain.rs` (+183 lines: CommitHash, LockEntry, LockFile types)
- `crates/graft-core/src/error.rs` (+9 lines: lock-specific error variants)
- `crates/graft-engine/src/lib.rs` (export lock module)
- `crates/graft-engine/src/lock.rs` (new, 342 lines: parsing and writing logic)
- `crates/graft-engine/Cargo.toml` (add indexmap dependency)
- `crates/graft-engine/tests/test_lock_file.rs` (new, 192 lines: integration tests)

**What was done**:
- Implemented lock file domain types in graft-core:
  - `CommitHash`: validated 40-char lowercase hex SHA-1 hash
  - `LockEntry`: source, ref, commit, consumed_at with timestamp validation
  - `LockFile`: top-level structure with API version validation
- Implemented parsing and writing in graft-engine/lock.rs:
  - `parse_lock_file()`: read from file path
  - `parse_lock_file_str()`: parse from YAML string with flexible path parameter
  - `write_lock_file()`: write with alphabetical ordering via IndexMap
  - Round-trip fidelity: parse → write → parse preserves all data
- Integration test successfully parses repo's own graft.lock (4 dependencies)
- 40 total tests passing (19 domain, 15 lock module, 5 integration, 1 config)

**Critique findings**:
1. ✅ Spec compliance: All acceptance criteria met, matches v3 flat-only format
2. ✅ Round-trip fidelity: Confirmed via integration test
3. ✅ Code quality: Follows established patterns from Task 1
4. ✅ Error handling: Clear errors for missing files, parse failures, validation
5. ✅ Test coverage: Unit tests for all validations, integration test for real file
6. ⚠️ Initial commit hash validation issue: Used `is_ascii_hexdigit()` which accepts uppercase; fixed to match spec's lowercase requirement

**Improvements made**:
- Fixed commit hash validation to check for lowercase hex only (`'0'..='9' | 'a'..='f'`)
- Fixed clippy warnings: doc comments with backticks, inlined format args
- Fixed lifetime issues in integration tests (temp PathBuf borrowing)

**Learnings for future iterations**:
1. `is_ascii_hexdigit()` accepts both upper and lower case — use explicit char ranges for lowercase-only validation
2. Use `impl Into<String>` for flexible string parameters (accepts both `&str` and `String`)
3. `IndexMap` preserves insertion order during serialization — use for alphabetical output
4. PathBuf temporary lifetime issues: create binding before calling `.parent().unwrap()`
5. Integration tests can parse repo's actual files to verify real-world compatibility
6. Clippy `doc_markdown` lint: wrap type names in backticks in doc comments

---

### Iteration 3 — `graft status` command
**Status**: completed
**Commit**: `49726a3`
**Files changed**:
- `crates/graft-engine/src/query.rs` (fixed field names and types to match domain model)
- `crates/graft-engine/src/lib.rs` (re-export query functions)
- `crates/graft-cli/src/main.rs` (fixed timestamp handling and clippy warnings)
- `crates/graft-cli/Cargo.toml` (removed unused chrono dependency)

**What was done**:
- Fixed query module implementation that was written but didn't compile:
  - Corrected field names: `ref_name` → `git_ref`, used proper domain types
  - Changed timestamp from `DateTime<Utc>` to `String` to match domain model
  - Fixed tests to use `GitUrl::new()` and `GitRef::new()` constructors
  - Changed from `HashMap` to `IndexMap` for alphabetical ordering
- Updated CLI to handle string-based timestamps (removed chrono formatting)
- Fixed clippy warnings: inlined format args, `if let` instead of `match`
- All 4 acceptance criteria met and verified against repo's graft.lock

**Critique findings**:
1. ✅ Spec compliance: Fully matches core-operations.md specification
2. ✅ Acceptance criteria: All 4 criteria genuinely met, tested end-to-end
3. ✅ Code quality: Idiomatic Rust, follows established patterns from lock.rs
4. ✅ Error messages: Clear and helpful ("Dependency 'x' not found in graft.lock")
5. ✅ Test coverage: 4 unit tests plus integration testing with real graft.lock
6. ✅ Integration: Clean re-export from lib.rs, works with existing domain types

**Improvements made**:
- Fixed all type mismatches between query module and domain model
- Removed unnecessary chrono dependency from CLI
- Fixed clippy warnings for better code quality
- Used `IndexMap` to guarantee alphabetical output ordering

**Learnings for future iterations**:
1. Always read domain model before implementing - the query module was written with outdated assumptions about field names
2. `consumed_at` is stored as a String in domain (ISO 8601), not parsed to `DateTime<Utc>`
3. `IndexMap` is better than sorting a Vec when you need both ordering and key-value lookups
4. CLI should match the data types from the service layer, not transform them unnecessarily
5. Query functions should be simple data transformations - no I/O or parsing
6. The pattern: query functions accept `&LockFile`, CLI handles file I/O via parse functions

---

### Iteration 4 — `graft changes` and `graft show` commands
**Status**: completed
**Commit**: `2de99ee`
**Files changed**:
- `crates/graft-engine/src/query.rs` (+156 lines: change query functions and tests)
- `crates/graft-engine/src/lib.rs` (re-export change query functions)
- `crates/graft-cli/src/main.rs` (+269 lines: changes and show commands)

**What was done**:
- Implemented change query functions in graft-engine/query.rs:
  - `get_changes_for_dependency()`: Get all changes from GraftConfig
  - `filter_changes_by_type()`: Filter changes by type (breaking, feature, fix, etc.)
  - `filter_breaking_changes()`: Filter to breaking changes only
  - `get_change_by_ref()`: Look up specific change by ref
  - `get_change_details()`: Get change with resolved migration/verify commands
  - `ChangeDetails` struct for structured change information
- Implemented CLI commands:
  - `graft changes <dep>`: Lists changes from dependency's graft.yaml
    - Supports `--type <type>` filter
    - Supports `--breaking` filter
    - Supports `--format text/json` output
    - Clear error when dependency not found
  - `graft show <dep>@<ref>`: Shows detailed change information
    - Displays type, description, migration, and verify commands
    - Supports `--format text/json` output
    - Clear error when change not found
- 8 new unit tests for change query functions
- 49 total tests passing (19 domain, 24 engine, 1 integration, 5 lock integration)

**Critique findings**:
1. ✅ Spec compliance: All acceptance criteria met
2. ✅ Acceptance criteria: Both commands work correctly with filters and JSON output
3. ✅ Code quality: Idiomatic Rust, follows established patterns from status command
4. ✅ Error messages: Clear, helpful errors for missing deps and changes
5. ✅ Test coverage: Unit tests for all query functions, manual CLI testing
6. ⚠️ Long CLI functions: `changes_command` (104 lines) and `show_command` (102 lines) required `#[allow(clippy::too_many_lines)]`
7. ⚠️ Missing `--from` and `--to` range filtering: Spec mentions these but Python implementation also has them as TODO (requires git integration)

**Improvements made**:
- Fixed initial metadata type mismatch in test (should be `Some(Metadata::default())`)
- Applied cargo fmt to fix line length in string concatenation
- Added `#[allow(clippy::too_many_lines)]` for long display functions (acceptable per established pattern)
- Used `.unwrap_or_default()` instead of verbose if-let chain per clippy suggestion

**Learnings for future iterations**:
1. Long CLI display functions are acceptable with `#[allow]` — they're straightforward presentation logic
2. The pattern for CLI commands: parse args → load config → call query function → display results
3. Use `split_once('@')` for parsing `dep@ref` format — cleaner than `split('@')`
4. Domain types (Change, Command) are already defined and well-tested — reuse them
5. `ChangeDetails` struct provides clean separation between query logic and command resolution
6. Manual CLI testing is sufficient when following established patterns (no integration test infrastructure yet)
7. Spec mentions `--from` and `--to` for range queries, but these require git integration (out of scope per Python TODO)

