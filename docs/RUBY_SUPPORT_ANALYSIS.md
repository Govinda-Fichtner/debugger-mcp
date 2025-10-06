# Ruby Support Analysis for Debugger MCP

## Research Summary

This document analyzes adding Ruby debugging support to the debugger MCP server, based on research of nvim-dap and ruby/debug.

---

## Ruby Debugger: rdbg (ruby/debug gem)

### Installation
```bash
gem install debug
# or in Gemfile:
gem "debug", ">= 1.0.0"
```

### Command-Line Interface

**STDIO DAP Mode (Used by MCP Server)**:
```bash
rdbg --command target.rb
```

**Key Flags**:
- `--command`: Use stdio for DAP communication (default mode)
- `--nonstop`: Continue execution without stopping at entry
- `--stop-at-load`: Stop immediately when debugger loads

**Socket DAP Server Mode** (NOT used by MCP):
```bash
rdbg --open --port <PORT> target.rb
```
- `--open`: Start DAP server via TCP/UNIX socket
- Note: Our MCP server uses stdio mode, not socket mode

**With Bundle**:
```bash
bundle exec rdbg --command target.rb
```

### DAP Protocol Support

**Launch Request**:
- Sends initial capabilities
- Supports `localfs` or `localfsMap` for path mapping
- `nonstop` flag controls initial stop behavior

**Attach Request**:
- Similar to launch
- Can attach to already running process

**Supported DAP Commands**:
- initialize
- launch / attach
- setBreakpoints
- continue
- next (step over)
- stepIn
- stepOut
- stackTrace
- scopes
- variables
- evaluate
- disconnect

---

## Comparison: debugpy vs rdbg

| Feature | Python (debugpy) | Ruby (rdbg) |
|---------|------------------|-------------|
| **Installation** | `pip install debugpy` | `gem install debug` |
| **Command** | `python -m debugpy` | `rdbg` or `bundle exec rdbg` |
| **DAP Mode** | `--listen <host>:<port>` | `--open --port <port>` |
| **Stop at Entry** | Via DAP launch request | `--open` (default) or `--stop-at-load` |
| **Continue at Entry** | Via DAP launch request | `--open --nonstop` |
| **Default Port** | None (must specify) | None (must specify) |
| **Default Host** | 127.0.0.1 | 127.0.0.1 |
| **Bundle Support** | N/A | `bundle exec` wrapper |
| **Path Mapping** | Via DAP protocol | Via `localfs`/`localfsMap` |

---

## nvim-dap Configuration (Reference)

From nvim-dap wiki, Ruby is configured as:

```lua
dap.adapters.ruby = function(callback, config)
  callback {
    type = "server",
    host = "127.0.0.1",
    port = "${port}",
    executable = {
      command = "bundle",
      args = { "exec", "rdbg", "-n", "--open", "--port", "${port}",
        "-c", "--", "bundle", "exec", config.command, config.script,
      },
    },
  }
end
```

**Key Observations**:
- Uses `bundle exec` wrapper
- `-n` flag (equivalent to `--nonstop`)
- `-c` flag before `--`
- Separates command and script

---

## Implementation Plan for Debugger MCP

### 1. Adapter Detection (Language Selection)

**Current (Python)**:
```rust
match language.as_str() {
    "python" => { /* spawn debugpy */ }
    _ => Err(Error::UnsupportedLanguage)
}
```

**Updated (Python + Ruby)**:
```rust
match language.as_str() {
    "python" => { /* spawn debugpy */ }
    "ruby" => { /* spawn rdbg */ }
    _ => Err(Error::UnsupportedLanguage)
}
```

### 2. Adapter Spawning

**Python (debugpy)**:
```rust
Command::new("python3")
    .args([
        "-m", "debugpy",
        "--listen", &format!("{}:{}", host, port),
        "--wait-for-client",
        program
    ])
```

**Ruby (rdbg)** - STDIO Mode (Implemented):
```rust
Command::new("rdbg")
    .args([
        "--command",  // Use stdio for DAP communication
        "--nonstop",  // Optional: only if stopOnEntry is false
        program
    ])
```

**Why `--command` instead of `--open`**:
- `--command`: Uses stdin/stdout for DAP protocol (like debugpy)
- `--open`: Creates TCP/UNIX socket (requires separate connection)
- Our DAP client uses stdin/stdout, so `--command` is required

**Bundle Support** (Future Enhancement):
```rust
Command::new("bundle")
    .args([
        "exec", "rdbg",
        "--command",
        program
    ])
```

### 3. stopOnEntry Handling

**Python**:
- Handled via DAP launch request `stopOnEntry` parameter
- debugpy honors this in the protocol

**Ruby**:
- Default behavior with `--open`: STOPS at program start
- Add `--nonstop` flag if `stopOnEntry: false`

**Implementation**:
```rust
let mut args = vec!["--open", "--port", &port, "--host", &host];

if !stop_on_entry {
    args.push("--nonstop");
}

args.push(program);
```

### 4. Launch Arguments Structure

**Current DAP Launch Args (Python)**:
```json
{
  "program": "/path/to/script.py",
  "args": ["arg1", "arg2"],
  "cwd": "/working/dir",
  "stopOnEntry": true
}
```

**Needed for Ruby** (same structure):
```json
{
  "program": "/path/to/script.rb",
  "args": ["arg1", "arg2"],
  "cwd": "/working/dir",
  "stopOnEntry": true
}
```

**Path Mapping** (Ruby-specific):
- May need to send `localfs: true` in launch request
- Similar to how Python handles path mapping

---

## Code Changes Required

### File: `src/dap/adapter.rs` (or equivalent)

**Current**:
```rust
pub enum AdapterId {
    Python,
}

impl AdapterId {
    pub fn spawn(&self, program: &str, port: u16) -> Result<Child> {
        match self {
            AdapterId::Python => {
                Command::new("python3")
                    .args(["-m", "debugpy", "--listen", ...])
                    .spawn()
            }
        }
    }
}
```

**Updated**:
```rust
pub enum AdapterId {
    Python,
    Ruby,
}

impl AdapterId {
    pub fn spawn(&self, program: &str, port: u16, stop_on_entry: bool) -> Result<Child> {
        match self {
            AdapterId::Python => {
                Command::new("python3")
                    .args(["-m", "debugpy", ...])
                    .spawn()
            }
            AdapterId::Ruby => {
                let mut cmd = Command::new("rdbg");
                cmd.args(["--open", "--port", &port.to_string()]);

                if !stop_on_entry {
                    cmd.arg("--nonstop");
                }

                cmd.arg(program)
                   .spawn()
            }
        }
    }
}
```

### File: `src/debug/session.rs`

**Update language detection**:
```rust
pub async fn new(language: &str, program: String) -> Result<Self> {
    let adapter_id = match language.to_lowercase().as_str() {
        "python" => AdapterId::Python,
        "ruby" => AdapterId::Ruby,
        _ => return Err(Error::UnsupportedLanguage(language.to_string())),
    };

    // ... rest of initialization
}
```

### File: `Dockerfile`

**Add Ruby and rdbg**:
```dockerfile
# Current: Python + debugpy
RUN apk add --no-cache python3 py3-pip && \
    pip install --break-system-packages debugpy

# Add: Ruby + debug gem
RUN apk add --no-cache ruby ruby-dev ruby-bundler && \
    gem install debug
```

---

## Testing Strategy (TDD Approach)

### Phase 1: Unit Tests

**Test 1**: Language detection for Ruby
```rust
#[test]
fn test_ruby_language_detection() {
    let session = DebugSession::new("ruby", "test.rb".to_string()).await;
    assert!(session.is_ok());
}
```

**Test 2**: Ruby adapter selection
```rust
#[test]
fn test_ruby_adapter_id() {
    let adapter_id = AdapterId::from_language("ruby");
    assert_eq!(adapter_id, AdapterId::Ruby);
}
```

**Test 3**: Ruby command generation
```rust
#[test]
fn test_ruby_spawn_command() {
    let cmd = AdapterId::Ruby.build_command("test.rb", 5678, true);
    assert!(cmd.contains("rdbg"));
    assert!(cmd.contains("--open"));
    assert!(cmd.contains("--port"));
    assert!(!cmd.contains("--nonstop")); // stopOnEntry: true
}
```

### Phase 2: Integration Tests (FizzBuzz Ruby)

**Create**: `tests/fixtures/fizzbuzz.rb`
```ruby
def fizzbuzz(n)
  if n % 15 == 0
    "FizzBuzz"
  elsif n % 3 == 0
    "Fizz"
  elsif n % 5 == 0  # BUG: should be % 5, not % 4
    "Buzz"
  else
    n.to_s
  end
end

def main
  (1..100).each do |i|
    puts fizzbuzz(i)
  end
end

main if __FILE__ == $0
```

**Test 1**: Basic Ruby debugging session
```rust
#[tokio::test]
#[ignore]
async fn test_ruby_debugging_session() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let tools_handler = ToolsHandler::new(session_manager);

    let start_args = json!({
        "language": "ruby",
        "program": "/workspace/fizzbuzz.rb",
        "stopOnEntry": true
    });

    let result = tools_handler.handle_tool("debugger_start", start_args).await;
    assert!(result.is_ok());
}
```

**Test 2**: Ruby stopOnEntry test (same as Python)
**Test 3**: Ruby breakpoint test
**Test 4**: Ruby frameId test
**Test 5**: Ruby step commands test

### Phase 3: User Feedback Tests (Ruby versions)

Mirror all Python user feedback tests:
- `test_ruby_frameid_required_for_local_variables`
- `test_ruby_frame_ids_change_between_stops`
- `test_ruby_list_breakpoints`
- `test_ruby_pattern_inspect_variable_at_breakpoint`
- `test_ruby_step_commands_comprehensive`
- `test_ruby_wait_for_stop_timing_behavior`

---

## TDD Implementation Steps

### Step 1: Red Phase (Write Failing Tests)
1. Add `tests/fixtures/fizzbuzz.rb`
2. Write failing test: `test_ruby_language_detection`
3. Run tests → FAIL ✅

### Step 2: Green Phase (Minimal Implementation)
1. Add `AdapterId::Ruby` enum variant
2. Add Ruby language detection
3. Run tests → PASS ✅
4. **Commit**: "feat: Add Ruby language detection"

### Step 3: Red Phase (Ruby Adapter)
1. Write failing test: `test_ruby_adapter_spawning`
2. Run tests → FAIL ✅

### Step 4: Green Phase (Ruby Adapter)
1. Implement Ruby adapter spawning logic
2. Run tests → PASS ✅
3. **Verify existing Python tests still pass** ✅
4. **Commit**: "feat: Add Ruby adapter spawning"

### Step 5: Red Phase (stopOnEntry)
1. Write failing test: `test_ruby_stopOnEntry`
2. Run tests → FAIL ✅

### Step 6: Green Phase (stopOnEntry)
1. Implement `--nonstop` flag handling
2. Run tests → PASS ✅
3. **Commit**: "feat: Add Ruby stopOnEntry support"

### Step 7-N: Repeat for Each Feature
- Breakpoints
- Continue
- Stack trace
- Evaluate
- Step commands
- etc.

**Each cycle**:
- Write failing test FIRST
- Implement minimal code to pass
- Verify ALL tests pass (Python + Ruby)
- Commit with descriptive message

---

## Docker Considerations

### Binary Deployment (No Changes)
Users install rdbg on their system:
```bash
gem install debug
```

### Docker Deployment (Dockerfile Changes)

**Update Dockerfile**:
```dockerfile
FROM rust:1.83-alpine AS builder

# Build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime image
FROM alpine:latest

# Install Python + debugpy
RUN apk add --no-cache python3 py3-pip && \
    pip install --break-system-packages debugpy

# NEW: Install Ruby + debug gem
RUN apk add --no-cache ruby ruby-dev ruby-bundler && \
    gem install debug

# Copy binary
COPY --from=builder /app/target/release/debugger_mcp /usr/local/bin/

# Create user
RUN addgroup -g 1000 mcpuser && \
    adduser -D -u 1000 -G mcpuser mcpuser

USER mcpuser
WORKDIR /workspace

ENTRYPOINT ["debugger_mcp"]
```

**Test both**:
1. Binary mode: User has rdbg installed
2. Docker mode: Container has rdbg installed

---

## Potential Gotchas

### 1. Bundle vs Direct rdbg
- nvim-dap uses `bundle exec` wrapper
- We should support both modes
- Start simple (direct rdbg), add bundle later

### 2. Port Allocation
- Same as Python: find free port dynamically
- No changes needed

### 3. Path Mapping
- Ruby debugger has `localfs` option
- May need to set in launch request
- Test with Docker path mapping

### 4. Ruby Version Compatibility
- debug gem requires Ruby 2.6+
- Alpine Linux has recent Ruby
- Document minimum version requirement

### 5. Gem Dependencies
- Some projects use Bundler
- May need `bundle exec rdbg` support
- Add as enhancement after basic support works

---

## Success Criteria

### Functionality
- ✅ Ruby programs can be debugged
- ✅ stopOnEntry works correctly
- ✅ Breakpoints can be set and hit
- ✅ Stack trace retrieval works
- ✅ Variable evaluation works (with frameId)
- ✅ Step commands work (over, into, out)

### Testing
- ✅ All Python tests still pass
- ✅ Ruby unit tests pass
- ✅ Ruby integration tests pass (FizzBuzz)
- ✅ Ruby user feedback tests pass
- ✅ Same test coverage as Python

### Deployment
- ✅ Works as plain binary (user installs rdbg)
- ✅ Works in Docker container (rdbg in image)
- ✅ Documentation updated for both modes

### Documentation
- ✅ Getting started guide includes Ruby
- ✅ Ruby examples provided
- ✅ Tool descriptions mention Ruby support
- ✅ Deployment guide covers Ruby setup

---

## Implementation Timeline Estimate

**Phase 1**: Basic Ruby Support (2-3 hours)
- Language detection
- Adapter spawning
- Basic tests
- ~3-5 commits

**Phase 2**: Full Ruby Debugging (3-4 hours)
- stopOnEntry support
- Breakpoints
- Stack trace
- Evaluate
- ~5-7 commits

**Phase 3**: Step Commands (1-2 hours)
- step_over
- step_into
- step_out
- ~2-3 commits

**Phase 4**: Integration Tests (2-3 hours)
- FizzBuzz Ruby tests
- User feedback tests
- ~3-5 commits

**Phase 5**: Docker & Documentation (1-2 hours)
- Dockerfile updates
- Documentation
- ~2-3 commits

**Total**: ~9-14 hours, ~15-25 commits

---

## Next Steps (Awaiting Confirmation)

1. ✅ Research complete
2. ✅ Analysis documented
3. ⏳ Get confirmation on approach
4. Start TDD implementation:
   - Create fizzbuzz.rb fixture
   - Write first failing test
   - Implement minimal code
   - Commit frequently
   - Run all tests after each change

**Ready to proceed with implementation?**
