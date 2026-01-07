//! Stage state manipulation
//! Usage: loom stage <id> [complete|block|reset|ready]

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::session::{Session, SessionStatus};
use crate::models::stage::StageStatus;
use crate::verify::transitions::{load_stage, save_stage, trigger_dependents};

/// Mark a stage as complete, optionally running acceptance criteria.
/// If acceptance criteria pass, auto-verifies the stage and triggers dependents.
/// If --no-verify is used or criteria fail, marks as Completed for manual review.
pub fn complete(stage_id: String, session_id: Option<String>, no_verify: bool) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;

    // Resolve session_id: CLI arg > stage.session field > scan sessions directory
    let session_id = session_id
        .or_else(|| stage.session.clone())
        .or_else(|| find_session_for_stage(&stage_id, work_dir));

    // Resolve worktree path from stage's worktree field
    let working_dir: Option<PathBuf> = stage
        .worktree
        .as_ref()
        .map(|w| PathBuf::from(".worktrees").join(w))
        .filter(|p| p.exists());

    // Track whether all acceptance criteria passed
    let mut all_passed = true;

    // Run acceptance criteria unless --no-verify is specified
    if !no_verify && !stage.acceptance.is_empty() {
        println!("Running acceptance criteria for stage '{stage_id}'...");
        if let Some(ref dir) = working_dir {
            println!("  (working directory: {})", dir.display());
        }

        for criterion in &stage.acceptance {
            println!("  → {criterion}");
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(criterion);

            if let Some(ref dir) = working_dir {
                cmd.current_dir(dir);
            }

            let status = cmd
                .status()
                .with_context(|| format!("Failed to run: {criterion}"))?;

            if !status.success() {
                all_passed = false;
                println!("  ✗ FAILED: {criterion}");
                break;
            }
            println!("  ✓ passed");
        }

        if all_passed {
            println!("All acceptance criteria passed!");
        }
    } else if no_verify {
        // --no-verify means we skip criteria, so don't auto-verify
        all_passed = false;
    } else {
        // No acceptance criteria defined - treat as passed
        all_passed = true;
    }

    // Always try to kill the tmux session for this stage (even without session_id)
    cleanup_tmux_for_stage(&stage_id);

    // Cleanup session resources (update session status, remove signal)
    if let Some(ref sid) = session_id {
        cleanup_session_resources(&stage_id, sid, work_dir);
    }

    // Auto-verify if all criteria passed, otherwise mark as Completed
    if all_passed {
        stage.mark_verified();
        save_stage(&stage, work_dir)?;
        println!("Stage '{stage_id}' verified!");

        // Trigger dependent stages
        let triggered = trigger_dependents(&stage_id, work_dir)
            .context("Failed to trigger dependent stages")?;

        if !triggered.is_empty() {
            println!("Triggered {} dependent stage(s):", triggered.len());
            for dep_id in &triggered {
                println!("  → {dep_id}");
            }
        }
    } else {
        stage.complete(None);
        save_stage(&stage, work_dir)?;
        println!("Stage '{stage_id}' marked as completed (needs manual verification)");
    }

    Ok(())
}

/// Kill tmux session for a stage (best-effort, doesn't require session_id)
fn cleanup_tmux_for_stage(stage_id: &str) {
    let tmux_name = format!("loom-{stage_id}");
    match Command::new("tmux")
        .args(["kill-session", "-t", &tmux_name])
        .output()
    {
        Ok(output) if output.status.success() => {
            println!("Killed tmux session '{tmux_name}'");
        }
        Ok(_) => {
            // Session may not exist or already dead - this is fine
        }
        Err(e) => {
            eprintln!("Warning: failed to kill tmux session '{tmux_name}': {e}");
        }
    }
}

/// Find session ID for a stage by scanning .work/sessions/
fn find_session_for_stage(stage_id: &str, work_dir: &Path) -> Option<String> {
    let sessions_dir = work_dir.join("sessions");
    if !sessions_dir.exists() {
        return None;
    }

    let entries = fs::read_dir(&sessions_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // Try to read and parse session file
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(session) = session_from_markdown(&content) {
                if session.stage_id.as_deref() == Some(stage_id) {
                    return Some(session.id);
                }
            }
        }
    }
    None
}

/// Clean up resources associated with a completed stage
///
/// This function performs best-effort cleanup and logs warnings on failure:
/// 1. Updates session status to Completed
/// 2. Removes the signal file
fn cleanup_session_resources(_stage_id: &str, session_id: &str, work_dir: &Path) {
    // 1. Update session status to Completed
    if let Err(e) = update_session_status(work_dir, session_id, SessionStatus::Completed) {
        eprintln!("Warning: failed to update session status: {e}");
    }

    // 2. Remove signal file
    let signal_path = work_dir.join("signals").join(format!("{session_id}.md"));
    match fs::remove_file(&signal_path) {
        Ok(()) => {
            println!("Removed signal file '{}'", signal_path.display());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Signal file may not exist - this is fine
        }
        Err(e) => {
            eprintln!(
                "Warning: failed to remove signal file '{}': {e}",
                signal_path.display()
            );
        }
    }
}

/// Update a session's status in .work/sessions/
fn update_session_status(work_dir: &Path, session_id: &str, status: SessionStatus) -> Result<()> {
    let sessions_dir = work_dir.join("sessions");
    let session_path = sessions_dir.join(format!("{session_id}.md"));

    if !session_path.exists() {
        bail!("Session file not found: {}", session_path.display());
    }

    let content = fs::read_to_string(&session_path)
        .with_context(|| format!("Failed to read session file: {}", session_path.display()))?;

    // Parse session from markdown
    let session = session_from_markdown(&content)?;

    // Update status
    let mut session = session;
    session.status = status;
    session.last_active = chrono::Utc::now();

    // Write back
    let updated_content = session_to_markdown(&session);
    fs::write(&session_path, updated_content)
        .with_context(|| format!("Failed to write session file: {}", session_path.display()))?;

    Ok(())
}

/// Parse session from markdown with YAML frontmatter
fn session_from_markdown(content: &str) -> Result<Session> {
    let yaml_content = content
        .strip_prefix("---\n")
        .and_then(|s| s.split_once("\n---"))
        .map(|(yaml, _)| yaml)
        .ok_or_else(|| anyhow::anyhow!("Invalid session file format: missing frontmatter"))?;

    serde_yaml::from_str(yaml_content).context("Failed to parse session YAML")
}

/// Convert session to markdown format
fn session_to_markdown(session: &Session) -> String {
    let yaml = serde_yaml::to_string(session).unwrap_or_else(|_| String::from("{}"));

    format!(
        "---\n{yaml}---\n\n# Session: {}\n\n## Details\n\n- **Status**: {:?}\n- **Stage**: {}\n- **Tmux**: {}\n",
        session.id,
        session.status,
        session.stage_id.as_ref().unwrap_or(&"None".to_string()),
        session.tmux_session.as_ref().unwrap_or(&"None".to_string()),
    )
}

/// Block a stage with a reason
pub fn block(stage_id: String, reason: String) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;
    stage.status = StageStatus::Blocked;
    stage.close_reason = Some(reason.clone());
    stage.updated_at = chrono::Utc::now();
    save_stage(&stage, work_dir)?;

    println!("Stage '{stage_id}' blocked");
    println!("Reason: {reason}");
    Ok(())
}

/// Reset a stage to pending
pub fn reset(stage_id: String, hard: bool, kill_session: bool) -> Result<()> {
    let work_dir = Path::new(".work");

    // Kill tmux session if requested
    if kill_session {
        let tmux_name = format!("loom-{stage_id}");
        let _ = std::process::Command::new("tmux")
            .args(["kill-session", "-t", &tmux_name])
            .output();
    }

    let mut stage = load_stage(&stage_id, work_dir)?;
    stage.status = StageStatus::Pending;
    stage.completed_at = None;
    stage.close_reason = None;
    stage.updated_at = chrono::Utc::now();

    // Hard reset also clears session assignment
    if hard {
        stage.session = None;
    }

    save_stage(&stage, work_dir)?;

    let mode = if hard { "hard" } else { "soft" };
    println!("Stage '{stage_id}' reset to pending ({mode} reset)");
    Ok(())
}

/// Mark a stage as ready for execution
pub fn ready(stage_id: String) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;
    stage.mark_ready();
    save_stage(&stage, work_dir)?;

    println!("Stage '{stage_id}' marked as ready");
    Ok(())
}

/// Mark a stage as waiting for user input (called by hooks)
pub fn waiting(stage_id: String) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;

    // Only transition if currently executing
    if stage.status != StageStatus::Executing {
        // Silently skip if not executing - hook may fire at wrong time
        eprintln!(
            "Note: Stage '{}' is {:?}, not executing. Skipping waiting transition.",
            stage_id, stage.status
        );
        return Ok(());
    }

    stage.mark_waiting_for_input();
    save_stage(&stage, work_dir)?;

    println!("Stage '{stage_id}' waiting for user input");
    Ok(())
}

/// Resume a stage from waiting for input state (called by hooks)
pub fn resume_from_waiting(stage_id: String) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;

    // Only transition if currently waiting for input
    if stage.status != StageStatus::WaitingForInput {
        // Silently skip if not waiting - hook may fire at wrong time
        eprintln!(
            "Note: Stage '{}' is {:?}, not waiting. Skipping resume transition.",
            stage_id, stage.status
        );
        return Ok(());
    }

    stage.mark_executing();
    save_stage(&stage, work_dir)?;

    println!("Stage '{stage_id}' resumed execution");
    Ok(())
}

/// Hold a stage (prevent auto-execution even when ready)
pub fn hold(stage_id: String) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;

    if stage.held {
        println!("Stage '{stage_id}' is already held");
        return Ok(());
    }

    stage.hold();
    save_stage(&stage, work_dir)?;

    println!("Stage '{stage_id}' held");
    println!("The stage will not auto-execute. Use 'loom stage release {stage_id}' to unlock.");
    Ok(())
}

/// Release a held stage (allow auto-execution)
pub fn release(stage_id: String) -> Result<()> {
    let work_dir = Path::new(".work");

    let mut stage = load_stage(&stage_id, work_dir)?;

    if !stage.held {
        println!("Stage '{stage_id}' is not held");
        return Ok(());
    }

    stage.release();
    save_stage(&stage, work_dir)?;

    println!("Stage '{stage_id}' released");
    Ok(())
}
