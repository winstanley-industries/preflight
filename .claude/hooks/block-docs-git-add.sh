#!/bin/bash
# Blocks git add commands that reference .docs

COMMAND=$(cat | jq -r '.tool_input.command // empty')

if echo "$COMMAND" | grep -q "git add" && echo "$COMMAND" | grep -q "\.docs"; then
  cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": ".docs is gitignored — do not stage these files"
  }
}
EOF
fi

exit 0
