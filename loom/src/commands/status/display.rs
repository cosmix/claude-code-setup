use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::fs::work_dir::WorkDir;
use crate::models::constants::DEFAULT_CONTEXT_LIMIT;
use crate::models::keys::frontmatter;
use crate::models::runner::{Runner, RunnerStatus};
use crate::models::session::{Session, SessionStatus};
use crate::models::failure::FailureType;
use crate::models::stage::{Stage, StageStatus};
use crate::models::worktree::WorktreeStatus;
use crate::orchestrator::terminal::BackendType;
use crate::parser::frontmatter::extract_yaml_frontmatter;
use crate::parser::markdown::MarkdownDocument;

pub fn load_runners(work_dir: &WorkDir) -> Result<(Vec<Runner>, usize)> {
    let runners_dir = work_dir.runners_dir();
    let mut runners = Vec::new();
    let mut count = 0;

    if !runners_dir.exists() {
        return Ok((runners, 0));
    }

    for entry in fs::read_dir(&runners_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            count += 1;
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(doc) = MarkdownDocument::parse(&content) {
                    if let Some(runner) = parse_runner_from_doc(&doc) {
                        runners.push(runner);
                    }
                }
            }
        }
    }

    Ok((runners, count))
}

fn parse_runner_from_doc(doc: &MarkdownDocument) -> Option<Runner> {
    let id = doc.get_frontmatter(frontmatter::ID)?.clone();
    let name = doc.get_frontmatter(frontmatter::NAME)?.clone();
    let runner_type = doc.get_frontmatter(frontmatter::RUNNER_TYPE)?.clone();

    let context_tokens = doc
        .get_frontmatter(frontmatter::CONTEXT_TOKENS)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let context_limit = doc
        .get_frontmatter(frontmatter::CONTEXT_LIMIT)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CONTEXT_LIMIT);

    Some(Runner {
        id,
        name,
        runner_type,
        status: RunnerStatus::Idle,
        assigned_track: doc.get_frontmatter(frontmatter::ASSIGNED_TRACK).cloned(),
        context_tokens,
        context_limit,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    })
}

pub fn display_runner_health(runner: &Runner) {
    let health = runner.context_health();
    let health_str = format!("{health:.1}%");
    let context_tokens = runner.context_tokens;
    let context_limit = runner.context_limit;
    let status_str = format!("{context_tokens}/{context_limit} tokens");

    let colored_health = if health < 60.0 {
        health_str.green()
    } else if health < 75.0 {
        health_str.yellow()
    } else {
        health_str.red()
    };

    println!(
        "  {} [{}] {}",
        runner.name,
        colored_health,
        status_str.dimmed()
    );
}

pub fn count_files(dir: &std::path::Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let count = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "md"))
        .count();

    Ok(count)
}

pub fn display_stages(work_dir: &WorkDir) -> Result<()> {
    let stages_dir = work_dir.stages_dir();
    if !stages_dir.exists() {
        return Ok(());
    }

    let mut stages = Vec::new();
    for entry in fs::read_dir(&stages_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            if let Ok(content) = fs::read_to_string(&path) {
                // Use proper YAML deserialization for full stage parsing
                if let Ok(stage) = parse_stage_from_markdown(&content) {
                    stages.push(stage);
                }
            }
        }
    }

    if stages.is_empty() {
        return Ok(());
    }

    println!("\n{}", "Active Stages".bold());

    // Group stages by status in logical order
    let status_order = [
        (StageStatus::Completed, "✓", "Completed"),
        (StageStatus::Executing, "▶", "Executing"),
        (StageStatus::Queued, "○", "Ready"),
        (StageStatus::WaitingForInput, "?", "Waiting for Input"),
        (StageStatus::NeedsHandoff, "↻", "Needs Handoff"),
        (StageStatus::Blocked, "✗", "Blocked"),
        (StageStatus::WaitingForDeps, "·", "Pending"),
        (StageStatus::Skipped, "⊘", "Skipped"),
    ];

    // Find max ID length for alignment
    let max_id_len = stages.iter().map(|s| s.id.len()).max().unwrap_or(0);

    for (status, icon, label) in status_order {
        let matching: Vec<_> = stages.iter().filter(|s| s.status == status).collect();
        if matching.is_empty() {
            continue;
        }

        let header = format!("{icon} {label} ({})", matching.len());
        let colored_header = match status {
            StageStatus::Completed => header.green(),
            StageStatus::Executing => header.blue(),
            StageStatus::Queued => header.cyan(),
            StageStatus::WaitingForInput => header.magenta(),
            StageStatus::NeedsHandoff => header.yellow(),
            StageStatus::Blocked => header.red(),
            StageStatus::WaitingForDeps => header.dimmed(),
            StageStatus::Skipped => header.dimmed().strikethrough(),
        };
        println!("  {colored_header}");

        for stage in matching {
            let padded_id = format!("{:width$}", stage.id, width = max_id_len);
            let held_indicator = if stage.held {
                " [HELD]".yellow()
            } else {
                "".normal()
            };

            // Build status-specific suffix (retry info for blocked, etc.)
            let status_suffix = if stage.status == StageStatus::Blocked {
                let max = stage.max_retries.unwrap_or(3);
                let failure_label = stage
                    .failure_info
                    .as_ref()
                    .map(|i| match i.failure_type {
                        FailureType::SessionCrash => "crash",
                        FailureType::TestFailure => "test",
                        FailureType::BuildFailure => "build",
                        FailureType::CodeError => "code",
                        FailureType::Timeout => "timeout",
                        FailureType::ContextExhausted => "context",
                        FailureType::UserBlocked => "user",
                        FailureType::MergeConflict => "merge",
                        FailureType::Unknown => "error",
                    })
                    .unwrap_or("error");

                format!(" [{}] ({}/{} retries)", failure_label, stage.retry_count, max)
                    .red()
            } else {
                "".normal()
            };

            println!(
                "    {}  {}{}{}",
                padded_id.dimmed(),
                stage.name,
                held_indicator,
                status_suffix
            );
        }
        println!();
    }

    Ok(())
}

/// Parse a Stage from markdown with YAML frontmatter
///
/// Uses full YAML deserialization to properly handle all stage fields
/// including nested structures like failure_info.
fn parse_stage_from_markdown(content: &str) -> Result<Stage> {
    let frontmatter = extract_yaml_frontmatter(content)
        .context("Failed to extract YAML frontmatter from stage file")?;

    let stage: Stage = serde_yaml::from_value(frontmatter)
        .context("Failed to deserialize Stage from YAML frontmatter")?;

    Ok(stage)
}

/// Check if a session is orphaned (status says running/spawning but backend says otherwise)
fn is_session_orphaned(session: &Session) -> bool {
    // Only check for orphaned sessions if status indicates active
    if !matches!(
        session.status,
        SessionStatus::Spawning | SessionStatus::Running
    ) {
        return false;
    }

    // Detect backend type from session properties
    let backend_type = if session.tmux_session.is_some() {
        Some(BackendType::Tmux)
    } else if session.pid.is_some() {
        Some(BackendType::Native)
    } else {
        None
    };

    match backend_type {
        Some(BackendType::Tmux) => {
            // For tmux backend, check if tmux session exists
            if let Some(tmux_name) = &session.tmux_session {
                !tmux_session_exists(tmux_name)
            } else {
                false
            }
        }
        Some(BackendType::Native) => {
            // For native backend, check if PID is alive
            if let Some(pid) = session.pid {
                !is_pid_alive(pid)
            } else {
                false
            }
        }
        None => {
            // No backend information - not orphaned
            false
        }
    }
}

fn tmux_session_exists(session_name: &str) -> bool {
    std::process::Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn is_pid_alive(pid: u32) -> bool {
    // On Unix systems, check if /proc/<pid> exists
    #[cfg(unix)]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }

    // Fallback for non-Unix systems (Windows)
    #[cfg(not(unix))]
    {
        // On Windows, we could use sysinfo or similar
        // For now, assume alive (conservative approach)
        true
    }
}

pub fn display_sessions(work_dir: &WorkDir) -> Result<()> {
    let sessions_dir = work_dir.sessions_dir();
    if !sessions_dir.exists() {
        return Ok(());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(doc) = MarkdownDocument::parse(&content) {
                    if let Some(session) = parse_session_from_doc(&doc) {
                        sessions.push(session);
                    }
                }
            }
        }
    }

    if !sessions.is_empty() {
        println!("\n{}", "Active Sessions".bold());
        for session in sessions {
            let is_orphaned = is_session_orphaned(&session);

            let status_color = if is_orphaned {
                "orphaned".red()
            } else {
                match session.status {
                    SessionStatus::Spawning => "spawning".yellow(),
                    SessionStatus::Running => "running".green(),
                    SessionStatus::Paused => "paused".yellow(),
                    SessionStatus::Completed => "completed".dimmed(),
                    SessionStatus::Crashed => "crashed".red(),
                    SessionStatus::ContextExhausted => "context-exhausted".red(),
                }
            };

            let stage_info = session
                .stage_id
                .as_ref()
                .map(|s| format!(" (stage: {s})"))
                .unwrap_or_default();

            println!("  {}{} [{}]", session.id, stage_info, status_color);
        }
    }

    Ok(())
}

fn parse_session_from_doc(doc: &MarkdownDocument) -> Option<Session> {
    let id = doc.get_frontmatter(frontmatter::ID)?.clone();
    let status_str = doc.get_frontmatter(frontmatter::STATUS)?;
    let status = match status_str.as_str() {
        "spawning" => SessionStatus::Spawning,
        "running" => SessionStatus::Running,
        "paused" => SessionStatus::Paused,
        "completed" => SessionStatus::Completed,
        "crashed" => SessionStatus::Crashed,
        "context-exhausted" => SessionStatus::ContextExhausted,
        _ => return None,
    };

    let context_tokens = doc
        .get_frontmatter("context_tokens")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let context_limit = doc
        .get_frontmatter("context_limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CONTEXT_LIMIT);

    Some(Session {
        id,
        stage_id: doc.get_frontmatter("stage_id").cloned(),
        tmux_session: doc
            .get_frontmatter("tmux_session")
            .cloned()
            .filter(|s| !s.is_empty() && s != "null"),
        worktree_path: doc.get_frontmatter("worktree_path").map(|s| s.into()),
        pid: doc.get_frontmatter("pid").and_then(|s| s.parse().ok()),
        status,
        context_tokens,
        context_limit,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        session_type: crate::models::session::SessionType::default(),
        merge_source_branch: None,
        merge_target_branch: None,
    })
}

/// Display worktree status for all active worktrees
pub fn display_worktrees(work_dir: &WorkDir) -> Result<()> {
    let work_root = work_dir.root().parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Work directory has no parent: {}",
            work_dir.root().display()
        )
    })?;

    let worktrees_dir = work_root.join(".worktrees");
    if !worktrees_dir.exists() {
        return Ok(());
    }

    let mut worktrees = Vec::new();
    for entry in fs::read_dir(&worktrees_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let stage_id = entry.file_name().to_str().unwrap_or("unknown").to_string();
            let status = detect_worktree_status(&path);
            worktrees.push((stage_id, status));
        }
    }

    if worktrees.is_empty() {
        return Ok(());
    }

    println!("\n{}", "Worktrees".bold());

    for (stage_id, status) in worktrees {
        let status_display = format_worktree_status(&status);
        println!("  {stage_id}  {status_display}");
    }

    Ok(())
}

/// Detect the status of a worktree directory
fn detect_worktree_status(worktree_path: &Path) -> WorktreeStatus {
    // Check for merge conflicts using git diff --name-only --diff-filter=U
    if has_merge_conflicts(worktree_path) {
        return WorktreeStatus::Conflict;
    }

    // Check if a merge is in progress by looking for MERGE_HEAD
    let git_path = worktree_path.join(".git");
    let is_merging = if git_path.is_file() {
        // Read gitdir path and check for MERGE_HEAD there
        if let Ok(content) = fs::read_to_string(&git_path) {
            if let Some(gitdir) = content.strip_prefix("gitdir: ") {
                let gitdir_path = std::path::PathBuf::from(gitdir.trim());
                gitdir_path.join("MERGE_HEAD").exists()
            } else {
                false
            }
        } else {
            false
        }
    } else {
        worktree_path.join(".git").join("MERGE_HEAD").exists()
    };

    if is_merging {
        return WorktreeStatus::Merging;
    }

    WorktreeStatus::Active
}

/// Check if there are unmerged paths (merge conflicts) in the worktree
fn has_merge_conflicts(worktree_path: &Path) -> bool {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
        .current_dir(worktree_path)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            !stdout.trim().is_empty()
        }
        Err(_) => false,
    }
}

/// Format worktree status for display
fn format_worktree_status(status: &WorktreeStatus) -> colored::ColoredString {
    match status {
        WorktreeStatus::Conflict => "[CONFLICT]".red().bold(),
        WorktreeStatus::Merging => "[MERGING]".yellow().bold(),
        WorktreeStatus::Merged => "[MERGED]".green(),
        WorktreeStatus::Creating => "[CREATING]".cyan(),
        WorktreeStatus::Removed => "[REMOVED]".dimmed(),
        WorktreeStatus::Active => "[ACTIVE]".green(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_session_orphaned_with_tmux_backend() {
        let mut session = Session::new();
        session.status = SessionStatus::Running;
        session.set_tmux_session("test-session".to_string());

        // This will call tmux has-session, which will likely fail in test environment
        // but the logic path is exercised
        let _result = is_session_orphaned(&session);
        // We don't assert the result since it depends on tmux availability
    }

    #[test]
    fn test_is_session_orphaned_with_native_backend() {
        let mut session = Session::new();
        session.status = SessionStatus::Running;
        session.set_pid(std::process::id());

        // Should detect current process as alive
        assert!(!is_session_orphaned(&session));

        // Test with non-existent PID
        session.set_pid(999999);
        assert!(is_session_orphaned(&session));
    }

    #[test]
    fn test_is_session_orphaned_terminal_states() {
        let mut session = Session::new();
        session.set_pid(999999);

        // Terminal states should not be considered orphaned
        session.status = SessionStatus::Completed;
        assert!(!is_session_orphaned(&session));

        session.status = SessionStatus::Crashed;
        assert!(!is_session_orphaned(&session));

        session.status = SessionStatus::ContextExhausted;
        assert!(!is_session_orphaned(&session));
    }

    #[test]
    fn test_is_session_orphaned_no_backend_info() {
        let mut session = Session::new();
        session.status = SessionStatus::Running;
        // No tmux_session or pid set

        // Should not be considered orphaned if backend info is missing
        assert!(!is_session_orphaned(&session));
    }

    #[test]
    fn test_is_pid_alive_current_process() {
        let current_pid = std::process::id();
        assert!(is_pid_alive(current_pid));
    }

    #[test]
    fn test_is_pid_alive_non_existent() {
        // PID 999999 is very unlikely to exist
        assert!(!is_pid_alive(999999));
    }

    #[test]
    fn test_parse_stage_with_retry_info() {
        use crate::models::failure::FailureType;

        // Test with full YAML frontmatter including nested failure_info
        let content = r#"---
id: stage-test-1
name: Test Stage
status: blocked
dependencies: []
acceptance: []
setup: []
files: []
child_stages: []
retry_count: 2
max_retries: 3
created_at: 2025-01-10T12:00:00Z
updated_at: 2025-01-10T12:00:00Z
failure_info:
  failure_type: session-crash
  detected_at: 2025-01-10T12:00:00Z
  evidence:
    - "Session crashed unexpectedly"
---

# Stage: Test Stage
"#;

        let stage = parse_stage_from_markdown(content).expect("Should parse stage from markdown");

        assert_eq!(stage.id, "stage-test-1");
        assert_eq!(stage.name, "Test Stage");
        assert_eq!(stage.status, StageStatus::Blocked);
        assert_eq!(stage.retry_count, 2);
        assert_eq!(stage.max_retries, Some(3));
        assert!(stage.failure_info.is_some());

        if let Some(failure_info) = stage.failure_info {
            assert_eq!(failure_info.failure_type, FailureType::SessionCrash);
            assert_eq!(failure_info.evidence.len(), 1);
        }
    }

    #[test]
    fn test_parse_stage_skipped() {
        let content = r#"---
id: stage-test-2
name: Skipped Stage
status: skipped
dependencies: []
acceptance: []
setup: []
files: []
child_stages: []
created_at: 2025-01-10T12:00:00Z
updated_at: 2025-01-10T12:00:00Z
---

# Stage: Skipped Stage
"#;

        let stage = parse_stage_from_markdown(content).expect("Should parse stage from markdown");

        assert_eq!(stage.id, "stage-test-2");
        assert_eq!(stage.name, "Skipped Stage");
        assert_eq!(stage.status, StageStatus::Skipped);
    }
}
