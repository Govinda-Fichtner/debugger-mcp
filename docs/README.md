# Debugger MCP Server Documentation

Complete documentation for the DAP-based MCP debugging server enabling AI-assisted debugging across multiple programming languages.

---

## 📁 Documentation Structure

All documentation files are organized in a flat structure with category prefixes for easy navigation:

### Architecture Documentation (`ARCHITECTURE_*.md`)

System design and technical decisions:

- **[ARCHITECTURE_PROPOSAL.md](./ARCHITECTURE_PROPOSAL.md)** - Complete architecture proposal (68 pages)
  - System design, MCP interface, multi-language abstraction, implementation roadmap
- **[ARCHITECTURE_COMPONENTS.md](./ARCHITECTURE_COMPONENTS.md)** - Detailed component specifications
  - Module structure, concurrency patterns, testing strategy
- **[ARCHITECTURE_LOGGING.md](./ARCHITECTURE_LOGGING.md)** - Logging system design
- **[ARCHITECTURE_TECHNOLOGY_STACK.md](./ARCHITECTURE_TECHNOLOGY_STACK.md)** - Technology choices and rationale

### Contributing Documentation (`CONTRIBUTING_*.md`)

Guides for developers and contributors:

- **[CONTRIBUTING_GETTING_STARTED.md](./CONTRIBUTING_GETTING_STARTED.md)** - Developer quick start guide
  - Setup, building, development workflow, testing
- **[CONTRIBUTING_TESTING_GUIDE.md](./CONTRIBUTING_TESTING_GUIDE.md)** - Comprehensive testing guide
  - Unit tests, integration tests, test strategy, coverage goals
- **[CONTRIBUTING_PRE_COMMIT.md](./CONTRIBUTING_PRE_COMMIT.md)** - Pre-commit hooks setup
- **[CONTRIBUTING_INSTALLATION.md](./CONTRIBUTING_INSTALLATION.md)** - Tool installation steps
- **[CONTRIBUTING_ADDING_LANGUAGE.md](./CONTRIBUTING_ADDING_LANGUAGE.md)** - How to add language support

### Usage Documentation (`USAGE_*.md`)

User guides and deployment:

- **[USAGE_DOCKER.md](./USAGE_DOCKER.md)** - Docker deployment guide
  - Container setup, MCP client integration, production deployment
- **[USAGE_TROUBLESHOOTING.md](./USAGE_TROUBLESHOOTING.md)** - Common issues and solutions
- **[USAGE_EXPRESSION_SYNTAX.md](./USAGE_EXPRESSION_SYNTAX.md)** - Language-specific expression syntax
- **[USAGE_INTEGRATION_TESTS.md](./USAGE_INTEGRATION_TESTS.md)** - Integration test specifications

### Process Documentation (`PROCESS_*.md`)

Development and release processes:

- **[PROCESS_CI_WORKFLOWS.md](./PROCESS_CI_WORKFLOWS.md)** - CI/CD workflows and pipelines
  - ci.yml and integration-tests-matrix.yml explained
  - Success/failure criteria, troubleshooting
- **[PROCESS_CROSS_PLATFORM_BUILDS.md](./PROCESS_CROSS_PLATFORM_BUILDS.md)** - Building for multiple platforms
- **[PROCESS_RELEASE.md](./PROCESS_RELEASE.md)** - How to create releases
- **[PROCESS_LOG_VALIDATION.md](./PROCESS_LOG_VALIDATION.md)** - Log validation system

---

## 🚀 Quick Navigation

### I want to...

**Understand the architecture**
1. Read the [Architecture Proposal](./ARCHITECTURE_PROPOSAL.md) (Executive Summary + Architecture sections)
2. Review [Component Specifications](./ARCHITECTURE_COMPONENTS.md)
3. Check [Technology Stack](./ARCHITECTURE_TECHNOLOGY_STACK.md) rationale

**Contribute to the codebase**
1. Start with [Getting Started Guide](./CONTRIBUTING_GETTING_STARTED.md)
2. Set up [Pre-commit Hooks](./CONTRIBUTING_PRE_COMMIT.md)
3. Follow [Testing Guide](./CONTRIBUTING_TESTING_GUIDE.md)

**Deploy or use the server**
1. Follow [Docker Deployment](./USAGE_DOCKER.md) guide
2. Refer to [Troubleshooting](./USAGE_TROUBLESHOOTING.md) if issues arise
3. Use [Expression Syntax Guide](./USAGE_EXPRESSION_SYNTAX.md) for language-specific queries

**Add a new programming language**
1. Read [Adding New Language Guide](./CONTRIBUTING_ADDING_LANGUAGE.md)
2. Review [Architecture Proposal](./ARCHITECTURE_PROPOSAL.md) Section 6 (Multi-Language Abstraction)

**Work on CI/CD or releases**
1. Understand [CI Workflows](./PROCESS_CI_WORKFLOWS.md)
2. Follow [Release Process](./PROCESS_RELEASE.md)
3. Check [Cross-Platform Builds](./PROCESS_CROSS_PLATFORM_BUILDS.md)

---

## 🎯 Key Concepts

### What is This Project?

A **Debug Adapter Protocol (DAP) based Model Context Protocol (MCP) server** that enables AI coding agents (Claude, Gemini CLI, etc.) to programmatically debug applications across multiple programming languages through a unified interface.

**Key Features:**
- 🌍 **Language-agnostic**: Supports Python, Ruby, JavaScript/Node.js, Go, Rust (via 40+ DAP implementations)
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
- ✅ 193 comprehensive unit tests
- ✅ 5 language integration tests (100% pass rate)
- ✅ Docker deployment support
- ✅ Complete documentation

**Supported Languages:**
| Language | Status | Test Coverage |
|----------|--------|---------------|
| Python   | ✅ Production | 100% Functional |
| Ruby     | ✅ Production | 100% Functional |
| Node.js  | ✅ Production | 100% Functional |
| Go       | ✅ Production | 100% Functional |
| Rust     | ✅ Production | 100% Functional |

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

**Last Updated**: 2025-10-19
**Documentation Version**: 3.0 (Flat Structure with Category Prefixes)
