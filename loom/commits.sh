#!/bin/bash
# Commit script for feat-self-propel-with-state branch changes
# Generated from analysis of modified files
# Run from the loom/ directory

set -e  # Exit on error

echo "=== Committing changes in logical groups ==="

# Commit 1: Core stage model with state machine and persistence
echo ""
echo "=== Commit 1: Stage state machine with persistence ==="
git add src/models/stage.rs src/verify/transitions.rs
git commit -m "$(cat <<'EOF'
feat: implement validated stage state machine with persistence

Add Stage model with strict 8-state lifecycle and transition validation:
- WaitingForDeps → Queued → Executing → Completed → Verified
- Additional states: Blocked, NeedsHandoff, WaitingForInput

Implement transition_stage() for persisting validated state changes.
All transitions verified against allowable paths before applying.
Includes comprehensive tests for all state transitions.
EOF
)"

# Commit 2: Execution graph for dependency scheduling
echo ""
echo "=== Commit 2: Execution graph ==="
git add src/plan/graph.rs
git commit -m "$(cat <<'EOF'
feat: implement execution graph for dependency-based scheduling

Add ExecutionGraph DAG with:
- Cycle detection via DFS
- Automatic ready-stage calculation when dependencies complete
- Parallel group support for concurrent execution
- Topological sort for execution order (Kahn's algorithm)

Tracks stage dependencies and determines which stages can run.
EOF
)"

# Commit 3: Daemon orchestrator with Unix socket IPC
echo ""
echo "=== Commit 3: Daemon orchestrator ==="
git add src/daemon/server.rs
git commit -m "$(cat <<'EOF'
feat: implement daemon orchestrator with Unix socket IPC

Add DaemonServer background daemon that:
- Listens on Unix socket for commands (Ping, Stop, Subscribe)
- Orchestrates stage execution based on dependency graph
- Spawns parallel worker sessions for ready stages
- Broadcasts status updates and streams logs to subscribers

Multi-threaded architecture with orchestrator, status broadcaster,
and log tailer threads.
EOF
)"

# Commit 4: Bi-directional state sync and embedded signal context
echo ""
echo "=== Commit 4: State sync and embedded signals ==="
git add src/orchestrator/core/recovery.rs \
        src/orchestrator/core/orchestrator.rs \
        src/orchestrator/core/stage_executor.rs \
        src/orchestrator/signals.rs
git commit -m "$(cat <<'EOF'
feat: add bi-directional state sync and embedded signal context

State synchronization:
- Add sync_queued_status_to_files() to sync graph state to files
- Sync after recovery and in main orchestration loop
- Ensures files reflect when dependencies are satisfied

Embedded signal context:
- Add EmbeddedContext with plan overview, handoff, and structure map
- Embed all context directly in signal files
- Agents no longer need to read from main repo symlinks
EOF
)"

# Commit 5: Session termination via window title
echo ""
echo "=== Commit 5: Improved session termination ==="
git add src/orchestrator/terminal/native.rs
git commit -m "$(cat <<'EOF'
feat: improve session termination via window title closing

- Add close_window_by_title() using wmctrl or xdotool
- Prefer title-based window closing over PID-based killing
- Fixes issues with multi-window terminals (e.g., gnome-terminal)
  where killing by PID would close all windows
- Fall back to PID-based killing if title method unavailable
EOF
)"

# Commit 6: Merge to current branch
echo ""
echo "=== Commit 6: Merge target fix ==="
git add src/commands/merge/execute.rs
git commit -m "$(cat <<'EOF'
fix: merge to current branch instead of default branch

Change merge target from default_branch() to current_branch() to
ensure stages merge to the branch the main repository is currently
on, enabling proper multi-branch workflows.
EOF
)"

# Commit 7: Status enum renames across codebase and tests
echo ""
echo "=== Commit 7: Status enum renames ==="
git add src/commands/graph.rs \
        src/commands/init.rs \
        src/commands/stage.rs \
        src/commands/status/display.rs \
        src/commands/verify.rs \
        src/orchestrator/attach/parsers.rs \
        src/orchestrator/continuation.rs \
        tests/e2e/helpers.rs \
        tests/e2e/manual_mode.rs \
        tests/e2e/parallel.rs \
        tests/e2e/sequential.rs \
        tests/failure_resume.rs \
        tests/stage_transitions.rs
git commit -m "$(cat <<'EOF'
refactor: rename status enums for semantic clarity

Update stage status enum variants across codebase:
- Pending → WaitingForDeps (waiting for dependencies to complete)
- Ready → Queued (queued for execution)

Updates all references in commands, orchestrator, and test files
to use the new semantically clearer names.
EOF
)"

echo ""
echo "=== All commits complete ==="
git log --oneline -7
