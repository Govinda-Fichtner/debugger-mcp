#!/bin/bash
# Verify Ruby stopOnEntry Issue - Test Should FAIL
#
# This script runs the test that demonstrates the Ruby stopOnEntry bug.
# Expected: Test FAILS because rdbg doesn't send 'stopped' event
# After implementing the fix, this test should PASS.

set -e

echo "═══════════════════════════════════════════════════════════"
echo "  RUBY STOPENTRY ISSUE VERIFICATION"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Running test that demonstrates the bug..."
echo "Expected: TEST WILL FAIL (this proves the bug exists)"
echo ""

# Check if we're in Docker or need to use Docker
if command -v cargo &> /dev/null && command -v rdbg &> /dev/null; then
    echo "✅ cargo and rdbg found - running natively"
    echo ""

    # Run the test (will fail)
    cargo test --test test_ruby_stopentry_issue test_ruby_stopentry_issue_demonstration -- --ignored --nocapture || {
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "  ✅ TEST FAILED AS EXPECTED!"
        echo "═══════════════════════════════════════════════════════════"
        echo ""
        echo "This FAILURE proves the bug exists:"
        echo "  • rdbg was spawned with --stop-at-load"
        echo "  • Launch request had stopOnEntry: true"
        echo "  • But NO 'stopped' event was received"
        echo "  • Program ran to completion"
        echo ""
        echo "Next step: Implement the pause workaround fix"
        echo ""
        exit 0
    }

    echo ""
    echo "⚠️  WARNING: Test PASSED unexpectedly!"
    echo "This might mean:"
    echo "  1. rdbg fixed the issue in a newer version"
    echo "  2. The test needs to be updated"
    echo "  3. The workaround is already implemented"
    exit 1

else
    echo "⚠️  cargo or rdbg not found - trying Docker"
    echo ""

    if ! command -v docker &> /dev/null; then
        echo "❌ Docker not found either"
        echo "Please install:"
        echo "  • Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  • rdbg: gem install debug"
        echo "Or use Docker: apt-get install docker.io"
        exit 1
    fi

    echo "Building Ruby Docker image..."
    docker build -f Dockerfile.ruby -t debugger-mcp:ruby . -q

    echo "Running test in Docker..."
    docker run --rm -v "$(pwd):/app" -w /app debugger-mcp:ruby \
        cargo test --test test_ruby_stopentry_issue test_ruby_stopentry_issue_demonstration -- --ignored --nocapture || {
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "  ✅ TEST FAILED AS EXPECTED!"
        echo "═══════════════════════════════════════════════════════════"
        echo ""
        echo "This FAILURE proves the bug exists"
        echo "Next step: Implement the pause workaround fix"
        echo ""
        exit 0
    }

    echo ""
    echo "⚠️  WARNING: Test PASSED unexpectedly!"
    exit 1
fi
