#!/bin/bash
#
# Test Results Analyzer
# Analyzes integration test outputs and generates comprehensive summary
#
# Usage: ./scripts/analyze-test-results.sh <artifacts-directory>
# Example: ./scripts/analyze-test-results.sh test-artifacts/
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

ARTIFACTS_DIR="${1:-.}"
DEBUG_LOG="${2:-analysis-debug.log}"

# Initialize debug log
exec 3>&1 4>&2  # Save stdout and stderr file descriptors
exec 1> >(tee -a "$DEBUG_LOG")
exec 2>&1

echo "════════════════════════════════════════════════════════════════"
echo "🔍 Test Results Analysis - Debug Log"
echo "════════════════════════════════════════════════════════════════"
echo "Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo "Artifacts Directory: $ARTIFACTS_DIR"
echo "Debug Log: $DEBUG_LOG"
echo "Working Directory: $(pwd)"
echo "════════════════════════════════════════════════════════════════"
echo

echo "🔍 Analyzing test results in: $ARTIFACTS_DIR"
echo

# Check if jq is available (required for JSON parsing)
echo "📋 Step 1: Checking prerequisites"
echo "  → Checking for jq command..."
if ! command -v jq &> /dev/null; then
    echo "  ❌ jq is not installed"
    echo ""
    echo "jq is required to parse test-results.json files."
    echo "Install it with:"
    echo "  - Debian/Ubuntu: sudo apt-get install jq"
    echo "  - macOS: brew install jq"
    echo "  - Docker: Add 'jq' to apt-get install in Dockerfile.integration-tests"
    echo ""
    exit 1
fi
echo "  ✅ jq found: $(which jq)"
echo "  ✅ jq version: $(jq --version)"
echo

echo "📂 Step 2: Checking artifacts directory structure"
echo "  → Directory: $ARTIFACTS_DIR"
if [[ ! -d "$ARTIFACTS_DIR" ]]; then
    echo "  ❌ Artifacts directory not found: $ARTIFACTS_DIR"
    exit 1
fi
echo "  ✅ Artifacts directory exists"
echo
echo "  → Listing artifacts directory structure:"
ls -lah "$ARTIFACTS_DIR" || echo "  ⚠️  Failed to list directory"
echo
echo "  → Finding test-results.json files:"
find "$ARTIFACTS_DIR" -name "test-results.json" -exec ls -lh {} \; || echo "  ⚠️  No test-results.json files found"
echo

# Function to analyze JSON test results
analyze_language_json() {
    local lang="$1"
    local json_file="$2"

    echo "    🔍 Analyzing JSON for $lang: $json_file" >&2

    # Validate JSON file exists
    if [[ ! -f "$json_file" ]]; then
        echo "      ❌ JSON file does not exist" >&2
        return 1
    fi

    # Check file size
    local file_size=$(stat -f%z "$json_file" 2>/dev/null || stat -c%s "$json_file" 2>/dev/null || echo "0")
    echo "      📏 File size: $file_size bytes" >&2

    if [[ "$file_size" -eq 0 ]]; then
        echo "      ❌ JSON file is empty (0 bytes)" >&2
        return 1
    fi

    # Show first few lines of file
    echo "      📄 File content (first 5 lines):" >&2
    head -5 "$json_file" | sed 's/^/        /' >&2

    # Validate JSON syntax
    echo "      🔧 Validating JSON syntax..." >&2
    if ! jq empty "$json_file" 2>/dev/null; then
        echo "      ❌ Invalid JSON syntax" >&2
        echo "      📋 jq error:" >&2
        jq empty "$json_file" 2>&1 | sed 's/^/        /' >&2
        return 1
    fi
    echo "      ✅ JSON syntax valid" >&2

    # Parse JSON and extract fields
    echo "      📊 Extracting fields..." >&2
    local overall_success=$(jq -r '.test_run.overall_success // false' "$json_file" 2>/dev/null)
    echo "        overall_success: $overall_success" >&2
    local session_started=$(jq -r '.operations.session_started // false' "$json_file" 2>/dev/null)
    echo "        session_started: $session_started" >&2
    local breakpoint_set=$(jq -r '.operations.breakpoint_set // false' "$json_file" 2>/dev/null)
    echo "        breakpoint_set: $breakpoint_set" >&2
    local breakpoint_verified=$(jq -r '.operations.breakpoint_verified // false' "$json_file" 2>/dev/null)
    echo "        breakpoint_verified: $breakpoint_verified" >&2
    local execution_continued=$(jq -r '.operations.execution_continued // false' "$json_file" 2>/dev/null)
    echo "        execution_continued: $execution_continued" >&2
    local stopped_at_breakpoint=$(jq -r '.operations.stopped_at_breakpoint // false' "$json_file" 2>/dev/null)
    echo "        stopped_at_breakpoint: $stopped_at_breakpoint" >&2
    local stack_trace=$(jq -r '.operations.stack_trace_retrieved // false' "$json_file" 2>/dev/null)
    echo "        stack_trace_retrieved: $stack_trace" >&2
    local evaluation=$(jq -r '.operations.variable_evaluated // false' "$json_file" 2>/dev/null)
    echo "        variable_evaluated: $evaluation" >&2
    local disconnect=$(jq -r '.operations.session_disconnected // false' "$json_file" 2>/dev/null)
    echo "        session_disconnected: $disconnect" >&2
    local error_count=$(jq -r '.errors | length // 0' "$json_file" 2>/dev/null)
    echo "        error_count: $error_count" >&2

    # Validate JSON parsing worked
    echo "      🔍 Validating parsed values..." >&2
    if [[ "$overall_success" != "true" && "$overall_success" != "false" ]]; then
        echo "      ❌ Invalid overall_success value: '$overall_success' (expected 'true' or 'false')" >&2
        return 1
    fi
    echo "      ✅ All values parsed successfully" >&2

    # Convert boolean strings to counts (1 for true, 0 for false)
    local session_count=$([[ "$session_started" == "true" ]] && echo 1 || echo 0)
    local bp_count=$([[ "$breakpoint_set" == "true" ]] && echo 1 || echo 0)
    local bp_verified_count=$([[ "$breakpoint_verified" == "true" ]] && echo 1 || echo 0)
    local cont_count=$([[ "$execution_continued" == "true" ]] && echo 1 || echo 0)
    local stopped_count=$([[ "$stopped_at_breakpoint" == "true" ]] && echo 1 || echo 0)
    local stack_count=$([[ "$stack_trace" == "true" ]] && echo 1 || echo 0)
    local eval_count=$([[ "$evaluation" == "true" ]] && echo 1 || echo 0)
    local disc_count=$([[ "$disconnect" == "true" ]] && echo 1 || echo 0)

    # Determine status based on JSON data
    if [[ "$overall_success" == "true" ]]; then
        # All operations succeeded
        echo "$lang|✅ PASS|100%|Fully Functional (JSON)|$session_count|$bp_verified_count|$cont_count|$stack_count|$eval_count|$disc_count"
    elif [[ $error_count -gt 0 ]]; then
        # Has errors, check what's working
        if [[ $session_count -eq 1 && $bp_count -eq 1 ]]; then
            echo "$lang|⚠️  PARTIAL|40%|Limited Functionality (JSON)|$session_count|$bp_verified_count|$cont_count|$stack_count|$eval_count|$disc_count"
        else
            echo "$lang|❌ FAIL|0%|Non-functional (JSON)|$session_count|$bp_verified_count|$cont_count|$stack_count|$eval_count|$disc_count"
        fi
    else
        # Check how many operations succeeded
        local op_count=$((session_count + bp_verified_count + cont_count + stack_count + eval_count + disc_count))
        if [[ $op_count -ge 6 ]]; then
            echo "$lang|✅ PASS|100%|Fully Functional (JSON)|$session_count|$bp_verified_count|$cont_count|$stack_count|$eval_count|$disc_count"
        elif [[ $op_count -ge 4 ]]; then
            echo "$lang|⚠️  PARTIAL|60%|Partially Functional (JSON)|$session_count|$bp_verified_count|$cont_count|$stack_count|$eval_count|$disc_count"
        else
            echo "$lang|⚠️  PARTIAL|40%|Limited Functionality (JSON)|$session_count|$bp_verified_count|$cont_count|$stack_count|$eval_count|$disc_count"
        fi
    fi

    return 0
}

# Function to analyze a single language test output
analyze_language() {
    local lang="$1"
    local file="$2"

    echo "  📊 Analyzing $lang test results" >&2
    echo "    📂 Test output file: $file" >&2

    if [[ ! -f "$file" ]]; then
        echo "    ❌ Test output file not found: $file" >&2
        return 1
    fi

    local file_size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo "0")
    echo "    📏 Test output size: $file_size bytes" >&2

    # Parse JSON test results
    local json_file="${file%/*}/test-results.json"
    echo "    🔍 Looking for test-results.json: $json_file" >&2

    if [[ -f "$json_file" ]]; then
        echo "    ✅ test-results.json found" >&2
        local json_result
        json_result=$(analyze_language_json "$lang" "$json_file" 2>&2)
        local exit_code=$?
        if [[ $exit_code -eq 0 ]]; then
            echo "    ✅ JSON analysis successful" >&2
            echo "$json_result"
            return 0
        fi
        # JSON parsing failed
        echo "    ❌ JSON parsing failed (exit code: $exit_code)" >&2
        echo "$lang|❌ FAIL|0%|Invalid JSON Format|0|0|0|0|0|0"
        return 0
    fi

    # No JSON file found - test didn't generate results
    echo "    ❌ test-results.json not found" >&2
    echo "$lang|❌ FAIL|0%|No Test Results|0|0|0|0|0|0"
    return 0
}

# Analyze all languages
echo "📊 Step 3: Analyzing integration test results" >&2
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
echo >&2

# Define languages and their test output files
# After adding ai_client dimension, artifacts are named test-output-{language}-{ai_client}
# We analyze BOTH Claude and Codex tests for comprehensive coverage
declare -A LANGUAGES=(
    ["Python (claude)"]="$ARTIFACTS_DIR/test-output-python-claude/python-test-output.txt"
    ["Python (codex)"]="$ARTIFACTS_DIR/test-output-python-codex/python-test-output.txt"
    ["Ruby (claude)"]="$ARTIFACTS_DIR/test-output-ruby-claude/ruby-test-output.txt"
    ["Ruby (codex)"]="$ARTIFACTS_DIR/test-output-ruby-codex/ruby-test-output.txt"
    ["Node.js (claude)"]="$ARTIFACTS_DIR/test-output-nodejs-claude/nodejs-test-output.txt"
    ["Node.js (codex)"]="$ARTIFACTS_DIR/test-output-nodejs-codex/nodejs-test-output.txt"
    ["Go (claude)"]="$ARTIFACTS_DIR/test-output-go-claude/go-test-output.txt"
    ["Go (codex)"]="$ARTIFACTS_DIR/test-output-go-codex/go-test-output.txt"
    ["Rust (claude)"]="$ARTIFACTS_DIR/test-output-rust-claude/rust-test-output.txt"
    ["Rust (codex)"]="$ARTIFACTS_DIR/test-output-rust-codex/rust-test-output.txt"
)

echo "📋 Configured languages and their output files:" >&2
for lang in "Python (claude)" "Python (codex)" "Ruby (claude)" "Ruby (codex)" "Node.js (claude)" "Node.js (codex)" "Go (claude)" "Go (codex)" "Rust (claude)" "Rust (codex)"; do
    file="${LANGUAGES[$lang]}"
    echo "  - $lang: $file" >&2
done
echo >&2

# Collect results
declare -a RESULTS=()
declare -A LANG_STATUS=()

echo "🔄 Beginning analysis for each language..." >&2
echo >&2
for lang in "Python (claude)" "Python (codex)" "Ruby (claude)" "Ruby (codex)" "Node.js (claude)" "Node.js (codex)" "Go (claude)" "Go (codex)" "Rust (claude)" "Rust (codex)"; do
    echo "──────────────────────────────────────────────────────────────" >&2
    file="${LANGUAGES[$lang]}"
    result=$(analyze_language "$lang" "$file" || echo "$lang|❌ SKIP|0%|Not Tested|0|0|0|0|0|0")
    RESULTS+=("$result")
    LANG_STATUS[$lang]=$(echo "$result" | cut -d'|' -f2)
    echo "  📌 Result for $lang: ${LANG_STATUS[$lang]}" >&2
    echo >&2
done
echo "──────────────────────────────────────────────────────────────" >&2
echo >&2

echo "📊 Step 4: Generating summary table" >&2
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
echo >&2

echo "📊 INTEGRATION TEST SUMMARY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Print summary table
echo "| Language       | Status      | Pass Rate | Functionality         | Operations |"
echo "|----------------|-------------|-----------|------------------------|------------|"

for result in "${RESULTS[@]}"; do
    IFS='|' read -r lang status rate func session bp cont stack eval disc <<< "$result"

    # Build operations summary
    ops=""
    [[ $session -gt 0 ]] && ops+="S" || ops+="-"
    [[ $bp -gt 0 ]] && ops+="B" || ops+="-"
    [[ $cont -gt 0 ]] && ops+="C" || ops+="-"
    [[ $stack -gt 0 ]] && ops+="T" || ops+="-"
    [[ $eval -gt 0 ]] && ops+="E" || ops+="-"
    [[ $disc -gt 0 ]] && ops+="D" || ops+="-"

    printf "| %-14s | %-11s | %-9s | %-22s | %-10s |\n" "$lang" "$status" "$rate" "$func" "$ops"
done

echo
echo "**Legend:** S=Session Start, B=Breakpoint, C=Continue, T=stack Trace, E=Evaluation, D=Disconnect"
echo

# Calculate overall metrics
total_tests=10  # 5 languages × 2 AI clients (claude + codex)
passing_tests=0
partial_tests=0
failing_tests=0

for lang in "${!LANG_STATUS[@]}"; do
    status="${LANG_STATUS[$lang]}"
    if [[ "$status" == "✅ PASS" ]]; then
        passing_tests=$((passing_tests + 1))
    elif [[ "$status" == "⚠️  PARTIAL" ]]; then
        partial_tests=$((partial_tests + 1))
    else
        failing_tests=$((failing_tests + 1))
    fi
done

overall_rate=$((passing_tests * 100 / total_tests))

echo "### Overall Results"
echo
echo "- **Total Tests:** $total_tests (5 languages × 2 AI clients)"
echo "- **Fully Functional:** $passing_tests ($((passing_tests * 100 / total_tests))%)"
echo "- **Partially Functional:** $partial_tests ($((partial_tests * 100 / total_tests))%)"
echo "- **Non-Functional:** $failing_tests ($((failing_tests * 100 / total_tests))%)"
echo "- **Overall Success Rate:** ${overall_rate}%"
echo

# Determine CI status
# Check for API credit issues first
api_credit_issues=0
for lang in "${!LANG_STATUS[@]}"; do
    status="${LANG_STATUS[$lang]}"
    if [[ "$status" == "⚠️  SKIPPED" ]]; then
        api_credit_issues=$((api_credit_issues + 1))
    fi
done

if [[ $api_credit_issues -gt 0 ]]; then
    echo -e "${RED}🚨 TEST INFRASTRUCTURE FAILURE${NC}"
    echo
    echo "⚠️  **$api_credit_issues test(s) skipped due to API credit exhaustion**"
    echo
    echo "Integration tests could not run due to insufficient API credits."
    echo "This is NOT a functionality issue - it's a test infrastructure problem."
    echo
    echo "**Action Required:**"
    echo "  1. Check API credit balance (Claude/Codex)"
    echo "  2. Add credits or wait for reset"
    echo "  3. Re-run tests to verify actual functionality"
    echo

    # List affected tests
    echo "**Affected Tests:**"
    for lang in "${!LANG_STATUS[@]}"; do
        status="${LANG_STATUS[$lang]}"
        if [[ "$status" == "⚠️  SKIPPED" ]]; then
            echo "  - $lang (test not executed)"
        fi
    done
    exit 2  # Exit code 2 for infrastructure issues
elif [[ $passing_tests -eq $total_tests ]]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED${NC}"
    echo
    echo "All language/AI client combinations are fully functional with complete debugging capabilities."
    exit 0
elif [[ $passing_tests -ge 6 ]]; then
    echo -e "${YELLOW}⚠️  PARTIAL SUCCESS${NC}"
    echo
    echo "Most tests are passing, but some need attention:"
    echo

    # List non-passing tests
    for lang in "${!LANG_STATUS[@]}"; do
        status="${LANG_STATUS[$lang]}"
        if [[ "$status" != "✅ PASS" ]]; then
            file="${LANGUAGES[$lang]}"
            echo "  - **$lang** ($status):"

            # Show specific issues
            if [[ -f "$file" ]]; then
                if grep -q "missing debug symbols" "$file" 2>/dev/null; then
                    echo "    - Missing debug symbols"
                fi
                if grep -q "verified: false" "$file" 2>/dev/null; then
                    echo "    - Breakpoint verification failed"
                fi
                if ! grep -q "Retrieved.*stack trace\|inspected.*stack trace\|Features Tested:.*stack trace" "$file" 2>/dev/null; then
                    echo "    - Stack trace unavailable"
                fi
                if ! grep -q "Evaluated variable\|Variable Evaluations.*successful" "$file" 2>/dev/null; then
                    echo "    - Expression evaluation unavailable"
                fi
            fi
        fi
    done
    exit 1
else
    echo -e "${RED}❌ TESTS FAILED${NC}"
    echo
    echo "Multiple tests are not working correctly. Review test outputs for details."
    exit 1
fi
