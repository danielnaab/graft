#!/usr/bin/env bash
# State query: list slice directories and their plan status
# Produces JSON: { slices: [...], counts: { draft: N, accepted: N, in_progress: N, done: N } }
#
# Each slice entry: { path, status, slug, steps_total, steps_done }
# Convention: slices/<slug>/plan.md
# Handles missing slices/ directory gracefully.

SLICES_DIR="slices"

# If directory doesn't exist, return empty result
if [ ! -d "$SLICES_DIR" ]; then
  echo '{"slices":[],"counts":{"draft":0,"accepted":0,"in_progress":0,"done":0}}'
  exit 0
fi

# Find all plan.md files in slice directories
files=$(find "$SLICES_DIR" -mindepth 2 -maxdepth 2 -name 'plan.md' -type f 2>/dev/null | sort)

if [ -z "$files" ]; then
  echo '{"slices":[],"counts":{"draft":0,"accepted":0,"in_progress":0,"done":0}}'
  exit 0
fi

draft=0
accepted=0
in_progress=0
done_count=0
slices_json="[]"

for file in $files; do
  # Extract status from YAML frontmatter (between --- markers)
  status=""
  in_frontmatter=false
  while IFS= read -r line; do
    if [ "$line" = "---" ]; then
      if [ "$in_frontmatter" = true ]; then
        break
      else
        in_frontmatter=true
        continue
      fi
    fi
    if [ "$in_frontmatter" = true ]; then
      case "$line" in
        status:*)
          status=$(echo "$line" | sed 's/^status:[[:space:]]*//' | tr -d '"' | tr -d "'")
          ;;
      esac
    fi
  done < "$file"

  # Default to draft if no status found
  if [ -z "$status" ]; then
    status="draft"
  fi

  # Count checkboxes (grep -c exits 1 when count is 0, so use `|| true`)
  steps_total=$(grep -cE '^\s*- \[[ x]\]' "$file" 2>/dev/null || true)
  steps_done=$(grep -cE '^\s*- \[x\]' "$file" 2>/dev/null || true)
  : "${steps_total:=0}"
  : "${steps_done:=0}"

  # Extract slug from parent directory name
  slug=$(basename "$(dirname "$file")")

  # Build slice JSON entry
  slices_json=$(echo "$slices_json" | jq --arg path "$file" \
    --arg status "$status" \
    --arg slug "$slug" \
    --argjson steps_total "$steps_total" \
    --argjson steps_done "$steps_done" \
    '. + [{"path": $path, "status": $status, "slug": $slug, "steps_total": $steps_total, "steps_done": $steps_done}]')

  # Update counts
  case "$status" in
    draft) draft=$((draft + 1)) ;;
    accepted) accepted=$((accepted + 1)) ;;
    in-progress) in_progress=$((in_progress + 1)) ;;
    done) done_count=$((done_count + 1)) ;;
  esac
done

jq -n \
  --argjson slices "$slices_json" \
  --argjson draft "$draft" \
  --argjson accepted "$accepted" \
  --argjson in_progress "$in_progress" \
  --argjson done "$done_count" \
  '{slices: $slices, counts: {draft: $draft, accepted: $accepted, in_progress: $in_progress, done: $done}}'
