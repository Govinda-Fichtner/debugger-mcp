# DAP MCP Server - Debug Adapter Protocol for AI Agents

**Enable AI coding agents to programmatically debug applications across multiple programming languages.**

## What is This?

A Rust-based MCP (Model Context Protocol) server that exposes debugging capabilities to AI assistants like Claude Desktop by bridging to the Debug Adapter Protocol (DAP).

**In short**: AI agents can now set breakpoints, step through code, inspect variables, and investigate bugs autonomously.

## Status

🎉 **Phase: Multi-Language Support - Validated and Production-Ready** 🎉

- ✅ Comprehensive architecture proposal (135+ pages)
- ✅ Technology stack selected (Rust, Tokio, Clap, DAP)
- ✅ MVP implementation plan (Python → Ruby → multi-language)
- ✅ TDD strategy with FizzBuzz integration test
- ✅ MCP server with STDIO transport (~400 LOC)
- ✅ Complete DAP client with async correlation (~270 LOC)
- ✅ Debug session management (~400 LOC)
- ✅ 13 MCP tools implemented
- ✅ **Python support** via debugpy - Fully validated
- ✅ **Ruby support** via rdbg (debug gem) - Fully validated with entry breakpoint solution
- ✅ Comprehensive integration tests (Python + Ruby) - All passing
- ✅ Language-specific Docker images (Python, Ruby)
- ✅ **End-to-end validation with Claude** - 100% success rate

## Quick Links

- **[Docker Deployment Guide](docs/DOCKER.md)** - Running with Docker (recommended)
- **[Getting Started](docs/GETTING_STARTED.md)** - Developer setup and first steps
- **[Adding New Languages](docs/ADDING_NEW_LANGUAGE.md)** - Guide for adding language support
- **[Main Architecture Proposal](docs/DAP_MCP_SERVER_PROPOSAL.md)** - Complete system design (68 pages)
- **[MVP Implementation Plan](docs/MVP_IMPLEMENTATION_PLAN.md)** - Phase 1 development guide
- **[MVP Implementation Status](docs/MVP_IMPLEMENTATION_STATUS.md)** - Current implementation status
- **[Documentation Index](docs/README.md)** - All documentation

## Features

### Supported Languages ✅

| Language | Debugger | Status | Notes | Docker Image |
|----------|----------|--------|-------|--------------|
| **Python** | debugpy | ✅ **Validated** | Native stopOnEntry support | `Dockerfile.python` |
| **Ruby** | rdbg (debug gem) | ✅ **Validated** | Entry breakpoint solution | `Dockerfile.ruby` |
| Node.js | inspector protocol | ⏳ Planned | Built-in debugger | - |
| Go | delve | ⏳ Planned | Popular Go debugger | - |
| Rust | CodeLLDB | ⏳ Planned | LLDB-based debugging | - |

### Implemented Features ✅
- ✅ Start/stop debugging sessions (`debugger_start`, `debugger_disconnect`)
- ✅ Set source breakpoints (`debugger_set_breakpoint`, `debugger_list_breakpoints`)
- ✅ Execution control (`debugger_continue`, `debugger_wait_for_stop`)
- ✅ Expression evaluation (`debugger_evaluate`)
- ✅ Stack trace inspection (`debugger_stack_trace`)
- ✅ Step commands (`debugger_step_over`, `debugger_step_into`, `debugger_step_out`)
- ✅ Session state queries (`debugger_session_state`)

### Planned Features
- ⏳ Conditional breakpoints, logpoints
- ⏳ Exception breakpoints

### Future Enhancements
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
   debugpy      rdbg    node-debug   delve  CodeLLDB
   (Python)   (Ruby)    (Node.js)    (Go)   (Rust/C++)
```

## Usage

### Option 1: Docker (Recommended)

Choose the Docker image based on your project's language:

```bash
# For Python projects (~120 MB)
docker build -f Dockerfile.python -t debugger-mcp:python .
docker run -i debugger-mcp:python

# For Ruby projects (~100 MB)
docker build -f Dockerfile.ruby -t debugger-mcp:ruby .
docker run -i debugger-mcp:ruby
```

**Configure with Claude Desktop:**

```json
{
  "mccpServers": {
    "debugger": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-v", "${workspaceFolder}:/workspace",
        "debugger-mcp:latest"
      ]
    }
  }
}
```

See [Docker Documentation](docs/DOCKER.md) for details.

### Option 2: Native Install

```bash
# Build from source
cargo build --release

# Run as MCP Server
./target/release/debugger_mcp serve
```

**Configure with Claude Desktop:**

```json
{
  "mcpServers": {
    "debugger": {
      "command": "/path/to/debugger_mcp",
      "args": ["serve"]
    }
  }
}
```

### Use with AI Agent

**Python Example:**
```
User: "My Python script crashes. Can you debug it?"

Claude:
  → debugger_start(language="python", program="script.py", stopOnEntry=true)
  → debugger_set_breakpoint(sourcePath="script.py", line=42)
  → debugger_continue()
  → debugger_wait_for_stop()
  [Program stops at breakpoint]
  → stack = debugger_stack_trace()
  → debugger_evaluate(expression="user_data", frameId=stack.stackFrames[0].id)

  "The crash occurs because 'user_data' is None when fetch_user() fails.
   The code doesn't check for None before accessing user_data.name..."
```

**Ruby Example:**
```
User: "My Ruby script has a bug in the fizzbuzz function. Can you debug it?"

Claude:
  → debugger_start(language="ruby", program="fizzbuzz.rb", stopOnEntry=true)
  → debugger_set_breakpoint(sourcePath="fizzbuzz.rb", line=9)
  → debugger_continue()
  → debugger_wait_for_stop()
  [Program stops at breakpoint]
  → stack = debugger_stack_trace()
  → debugger_evaluate(expression="n", frameId=stack.stackFrames[0].id)

  "The bug is on line 9: it checks 'n % 4' instead of 'n % 5' for Buzz.
   This causes incorrect output for numbers divisible by 5..."
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

### ✅ Phase 1: MVP - Python Support (COMPLETE)
- ✅ Implement MCP server with STDIO transport
- ✅ Implement DAP client for debugpy
- ✅ Core tools: start, stop, breakpoint, continue, evaluate, stack_trace
- ✅ Session manager with concurrent access
- ✅ Pass FizzBuzz integration test
- ✅ End-to-end validation with Claude

### ✅ Phase 2: Ruby Validation (COMPLETE)
- ✅ Add Ruby debugger support (rdbg)
- ✅ Validate language abstraction works
- ✅ Implement entry breakpoint solution for stopOnEntry
- ✅ Document findings and create language addition guide
- ✅ End-to-end validation with Claude (100% success)

### 📅 Phase 3: Multi-Language (Weeks 5-8)
- Node.js support (inspector protocol)
- Go support (delve)
- Rust support (CodeLLDB)
- Advanced features refinement
- Performance optimization

### 📅 Phase 4: Production (Weeks 9-12)
- Conditional breakpoints, logpoints
- Exception handling
- Security hardening
- Comprehensive testing
- Apply DAP sequence fix to all languages (Issue #1)

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
