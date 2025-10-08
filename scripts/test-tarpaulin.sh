#!/bin/bash
# Test tarpaulin with the same configuration as pre-commit hook

set -e

echo "Running tarpaulin with pre-commit hook configuration..."
echo ""
echo "Command: cargo tarpaulin --lib --exclude-files \"tests/*\" --out Stdout --fail-under 33 --skip-clean --timeout 120"
echo ""

cargo tarpaulin \
  --lib \
  --exclude-files "tests/*" \
  --out Stdout \
  --fail-under 33 \
  --skip-clean \
  --timeout 120

echo ""
echo "✅ Coverage check passed!"
