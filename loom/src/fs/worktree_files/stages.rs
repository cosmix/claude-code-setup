//! Stage file operations for worktrees

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::sessions::find_sessions_for_stage;
use crate::fs::stage_files::find_stage_file;

/// Archive a stage file by moving it to the archive directory
pub(crate) fn archive_stage_file(stage_id: &str, work_dir: &Path) -> Result<()> {
    let stages_dir = work_dir.join("stages");
    let archive_dir = work_dir.join("archive");

    // Find the stage file
    let stage_file = find_stage_file(&stages_dir, stage_id)?;
    let Some(stage_file) = stage_file else {
        return Ok(()); // No file to archive
    };

    // Ensure archive directory exists
    fs::create_dir_all(&archive_dir).with_context(|| "Failed to create archive directory")?;

    // Move to archive
    let archive_path = archive_dir.join(stage_file.file_name().unwrap_or_default());
    fs::rename(&stage_file, &archive_path)
        .with_context(|| format!("Failed to archive stage file to {}", archive_path.display()))?;

    Ok(())
}

/// Check if any files exist for a stage that would need cleanup
pub fn stage_has_files(stage_id: &str, work_dir: &Path) -> bool {
    // Check for sessions
    if let Ok(sessions) = find_sessions_for_stage(stage_id, work_dir) {
        if !sessions.is_empty() {
            return true;
        }
    }

    // Check for stage file
    let stages_dir = work_dir.join("stages");
    if let Ok(Some(_)) = find_stage_file(&stages_dir, stage_id) {
        return true;
    }

    false
}
