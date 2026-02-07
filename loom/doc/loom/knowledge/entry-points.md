# Entry Points

> Key files agents should read first to understand the codebase.
> This file is append-only - agents add discoveries, never delete.

(Add entry points as you discover them)

## Key Utility Locations (Updated)

- `src/utils.rs` — Shared utilities (truncation, formatting, terminal cleanup, color helpers)
- `src/commands/common/mod.rs` — Command-layer utilities (find_work_dir, detect_session, detect_stage_id) + re-exports of utils
- `src/plan/graph/levels.rs` — Generic DAG level computation (compute_all_levels)
- `src/git/merge/lock.rs` — MergeLock for atomic merge operations
- `src/verify/transitions/persistence.rs` — Canonical stage loading (load_stage)
