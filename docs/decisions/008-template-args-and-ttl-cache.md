# ADR 008: Template Args and TTL Cache

**Date**: 2026-02-21
**Status**: Implemented

## Context

The plan command introduced two engine gaps during the first software-factory template integration:

1. **Template args**: Templates had no mechanism to receive user-provided input (like a task description). CLI trailing args were appended to the `run:` command, not the template renderer.
2. **TTL cache**: State queries with `deterministic: true` cache indefinitely by commit hash, which is wrong for queries that inspect the working tree (e.g., `cargo fmt --check`). The only alternative was `deterministic: false` which disables caching entirely.

## Decision

### Template Args

CLI trailing arguments are available as `{{ args }}` in stdin templates. When a command has `stdin:` configured:
- Arguments are joined as a string and injected into `TemplateContext` as `args`
- Arguments are NOT appended to the `run:` command (consumed by the template instead)
- When no arguments are provided, `{{ args }}` is undefined (templates use `{% if args is defined %}` guards)

### TTL Cache

A `cache.ttl` field (seconds) expires cached results after a duration:
- Only meaningful when `deterministic: true` (commit-keyed cache)
- Results older than `ttl` seconds are re-executed even if the commit hash matches
- Results without TTL retain current behavior (valid indefinitely when deterministic)

## Behavioral Expectations

### Template Args

```gherkin
Given a command with stdin template
When the user runs the command with trailing arguments
Then the arguments are joined as a string and available as {{ args }} in the template
And the arguments are NOT appended to the run command
```

```gherkin
Given a command with stdin template
When the user runs the command without trailing arguments
Then {{ args }} is undefined in the template
And templates can use {% if args is defined %} guards
```

### TTL Cache

```gherkin
Given a state query with cache.ttl set to 120
When a cached result exists for the current commit
And the cached result is older than 120 seconds
Then graft re-executes the query
And caches the new result
```

```gherkin
Given a state query with cache.ttl set to 120
When a cached result exists for the current commit
And the cached result is newer than 120 seconds
Then graft returns the cached result
```

```gherkin
Given a state query with deterministic cache and no TTL
When a cached result exists for the current commit
Then graft returns the cached result indefinitely
```

## Implementation

### Template Args

- `crates/graft-engine/src/template.rs` — `TemplateContext::new()` accepts `args: &[String]`, joins them, and injects `args` into the Tera context
- `crates/graft-engine/src/command.rs` — `execute_command_with_context()` passes CLI args to `TemplateContext`; when `stdin` is present, args are not appended to the shell command

### TTL Cache

- `crates/graft-engine/src/domain.rs` — `StateCache` has `ttl: Option<u64>` field
- `crates/graft-engine/src/config.rs` — Parses `cache.ttl` from YAML
- `crates/graft-engine/src/state.rs` — Cache lookup checks TTL expiry against file modification time

### Configuration Example

```yaml
state:
  verify:
    run: "cargo fmt --check && cargo clippy -- -D warnings && cargo test"
    cache:
      deterministic: true
      ttl: 120  # re-run if cache is older than 2 minutes
    timeout: 120
```
