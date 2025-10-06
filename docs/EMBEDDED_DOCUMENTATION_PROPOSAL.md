# Embedded Documentation Resources Proposal

**Date**: 2025-10-06
**Status**: Proposal for Review
**Complements**: MCP_DOCUMENTATION_IMPROVEMENT_PROPOSAL.md

## Executive Summary

This proposal adds **embedded documentation resources** that expose comprehensive guides from `/docs` as MCP resources, making them accessible to AI agents through standard MCP resource URIs. This complements the in-tool descriptions with deeper, tutorial-style documentation.

## Why This Works Exceptionally Well

### 1. **Aligned with MCP Specification** ✅

The MCP spec explicitly supports embedded resources with:
- Custom URI schemes (we'll use `debugger-docs://`)
- Text/markdown MIME types
- Annotations for audience and priority
- Client-side fetching via `resources/read`

### 2. **GitHub as Canonical Source** ✅

Using GitHub URLs as the underlying storage:
- ✅ **Version controlled** - Documentation evolves with code
- ✅ **Always up-to-date** - Single source of truth
- ✅ **Accessible** - Public HTTPS URLs
- ✅ **Reviewable** - PRs can review documentation changes
- ✅ **Collaborative** - Others can contribute

### 3. **Separation of Concerns** ✅

**Tools/Resources** (in-server):
- Brief, actionable descriptions
- Parameter schemas
- Quick reference

**Embedded Docs** (GitHub):
- Detailed explanations
- Architecture discussions
- Troubleshooting guides
- Design decisions

This creates a **layered documentation** approach:
- Layer 1: Tool descriptions (immediate, concise)
- Layer 2: Workflow resources (structured, step-by-step)
- Layer 3: Embedded docs (comprehensive, tutorial-style)

## Proposed Documentation Structure

### Documentation Categories

We'll organize docs into **5 categories** that map to AI agent needs:

#### 1. **Getting Started** 🚀
*For agents new to the debugger*

- `debugger-docs://getting-started` → Comprehensive introduction
- `debugger-docs://quickstart` → 5-minute tutorial
- `debugger-docs://architecture` → How the debugger works

#### 2. **Usage Guides** 📚
*For completing specific tasks*

- `debugger-docs://guide/debugging-python` → Python-specific guide
- `debugger-docs://guide/async-initialization` → Understanding async behavior
- `debugger-docs://guide/breakpoints` → Breakpoint strategies
- `debugger-docs://guide/state-management` → Session state lifecycle

#### 3. **Reference** 📖
*For looking up details*

- `debugger-docs://reference/states` → All session states
- `debugger-docs://reference/errors` → Error codes and meanings
- `debugger-docs://reference/tools` → Complete tool reference
- `debugger-docs://reference/dap-protocol` → DAP integration details

#### 4. **Troubleshooting** 🔧
*For debugging issues*

- `debugger-docs://troubleshooting/common-issues` → FAQ
- `debugger-docs://troubleshooting/performance` → Optimization tips
- `debugger-docs://troubleshooting/integration` → MCP integration issues
- `debugger-docs://troubleshooting/race-conditions` → Timing problems

#### 5. **Advanced Topics** 🎓
*For power users*

- `debugger-docs://advanced/logging` → Log validation system
- `debugger-docs://advanced/extending` → Adding new adapters
- `debugger-docs://advanced/testing` → Integration testing

## Implementation Design

### Resource Mapping Strategy

**Option A: Direct GitHub Links** (RECOMMENDED)

Map each doc resource to its GitHub URL:

```rust
pub fn list_documentation_resources() -> Vec<Resource> {
    let base_url = "https://raw.githubusercontent.com/Govinda-Fichtner/debugger-mcp/main/docs";

    vec![
        Resource {
            uri: "debugger-docs://getting-started".to_string(),
            name: "Getting Started Guide".to_string(),
            description: Some("Comprehensive introduction for AI agents new to the debugger".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: Some(json!({
                "audience": ["assistant"],
                "priority": 1.0,
                "category": "getting-started",
                "githubUrl": format!("{}/GETTING_STARTED.md", base_url),
                "estimatedReadTime": "5 minutes"
            })),
        },
        Resource {
            uri: "debugger-docs://guide/async-initialization".to_string(),
            name: "Async Initialization Guide".to_string(),
            description: Some("Understanding asynchronous session initialization and state polling".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: Some(json!({
                "audience": ["assistant"],
                "priority": 0.9,
                "category": "usage-guide",
                "githubUrl": format!("{}/ASYNC_INIT_IMPLEMENTATION.md", base_url),
                "topics": ["async", "initialization", "state-polling"],
                "estimatedReadTime": "10 minutes"
            })),
        },
        // ... more resources
    ]
}
```

**Benefits:**
- ✅ Always up-to-date (pulls from GitHub)
- ✅ No duplication (single source of truth)
- ✅ Reviewable (PRs show doc changes)
- ✅ Collaborative (others can contribute)

**Option B: Embedded at Compile Time** (Alternative)

Use `include_str!` to embed docs in binary:

```rust
pub fn read_documentation(uri: &str) -> Result<ResourceContents> {
    let content = match uri {
        "debugger-docs://getting-started" => {
            include_str!("../../docs/GETTING_STARTED.md")
        }
        "debugger-docs://guide/async-initialization" => {
            include_str!("../../docs/ASYNC_INIT_IMPLEMENTATION.md")
        }
        _ => return Err(Error::InvalidRequest(format!("Unknown doc: {}", uri)))
    };

    Ok(ResourceContents {
        uri: uri.to_string(),
        mime_type: "text/markdown".to_string(),
        text: Some(content.to_string()),
        blob: None,
    })
}
```

**Benefits:**
- ✅ No network dependency
- ✅ Offline capable
- ✅ Guaranteed available

**Drawbacks:**
- ❌ Increases binary size
- ❌ Requires recompile for doc updates
- ❌ Harder to keep in sync

**RECOMMENDATION**: Use **Option A (GitHub links)** because:
1. Documentation updates don't require recompilation
2. Binary stays small
3. GitHub is highly available (99.95%+ uptime)
4. Can add caching layer if needed

### Implementation Code

**File: `src/mcp/resources/documentation.rs` (NEW)**

```rust
use crate::{Error, Result};
use crate::mcp::resources::Resource;
use serde_json::json;

/// GitHub repository base URL for documentation
const GITHUB_DOCS_BASE: &str = "https://raw.githubusercontent.com/Govinda-Fichtner/debugger-mcp/main/docs";

/// Documentation resource handler
pub struct DocumentationHandler;

impl DocumentationHandler {
    pub fn new() -> Self {
        Self
    }

    /// List all available documentation resources
    pub fn list_resources() -> Vec<Resource> {
        vec![
            // === GETTING STARTED ===
            Resource {
                uri: "debugger-docs://getting-started".to_string(),
                name: "Getting Started with Debugger MCP".to_string(),
                description: Some(
                    "Comprehensive introduction for AI agents. Covers basic concepts, \
                    typical workflows, and how to get started debugging programs. \
                    READ THIS FIRST if you're new to this debugger."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 1.0,
                    "category": "getting-started",
                    "githubUrl": format!("{}/GETTING_STARTED.md", GITHUB_DOCS_BASE),
                    "topics": ["introduction", "basics", "quickstart"],
                    "readTime": "5 min"
                })),
            },

            // === USAGE GUIDES ===
            Resource {
                uri: "debugger-docs://guide/async-initialization".to_string(),
                name: "Async Initialization Guide".to_string(),
                description: Some(
                    "Deep dive into asynchronous session initialization. Explains why \
                    debugger_start returns immediately, how to poll for state changes, \
                    and how pre-launch breakpoints work. CRITICAL for understanding \
                    the debugger's async behavior."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 0.9,
                    "category": "usage-guide",
                    "githubUrl": format!("{}/ASYNC_INIT_IMPLEMENTATION.md", GITHUB_DOCS_BASE),
                    "topics": ["async", "initialization", "state-polling", "breakpoints"],
                    "readTime": "10 min",
                    "prerequisites": ["getting-started"]
                })),
            },

            Resource {
                uri: "debugger-docs://guide/workflows".to_string(),
                name: "Complete Debugging Workflows".to_string(),
                description: Some(
                    "Step-by-step workflows for common debugging scenarios. Includes \
                    basic debugging, multiple breakpoints, expression evaluation, and more. \
                    Use these as templates for your debugging tasks."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 0.85,
                    "category": "usage-guide",
                    "githubUrl": format!("{}/COMPLETE_SOLUTION_SUMMARY.md", GITHUB_DOCS_BASE),
                    "topics": ["workflows", "examples", "patterns"],
                    "readTime": "8 min"
                })),
            },

            // === TROUBLESHOOTING ===
            Resource {
                uri: "debugger-docs://troubleshooting/common-issues".to_string(),
                name: "Common Issues and Solutions".to_string(),
                description: Some(
                    "Frequently encountered problems and how to solve them. Covers \
                    SessionNotFound, InvalidState, timeout issues, breakpoint problems, \
                    and more. CHECK HERE FIRST when encountering errors."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 0.95,
                    "category": "troubleshooting",
                    "githubUrl": format!("{}/MCP_INTEGRATION_FIX_PROPOSAL.md", GITHUB_DOCS_BASE),
                    "topics": ["errors", "debugging", "recovery", "solutions"],
                    "readTime": "12 min"
                })),
            },

            // === ADVANCED TOPICS ===
            Resource {
                uri: "debugger-docs://advanced/logging".to_string(),
                name: "Logging and Validation System".to_string(),
                description: Some(
                    "How the debugger's comprehensive logging system works. Explains \
                    emoji-coded logs, log validation, and how to use logs for debugging \
                    issues. Useful for understanding internal behavior."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 0.5,
                    "category": "advanced",
                    "githubUrl": format!("{}/LOG_VALIDATION_SYSTEM.md", GITHUB_DOCS_BASE),
                    "topics": ["logging", "observability", "debugging"],
                    "readTime": "15 min"
                })),
            },

            Resource {
                uri: "debugger-docs://advanced/dap-protocol".to_string(),
                name: "DAP Protocol Integration".to_string(),
                description: Some(
                    "Technical details of the Debug Adapter Protocol (DAP) integration. \
                    Covers event handling, message sequencing, and protocol specifics. \
                    For understanding the underlying debugging mechanism."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 0.4,
                    "category": "advanced",
                    "githubUrl": format!("{}/DAP_FIX_COMPLETE.md", GITHUB_DOCS_BASE),
                    "topics": ["dap", "protocol", "architecture", "internals"],
                    "readTime": "20 min"
                })),
            },

            // === INDEX / TABLE OF CONTENTS ===
            Resource {
                uri: "debugger-docs://index".to_string(),
                name: "Documentation Index".to_string(),
                description: Some(
                    "Complete table of contents for all documentation. Lists all \
                    available guides organized by category. START HERE to discover \
                    what documentation is available."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                annotations: Some(json!({
                    "audience": ["assistant"],
                    "priority": 0.8,
                    "category": "meta",
                    "topics": ["index", "contents", "navigation"],
                    "readTime": "2 min"
                })),
            },
        ]
    }

    /// Read documentation resource by URI
    /// This fetches content from GitHub
    pub async fn read_resource(uri: &str) -> Result<String> {
        // Parse URI to get document identifier
        if !uri.starts_with("debugger-docs://") {
            return Err(Error::InvalidRequest(
                format!("Invalid documentation URI: {}", uri)
            ));
        }

        // Special case: index is generated dynamically
        if uri == "debugger-docs://index" {
            return Ok(Self::generate_index());
        }

        // Find the resource to get GitHub URL
        let resources = Self::list_resources();
        let resource = resources
            .iter()
            .find(|r| r.uri == uri)
            .ok_or_else(|| {
                Error::InvalidRequest(format!("Unknown documentation: {}", uri))
            })?;

        // Extract GitHub URL from annotations
        let github_url = resource
            .annotations
            .as_ref()
            .and_then(|a| a.get("githubUrl"))
            .and_then(|u| u.as_str())
            .ok_or_else(|| {
                Error::Internal("Documentation resource missing githubUrl".to_string())
            })?;

        // Fetch from GitHub
        Self::fetch_from_github(github_url).await
    }

    /// Fetch documentation content from GitHub
    async fn fetch_from_github(url: &str) -> Result<String> {
        // Use reqwest or similar HTTP client
        let response = reqwest::get(url)
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch documentation: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "GitHub returned status {}: {}",
                response.status(),
                url
            )));
        }

        response
            .text()
            .await
            .map_err(|e| Error::Internal(format!("Failed to read documentation: {}", e)))
    }

    /// Generate the documentation index dynamically
    fn generate_index() -> String {
        let resources = Self::list_resources();
        let mut index = String::from("# Debugger MCP Documentation Index\n\n");
        index.push_str("Complete guide to using the Debugger MCP server.\n\n");

        // Group by category
        let categories = vec![
            ("getting-started", "🚀 Getting Started", "New to the debugger? Start here!"),
            ("usage-guide", "📚 Usage Guides", "Learn how to use specific features"),
            ("troubleshooting", "🔧 Troubleshooting", "Solve common problems"),
            ("advanced", "🎓 Advanced Topics", "Deep dives for power users"),
        ];

        for (category, title, description) in categories {
            let category_resources: Vec<_> = resources
                .iter()
                .filter(|r| {
                    r.annotations
                        .as_ref()
                        .and_then(|a| a.get("category"))
                        .and_then(|c| c.as_str()) == Some(category)
                })
                .collect();

            if !category_resources.is_empty() {
                index.push_str(&format!("\n## {}\n\n", title));
                index.push_str(&format!("{}\n\n", description));

                for resource in category_resources {
                    let read_time = resource
                        .annotations
                        .as_ref()
                        .and_then(|a| a.get("readTime"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("N/A");

                    index.push_str(&format!(
                        "### {}\n\n**URI**: `{}`  \n**Read Time**: {}  \n\n{}\n\n",
                        resource.name,
                        resource.uri,
                        read_time,
                        resource.description.as_deref().unwrap_or("No description")
                    ));
                }
            }
        }

        index.push_str("\n---\n\n");
        index.push_str("💡 **Tip**: Access any documentation by calling `resources/read` with the URI.\n\n");
        index.push_str("🔗 **Source**: All documentation is version controlled on GitHub.\n");

        index
    }
}
```

### Integration with Resources Handler

**File: `src/mcp/resources/mod.rs`**

```rust
mod documentation;  // NEW

use documentation::DocumentationHandler;

impl ResourcesHandler {
    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = vec![/* existing resources */];

        // Add documentation resources
        resources.extend(DocumentationHandler::list_resources());

        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContents> {
        if uri.starts_with("debugger-docs://") {
            // Handle documentation resources
            let content = DocumentationHandler::read_resource(uri).await?;
            return Ok(ResourceContents {
                uri: uri.to_string(),
                mime_type: "text/markdown".to_string(),
                text: Some(content),
                blob: None,
            });
        }

        // Existing resource handling...
    }
}
```

## Documentation Files to Create

### 1. `/docs/GETTING_STARTED.md` (NEW)

A comprehensive introduction for AI agents:

```markdown
# Getting Started with Debugger MCP

## What is This?

The Debugger MCP is a Model Context Protocol server that enables AI agents
to debug programs interactively. It supports:

- 🐍 Python debugging (via debugpy)
- 🎯 Breakpoints and stepping
- 📊 Stack traces and variable inspection
- ⚡ Async initialization for fast response

## Quick Start (5 Minutes)

### Step 1: Start a Debugging Session

```javascript
const result = await debugger_start({
  language: "python",
  program: "/path/to/script.py",
  stopOnEntry: true  // IMPORTANT: Pause at first line
});

const sessionId = result.sessionId;
```

**What happens**: Returns immediately (< 100ms) with a session ID.
The debugger initializes in the background.

### Step 2: Wait for Initialization

```javascript
let state;
do {
  const result = await debugger_session_state({sessionId});
  state = result.state;
  await sleep(100);  // Poll every 100ms
} while (state === "Initializing");
```

**What happens**: Session transitions from Initializing → Stopped (at entry)

[... continues with complete tutorial ...]
```

### 2. Update Existing Docs

Add **frontmatter metadata** to existing docs for better discoverability:

```markdown
---
title: Async Initialization Implementation
category: usage-guide
audience: assistant
topics: [async, initialization, state-management]
readTime: 10 min
priority: 0.9
---

# Async Initialization Implementation

[... existing content ...]
```

## Benefits of This Approach

### For AI Agents (Claude Code)

1. **📚 Comprehensive Learning**
   - Can read detailed guides beyond tool descriptions
   - Understands architecture and design decisions
   - Learns troubleshooting strategies

2. **🎯 Contextual Access**
   - Discovers docs through `resources/list`
   - Reads specific guides on-demand
   - Doesn't need to load everything upfront

3. **🔄 Always Current**
   - GitHub is source of truth
   - No stale documentation
   - Updates propagate immediately

4. **📊 Prioritized Learning**
   - Annotations indicate importance
   - Can start with high-priority docs
   - Read time helps plan

### For Human Developers

1. **✍️ Single Source of Truth**
   - One place to update docs (GitHub)
   - No duplication between server and docs
   - Version controlled

2. **🤝 Collaborative**
   - Others can contribute via PRs
   - Review process ensures quality
   - Community can improve docs

3. **📖 Readable on GitHub**
   - Docs work for humans too
   - Nice formatting on GitHub
   - Searchable and linkable

## Usage Patterns for Claude

### Pattern 1: First-Time User

```
User: "Debug this Python script"
Claude: Let me check the documentation...
        [resources/list → finds debugger-docs://getting-started]
        [resources/read debugger-docs://getting-started]
        [Reads guide, understands workflow]
        Now I'll follow the standard workflow:
        1. debugger_start with stopOnEntry
        2. Poll debugger_session_state
        3. Set breakpoint
        ...
```

### Pattern 2: Encountering Error

```
Claude: [Gets SessionNotFound error]
        Let me check the troubleshooting guide...
        [resources/read debugger-docs://troubleshooting/common-issues]
        [Finds SessionNotFound section]
        Ah, I need to verify the session ID is still valid.
        [Checks debugger://sessions resource]
        The session was disconnected. I'll start a new one.
```

### Pattern 3: Advanced Task

```
User: "I need to understand the async behavior better"
Claude: Let me find the relevant documentation...
        [resources/list → filters by topic "async"]
        [Finds debugger-docs://guide/async-initialization]
        [resources/read debugger-docs://guide/async-initialization]
        [Reads detailed guide]
        Now I understand - the session initializes in background,
        and I can set breakpoints during initialization...
```

## Implementation Checklist

### Phase 1: Infrastructure (2 hours)
- [ ] Create `src/mcp/resources/documentation.rs`
- [ ] Add reqwest dependency to Cargo.toml
- [ ] Implement `DocumentationHandler::list_resources()`
- [ ] Implement `DocumentationHandler::read_resource()`
- [ ] Implement `DocumentationHandler::generate_index()`
- [ ] Integrate with `ResourcesHandler`

### Phase 2: New Documentation (3 hours)
- [ ] Write `/docs/GETTING_STARTED.md`
- [ ] Write `/docs/TROUBLESHOOTING.md`
- [ ] Write `/docs/WORKFLOWS.md`
- [ ] Create `/docs/README.md` (index for humans)

### Phase 3: Update Existing Docs (1 hour)
- [ ] Add frontmatter to ASYNC_INIT_IMPLEMENTATION.md
- [ ] Add frontmatter to COMPLETE_SOLUTION_SUMMARY.md
- [ ] Add frontmatter to LOG_VALIDATION_SYSTEM.md
- [ ] Add frontmatter to MCP_INTEGRATION_FIX_PROPOSAL.md
- [ ] Add frontmatter to DAP_FIX_COMPLETE.md

### Phase 4: Testing (2 hours)
- [ ] Test resource listing
- [ ] Test resource reading (GitHub fetch)
- [ ] Test index generation
- [ ] Update Claude Code integration test
- [ ] Verify annotations are correct

### Phase 5: Documentation (1 hour)
- [ ] Update main README.md with documentation info
- [ ] Add "Documentation Resources" section
- [ ] Document how to add new docs

**Total**: 9 hours

## Fallback Strategy

If GitHub is unavailable (rare), we can:

1. **Cache responses** - Store fetched docs in memory
2. **Return cached version** - Use stale docs if GitHub down
3. **Graceful degradation** - Return error with helpful message

```rust
// Add simple in-memory cache
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

struct DocCache {
    cache: Arc<RwLock<HashMap<String, (String, std::time::Instant)>>>,
}

impl DocCache {
    const TTL: std::time::Duration = std::time::Duration::from_secs(300); // 5 min

    async fn get_or_fetch(&self, url: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((content, timestamp)) = cache.get(url) {
                if timestamp.elapsed() < Self::TTL {
                    return Ok(content.clone());
                }
            }
        }

        // Fetch from GitHub
        match Self::fetch_from_github(url).await {
            Ok(content) => {
                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(url.to_string(), (content.clone(), std::time::Instant::now()));
                Ok(content)
            }
            Err(e) => {
                // Return cached even if stale
                let cache = self.cache.read().await;
                if let Some((content, _)) = cache.get(url) {
                    return Ok(content.clone());
                }
                Err(e)
            }
        }
    }
}
```

## Security Considerations

1. **Rate Limiting**: GitHub has rate limits (60 req/hour unauthenticated)
   - ✅ Mitigated by caching
   - ✅ Most docs read once per session

2. **Content Trust**: Using our own GitHub repository
   - ✅ We control the content
   - ✅ PRs are reviewed
   - ✅ Branch protection on main

3. **Network Dependency**: Requires internet access
   - ✅ Acceptable for MCP servers
   - ✅ Fallback to cache if needed
   - ✅ Could add embedded fallback

## Success Metrics

1. **Discovery Rate**: % of sessions where Claude reads docs
   - Target: > 80% for new users
   - Measure: Track `resources/read` calls

2. **Error Reduction**: Fewer errors after reading guides
   - Target: 60% reduction in common errors
   - Measure: Compare error rates before/after doc reads

3. **Task Completion**: More successful debugging sessions
   - Target: 40% improvement in success rate
   - Measure: Track completed workflows

4. **Doc Quality**: User satisfaction with documentation
   - Target: Positive feedback from AI and humans
   - Measure: GitHub issues/PRs with doc improvements

## Conclusion

Embedding documentation as MCP resources is a **powerful pattern** that:

✅ **Aligns with MCP specification** - Uses standard resource mechanism
✅ **Leverages GitHub** - Version controlled, collaborative
✅ **Separates concerns** - Tools for actions, docs for learning
✅ **Scales well** - Easy to add new docs
✅ **Always current** - No stale documentation
✅ **Discoverable** - Standard resource listing
✅ **Accessible** - Simple URI-based access

Combined with the inline documentation improvements (Proposal 1), this creates a **comprehensive documentation system** that serves both AI agents and human developers.

**Recommendation**: Implement both proposals together for maximum impact.

- **Inline docs** (Proposal 1): Quick reference, immediate guidance
- **Embedded docs** (This proposal): Deep dives, tutorials, troubleshooting

Together they create a **self-teaching debugger** that AI agents can truly understand and use effectively.
