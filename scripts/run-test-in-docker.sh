#!/bin/bash
set -e

# Verify test fixture
echo "=== Verifying test fixture ==="
ls -la /workspace/fizzbuzz-rust-test/fizzbuzz.rs
echo ""

# Compile test fixture to verify it works
echo "=== Compiling test fixture ==="
rustc -g /workspace/fizzbuzz-rust-test/fizzbuzz.rs -o /tmp/fizzbuzz_test
echo "✅ Test file compiles"
echo ""

# Run the binary briefly
echo "=== Running binary ==="
timeout 1 /tmp/fizzbuzz_test | head -5 || true
echo "✅ Binary runs"
echo ""

# Run integration tests one at a time with backtrace
echo "=== Running integration tests ==="
export RUST_BACKTRACE=1

# Run tests one at a time to see which ones pass/fail
echo ""
echo "--- Test 1: Compilation test ---"
cargo test --test test_rust_integration test_rust_compilation_single_file -- --ignored --nocapture --test-threads=1 && echo "✅ PASSED" || echo "❌ FAILED"

echo ""
echo "--- Test 2: Stack trace regression test ---"
cargo test --test test_rust_integration test_rust_stack_trace_uses_correct_thread_id -- --ignored --nocapture --test-threads=1 && echo "✅ PASSED" || echo "❌ FAILED"

echo ""
echo "--- Test 3: Evaluate regression test ---"
cargo test --test test_rust_integration test_rust_evaluate_uses_watch_context -- --ignored --nocapture --test-threads=1 && echo "✅ PASSED" || echo "❌ FAILED"
