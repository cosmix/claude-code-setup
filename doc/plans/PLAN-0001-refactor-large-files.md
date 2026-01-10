<!-- loom METADATA -->

```yaml
loom:
  version: 1
  stages:
    - id: stage-1-foundation-models
      name: "Foundation Models (stage.rs, session.rs)"
      dependencies: []
      parallel_group: "stage-1"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/models/stage.rs"
        - "loom/src/models/session.rs"
        - "loom/src/models/stage/**/*.rs"
        - "loom/src/models/session/**/*.rs"

    - id: stage-2-validation
      name: "Validation (schema.rs, criteria.rs)"
      dependencies: ["stage-1-foundation-models"]
      parallel_group: "stage-2"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/plan/schema.rs"
        - "loom/src/verify/criteria.rs"
        - "loom/src/plan/schema/**/*.rs"
        - "loom/src/verify/criteria/**/*.rs"

    - id: stage-3-transitions
      name: "Transitions (transitions.rs)"
      dependencies: ["stage-2-validation"]
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/verify/transitions.rs"
        - "loom/src/verify/transitions/**/*.rs"

    - id: stage-4-init-command
      name: "Init Command (init.rs)"
      dependencies: ["stage-3-transitions"]
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/commands/init.rs"
        - "loom/src/commands/init/**/*.rs"

    - id: stage-5-stage-monitor
      name: "Stage Command & Monitor"
      dependencies: ["stage-4-init-command"]
      parallel_group: "stage-5"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/commands/stage.rs"
        - "loom/src/orchestrator/monitor.rs"
        - "loom/src/commands/stage/**/*.rs"
        - "loom/src/orchestrator/monitor/**/*.rs"

    - id: stage-6-signals-server
      name: "Signals & Server"
      dependencies: ["stage-5-stage-monitor"]
      parallel_group: "stage-6"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/orchestrator/signals.rs"
        - "loom/src/daemon/server.rs"
        - "loom/src/orchestrator/signals/**/*.rs"
        - "loom/src/daemon/server/**/*.rs"

    - id: stage-7-independent-modules
      name: "Independent Modules (terminal, completions, handoff, graph)"
      dependencies: ["stage-6-signals-server"]
      parallel_group: "stage-7"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/orchestrator/terminal/tmux.rs"
        - "loom/src/orchestrator/terminal/native.rs"
        - "loom/src/completions/dynamic.rs"
        - "loom/src/handoff/generator.rs"
        - "loom/src/plan/graph.rs"
        - "loom/src/orchestrator/terminal/tmux/**/*.rs"
        - "loom/src/orchestrator/terminal/native/**/*.rs"
        - "loom/src/completions/dynamic/**/*.rs"
        - "loom/src/handoff/generator/**/*.rs"
        - "loom/src/plan/graph/**/*.rs"

    - id: stage-8-fs-git
      name: "FS & Git Modules"
      dependencies: ["stage-7-independent-modules"]
      parallel_group: "stage-8"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/fs/worktree_files.rs"
        - "loom/src/fs/permissions.rs"
        - "loom/src/git/cleanup.rs"
        - "loom/src/git/worktree.rs"
        - "loom/src/fs/worktree_files/**/*.rs"
        - "loom/src/fs/permissions/**/*.rs"
        - "loom/src/git/cleanup/**/*.rs"
        - "loom/src/git/worktree/**/*.rs"

    - id: stage-9-commands-orchestrator
      name: "Commands & Orchestrator"
      dependencies: ["stage-8-fs-git"]
      parallel_group: "stage-9"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/src/commands/run.rs"
        - "loom/src/commands/graph.rs"
        - "loom/src/commands/status/display.rs"
        - "loom/src/commands/merge/execute.rs"
        - "loom/src/orchestrator/continuation.rs"
        - "loom/src/orchestrator/attach/mod.rs"
        - "loom/src/commands/run/**/*.rs"
        - "loom/src/commands/graph/**/*.rs"
        - "loom/src/commands/status/display/**/*.rs"
        - "loom/src/commands/merge/execute/**/*.rs"
        - "loom/src/orchestrator/continuation/**/*.rs"
        - "loom/src/orchestrator/attach/**/*.rs"

    - id: stage-10-test-helpers
      name: "Test Helpers"
      dependencies: ["stage-9-commands-orchestrator"]
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/tests/e2e/helpers.rs"
        - "loom/tests/e2e/helpers/**/*.rs"

    - id: stage-11-test-modules
      name: "Test Modules"
      dependencies: ["stage-10-test-helpers"]
      parallel_group: "stage-11"
      acceptance:
        - "cargo build"
        - "cargo test"
        - "cargo clippy -- -D warnings"
      files:
        - "loom/tests/e2e/fixtures.rs"
        - "loom/tests/e2e/daemon_config.rs"
        - "loom/tests/e2e/criteria_validation.rs"
        - "loom/tests/e2e/handoff.rs"
        - "loom/tests/e2e/sessions.rs"
        - "loom/tests/e2e/fixtures/**/*.rs"
        - "loom/tests/e2e/daemon_config/**/*.rs"
        - "loom/tests/e2e/criteria_validation/**/*.rs"
        - "loom/tests/e2e/handoff/**/*.rs"
        - "loom/tests/e2e/sessions/**/*.rs"
```

<!-- END loom METADATA -->

# Refactoring Plan: Files Exceeding 400 Lines

## Summary

**31 files** exceed the 400-line limit defined in CLAUDE.md:

- **25 source files** in `loom/src` (~17,000 excess lines)
- **6 test files** in `loom/tests/e2e` (~1,000 excess lines)

Target: Split each file to **150-350 lines** following existing module patterns.

---

## Execution Diagram

```
[Stage 1] ─────────────────────────────────────────────────────────────────────┐
  models/stage.rs, models/session.rs                              (PARALLEL)   │
                                                                               │
[Stage 2] ─────────────────────────────────────────────────────────────────────┤
  plan/schema.rs, verify/criteria.rs                              (PARALLEL)   │
                                                                               │
[Stage 3] ─────────────────────────────────────────────────────────────────────┤
  verify/transitions.rs                                           (SERIAL)     │
                                                                               │
[Stage 4] ─────────────────────────────────────────────────────────────────────┤
  commands/init.rs                                                (SERIAL)     │
                                                                               │
[Stage 5] ─────────────────────────────────────────────────────────────────────┤
  commands/stage.rs, orchestrator/monitor.rs                      (PARALLEL)   │
                                                                               │
[Stage 6] ─────────────────────────────────────────────────────────────────────┤
  orchestrator/signals.rs, daemon/server.rs                       (PARALLEL)   │
                                                                               │
[Stage 7] ─────────────────────────────────────────────────────────────────────┤
  orchestrator/terminal/tmux.rs, orchestrator/terminal/native.rs  (PARALLEL)   │
  completions/dynamic.rs, handoff/generator.rs, plan/graph.rs                  │
                                                                               │
[Stage 8] ─────────────────────────────────────────────────────────────────────┤
  fs/worktree_files.rs, fs/permissions.rs                         (PARALLEL)   │
  git/cleanup.rs, git/worktree.rs                                              │
                                                                               │
[Stage 9] ─────────────────────────────────────────────────────────────────────┤
  commands/run.rs, commands/graph.rs                              (PARALLEL)   │
  commands/status/display.rs, commands/merge/execute.rs                        │
  orchestrator/continuation.rs, orchestrator/attach/mod.rs                     │
                                                                               │
[Stage 10] ────────────────────────────────────────────────────────────────────┤
  tests/e2e/helpers.rs                                            (SERIAL)     │
                                                                               │
[Stage 11] ────────────────────────────────────────────────────────────────────┘
  tests/e2e/fixtures.rs, tests/e2e/daemon_config.rs               (PARALLEL)
  tests/e2e/criteria_validation.rs, tests/e2e/handoff.rs
  tests/e2e/sessions.rs
```

---

## Critical Files (800+ lines) - Stages 1-6

### Stage 1: Foundation Models (PARALLEL)

#### 1.1 `models/stage.rs` (867 lines)

```
models/stage/
  mod.rs           (~30 lines)  - pub use types::{Stage, StageStatus}
  types.rs         (~110 lines) - Stage struct, StageStatus enum
  transitions.rs   (~90 lines)  - State machine methods (can_transition_to, try_transition)
  methods.rs       (~160 lines) - Stage methods (new, add_dependency, try_*)
  tests.rs         (~480 lines) - All tests
```

#### 1.2 `models/session.rs` (747 lines)

```
models/session/
  mod.rs           (~30 lines)  - pub use types::{Session, SessionStatus, SessionType}
  types.rs         (~80 lines)  - Session struct, SessionType, SessionStatus enums
  transitions.rs   (~80 lines)  - SessionStatus state machine
  methods.rs       (~120 lines) - Session methods (new, new_merge, try_*, context_health)
  tests.rs         (~430 lines) - All tests
```

### Stage 2: Validation (PARALLEL)

#### 2.1 `plan/schema.rs` (886 lines)

```
plan/schema/
  mod.rs           (~30 lines)  - Re-exports
  types.rs         (~60 lines)  - LoomMetadata, LoomConfig, StageDefinition, ValidationError
  validation.rs    (~100 lines) - validate, validate_acceptance_criterion
  tests.rs         (~690 lines) - All tests
```

#### 2.2 `verify/criteria.rs` (869 lines)

```
verify/criteria/
  mod.rs           (~40 lines)  - Re-exports
  config.rs        (~40 lines)  - CriteriaConfig, DEFAULT_COMMAND_TIMEOUT
  result.rs        (~110 lines) - CriterionResult, AcceptanceResult
  runner.rs        (~100 lines) - run_acceptance, run_acceptance_with_config
  executor.rs      (~120 lines) - run_single_criterion, spawn_shell_command
  tests.rs         (~455 lines) - All tests
```

### Stage 3: Transitions (SERIAL)

#### 3.1 `verify/transitions.rs` (834 lines)

```
verify/transitions/
  mod.rs           (~30 lines)  - Re-exports
  state.rs         (~80 lines)  - transition_stage, trigger_dependents
  persistence.rs   (~110 lines) - load_stage, save_stage, list_all_stages
  serialization.rs (~70 lines)  - parse_stage_from_markdown, serialize_stage_to_markdown
  tests.rs         (~475 lines) - All tests
```

### Stage 4: Init Command (SERIAL)

#### 4.1 `commands/init.rs` (791 lines)

```
commands/init/
  mod.rs           (~30 lines)  - pub use execute::execute
  execute.rs       (~80 lines)  - execute, print_header, print_summary
  plan_setup.rs    (~150 lines) - initialize_with_plan, create_stage_from_definition
  cleanup.rs       (~120 lines) - All cleanup functions
  tests.rs         (~310 lines) - All tests
```

**Note:** Remove duplicate `serialize_stage_to_markdown`, import from `verify/transitions`.

### Stage 5: Stage Command & Monitor (PARALLEL)

#### 5.1 `commands/stage.rs` (968 lines)

```
commands/stage/
  mod.rs           (~40 lines)  - Re-exports
  complete.rs      (~110 lines) - complete(), cleanup_terminal_for_stage
  session.rs       (~120 lines) - find_session_for_stage, cleanup_session_resources
  state.rs         (~150 lines) - block, reset, ready, waiting, hold, release
  skip_retry.rs    (~60 lines)  - skip, retry
  tests.rs         (~480 lines) - All tests
```

#### 5.2 `orchestrator/monitor.rs` (941 lines)

```
orchestrator/monitor/
  mod.rs           (~50 lines)  - Re-exports
  config.rs        (~40 lines)  - MonitorConfig
  events.rs        (~50 lines)  - MonitorEvent enum
  core.rs          (~150 lines) - Monitor struct, new, poll
  detection.rs     (~150 lines) - detect_stage_changes, detect_session_changes
  handlers.rs      (~100 lines) - handle_context_critical, handle_session_crash
  context.rs       (~60 lines)  - ContextHealth, context_health, context_usage_percent
  tests.rs         (~380 lines) - All tests
```

### Stage 6: Signals & Server (PARALLEL)

#### 6.1 `orchestrator/signals.rs` (1,355 lines) - LARGEST FILE

```
orchestrator/signals/
  mod.rs           (~40 lines)  - Re-exports
  types.rs         (~60 lines)  - EmbeddedContext, DependencyStatus, SignalContent
  generate.rs      (~150 lines) - generate_signal, build_embedded_context
  merge.rs         (~120 lines) - generate_merge_signal, read_merge_signal
  crud.rs          (~80 lines)  - update_signal, remove_signal, read_signal, list_signals
  format.rs        (~150 lines) - format_signal_content, format_dependency_table
  tests.rs         (~570 lines) - All tests
```

#### 6.2 `daemon/server.rs` (1,169 lines)

```
daemon/server/
  mod.rs           (~50 lines)  - pub use core::DaemonServer
  core.rs          (~120 lines) - DaemonServer struct, new, with_config
  lifecycle.rs     (~150 lines) - start, run_foreground, shutdown
  orchestrator.rs  (~150 lines) - spawn_orchestrator, build_execution_graph
  broadcast.rs     (~150 lines) - spawn_log_tailer, spawn_status_broadcaster
  status.rs        (~200 lines) - collect_status, detect_worktree_status
  client.rs        (~100 lines) - handle_client_connection
  tests.rs         (~250 lines) - All tests
```

---

## Moderate Files (400-800 lines) - Stages 7-9

### Stage 7: Independent Modules (PARALLEL)

| File                              | Lines | Split To                                         |
| --------------------------------- | ----- | ------------------------------------------------ |
| `orchestrator/terminal/tmux.rs`   | 697   | `tmux/{mod,session_ops,helpers,query,types}.rs`  |
| `orchestrator/terminal/native.rs` | 552   | `native/{mod,detection,spawner,window_ops}.rs`   |
| `completions/dynamic.rs`          | 533   | `dynamic/{mod,plans,stages,sessions}.rs`         |
| `handoff/generator.rs`            | 553   | `generator/{mod,content,formatter,numbering}.rs` |
| `plan/graph.rs`                   | 480   | `graph/{mod,nodes,cycle,scheduling}.rs`          |

### Stage 8: FS & Git Modules (PARALLEL)

| File                   | Lines | Split To                                                  |
| ---------------------- | ----- | --------------------------------------------------------- |
| `fs/worktree_files.rs` | 647   | `worktree_files/{mod,cleanup,sessions,signals,config}.rs` |
| `fs/permissions.rs`    | 646   | `permissions/{mod,hooks,settings,trust,constants}.rs`     |
| `git/cleanup.rs`       | 526   | `cleanup/{mod,worktree,branch,config,batch}.rs`           |
| `git/worktree.rs`      | 464   | `worktree/{mod,settings,operations,parser,checks}.rs`     |

### Stage 9: Commands & Orchestrator (PARALLEL)

| File                           | Lines | Split To                                              |
| ------------------------------ | ----- | ----------------------------------------------------- |
| `commands/run.rs`              | 640   | `run/{mod,foreground,graph_loader,frontmatter}.rs`    |
| `commands/graph.rs`            | 555   | `graph/{mod,display,levels,indicators}.rs`            |
| `commands/status/display.rs`   | 566   | `display/{mod,runners,stages,sessions,worktrees}.rs`  |
| `commands/merge/execute.rs`    | 462   | `execute/{mod,recovery,operations,session}.rs`        |
| `orchestrator/continuation.rs` | 601   | `continuation/{mod,context,session_io,yaml_parse}.rs` |
| `orchestrator/attach/mod.rs`   | 597   | Move content to `attach/{types,helpers,loaders}.rs`   |

---

## Test Files - Stages 10-11

### Stage 10: Test Helpers (SERIAL - other tests depend on this)

#### `tests/e2e/helpers.rs` (507 lines)

```
tests/e2e/helpers/
  mod.rs           (~40 lines)  - Re-exports all public functions
  git.rs           (~70 lines)  - create_temp_git_repo, init_loom_with_plan
  stage_io.rs      (~120 lines) - create_stage_file, read_stage_file
  session_io.rs    (~100 lines) - create_session_file, read_session_file, create_signal_file
  utils.rs         (~80 lines)  - is_tmux_available, cleanup_tmux_sessions, wait_for_condition
  tests.rs         (~180 lines) - All internal tests
```

### Stage 11: Test Modules (PARALLEL)

| File                               | Lines | Split To                                                                  |
| ---------------------------------- | ----- | ------------------------------------------------------------------------- |
| `tests/e2e/fixtures.rs`            | 485   | `fixtures/{mod,plans,tests}.rs`                                           |
| `tests/e2e/daemon_config.rs`       | 644   | `daemon_config/{mod,defaults,intervals,manual_mode,parallel_sessions}.rs` |
| `tests/e2e/criteria_validation.rs` | 549   | `criteria_validation/{mod,stage_id,acceptance,dependencies,structure}.rs` |
| `tests/e2e/handoff.rs`             | 442   | `handoff/{mod,context_detection,generation,session_integration}.rs`       |
| `tests/e2e/sessions.rs`            | 407   | `sessions/{mod,creation,status,context,lifecycle}.rs`                     |

---

## Refactoring Pattern

For each file, follow this workflow:

1. **Create directory**: `mkdir -p src/module/file/`
2. **Create types/constants first** (no dependencies)
3. **Move pure functions** (helpers without state)
4. **Move IO operations** (file/process operations)
5. **Create mod.rs** with re-exports to preserve public API
6. **Move tests** to `tests.rs`
7. **Update imports** throughout codebase
8. **Run tests**: `cargo test`
9. **Run clippy**: `cargo clippy`

---

## Verification

After each stage:

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

After all stages:

```bash
# Count lines in modified files
fd -e rs -p 'loom/src' | xargs wc -l | sort -n | tail -50

# Verify no file exceeds 400 lines
fd -e rs -p 'loom/src' -x wc -l | awk '$1 > 400 {print}'
```

---

## Files Summary

| Severity           | Count  | Total Lines | Priority           |
| ------------------ | ------ | ----------- | ------------------ |
| Critical (800+)    | 10     | ~9,400      | High - Stages 1-6  |
| Severe (600-800)   | 6      | ~3,800      | Medium - Stage 7-8 |
| Moderate (400-600) | 9      | ~4,700      | Medium - Stage 9   |
| Tests (400+)       | 6      | ~3,000      | Low - Stages 10-11 |
| **Total**          | **31** | **~20,900** |                    |

---

## Notes

- **Duplicate code**: `serialize_stage_to_markdown` exists in both `verify/transitions.rs` and `commands/init.rs` - consolidate in Stage 4
- **Shared utilities**: Extract YAML frontmatter parsing to `parser/frontmatter.rs` if used in 3+ places
- **Test isolation**: Test helpers must be split first (Stage 10) before other test files
