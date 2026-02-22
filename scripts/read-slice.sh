#!/usr/bin/env bash
# Read a slice's plan.md and output structured JSON.
#
# Usage: bash scripts/read-slice.sh slices/<slug>
#
# Output JSON:
# {
#   "status": "draft",
#   "slug": "<slug>",
#   "steps_total": 3,
#   "steps_done": 1,
#   "next_step_number": 2,
#   "next_step": "**Step name**\n  - **Delivers** -- ...",
#   "story": "...",
#   "approach": "...",
#   "acceptance_criteria": "...",
#   "content": "<full file content>"
# }

set -euo pipefail

SLICE_DIR="${1:?Usage: read-slice.sh slices/<slug>}"
PLAN_FILE="$SLICE_DIR/plan.md"

if [ ! -f "$PLAN_FILE" ]; then
  echo "Error: $PLAN_FILE not found" >&2
  exit 1
fi

content=$(cat "$PLAN_FILE")

# Extract status from frontmatter
status=$(echo "$content" | sed -n '/^---$/,/^---$/{ /^status:/{ s/^status:[[:space:]]*//; s/["\x27]//g; p; } }')
: "${status:=draft}"

slug=$(basename "$SLICE_DIR")

# Count checkboxes
steps_total=$(echo "$content" | grep -cE '^\s*- \[[ x]\]' || true)
steps_done=$(echo "$content" | grep -cE '^\s*- \[x\]' || true)
: "${steps_total:=0}"
: "${steps_done:=0}"

# Find the next unchecked step number (1-indexed)
next_step_number=0
step_idx=0
while IFS= read -r line; do
  if echo "$line" | grep -qE '^\s*- \[[ x]\]'; then
    step_idx=$((step_idx + 1))
    if echo "$line" | grep -qE '^\s*- \[ \]'; then
      next_step_number=$step_idx
      break
    fi
  fi
done <<< "$content"

# Extract the next unchecked step block (from `- [ ]` to the next `- [` or end of steps)
next_step=""
if [ "$next_step_number" -gt 0 ]; then
  in_step=false
  while IFS= read -r line; do
    if [ "$in_step" = true ]; then
      # Stop at next checkbox or section header
      if echo "$line" | grep -qE '^\s*- \[[ x]\]|^##'; then
        break
      fi
      next_step="$next_step
$line"
    fi
    if echo "$line" | grep -qE '^\s*- \[ \]' && [ "$in_step" = false ]; then
      # Find the right unchecked step by counting
      in_step=true
      next_step="$line"
    fi
  done <<< "$content"

  # If we found multiple unchecked steps, we only want the first one
  # Re-extract: find the Nth step (next_step_number) specifically
  next_step=""
  in_step=false
  current=0
  while IFS= read -r line; do
    if echo "$line" | grep -qE '^\s*- \[[ x]\]'; then
      current=$((current + 1))
      if [ "$in_step" = true ]; then
        break
      fi
      if [ "$current" -eq "$next_step_number" ]; then
        in_step=true
        next_step="$line"
        continue
      fi
    elif [ "$in_step" = true ]; then
      if echo "$line" | grep -qE '^##'; then
        break
      fi
      next_step="$next_step
$line"
    fi
  done <<< "$content"
fi

# Extract sections by header
extract_section() {
  local header="$1"
  local text="$2"
  local in_section=false
  local result=""
  while IFS= read -r line; do
    if echo "$line" | grep -qE "^##[[:space:]]+${header}$"; then
      in_section=true
      continue
    fi
    if [ "$in_section" = true ]; then
      if echo "$line" | grep -qE '^##[[:space:]]'; then
        break
      fi
      result="$result$line
"
    fi
  done <<< "$text"
  # Trim leading/trailing whitespace
  echo "$result" | sed -e 's/^[[:space:]]*//' -e '/^$/d' | head -c 2000
}

story=$(extract_section "Story" "$content")
approach=$(extract_section "Approach" "$content")
acceptance_criteria=$(extract_section "Acceptance Criteria" "$content")

jq -n \
  --arg status "$status" \
  --arg slug "$slug" \
  --argjson steps_total "$steps_total" \
  --argjson steps_done "$steps_done" \
  --argjson next_step_number "$next_step_number" \
  --arg next_step "$next_step" \
  --arg story "$story" \
  --arg approach "$approach" \
  --arg acceptance_criteria "$acceptance_criteria" \
  --arg content "$content" \
  '{status: $status, slug: $slug, steps_total: $steps_total, steps_done: $steps_done, next_step_number: $next_step_number, next_step: $next_step, story: $story, approach: $approach, acceptance_criteria: $acceptance_criteria, content: $content}'
