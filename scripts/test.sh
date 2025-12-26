#!/usr/bin/env bash
# Test script for Graft
# Runs tests with coverage

set -e  # Exit on error

echo "==> Running tests for Graft"

# Run tests with coverage
uv run pytest

# Check exit code
if [ $? -eq 0 ]; then
    echo ""
    echo "All tests passed! ✓"
    echo "Coverage report: htmlcov/index.html"
else
    echo ""
    echo "Some tests failed! ✗"
    exit 1
fi
