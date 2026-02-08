#!/bin/bash
# Runs `just check` before allowing `gh pr create` commands

COMMAND=$(cat | jq -r '.tool_input.command // empty')

if echo "$COMMAND" | grep -q "gh pr create"; then
  if ! just check >&2; then
    cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "just check failed — fix issues before creating a PR"
  }
}
EOF
  fi
fi

exit 0
