#!/usr/bin/env bash
# Ralph loop for graft Rust rewrite.
#
# Usage:
#   ./notes/2026-02-15-rust-rewrite/ralph.sh          # run until done (max 20 iterations)
#   ./notes/2026-02-15-rust-rewrite/ralph.sh 5         # run up to 5 iterations
#
# The loop runs claude --print with the prompt, which reads the plan, picks the
# next incomplete task, implements it, verifies it, commits, and logs progress.
# The loop exits when all tasks are marked [x] or max iterations are reached.

set -euo pipefail

# Kill the entire process group on Ctrl+C or SIGTERM
trap 'echo ""; echo "Interrupted. Stopping..."; kill 0' SIGINT SIGTERM

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MAX_ITERATIONS=${1:-20}
MODEL="${RALPH_MODEL:-sonnet}"
PROMPT_FILE="${SCRIPT_DIR}/prompt.md"
PLAN_FILE="${SCRIPT_DIR}/plan.md"
PROGRESS_FILE="${SCRIPT_DIR}/progress.md"
LOG_DIR="${SCRIPT_DIR}/logs"

cd "$REPO_ROOT"
mkdir -p "$LOG_DIR"

# Initialize progress file if it doesn't exist
if [[ ! -f "$PROGRESS_FILE" ]]; then
  cat > "$PROGRESS_FILE" <<'EOF'
---
status: working
purpose: "Append-only progress log for graft Rust rewrite Ralph loop"
---

# Progress Log

## Consolidated Patterns

(Patterns discovered across iterations that future iterations should know about)

---

EOF
fi

for ((i=1; i<=MAX_ITERATIONS; i++)); do
  LOG_FILE="${LOG_DIR}/iteration-${i}.log"

  echo ""
  echo "================================================================"
  echo "  Ralph iteration $i/$MAX_ITERATIONS"
  echo "  $(date '+%Y-%m-%d %H:%M:%S')"
  echo "  Model: $MODEL"
  echo "  Log: $LOG_FILE"
  echo "================================================================"
  echo ""

  # --verbose streams tool-call activity to stderr so you can follow along.
  # All output is also captured to a per-iteration log file.
  claude --dangerously-skip-permissions --print --verbose --model "$MODEL" \
    "$(cat "$PROMPT_FILE")" 2>&1 | tee "$LOG_FILE"

  # Check if all tasks are complete
  if ! grep -q '^\- \[ \]' "$PLAN_FILE"; then
    echo ""
    echo "================================================================"
    echo "  All tasks complete after $i iterations."
    echo "================================================================"
    exit 0
  fi

  echo "=== Iteration $i complete, tasks remain ==="
done

echo ""
echo "================================================================"
echo "  Max iterations ($MAX_ITERATIONS) reached. Tasks remain."
echo "================================================================"
exit 1
