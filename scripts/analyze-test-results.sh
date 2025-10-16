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

echo "🔍 Analyzing test results in: $ARTIFACTS_DIR"
echo

# Function to analyze a single language test output
analyze_language() {
    local lang="$1"
    local file="$2"

    if [[ ! -f "$file" ]]; then
        echo "⚠️  File not found: $file"
        return 1
    fi

    # Extract overall test result
    local test_result=$(grep "test result:" "$file" | tail -1)

    # If tests failed to run, mark as failure
    if [[ ! $test_result =~ "ok" ]]; then
        echo "$lang|❌ FAIL|0%|Tests Failed|0|0|0|0|0|0"
        return
    fi

    # Look for Claude Code integration test success markers
    # These are definitive proof of full functionality
    local claude_success=$(grep -c "All tasks completed successfully\|All objectives completed successfully" "$file" || true)

    # Look for comprehensive feature list from Claude Code test
    # This pattern indicates the test exercised all debugging capabilities
    # Different languages use different formats:
    # - Go/Rust: "Features Tested: ... stack trace ... variable"
    # - Python: "inspected stack trace and evaluated ... variable"
    local comprehensive_features=$(grep -c "Features Tested:.*stack trace.*variable\|variable.*stack trace\|inspected stack trace and evaluated" "$file" || true)

    # Check for test infrastructure issues (not capability issues)
    local credit_balance_low=$(grep -c "Credit balance is too low\|credit.*too low\|insufficient.*credit" "$file" || true)

    # Check for explicit failure indicators
    local verified_false=$(grep -c "verified: false" "$file" || true)
    local missing_symbols=$(grep -c "missing debug symbols" "$file" || true)
    local breakpoint_never_hit=$(grep -c "Breakpoint Never Hit\|breakpoint never hit" "$file" || true)

    # Basic operations (required for any passing score)
    # Different output formats: "Session started:" or "Starting ... debug session"
    local session_started=$(grep -iEc "session started:|Starting.*debug session" "$file" || true)
    local breakpoint_set=$(grep -ic "Breakpoint set, verified: true" "$file" || true)
    local execution_continued=$(grep -ic "Execution continued" "$file" || true)
    local disconnect=$(grep -ic "Session disconnected successfully\|Disconnected.*session" "$file" || true)

    # Advanced operations (look for positive evidence only)
    # These patterns indicate SUCCESS, not just mentions
    local stack_trace=$(grep -iEc "Retrieved.*stack trace|inspected.*stack trace|Features Tested:.*stack trace" "$file" || true)
    local evaluation=$(grep -iEc "Evaluated variable|Evaluated expression|Inspected variable|Variable Evaluations:.*successful|evaluating expressions" "$file" || true)

    # DECISION LOGIC
    # Priority 0: Check for test infrastructure failures (not capability issues)
    if [[ $credit_balance_low -gt 0 ]]; then
        # Claude Code test failed due to API issues, not functionality issues
        if [[ $session_started -gt 0 ]] && [[ $breakpoint_set -gt 0 ]] && [[ $execution_continued -gt 0 ]]; then
            echo "$lang|⚠️  SKIPPED|N/A|API Credit Exhausted|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        else
            echo "$lang|⚠️  SKIPPED|N/A|API Credit Exhausted|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        fi
        return
    fi

    # Priority 1: If Claude Code test shows comprehensive success → 100% PASS
    if [[ $claude_success -gt 0 ]] && [[ $comprehensive_features -gt 0 ]]; then
        echo "$lang|✅ PASS|100%|Fully Functional|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        return
    fi

    # Priority 2: If has explicit failures → Limited functionality
    if [[ $verified_false -gt 0 ]] || [[ $missing_symbols -gt 0 ]] || [[ $breakpoint_never_hit -gt 0 ]]; then
        if [[ $session_started -gt 0 ]]; then
            echo "$lang|⚠️  PARTIAL|40%|Limited Functionality|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        else
            echo "$lang|❌ FAIL|0%|Non-functional|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        fi
        return
    fi

    # Priority 3: Check if all 6 core operations have positive evidence
    if [[ $session_started -gt 0 ]] && [[ $breakpoint_set -gt 0 ]] && \
       [[ $execution_continued -gt 0 ]] && [[ $stack_trace -gt 0 ]] && \
       [[ $evaluation -gt 0 ]] && [[ $disconnect -gt 0 ]]; then
        echo "$lang|✅ PASS|100%|Fully Functional|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        return
    fi

    # Priority 4: Partial functionality - basic operations work
    if [[ $session_started -gt 0 ]] && [[ $breakpoint_set -gt 0 ]] && \
       [[ $execution_continued -gt 0 ]] && [[ $disconnect -gt 0 ]]; then
        if [[ $stack_trace -gt 0 ]] || [[ $evaluation -gt 0 ]]; then
            echo "$lang|⚠️  PARTIAL|80%|Mostly Functional|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        else
            echo "$lang|⚠️  PARTIAL|60%|Partially Functional|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
        fi
        return
    fi

    # Default: Non-functional
    echo "$lang|❌ FAIL|0%|Non-functional|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
}

# Analyze all languages
echo "📊 INTEGRATION TEST SUMMARY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Define languages and their test output files
declare -A LANGUAGES=(
    ["Python"]="$ARTIFACTS_DIR/test-output-python/python-test-output.txt"
    ["Ruby"]="$ARTIFACTS_DIR/test-output-ruby/ruby-test-output.txt"
    ["Node.js"]="$ARTIFACTS_DIR/test-output-nodejs/nodejs-test-output.txt"
    ["Go"]="$ARTIFACTS_DIR/test-output-go/go-test-output.txt"
    ["Rust"]="$ARTIFACTS_DIR/test-output-rust/rust-test-output.txt"
)

# Collect results
declare -a RESULTS=()
declare -A LANG_STATUS=()

for lang in "Python" "Ruby" "Node.js" "Go" "Rust"; do
    file="${LANGUAGES[$lang]}"
    result=$(analyze_language "$lang" "$file" 2>/dev/null || echo "$lang|❌ SKIP|0%|Not Tested|0|0|0|0|0|0")
    RESULTS+=("$result")
    LANG_STATUS[$lang]=$(echo "$result" | cut -d'|' -f2)
done

# Print summary table
echo "| Language | Status | Pass Rate | Functionality | Operations |"
echo "|----------|--------|-----------|---------------|------------|"

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

    printf "| %-8s | %-11s | %-9s | %-17s | %-10s |\n" "$lang" "$status" "$rate" "$func" "$ops"
done

echo
echo "**Legend:** S=Session Start, B=Breakpoint, C=Continue, T=stack Trace, E=Evaluation, D=Disconnect"
echo

# Calculate overall metrics
total_langs=5
passing_langs=0
partial_langs=0
failing_langs=0

for lang in "${!LANG_STATUS[@]}"; do
    status="${LANG_STATUS[$lang]}"
    if [[ "$status" == "✅ PASS" ]]; then
        passing_langs=$((passing_langs + 1))
    elif [[ "$status" == "⚠️  PARTIAL" ]]; then
        partial_langs=$((partial_langs + 1))
    else
        failing_langs=$((failing_langs + 1))
    fi
done

overall_rate=$((passing_langs * 100 / total_langs))

echo "### Overall Results"
echo
echo "- **Total Languages:** $total_langs"
echo "- **Fully Functional:** $passing_langs ($((passing_langs * 100 / total_langs))%)"
echo "- **Partially Functional:** $partial_langs ($((partial_langs * 100 / total_langs))%)"
echo "- **Non-Functional:** $failing_langs ($((failing_langs * 100 / total_langs))%)"
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
    echo "⚠️  **$api_credit_issues language(s) skipped due to API credit exhaustion**"
    echo
    echo "Claude Code integration tests could not run due to insufficient API credits."
    echo "This is NOT a functionality issue - it's a test infrastructure problem."
    echo
    echo "**Action Required:**"
    echo "  1. Check Claude API credit balance"
    echo "  2. Add credits or wait for reset"
    echo "  3. Re-run tests to verify actual functionality"
    echo

    # List affected languages
    echo "**Affected Languages:**"
    for lang in "${!LANG_STATUS[@]}"; do
        status="${LANG_STATUS[$lang]}"
        if [[ "$status" == "⚠️  SKIPPED" ]]; then
            echo "  - $lang (comprehensive test not executed)"
        fi
    done
    exit 2  # Exit code 2 for infrastructure issues
elif [[ $passing_langs -eq $total_langs ]]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED${NC}"
    echo
    echo "All languages are fully functional with complete debugging capabilities."
    exit 0
elif [[ $passing_langs -ge 3 ]]; then
    echo -e "${YELLOW}⚠️  PARTIAL SUCCESS${NC}"
    echo
    echo "Most languages are working, but some need attention:"
    echo

    # List non-passing languages
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
    echo "Multiple languages are not working correctly. Review test outputs for details."
    exit 1
fi
