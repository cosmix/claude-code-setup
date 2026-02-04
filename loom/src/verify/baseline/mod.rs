//! Baseline capture and change impact analysis
//!
//! This module provides functionality to:
//! 1. Capture test state before stage changes (baseline)
//! 2. Compare current state against baseline after changes
//! 3. Detect new failures, fixed failures, and warning changes
//!
//! # Usage
//!
//! Baselines are captured automatically when a stage starts execution
//! (if change_impact is configured in the plan). At stage completion,
//! the current state is compared against the baseline to detect regressions.
//!
//! # Configuration
//!
//! Change impact is configured at the plan level:
//!
//! ```yaml
//! loom:
//!   version: 1
//!   change_impact:
//!     baseline_command: "cargo test --no-fail-fast 2>&1 || true"
//!     failure_patterns:
//!       - "FAILED"
//!       - "error\\[E"
//!     policy: fail  # or: warn, skip
//! ```

pub mod capture;
pub mod compare;
pub mod types;

pub use capture::{baseline_exists, capture_baseline, load_baseline, save_baseline};
pub use compare::{compare_to_baseline, ensure_baseline_captured};
pub use types::{ChangeImpact, TestBaseline};
