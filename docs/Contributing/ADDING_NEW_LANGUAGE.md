# Adding a New Language to the DAP MCP Server

**Guide Version**: 1.0
**Last Updated**: October 7, 2025
**Based on**: Python (debugpy) and Ruby (rdbg) implementations

---

## Overview

This guide documents the **proven process** for adding a new programming language to the DAP MCP Server, based on successful implementations of Python and Ruby support.

**Time Estimate**: 1-2 days for basic support, 3-5 days for full validation

---

## Prerequisites

Before adding a new language, ensure:

1. ✅ **DAP adapter exists** for the target language
2. ✅ **Adapter is well-maintained** and actively developed
3. ✅ **DAP protocol is implemented** correctly by the adapter
4. ✅ **Installation is straightforward** (package manager available)
5. ✅ **Documentation exists** for the adapter's DAP implementation

### Known DAP Adapters

| Language | Adapter | Status | Package Manager |
|----------|---------|--------|-----------------|
| Python | debugpy | ✅ Validated | `pip install debugpy` |
| Ruby | rdbg (debug gem) | ✅ Validated | `gem install debug` |
| Node.js | inspector protocol | 🔄 Built-in | N/A |
| Go | delve | ⏳ Planned | `go install github.com/go-delve/delve/cmd/dlv@latest` |
| Rust | CodeLLDB | ⏳ Planned | Extension-based |
| Java | java-debug | ⏳ Planned | Extension-based |

---

## Implementation Steps

### Step 1: Research the Debug Adapter (1-2 hours)

**Goal**: Understand how the adapter works and what DAP features it supports.

#### Research Checklist

- [ ] Find official documentation
- [ ] Identify launch modes (launch vs attach)
- [ ] Check transport mechanism (stdio, TCP, pipe)
- [ ] List supported DAP capabilities
- [ ] Find example configurations
- [ ] Test adapter manually (outside MCP)

#### Key Questions

1. **How is the adapter invoked?**
   - Command: `debugpy`, `rdbg`, `node --inspect`, etc.
   - Args: What arguments does it accept?

2. **What transport does it use?**
   - STDIO (debugpy, rdbg socket mode)
   - TCP socket (rdbg, Node.js inspector)
   - Named pipe (rare)

3. **Does it support stopOnEntry?**
   - Native support (debugpy: ✅, Node.js: ✅)
   - Requires workaround (rdbg: ❌, needs entry breakpoint)

4. **What are its limitations?**
   - Known bugs (rdbg pause bug)
   - Missing features
   - Performance characteristics

#### Example Research Notes

**Ruby (rdbg)**:
```markdown
- Adapter: rdbg (debug gem)
- Command: `rdbg`
- Transport: TCP socket (`--open --port <PORT>`)
- stopOnEntry: ❌ Not supported (requires entry breakpoint workaround)
- Pause: ❌ Broken in socket mode (returns success but doesn't pause)
- Breakpoints: ✅ Works reliably
- Launch args: `rdbg --open --port <PORT> [--stop-at-load|--nonstop] <script>`
```

---

### Step 2: Create Adapter Configuration (30 minutes)

**File**: `src/adapters/<language>.rs`

#### 2.1. Create Adapter Struct

```rust
// src/adapters/ruby.rs
use serde_json::{json, Value};
use crate::{Result, Error};

/// Ruby rdbg (debug gem) adapter configuration
pub struct RubyAdapter;

impl RubyAdapter {
    pub fn command() -> String {
        "rdbg".to_string()
    }

    pub fn adapter_id() -> &'static str {
        "rdbg"
    }
}
```

#### 2.2. Implement Launch Configuration

```rust
impl RubyAdapter {
    pub fn launch_args_with_options(
        program: &str,
        args: &[String],
        cwd: Option<&str>,
        stop_on_entry: bool,
    ) -> Value {
        let mut launch = json!({
            "request": "launch",
            "type": "ruby",
            "program": program,
            "args": args,
            "stopOnEntry": stop_on_entry,
        });

        if let Some(cwd_path) = cwd {
            launch["cwd"] = json!(cwd_path);
        }

        launch
    }
}
```

#### 2.3. Add Adapter to Registry

```rust
// src/adapters/mod.rs
pub mod python;
pub mod ruby;
// pub mod nodejs;  // New language

pub fn get_adapter(language: &str) -> Result<Box<dyn Adapter>> {
    match language {
        "python" => Ok(Box::new(python::PythonAdapter)),
        "ruby" => Ok(Box::new(ruby::RubyAdapter)),
        // "nodejs" => Ok(Box::new(nodejs::NodeAdapter)),
        _ => Err(Error::AdapterNotFound(language.into())),
    }
}
```

---

### Step 3: Test Adapter Manually (1 hour)

**Goal**: Verify adapter works outside MCP before integration.

#### 3.1. Install Adapter

```bash
# Python
pip install debugpy

# Ruby
gem install debug

# Node.js (built-in)
node --version

# Go
go install github.com/go-delve/delve/cmd/dlv@latest
```

#### 3.2. Create Test Script

```python
# tests/fixtures/test_<language>.py
def test_function(n):
    if n == 1:
        return "One"
    return str(n)

print(test_function(1))
```

#### 3.3. Test Adapter Directly

**Python (debugpy)**:
```bash
python -m debugpy --listen localhost:5678 --wait-for-client tests/fixtures/test_python.py
# Connect with DAP client...
```

**Ruby (rdbg)**:
```bash
rdbg --open --port 12345 --stop-at-load tests/fixtures/test_ruby.rb
# Connect to localhost:12345...
```

**Node.js**:
```bash
node --inspect-brk=9229 tests/fixtures/test_node.js
# Connect to localhost:9229...
```

---

### Step 4: Implement Transport Layer (2-4 hours)

#### 4.1. Determine Transport Type

**STDIO Transport** (debugpy):
```rust
// Spawn process with STDIO pipes
let child = Command::new("python")
    .args(&["-m", "debugpy", "--connect", "localhost:5678"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
```

**TCP Socket Transport** (rdbg, Node.js):
```rust
// Spawn process, then connect to socket
let child = Command::new("rdbg")
    .args(&["--open", "--port", &port.to_string()])
    .spawn()?;

// Connect to TCP socket
let socket = TcpStream::connect(("localhost", port)).await?;
```

#### 4.2. Handle Connection Timing

**Critical**: Different adapters have different startup times.

```rust
// Helper function with retry logic
pub async fn connect_with_retry(
    port: u16,
    timeout: Duration,
) -> Result<TcpStream> {
    let start = Instant::now();
    let mut attempts = 0;

    loop {
        match TcpStream::connect(("localhost", port)).await {
            Ok(socket) => return Ok(socket),
            Err(_) if start.elapsed() < timeout => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
            Err(e) => return Err(Error::Connection(format!(
                "Failed after {} attempts: {}", attempts, e
            ))),
        }
    }
}
```

---

### Step 5: Handle stopOnEntry (CRITICAL) (2-4 hours)

**This is the most complex and critical step.**

#### 5.1. Check Native Support

Test if adapter honors `stopOnEntry: true`:

```rust
// In test
let launch_args = json!({
    "request": "launch",
    "program": "test.py",
    "stopOnEntry": true
});

// After configurationDone, does it send 'stopped' event?
```

#### 5.2. Implement Workaround if Needed

If adapter doesn't support stopOnEntry (like rdbg), use **entry breakpoint pattern**:

```rust
// In initialize_and_launch()
if adapter_type == Some("ruby") && stop_on_entry {
    info!("🔧 Applying entry breakpoint workaround");

    // 1. Find first executable line
    let entry_line = find_first_executable_line(program_path)?;

    // 2. Set breakpoint BEFORE configurationDone (per DAP spec)
    let source = Source {
        path: Some(program_path.to_string()),
        name: None,
        source_reference: None,
    };

    let breakpoint = SourceBreakpoint {
        line: entry_line as i32,
        column: None,
        condition: None,
        hit_condition: None,
    };

    self.set_breakpoints(source, vec![breakpoint]).await?;

    // 3. NOW send configurationDone (per DAP spec)
    self.configuration_done().await?;
}
```

#### 5.3. Implement First Executable Line Detection

**Key Insight**: Each language has different comment/declaration syntax.

**Ruby**:
```rust
fn find_first_executable_line_ruby(path: &str) -> usize {
    let content = fs::read_to_string(path)?;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty, comments, shebang
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip non-executable declarations
        if trimmed.starts_with("require")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("module ") {
            continue;
        }

        return line_num + 1; // DAP uses 1-indexed
    }

    1 // Fallback
}
```

**Python**:
```rust
fn find_first_executable_line_python(path: &str) -> usize {
    let content = fs::read_to_string(path)?;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty, comments, docstrings
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("\"\"\"")
            || trimmed.starts_with("'''") {
            continue;
        }

        // Skip imports
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            continue;
        }

        // Skip class/function definitions
        if trimmed.starts_with("class ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("async def ") {
            continue;
        }

        return line_num + 1;
    }

    1 // Fallback
}
```

---

### Step 6: Implement DAP Sequence (2-3 hours)

**Critical**: Follow the correct DAP sequence!

#### Correct DAP Sequence (from specification)

```
1. → initialize request
2. ← initialize response
3. ← initialized event
4. → setBreakpoints (if stopOnEntry or user breakpoints)  ← BEFORE configurationDone!
5. → setExceptionBreakpoints (optional)
6. → configurationDone  ← AFTER breakpoints!
7. ← stopped event (if stopOnEntry or breakpoint hit)
8. [debugging session continues...]
```

#### Implementation in initialize_and_launch()

```rust
pub async fn initialize_and_launch(
    &self,
    adapter_id: &str,
    launch_args: Value,
    adapter_type: Option<&str>,
) -> Result<()> {
    // 1. Send initialize
    self.send_initialize(adapter_id).await?;

    // 2. Wait for initialized event
    self.wait_for_event("initialized", Duration::from_secs(5)).await?;

    // 3. Apply workarounds BEFORE configurationDone
    let needs_workaround = adapter_type == Some("ruby")
        && launch_args.get("stopOnEntry").and_then(|v| v.as_bool()).unwrap_or(false);

    if needs_workaround {
        // Set entry breakpoint
        let program_path = launch_args["program"].as_str().unwrap();
        let entry_line = find_first_executable_line(program_path, adapter_type)?;

        let source = Source { path: Some(program_path.into()), .. };
        let bp = SourceBreakpoint { line: entry_line as i32, .. };

        self.set_breakpoints(source, vec![bp]).await?;
    }

    // 4. Send launch request
    self.send_launch(launch_args).await?;

    // 5. Send configurationDone (AFTER breakpoints)
    self.configuration_done().await?;

    Ok(())
}
```

---

### Step 7: Create Docker Image (1 hour)

**File**: `Dockerfile.<language>`

```dockerfile
# Dockerfile.ruby
FROM rust:1.70-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM ruby:3.2-alpine
RUN apk add --no-cache bash
RUN gem install debug --no-document

COPY --from=builder /build/target/release/debugger_mcp /usr/local/bin/

ENTRYPOINT ["debugger_mcp", "serve"]
```

**Build and test**:
```bash
docker build -f Dockerfile.ruby -t debugger-mcp:ruby .
docker run -i debugger-mcp:ruby
```

---

### Step 8: Write Integration Tests (3-4 hours)

#### 8.1. Create Test Fixture

```python
# tests/fixtures/fizzbuzz_<language>.py
def fizzbuzz(n):
    if n % 15 == 0:
        return "FizzBuzz"
    elif n % 3 == 0:
        return "Fizz"
    elif n % 5 == 0:  # BUG: Change to % 4 for testing
        return "Buzz"
    else:
        return str(n)

for i in range(1, 101):
    print(fizzbuzz(i))
```

#### 8.2. Create Integration Test

```rust
#[tokio::test]
async fn test_<language>_debugging_workflow() {
    // 1. Start session
    let session_id = start_debugger(
        "<language>",
        "tests/fixtures/fizzbuzz_<language>.ext",
        true, // stopOnEntry
    ).await?;

    // 2. Wait for stop at entry
    wait_for_stopped(&session_id, "breakpoint").await?;

    // 3. Set breakpoint at bug location
    let bp = set_breakpoint(
        &session_id,
        "tests/fixtures/fizzbuzz_<language>.ext",
        9, // Line with bug
    ).await?;
    assert!(bp.verified);

    // 4. Continue to breakpoint
    continue_execution(&session_id).await?;
    wait_for_stopped(&session_id, "breakpoint").await?;

    // 5. Evaluate expressions
    let n = evaluate(&session_id, "n", Some(frame_id)).await?;
    assert_eq!(n.result, "4");

    // 6. Verify bug
    let check = evaluate(&session_id, "n % 5 == 0", Some(frame_id)).await?;
    assert_eq!(check.result, "false"); // Should be true!

    // 7. Disconnect
    disconnect(&session_id).await?;
}
```

---

### Step 9: Validate and Document (2-3 hours)

#### 9.1. End-to-End Testing

Run complete debugging workflow with AI agent:

```
User: "Debug my <language> script"

Claude:
  → debugger_start(language="<language>", program="script.ext", stopOnEntry=true)
  ← Session created, stopped at entry

  → debugger_set_breakpoint(sourcePath="script.ext", line=42)
  ← Breakpoint verified

  → debugger_continue()
  → debugger_wait_for_stop()
  ← Stopped at breakpoint

  → debugger_stack_trace()
  ← Stack frames received

  → debugger_evaluate(expression="variable", frameId=1)
  ← Value received

  "I found the bug! The variable 'x' is null at line 42..."
```

#### 9.2. Document Findings

Create `docs/<LANGUAGE>_IMPLEMENTATION.md`:

```markdown
# <Language> Debugging Support

## Status: ✅ Fully Validated

## Adapter Details
- **Adapter**: <adapter name>
- **Transport**: <STDIO/TCP/Pipe>
- **stopOnEntry**: <✅ Native / ❌ Requires workaround>
- **Installation**: `<install command>`

## Known Issues
1. **Issue description**
   - Workaround: <solution>

## Performance
- Session start: <Xms>
- Breakpoint set: <Xms>
- Evaluation: <Xms>

## Example Usage
[code examples]
```

---

## Key Learnings from Python and Ruby

### 1. DAP Specification Compliance is Critical

**Lesson**: Always follow the correct DAP sequence.

❌ **Wrong** (our original implementation):
```
initialize → initialized → configurationDone → setBreakpoints
```

✅ **Correct** (per DAP spec):
```
initialize → initialized → setBreakpoints → configurationDone
```

**Impact**: Ruby (rdbg) failed because it's strict about sequence. Python (debugpy) worked by luck (forgiving implementation).

### 2. stopOnEntry is Not Universal

**Python (debugpy)**: ✅ Native support, works perfectly
**Ruby (rdbg)**: ❌ No native support, requires entry breakpoint workaround

**Solution**: Entry breakpoint pattern (works for all languages):
1. Detect first executable line
2. Set breakpoint BEFORE configurationDone
3. Program stops at entry point

### 3. Transport Mechanisms Vary

**Python (debugpy)**: STDIO or TCP
**Ruby (rdbg)**: TCP only (socket mode)
**Node.js**: TCP (inspector protocol)

**Lesson**: Abstract transport in adapter configuration.

### 4. Timing is Critical

Different adapters have different startup times:
- debugpy: ~100-200ms
- rdbg: ~50-100ms
- Node.js: ~200-300ms

**Solution**: Retry logic with exponential backoff:
```rust
pub async fn connect_with_retry(port: u16, max_wait: Duration) -> Result<TcpStream> {
    let start = Instant::now();
    while start.elapsed() < max_wait {
        if let Ok(socket) = TcpStream::connect(("localhost", port)).await {
            return Ok(socket);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(Error::Timeout)
}
```

### 5. Adapter Bugs Exist

**rdbg pause bug**: Accepts pause request but doesn't actually pause.
- Returns: `{success: true}`
- Reality: Program continues running, no stopped event

**Lesson**: Test thoroughly, have workarounds ready.

### 6. Language-Specific Parsing

Each language has different comment/declaration syntax:

```rust
// Ruby: #, require, class, module
// Python: #, import, from, class, def
// JavaScript: //, import, export, class, function
// Go: //, package, import, func, type
```

**Solution**: Language-specific first-line detection functions.

---

## Checklist for New Language

### Research Phase
- [ ] Adapter exists and is maintained
- [ ] DAP implementation is complete
- [ ] Documentation is available
- [ ] Adapter tested manually
- [ ] Known limitations documented

### Implementation Phase
- [ ] Adapter struct created (`src/adapters/<language>.rs`)
- [ ] Launch configuration implemented
- [ ] Transport layer configured
- [ ] stopOnEntry handling implemented
- [ ] DAP sequence verified
- [ ] Adapter added to registry

### Testing Phase
- [ ] Unit tests for adapter
- [ ] Integration test (FizzBuzz)
- [ ] End-to-end test with AI agent
- [ ] Performance benchmarked
- [ ] Edge cases handled

### Documentation Phase
- [ ] Implementation guide created
- [ ] Known issues documented
- [ ] README updated
- [ ] Docker image created
- [ ] Examples added

### Validation Phase
- [ ] All features working
- [ ] No regressions in other languages
- [ ] Performance acceptable
- [ ] Documentation complete
- [ ] Ready for production

---

## Estimated Timelines

| Phase | Time Estimate | Complexity |
|-------|---------------|------------|
| Research | 1-2 hours | Low |
| Adapter Config | 30 minutes | Low |
| Manual Testing | 1 hour | Low |
| Transport Layer | 2-4 hours | Medium |
| stopOnEntry | 2-4 hours | High |
| DAP Sequence | 2-3 hours | Medium |
| Docker Image | 1 hour | Low |
| Integration Tests | 3-4 hours | Medium |
| Documentation | 2-3 hours | Low |
| **Total** | **1-2 days** | **Medium** |

**Full Validation**: +3-5 days for comprehensive testing and edge cases

---

## Support Matrix

| Feature | Python | Ruby | Node.js | Go | Rust |
|---------|--------|------|---------|-----|------|
| stopOnEntry | ✅ Native | ✅ Workaround | ✅ Native | ⏳ TBD | ⏳ TBD |
| Breakpoints | ✅ | ✅ | ⏳ | ⏳ | ⏳ |
| Stepping | ✅ | ✅ | ⏳ | ⏳ | ⏳ |
| Evaluation | ✅ | ✅ | ⏳ | ⏳ | ⏳ |
| Stack Trace | ✅ | ✅ | ⏳ | ⏳ | ⏳ |

---

## References

- **DAP Specification**: https://microsoft.github.io/debug-adapter-protocol/specification
- **Python Implementation**: `docs/PYTHON_IMPLEMENTATION.md`
- **Ruby Implementation**: `docs/RDBG_ANALYSIS_AND_SOLUTION.md`
- **Entry Breakpoint Solution**: `docs/RUBY_STOPENTRY_FIX.md`
- **Success Report**: `/home/vagrant/projects/fizzbuzz-ruby-test/SUCCESS_REPORT.md`

---

## Contact & Contributions

Questions about adding a new language? Check existing implementations or open an issue on GitHub.

**Last Updated**: October 7, 2025
**Validated Languages**: Python (debugpy), Ruby (rdbg)
**Next Target**: Node.js (inspector protocol)
