#!/usr/bin/env bash
# State query: verification status for graft.yaml state.verify
# Produces JSON: { format, lint, tests }
#
# Runs all three checks regardless of individual failures.
# On success: compact summary. On failure: first N lines of output.

MAX_LINES=40

fmt_output=$(cargo fmt --all --check 2>&1)
fmt_exit=$?

lint_output=$(cargo clippy -- -D warnings 2>&1)
lint_exit=$?

test_output=$(cargo test 2>&1)
test_exit=$?

# On success, produce a compact summary.
# On failure, keep the first N lines where errors appear.
if [ $fmt_exit -eq 0 ]; then
  fmt_output="OK"
else
  fmt_output=$(echo "$fmt_output" | head -n "$MAX_LINES")
fi

if [ $lint_exit -eq 0 ]; then
  lint_output="OK"
else
  lint_output=$(echo "$lint_output" | head -n "$MAX_LINES")
fi

if [ $test_exit -eq 0 ]; then
  # Sum passed/failed/ignored counts across all crates into one line.
  total_passed=$(echo "$test_output" | grep -oP '\d+ passed' | awk '{s+=$1} END {print s+0}')
  total_failed=$(echo "$test_output" | grep -oP '\d+ failed' | awk '{s+=$1} END {print s+0}')
  total_ignored=$(echo "$test_output" | grep -oP '\d+ ignored' | awk '{s+=$1} END {print s+0}')
  test_output="OK. ${total_passed} passed, ${total_failed} failed, ${total_ignored} ignored"
else
  test_output=$(echo "$test_output" | head -n "$MAX_LINES")
fi

jq -n \
  --arg format "$fmt_output" \
  --arg lint "$lint_output" \
  --arg tests "$test_output" \
  '{format: $format, lint: $lint, tests: $tests}'
