# Changelog

## [Unreleased] - 2025-10-07

### Documentation - Ruby Validation and Language Addition Guide

#### Comprehensive Documentation Update
- **Status**: ✅ Ruby debugging fully validated with 100% success rate
- **New Guides**:
  - `docs/ADDING_NEW_LANGUAGE.md` - Complete guide for adding language support (150+ lines)
  - `docs/IMPLEMENTATION_STATUS_OCT_2025.md` - Current implementation status with both Python and Ruby validated
- **Updated Files**:
  - `README.md` - Updated status to "Production-Ready", Phase 2 marked complete, added language addition guide link
  - Supported Languages table enhanced with validation status
- **Success Validation**:
  - End-to-end testing with Claude: 100% success rate
  - All 13 MCP tools working across Python and Ruby
  - Bug identification successful (fizzbuzz n % 4 → n % 5)
- **Key Learnings Documented**:
  - DAP specification compliance critical
  - stopOnEntry not universal (entry breakpoint pattern works for all)
  - Transport mechanisms vary (STDIO vs TCP)
  - Language-specific parsing needed for first executable line detection
  - Adapter bugs exist (workarounds documented)
- **References**:
  - Success report: `/home/vagrant/projects/fizzbuzz-ruby-test/SUCCESS_REPORT.md`
  - GitHub Issue #1: Proper DAP sequence for all languages
  - Language addition guide: `docs/ADDING_NEW_LANGUAGE.md`

### Fixed - Ruby stopOnEntry Issue

#### Ruby Debugging: stopOnEntry Now Works Correctly (Entry Breakpoint Solution)
- **Issue**: Ruby debugger (rdbg) in socket mode didn't honor `--stop-at-load` flag
- **Symptom**: Programs ran to completion without stopping at entry point, making debugging impossible for fast-executing scripts
- **Root Cause**: Implementation was violating DAP specification by setting breakpoints AFTER configurationDone instead of BEFORE
- **Solution**: Implemented entry breakpoint pattern - set breakpoint at first executable line BEFORE configurationDone (per DAP spec)
- **Impact**: Ruby debugging now works correctly following DAP specification; solution is language-agnostic
- **Performance**: +50-150ms startup time for Ruby with stopOnEntry (file read + line detection)
- **Files Changed**:
  - `src/dap/client.rs`: Added `find_first_executable_line_ruby()`, modified `initialize_and_launch()` to set entry breakpoint before configurationDone
  - `src/debug/session.rs`: Pass adapter type for language-specific workarounds
  - `tests/test_event_driven.rs`: Updated test call site
- **Tests Added**:
  - `tests/test_ruby_stopentry_issue.rs` (380 lines): Failing test demonstrating bug + passing test proving fix
  - Verification script: `scripts/verify_stopentry_issue.sh`
- **Documentation**:
  - `docs/RDBG_ANALYSIS_AND_SOLUTION.md` - Root cause analysis and correct DAP sequence
  - `docs/RUBY_STOPENTRY_FIX.md` - Implementation plan
  - `docs/RUBY_STOPENTRY_FIX_IMPLEMENTATION.md` - Detailed walkthrough
  - `RUBY_STOPENTRY_FIX_COMPLETE.md` - Executive summary
  - `/home/vagrant/projects/fizzbuzz-ruby-test/RDBG_BUG_REPORT.md` - Original bug report
- **References**:
  - DAP Specification: https://microsoft.github.io/debug-adapter-protocol/specification
  - Correct sequence: initialize → initialized → setBreakpoints → configurationDone

### Changed
- `DapClient::initialize_and_launch()` now accepts optional `adapter_type` parameter for language-specific workarounds
- `DapClient::initialize_and_launch_with_timeout()` signature updated to pass adapter type
- Ruby debugging now follows correct DAP sequence (breakpoints before configurationDone)
- Startup time increased by ~50-150ms for Ruby with stopOnEntry (file parsing overhead)

### Added
- `DapClient::find_first_executable_line_ruby()` helper for entry point detection
- Entry breakpoint pattern for stopOnEntry (DAP spec compliant)
- Language-specific debugging workarounds framework (extensible to other languages)
- Comprehensive test suite for Ruby stopOnEntry issue (TDD approach)

---

## [Previous] - 2025-10-05

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
