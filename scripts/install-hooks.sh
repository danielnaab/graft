#!/bin/bash
# Install git hooks for graft development

set -e

REPO_ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$REPO_ROOT/.githooks"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks for graft..."

# Check if hooks directory exists
if [ ! -d "$HOOKS_DIR" ]; then
    echo "Error: Hooks directory not found at $HOOKS_DIR"
    exit 1
fi

# Copy pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
    cp "$HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
    chmod +x "$GIT_HOOKS_DIR/pre-commit"
    echo "✓ Installed pre-commit hook"
else
    echo "Warning: pre-commit hook not found"
fi

echo ""
echo "Git hooks installed successfully!"
echo ""
echo "The pre-commit hook will run before each commit:"
echo "  - Tests (uv run pytest)"
echo "  - Type checking (uv run mypy src/)"
echo "  - Linting (uv run ruff check src/ tests/)"
echo ""
echo "If you need to skip the hook (emergency only), use:"
echo "  git commit --no-verify"
echo ""
