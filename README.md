# DAP MCP Server - Debug Adapter Protocol for AI Agents

**Enable AI coding agents to programmatically debug applications across multiple programming languages.**

## What is This?

A Rust-based MCP (Model Context Protocol) server that exposes debugging capabilities to AI assistants like Claude Desktop by bridging to the Debug Adapter Protocol (DAP).

**In short**: AI agents can now set breakpoints, step through code, inspect variables, and investigate bugs autonomously.

## Status

🚧 **Phase: Architecture Complete** 🚧

- ✅ Comprehensive architecture proposal (135+ pages)
- ✅ Technology stack selected (Rust, Tokio, Clap, DAP)
- ✅ MVP implementation plan (Python → Ruby → multi-language)
- ✅ TDD strategy with FizzBuzz integration test
- ⏳ Implementation: Not started

## Quick Links

- **[Main Architecture Proposal](docs/DAP_MCP_SERVER_PROPOSAL.md)** - Complete system design (68 pages)
- **[MVP Implementation Plan](docs/MVP_IMPLEMENTATION_PLAN.md)** - Phase 1 development guide
- **[Getting Started](GETTING_STARTED.md)** - Developer setup and first steps
- **[Documentation Index](docs/README.md)** - All documentation

## Features (Planned)

### Phase 1: MVP (Python Support)
- ✅ Start/stop debugging sessions
- ✅ Set breakpoints (source, conditional, logpoints)
- ✅ Execution control (continue, pause, step over/into/out)
- ✅ Variable inspection
- ✅ Expression evaluation
- ✅ Stack trace inspection

### Phase 2: Multi-Language
- Python (debugpy)
- Ruby (rdbg)
- Node.js (inspector protocol)
- Go (delve)
- Rust (CodeLLDB)
- More via plugin system

### Phase 3: Advanced Features
- Exception breakpoints
- Multi-threaded debugging
- Remote debugging
- Attach to running processes
- Performance optimization

## Architecture

```
AI Agent (Claude Desktop, Gemini CLI, etc.)
    ↕ MCP Protocol (JSON-RPC)
┌─────────────────────────────────────────┐
│     DAP MCP Server (Rust/Tokio)         │
│  ┌────────────────────────────────────┐ │
│  │  MCP Protocol Layer                │ │
│  │  (Resources + Tools)               │ │
│  └──────────────┬─────────────────────┘ │
│  ┌──────────────┴─────────────────────┐ │
│  │  Language-Agnostic Abstraction     │ │
│  └──────────────┬─────────────────────┘ │
│  ┌──────────────┴─────────────────────┐ │
│  │  DAP Protocol Client               │ │
│  └──────────────┬─────────────────────┘ │
└─────────────────┼─────────────────────────┘
                  ↕ Debug Adapter Protocol
        ┌─────────┼──────────┐
   debugpy   node-debug   delve  CodeLLDB
   (Python)  (Node.js)    (Go)   (Rust/C++)
```

## Usage (Future)

### Install

```bash
cargo install debugger_mcp
```

### Run as MCP Server

```bash
debugger_mcp serve
```

### Configure with Claude Desktop

```json
{
  "mcpServers": {
    "debugger": {
      "command": "debugger_mcp",
      "args": ["serve"]
    }
  }
}
```

### Use with AI Agent

```
User: "My Python script crashes. Can you debug it?"

Claude:
  → debugger_start(language="python", program="script.py")
  → debugger_set_exception_breakpoints(filters=["uncaught"])
  → debugger_continue()
  [Program crashes]
  → debugger_evaluate("locals()")
  → Read stack trace
  
  "The crash occurs because 'user_data' is None when fetch_user() fails.
   The code doesn't check for None before accessing user_data.name..."
```

## Technology Stack

| Component | Technology | Why? |
|-----------|-----------|------|
| Language | Rust | Safety, performance, async |
| CLI | Clap | Industry standard, derive macros |
| Async Runtime | Tokio | Comprehensive, battle-tested |
| Serialization | serde | De facto standard |
| Error Handling | anyhow + thiserror | Ergonomic, clear errors |
| Logging | tracing | Structured, async-aware |

## Development

### Prerequisites

- Rust 1.70+ (`rustup update`)
- Python 3.8+ with debugpy (`pip install debugpy`)
- (Optional) Ruby 3.0+ with rdbg (`gem install debug`)

### Quick Start

```bash
# Clone repository
git clone https://github.com/yourusername/debugger_mcp
cd debugger_mcp

# Run tests
cargo test

# Run server
cargo run -- serve

# Run with verbose logging
cargo run -- serve --verbose
```

### Development Workflow

1. Read [Getting Started](GETTING_STARTED.md)
2. Follow [MVP Implementation Plan](docs/MVP_IMPLEMENTATION_PLAN.md)
3. Write tests first (TDD)
4. Implement features
5. Run integration tests

**Key Integration Test**: FizzBuzz debugging scenario
- Tests all core features (breakpoints, stepping, variables)
- Works with Python and Ruby (validates abstraction)
- See `tests/integration/fizzbuzz_test.rs`

## Documentation

### For Decision Makers
- [Executive Summary](docs/DAP_MCP_SERVER_PROPOSAL.md#executive-summary)
- [Use Cases](docs/DAP_MCP_SERVER_PROPOSAL.md#8-use-cases-and-user-journeys)
- [Implementation Timeline](docs/MVP_IMPLEMENTATION_PLAN.md#phased-development-plan)

### For Architects
- [Architecture Overview](docs/DAP_MCP_SERVER_PROPOSAL.md#2-architecture-overview)
- [Component Specifications](docs/architecture/COMPONENTS.md)
- [Multi-Language Abstraction](docs/DAP_MCP_SERVER_PROPOSAL.md#6-multi-language-abstraction-layer)

### For Developers
- [Getting Started Guide](GETTING_STARTED.md)
- [TDD Workflow](docs/MVP_IMPLEMENTATION_PLAN.md#tdd-workflow)
- [Component Details](docs/architecture/COMPONENTS.md)

## Project Structure

```
debugger_mcp/
├── README.md                     # This file
├── GETTING_STARTED.md            # Developer setup
├── SUMMARY.md                    # Project summary
├── docs/                         # Architecture documentation
│   ├── DAP_MCP_SERVER_PROPOSAL.md   # Main proposal (68 pages)
│   ├── MVP_IMPLEMENTATION_PLAN.md   # Phase 1 plan
│   ├── architecture/
│   │   └── COMPONENTS.md            # Component specs
│   └── research/                    # Background research
├── src/                          # Source code (TBD)
├── tests/                        # Tests (TBD)
└── Cargo.toml                    # Dependencies
```

## Roadmap

### ✅ Phase 0: Research & Architecture (Complete)
- Research DAP protocol, MCP, existing implementations
- Design architecture and component specifications
- Create comprehensive documentation
- Define MVP scope and test strategy

### ⏳ Phase 1: MVP - Python Support (Weeks 1-3)
- Implement MCP server with STDIO transport
- Implement DAP client for debugpy
- Core tools: start, stop, breakpoint, continue, evaluate
- Pass FizzBuzz integration test

### 📅 Phase 2: Ruby Validation (Week 4)
- Add Ruby debugger support (rdbg)
- Validate language abstraction works
- Document findings and refactor

### 📅 Phase 3: Multi-Language (Weeks 5-8)
- Node.js, Go, Rust support
- Advanced features (stepping, stack traces)
- Performance optimization

### 📅 Phase 4: Production (Weeks 9-12)
- Conditional breakpoints, logpoints
- Exception handling
- Security hardening
- Comprehensive testing

### 📅 Phase 5: Community (Weeks 13+)
- Open source release
- Plugin API for custom adapters
- VS Code extension
- Community building

## Contributing (Future)

Once implementation begins:

1. Read architecture docs
2. Check GitHub issues
3. Follow TDD workflow
4. Submit PR with tests
5. All contributions welcome!

## License

TBD (likely MIT or Apache 2.0)

## Contact

**Project Status**: Architecture phase complete, implementation starting
**Documentation**: 135+ pages, 40,000+ words
**Timeline**: 20 weeks to v1.0

---

**Built with ❤️ and 🦀 by the community**
