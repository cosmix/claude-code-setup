# Mistakes & Lessons Learned

> Record mistakes made during development and how to avoid them.
> This file is append-only - agents add discoveries, never delete.
>
> Format: Describe what went wrong, why, and how to avoid it next time.

(Add mistakes and lessons as you encounter them)

## AppleScript Injection in notify.rs

**What happened:** macOS notification function used incomplete escaping (only quotes, not backslashes). Could allow injection via crafted plan descriptions.
**Why:** Copy-pasted inline escaping instead of using existing `escape_applescript_string()` from `emulator.rs`.
**How to avoid:** Always reuse existing security functions. Search for existing escape/sanitize functions before writing inline versions.

## Truncate utilities placed in wrong layer

**What happened:** `truncate`/`truncate_for_display` were defined in `commands/common/mod.rs` but used by `orchestrator/`, `fs/`, `verify/` — all lower layers. This created upward imports violating layering.
**Why:** Functions grew organically in `commands/common` without considering which layers need them.
**How to avoid:** Generic utility functions that cross layer boundaries belong in `utils.rs`, not in any specific layer module.
