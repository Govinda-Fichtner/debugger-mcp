
# Rust-specific targets
.PHONY: test-rust-docker
test-rust-docker:
	@echo "Running Rust integration tests in Docker..."
	@./scripts/test-rust-docker.sh

.PHONY: test-rust-unit
test-rust-unit:
	@echo "Running Rust unit tests (non-Docker)..."
	@cargo test --test test_rust_integration --lib

.PHONY: test-rust-all
test-rust-all: test-rust-unit test-rust-docker
	@echo "All Rust tests complete!"
