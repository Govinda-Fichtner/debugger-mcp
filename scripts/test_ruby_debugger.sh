#!/bin/bash
# Test script to verify Ruby debugger works correctly
# This script tests the command-line argument structure without running DAP

set -e

echo "=== Ruby Debugger Verification Script ==="
echo ""

# Test 1: Verify rdbg is installed in Docker image
echo "Test 1: Checking if rdbg is installed in Docker image..."
if docker run --rm debugger-mcp:ruby rdbg --version > /dev/null 2>&1; then
    echo "✅ rdbg is installed"
else
    echo "❌ rdbg is NOT installed"
    exit 1
fi
echo ""

# Test 2: Verify rdbg help works
echo "Test 2: Checking rdbg help output..."
HELP_OUTPUT=$(docker run --rm debugger-mcp:ruby rdbg --help 2>&1 | head -5)
if echo "$HELP_OUTPUT" | grep -q "rdbg \[options\]"; then
    echo "✅ rdbg help works"
else
    echo "❌ rdbg help failed"
    exit 1
fi
echo ""

# Test 3: Verify --stop-at-load flag is recognized
echo "Test 3: Checking --stop-at-load flag..."
if docker run --rm debugger-mcp:ruby rdbg --help 2>&1 | grep -q "stop-at-load"; then
    echo "✅ --stop-at-load flag exists"
else
    echo "❌ --stop-at-load flag not found"
    exit 1
fi
echo ""

# Test 4: Verify --nonstop flag is recognized
echo "Test 4: Checking --nonstop flag..."
if docker run --rm debugger-mcp:ruby rdbg --help 2>&1 | grep -q "nonstop"; then
    echo "✅ --nonstop flag exists"
else
    echo "❌ --nonstop flag not found"
    exit 1
fi
echo ""

# Test 5: Verify Ruby version
echo "Test 5: Checking Ruby version..."
RUBY_VERSION=$(docker run --rm debugger-mcp:ruby ruby --version)
echo "Ruby version: $RUBY_VERSION"
if echo "$RUBY_VERSION" | grep -q "ruby [0-9]"; then
    echo "✅ Ruby is installed"
else
    echo "❌ Ruby version check failed"
    exit 1
fi
echo ""

# Test 6: Verify fizzbuzz.rb test file exists
echo "Test 6: Checking test fixture..."
if [ -f "/home/vagrant/projects/debugger_mcp/tests/fixtures/fizzbuzz.rb" ]; then
    echo "✅ fizzbuzz.rb fixture exists"
else
    echo "⚠️  fizzbuzz.rb fixture not found at tests/fixtures/fizzbuzz.rb"
fi
echo ""

# Test 7: Run unit tests
echo "Test 7: Running Ruby adapter unit tests..."
if docker run --rm -v $(pwd):/app -w /app rust:1.83-alpine sh -c \
    "apk add --no-cache musl-dev > /dev/null 2>&1 && cargo test --test test_ruby_integration 2>&1" | \
    grep -q "test result: ok"; then
    echo "✅ All Ruby unit tests pass"
else
    echo "❌ Some Ruby unit tests failed"
    exit 1
fi
echo ""

# Test 8: Verify Docker image tags
echo "Test 8: Checking Docker image tags..."
if docker images | grep -q "debugger-mcp.*ruby"; then
    echo "✅ Docker image is tagged correctly"
    docker images | grep "debugger-mcp" | grep "ruby"
else
    echo "❌ Docker image tag not found"
    exit 1
fi
echo ""

echo "=== All Verification Tests Passed! ==="
echo ""
echo "Ruby debugger is ready for testing with Claude Code."
echo "Follow the guide at: /home/vagrant/projects/fizzbuzz-ruby-test/QUICK_START.md"
echo ""
echo "Expected command structure:"
echo "  With stopOnEntry=true:  rdbg --stop-at-load /workspace/program.rb [args]"
echo "  With stopOnEntry=false: rdbg --nonstop /workspace/program.rb [args]"
echo ""
