# Documentation Cleanup Plan - REVISED

## Philosophy
**Keep ONLY what's relevant RIGHT NOW for:**
1. Understanding the architecture
2. Contributing to the codebase
3. Using debugger_mcp across different languages

Everything else → Obsidian for historical reference.

---

## Answers to Your Questions

### 1. CHANGELOG.md
**Finding**: Actively maintained (last update Oct 7, 2025), contains detailed release notes.
**Decision**: **MOVE TO OBSIDIAN**
- Git tags and releases capture version history
- Too detailed and historical for repository root
- Can be regenerated from git history if needed

### 2. debug_prompt.md
**Finding**: Simple test prompt for Go debugging (11 lines), appears ad-hoc/temporary.
**Decision**: **DELETE**
- Not documentation, just a test note
- No value for users/contributors

### 3. Test Scripts in Root
**Decision**: **DELETE** (as you confirmed)
- test_correct_sequence.py
- test_debugpy_manual.py
- test_launch_sequence.py

### 4. Core Dumps
**Decision**: **DELETE** (as you confirmed)
- core.236, core.238, core.239, core.47, core.690, core.9

---

## REVISED Categorization: Much More Aggressive

### KEEP IN docs/ (Only 16 Files - Down from 29)

#### For Understanding Architecture (4 files)
- `DAP_MCP_SERVER_PROPOSAL.md` ✅ - Primary architecture (68 pages, comprehensive)
- `architecture/COMPONENTS.md` ✅ - Component specifications
- `LOGGING_ARCHITECTURE.md` ✅ - Logging design
- `research/rust-mcp-technology-stack.md` ✅ - Technology choices explained

#### For Contributing (7 files)
- `GETTING_STARTED.md` ✅ - Developer onboarding
- `TESTING.md` ✅ - Testing guide
- `TESTING_STRATEGY.md` ✅ - Testing approach
- `TESTING_EXAMPLE.md` ✅ - Code examples
- `PRE_COMMIT_SETUP.md` ✅ - Setup automation
- `INSTALLATION_CHECKLIST.md` ✅ - Tool installation
- `ADDING_NEW_LANGUAGE.md` ✅ - Extending to new languages

#### For Usage (5 files)
- `README.md` ✅ - Documentation index
- `DOCKER.md` ✅ - Deployment
- `TROUBLESHOOTING.md` ✅ - Common issues
- `EXPRESSION_SYNTAX_GUIDE.md` ✅ - Language-specific expression syntax
- `INTEGRATION_TESTS.md` ✅ - Test specifications

#### Current Processes (4 files - Keep lean)
- `CI_CD_PIPELINE.md` ✅ - CI/CD reference
- `CROSS_PLATFORM_BUILDS.md` ✅ - Build process
- `RELEASE_PROCESS.md` ✅ - How to release
- `LOG_VALIDATION_SYSTEM.md` ✅ - Validation system

**Total kept in docs/: 20 files** (vs 29 originally proposed)

---

### MOVE TO OBSIDIAN (85 Files - Very Aggressive)

#### All Historical Status/Implementation Docs (55+ files)
Everything with STATUS, COMPLETE, FIX, IMPLEMENTATION, SUMMARY in the name:
- All 41 status/completion files from original analysis
- `MVP_IMPLEMENTATION_PLAN.md` ❌ (historical plan)
- `MVP_IMPLEMENTATION_STATUS.md` ❌ (snapshot in time)
- `MVP_STATUS_REPORT.md` ❌ (snapshot in time)
- `IMPLEMENTATION_STATUS_OCT_2025.md` ❌ (snapshot)
- All the detailed fix/solution docs
- All postmortems and lessons learned

#### All Proposals (13 files)
Good for historical context, not needed for current development:
- `DOCKER_STRATEGY_ANALYSIS.md`
- `EMBEDDED_DOCUMENTATION_PROPOSAL.md`
- `ENHANCED_CI_PROPOSAL.md`
- `INTEGRATION_TEST_CI_PROPOSAL.md`
- `MCP_DOCUMENTATION_IMPROVEMENT_PROPOSAL.md`
- `MCP_INTEGRATION_FIX_PROPOSAL.md`
- `PROPOSED_INTEGRATION_TESTS.md`
- Plus others from original list

#### Research (Most of it - 10+ files)
Keep only tech stack doc, move rest:
- `dap-client-research.md`
- `NODEJS_RESEARCH.md`
- `RUBY_SUPPORT_ANALYSIS.md`
- `RUST_DEBUGGING_RESEARCH_AND_PROPOSAL.md`
- `research/go-experiments-log.md`
- `research/go-implementation-plan.md`
- `research/go-testing-strategy.md`
- `research/go-vs-java-comparison.md`
- `research/RESEARCH_SUMMARY.md`

#### Documentation About Documentation (3 files)
Meta docs not needed in repo:
- `DOCUMENTATION_STRATEGY_SUMMARY.md`
- `USER_FEEDBACK_IMPROVEMENTS.md` (historical improvements done)

#### Specialized/Niche Guides (7 files)
Useful but too specific for main docs:
- `CONTAINER_PATH_GUIDE.md`
- `DAP_TIMING_ANALYSIS.md`
- `DAP_EVENT_DRIVEN_DESIGN.md`
- `DAP_PROTOCOL_SEQUENCE.md`
- `NODEJS_CHILD_SESSION_TODO.md`
- `RUBY_DAP_STDIO_ISSUE.md`
- `WORKFLOW_TEST_RESULTS.md`

#### Architecture Deep Dives (8 files)
Valuable but too detailed for repo:
- `MULTI_LANGUAGE_IMPLEMENTATION_SUMMARY.md`
- `MULTI_SESSION_ARCHITECTURE.md`
- `NODEJS_MULTI_SESSION_ARCHITECTURE.md`
- `NODEJS_STOPONENTRY_ANALYSIS.md`
- `TEST_COVERAGE_GAP_ANALYSIS.md`
- `TEST_PYRAMID_ANALYSIS.md`
- `COVERAGE_STRATEGY.md`
- `COVERAGE_PROGRESS.md`

#### Root Folder Summaries (6 files)
- `ENHANCED_CI_SUMMARY.md`
- `IMPLEMENTATION_COMPLETE.md`
- `IMPROVEMENTS_IMPLEMENTED.md`
- `MCP_NOTIFICATIONS.md`
- `NEW_TOOLS_SUMMARY.md`
- `READY_FOR_TESTING.md`

#### Root Folder Other (2 files)
- `CHANGELOG.md` (git history is sufficient)
- `CRITICAL_BUG_FIX.md`
- `RUBY_STOPENTRY_FIX_COMPLETE.md`

**Total moved to Obsidian: ~85 files**

---

### DELETE FROM ROOT (11 Files)

**Core dumps (6):**
- core.236, core.238, core.239, core.47, core.690, core.9

**Test scripts (3):**
- test_correct_sequence.py
- test_debugpy_manual.py
- test_launch_sequence.py

**Temporary/Ad-hoc (2):**
- debug_prompt.md
- CRITICAL_BUG_FIX.md (duplicate)

---

### KEEP IN ROOT (4 Files Only)

- `README.md` ✅ - Main project readme (will update to be concise)
- `CLAUDE.md` ✅ - Claude Code configuration
- `LICENSE` ✅ - Legal
- `CLEANUP_ANALYSIS.md` ✅ - This analysis (temporary, for PR context)

**CHANGELOG.md removed** - git history and releases are sufficient

---

## New Documentation Structure

### Repository Structure (Minimal)
```
debugger_mcp/
├── README.md                    (Concise overview, links to docs/)
├── CLAUDE.md                    (Claude Code config)
├── LICENSE
└── docs/
    ├── README.md                (Documentation index)
    │
    ├── Architecture/            (4 files)
    │   ├── DAP_MCP_SERVER_PROPOSAL.md
    │   ├── COMPONENTS.md
    │   ├── LOGGING_ARCHITECTURE.md
    │   └── rust-mcp-technology-stack.md
    │
    ├── Contributing/            (7 files)
    │   ├── GETTING_STARTED.md
    │   ├── TESTING.md
    │   ├── TESTING_STRATEGY.md
    │   ├── TESTING_EXAMPLE.md
    │   ├── PRE_COMMIT_SETUP.md
    │   ├── INSTALLATION_CHECKLIST.md
    │   └── ADDING_NEW_LANGUAGE.md
    │
    ├── Usage/                   (5 files)
    │   ├── DOCKER.md
    │   ├── TROUBLESHOOTING.md
    │   ├── EXPRESSION_SYNTAX_GUIDE.md
    │   └── INTEGRATION_TESTS.md
    │
    └── Processes/               (4 files)
        ├── CI_CD_PIPELINE.md
        ├── CROSS_PLATFORM_BUILDS.md
        ├── RELEASE_PROCESS.md
        └── LOG_VALIDATION_SYSTEM.md
```

### Obsidian Vault Structure
```
Development Projects/Debugger-MCP/Documentation/
├── Historical-Implementation/   (~55 files)
│   ├── Status-Reports/
│   ├── Completion-Reports/
│   ├── Bug-Fixes/
│   └── Postmortems/
├── Proposals-and-Decisions/     (~13 files)
├── Research/                    (~10 files)
├── Architecture-Deep-Dives/     (~8 files)
├── Specialized-Guides/          (~7 files)
└── Project-Summaries/           (~8 files)
```

---

## Updated README.md Strategy

### Root README.md
**Purpose**: Quick overview for GitHub visitors
**Length**: ~200 lines (currently ~450 lines)
**Content**:
- What it is (1-2 paragraphs)
- Key features (bullet list)
- Quick start (3 commands)
- Supported languages (table)
- Link to docs/ for everything else
- Status badges (CI, coverage, etc.)

### docs/README.md
**Purpose**: Documentation hub for users/contributors
**Length**: ~150 lines (currently ~326 lines)
**Content**:
- Documentation structure overview
- Quick links to key docs
- "Start here for X" sections
- Reorganized to match new folder structure

---

## Benefits of This Aggressive Cleanup

### Repository
- ✅ **85% fewer docs** (20 vs 105)
- ✅ **Clear purpose**: Every doc has a reason to exist NOW
- ✅ **Easy navigation**: Organized by purpose (Architecture/Contributing/Usage/Processes)
- ✅ **Clean root**: 4 files vs 18
- ✅ **Faster onboarding**: Less to read, better organized

### Obsidian Vault
- ✅ **Complete history**: Nothing lost
- ✅ **Better organization**: By type (implementation, research, proposals)
- ✅ **Searchable**: Full-text search across all historical docs
- ✅ **Linkable**: Cross-reference decisions and implementations

### Workflow
- ✅ **Contributors see only what matters NOW**
- ✅ **Historical context available in Obsidian**
- ✅ **README.md stays concise and welcoming**
- ✅ **Documentation stays current** (easier to maintain)

---

## Next Steps

1. **Create Obsidian folder structure** (6 folders)
2. **Move 85 files** to Obsidian with organization
3. **Reorganize docs/** into subdirectories (Architecture/Contributing/Usage/Processes)
4. **Delete 11 files** from root
5. **Rewrite docs/README.md** to match new structure
6. **Rewrite root README.md** to be concise with links
7. **Commit with detailed message**

**Ready to execute?**
