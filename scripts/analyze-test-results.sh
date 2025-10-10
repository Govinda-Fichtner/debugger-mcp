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

    # Extract test result
    local test_result=$(grep "test result:" "$file" | tail -1)

    # Check for critical proof points
    local session_started=$(grep -c "debug session started:" "$file" || true)
    local breakpoint_set=$(grep -c "Breakpoint set, verified: true" "$file" || true)
    local execution_continued=$(grep -c "Execution continued" "$file" || true)
    local stack_trace=$(grep -c "Stack trace retrieved:" "$file" || true)
    local evaluation=$(grep -c "Evaluation result:" "$file" || true)
    local disconnect=$(grep -c "Session disconnected successfully" "$file" || true)

    # Determine overall status
    local status="❌ FAIL"
    local pass_rate="0%"
    local functionality="Non-functional"

    if [[ $test_result =~ "ok" ]] && [[ $session_started -gt 0 ]] && \
       [[ $breakpoint_set -gt 0 ]] && [[ $execution_continued -gt 0 ]] && \
       [[ $stack_trace -gt 0 ]] && [[ $evaluation -gt 0 ]] && \
       [[ $disconnect -gt 0 ]]; then
        status="✅ PASS"
        pass_rate="100%"
        functionality="Fully Functional"
    elif [[ $test_result =~ "ok" ]] && [[ $session_started -gt 0 ]] && \
         [[ $breakpoint_set -gt 0 ]] && [[ $execution_continued -gt 0 ]]; then
        status="⚠️  PARTIAL"
        if [[ $stack_trace -gt 0 ]] || [[ $evaluation -gt 0 ]]; then
            pass_rate="80%"
            functionality="Mostly Functional"
        else
            pass_rate="60%"
            functionality="Partially Functional"
        fi
    elif [[ $test_result =~ "ok" ]]; then
        status="⚠️  PARTIAL"
        pass_rate="40%"
        functionality="Limited Functionality"
    fi

    echo "$lang|$status|$pass_rate|$functionality|$session_started|$breakpoint_set|$execution_continued|$stack_trace|$evaluation|$disconnect"
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
        ((passing_langs++))
    elif [[ "$status" == "⚠️  PARTIAL" ]]; then
        ((partial_langs++))
    else
        ((failing_langs++))
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
if [[ $passing_langs -eq $total_langs ]]; then
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
                if ! grep -q "Stack trace retrieved:" "$file" 2>/dev/null; then
                    echo "    - Stack trace unavailable"
                fi
                if ! grep -q "Evaluation result:" "$file" 2>/dev/null; then
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
