# DAP MCP Server - Debug Adapter Protocol for AI Agents

**Enable AI coding agents to programmatically debug applications across multiple programming languages.**

## What is This?

A Rust-based MCP (Model Context Protocol) server that exposes debugging capabilities to AI assistants like Claude Desktop by bridging to the Debug Adapter Protocol (DAP).

**In short**: AI agents can now set breakpoints, step through code, inspect variables, and investigate bugs autonomously.

## Status

🎉 **Phase: Multi-Language Support - Production-Ready** 🎉

- ✅ Comprehensive architecture (see [docs/](docs/))
- ✅ MCP server with STDIO transport
- ✅ Complete DAP client with async correlation
- ✅ Debug session management
- ✅ 13 MCP tools implemented
- ✅ **5 languages fully validated**: Python, Ruby, Node.js, Go, Rust
- ✅ Comprehensive integration tests - All passing
- ✅ Language-specific Docker images
- ✅ **End-to-end validation with Claude** - 100% success rate

## Quick Links

- **[Documentation Hub](docs/README.md)** - Complete documentation index
- **[Getting Started](docs/Contributing/GETTING_STARTED.md)** - Developer setup
- **[Docker Deployment](docs/Usage/DOCKER.md)** - Deployment guide (recommended)
- **[Troubleshooting](docs/Usage/TROUBLESHOOTING.md)** - Common issues
- **[Architecture Proposal](docs/Architecture/DAP_MCP_SERVER_PROPOSAL.md)** - Complete system design

## Features

### Supported Languages ✅

| Language | Debugger | Status | Docker Image |
|----------|----------|--------|--------------|
| **Python** | debugpy | ✅ Production | `Dockerfile.python` |
| **Ruby** | rdbg (debug gem) | ✅ Production | `Dockerfile.ruby` |
| **Node.js** | vscode-js-debug | ✅ Production | `Dockerfile.nodejs` |
| **Rust** | CodeLLDB | ✅ Production | `Dockerfile.rust` |
| **Go** | delve | ✅ Production | - |

### Implemented Features ✅
- ✅ Start/stop debugging sessions
- ✅ Set source breakpoints
- ✅ Execution control (continue, step, wait)
- ✅ Expression evaluation
- ✅ Stack trace inspection
- ✅ Session state queries

### Planned Features
- ⏳ Conditional breakpoints, logpoints
- ⏳ Exception breakpoints
- ⏳ Multi-threaded debugging
- ⏳ Remote debugging

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
# For Python projects
docker build -f Dockerfile.python -t debugger-mcp:python .
docker run -i debugger-mcp:python

# For Ruby projects
docker build -f Dockerfile.ruby -t debugger-mcp:ruby .
docker run -i debugger-mcp:ruby

# For Node.js projects
docker build -f Dockerfile.nodejs -t debugger-mcp:nodejs .
docker run -i debugger-mcp:nodejs

# For Rust projects
docker build -f Dockerfile.rust -t debugger-mcp:rust .
docker run -i debugger-mcp:rust
```

**Configure with Claude Desktop:**

```json
{
  "mcpServers": {
    "debugger": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-v", "/home/user/projects:/workspace",
        "debugger-mcp:nodejs"
      ]
    }
  }
}
```

**🚨 Important**: When debugging with Docker, use `/workspace/...` paths (container) not `/home/user/projects/...` paths (host)!

See [Docker Documentation](docs/Usage/DOCKER.md) for complete deployment guide.

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

### Example: AI-Assisted Debugging

**Python Example:**
```
User: "My Python script crashes. Can you debug it?"

Claude:
  → debugger_start(language="python", program="/workspace/script.py", stopOnEntry=true)
  → debugger_set_breakpoint(sourcePath="/workspace/script.py", line=42)
  → debugger_continue()
  → debugger_wait_for_stop()
  [Program stops at breakpoint]
  → stack = debugger_stack_trace()
  → debugger_evaluate(expression="user_data", frameId=stack.stackFrames[0].id)

  "The crash occurs because 'user_data' is None when fetch_user() fails.
   The code doesn't check for None before accessing user_data.name..."
```

See [Expression Syntax Guide](docs/Usage/EXPRESSION_SYNTAX_GUIDE.md) for language-specific evaluation syntax.

## Technology Stack

| Component | Technology | Why? |
|-----------|-----------|------|
| Language | Rust | Safety, performance, async |
| CLI | Clap | Industry standard |
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

# Install pre-commit hooks (recommended)
pre-commit install --install-hooks
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push

# Run tests
cargo test

# Run server
cargo run -- serve
```

### Pre-commit Hooks

The project uses automated git hooks for code quality:

- **Rust linting** (clippy, rustfmt)
- **Security scanning** (gitleaks, cargo-audit, cargo-deny)
- **Test execution** (unit tests on commit, all tests on push)
- **Code coverage** (60% minimum threshold)
- **Commit message validation** (Conventional Commits)

See [Pre-commit Setup Guide](docs/Contributing/PRE_COMMIT_SETUP.md) for installation instructions.

## Documentation

### Quick Navigation

**Want to understand the architecture?**
→ Start with [Architecture Proposal](docs/Architecture/DAP_MCP_SERVER_PROPOSAL.md)

**Want to contribute?**
→ Start with [Getting Started Guide](docs/Contributing/GETTING_STARTED.md)

**Want to deploy?**
→ Start with [Docker Deployment](docs/Usage/DOCKER.md)

**Want to add a new language?**
→ See [Adding New Language Guide](docs/Contributing/ADDING_NEW_LANGUAGE.md)

**Full documentation index:**
→ See [docs/README.md](docs/README.md)

### Documentation Structure

- **[Architecture/](docs/Architecture/)** - System design and technical decisions
- **[Contributing/](docs/Contributing/)** - Developer guides and setup
- **[Usage/](docs/Usage/)** - Deployment and user guides
- **[Processes/](docs/Processes/)** - Development and release processes

## Roadmap

### ✅ Phase 0: Research & Architecture (Complete)
Research, design, comprehensive documentation

### ✅ Phase 1: MVP - Python Support (Complete)
MCP server, DAP client, core tools, integration tests

### ✅ Phase 2: Ruby Validation (Complete)
Ruby support, language abstraction validation, entry breakpoint solution

### ✅ Phase 3: Multi-Language Support (Complete)
Node.js, Rust, Go support, Docker images, production-ready

### 📅 Phase 4: Production Features (In Progress)
Conditional breakpoints, exception handling, security hardening

### 📅 Phase 5: Community (Future)
Open source release, plugin API, VS Code extension

## Historical Documentation

Historical implementation notes, proposals, research, and completed work have been archived for reference:
- **Location**: Personal Obsidian vault at `/Development Projects/Debugger-MCP/Documentation/`
- **Contents**: Status reports, bug fixes, postmortems, proposals, research, deep-dives
- **Purpose**: Preserve complete project history while keeping repository focused on current documentation

This keeps the repository clean and focused while maintaining full historical context for future reference.

## Contributing (Future)

Once open source:
1. Read [Getting Started](docs/Contributing/GETTING_STARTED.md)
2. Review [Architecture](docs/Architecture/DAP_MCP_SERVER_PROPOSAL.md)
3. Check GitHub issues
4. Follow [Testing Strategy](docs/Contributing/TESTING_STRATEGY.md)
5. Submit PR with tests

## License

TBD (likely MIT or Apache 2.0)

---

**Built with ❤️ and 🦀 using Rust**

*Last Updated: 2025-10-10*
