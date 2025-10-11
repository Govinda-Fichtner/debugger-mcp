# Debugger MCP Server Documentation

Complete documentation for the DAP-based MCP debugging server enabling AI-assisted debugging across multiple programming languages.

---

## 📁 Documentation Structure

### [**Architecture/**](./Architecture/) 🏗️
System design and technical decisions

- **[DAP_MCP_SERVER_PROPOSAL.md](./Architecture/DAP_MCP_SERVER_PROPOSAL.md)** - Complete architecture proposal (68 pages)
  - System design, MCP interface, multi-language abstraction, implementation roadmap
- **[COMPONENTS.md](./Architecture/architecture/COMPONENTS.md)** - Detailed component specifications
  - Module structure, concurrency patterns, testing strategy
- **[LOGGING_ARCHITECTURE.md](./Architecture/LOGGING_ARCHITECTURE.md)** - Logging system design
- **[rust-mcp-technology-stack.md](./Architecture/rust-mcp-technology-stack.md)** - Technology choices and rationale

### [**Contributing/**](./Contributing/) 🤝
Guides for developers and contributors

- **[GETTING_STARTED.md](./Contributing/GETTING_STARTED.md)** - Developer quick start guide
  - Setup, building, development workflow, testing
- **[TESTING.md](./Contributing/TESTING.md)** - Testing guide
- **[TESTING_STRATEGY.md](./Contributing/TESTING_STRATEGY.md)** - Testing approach and philosophy
- **[TESTING_EXAMPLE.md](./Contributing/TESTING_EXAMPLE.md)** - Code examples for tests
- **[PRE_COMMIT_SETUP.md](./Contributing/PRE_COMMIT_SETUP.md)** - Pre-commit hooks setup
- **[INSTALLATION_CHECKLIST.md](./Contributing/INSTALLATION_CHECKLIST.md)** - Tool installation steps
- **[ADDING_NEW_LANGUAGE.md](./Contributing/ADDING_NEW_LANGUAGE.md)** - How to add language support

### [**Usage/**](./Usage/) 📖
User guides and deployment

- **[DOCKER.md](./Usage/DOCKER.md)** - Docker deployment guide
  - Container setup, MCP client integration, production deployment
- **[TROUBLESHOOTING.md](./Usage/TROUBLESHOOTING.md)** - Common issues and solutions
- **[EXPRESSION_SYNTAX_GUIDE.md](./Usage/EXPRESSION_SYNTAX_GUIDE.md)** - Language-specific expression syntax
- **[INTEGRATION_TESTS.md](./Usage/INTEGRATION_TESTS.md)** - Integration test specifications

### [**Processes/**](./Processes/) ⚙️
Development and release processes

- **[CI_CD_PIPELINE.md](./Processes/CI_CD_PIPELINE.md)** - CI/CD configuration and workflows
- **[CROSS_PLATFORM_BUILDS.md](./Processes/CROSS_PLATFORM_BUILDS.md)** - Building for multiple platforms
- **[RELEASE_PROCESS.md](./Processes/RELEASE_PROCESS.md)** - How to create releases
- **[LOG_VALIDATION_SYSTEM.md](./Processes/LOG_VALIDATION_SYSTEM.md)** - Log validation system

---

## 🚀 Quick Navigation

### I want to...

**Understand the architecture**
1. Read the [Architecture Proposal](./Architecture/DAP_MCP_SERVER_PROPOSAL.md) (Executive Summary + Architecture sections)
2. Review [Component Specifications](./Architecture/architecture/COMPONENTS.md)
3. Check [Technology Stack](./Architecture/rust-mcp-technology-stack.md) rationale

**Contribute to the codebase**
1. Start with [Getting Started Guide](./Contributing/GETTING_STARTED.md)
2. Set up [Pre-commit Hooks](./Contributing/PRE_COMMIT_SETUP.md)
3. Follow [Testing Strategy](./Contributing/TESTING_STRATEGY.md)
4. Reference [Testing Examples](./Contributing/TESTING_EXAMPLE.md)

**Deploy or use the server**
1. Follow [Docker Deployment](./Usage/DOCKER.md) guide
2. Refer to [Troubleshooting](./Usage/TROUBLESHOOTING.md) if issues arise
3. Use [Expression Syntax Guide](./Usage/EXPRESSION_SYNTAX_GUIDE.md) for language-specific queries

**Add a new programming language**
1. Read [Adding New Language Guide](./Contributing/ADDING_NEW_LANGUAGE.md)
2. Review [Architecture Proposal](./Architecture/DAP_MCP_SERVER_PROPOSAL.md) Section 6 (Multi-Language Abstraction)

**Work on CI/CD or releases**
1. Understand [CI/CD Pipeline](./Processes/CI_CD_PIPELINE.md)
2. Follow [Release Process](./Processes/RELEASE_PROCESS.md)
3. Check [Cross-Platform Builds](./Processes/CROSS_PLATFORM_BUILDS.md)

---

## 🎯 Key Concepts

### What is This Project?

A **Debug Adapter Protocol (DAP) based Model Context Protocol (MCP) server** that enables AI coding agents (Claude, Gemini CLI, etc.) to programmatically debug applications across multiple programming languages through a unified interface.

**Key Features:**
- 🌍 **Language-agnostic**: Supports Python, Ruby, JavaScript/Node.js, Go, Rust, C/C++ (via 40+ DAP implementations)
- 🤖 **AI-native**: Native MCP protocol for seamless AI agent integration
- 🔧 **Production-ready**: Rust + Tokio for reliability and performance
- 🔌 **Extensible**: Plugin system for new debuggers without core changes

### Why This Matters

- **Autonomous debugging**: AI can investigate bugs independently
- **Reduced debugging time**: 40-50% of dev time is debugging
- **Enhanced AI workflows**: AI explains code by stepping through execution
- **Standard interface**: One API for all debuggers

### Core Technologies

- **[DAP](https://microsoft.github.io/debug-adapter-protocol/)** - Microsoft's language-agnostic debugging standard
- **[MCP](https://spec.modelcontextprotocol.io/)** - Anthropic's protocol for AI agent capabilities
- **Rust + Tokio** - Safe, performant async implementation

---

## 📊 Project Status

**Current Phase**: Production-Ready ✅

- ✅ Multi-language support (Python, Ruby, Node.js, Go, Rust)
- ✅ 13 MCP tools fully functional
- ✅ 100+ comprehensive tests
- ✅ Docker deployment support
- ✅ Complete documentation

**Supported Languages:**
| Language | Status | Test Coverage |
|----------|--------|---------------|
| Python | ✅ Production | 100% |
| Ruby | ✅ Production | 100% |
| Node.js | ✅ Production | 100% |
| Go | ✅ Production | 100% |
| Rust | ✅ Production | 100% |

---

## 🔗 External Resources

### Specifications
- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io/)
- [DAP GitHub Repository](https://github.com/microsoft/debug-adapter-protocol)

### Debug Adapters
- [debugpy](https://github.com/microsoft/debugpy) - Python
- [rdbg](https://github.com/ruby/debug) - Ruby
- [vscode-js-debug](https://github.com/microsoft/vscode-js-debug) - JavaScript/Node.js
- [delve](https://github.com/go-delve/delve) - Go
- [CodeLLDB](https://github.com/vadimcn/codelldb) - Rust/C/C++

### Technologies
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [Clap](https://docs.rs/clap/) - CLI framework
- [serde](https://serde.rs/) - Serialization

---

## 📝 Historical Documentation

Historical implementation notes, proposals, research, and completed work have been archived in Obsidian for reference:
- Location: `/Development Projects/Debugger-MCP/Documentation/`
- Includes: Status reports, bug fixes, proposals, research, deep-dives

This keeps the repository focused on current, actionable documentation while preserving the complete project history.

---

**Last Updated**: 2025-10-10
**Documentation Version**: 2.0 (Reorganized Structure)
