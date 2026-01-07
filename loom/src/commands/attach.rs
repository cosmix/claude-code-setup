//! Attach to running sessions
//!
//! Usage:
//!   loom attach <stage_id|session_id>  - Attach to a specific session
//!   loom attach orch                   - Attach to the orchestrator
//!   loom attach all                    - Attach to all sessions (tiled view)
//!   loom attach all --gui              - Open each session in a GUI terminal window
//!   loom attach list                   - List attachable sessions

use anyhow::{bail, Context, Result};

use crate::commands::run;
use crate::orchestrator::{
    attach_by_session, attach_by_stage, attach_overview_session, create_overview_session,
    create_tiled_overview, format_attachable_list, list_attachable, print_many_sessions_warning,
    print_overview_instructions, print_tiled_instructions, spawn_gui_windows,
};

/// Attach terminal to running session
pub fn execute(target: String) -> Result<()> {
    // Handle orchestrator attach
    if target == "orch" || target == "orchestrator" {
        return run::attach_orchestrator();
    }

    let work_dir = std::env::current_dir()?.join(".work");
    if !work_dir.exists() {
        bail!("No .work/ directory found. Run 'loom init' first.");
    }

    if target.starts_with("stage-") {
        attach_by_stage(&target, &work_dir)
    } else if target.starts_with("session-") {
        attach_by_session(&target, &work_dir)
    } else {
        attach_by_session(&target, &work_dir)
            .or_else(|_| attach_by_stage(&target, &work_dir))
            .with_context(|| format!("Could not find session or stage with identifier '{target}'"))
    }
}

/// List all attachable sessions
pub fn list() -> Result<()> {
    let work_dir = std::env::current_dir()?.join(".work");
    if !work_dir.exists() {
        println!("(no .work/ directory - run 'loom init' first)");
        return Ok(());
    }

    let sessions = list_attachable(&work_dir)?;

    if sessions.is_empty() {
        println!("No attachable sessions found.");
        println!("\nSessions must be in 'running' or 'paused' state.");
        return Ok(());
    }

    print!("{}", format_attachable_list(&sessions));

    Ok(())
}

/// Attach to all running sessions
///
/// Default mode creates a tiled pane view where all sessions are visible
/// simultaneously. Use --windows for legacy window-per-session mode.
/// Use --gui to spawn separate terminal windows.
pub fn execute_all(
    gui_mode: bool,
    detach_existing: bool,
    windows_mode: bool,
    layout: String,
) -> Result<()> {
    let work_dir = std::env::current_dir()?.join(".work");
    if !work_dir.exists() {
        bail!("No .work/ directory found. Run 'loom init' first.");
    }

    let sessions = list_attachable(&work_dir)?;

    if sessions.is_empty() {
        println!("No attachable sessions found.");
        println!("\nSessions must be in 'running' or 'paused' state.");
        return Ok(());
    }

    if gui_mode {
        spawn_gui_windows(&sessions, detach_existing)
    } else if windows_mode {
        // Legacy: one window per session
        println!(
            "\nCreating overview session with {} loom session(s)...",
            sessions.len()
        );
        let overview_name = create_overview_session(&sessions, detach_existing)?;
        print_overview_instructions(sessions.len());
        attach_overview_session(&overview_name)
    } else {
        // Default: tiled panes
        print_many_sessions_warning(sessions.len());
        println!(
            "\nCreating tiled view with {} loom session(s)...",
            sessions.len()
        );
        let overview_name = create_tiled_overview(&sessions, &layout, detach_existing)?;
        print_tiled_instructions(sessions.len());
        attach_overview_session(&overview_name)
    }
}
