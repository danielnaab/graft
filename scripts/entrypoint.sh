#!/usr/bin/env bash
set -euo pipefail

cmd="${1:-rebuild}"
shift || true

case "$cmd" in
  init)
    [ -f .env ] || cp -n .env.example .env || true
    if [ ! -d .dvc ]; then
      dvc init -q
    fi
    echo "Initialized DVC. Run bin/install-hooks from host to install git hooks."
    ;;
  sync)
    python3 scripts/generate_dvc.py
    ;;
  check)
    echo "Generating dvc.yaml from prompt files..."
    python3 scripts/generate_dvc.py
    ;;
  status)
    dvc status
    ;;
  new)
    name="${1:-}"
    topic="${2:-strategy}"
    if [ -z "$name" ]; then
      echo "Usage: bin/graft new <name> [topic]"
      echo "  name:  Document name (e.g., 'exec-summary')"
      echo "  topic: Topic directory under docs/ (default: strategy)"
      exit 2
    fi
    dir="docs/$topic"
    mkdir -p "$dir"
    cat > "$dir/$name.prompt.md" <<P
---
# Optional: override the model (default: bedrock-claude-v4.5-sonnet-us)
# model: bedrock-claude-v4.5-sonnet-us
deps:
  - docs/$topic/foundations.md
---
# ${name^} — Prompt
Write clear, concise strategic content. Edit only where source diffs imply semantic change.
P
    echo "Regenerating dvc.yaml..."
    python3 scripts/generate_dvc.py
    echo "Scaffolded docs/$topic/$name.prompt.md"
    ;;
  diff)
    name="${1:-}"
    if [ -z "$name" ]; then
      echo "Usage: bin/graft diff <stage-name>"
      echo "  Example: bin/graft diff exec_summary"
      exit 2
    fi
    # Find the prompt file for this stage
    prompt_file=$(find docs -name "*.prompt.md" -type f | while read p; do
      stage=$(basename "$p" .prompt.md | tr '-' '_')
      if [ "$stage" = "$name" ]; then echo "$p"; break; fi
    done)
    if [ -z "$prompt_file" ]; then
      echo "Error: Could not find prompt file for stage '$name'"
      exit 1
    fi
    out_file="${prompt_file%.prompt.md}.md"
    python3 scripts/pack_prompt.py --prompt "$prompt_file" --prev "$out_file" --out "build/${name}.promptpack.txt" --params-out "build/${name}.params.json" --name "${name}"
    echo "build/${name}.promptpack.txt"
    ;;
  uses)
    file="${1:-}"
    if [ -z "$file" ]; then
      echo "Usage: bin/graft uses <file>"
      echo "  Example: bin/graft uses docs/strategy/messaging-framework.md"
      echo ""
      echo "Shows which prompts depend on the given file (reverse dependency lookup)."
      exit 2
    fi
    python3 scripts/find_uses.py "$file"
    ;;
  rebuild|"")
    echo "🔧 Generating dvc.yaml from prompt files..." >&2
    python3 scripts/generate_dvc.py

    # Show pipeline summary before running
    total_stages=$(dvc status 2>/dev/null | grep -c "changed" || echo "0")
    if [[ "$total_stages" -gt 0 ]]; then
      echo "📋 Pipeline has $total_stages stage(s) to run" >&2
      echo "" >&2
    fi

    echo "🚀 Running DVC pipeline..." >&2
    dvc repro

    echo "" >&2
    echo "✅ Done." >&2
    ;;
  help|--help|-h)
    echo "Usage: bin/graft [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  init            Initialize project (DVC + hooks)"
    echo "  sync            Regenerate dvc.yaml from prompt files"
    echo "  check           Alias for sync"
    echo "  status          Show DVC pipeline status"
    echo "  new <name>      Create a new document prompt"
    echo "  diff <stage>    Inspect prompt context for a stage"
    echo "  uses <file>     Show which prompts depend on a file"
    echo "  rebuild         Regenerate dvc.yaml and run pipeline (default)"
    echo "  help            Show this help message"
    ;;
  *)
    echo "Error: Unknown command '$cmd'"
    echo ""
    echo "Usage: bin/graft [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  init            Initialize project (DVC + hooks)"
    echo "  sync            Regenerate dvc.yaml from prompt files"
    echo "  check           Alias for sync"
    echo "  status          Show DVC pipeline status"
    echo "  new <name>      Create a new document prompt"
    echo "  diff <stage>    Inspect prompt context for a stage"
    echo "  uses <file>     Show which prompts depend on a file"
    echo "  rebuild         Regenerate dvc.yaml and run pipeline (default)"
    echo "  help            Show this help message"
    exit 2
    ;;
esac
