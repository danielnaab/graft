#!/usr/bin/env bash
# State query: verification status for graft.yaml state.verify
# Produces JSON: { format, lint, tests }
#
# Runs all three checks regardless of individual failures.
# Output is truncated to keep state query results a reasonable size.

MAX_LINES=40

fmt_output=$(cargo fmt --all --check 2>&1)
fmt_exit=$?

lint_output=$(cargo clippy -- -D warnings 2>&1)
lint_exit=$?

test_output=$(cargo test 2>&1)
test_exit=$?

# Truncate to first N lines (where errors appear) for format and lint.
# Truncate to last N lines (where the summary appears) for tests.
fmt_output=$(echo "$fmt_output" | head -n "$MAX_LINES")
lint_output=$(echo "$lint_output" | head -n "$MAX_LINES")
test_output=$(echo "$test_output" | tail -n "$MAX_LINES")

jq -n \
  --arg format "$fmt_output" \
  --arg lint "$lint_output" \
  --arg tests "$test_output" \
  '{format: $format, lint: $lint, tests: $tests}'
