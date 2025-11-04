#!/bin/bash
# Test validation logic locally

echo "=== Graft Validation Test ==="
echo ""

# Test 1: DVC synchronization
echo "Test 1: DVC Synchronization"
GRAFT_DIR=. bin/graft sync > /tmp/sync-test.log 2>&1
if git diff --quiet dvc.yaml; then
  echo "✅ PASS: dvc.yaml is synchronized"
else
  echo "❌ FAIL: dvc.yaml needs update"
  echo "Run: git add dvc.yaml"
fi
echo ""

# Test 2: Prompt file validation
echo "Test 2: Prompt File Validation"
if python3 scripts/validate.py; then
  echo "✅ PASS: All prompt files validated"
else
  echo "❌ FAIL: Prompt validation failed"
fi
echo ""

# Test 3: Stale documentation
echo "Test 3: Stale Documentation"
status_output=$(GRAFT_DIR=. bin/graft status 2>&1 || true)
if echo "$status_output" | grep -q "changed"; then
  echo "❌ FAIL: Stale documentation detected"
  echo "$status_output" | grep "changed" | head -5
  echo "Run: bin/graft rebuild"
else
  echo "✅ PASS: All documentation is up to date"
fi
echo ""

echo "=== Validation Complete ==="
