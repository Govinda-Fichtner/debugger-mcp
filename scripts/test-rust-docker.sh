#!/bin/bash
set -e

echo "🧪 Running Rust integration tests..."
echo ""

# Parse command line arguments
RUN_ALL=false
if [ "$1" == "--all" ]; then
    RUN_ALL=true
fi

# Build the Docker image if needed
echo "📦 Building Docker image..."
docker build -f Dockerfile.rust -t mcp-debugger-rust:latest . > /dev/null 2>&1
echo "✅ Docker image built"
echo ""

# Run unit tests in Docker (don't require running debugger)
echo "🚀 Running Rust unit tests in Docker..."
echo ""

docker run --rm \
  -v "$(pwd):/workspace" \
  -w /workspace \
  mcp-debugger-rust:latest \
  cargo test --test test_rust_integration \
    test_rust_adapter_metadata \
    test_rust_adapter_command \
    test_rust_adapter_id \
    test_rust_adapter_args \
    test_rust_launch_args_structure \
    test_rust_launch_args_no_cwd \
    test_rust_launch_args_no_stop_on_entry \
    test_rust_compilation_error \
    -- --nocapture 2>&1 | tail -30

echo ""
echo "✅ Unit tests passed!"
echo ""

# Optionally run regression tests
if [ "$RUN_ALL" == "true" ]; then
    echo "🧪 Running regression tests in Docker..."
    echo ""

    docker run --rm \
      -v "$(pwd):/workspace" \
      -v "$(pwd)/tests/fixtures:/workspace/fizzbuzz-rust-test" \
      -w /workspace \
      mcp-debugger-rust:latest \
      bash -c "timeout 120 cargo test --test test_rust_integration -- --ignored --nocapture --test-threads=1 test_rust_stack_trace test_rust_evaluate" || {
        echo "❌ Regression tests failed"
        exit 1
      }

    echo ""
    echo "✅ All tests (unit + regression) passed!"
else
    echo "ℹ️  To run regression tests, use: $0 --all"
    echo "   Regression tests run in Docker with CodeLLDB and verify:"
    echo "   - test_rust_stack_trace_uses_correct_thread_id"
    echo "   - test_rust_evaluate_uses_watch_context"
fi

echo ""
echo "🎉 Test run complete!"
