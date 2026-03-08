---
status: done
created: 2026-03-06
depends_on:
  - grove-durable-error-messages
---

# Surface dependency config loading errors with diagnostics

## Story

When `:scion start` fails because a dependency's `graft.yaml` can't be loaded,
the error message is "dependency 'software-factory' not found" — with no
indication of WHY it wasn't found. The root cause is `load_dep_configs()` in
`graft-engine/src/config.rs`, which uses `.ok()` to silently discard parse
errors, missing files, and uninitialized submodules. The downstream scion code
sees an empty `dep_configs` list and reports a generic "not found" error.

After this slice, `load_dep_configs` reports what went wrong for each dependency
that failed to load, and the scion "not found" error includes a diagnostic hint.
Users see exactly why the operation failed and how to fix it.

## Approach

Two changes at different layers:

### 1. Change `load_dep_configs` return type to include warnings

Current signature:
```rust
pub fn load_dep_configs(repo_path, config) -> Vec<(String, GraftConfig)>
```

New signature:
```rust
pub fn load_dep_configs(repo_path, config) -> (Vec<(String, GraftConfig)>, Vec<String>)
```

Returns `(successes, warnings)`. For each dependency where `parse_graft_yaml`
fails, push a diagnostic warning string instead of silently dropping:
- File not found: `"dependency '{name}': .graft/{name}/graft.yaml not found (submodule not initialized?)"`
- Parse error: `"dependency '{name}': failed to parse .graft/{name}/graft.yaml: {error}"`

### 2. Improve the scion "dependency not found" error message

In `scion.rs:732-735`, change:
```
"dependency '{dep}' not found for start command '{start_value}'"
```
to:
```
"dependency '{dep}' not found for start command '{start_value}'. \
 Check that .graft/{dep}/ exists and contains a valid graft.yaml"
```

### Caller updates

**Grove (4 sites in transcript.rs at lines 1674, 1701, 1750, 1783)**:

Before:
```rust
let dep_configs = config
    .as_ref()
    .map(|c| graft_engine::load_dep_configs(&repo_path, c))
    .unwrap_or_default();
```

After:
```rust
let (dep_configs, dep_warnings) = config
    .as_ref()
    .map(|c| graft_engine::load_dep_configs(&repo_path, c))
    .unwrap_or_default();
for w in &dep_warnings {
    self.show_warning(w);
}
```

Note: `unwrap_or_default()` works because `(Vec<_>, Vec<_>)` implements
`Default` (both fields default to empty vecs).

**graft-cli (4 sites in main.rs at lines 2766, 2781, 2803, 2837)**: Same
destructuring pattern. Print warnings to stderr via `eprintln!("warning: {w}")`.

## Acceptance Criteria

- `load_dep_configs` returns `(Vec<(String, GraftConfig)>, Vec<String>)`
- Missing dependency graft.yaml produces a warning mentioning the file path
- Unparseable dependency graft.yaml produces a warning with the parse error
- Successfully loaded dependencies still work as before
- Grove scion commands display warnings before attempting the operation
- graft CLI scion commands print warnings to stderr
- The scion "dependency not found" error includes a diagnostic hint
- `cargo test && cargo clippy -- -D warnings && cargo fmt --check` passes

## Steps

- [x] **Change `load_dep_configs` to return warnings**
  - **Delivers** — diagnostic visibility for dependency loading failures
  - **Done when** — `load_dep_configs` returns `(Vec<(String, GraftConfig)>,
    Vec<String>)`; for each dependency where `parse_graft_yaml` fails, the
    error is captured as a descriptive warning string; `.ok()` is replaced
    with explicit match on the error; existing tests updated
  - **Files** — `crates/graft-engine/src/config.rs`,
    `crates/graft-engine/src/lib.rs`

- [x] **Update grove scion handlers to surface dep warnings**
  - **Delivers** — grove users see why dependency configs failed to load
  - **Done when** — all 4 `load_dep_configs` call sites in transcript.rs
    destructure the return tuple; each warning is displayed via
    `self.show_warning(w)` (from grove-durable-error-messages slice)
  - **Files** — `crates/grove-cli/src/tui/transcript.rs`

- [x] **Update graft-cli callers to print dep warnings**
  - **Delivers** — CLI users see why dependency configs failed to load
  - **Done when** — all 4 `load_dep_configs` call sites in main.rs
    destructure the return tuple; each warning is printed to stderr
  - **Files** — `crates/graft-cli/src/main.rs`

- [x] **Improve scion "dependency not found" error message**
  - **Delivers** — actionable guidance when dependency lookup fails
  - **Done when** — error at `scion.rs:732-735` includes diagnostic hint:
    "Check that .graft/{dep}/ exists and contains a valid graft.yaml";
    existing scion tests updated to match new error text
  - **Files** — `crates/graft-engine/src/scion.rs`
