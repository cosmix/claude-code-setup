//! Merge status detection utilities for stages.
//!
//! This module re-exports merge status utilities from the git module.
//! The actual implementation lives in `crate::git::merge::status`.

// Re-export everything from git::merge::status
pub use crate::git::merge::{build_merge_report, check_merge_state, MergeState, MergeStatusReport};
