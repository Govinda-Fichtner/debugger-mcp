#!/bin/bash
set -e

echo "🧪 Running Rust integration tests..."
echo ""

# Build the Docker image if needed
echo "📦 Building Docker image..."
docker build -f Dockerfile.rust -t mcp-debugger-rust:latest . > /dev/null 2>&1
echo "✅ Docker image built"
echo ""

# Note: These tests need to be run with actual MCP server interaction
# For now, run the unit tests that don't require a running debugger
echo "🚀 Running Rust unit tests (non-Docker tests)..."
echo ""

cargo test --test test_rust_integration \
  test_rust_adapter_metadata \
  test_rust_adapter_command \
  test_rust_adapter_id \
  test_rust_adapter_args \
  test_rust_launch_args_structure \
  test_rust_launch_args_no_cwd \
  test_rust_launch_args_no_stop_on_entry \
  test_rust_compilation_error \
  -- --nocapture

echo ""
echo "✅ Unit tests passed!"
echo ""
echo "ℹ️  Integration tests (test_rust_stack_trace_uses_correct_thread_id, etc.)"
echo "   require a running MCP server and should be tested manually with Claude Code."
echo ""
echo "🎉 Test run complete!"
