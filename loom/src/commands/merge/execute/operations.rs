//! Merge operations and stage status management
//!
//! Contains functions for updating stage status after merge and
//! utility functions for worktree path resolution.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::fs::stage_files::find_stage_file;
use crate::models::stage::StageStatus;
use crate::verify::transitions::{load_stage, save_stage};

/// Update stage status to Completed and mark as merged after successful merge
pub fn mark_stage_merged(stage_id: &str, work_dir: &Path) -> Result<()> {
    let stages_dir = work_dir.join("stages");

    // Only update if stage file exists
    if find_stage_file(&stages_dir, stage_id)?.is_none() {
        // Stage file doesn't exist (might be a worktree without loom tracking)
        return Ok(());
    }

    // Load stage and update fields
    let mut stage = load_stage(stage_id, work_dir)?;

    // Transition to Completed status (if not already)
    if stage.status != StageStatus::Completed {
        crate::verify::transitions::transition_stage(stage_id, StageStatus::Completed, work_dir)
            .with_context(|| format!("Failed to update stage status for: {stage_id}"))?;
        // Reload after transition to get updated state
        stage = load_stage(stage_id, work_dir)?;
        println!("Updated stage status to Completed");
    }

    // Mark as merged (manual merge succeeded)
    if !stage.merged {
        stage.merged = true;
        save_stage(&stage, work_dir)?;
        println!("Marked stage as merged");
    }

    Ok(())
}

/// Get the worktree path for a stage
pub fn worktree_path(stage_id: &str) -> PathBuf {
    std::env::current_dir()
        .unwrap_or_default()
        .join(".worktrees")
        .join(stage_id)
}
