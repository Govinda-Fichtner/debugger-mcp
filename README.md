# DAP MCP Server

[![CI](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/ci.yml)
[![Integration Tests](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/integration-tests-matrix.yml/badge.svg)](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/integration-tests-matrix.yml)

Enable AI agents to programmatically debug applications across multiple languages through a unified MCP interface.

---

## What is This?

A Rust-based **MCP (Model Context Protocol) server** that exposes debugging capabilities to AI assistants (Claude, Gemini CLI, etc.) by bridging to the **Debug Adapter Protocol (DAP)**.

**In short:** AI agents can set breakpoints, step through code, inspect variables, and investigate bugs autonomously across Python, Ruby, Node.js, Go, and Rust.

---

## Quick Start

### 1. Install

**Docker (Recommended):**
```bash
# Choose image for your language
docker build -f Dockerfile.python -t debugger-mcp:python .
docker run -i debugger-mcp:python
```

**Native:**
```bash
cargo build --release
./target/release/debugger_mcp serve
```

### 2. Configure Claude Desktop

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

### 3. Debug

Start debugging from Claude!

**Detailed setup:** [Getting Started Guide](docs/Contributing/GETTING_STARTED.md)

---

## Status

🎉 **Production-Ready** - Multi-Language Support

**Supported Languages:** Python, Ruby, Node.js, Go, Rust (all 100% functional)

### Continuous Integration

| Workflow | Purpose | Latest Status |
|----------|---------|---------------|
| **[CI](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/ci.yml)** | Code quality, security, unit tests (193 tests) | ![CI](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/ci.yml/badge.svg) |
| **[Integration Tests](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/integration-tests-matrix.yml)** | End-to-end AI debugging: 5 languages × 2 AI clients (Claude Code + Codex) | ![Integration](https://github.com/Govinda-Fichtner/debugger-mcp/actions/workflows/integration-tests-matrix.yml/badge.svg) |

**What Integration Tests Do:** Real AI agents (Claude Code and Codex) autonomously debug programs via MCP, validating 8 debugging operations per test across all languages.

**Latest Results (10/10 tests passing):**

| Language       | Claude Code | Codex | Operations |
|----------------|-------------|-------|------------|
| Python         | ✅ PASS     | ✅ PASS | SBCTED   |
| Ruby           | ✅ PASS     | ✅ PASS | SBCTED   |
| Node.js        | ✅ PASS     | ✅ PASS | SBCTED   |
| Go             | ✅ PASS     | ✅ PASS | SBCTED   |
| Rust           | ✅ PASS     | ✅ PASS | SBCTED   |

**Legend:** S=Session Start, B=Breakpoint, C=Continue, T=Trace, E=Evaluate, D=Disconnect

**Understanding CI:** See [CI Workflows Documentation](docs/PROCESS_CI_WORKFLOWS.md)

---

## Features

### Supported Languages

| Language | Debugger | Docker Image |
|----------|----------|--------------|
| Python   | debugpy  | `Dockerfile.python` |
| Ruby     | rdbg     | `Dockerfile.ruby` |
| Node.js  | vscode-js-debug | `Dockerfile.nodejs` |
| Go       | delve    | `Dockerfile.go` |
| Rust     | CodeLLDB | `Dockerfile.rust` |

### Debugging Capabilities

✅ **Current:**
- Start/stop debugging sessions
- Set source breakpoints
- Execution control (continue, step over/into/out, pause)
- Expression evaluation
- Stack trace inspection
- Variable inspection

⏳ **Planned:**
- Conditional breakpoints & logpoints
- Exception breakpoints
- Multi-threaded debugging
- Remote debugging
- Data breakpoints

---

## Architecture

```
AI Agent (Claude, Gemini, etc.)
    ↕ MCP Protocol (JSON-RPC)
┌──────────────────────────────────┐
│   DAP MCP Server (Rust/Tokio)   │
│ ┌─────────────────────────────┐  │
│ │  MCP Layer (Tools/Resources)│  │
│ └──────────┬──────────────────┘  │
│ ┌──────────┴──────────────────┐  │
│ │  Language-Agnostic Core     │  │
│ └──────────┬──────────────────┘  │
│ ┌──────────┴──────────────────┐  │
│ │  DAP Protocol Client        │  │
│ └─────────────────────────────┘  │
└──────────┼───────────────────────┘
           ↕ Debug Adapter Protocol
    ┌──────┴──────┐
debugpy  rdbg  delve  CodeLLDB
(Python)(Ruby) (Go)  (Rust/C++)
```

**Deep dive:** [Architecture Proposal](docs/Architecture/DAP_MCP_SERVER_PROPOSAL.md)

---

## Usage Example

```
User: "My Python script crashes. Can you debug it?"

Claude:
  → debugger_start(language="python", program="/workspace/script.py")
  → debugger_set_breakpoint(sourcePath="/workspace/script.py", line=42)
  → debugger_continue()
  → debugger_wait_for_stop()
  [Program stops at breakpoint]
  → stack = debugger_stack_trace()
  → debugger_evaluate(expression="user_data")

  "The crash occurs because 'user_data' is None when fetch_user() fails.
   The code doesn't check for None before accessing user_data.name..."
```

**Expression syntax by language:** [Expression Guide](docs/Usage/EXPRESSION_SYNTAX_GUIDE.md)

---

## Common Issues

**❓ Breakpoint not verified?**
→ Ensure debug symbols: `-g` flag for rustc/gcc, `debugpy` for Python
→ Check source path matches exactly

**❓ Session timeout?**
→ Verify debugger installed: `pip install debugpy`, `gem install debug`, etc.
→ Check debugger in PATH

**❓ Docker path issues?**
→ Use container paths: `/workspace/...` (not host paths like `/home/user/...`)
→ Ensure volume mounted correctly

**Full guide:** [Troubleshooting Documentation](docs/Usage/TROUBLESHOOTING.md)

---

## Documentation

### By Use Case

**🚀 Getting started?**
→ [Getting Started Guide](docs/Contributing/GETTING_STARTED.md)

**🐳 Deploying with Docker?**
→ [Docker Deployment Guide](docs/Usage/DOCKER.md)

**🏗️ Understanding architecture?**
→ [Architecture Proposal](docs/Architecture/DAP_MCP_SERVER_PROPOSAL.md)

**➕ Adding a new language?**
→ [New Language Guide](docs/Contributing/ADDING_NEW_LANGUAGE.md)

**✅ Understanding CI/CD?**
→ [CI Workflows](docs/Processes/CI_WORKFLOWS.md)

**🐛 Troubleshooting issues?**
→ [Troubleshooting Guide](docs/Usage/TROUBLESHOOTING.md)

**🧪 Writing tests?**
→ [Testing Guide](docs/Contributing/TESTING_GUIDE.md)

### Documentation Structure

- **[Architecture/](docs/Architecture/)** - System design, components, technical decisions
- **[Contributing/](docs/Contributing/)** - Developer guides, testing, setup
- **[Usage/](docs/Usage/)** - Deployment, Docker, expressions, troubleshooting
- **[Processes/](docs/Processes/)** - CI/CD, releases, cross-platform builds

**Complete index:** [docs/README.md](docs/README.md)

---

## Development

### Prerequisites

- Rust 1.70+ (`rustup update`)
- Docker (for integration tests)
- Language-specific debuggers (for testing):
  - Python: `pip install debugpy`
  - Ruby: `gem install debug`
  - Node.js: `npm install -g node-debug2`

### Build & Test

```bash
# Clone
git clone https://github.com/Govinda-Fichtner/debugger-mcp.git
cd debugger-mcp

# Install pre-commit hooks (recommended)
pre-commit install --install-hooks
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push

# Build
cargo build --release

# Run unit tests
cargo test

# Run integration tests (requires debuggers)
cargo test --test '*integration*' -- --ignored
```

### Pre-commit Hooks

Automated quality checks run before commit/push:
- Formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Unit tests
- Security scanning (`gitleaks`, `cargo-audit`)
- Code coverage (60% minimum)

**Setup:** [Pre-commit Guide](docs/Contributing/PRE_COMMIT_SETUP.md)

---

## Contributing

We welcome contributions! See [Getting Started](docs/Contributing/GETTING_STARTED.md) for:
- Development setup
- Architecture overview
- Testing guidelines
- Code style

**Contribution workflow:**
1. Fork repository
2. Create feature branch
3. Make changes with tests
4. Run `pre-commit run --all-files`
5. Submit pull request

---

## Roadmap

### ✅ Completed Phases

- **Phase 0:** Research & Architecture
- **Phase 1:** MVP - Python Support
- **Phase 2:** Ruby Validation
- **Phase 3:** Multi-Language Support (Python, Ruby, Node.js, Go, Rust)

### 🚧 Current Phase

**Phase 4: Production Features**
- Conditional breakpoints
- Exception handling
- Security hardening
- Performance optimization

### 📅 Future Phases

**Phase 5: Community**
- Open source release
- Plugin API
- VS Code extension
- Additional languages (Java, C#, PHP)

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Language | Rust | Memory safety, performance, async |
| CLI | Clap | Industry standard, derive macros |
| Async Runtime | Tokio | Battle-tested, comprehensive |
| Serialization | serde + serde_json | De facto standard |
| Error Handling | anyhow + thiserror | Ergonomic, clear messages |
| Logging | tracing | Structured, async-aware |

---

## License

TBD (likely MIT or Apache 2.0)

---

**Built with ❤️ and 🦀 using Rust**

*Last Updated: 2025-10-19*
