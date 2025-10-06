# Changelog

## [Unreleased] - 2025-10-05

### Fixed - Critical MCP Protocol and Docker Build Issues

#### Issue 1: MCP Protocol Violation (CRITICAL)
- **Fixed**: Server was using LSP transport (Content-Length headers) instead of MCP line-based JSON-RPC
- **Impact**: Server was completely non-functional with all MCP clients
- **Solution**: Rewrote `src/mcp/transport.rs` to use proper MCP stdio transport
- **Added**: 4 regression tests to prevent future protocol violations

#### Issue 2: Docker Build - Cargo.lock
- **Fixed**: Cargo.lock was in .dockerignore but referenced in Dockerfile
- **Solution**: Removed Cargo.lock from .dockerignore for reproducible builds

#### Issue 3: Rust Edition Compatibility  
- **Fixed**: Edition 2024 requires nightly Rust but Dockerfile used stable
- **Solution**: Changed to stable edition 2021 in Cargo.toml

#### Issue 4: Alpine Linux PEP 668
- **Fixed**: pip install failed on Alpine 3.21 due to PEP 668
- **Solution**: Added --break-system-packages flag to pip install

#### Issue 5: ARM64 Architecture Support
- **Fixed**: Hardcoded x86_64 target prevented ARM64 builds
- **Solution**: Build for native architecture (supports both x86_64 and aarch64)

### Added
- MCP protocol regression tests (4 new tests)
- Documentation: docs/FIXES_2025_10_05.md

### Changed
- Test count: 79 → 114 tests (+35 tests)
- Coverage: 61.90% → 67.29% (+5.39% improvement)
- Multi-architecture Docker support (x86_64 + ARM64)

### Test Coverage Improvements (Phases 5-6)
- **Phase 5**: Added 6 MCP transport implementation tests using in-memory pipes
- **Phase 6**: Added 19 error path tests for MCP tools (invalid arguments, missing fields)
- **Phase 6B**: Added 4 protocol error path tests (handler initialization, error responses)
- **Result**: mcp/protocol.rs achieved 100% coverage (83/83 lines)

### References
- MCP Specification: https://spec.modelcontextprotocol.io/
- All 114 tests passing ✅
- Coverage report: coverage/tarpaulin-report.html

---

## [0.1.0] - Previous

### Added - Test Coverage Improvement
- Increased coverage from 3% to 61.90% (20.6x improvement)
- Added 79 comprehensive unit tests
- Implemented trait-based dependency injection
- Created Mockall-based testing infrastructure
- Added fake DAP adapter for integration testing

### Documentation
- docs/COVERAGE_PROGRESS.md - Phase-by-phase progress tracking
- docs/TESTING_STRATEGY.md - Complete testing roadmap
- docs/TESTING_EXAMPLE.md - Code examples
- docs/PHASE_4_NOTES.md - Integration testing notes
- docs/FINAL_SUMMARY.md - Complete project summary

### Infrastructure
- Tarpaulin configuration for coverage tracking
- DapTransportTrait and McpTransportTrait abstractions
- Mock-based testing patterns
- HTML and XML coverage reports
