# Future Enhancement: MCP Resources for Documentation

## Overview

Currently, comprehensive documentation exists in markdown files under `/docs`, but it's not exposed via the MCP protocol. This enhancement would make documentation directly accessible to AI agents through MCP resources.

## Proposed MCP Resources

### 1. debugger://patterns

**Content**: Common debugging patterns cookbook

**From**: docs/USER_FEEDBACK_IMPROVEMENTS.md (Pattern 1-6)

**Examples**:
- Pattern: Inspect variable at breakpoint
- Pattern: Step through code line by line
- Pattern: Debug loop - multiple iterations
- Pattern: Compare expressions
- Pattern: Navigate call stack
- Pattern: Step into → inspect → step out

**Benefit**: AI agents can reference these patterns directly via MCP resource fetch

---

### 2. debugger://quickref

**Content**: Quick reference card

**From**: docs/USER_FEEDBACK_IMPROVEMENTS.md (Quick Reference Card section)

**Includes**:
- Essential workflow (7 steps)
- Golden rules (4 critical rules)
- Tool categories
- Performance expectations table
- Troubleshooting quick checks

**Benefit**: Quick lookup for common operations and best practices

---

### 3. debugger://deployment

**Content**: Deployment context guide

**From**: docs/USER_FEEDBACK_IMPROVEMENTS.md (Deployment Contexts section)

**Covers**:
- Native installation (no path mapping)
- Docker containers (volume mount path mapping)
- Kubernetes (PersistentVolumeClaim mounts)
- WSL (Windows Subsystem for Linux)
- Decision tree for path mapping
- Troubleshooting path issues

**Benefit**: Helps users understand path mapping based on their deployment

---

### 4. debugger://errors

**Content**: Error messages reference

**From**: docs/USER_FEEDBACK_IMPROVEMENTS.md (Error Messages Reference)

**Includes**:
- NameError: name 'variable' is not defined → frameId missing
- Unable to find thread for evaluation → stale frame ID
- Cannot get stack trace while running → need to wait for stop
- Timeout waiting for program to stop → breakpoint/path issues
- Source not available / File not found → path mapping issues

**Benefit**: AI agents can look up error solutions programmatically

---

## Implementation Approach

### Option 1: Static Resources (Simple)

Add to `src/mcp/resources/mod.rs`:

```rust
pub async fn read_resource(&self, uri: &str) -> Result<ResourceContents> {
    match uri {
        "debugger://patterns" => {
            Ok(ResourceContents {
                uri: uri.to_string(),
                mimeType: Some("text/markdown".to_string()),
                text: Some(include_str!("../../docs/patterns.md").to_string()),
            })
        }
        "debugger://quickref" => {
            Ok(ResourceContents {
                uri: uri.to_string(),
                mimeType: Some("text/markdown".to_string()),
                text: Some(include_str!("../../docs/quickref.md").to_string()),
            })
        }
        "debugger://deployment" => {
            Ok(ResourceContents {
                uri: uri.to_string(),
                mimeType: Some("text/markdown".to_string()),
                text: Some(include_str!("../../docs/deployment.md").to_string()),
            })
        }
        "debugger://errors" => {
            Ok(ResourceContents {
                uri: uri.to_string(),
                mimeType: Some("text/markdown".to_string()),
                text: Some(include_str!("../../docs/errors.md").to_string()),
            })
        }
        _ => Err(Error::ResourceNotFound(uri.to_string()))
    }
}
```

**Pros**:
- Simple implementation
- Fast (embedded at compile time)
- No runtime file I/O

**Cons**:
- Requires rebuild to update docs
- Duplicates content from USER_FEEDBACK_IMPROVEMENTS.md

---

### Option 2: Dynamic Resources (Flexible)

Keep docs in single markdown file, parse sections dynamically:

```rust
pub async fn read_resource(&self, uri: &str) -> Result<ResourceContents> {
    let docs = std::fs::read_to_string("docs/USER_FEEDBACK_IMPROVEMENTS.md")?;

    let content = match uri {
        "debugger://patterns" => extract_section(&docs, "## 6. Common Debugging Patterns"),
        "debugger://quickref" => extract_section(&docs, "## 8. Quick Reference Card"),
        "debugger://deployment" => extract_section(&docs, "## 5. Path Mapping"),
        "debugger://errors" => extract_section(&docs, "## 7. Error Messages Reference"),
        _ => return Err(Error::ResourceNotFound(uri.to_string()))
    };

    Ok(ResourceContents {
        uri: uri.to_string(),
        mimeType: Some("text/markdown".to_string()),
        text: Some(content),
    })
}
```

**Pros**:
- Single source of truth (USER_FEEDBACK_IMPROVEMENTS.md)
- Updates without rebuild
- Easier to maintain

**Cons**:
- Runtime file I/O
- Requires section parsing logic
- Docker container needs mounted docs

---

## Recommended Approach

**Hybrid**: Static resources with extracted markdown files

1. Extract sections from USER_FEEDBACK_IMPROVEMENTS.md into separate files:
   - `docs/patterns.md`
   - `docs/quickref.md`
   - `docs/deployment.md`
   - `docs/errors.md`

2. Embed them with `include_str!` (Option 1)

3. Keep USER_FEEDBACK_IMPROVEMENTS.md as comprehensive reference

**Benefits**:
- Fast (compiled in)
- Clean separation of concerns
- Easy to reference individual topics
- No duplication (extract, don't duplicate)

---

## Implementation Steps

1. **Extract Sections** (~30 min)
   - Create `docs/patterns.md` from Pattern 1-6
   - Create `docs/quickref.md` from Quick Reference section
   - Create `docs/deployment.md` from Deployment Contexts section
   - Create `docs/errors.md` from Error Messages section

2. **Update Resource Handler** (~15 min)
   - Add 4 new resource URIs to `src/mcp/resources/mod.rs`
   - Use `include_str!` to embed content
   - Add to `list_resources()` response

3. **Update Tool Descriptions** (~5 min)
   - Change references from `debugger://patterns` to actual URIs
   - Update SEE ALSO sections to reference new resources

4. **Test** (~10 min)
   - Verify resources accessible via MCP
   - Check content renders correctly
   - Test with AI agent requests

**Total Effort**: ~1 hour

---

## Usage Example

Once implemented, AI agents could:

```javascript
// Get common patterns
const patterns = await mcp.readResource("debugger://patterns")
// Returns markdown with all 6 debugging patterns

// Get quick reference
const quickref = await mcp.readResource("debugger://quickref")
// Returns essential workflow and golden rules

// Get deployment guide
const deployment = await mcp.readResource("debugger://deployment")
// Returns path mapping guide for current context

// Get error solutions
const errors = await mcp.readResource("debugger://errors")
// Returns error message reference with solutions
```

---

## Priority

**Medium Priority** - Nice to have but not critical

Current state:
- ✅ Documentation exists and is comprehensive
- ✅ AI agents can read from markdown files
- ✅ Tool descriptions now include critical info

Benefits of implementation:
- More discoverable (via MCP resources/list)
- Structured access for AI agents
- Cleaner separation of reference docs

**Recommendation**: Implement when:
1. User feedback requests more structured docs
2. Multiple AI agents need reference docs
3. Documentation becomes harder to navigate

---

## Alternative: Keep Current Approach

**Current state is acceptable**:
- Comprehensive markdown docs exist
- Tool descriptions updated with critical info
- AI agents can read markdown directly
- No duplication of content

**When to implement MCP resources**:
- User explicitly requests structured resource access
- Multiple MCP clients need consistent doc access
- Documentation grows beyond manageable markdown files

---

## Decision Point

For now: **DEFER IMPLEMENTATION**

Reasons:
1. Tool descriptions now contain critical information
2. Comprehensive markdown docs available
3. No user requests for MCP resource access
4. Focus should be on code quality and testing

Future trigger to implement:
- User requests structured documentation access
- Multiple AI agents need coordinated doc access
- Tool descriptions become too large

---

**Status**: Documented for future consideration
**Effort**: ~1 hour when decided to implement
**Priority**: Low (nice to have, not critical)
