#!/usr/bin/env bash
# prefer-modern-tools.sh - PreToolUse hook to guide CLI tool selection
#
# This hook intercepts Bash commands and provides guidance:
#
# For grep:
#   - Standard: Use Claude Code's native Grep tool
#   - Advanced (flags, pipes): Use 'rg' (ripgrep) instead of 'grep'
#
# For find:
#   - Standard: Use Claude Code's native Glob tool
#   - Advanced (flags, pipes): Use 'fd' instead of 'find'
#
# Per CLAUDE.md rule 6:
#   "If you must use CLI search, use `rg` or `fd` — never `grep` or `find`."
#
# Environment variables:
#   TOOL_NAME  - Name of the tool being invoked (from Claude Code)
#   TOOL_INPUT - The tool's input (command string for Bash)
#
# Exit codes:
#   0 - Allow the command to proceed
#   2 - Block the command and return guidance to Claude
#
# Output format when blocking:
#   {"continue": false, "reason": "..."}

set -euo pipefail

# Only check Bash tool uses
if [[ "${TOOL_NAME:-}" != "Bash" ]]; then
    exit 0
fi

# Get the command being executed
COMMAND="${TOOL_INPUT:-}"

if [[ -z "$COMMAND" ]]; then
    exit 0
fi

# Helper to output blocking JSON and exit
block_with_reason() {
    local reason="$1"
    # Escape special characters for JSON
    reason="${reason//\\/\\\\}"  # Escape backslashes
    reason="${reason//\"/\\\"}"  # Escape quotes
    reason="${reason//$'\n'/\\n}"  # Escape newlines
    reason="${reason//$'\r'/}"  # Remove carriage returns

    printf '{"continue": false, "reason": "%s"}\n' "$reason"
    exit 2
}

# Check if command uses grep
uses_grep() {
    local cmd="$1"
    echo "$cmd" | grep -qE '(^|[[:space:]])(\/usr\/bin\/|\/bin\/)?grep[[:space:]]'
}

# Check if command uses find
uses_find() {
    local cmd="$1"
    echo "$cmd" | grep -qE '(^|[[:space:]])(\/usr\/bin\/|\/bin\/)?find[[:space:]]'
}

# Check for grep usage
if uses_grep "$COMMAND"; then
    block_with_reason "DO NOT USE 'grep' - choose the right tool for your needs:

STANDARD (simple pattern searches):
  Use Claude Code's native Grep tool instead of bash grep.

  Example: Grep(pattern='pattern', path='.', output_mode='content')

  The Grep tool supports: -A/-B/-C (context), -i (case insensitive),
  -n (line numbers), glob filtering, and various output modes.

ADVANCED (piped commands, special flags like -v, -o, -w):
  Use 'rg' (ripgrep) instead of 'grep'.

  Instead of:  grep -v pattern file | grep other
  Use:         rg -v pattern file | rg other

  'rg' is faster, respects .gitignore, and has better defaults.

Choose the appropriate tool and retry."
fi

# Check for find usage
if uses_find "$COMMAND"; then
    block_with_reason "DO NOT USE 'find' - choose the right tool for your needs:

STANDARD (simple file pattern matching):
  Use Claude Code's native Glob tool instead of bash find.

  Example: Glob(pattern='**/*.rs')

  The Glob tool is fast, returns files sorted by modification time,
  and integrates with Claude Code's context.

ADVANCED (piped commands, -exec, -mtime, -size, -perm, etc.):
  Use 'fd' instead of 'find'.

  Instead of:  find . -name '*.rs' -mtime -7 | xargs wc -l
  Use:         fd -e rs --changed-within 1w | xargs wc -l

  'fd' is faster, has simpler syntax, and respects .gitignore.

Choose the appropriate tool and retry."
fi

# Command is allowed
exit 0
