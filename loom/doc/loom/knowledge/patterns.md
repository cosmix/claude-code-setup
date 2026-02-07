# Architectural Patterns

> Discovered patterns in the codebase that help agents understand how things work.
> This file is append-only - agents add discoveries, never delete.

(Add patterns as you discover them)

## Shared Utilities Pattern

Generic utility functions (string manipulation, formatting, display helpers) should live in `utils.rs` at the crate root, NOT in layer-specific modules like `commands/common/`. This ensures all layers can import without violating the dependency hierarchy.

Current shared utilities in `utils.rs`:

- `truncate(s, max_chars)` — UTF-8 safe string truncation with "..." ellipsis
- `truncate_for_display(s, max_len)` — Collapses multiline + truncates with "…"
- `format_elapsed(seconds)` — Compact duration formatting
- `format_elapsed_verbose(seconds)` — Verbose duration formatting
- `cleanup_terminal()` / `install_terminal_panic_hook()` — Terminal state restoration
- `context_pct_terminal_color(pct)` / `context_pct_tui_color(pct)` — Color by context %

## Re-export Pattern for Backward Compatibility

When moving functions to a new canonical location, add `pub use` re-exports at the old location so existing in-layer callers don't need updating. Cross-layer callers should be updated to the canonical import path.
