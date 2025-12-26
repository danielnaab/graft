#!/usr/bin/env bash
# Setup script for Graft
# Checks prerequisites and installs dependencies

set -e  # Exit on error

echo "==> Setting up Graft"

# Check Python version
echo "Checking Python version..."
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | cut -d ' ' -f 2)
    echo "Found Python $PYTHON_VERSION"
else
    echo "Error: Python 3 not found"
    exit 1
fi

# Check uv
echo "Checking for uv..."
if ! command -v uv &> /dev/null; then
    echo "Error: uv not found"
    echo "Install uv: https://docs.astral.sh/uv/"
    echo "  curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

UV_VERSION=$(uv --version | cut -d ' ' -f 2)
echo "Found uv $UV_VERSION"

# Install dependencies
echo "Installing dependencies..."
uv sync

echo "Setup complete!"
echo ""
echo "Next steps:"
echo "  Run tests:    uv run pytest"
echo "  Run linter:   uv run ruff check ."
echo "  Run CLI:      uv run graft-cli --help"
