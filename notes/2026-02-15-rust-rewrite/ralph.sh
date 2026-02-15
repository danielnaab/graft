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

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MAX_ITERATIONS=${1:-20}
PROMPT_FILE="${SCRIPT_DIR}/prompt.md"
PLAN_FILE="${SCRIPT_DIR}/plan.md"
PROGRESS_FILE="${SCRIPT_DIR}/progress.md"

cd "$REPO_ROOT"

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
  echo ""
  echo "================================================================"
  echo "  Ralph iteration $i/$MAX_ITERATIONS"
  echo "  $(date '+%Y-%m-%d %H:%M:%S')"
  echo "================================================================"
  echo ""

  claude --dangerously-skip-permissions --print "$(cat "$PROMPT_FILE")"

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
