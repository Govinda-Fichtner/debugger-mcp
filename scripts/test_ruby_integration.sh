#!/bin/bash
# Run Ruby integration tests with rdbg installed

set -e

echo "=== Installing dependencies ==="
apk add --no-cache build-base rust cargo musl-dev > /dev/null 2>&1
gem install debug --no-document > /dev/null 2>&1

echo "✅ Ruby and rdbg installed"
echo ""
rdbg --version
echo ""

echo "=== Running integration tests ==="
cargo test --test test_ruby_socket_adapter -- --ignored --test-threads=1
