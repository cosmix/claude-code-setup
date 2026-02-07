//! Merge conflict resolver spawning for CLI path
//!
//! When the daemon is not running, this module handles spawning
//! a merge resolution session directly from the CLI.

use anyhow::{Context, Result};
use std::path::Path;

use crate::daemon::DaemonServer;
use crate::git::branch::branch_name_for_stage;
use crate::models::session::Session;
use crate::models::stage::Stage;
use crate::orchestrator::continuation::save_session;
use crate::orchestrator::signals::generate_merge_signal;
use crate::orchestrator::terminal::native::NativeBackend;
use crate::orchestrator::terminal::TerminalBackend;

/// Spawn a merge conflict resolver session from the CLI.
///
/// When the daemon is not running, this spawns a native terminal session
/// to resolve merge conflicts. If the daemon IS running, it returns early
/// since the daemon handles merge resolution automatically.
///
/// # Arguments
/// * `stage` - The stage with merge conflicts
/// * `conflicting_files` - List of files with conflicts
/// * `merge_point` - The target branch to merge into
/// * `repo_root` - Path to the main repository root
/// * `work_dir` - Path to the .work directory
///
/// # Returns
/// * `Ok(session_id)` - The ID of the spawned resolver session
pub fn spawn_merge_resolver(
    stage: &Stage,
    conflicting_files: &[String],
    merge_point: &str,
    repo_root: &Path,
    work_dir: &Path,
) -> Result<String> {
    // If daemon is running, it handles merge resolution automatically
    if DaemonServer::is_running(work_dir) {
        return Ok("daemon-managed".to_string());
    }

    // Create terminal backend for spawning
    let backend =
        NativeBackend::new(work_dir.to_path_buf()).context("Failed to create terminal backend")?;

    // Get the source branch name for this stage
    let source_branch = branch_name_for_stage(&stage.id);

    // Create a merge resolution session
    let session = Session::new_merge(source_branch.clone(), merge_point.to_string());
    let session_id = session.id.clone();

    // Generate the merge signal file
    let signal_path = generate_merge_signal(
        &session,
        stage,
        &source_branch,
        merge_point,
        conflicting_files,
        work_dir,
    )
    .context("Failed to generate merge signal")?;

    // Spawn the merge session in a terminal
    let spawned_session = backend
        .spawn_merge_session(stage, session, &signal_path, repo_root)
        .context("Failed to spawn merge resolver session")?;

    // Save the session file
    save_session(&spawned_session, work_dir).context("Failed to save merge resolver session")?;

    Ok(session_id)
}
