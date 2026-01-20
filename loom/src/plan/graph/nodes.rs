//! Graph node types for the execution graph

use crate::models::stage::StageOutput;
use serde::{Deserialize, Serialize};

/// A node in the execution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageNode {
    pub id: String,
    pub name: String,
    pub dependencies: Vec<String>,
    pub parallel_group: Option<String>,
    pub status: NodeStatus,
    /// Stage description - provides task context for the agent
    #[serde(default)]
    pub description: Option<String>,
    /// Acceptance criteria - commands to verify stage completion
    #[serde(default)]
    pub acceptance: Vec<String>,
    /// Setup commands to run before stage execution
    #[serde(default)]
    pub setup: Vec<String>,
    /// Files to modify in this stage
    #[serde(default)]
    pub files: Vec<String>,
    /// Whether to auto-merge after completion
    #[serde(default)]
    pub auto_merge: Option<bool>,
    /// Structured outputs from this stage for dependent stages
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<StageOutput>,
    /// Whether this stage's work has been merged to the merge point (main).
    ///
    /// A stage with `status: Completed` but `merged: false` has finished its work
    /// but the changes are still on the stage branch. Dependent stages cannot be
    /// scheduled until `merged: true` because they need the merged changes as their base.
    #[serde(default)]
    pub merged: bool,
}

/// Status of a node in the execution graph.
///
/// Mirrors StageStatus to accurately reflect all possible stage states:
/// - `WaitingForDeps` - Dependencies not yet satisfied
/// - `Queued` - Dependencies satisfied AND merged, ready to execute
/// - `Executing` - Currently running
/// - `WaitingForInput` - Paused waiting for user input
/// - `Completed` - Work done (but may not yet be merged to main)
/// - `Blocked` - Hit an error
/// - `NeedsHandoff` - Context limit reached, needs new session
/// - `Skipped` - Intentionally skipped (does NOT satisfy dependencies)
/// - `MergeConflict` - Completed but merge has conflicts
/// - `CompletedWithFailures` - Completed but acceptance criteria failed
/// - `MergeBlocked` - Merge failed with error (not conflicts)
///
/// # Scheduling Invariant
///
/// A stage is ready to schedule (transitions to `Queued`) only when ALL dependencies
/// have BOTH `status == Completed` AND `merged == true`. This ensures the dependent
/// stage can use the merge point (main) as its base, containing all dependency work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    /// Waiting for upstream dependencies to complete AND merge.
    /// Transitions to `Queued` when all deps are `Completed` with `merged: true`.
    #[serde(rename = "waiting-for-deps", alias = "pending")]
    WaitingForDeps,

    /// Dependencies satisfied and merged; queued for execution.
    /// Orchestrator will pick from Queued stages to spawn sessions.
    #[serde(rename = "queued", alias = "ready")]
    Queued,

    /// Currently executing in a session.
    #[serde(rename = "executing")]
    Executing,

    /// Stage needs user input/decision before continuing.
    /// Execution is paused until input is provided.
    #[serde(rename = "waiting-for-input")]
    WaitingForInput,

    /// Successfully completed. Work is done but may not be merged yet.
    /// Does NOT satisfy dependent stages until `merged: true` is set.
    #[serde(rename = "completed")]
    Completed,

    /// Blocked due to error.
    #[serde(rename = "blocked")]
    Blocked,

    /// Session hit context limit; needs new session to continue.
    /// Will transition back to Queued when ready to resume.
    #[serde(rename = "needs-handoff", alias = "needshandoff")]
    NeedsHandoff,

    /// Intentionally skipped (does NOT satisfy dependencies).
    #[serde(rename = "skipped")]
    Skipped,

    /// Stage completed work but has merge conflicts to resolve.
    /// Spawns a conflict resolution session to handle the merge.
    #[serde(rename = "merge-conflict")]
    MergeConflict,

    /// Stage finished executing but acceptance criteria failed.
    /// Can be retried by transitioning back to Executing.
    #[serde(rename = "completed-with-failures")]
    CompletedWithFailures,

    /// Stage merge failed with an actual error (not conflicts).
    /// Can be retried by transitioning back to Executing.
    #[serde(rename = "merge-blocked")]
    MergeBlocked,
}
