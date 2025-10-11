# DAP-Based Debugging MCP Server - Architecture Proposal

**Document Version:** 1.0
**Date:** October 5, 2025
**Status:** Proposal

---

## Executive Summary

This document proposes a **Debug Adapter Protocol (DAP) based Model Context Protocol (MCP) server** implemented in Rust. The server will enable AI coding agents (Claude Code, Gemini CLI, Codex, etc.) to programmatically debug applications across multiple programming languages through a unified, language-agnostic interface.

**Key Value Propositions:**
- **Language-Agnostic Debugging:** Single MCP interface supports Python, JavaScript/Node.js, Go, Rust, C/C++, Java, Ruby, and more
- **AI Agent Integration:** Native MCP protocol enables seamless integration with Claude Desktop and other AI coding assistants
- **Proven Standards:** Leverages Microsoft's Debug Adapter Protocol (40+ debugger implementations)
- **Production-Ready Technology:** Rust implementation with Tokio async runtime for reliability and performance
- **Extensible Architecture:** Plugin system for adding new language debuggers without core changes

---

## Table of Contents

1. [Background and Motivation](#1-background-and-motivation)
2. [Architecture Overview](#2-architecture-overview)
3. [Key Concepts and Terminology](#3-key-concepts-and-terminology)
4. [Technology Stack](#4-technology-stack)
5. [MCP Interface Design](#5-mcp-interface-design)
6. [Multi-Language Abstraction Layer](#6-multi-language-abstraction-layer)
7. [Component Specifications](#7-component-specifications)
8. [Use Cases and User Journeys](#8-use-cases-and-user-journeys)
9. [Implementation Plan](#9-implementation-plan)
10. [Risk Assessment and Mitigation](#10-risk-assessment-and-mitigation)
11. [Future Enhancements](#11-future-enhancements)
12. [Conclusion](#12-conclusion)

---

## 1. Background and Motivation

### 1.1 The Problem

Modern AI coding agents excel at generating, refactoring, and explaining code but lack robust debugging capabilities. Current limitations include:

- **No standardized debugging interface** for AI agents across languages
- **Manual debugging workflows** requiring human intervention at each step
- **Context loss** when switching between AI assistance and debugging tools
- **Language-specific tools** requiring separate integration for each runtime
- **Limited autonomous problem-solving** without execution visibility

### 1.2 The Opportunity

The Debug Adapter Protocol (DAP) provides a proven, language-agnostic abstraction for debuggers used by VS Code, Vim, Emacs, and other editors. By exposing DAP through the Model Context Protocol (MCP), we can enable AI agents to:

- **Set breakpoints** at suspected error locations
- **Step through code** to understand execution flow
- **Inspect variables** to validate assumptions about program state
- **Evaluate expressions** to test hypotheses
- **Track stack traces** to understand call hierarchies
- **Modify variables** to test fixes before code changes

### 1.3 Why Rust?

Rust is the ideal implementation language because:

- **Performance:** Zero-cost abstractions and minimal overhead for process management
- **Safety:** Memory safety and thread safety prevent entire classes of bugs
- **Concurrency:** Tokio async runtime enables handling multiple debug sessions efficiently
- **Ecosystem:** Mature crates for JSON-RPC, process spawning, and IPC communication
- **Reliability:** Error handling with `Result` type ensures robust operation
- **Cross-platform:** Works on Linux, macOS, and Windows

---

## 2. Architecture Overview

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     AI Coding Agent                             │
│            (Claude Desktop, Gemini CLI, Codex)                  │
└────────────────────────┬────────────────────────────────────────┘
                         │ MCP Protocol (JSON-RPC 2.0)
                         │ STDIO or HTTP/SSE Transport
┌────────────────────────┴────────────────────────────────────────┐
│                  DAP MCP Server (Rust)                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │          MCP Protocol Layer                              │  │
│  │  • Resources (session state, breakpoints)                │  │
│  │  • Tools (debug operations)                              │  │
│  │  • Session management                                    │  │
│  └───────────────────────┬──────────────────────────────────┘  │
│  ┌───────────────────────┴──────────────────────────────────┐  │
│  │     Debug Adapter Abstraction Layer                      │  │
│  │  • Language-agnostic debugging interface                 │  │
│  │  • Session lifecycle management                          │  │
│  │  • State machine (idle → running → paused → terminated)  │  │
│  │  • Breakpoint coordination                               │  │
│  │  • Variable inspection API                               │  │
│  └───────────────────────┬──────────────────────────────────┘  │
│  ┌───────────────────────┴──────────────────────────────────┐  │
│  │        DAP Client Implementation                         │  │
│  │  • DAP protocol handling (initialize, requests, events)  │  │
│  │  • Async request/response correlation                    │  │
│  │  • Event stream processing                               │  │
│  │  • Capability negotiation                                │  │
│  └───────────────────────┬──────────────────────────────────┘  │
│  ┌───────────────────────┴──────────────────────────────────┐  │
│  │      Debugger Process Manager                            │  │
│  │  • Adapter process spawning (tokio::process)             │  │
│  │  • STDIO/TCP transport management                        │  │
│  │  • Process lifecycle (spawn → monitor → terminate)       │  │
│  │  • Error recovery and retry logic                        │  │
│  └───────────────────────┬──────────────────────────────────┘  │
└────────────────────────┬─┴──────────────────────────────────────┘
                         │ Debug Adapter Protocol
                         │ STDIO or TCP Socket
         ┌───────────────┼───────────────┬──────────────┐
         │               │               │              │
┌────────┴─────┐  ┌──────┴──────┐  ┌────┴─────┐  ┌────┴─────┐
│   debugpy    │  │ node-debug  │  │   delve  │  │ CodeLLDB │
│   (Python)   │  │  (Node.js)  │  │   (Go)   │  │ (C/C++/  │
│              │  │             │  │          │  │  Rust)   │
└──────┬───────┘  └──────┬──────┘  └────┬─────┘  └────┬─────┘
       │                 │              │             │
┌──────┴───────┐  ┌──────┴──────┐  ┌───┴──────┐  ┌───┴──────┐
│   Python     │  │  Node.js    │  │    Go    │  │  Rust    │
│   Process    │  │  Process    │  │  Process │  │  Process │
└──────────────┘  └─────────────┘  └──────────┘  └──────────┘
```

### 2.2 Layered Design Principles

1. **MCP Protocol Layer**: Exposes debugging capabilities through MCP resources and tools
2. **Debug Adapter Abstraction**: Provides language-agnostic debugging interface
3. **DAP Client**: Implements Debug Adapter Protocol communication
4. **Process Manager**: Handles debugger adapter lifecycle and IPC

Each layer is isolated with clear interfaces, enabling:
- Independent testing of each layer
- Easy addition of new debugger adapters
- Flexible deployment (local, remote, containerized)

### 2.3 Concurrency Model

```
┌─────────────────────────────────────────────────────────────┐
│                    Tokio Runtime                            │
│                                                             │
│  ┌──────────────────┐                                      │
│  │  Main MCP Server │  (STDIO transport)                   │
│  │      Task        │                                      │
│  └────────┬─────────┘                                      │
│           │                                                 │
│  ┌────────┴──────────────────────────────────┐            │
│  │  Session Manager Task                     │            │
│  │  Arc<RwLock<HashMap<SessionId, Session>>> │            │
│  └────────┬──────────────────────────────────┘            │
│           │                                                 │
│  ┌────────┴─────────┬─────────────┬─────────────┐         │
│  │ Session Actor 1  │ Session 2   │ Session 3   │  ...    │
│  │ ┌──────────────┐ │ (Actor)     │ (Actor)     │         │
│  │ │ DAP Client   │ │             │             │         │
│  │ │ Task         │ │             │             │         │
│  │ └──────┬───────┘ │             │             │         │
│  │ ┌──────┴───────┐ │             │             │         │
│  │ │ Debugger     │ │             │             │         │
│  │ │ Process Task │ │             │             │         │
│  │ └──────────────┘ │             │             │         │
│  └──────────────────┴─────────────┴─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

**Concurrency Strategy:**
- **Main server task**: Handles MCP protocol over STDIO
- **Session manager**: Coordinates all debug sessions
- **Per-session actors**: Isolate session state and prevent cross-contamination
- **Debugger process tasks**: One task per spawned debugger adapter
- **Async throughout**: Non-blocking I/O for maximum throughput

---

## 3. Key Concepts and Terminology

### 3.1 Debug Adapter Protocol (DAP)

**Definition:** A standardized, JSON-RPC based protocol developed by Microsoft for communication between development tools and debuggers.

**Key Characteristics:**
- Language-agnostic abstraction layer
- Bidirectional communication (requests, responses, events)
- Capability-based feature negotiation
- Stateful debug sessions
- STDIO or TCP transport

**Core Message Types:**
- **Requests**: Client → Adapter (e.g., `setBreakpoints`, `continue`, `stackTrace`)
- **Responses**: Adapter → Client (success/failure, result data)
- **Events**: Adapter → Client async notifications (e.g., `stopped`, `output`, `terminated`)

### 3.2 Model Context Protocol (MCP)

**Definition:** A protocol for exposing context and capabilities to AI models through resources, tools, and prompts.

**Key Characteristics:**
- JSON-RPC 2.0 over STDIO or HTTP/SSE
- Resources: Read-only contextual data
- Tools: Executable actions the AI can invoke
- Prompts: Templated workflows

**Design Philosophy:**
- **Resources** = Nouns (session state, breakpoint lists)
- **Tools** = Verbs (start debugging, set breakpoint, step)
- **Prompts** = Workflows (debug crash, find memory leak)

### 3.3 Debug Session

**Definition:** A stateful debugging instance from initialization to termination.

**Lifecycle States:**
```
IDLE → INITIALIZING → CONFIGURED → RUNNING → PAUSED → TERMINATED
  ↑                                    ↓        ↑
  └────────────────────────────────────┴────────┘
                (step/continue)
```

**Session Properties:**
- Unique session ID
- Associated debugger adapter
- Target program/process
- Breakpoint state
- Thread and stack frame state
- Variable references

### 3.4 Debug Adapter

**Definition:** An intermediary process that translates DAP messages to language-specific debugger commands.

**Examples:**
- **debugpy**: Python debugging (wraps `pdb` / CPython debug hooks)
- **vscode-node-debug2**: Node.js/JavaScript (Chrome DevTools Protocol)
- **delve**: Go debugging (native Go debugger)
- **CodeLLDB**: C/C++/Rust (wraps LLDB)
- **Java Debug Server**: Java debugging (Java Debug Interface)

### 3.5 Breakpoint

**Definition:** A marker that pauses execution when reached.

**Types Supported:**
1. **Source Breakpoints**: Line/column in source file
2. **Function Breakpoints**: Function name
3. **Conditional Breakpoints**: Break when expression is true
4. **Logpoints**: Log message without pausing
5. **Data Breakpoints**: Break on memory access (watchpoints)
6. **Exception Breakpoints**: Break on thrown exceptions

### 3.6 Variable Inspection

**Definition:** Hierarchical exploration of program state.

**Hierarchy:**
```
Stack Frame
  └─► Scopes (Locals, Arguments, Globals)
      └─► Variables
          └─► Nested Variables (for structured data)
```

**Features:**
- Lazy loading (expand on demand)
- Expression evaluation
- Value modification
- Memory address inspection

---

## 4. Technology Stack

### 4.1 Core Dependencies

| Component | Crate | Version | Purpose |
|-----------|-------|---------|---------|
| **MCP Protocol** | `mcp-sdk` | 0.x | Official Rust MCP SDK |
| **Async Runtime** | `tokio` | 1.x | Process management, async I/O |
| **Serialization** | `serde`, `serde_json` | 1.0 | JSON encoding/decoding |
| **DAP Protocol** | `dap` or `dap-types` | 0.x | Debug Adapter Protocol types |
| **Error Handling** | `anyhow`, `thiserror` | 1.0 | Application and library errors |
| **IPC** | `interprocess` | 2.x | Cross-platform IPC with Tokio |
| **Channels** | `flume` | 0.x | Fast async MPMC channels |
| **Logging** | `tracing`, `tracing-subscriber` | 0.1/0.3 | Structured logging |

### 4.2 Cargo.toml Configuration

```toml
[package]
name = "dap-mcp-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# MCP Protocol
mcp-sdk = "0.3"

# Async Runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Debug Adapter Protocol
dap = "0.5"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# IPC and Channels
interprocess = { version = "2", features = ["tokio"] }
flume = "0.11"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
async-trait = "0.1"

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
```

### 4.3 Architecture Justifications

#### Why Tokio?
- **Industry standard** for async Rust
- **Comprehensive** process, filesystem, and network support
- **Battle-tested** in production systems
- **Rich ecosystem** of compatible crates

#### Why Arc + RwLock for State?
- **Read-heavy workload**: Debugging queries (stack traces, variables) far exceed writes
- **RwLock** allows multiple concurrent readers
- **Arc** enables safe sharing across async tasks
- **Alternative considered**: Actor model with message passing (adds latency for simple reads)

#### Why STDIO Transport?
- **Simplicity**: No network stack, firewall, or port management
- **Security**: Process isolation, no remote access concerns
- **Standard**: MCP servers typically use STDIO with Claude Desktop
- **Future**: Can add HTTP/SSE for remote scenarios

---

## 5. MCP Interface Design

### 5.1 Resources (Read-Only Debugging Context)

Resources expose debugging state for AI agent awareness.

#### 5.1.1 Session Resources

**Resource URI Pattern**: `debugger://sessions/{session_id}`

```json
{
  "uri": "debugger://sessions/550e8400-e29b-41d4-a716-446655440000",
  "name": "Python Debug Session - main.py",
  "description": "Active debug session for Python application",
  "mimeType": "application/json",
  "content": {
    "sessionId": "550e8400-e29b-41d4-a716-446655440000",
    "state": "paused",
    "language": "python",
    "adapter": "debugpy",
    "program": "/home/user/project/main.py",
    "pid": 12345,
    "stoppedReason": "breakpoint",
    "stoppedThreadId": 1,
    "capabilities": {
      "supportsConditionalBreakpoints": true,
      "supportsLogPoints": true,
      "supportsDataBreakpoints": false,
      "supportsStepBack": false
    }
  }
}
```

**Use Cases:**
- AI checks if debugging session exists before operations
- AI determines session state (can't step if not paused)
- AI discovers debugger capabilities before using advanced features

#### 5.1.2 Breakpoint Resources

**Resource URI**: `debugger://breakpoints`

```json
{
  "uri": "debugger://breakpoints",
  "name": "All Active Breakpoints",
  "mimeType": "application/json",
  "content": {
    "breakpoints": [
      {
        "id": 1,
        "source": "/home/user/project/main.py",
        "line": 42,
        "verified": true,
        "condition": "x > 10",
        "hitCount": 3
      },
      {
        "id": 2,
        "source": "/home/user/project/utils.py",
        "line": 15,
        "verified": true,
        "logMessage": "Processing item: {item}"
      }
    ]
  }
}
```

**Use Cases:**
- AI reviews existing breakpoints before adding new ones
- AI removes obsolete breakpoints
- AI checks if breakpoint was successfully verified

#### 5.1.3 Stack Trace Resources

**Resource URI**: `debugger://sessions/{session_id}/stackTrace`

```json
{
  "uri": "debugger://sessions/550e8400/stackTrace",
  "name": "Current Stack Trace",
  "mimeType": "application/json",
  "content": {
    "threadId": 1,
    "frames": [
      {
        "id": 1000,
        "name": "processData",
        "source": "/home/user/project/main.py",
        "line": 42,
        "column": 12
      },
      {
        "id": 1001,
        "name": "handleRequest",
        "source": "/home/user/project/server.py",
        "line": 78,
        "column": 5
      },
      {
        "id": 1002,
        "name": "main",
        "source": "/home/user/project/main.py",
        "line": 120,
        "column": 1
      }
    ]
  }
}
```

**Use Cases:**
- AI analyzes call hierarchy to understand code flow
- AI identifies frame to evaluate expressions in
- AI tracks recursion depth

#### 5.1.4 Variable Resources

**Resource URI**: `debugger://sessions/{session_id}/frames/{frame_id}/variables`

```json
{
  "uri": "debugger://sessions/550e8400/frames/1000/variables",
  "name": "Local Variables (processData frame)",
  "mimeType": "application/json",
  "content": {
    "locals": [
      {
        "name": "data",
        "value": "[1, 2, 3, 4, 5]",
        "type": "list",
        "variablesReference": 5001
      },
      {
        "name": "result",
        "value": "None",
        "type": "NoneType",
        "variablesReference": 0
      },
      {
        "name": "x",
        "value": "15",
        "type": "int",
        "variablesReference": 0
      }
    ]
  }
}
```

**Use Cases:**
- AI inspects variable values to verify assumptions
- AI expands structured variables (lists, dicts, objects)
- AI compares variable state across frames

### 5.2 Tools (Executable Debugging Actions)

Tools enable AI to control debugging execution.

#### 5.2.1 Session Management Tools

##### Tool: `debugger_start`

**Purpose**: Launch or attach to a program for debugging

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "mode": {
      "type": "string",
      "enum": ["launch", "attach"],
      "description": "Launch new process or attach to existing"
    },
    "language": {
      "type": "string",
      "enum": ["python", "javascript", "go", "rust", "cpp", "java"],
      "description": "Programming language of target program"
    },
    "program": {
      "type": "string",
      "description": "Path to program file (launch mode)"
    },
    "args": {
      "type": "array",
      "items": {"type": "string"},
      "description": "Command-line arguments"
    },
    "cwd": {
      "type": "string",
      "description": "Working directory"
    },
    "env": {
      "type": "object",
      "description": "Environment variables"
    },
    "pid": {
      "type": "integer",
      "description": "Process ID (attach mode)"
    },
    "stopOnEntry": {
      "type": "boolean",
      "default": false,
      "description": "Break at program entry point"
    }
  },
  "required": ["mode", "language"]
}
```

**Output**:
```json
{
  "sessionId": "550e8400-e29b-41d4-a716-446655440000",
  "state": "running",
  "pid": 12345
}
```

**Example Call**:
```json
{
  "mode": "launch",
  "language": "python",
  "program": "/home/user/project/main.py",
  "args": ["--config", "dev.yaml"],
  "stopOnEntry": true
}
```

##### Tool: `debugger_stop`

**Purpose**: Terminate a debug session

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {
      "type": "string",
      "description": "Session to terminate"
    },
    "terminateDebuggee": {
      "type": "boolean",
      "default": true,
      "description": "Kill debuggee process or just detach"
    }
  },
  "required": ["sessionId"]
}
```

#### 5.2.2 Execution Control Tools

##### Tool: `debugger_continue`

**Purpose**: Resume program execution until next breakpoint

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "threadId": {
      "type": "integer",
      "description": "Specific thread (optional, default all threads)"
    }
  },
  "required": ["sessionId"]
}
```

##### Tool: `debugger_pause`

**Purpose**: Pause running program

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "threadId": {"type": "integer"}
  },
  "required": ["sessionId"]
}
```

##### Tool: `debugger_step_over`

**Purpose**: Execute next statement, stepping over function calls

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "threadId": {"type": "integer"},
    "granularity": {
      "type": "string",
      "enum": ["statement", "line", "instruction"],
      "default": "statement"
    }
  },
  "required": ["sessionId", "threadId"]
}
```

##### Tool: `debugger_step_into`

**Purpose**: Execute next statement, stepping into function calls

**Input Schema**: Same as `debugger_step_over`

##### Tool: `debugger_step_out`

**Purpose**: Execute until current function returns

**Input Schema**: Same as `debugger_step_over`

#### 5.2.3 Breakpoint Tools

##### Tool: `debugger_set_breakpoint`

**Purpose**: Set a breakpoint at a source location

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "source": {
      "type": "string",
      "description": "Source file path"
    },
    "line": {
      "type": "integer",
      "description": "Line number (1-indexed)"
    },
    "column": {
      "type": "integer",
      "description": "Column number (optional)"
    },
    "condition": {
      "type": "string",
      "description": "Break only if expression is true"
    },
    "hitCondition": {
      "type": "string",
      "description": "Hit count condition (e.g., '>= 5')"
    },
    "logMessage": {
      "type": "string",
      "description": "Log message instead of breaking (logpoint)"
    }
  },
  "required": ["sessionId", "source", "line"]
}
```

**Output**:
```json
{
  "id": 1,
  "verified": true,
  "line": 42,
  "message": null
}
```

##### Tool: `debugger_remove_breakpoint`

**Purpose**: Remove a breakpoint

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "breakpointId": {"type": "integer"}
  },
  "required": ["sessionId", "breakpointId"]
}
```

##### Tool: `debugger_set_exception_breakpoints`

**Purpose**: Configure exception handling

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "filters": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": ["uncaught", "all", "userUnhandled"]
      }
    }
  },
  "required": ["sessionId", "filters"]
}
```

#### 5.2.4 Inspection Tools

##### Tool: `debugger_evaluate`

**Purpose**: Evaluate an expression in the context of a stack frame

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "expression": {
      "type": "string",
      "description": "Expression to evaluate"
    },
    "frameId": {
      "type": "integer",
      "description": "Stack frame context (optional)"
    },
    "context": {
      "type": "string",
      "enum": ["watch", "repl", "hover"],
      "default": "repl"
    }
  },
  "required": ["sessionId", "expression"]
}
```

**Output**:
```json
{
  "result": "15",
  "type": "int",
  "variablesReference": 0
}
```

##### Tool: `debugger_get_variables`

**Purpose**: Retrieve variables for a scope or parent variable

**Input Schema**:
```json
{
  "type": "object",
  "properties": {
    "sessionId": {"type": "string"},
    "variablesReference": {
      "type": "integer",
      "description": "Reference from scope or parent variable"
    },
    "filter": {
      "type": "string",
      "enum": ["indexed", "named"],
      "description": "Filter variable type"
    },
    "start": {
      "type": "integer",
      "description": "Pagination start index"
    },
    "count": {
      "type": "integer",
      "description": "Number of variables to retrieve"
    }
  },
  "required": ["sessionId", "variablesReference"]
}
```

**Output**:
```json
{
  "variables": [
    {
      "name": "data",
      "value": "[1, 2, 3, 4, 5]",
      "type": "list",
      "variablesReference": 5001
    }
  ]
}
```

### 5.3 Design Rationale

**Why these specific resources and tools?**

1. **Resources expose state, tools mutate state**: Clean separation of concerns
2. **Granular tools**: Each tool does one thing well, composable for complex workflows
3. **Async-friendly**: All tools return immediately, events update resources
4. **Language-agnostic**: No Python-specific or JavaScript-specific tools
5. **Safety**: Tools validate session state before operations (can't step if not paused)

**MCP Best Practices Applied:**
- Keep tool count manageable (~15 tools vs. 40+ DAP requests)
- Group related functionality (all stepping operations could be one tool with a type parameter)
- Provide clear error messages AI can understand and act on
- Use JSON schemas for automatic parameter validation

---

## 6. Multi-Language Abstraction Layer

### 6.1 The Challenge

Different programming languages have different:
- **Debuggers**: GDB (C/C++), pdb (Python), Delve (Go), etc.
- **Runtime models**: Compiled vs. interpreted, native vs. VM
- **Debug capabilities**: Not all support data breakpoints, reverse debugging, etc.
- **Launch mechanisms**: Binary execution vs. interpreter with script
- **Configuration**: Environment setup, paths, build artifacts

**Goal**: Hide these differences behind a unified MCP interface.

### 6.2 Adapter Registry Pattern

The server maintains a **registry** of debug adapter configurations:

```rust
pub struct AdapterConfig {
    /// Unique identifier
    pub id: String,

    /// Language this adapter supports
    pub language: String,

    /// Adapter type
    pub adapter_type: AdapterType,

    /// How to spawn the adapter
    pub spawn_config: SpawnConfig,

    /// Default capabilities
    pub default_capabilities: AdapterCapabilities,
}

pub enum AdapterType {
    /// Launch adapter as subprocess
    Executable {
        command: String,
        args: Vec<String>,
    },

    /// Connect to TCP server
    Server {
        host: String,
        port: u16,
    },

    /// Use Unix domain socket
    Pipe {
        path: String,
    },
}

pub struct SpawnConfig {
    /// Working directory
    pub cwd: Option<String>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Timeout for adapter startup
    pub startup_timeout: Duration,
}
```

**Built-in Adapter Configurations:**

```rust
// Python
AdapterConfig {
    id: "debugpy",
    language: "python",
    adapter_type: AdapterType::Executable {
        command: "python",
        args: vec!["-m", "debugpy.adapter"],
    },
    spawn_config: SpawnConfig {
        env: hashmap! {
            "PYTHONUNBUFFERED" => "1",
        },
        startup_timeout: Duration::from_secs(10),
    },
    default_capabilities: AdapterCapabilities {
        supports_conditional_breakpoints: true,
        supports_log_points: true,
        supports_data_breakpoints: false,
        supports_step_back: false,
        ...
    },
}

// Node.js
AdapterConfig {
    id: "node-debug",
    language: "javascript",
    adapter_type: AdapterType::Executable {
        command: "node",
        args: vec!["/path/to/vscode-node-debug2/out/src/nodeDebug.js"],
    },
    ...
}

// Go
AdapterConfig {
    id: "delve",
    language: "go",
    adapter_type: AdapterType::Executable {
        command: "dlv",
        args: vec!["dap"],
    },
    ...
}

// Rust/C/C++
AdapterConfig {
    id: "codelldb",
    language: "rust",
    adapter_type: AdapterType::Executable {
        command: "codelldb",
        args: vec!["--port", "0"],  // Dynamic port allocation
    },
    ...
}
```

### 6.3 Capability Abstraction

Not all debuggers support all features. The abstraction layer handles this through:

#### 6.3.1 Capability Negotiation

During `initialize` request:
1. MCP server sends client capabilities to adapter
2. Adapter responds with its capabilities
3. Server stores capabilities in session state
4. MCP tools check capabilities before operations

**Example: Data Breakpoints**

```rust
pub async fn set_data_breakpoint(
    session_id: &str,
    data_id: &str,
) -> Result<Breakpoint, Error> {
    let session = get_session(session_id)?;

    // Check capability
    if !session.capabilities.supports_data_breakpoints {
        return Err(Error::UnsupportedFeature(
            "Data breakpoints not supported by this debugger"
        ));
    }

    // Proceed with DAP request
    ...
}
```

#### 6.3.2 Graceful Degradation

When a feature isn't supported:
1. Return clear error message to AI agent
2. Suggest alternatives if possible
3. Don't crash or hang

**Example Error Response:**

```json
{
  "error": {
    "code": "FEATURE_NOT_SUPPORTED",
    "message": "Data breakpoints are not supported by debugpy (Python debugger). Consider using conditional breakpoints instead: `debugger_set_breakpoint` with a `condition` parameter.",
    "details": {
      "feature": "data_breakpoints",
      "adapter": "debugpy",
      "language": "python",
      "alternative": "conditional_breakpoints"
    }
  }
}
```

### 6.4 Launch Configuration Abstraction

Each language has different launch requirements. The abstraction layer provides **templates**:

```rust
pub trait LaunchConfigTemplate {
    /// Generate DAP launch configuration from MCP parameters
    fn build_launch_config(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        env: &HashMap<String, String>,
    ) -> serde_json::Value;
}
```

**Python Launch Template:**

```rust
impl LaunchConfigTemplate for PythonTemplate {
    fn build_launch_config(...) -> serde_json::Value {
        json!({
            "type": "python",
            "request": "launch",
            "program": program,
            "args": args,
            "cwd": cwd,
            "env": env,
            "stopOnEntry": false,
            "console": "integratedTerminal",
            "justMyCode": false,  // Debug into libraries
        })
    }
}
```

**Node.js Launch Template:**

```rust
impl LaunchConfigTemplate for NodeJsTemplate {
    fn build_launch_config(...) -> serde_json::Value {
        json!({
            "type": "node",
            "request": "launch",
            "program": program,
            "args": args,
            "cwd": cwd,
            "env": env,
            "stopOnEntry": false,
            "skipFiles": ["<node_internals>/**"],
        })
    }
}
```

**Go Launch Template:**

```rust
impl LaunchConfigTemplate for GoTemplate {
    fn build_launch_config(...) -> serde_json::Value {
        json!({
            "type": "go",
            "request": "launch",
            "mode": "debug",
            "program": program,
            "args": args,
            "cwd": cwd,
            "env": env,
        })
    }
}
```

### 6.5 Unified Session Lifecycle

Despite different adapters, all sessions follow the same lifecycle:

```
┌──────────────────────────────────────────────────────────┐
│                  IDLE (no session)                       │
└────────────────────┬─────────────────────────────────────┘
                     │ debugger_start
                     ▼
┌──────────────────────────────────────────────────────────┐
│              INITIALIZING                                │
│  • Spawn adapter process                                 │
│  • Send initialize request                               │
│  • Receive initialized event                             │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│              CONFIGURING                                 │
│  • Set breakpoints                                       │
│  • Set exception breakpoints                             │
│  • Send configurationDone                                │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│              LAUNCHING                                   │
│  • Send launch/attach request                            │
│  • Wait for response                                     │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│              RUNNING                                     │
│  • Program executing                                     │
│  • No interaction possible                               │
└────────┬────────────────────────┬─────────────────────────┘
         │ stopped event          │ exited/terminated event
         ▼                        ▼
┌──────────────────────────┐  ┌──────────────────────────┐
│        PAUSED            │  │      TERMINATED          │
│  • Can inspect state     │  │  • Session ended         │
│  • Can step/continue     │  │  • Resources cleaned up  │
│  • Can evaluate          │  └──────────────────────────┘
└────────┬─────────────────┘
         │ continue/step
         │
         └──► RUNNING
```

**State Machine Implementation:**

```rust
pub enum SessionState {
    Initializing,
    Configuring,
    Launching,
    Running,
    Paused {
        reason: StopReason,
        thread_id: Option<i64>,
    },
    Terminated {
        exit_code: Option<i32>,
    },
}

impl DebugSession {
    pub fn can_step(&self) -> bool {
        matches!(self.state, SessionState::Paused { .. })
    }

    pub fn can_continue(&self) -> bool {
        matches!(self.state, SessionState::Paused { .. })
    }

    pub fn can_set_breakpoint(&self) -> bool {
        !matches!(self.state, SessionState::Terminated { .. })
    }
}
```

### 6.6 Adding New Language Support

**Steps to add a new language:**

1. **Create adapter configuration**:
   ```rust
   AdapterConfig {
       id: "ruby-debug",
       language: "ruby",
       adapter_type: AdapterType::Executable {
           command: "rdbg",
           args: vec!["--open", "--command"],
       },
       ...
   }
   ```

2. **Implement launch template**:
   ```rust
   impl LaunchConfigTemplate for RubyTemplate {
       fn build_launch_config(...) -> serde_json::Value {
           json!({
               "type": "ruby",
               "request": "launch",
               "program": program,
               "args": args,
           })
       }
   }
   ```

3. **Register in adapter registry**:
   ```rust
   registry.register(ruby_config);
   ```

4. **Test with sample program**:
   - Create test Ruby script
   - Use `debugger_start` with `language: "ruby"`
   - Verify breakpoints, stepping, inspection work

**No MCP interface changes needed!** The abstraction layer handles everything.

---

## 7. Component Specifications

### 7.1 MCP Protocol Layer

**Module**: `src/mcp/mod.rs`

**Responsibilities**:
- Handle MCP JSON-RPC 2.0 protocol
- Expose resources and tools
- Manage STDIO transport
- Route requests to debug abstraction layer

**Key Types**:
```rust
pub struct McpServer {
    session_manager: Arc<SessionManager>,
    resource_handlers: HashMap<String, Box<dyn ResourceHandler>>,
    tool_handlers: HashMap<String, Box<dyn ToolHandler>>,
}

#[async_trait]
pub trait ResourceHandler: Send + Sync {
    async fn handle(&self, uri: &str) -> Result<Resource, Error>;
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn handle(&self, params: serde_json::Value) -> Result<serde_json::Value, Error>;
}
```

**Resource Handlers**:
- `SessionListResourceHandler` → `debugger://sessions`
- `SessionResourceHandler` → `debugger://sessions/{id}`
- `BreakpointsResourceHandler` → `debugger://breakpoints`
- `StackTraceResourceHandler` → `debugger://sessions/{id}/stackTrace`
- `VariablesResourceHandler` → `debugger://sessions/{id}/frames/{frameId}/variables`

**Tool Handlers**:
- `StartDebuggerTool`
- `StopDebuggerTool`
- `ContinueTool`
- `PauseTool`
- `StepOverTool`, `StepIntoTool`, `StepOutTool`
- `SetBreakpointTool`, `RemoveBreakpointTool`
- `EvaluateTool`, `GetVariablesTool`

### 7.2 Debug Abstraction Layer

**Module**: `src/debug/mod.rs`

**Responsibilities**:
- Manage debug sessions
- Provide language-agnostic debugging API
- Coordinate with DAP client layer
- Maintain session state

**Key Types**:
```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<DebugSession>>>>>,
    adapter_registry: Arc<AdapterRegistry>,
}

pub struct DebugSession {
    pub id: String,
    pub state: SessionState,
    pub language: String,
    pub adapter_id: String,
    pub program: Option<String>,
    pub pid: Option<u32>,
    pub capabilities: AdapterCapabilities,
    pub breakpoints: Vec<Breakpoint>,
    pub threads: Vec<Thread>,
    pub current_frame: Option<i64>,
    dap_client: Arc<DapClient>,
}

impl SessionManager {
    pub async fn create_session(
        &self,
        language: &str,
        config: LaunchConfig,
    ) -> Result<String, Error>;

    pub async fn get_session(&self, id: &str) -> Result<Arc<RwLock<DebugSession>>, Error>;

    pub async fn terminate_session(&self, id: &str) -> Result<(), Error>;
}

impl DebugSession {
    pub async fn set_breakpoint(&mut self, bp: BreakpointSpec) -> Result<Breakpoint, Error>;

    pub async fn continue_execution(&mut self, thread_id: Option<i64>) -> Result<(), Error>;

    pub async fn step_over(&mut self, thread_id: i64) -> Result<(), Error>;

    pub async fn evaluate(&mut self, expr: &str, frame_id: Option<i64>) -> Result<EvalResult, Error>;

    pub async fn get_stack_trace(&mut self, thread_id: i64) -> Result<Vec<StackFrame>, Error>;

    pub async fn get_variables(&mut self, var_ref: i64) -> Result<Vec<Variable>, Error>;
}
```

### 7.3 DAP Client Layer

**Module**: `src/dap/client.rs`

**Responsibilities**:
- Implement Debug Adapter Protocol client
- Send DAP requests and await responses
- Process DAP events
- Manage request sequence numbers

**Key Types**:
```rust
pub struct DapClient {
    process: Arc<RwLock<Option<Child>>>,
    transport: Arc<DapTransport>,
    event_sender: flume::Sender<DapEvent>,
    event_receiver: flume::Receiver<DapEvent>,
    request_id: Arc<AtomicU64>,
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<DapResponse>>>>,
}

impl DapClient {
    pub async fn initialize(&self, capabilities: ClientCapabilities) -> Result<ServerCapabilities, Error>;

    pub async fn launch(&self, config: serde_json::Value) -> Result<(), Error>;

    pub async fn attach(&self, config: serde_json::Value) -> Result<(), Error>;

    pub async fn set_breakpoints(&self, source: &str, breakpoints: &[SourceBreakpoint]) -> Result<Vec<Breakpoint>, Error>;

    pub async fn continue_execution(&self, thread_id: Option<i64>) -> Result<(), Error>;

    pub async fn next(&self, thread_id: i64, granularity: Option<SteppingGranularity>) -> Result<(), Error>;

    pub async fn step_in(&self, thread_id: i64) -> Result<(), Error>;

    pub async fn step_out(&self, thread_id: i64) -> Result<(), Error>;

    pub async fn stack_trace(&self, thread_id: i64, start_frame: Option<i64>, levels: Option<i64>) -> Result<StackTraceResponse, Error>;

    pub async fn scopes(&self, frame_id: i64) -> Result<Vec<Scope>, Error>;

    pub async fn variables(&self, variables_reference: i64, filter: Option<VariableFilter>) -> Result<Vec<Variable>, Error>;

    pub async fn evaluate(&self, expression: &str, frame_id: Option<i64>, context: Option<EvaluateContext>) -> Result<EvaluateResponse, Error>;

    pub async fn disconnect(&self, terminate_debuggee: bool) -> Result<(), Error>;

    pub fn subscribe_events(&self) -> flume::Receiver<DapEvent>;
}
```

**Event Processing**:
```rust
impl DapClient {
    async fn process_events(
        transport: Arc<DapTransport>,
        event_sender: flume::Sender<DapEvent>,
        pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<DapResponse>>>>,
    ) {
        while let Ok(message) = transport.receive().await {
            match message.message_type {
                "event" => {
                    let event: DapEvent = serde_json::from_value(message.body)?;
                    event_sender.send_async(event).await?;
                }
                "response" => {
                    let seq = message.request_seq;
                    if let Some(sender) = pending_requests.write().await.remove(&seq) {
                        let response: DapResponse = serde_json::from_value(message.body)?;
                        sender.send(response).ok();
                    }
                }
                _ => {}
            }
        }
    }
}
```

### 7.4 Process Manager

**Module**: `src/process/manager.rs`

**Responsibilities**:
- Spawn debug adapter processes
- Manage STDIO/TCP communication
- Handle process lifecycle
- Implement retry and recovery logic

**Key Types**:
```rust
pub struct ProcessManager;

impl ProcessManager {
    pub async fn spawn_adapter(
        config: &AdapterConfig,
    ) -> Result<(Child, DapTransport), Error> {
        match &config.adapter_type {
            AdapterType::Executable { command, args } => {
                let child = Command::new(command)
                    .args(args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .envs(&config.spawn_config.env)
                    .current_dir(config.spawn_config.cwd.as_ref().unwrap_or(&".".to_string()))
                    .kill_on_drop(true)
                    .spawn()?;

                let stdin = child.stdin.take().unwrap();
                let stdout = child.stdout.take().unwrap();

                let transport = DapTransport::new_stdio(stdin, stdout);

                Ok((child, transport))
            }
            AdapterType::Server { host, port } => {
                let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
                let transport = DapTransport::new_tcp(stream);
                Ok((/* no process */, transport))
            }
            AdapterType::Pipe { path } => {
                // Unix domain socket connection
                todo!("Implement pipe transport")
            }
        }
    }
}

pub struct DapTransport {
    writer: Arc<Mutex<Box<dyn AsyncWrite + Send + Unpin>>>,
    reader: Arc<Mutex<Box<dyn AsyncBufRead + Send + Unpin>>>,
}

impl DapTransport {
    pub async fn send(&self, message: &DapMessage) -> Result<(), Error> {
        let json = serde_json::to_string(message)?;
        let content_length = json.len();

        let mut writer = self.writer.lock().await;
        writer.write_all(format!("Content-Length: {}\r\n\r\n", content_length).as_bytes()).await?;
        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }

    pub async fn receive(&self) -> Result<DapMessage, Error> {
        let mut reader = self.reader.lock().await;

        // Read headers
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            if line == "\r\n" {
                break; // End of headers
            }

            if line.starts_with("Content-Length:") {
                content_length = line[15..].trim().parse()?;
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).await?;

        let message: DapMessage = serde_json::from_slice(&body)?;
        Ok(message)
    }
}
```

### 7.5 Adapter Registry

**Module**: `src/adapters/registry.rs`

**Responsibilities**:
- Store adapter configurations
- Provide adapter lookup by language
- Support custom adapter registration

**Key Types**:
```rust
pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterConfig>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: HashMap::new(),
        };

        // Register built-in adapters
        registry.register(python_adapter_config());
        registry.register(nodejs_adapter_config());
        registry.register(go_adapter_config());
        registry.register(rust_adapter_config());

        registry
    }

    pub fn register(&mut self, config: AdapterConfig) {
        self.adapters.insert(config.language.clone(), config);
    }

    pub fn get(&self, language: &str) -> Option<&AdapterConfig> {
        self.adapters.get(language)
    }

    pub fn supported_languages(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }
}

fn python_adapter_config() -> AdapterConfig {
    AdapterConfig {
        id: "debugpy".to_string(),
        language: "python".to_string(),
        adapter_type: AdapterType::Executable {
            command: "python".to_string(),
            args: vec!["-m".to_string(), "debugpy.adapter".to_string()],
        },
        spawn_config: SpawnConfig {
            cwd: None,
            env: hashmap! {
                "PYTHONUNBUFFERED".to_string() => "1".to_string(),
            },
            startup_timeout: Duration::from_secs(10),
        },
        default_capabilities: AdapterCapabilities::default(),
    }
}

// Similar functions for nodejs_adapter_config(), go_adapter_config(), etc.
```

---

## 8. Use Cases and User Journeys

### 8.1 Use Case 1: Debug a Crash

**Scenario**: AI agent helps user debug a Python script that crashes with an exception.

**User Journey**:

1. **User**: "My Python script crashes when I run it. Can you help debug it?"

2. **AI Agent** (internal reasoning):
   - Need to run the script under debugger
   - Should break on exceptions to inspect state
   - Need to examine variables when exception occurs

3. **AI → MCP Tool Call**: `debugger_start`
   ```json
   {
     "mode": "launch",
     "language": "python",
     "program": "/home/user/script.py",
     "stopOnEntry": false
   }
   ```

4. **MCP Server**:
   - Spawns `debugpy` adapter
   - Sends DAP `initialize` request
   - Sends DAP `launch` request
   - Returns session ID

5. **AI → MCP Tool Call**: `debugger_set_exception_breakpoints`
   ```json
   {
     "sessionId": "550e8400...",
     "filters": ["uncaught"]
   }
   ```

6. **AI → MCP Tool Call**: `debugger_continue`
   ```json
   {
     "sessionId": "550e8400..."
   }
   ```

7. **Program runs → Exception thrown → Debugger pauses**

8. **MCP Server**:
   - Receives DAP `stopped` event (reason: exception)
   - Updates session state to Paused
   - Updates resources

9. **AI → MCP Resource Read**: `debugger://sessions/550e8400/stackTrace`
   - Sees where exception occurred (file, line, function)

10. **AI → MCP Tool Call**: `debugger_evaluate`
    ```json
    {
      "sessionId": "550e8400...",
      "expression": "locals()",
      "frameId": 1000
    }
    ```
    - Inspects local variable values

11. **AI → MCP Resource Read**: `debugger://sessions/550e8400/frames/1000/variables`
    - Examines specific variables that might be None or invalid

12. **AI** (analysis):
    - Identifies variable `user_data` is `None`
    - Traces back why it's None by examining calling functions
    - Determines missing error handling for API failure

13. **AI → User**: "The crash occurs because `user_data` is None when `fetch_user()` fails. The code doesn't check for None before accessing `user_data.name`. Here's a fix..." [suggests code change]

14. **AI → MCP Tool Call**: `debugger_stop`
    ```json
    {
      "sessionId": "550e8400...",
      "terminateDebuggee": true
    }
    ```

**Value Delivered**:
- AI autonomously ran debugger
- Identified root cause without user intervention
- Provided actionable fix

---

### 8.2 Use Case 2: Understand Complex Code Flow

**Scenario**: User asks AI to explain how a complex algorithm works.

**User Journey**:

1. **User**: "Can you explain how the `merge_sort` function in `algorithms.py` works? I'm confused about the recursion."

2. **AI Agent**:
   - Could just read the code and explain it
   - Better: Step through it with a debugger to show actual execution flow

3. **AI → MCP Tool Call**: `debugger_start`
   ```json
   {
     "mode": "launch",
     "language": "python",
     "program": "/home/user/test_merge_sort.py",
     "stopOnEntry": true
   }
   ```

4. **AI → MCP Tool Call**: `debugger_set_breakpoint`
   ```json
   {
     "sessionId": "abc123...",
     "source": "/home/user/algorithms.py",
     "line": 45
   }
   ```
   (Line 45 is start of `merge_sort` function)

5. **AI → MCP Tool Call**: `debugger_continue`

6. **Debugger hits breakpoint**

7. **AI → MCP Resource Read**: `debugger://sessions/abc123/stackTrace`
   - Notes initial call with input array `[5, 2, 8, 1, 9]`

8. **AI → MCP Tool Call**: `debugger_step_into` (multiple times)
   - Steps through recursive calls
   - At each step, reads variables to see array subdivisions

9. **AI → MCP Tool Call**: `debugger_evaluate`
   ```json
   {
     "expression": "left",
     "frameId": 2000
   }
   ```
   - Sees left subarray is `[5, 2]`

10. **AI continues stepping**, building a trace:
    ```
    merge_sort([5, 2, 8, 1, 9])
      ↳ merge_sort([5, 2])
        ↳ merge_sort([5]) → returns [5]
        ↳ merge_sort([2]) → returns [2]
        ↳ merge([5], [2]) → returns [2, 5]
      ↳ merge_sort([8, 1, 9])
        ↳ merge_sort([8]) → returns [8]
        ↳ merge_sort([1, 9])
          ↳ merge_sort([1]) → returns [1]
          ↳ merge_sort([9]) → returns [9]
          ↳ merge([1], [9]) → returns [1, 9]
        ↳ merge([8], [1, 9]) → returns [1, 8, 9]
      ↳ merge([2, 5], [1, 8, 9]) → returns [1, 2, 5, 8, 9]
    ```

11. **AI → User**: "Here's how merge_sort works, visualized from the actual execution:\n\n[Shows call tree with array values at each step]\n\nNotice how the array is recursively split until each sub-array has 1 element (which is trivially sorted), then merged back together in sorted order."

**Value Delivered**:
- Dynamic explanation based on real execution
- Visual call tree with actual values
- Deeper understanding than static code reading

---

### 8.3 Use Case 3: Find Performance Bottleneck

**Scenario**: User's Node.js API is slow and asks AI to find the bottleneck.

**User Journey**:

1. **User**: "My API endpoint is taking 5 seconds to respond. Can you find what's slow?"

2. **AI Agent**:
   - Strategy: Use logpoints to timestamp each function call
   - Identify which function takes the most time

3. **AI → MCP Tool Call**: `debugger_start`
   ```json
   {
     "mode": "launch",
     "language": "javascript",
     "program": "/home/user/server.js",
     "args": ["--port", "3000"]
   }
   ```

4. **AI** (code analysis):
   - Reads API handler code
   - Identifies key functions: `validateRequest()`, `fetchUserData()`, `processPayment()`, `sendEmail()`, `logRequest()`

5. **AI → MCP Tool Call**: `debugger_set_breakpoint` (logpoint style)
   ```json
   {
     "sessionId": "xyz789...",
     "source": "/home/user/api/handler.js",
     "line": 42,
     "logMessage": "TIMING: validateRequest start - {Date.now()}"
   }
   ```
   (Repeats for each function entry/exit with timestamps)

6. **AI triggers the slow endpoint** (via separate HTTP request or instructs user to)

7. **MCP Server**:
   - Receives DAP `output` events with log messages
   - Passes to AI

8. **AI** (analyzes timestamps):
   ```
   TIMING: validateRequest start - 1609459200000
   TIMING: validateRequest end - 1609459200100 (100ms)
   TIMING: fetchUserData start - 1609459200100
   TIMING: fetchUserData end - 1609459200150 (50ms)
   TIMING: processPayment start - 1609459200150
   TIMING: processPayment end - 1609459204150 (4000ms) ← BOTTLENECK
   TIMING: sendEmail start - 1609459204150
   TIMING: sendEmail end - 1609459204900 (750ms)
   ```

9. **AI → MCP Tool Call**: `debugger_set_breakpoint`
   ```json
   {
     "sessionId": "xyz789...",
     "source": "/home/user/payment/processor.js",
     "line": 67
   }
   ```
   (Breakpoint inside `processPayment()`)

10. **AI triggers endpoint again → Breakpoint hit**

11. **AI → MCP Tool Call**: `debugger_step_over` (multiple times)
    - Steps through `processPayment()` function

12. **AI observes**:
    - Loop making 100 separate HTTP calls to payment API
    - Each call takes ~40ms
    - Total: 100 * 40ms = 4000ms

13. **AI → User**: "The bottleneck is in `processPayment()`. It's making 100 sequential HTTP calls to the payment API (one per item in the cart). This takes 4 seconds.\n\n**Solution**: Batch the payment API calls or use the bulk endpoint `/api/payments/batch`. This will reduce the time to ~50ms for a single batched request."

**Value Delivered**:
- Pinpointed exact bottleneck
- Explained root cause
- Suggested concrete optimization

---

### 8.4 Use Case 4: Verify Bug Fix

**Scenario**: User fixed a bug and wants AI to verify it's actually fixed.

**User Journey**:

1. **User**: "I fixed the bug where the shopping cart total was wrong. Can you verify it's fixed?"

2. **AI Agent**:
   - Strategy: Run test case under debugger
   - Set breakpoints at cart total calculation
   - Verify values are correct

3. **AI → MCP Tool Call**: `debugger_start`
   ```json
   {
     "mode": "launch",
     "language": "python",
     "program": "/home/user/test_cart.py"
   }
   ```

4. **AI → MCP Tool Call**: `debugger_set_breakpoint`
   ```json
   {
     "sessionId": "test123...",
     "source": "/home/user/cart.py",
     "line": 89,
     "condition": "len(items) > 0"
   }
   ```
   (Line 89 is where total is calculated)

5. **AI → MCP Tool Call**: `debugger_continue`

6. **Breakpoint hit**

7. **AI → MCP Resource Read**: `debugger://sessions/test123/frames/5000/variables`
   - Sees `items = [{'price': 10.00, 'qty': 2}, {'price': 5.00, 'qty': 1}]`

8. **AI → MCP Tool Call**: `debugger_evaluate`
   ```json
   {
     "sessionId": "test123...",
     "expression": "sum(item['price'] * item['qty'] for item in items)",
     "frameId": 5000
   }
   ```
   - Result: `25.00`

9. **AI → MCP Tool Call**: `debugger_step_over` (to execute total calculation)

10. **AI → MCP Tool Call**: `debugger_evaluate`
    ```json
    {
      "expression": "total"
    }
    ```
    - Result: `25.00` ✓

11. **AI → MCP Tool Call**: `debugger_continue` (run test to completion)

12. **Program exits successfully**

13. **AI → User**: "✓ Verified! The cart total is now calculated correctly. For a cart with:\n- 2x item at $10.00\n- 1x item at $5.00\n\nThe total is correctly computed as $25.00. The test passes."

**Value Delivered**:
- Automated verification
- Inspected actual values during execution
- Confidence in the fix

---

### 8.5 Use Case 5: Multi-Language Debugging (Microservices)

**Scenario**: User has a microservices app with Python backend and Node.js frontend. Frontend calls backend and gets an error.

**User Journey**:

1. **User**: "When I click the submit button, I get a 500 error. Can you debug this?"

2. **AI Agent**:
   - Strategy: Debug both services simultaneously
   - Start debugger for backend (Python)
   - Start debugger for frontend (Node.js)
   - Set breakpoints in API call chain

3. **AI → MCP Tool Call**: `debugger_start` (Backend)
   ```json
   {
     "mode": "launch",
     "language": "python",
     "program": "/home/user/backend/app.py",
     "env": {"FLASK_ENV": "development"}
   }
   ```
   - Session ID: `backend-session-123`

4. **AI → MCP Tool Call**: `debugger_start` (Frontend)
   ```json
   {
     "mode": "launch",
     "language": "javascript",
     "program": "/home/user/frontend/server.js"
   }
   ```
   - Session ID: `frontend-session-456`

5. **AI → MCP Tool Call**: `debugger_set_breakpoint` (Frontend)
   ```json
   {
     "sessionId": "frontend-session-456",
     "source": "/home/user/frontend/api.js",
     "line": 34
   }
   ```
   (Line 34: API call to backend)

6. **AI → MCP Tool Call**: `debugger_set_breakpoint` (Backend)
   ```json
   {
     "sessionId": "backend-session-123",
     "source": "/home/user/backend/routes.py",
     "line": 67
   }
   ```
   (Line 67: API endpoint handler)

7. **AI instructs user**: "Click the submit button now."

8. **Frontend breakpoint hits**

9. **AI → MCP Tool Call**: `debugger_evaluate` (Frontend)
   ```json
   {
     "sessionId": "frontend-session-456",
     "expression": "requestData"
   }
   ```
   - Sees request payload: `{"userId": null, "data": "..."}`
   - Notes: `userId` is `null` ← Suspicious

10. **AI → MCP Tool Call**: `debugger_continue` (Frontend)

11. **Backend breakpoint hits**

12. **AI → MCP Resource Read**: `debugger://sessions/backend-session-123/frames/7000/variables`
    - Sees `user_id` parameter is `None`

13. **AI → MCP Tool Call**: `debugger_step_over` (Backend, multiple times)
    - Steps through validation logic

14. **AI observes**:
    - Code does: `user = User.query.get(user_id)`
    - When `user_id` is None, `user` is None
    - Next line: `user.name` → AttributeError (None has no attribute 'name')

15. **AI → User**: "Found it! The frontend is sending `userId: null` in the request. The backend doesn't validate this and tries to access `user.name`, causing a crash.\n\n**Frontend fix**: Ensure userId is set before sending request (check authentication state).\n\n**Backend fix**: Add validation: `if not user_id: return error(400, 'User ID required')`."

**Value Delivered**:
- Debugged across language boundaries
- Traced issue from frontend → backend
- Identified root cause in frontend AND missing validation in backend
- **Demonstrates multi-language abstraction working seamlessly**

---

### 8.6 Common User Journey Patterns

All use cases follow similar patterns:

1. **Start debugging session** (`debugger_start`)
2. **Set strategic breakpoints** (entry points, suspected bugs, before crashes)
3. **Run program** (`debugger_continue`)
4. **When paused**, inspect state:
   - Read stack trace (understand call hierarchy)
   - Read variables (validate assumptions)
   - Evaluate expressions (test hypotheses)
5. **Step through code** (understand execution flow)
6. **Analyze findings** (AI correlates observations)
7. **Report to user** (explain issue + suggest fix)
8. **Clean up** (`debugger_stop`)

**AI Advantages with Debugging Access**:
- **Autonomous investigation**: No back-and-forth with user
- **Comprehensive analysis**: Check all variables, not just what user thinks to check
- **Pattern recognition**: AI knows common bug patterns and can verify them
- **Multi-step reasoning**: Connect cause and effect across function calls
- **Code + Runtime correlation**: Understand what code *actually does* vs. what it *should do*

---

## 9. Implementation Plan

### 9.1 Phase 1: MVP (Weeks 1-4)

**Goal**: Basic Python debugging support with core MCP integration

**Deliverables**:
- [ ] MCP server skeleton with STDIO transport
- [ ] DAP client implementation (basic requests/responses)
- [ ] Python adapter configuration (debugpy)
- [ ] Session lifecycle management
- [ ] Core MCP tools:
  - `debugger_start` (launch mode only)
  - `debugger_stop`
  - `debugger_continue`
  - `debugger_pause`
  - `debugger_set_breakpoint` (source breakpoints only)
  - `debugger_evaluate`
- [ ] Core MCP resources:
  - `debugger://sessions` (list)
  - `debugger://sessions/{id}` (detail)
- [ ] Basic error handling
- [ ] Logging and diagnostics

**Success Criteria**:
- Can launch Python script under debugger
- Can set breakpoint and pause execution
- Can inspect variables
- Can evaluate expressions
- Works with Claude Desktop

**Testing**:
- Unit tests for each component
- Integration test: Debug sample Python script
- Manual test with Claude Desktop

---

### 9.2 Phase 2: Multi-Language Support (Weeks 5-8)

**Goal**: Expand to Node.js, Go, and Rust

**Deliverables**:
- [ ] Adapter configurations:
  - Node.js (vscode-node-debug2 or inspector protocol)
  - Go (Delve)
  - Rust (CodeLLDB)
- [ ] Launch template system
- [ ] Capability negotiation
- [ ] Enhanced MCP tools:
  - `debugger_step_over`
  - `debugger_step_into`
  - `debugger_step_out`
  - `debugger_remove_breakpoint`
  - `debugger_get_variables`
- [ ] Enhanced MCP resources:
  - `debugger://breakpoints`
  - `debugger://sessions/{id}/stackTrace`
  - `debugger://sessions/{id}/frames/{frameId}/variables`
- [ ] Attach mode support

**Success Criteria**:
- All 4 languages (Python, Node.js, Go, Rust) work
- Can switch between languages seamlessly
- Adapters managed correctly (spawning, cleanup)

**Testing**:
- Test suite for each language
- Cross-language integration test (e.g., debug Python calling Node.js service)

---

### 9.3 Phase 3: Advanced Features (Weeks 9-12)

**Goal**: Production hardening and advanced debugging features

**Deliverables**:
- [ ] Conditional breakpoints
- [ ] Logpoints
- [ ] Exception breakpoints
- [ ] Multi-threaded debugging
- [ ] Stack trace pagination
- [ ] Variable lazy loading
- [ ] Expression modification (`setExpression`)
- [ ] Adapter process recovery (auto-restart on crash)
- [ ] Session persistence (survive server restart)
- [ ] Performance optimization:
  - Connection pooling for adapters
  - Response caching
  - Batched requests
- [ ] Comprehensive error messages for AI
- [ ] Security hardening:
  - Input validation
  - Resource limits (max sessions, max breakpoints)
  - Timeout enforcement
- [ ] Documentation:
  - API documentation
  - User guide for AI agents
  - Adapter development guide

**Success Criteria**:
- Handles edge cases gracefully
- Performance: <50ms tool latency, 100+ concurrent sessions
- No memory leaks
- Comprehensive test coverage (>80%)

**Testing**:
- Load testing (stress with many concurrent sessions)
- Fault injection (adapter crashes, network issues)
- Security testing (malicious inputs)

---

### 9.4 Phase 4: Community & Ecosystem (Weeks 13+)

**Goal**: Open source release and community adoption

**Deliverables**:
- [ ] Open source repository
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Package distribution (crates.io, Docker Hub)
- [ ] Plugin API for custom adapters
- [ ] Example custom adapter (e.g., PHP, Ruby)
- [ ] VS Code extension for configuration
- [ ] Claude Code integration guide
- [ ] Community documentation
- [ ] Example AI agent workflows

**Success Criteria**:
- 5+ community-contributed adapters
- 100+ GitHub stars
- Adopted by at least one major AI coding assistant

---

### 9.5 Development Milestones

| Milestone | Week | Deliverable |
|-----------|------|-------------|
| **M1: Prototype** | Week 2 | Basic MCP server + Python debugging |
| **M2: MVP** | Week 4 | Usable with Claude Desktop (Python only) |
| **M3: Multi-Language** | Week 8 | Python, Node.js, Go, Rust support |
| **M4: Production Alpha** | Week 12 | Advanced features, performance, security |
| **M5: Beta Release** | Week 16 | Open source, documentation, community |
| **M6: v1.0** | Week 20 | Stable release, package distribution |

---

## 10. Risk Assessment and Mitigation

### 10.1 Technical Risks

#### Risk 1: DAP Adapter Incompatibilities

**Risk**: Debug adapters may not fully comply with DAP spec or have quirks.

**Likelihood**: Medium
**Impact**: High (could break language support)

**Mitigation**:
- Test with real adapters early (not just spec)
- Build adapter abstraction layer with quirk handling
- Maintain adapter-specific workarounds
- Contribute fixes upstream to adapter projects

---

#### Risk 2: Performance Bottlenecks

**Risk**: Synchronous operations (process spawning, DAP requests) block server.

**Likelihood**: Medium
**Impact**: Medium (slow response times)

**Mitigation**:
- Async-first architecture (Tokio throughout)
- Benchmark early and often
- Implement connection pooling
- Use caching for expensive operations (stack traces, variable lookups)
- Set performance SLOs (e.g., <50ms P95 latency)

---

#### Risk 3: State Management Complexity

**Risk**: Managing stateful debug sessions with concurrent access is complex.

**Likelihood**: High
**Impact**: High (bugs, race conditions)

**Mitigation**:
- Use well-tested patterns (Arc + RwLock)
- Actor model for session isolation
- Comprehensive concurrency tests
- Fuzz testing for race conditions
- Clear state machine documentation

---

#### Risk 4: Process Management Failures

**Risk**: Debugger processes crash, hang, or leak resources.

**Likelihood**: Medium
**Impact**: High (broken debugging sessions)

**Mitigation**:
- Process monitoring and health checks
- Automatic restart on crash
- Timeouts for all operations
- Resource limits (max processes, memory)
- Graceful degradation (return error vs. crash server)

---

### 10.2 Ecosystem Risks

#### Risk 5: Debugger Adapter Availability

**Risk**: Some languages lack quality DAP adapters.

**Likelihood**: Low (most popular languages covered)
**Impact**: Medium (limits language support)

**Mitigation**:
- Focus on top 10 languages first (Python, JavaScript, Go, Rust, Java, C++, C#, Ruby, PHP, TypeScript)
- Provide adapter development guide
- Contribute to existing adapter projects
- Support both DAP and legacy protocols (e.g., GDB/MI)

---

#### Risk 6: MCP Specification Changes

**Risk**: MCP spec evolves, breaking compatibility.

**Likelihood**: Low (spec is stabilizing)
**Impact**: Medium (requires updates)

**Mitigation**:
- Use official MCP SDK (handles spec changes)
- Version MCP interface
- Monitor MCP spec repository
- Participate in MCP community

---

### 10.3 Adoption Risks

#### Risk 7: AI Agent Integration Complexity

**Risk**: AI agents struggle to use debugging tools effectively.

**Likelihood**: Medium
**Impact**: High (poor user experience)

**Mitigation**:
- Design tools for AI (clear names, structured errors)
- Provide example workflows in documentation
- Build MCP prompts for common debugging tasks
- Work with AI agent developers for feedback
- Iterate based on real usage

---

#### Risk 8: Limited User Demand

**Risk**: Developers don't need/want AI-assisted debugging.

**Likelihood**: Low (clear value proposition)
**Impact**: High (project failure)

**Mitigation**:
- Validate with user interviews
- Build MVP quickly for feedback
- Showcase compelling use cases
- Integrate with popular tools (Claude Desktop, VS Code)
- Open source for community adoption

---

### 10.4 Security Risks

#### Risk 9: Arbitrary Code Execution

**Risk**: Malicious AI agent uses debugger to execute arbitrary code.

**Likelihood**: Low (AI agents generally benign)
**Impact**: Critical (system compromise)

**Mitigation**:
- Input validation (sanitize file paths, expressions)
- Sandboxing (run debuggers in containers)
- Access controls (limit which programs can be debugged)
- Audit logging (track all debugging operations)
- Rate limiting (prevent abuse)

---

#### Risk 10: Information Disclosure

**Risk**: Debugger exposes sensitive data (secrets, PII) to AI agent.

**Likelihood**: Medium
**Impact**: High (privacy/security breach)

**Mitigation**:
- Warning messages when debugging production code
- Variable filtering (redact known secret patterns)
- User consent for debugging sessions
- Opt-in for sensitive operations (memory inspection)
- Clear documentation of privacy implications

---

## 11. Future Enhancements

### 11.1 Near-Term Enhancements (3-6 months)

1. **Remote Debugging Support**
   - Debug programs running on remote servers
   - SSH tunnel integration
   - Kubernetes pod debugging

2. **Time-Travel Debugging**
   - Record execution history
   - Step backward through code
   - Replay debugging sessions
   - Requires adapters with `supportsStepBack` capability (e.g., rr, gdb with reverse execution)

3. **Collaborative Debugging**
   - Multiple AI agents debugging same session
   - Shared breakpoints and watches
   - Real-time state synchronization

4. **Enhanced Visualization**
   - Export debug sessions as interactive HTML
   - Visual call graphs
   - Variable timeline views

5. **Smart Breakpoints**
   - AI suggests breakpoint locations based on code analysis
   - Adaptive breakpoints (move if line changed)
   - Contextual breakpoints (break when in specific call stack)

---

### 11.2 Long-Term Enhancements (6-12 months)

1. **Record & Replay**
   - Record full program execution
   - Deterministic replay
   - Share recordings for debugging

2. **Distributed Debugging**
   - Debug microservices as a system
   - Cross-service breakpoints
   - Distributed tracing integration

3. **Machine Learning Integration**
   - Anomaly detection (unusual variable values)
   - Bug prediction (likely error locations)
   - Pattern recognition (similar bugs in other code)

4. **IDE Integration**
   - VS Code extension
   - JetBrains plugin
   - Vim/Neovim integration

5. **Cloud Debugging**
   - Debug serverless functions (AWS Lambda, Google Cloud Functions)
   - Debug containers (Docker, Kubernetes)
   - Debug cloud services

---

### 11.3 Research Directions

1. **LLM-Optimized Debugging**
   - Compression of debug state for token efficiency
   - Summarization of large variable dumps
   - Natural language queries ("why is x null?")

2. **Automated Root Cause Analysis**
   - AI automatically sets breakpoints and steps through code
   - Reports findings without human intervention
   - Proposes and tests fixes

3. **Debugging as Code**
   - Version control debugging scripts
   - Automated regression debugging
   - CI/CD integration (debug failed tests)

4. **Secure Multi-Tenant Debugging**
   - Debug in production without exposing data
   - Privacy-preserving debugging
   - Compliance (GDPR, HIPAA)

---

## 12. Conclusion

### 12.1 Summary

This proposal outlines a **comprehensive architecture for a DAP-based MCP debugging server** that enables AI coding agents to programmatically debug applications across multiple programming languages.

**Key Innovations**:
1. **First-of-its-kind** MCP server specifically designed for AI-assisted debugging
2. **Language-agnostic** interface leveraging proven Debug Adapter Protocol
3. **Production-ready** Rust implementation with Tokio async runtime
4. **Extensible** architecture supporting easy addition of new language debuggers
5. **AI-optimized** tools and resources designed for LLM consumption

**Feasibility**: **HIGH**
- Built on proven technologies (DAP, MCP, Rust, Tokio)
- Clear component boundaries and interfaces
- Incremental implementation path (MVP in 4 weeks)
- Managed risks with concrete mitigation strategies

**Impact**: **HIGH**
- Unlocks new category of AI-assisted development (autonomous debugging)
- Applicable to billions of lines of existing code
- Reduces developer time spent on debugging (40-50% of dev time)
- Educational value (AI explains code by stepping through it)

---

### 12.2 Recommended Next Steps

**Immediate (Week 1)**:
1. Set up Rust development environment
2. Create project structure
3. Implement basic MCP server skeleton
4. Test STDIO transport with Claude Desktop

**Short-term (Weeks 2-4)**:
1. Implement DAP client (basic requests)
2. Integrate debugpy (Python adapter)
3. Build session manager
4. Implement core MCP tools and resources
5. End-to-end testing with sample Python script

**Medium-term (Weeks 5-12)**:
1. Add language support (Node.js, Go, Rust)
2. Implement advanced debugging features
3. Performance testing and optimization
4. Security hardening
5. Documentation

**Long-term (Weeks 13+)**:
1. Open source release
2. Community building
3. Ecosystem development (plugins, integrations)
4. Research projects (LLM-optimized debugging)

---

### 12.3 Success Metrics

**Technical Metrics**:
- [ ] Tool call latency P95 < 50ms
- [ ] Support 100+ concurrent debug sessions
- [ ] Test coverage > 80%
- [ ] Zero critical security vulnerabilities
- [ ] 99.9% uptime in production

**Adoption Metrics**:
- [ ] Integrated with Claude Desktop
- [ ] 1000+ active users within 6 months
- [ ] 10+ community-contributed language adapters
- [ ] 5+ production deployments

**Impact Metrics**:
- [ ] Reduce debugging time by 30% (user surveys)
- [ ] 90% user satisfaction rating
- [ ] Featured in AI coding assistant documentation
- [ ] Cited in academic research on AI-assisted development

---

### 12.4 Final Remarks

The convergence of **AI coding agents** and **standardized debugging protocols** creates a unique opportunity to fundamentally change how developers debug code. By exposing the Debug Adapter Protocol through the Model Context Protocol, we can enable AI agents to autonomously investigate bugs, explain code execution, and verify fixes—tasks that currently require significant human effort.

This project is **technically feasible** (built on proven standards), **practically useful** (addresses real pain points), and **strategically positioned** (ahead of the curve in AI-assisted development).

**The time to build this is now.**

---

## Appendices

### Appendix A: Technology Reference Links

- **Debug Adapter Protocol**: https://microsoft.github.io/debug-adapter-protocol/
- **DAP GitHub**: https://github.com/microsoft/debug-adapter-protocol
- **Model Context Protocol**: https://spec.modelcontextprotocol.io/
- **MCP Rust SDK**: https://github.com/modelcontextprotocol/rust-sdk
- **Tokio**: https://tokio.rs/
- **nvim-dap**: https://github.com/mfussenegger/nvim-dap
- **dap-mode**: https://github.com/emacs-lsp/dap-mode

### Appendix B: Debug Adapter Implementations

| Language | Adapter | Repository |
|----------|---------|------------|
| Python | debugpy | https://github.com/microsoft/debugpy |
| JavaScript/Node.js | js-debug | https://github.com/microsoft/vscode-js-debug |
| Go | Delve | https://github.com/go-delve/delve |
| Rust | CodeLLDB | https://github.com/vadimcn/vscode-lldb |
| C/C++ | cpptools | https://github.com/microsoft/vscode-cpptools |
| Java | java-debug | https://github.com/microsoft/java-debug |
| C# | netcoredbg | https://github.com/Samsung/netcoredbg |
| Ruby | rdbg | https://github.com/ruby/debug |
| PHP | vscode-php-debug | https://github.com/xdebug/vscode-php-debug |

### Appendix C: Glossary

- **DAP**: Debug Adapter Protocol - JSON-RPC protocol for debuggers
- **MCP**: Model Context Protocol - Protocol for AI agent capabilities
- **Debug Adapter**: Intermediary process translating DAP to debugger commands
- **Debug Session**: Stateful debugging instance from start to termination
- **Breakpoint**: Marker that pauses execution at a specific location
- **Stack Frame**: Function call context in the call stack
- **Scope**: Variable namespace (locals, arguments, globals)
- **Variable Reference**: Opaque handle to a variable for lazy loading
- **Logpoint**: Breakpoint that logs a message instead of pausing
- **Stepping**: Controlled execution (step over, step into, step out)
- **REPL**: Read-Eval-Print Loop for expression evaluation

---

**End of Proposal**
