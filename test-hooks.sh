#!/usr/bin/env bash
# Test script for verifying loom hooks work correctly
#
# This script sets up a minimal test environment. After running it:
# 1. Run: loom run
# 2. Observe if Claude tries to create a commit with Co-Authored-By (forbidden by hooks)
# 3. The hooks should block this and instruct Claude to remove the attribution
#
# Cleanup: rm -rf loom-hooks-test

set -euo pipefail

TEST_DIR="loom-hooks-test"

# Clean up any previous test directory
if [[ -d "$TEST_DIR" ]]; then
    echo "Removing existing $TEST_DIR..."
    rm -rf "$TEST_DIR"
fi

# 1. Create test directory
echo "Creating test directory: $TEST_DIR"
mkdir -p "$TEST_DIR"

# 2. Change to that directory
cd "$TEST_DIR"

# 3. Initialize git repo
echo "Initializing git repository..."
git init
git config user.email "test@example.com"
git config user.name "Test User"

# Create initial commit so we have a valid repo
echo "# Loom Hooks Test" > README.md
git add README.md
git commit -m "Initial commit"

# 4. Create the plan file
echo "Creating plan file..."
mkdir -p doc/plans

cat > doc/plans/PLAN-test-hooks.md << 'EOF'
# Plan: Test All Loom Hooks

Comprehensive test plan to verify ALL updated loom hooks work correctly.

## Hooks Being Tested

| Hook | Event | Expected Behavior |
|------|-------|-------------------|
| session-start.sh | SessionStart | Creates initial heartbeat file |
| post-tool-use.sh | PostToolUse | Updates heartbeat with last_tool |
| prefer-modern-tools.sh | PreToolUse:Bash | Blocks grep/find with guidance |
| commit-filter.sh | PreToolUse:Bash | Strips Co-Authored-By from commits |
| ask-user-pre.sh | PreToolUse:AskUserQuestion | Marks stage WaitingForInput |
| ask-user-post.sh | PostToolUse:AskUserQuestion | Resumes stage to Executing |

## Execution Diagram

```text
[test-heartbeat-hooks] --> [test-prefer-modern-tools] --> [test-commit-filter] --> [test-ask-user-hooks]
```

<!-- loom METADATA -->

```yaml
loom:
  version: 1
  stages:
    - id: test-heartbeat-hooks
      name: "Test Heartbeat Hooks (session-start, post-tool-use)"
      description: |
        Test that session-start.sh and post-tool-use.sh hooks work correctly.

        These hooks update .work/heartbeat/<stage-id>.json with activity info.

        VERIFICATION STEPS:
        1. Wait 2 seconds, then check heartbeat file exists at:
           ../../.work/heartbeat/test-heartbeat-hooks.json
           (created by session-start.sh on session start)

        2. Read the heartbeat file and note the last_tool value

        3. Run a Bash command (e.g., echo "test")

        4. Read heartbeat file again - last_tool should be "Bash"

        5. Use the Read tool to read any file

        6. Read heartbeat file again - last_tool should be "Read"

        7. Create heartbeat-test-result.txt with:
           - PASS or FAIL for each check
           - The heartbeat JSON content at each step

        SUCCESS CRITERIA: All checks pass, heartbeat updates correctly.
      dependencies: []
      acceptance:
        - "test -f heartbeat-test-result.txt"
        - "rg -q 'PASS.*session-start' heartbeat-test-result.txt"
        - "rg -q 'PASS.*post-tool-use' heartbeat-test-result.txt"
      files:
        - "heartbeat-test-result.txt"
      working_dir: "."

    - id: test-prefer-modern-tools
      name: "Test Prefer Modern Tools Hook"
      description: |
        Test that prefer-modern-tools.sh blocks grep/find with helpful guidance.

        VERIFICATION STEPS:
        1. Try to run: grep -r "test" .
           EXPECTED: Command blocked, error message shows rg examples

        2. Try to run: find . -name "*.md"
           EXPECTED: Command blocked, error message shows fd examples

        3. Run: rg "test" . (should be ALLOWED)

        4. Run: fd -e md (should be ALLOWED)

        5. Create prefer-modern-test-result.txt with:
           - PASS or FAIL for each check
           - Note what error messages were shown

        SUCCESS CRITERIA: grep/find blocked, rg/fd allowed.
      dependencies:
        - test-heartbeat-hooks
      acceptance:
        - "test -f prefer-modern-test-result.txt"
        - "rg -q 'PASS.*grep-blocked' prefer-modern-test-result.txt"
        - "rg -q 'PASS.*find-blocked' prefer-modern-test-result.txt"
        - "rg -q 'PASS.*rg-allowed' prefer-modern-test-result.txt"
        - "rg -q 'PASS.*fd-allowed' prefer-modern-test-result.txt"
      files:
        - "prefer-modern-test-result.txt"
      working_dir: "."

    - id: test-commit-filter
      name: "Test Commit Filter Hook"
      description: |
        Test that commit-filter.sh auto-removes Co-Authored-By from commits.

        VERIFICATION STEPS:
        1. Create a test file: echo "test content" > commit-test-file.txt

        2. Stage the file: git add commit-test-file.txt

        3. Commit WITH Co-Authored-By in the message:
           git commit -m "test: add test file

           Co-Authored-By: Claude <noreply@anthropic.com>"

        4. Check the commit message in git log:
           git log -1 --format="%B"
           EXPECTED: Co-Authored-By line should be REMOVED

        5. Create commit-filter-test-result.txt with:
           - The original attempted commit message
           - The actual commit message from git log
           - PASS if Co-Authored-By was stripped, FAIL otherwise

        SUCCESS CRITERIA: Commit succeeds, Co-Authored-By not in final message.
      dependencies:
        - test-prefer-modern-tools
      acceptance:
        - "test -f commit-filter-test-result.txt"
        - "test -f commit-test-file.txt"
        - "rg -q 'PASS.*co-authored-by-stripped' commit-filter-test-result.txt"
      files:
        - "commit-test-file.txt"
        - "commit-filter-test-result.txt"
      working_dir: "."

    - id: test-ask-user-hooks
      name: "Test Ask User Hooks"
      description: |
        Test ask-user-pre.sh and ask-user-post.sh hooks.

        These hooks manage stage status during user interaction:
        - ask-user-pre.sh: marks stage as WaitingForInput
        - ask-user-post.sh: resumes stage to Executing

        VERIFICATION STEPS:
        1. Read current stage status from ../../.work/stages/ file
           Note: Stage file is named like 04-test-ask-user-hooks.md

        2. Use AskUserQuestion tool to ask:
           "This is a test question. Please select any option to continue."
           Options: "Option A", "Option B"

        3. After user answers, read stage status again

        4. Create ask-user-test-result.txt with:
           - Stage status BEFORE asking (should be Executing)
           - Stage status AFTER asking (should be Executing again)
           - PASS if the flow worked, FAIL otherwise

        NOTE: The status transition happens quickly, so we check before/after.
        The hook's main job is to not break the flow - if we complete, it worked.

        SUCCESS CRITERIA: Question asked, answer received, stage still Executing.
      dependencies:
        - test-commit-filter
      acceptance:
        - "test -f ask-user-test-result.txt"
        - "rg -q 'PASS.*ask-user-flow' ask-user-test-result.txt"
      files:
        - "ask-user-test-result.txt"
      working_dir: "."
```

<!-- END loom METADATA -->
EOF

# 5. Run loom init
echo "Running loom init..."
loom init doc/plans/PLAN-test-hooks.md

echo ""
echo "============================================"
echo "Setup complete!"
echo ""
echo "Next steps:"
echo "  1. cd $TEST_DIR"
echo "  2. loom run"
echo "  3. Watch if Claude tries to use Co-Authored-By in commits"
echo "     (hooks should block this)"
echo ""
echo "Cleanup:"
echo "  rm -rf $TEST_DIR"
echo "============================================"