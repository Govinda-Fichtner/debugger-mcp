# Go vs Java Debug Adapter Comparison

**Research Date**: October 8, 2025
**Purpose**: Determine which language to add next to debugger-mcp

## Executive Summary

**Recommendation**: **Add Go (Delve) support next**

**Key Reasons**:
1. **Simpler Architecture**: Direct DAP support, no intermediate layers
2. **Easier Installation**: Single command (`go install`)
3. **Existing Patterns**: Matches Ruby/Node.js TCP Socket pattern exactly
4. **Lower Risk**: Independent process, no language server required
5. **Faster Implementation**: Estimated 2-3 days vs 1-2 weeks for Java

---

## Detailed Comparison Matrix

### 1. Architecture Complexity

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Debug Protocol** | DAP (native) | DAP → JDWP translation | **Go** |
| **Components** | 1 (dlv dap) | 3 (jdt.ls + java-debug + JDWP) | **Go** |
| **Process Model** | Single process | Multi-process coordination | **Go** |
| **Launch Complexity** | Direct spawn | LSP command callback | **Go** |
| **Protocol Layers** | 1 layer (DAP) | 2 layers (DAP + JDWP) | **Go** |

**Details**:

**Go (Delve)**:
```
AI Agent → MCP → debugger-mcp → dlv dap → Go program
                                  (DAP)
```

**Java**:
```
AI Agent → MCP → debugger-mcp → jdt.ls → java-debug → JDWP → JVM
                                  (LSP)     (DAP)      (JDWP)
```

---

### 2. Installation Requirements

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Installation Command** | `go install github.com/go-delve/delve/cmd/dlv@latest` | Complex multi-step | **Go** |
| **Dependencies** | None (assuming Go installed) | Eclipse JDT LS, Java 21+, Maven | **Go** |
| **Binary Size** | ~30 MB | ~100+ MB (with jdt.ls) | **Go** |
| **Setup Time** | < 30 seconds | Several minutes | **Go** |
| **User Friction** | Very low | High | **Go** |

**Java Installation Steps**:
1. Install Java 21+ runtime
2. Download/build Eclipse JDT LS
3. Build java-debug with Maven
4. Configure bundles paths
5. Verify language server works

---

### 3. Transport Mechanism

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Transport Type** | TCP Socket | TCP Socket | **Tie** |
| **Port Allocation** | Standard (find_free_port) | Dynamic (LSP callback) | **Go** |
| **Connection** | Direct connect | Wait for LSP response | **Go** |
| **Existing Code Reuse** | 100% (socket_helper) | Partial | **Go** |

**Go Launch Pattern** (matches Ruby/Node.js exactly):
```rust
// 1. Find free port
let port = socket_helper::find_free_port()?;

// 2. Spawn adapter
let child = Command::new("dlv")
    .args(&["dap", "--listen", &format!("127.0.0.1:{}", port)])
    .spawn()?;

// 3. Connect with retry
let socket = socket_helper::connect_with_retry(port, Duration::from_secs(2)).await?;
```

**Java Launch Pattern** (requires new architecture):
```rust
// 1. Check if jdt.ls is running (or spawn it)
// 2. Send LSP command: vscode.java.startDebugSession
// 3. Wait for callback with port number
// 4. Connect to returned port
// Complex state management needed
```

---

### 4. Launch Configuration

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Config Complexity** | Simple | Complex | **Go** |
| **Required Fields** | program, args | mainClass, classpath, vmArgs | **Go** |
| **File Reference** | Direct path | Class name resolution | **Go** |
| **Build Requirements** | None (binary or source) | Project compilation | **Go** |

**Go Launch Args**:
```rust
pub fn launch_args(program: &str, args: &[String], stop_on_entry: bool) -> Value {
    json!({
        "type": "go",
        "request": "launch",
        "mode": "debug",
        "program": program,  // Simple: just the .go file or binary
        "args": args,
        "stopOnEntry": stop_on_entry
    })
}
```

**Java Launch Args**:
```rust
pub fn launch_args(main_class: &str, args: &[String], stop_on_entry: bool) -> Value {
    json!({
        "type": "java",
        "request": "launch",
        "mainClass": main_class,  // Complex: needs class name, not file path
        "args": args,
        "vmArgs": [],  // Additional JVM configuration
        "classpath": [],  // Requires classpath resolution
        "projectName": "",  // Project context needed
        "console": "integratedTerminal"
    })
}
```

---

### 5. Language-Specific Features

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Concurrency Debugging** | Goroutines (native) | Threads (JVM) | Depends on use case |
| **Special Features** | `hideSystemGoroutines` | Multi-threaded, hot reload | Different strengths |
| **Standard Library** | Well supported | Well supported | **Tie** |
| **Runtime Complexity** | Low (compiled) | High (JVM) | **Go** |

**Go-Specific**:
- Goroutine inspection and switching
- Channel debugging
- Simple compiled binary execution
- No VM overhead

**Java-Specific**:
- JVM thread inspection
- Hot code reloading (HCR)
- Complex classpath and module system
- Heap and GC debugging

---

### 6. Existing Code Similarity

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Similar Adapter** | Ruby (TCP Socket) | None | **Go** |
| **Code Reuse %** | ~80% | ~40% | **Go** |
| **socket_helper.rs** | Full reuse | Partial reuse | **Go** |
| **Adapter Pattern** | Standard trait impl | Needs new abstraction | **Go** |

**Go Implementation** (based on Ruby adapter):
```rust
pub struct GoAdapter;

impl DebugAdapterLogger for GoAdapter {
    fn language_name(&self) -> &str { "Go" }
    fn language_emoji(&self) -> &str { "🐹" }
    fn transport_type(&self) -> &str { "TCP Socket" }
    fn adapter_id(&self) -> &str { "delve" }

    fn command_line(&self) -> String {
        format!("dlv dap --listen=127.0.0.1:{}", self.port)
    }

    // Standard error logging implementations...
}

pub async fn spawn(program: &str, args: &[String]) -> Result<GoDebugSession> {
    // 1. Find port (reuse socket_helper)
    let port = socket_helper::find_free_port()?;

    // 2. Build command
    let mut cmd = Command::new("dlv");
    cmd.args(&["dap", "--listen", &format!("127.0.0.1:{}", port)]);

    // 3. Spawn process
    let child = cmd.spawn()?;

    // 4. Connect (reuse socket_helper)
    let socket = socket_helper::connect_with_retry(port, Duration::from_secs(2)).await?;

    Ok(GoDebugSession { process: child, socket, port })
}
```

**Java Implementation** (requires new infrastructure):
```rust
// NEW: JDT Language Server manager
pub struct JdtLsManager {
    process: Option<Child>,
    lsp_client: LspClient,  // NEW: LSP client implementation needed
}

// NEW: LSP client for communication
pub struct LspClient {
    // JSON-RPC over STDIO or socket
    // Request/response correlation
    // Command execution
}

pub struct JavaAdapter {
    jdtls_manager: Arc<JdtLsManager>,  // Shared state
}

impl DebugAdapterLogger for JavaAdapter {
    // Standard implementations, but more complex
}

pub async fn spawn(main_class: &str, args: &[String]) -> Result<JavaDebugSession> {
    // 1. Ensure jdt.ls is running
    let jdtls = ensure_jdtls_running().await?;

    // 2. Send LSP command: vscode.java.startDebugSession
    let response = jdtls.execute_command("vscode.java.startDebugSession").await?;

    // 3. Extract port from response
    let port = response.get("port").as_u64()? as u16;

    // 4. Connect to port
    let socket = socket_helper::connect_with_retry(port, Duration::from_secs(5)).await?;

    Ok(JavaDebugSession {
        jdtls_manager: jdtls,
        socket,
        port
    })
}
```

---

### 7. Testing Strategy

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Test Complexity** | Low | High | **Go** |
| **Setup Requirements** | Install dlv | Install Java 21+, jdt.ls, java-debug | **Go** |
| **FizzBuzz Test** | Direct adaptation | Requires project setup | **Go** |
| **CI Integration** | Simple | Complex | **Go** |

**Go Test Plan** (simple):
```bash
# 1. Install delve
go install github.com/go-delve/delve/cmd/dlv@latest

# 2. Create test file
echo 'package main
func fizzbuzz(n int) string {
    if n%15 == 0 { return "FizzBuzz" }
    if n%3 == 0 { return "Fizz" }
    if n%5 == 0 { return "Buzz" }
    return fmt.Sprintf("%d", n)
}
func main() {
    for i := 1; i <= 100; i++ {
        fmt.Println(fizzbuzz(i))
    }
}' > fizzbuzz.go

# 3. Run test
cargo test test_go_adapter
```

**Java Test Plan** (complex):
```bash
# 1. Install Java 21+
# 2. Download/build Eclipse JDT LS
# 3. Build java-debug
# 4. Configure paths
# 5. Create Maven/Gradle project structure
# 6. Write FizzBuzz.java with proper package
# 7. Ensure classpath is set
# 8. Run test
cargo test test_java_adapter
```

---

### 8. Known Issues and Workarounds

| Aspect | Go (Delve) | Java (java-debug) | Winner |
|--------|------------|-------------------|---------|
| **Known Workarounds** | None identified | Classpath issues common | **Go** |
| **Community Maturity** | Mature, stable | Mature but complex | **Go** |
| **Documentation** | Excellent | Scattered | **Go** |
| **Maintenance Burden** | Low | Medium-High | **Go** |

**Go (Delve)**:
- ✅ Clean DAP implementation
- ✅ Well-documented
- ✅ Active development
- ⚠️ Single-use server (but this is fine, same as Ruby)

**Java (java-debug)**:
- ⚠️ Requires running language server
- ⚠️ Classpath resolution can be tricky
- ⚠️ Security vulnerability history (CVE-2023-20863)
- ⚠️ Complex dependency management

---

### 9. Implementation Time Estimate

| Phase | Go (Delve) | Java (java-debug) |
|-------|------------|-------------------|
| **Adapter Implementation** | 4 hours | 16 hours |
| **Testing** | 4 hours | 12 hours |
| **Documentation** | 2 hours | 6 hours |
| **Debugging Issues** | 2 hours | 10 hours |
| **TOTAL** | **12 hours (1.5 days)** | **44 hours (5.5 days)** |

**Go Implementation Breakdown**:
1. Create `src/adapters/golang.rs` - 2 hours
2. Implement `DebugAdapterLogger` trait - 1 hour
3. Implement `spawn()` function (copy from Ruby) - 1 hour
4. Create `launch_args()` - 30 min
5. Add tests - 2 hours
6. Add integration test - 2 hours
7. Update documentation - 1 hour
8. Manual testing and bug fixes - 2.5 hours

**Java Implementation Breakdown**:
1. Research jdt.ls integration - 4 hours
2. Implement LSP client - 8 hours
3. Create JdtLsManager - 4 hours
4. Create `src/adapters/java.rs` - 4 hours
5. Implement complex launch logic - 4 hours
6. Add tests - 6 hours
7. Add integration test (with project setup) - 6 hours
8. Update documentation - 3 hours
9. Manual testing and bug fixes - 5 hours

---

### 10. Risk Assessment

| Risk Category | Go (Delve) | Java (java-debug) |
|---------------|------------|-------------------|
| **Technical Risk** | **Low** | **High** |
| **Dependency Risk** | **Low** | **High** |
| **User Adoption Risk** | **Low** | **Medium** |
| **Maintenance Risk** | **Low** | **Medium-High** |

**Go Risks**:
- ✅ Low: Direct DAP support
- ✅ Low: Single dependency (dlv)
- ✅ Low: Simple installation
- ⚠️ Medium: Users must have Go installed

**Java Risks**:
- ⚠️ High: Complex multi-component architecture
- ⚠️ High: Multiple large dependencies
- ⚠️ High: Version compatibility issues (Java 21+, jdt.ls, java-debug)
- ⚠️ Medium: Classpath and project structure complexity
- ⚠️ Medium: Potential security vulnerabilities in dependency chain

---

## Comparative Code Examples

### Spawn Function Complexity

**Go (Simple - ~30 lines)**:
```rust
pub async fn spawn(
    program: &str,
    program_args: &[String],
    stop_on_entry: bool,
) -> Result<GoDebugSession> {
    let adapter = GoAdapter;
    adapter.log_selection();
    adapter.log_transport_init();

    // Find free port
    let port = socket_helper::find_free_port()?;

    // Build command
    let mut args = vec![
        "dap".to_string(),
        "--listen".to_string(),
        format!("127.0.0.1:{}", port),
    ];

    // Spawn process
    adapter.log_spawn_attempt();
    let child = Command::new("dlv")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            adapter.log_spawn_error(&e);
            Error::Process(format!("Failed to spawn dlv: {}", e))
        })?;

    adapter.log_connection_success();

    // Connect to socket
    let socket = socket_helper::connect_with_retry(port, Duration::from_secs(2))
        .await
        .map_err(|e| {
            adapter.log_connection_error(&e);
            e
        })?;

    Ok(GoDebugSession {
        process: child,
        socket,
        port,
    })
}
```

**Java (Complex - ~80+ lines)**:
```rust
pub async fn spawn(
    main_class: &str,
    program_args: &[String],
    workspace: &str,
) -> Result<JavaDebugSession> {
    let adapter = JavaAdapter::new();
    adapter.log_selection();

    // 1. Check if jdt.ls is already running
    let jdtls = if let Some(existing) = JDTLS_INSTANCES.lock().await.get(workspace) {
        existing.clone()
    } else {
        // Start new jdt.ls instance
        let jdtls_config = JdtLsConfig {
            java_home: std::env::var("JAVA_HOME")?,
            workspace_dir: workspace.to_string(),
            jdtls_path: find_jdtls_installation()?,
            bundles: vec![find_java_debug_jar()?],
        };

        let jdtls = spawn_jdtls(jdtls_config).await?;

        // Wait for initialization
        jdtls.wait_for_ready(Duration::from_secs(30)).await?;

        // Cache the instance
        JDTLS_INSTANCES.lock().await.insert(workspace.to_string(), jdtls.clone());
        jdtls
    };

    // 2. Send LSP command to start debug session
    let launch_config = json!({
        "type": "java",
        "request": "launch",
        "mainClass": main_class,
        "args": program_args,
        "projectName": infer_project_name(workspace)?,
    });

    let command = LspCommand {
        command: "vscode.java.startDebugSession".to_string(),
        arguments: vec![launch_config],
    };

    let response = jdtls
        .execute_command(command)
        .await
        .map_err(|e| {
            adapter.log_init_error(&e);
            Error::Process(format!("Failed to start debug session: {}", e))
        })?;

    // 3. Extract port from response
    let port = response
        .get("port")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::Protocol("No port in response".to_string()))?
        as u16;

    // 4. Connect to debug adapter
    adapter.log_connection_success();
    let socket = socket_helper::connect_with_retry(port, Duration::from_secs(5))
        .await
        .map_err(|e| {
            adapter.log_connection_error(&e);
            e
        })?;

    Ok(JavaDebugSession {
        jdtls_manager: jdtls,
        socket,
        port,
        workspace: workspace.to_string(),
    })
}

// Additional helper functions needed:
async fn spawn_jdtls(config: JdtLsConfig) -> Result<Arc<JdtLsManager>> { /* ... */ }
fn find_jdtls_installation() -> Result<String> { /* ... */ }
fn find_java_debug_jar() -> Result<String> { /* ... */ }
fn infer_project_name(workspace: &str) -> Result<String> { /* ... */ }
```

---

## Side-by-Side Feature Matrix

| Feature | Go (Delve) | Java (java-debug) |
|---------|------------|-------------------|
| **DAP Support** | ✅ Native | ✅ Via translation layer |
| **Breakpoints** | ✅ Yes | ✅ Yes |
| **Step Debugging** | ✅ Yes | ✅ Yes |
| **Variable Inspection** | ✅ Yes | ✅ Yes |
| **Exception Breakpoints** | ✅ Panic handling | ✅ Exception handling |
| **Conditional Breakpoints** | ✅ Yes | ✅ Yes |
| **Data Breakpoints** | ❌ No | ⚠️ Limited |
| **Hot Reload** | ❌ No | ✅ Yes (HCR) |
| **Concurrency** | ✅ Goroutines | ✅ Threads |
| **Remote Debugging** | ✅ Yes | ✅ Yes |
| **Attach Mode** | ✅ Yes | ✅ Yes |
| **Multi-session** | ✅ Yes | ✅ Yes |

---

## Recommendation: Add Go First

### Primary Justification

1. **Architectural Simplicity**: Delve's native DAP support eliminates translation layers
2. **Implementation Speed**: 80% code reuse from Ruby adapter = 1-2 days vs 5-6 days
3. **Lower Risk**: Independent process model, no language server dependency
4. **User Experience**: Simple installation (`go install dlv`) vs complex Java setup
5. **Validation**: Proves adapter pattern works for compiled languages before tackling Java

### Strategic Approach

**Phase 1: Go Implementation (Week 1)**
- Day 1-2: Implement Go adapter
- Day 3: Testing and validation
- Day 4: Documentation
- Day 5: Buffer for issues

**Phase 2: Java Implementation (Weeks 2-3)**
- Week 2: Research and prototype LSP client
- Week 3: Full implementation with tests
- Benefit: Lessons learned from Go help with Java

**Alternative (Not Recommended): Java First**
- Risk: Complex first implementation might reveal architecture issues
- Risk: If blocked on Java, no progress on multi-language support
- Risk: User frustration with complex installation

---

## Implementation Checklist

### Go (Delve) - Recommended Next

- [ ] Install delve for testing (`go install github.com/go-delve/delve/cmd/dlv@latest`)
- [ ] Create `src/adapters/golang.rs`
- [ ] Implement `GoAdapter` struct with `DebugAdapterLogger` trait
- [ ] Implement `spawn()` function (copy from Ruby, modify for dlv)
- [ ] Implement `launch_args()` function
- [ ] Create `tests/test_golang_adapter.rs`
- [ ] Create `tests/fixtures/fizzbuzz.go`
- [ ] Add integration test
- [ ] Update `src/adapters/mod.rs` to include golang module
- [ ] Update README with Go support
- [ ] Test with real Go programs

### Java (java-debug) - Future Work

- [ ] Research jdt.ls installation options
- [ ] Design LSP client architecture
- [ ] Implement `src/lsp/client.rs` (new module)
- [ ] Implement `src/lsp/jdtls_manager.rs` (new module)
- [ ] Create `src/adapters/java.rs`
- [ ] Implement complex launch logic
- [ ] Create Maven/Gradle test project structure
- [ ] Create `tests/test_java_adapter.rs`
- [ ] Add extensive documentation for Java setup
- [ ] Consider Docker-based installation for easier setup

---

## Conclusion

**Add Go (Delve) support first** because:

1. **Simplicity wins**: Clean architecture, direct DAP support
2. **Speed matters**: 1-2 days vs 5-6 days implementation time
3. **Risk management**: Lower complexity = fewer things that can go wrong
4. **User experience**: Simple installation vs complex Java setup
5. **Learning opportunity**: Validates adapter pattern for compiled languages
6. **Momentum**: Quick win enables faster progress to Java later

After Go is stable, adding Java support will be more informed and less risky.

---

**Next Steps**: Proceed to Phase 5 (Testing Strategy) for Go implementation validation plan.
