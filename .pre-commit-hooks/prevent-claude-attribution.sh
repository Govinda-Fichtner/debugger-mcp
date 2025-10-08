#!/bin/bash
# Prevent Claude Code attribution in commit messages
# Blocks commits containing automated attribution text

COMMIT_MSG_FILE=$1

if grep -qiE "(generated with.*claude|co-authored-by.*claude)" "$COMMIT_MSG_FILE"; then
    echo "❌ ERROR: Claude attribution found in commit message!" >&2
    echo "" >&2
    echo "Remove lines containing:" >&2
    echo "  - 'Generated with Claude Code'" >&2
    echo "  - 'Co-Authored-By: Claude'" >&2
    echo "" >&2
    exit 1
fi

exit 0
