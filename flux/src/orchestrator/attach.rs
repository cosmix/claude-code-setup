use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;

use crate::models::session::{Session, SessionStatus};
use crate::models::stage::Stage;
use crate::parser::markdown::MarkdownDocument;

/// Information about an attachable session
#[derive(Debug, Clone)]
pub struct AttachableSession {
    pub session_id: String,
    pub stage_id: Option<String>,
    pub stage_name: Option<String>,
    pub tmux_session: String,
    pub status: SessionStatus,
    pub context_percent: f64,
}

/// Attach to a tmux session by stage ID
/// - Looks up the session for the stage
/// - Prints helpful detach instructions first
/// - Executes: `tmux attach -t {session_name}`
/// - This will replace the current process (exec)
pub fn attach_by_stage(stage_id: &str, work_dir: &Path) -> Result<()> {
    let session = find_session_for_stage(work_dir, stage_id)?
        .ok_or_else(|| anyhow!("No active session found for stage '{stage_id}'"))?;

    let tmux_session = session
        .tmux_session
        .ok_or_else(|| anyhow!("Session '{}' has no tmux session assigned", session.id))?;

    print_attach_instructions(&tmux_session);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new("tmux")
            .arg("attach")
            .arg("-t")
            .arg(&tmux_session)
            .exec();
        Err(anyhow!("Failed to exec tmux: {error}"))
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new("tmux")
            .arg("attach")
            .arg("-t")
            .arg(&tmux_session)
            .status()
            .context("Failed to execute tmux command")?;

        if !status.success() {
            bail!("tmux attach failed with status: {}", status);
        }
        Ok(())
    }
}

/// Attach to a tmux session directly by session ID or tmux session name
pub fn attach_by_session(session_id: &str, work_dir: &Path) -> Result<()> {
    let session = load_session(work_dir, session_id)?;

    let tmux_session = session
        .tmux_session
        .ok_or_else(|| anyhow!("Session '{}' has no tmux session assigned", session.id))?;

    print_attach_instructions(&tmux_session);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new("tmux")
            .arg("attach")
            .arg("-t")
            .arg(&tmux_session)
            .exec();
        Err(anyhow!("Failed to exec tmux: {error}"))
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new("tmux")
            .arg("attach")
            .arg("-t")
            .arg(&tmux_session)
            .status()
            .context("Failed to execute tmux command")?;

        if !status.success() {
            bail!("tmux attach failed with status: {}", status);
        }
        Ok(())
    }
}

/// List all sessions that can be attached to
/// - Reads .work/sessions/ for session files
/// - Filters to Running or Paused sessions with tmux_session set
/// - Returns list with context health information
pub fn list_attachable(work_dir: &Path) -> Result<Vec<AttachableSession>> {
    let sessions_dir = work_dir.join("sessions");
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut attachable = Vec::new();

    let entries = std::fs::read_dir(&sessions_dir).with_context(|| {
        format!(
            "Failed to read sessions directory: {}",
            sessions_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        match load_session(work_dir, &session_id) {
            Ok(session) => {
                if !is_attachable(&session) {
                    continue;
                }

                let tmux_session = session.tmux_session.clone().unwrap();
                let context_percent = session.context_health() as f64;

                let (stage_id, stage_name) = if let Some(ref sid) = session.stage_id {
                    match load_stage(work_dir, sid) {
                        Ok(stage) => (Some(sid.clone()), Some(stage.name)),
                        Err(_) => (Some(sid.clone()), None),
                    }
                } else {
                    (None, None)
                };

                attachable.push(AttachableSession {
                    session_id: session.id,
                    stage_id,
                    stage_name,
                    tmux_session,
                    status: session.status,
                    context_percent,
                });
            }
            Err(_) => {
                continue;
            }
        }
    }

    attachable.sort_by(|a, b| a.session_id.cmp(&b.session_id));

    Ok(attachable)
}

/// Print the pre-attach instructions message
/// Shows helpful info about detaching and scrolling
pub fn print_attach_instructions(session_name: &str) {
    println!("\n┌─────────────────────────────────────────────────────────┐");
    println!("│  Attaching to session {session_name:<32}│");
    println!("│                                                         │");
    println!("│  To detach (return to flux): Press Ctrl+B then D        │");
    println!("│  To scroll: Ctrl+B then [ (exit scroll: q)              │");
    println!("└─────────────────────────────────────────────────────────┘\n");
}

/// Generate the formatted table for `flux attach list`
pub fn format_attachable_list(sessions: &[AttachableSession]) -> String {
    let mut output = String::new();

    output.push_str("SESSION          STAGE              STATUS      CONTEXT\n");

    for session in sessions {
        let stage_display = session
            .stage_name
            .as_ref()
            .map(|s| {
                if s.len() > 18 {
                    format!("{}...", &s[..15])
                } else {
                    s.clone()
                }
            })
            .unwrap_or_else(|| "-".to_string());

        let status_display = format_status(&session.status);

        let session_display = if session.session_id.len() > 16 {
            format!("{}...", &session.session_id[..13])
        } else {
            session.session_id.clone()
        };

        output.push_str(&format!(
            "{session_display:<16} {stage_display:<18} {status_display:<11} {:>3.0}%\n",
            session.context_percent
        ));
    }

    output
}

/// Load a session from .work/sessions/{id}.md
fn load_session(work_dir: &Path, session_id: &str) -> Result<Session> {
    let session_path = work_dir.join("sessions").join(format!("{session_id}.md"));

    if !session_path.exists() {
        bail!("Session file not found: {}", session_path.display());
    }

    let content = std::fs::read_to_string(&session_path)
        .with_context(|| format!("Failed to read session file: {}", session_path.display()))?;

    session_from_markdown(&content)
}

/// Load a stage from .work/stages/{id}.md
fn load_stage(work_dir: &Path, stage_id: &str) -> Result<Stage> {
    let stage_path = work_dir.join("stages").join(format!("{stage_id}.md"));

    if !stage_path.exists() {
        bail!("Stage file not found: {}", stage_path.display());
    }

    let content = std::fs::read_to_string(&stage_path)
        .with_context(|| format!("Failed to read stage file: {}", stage_path.display()))?;

    stage_from_markdown(&content)
}

/// Find session for a stage
fn find_session_for_stage(work_dir: &Path, stage_id: &str) -> Result<Option<Session>> {
    let sessions_dir = work_dir.join("sessions");
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let entries = std::fs::read_dir(&sessions_dir).with_context(|| {
        format!(
            "Failed to read sessions directory: {}",
            sessions_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        match load_session(work_dir, &session_id) {
            Ok(session) => {
                if session.stage_id.as_deref() == Some(stage_id) {
                    return Ok(Some(session));
                }
            }
            Err(_) => continue,
        }
    }

    Ok(None)
}

/// Check if a session can be attached to
fn is_attachable(session: &Session) -> bool {
    if session.tmux_session.is_none() {
        return false;
    }

    matches!(
        session.status,
        SessionStatus::Running | SessionStatus::Paused
    )
}

/// Format session status for display
fn format_status(status: &SessionStatus) -> String {
    match status {
        SessionStatus::Spawning => "spawning".to_string(),
        SessionStatus::Running => "running".to_string(),
        SessionStatus::Paused => "paused".to_string(),
        SessionStatus::Completed => "completed".to_string(),
        SessionStatus::Crashed => "crashed".to_string(),
        SessionStatus::ContextExhausted => "exhausted".to_string(),
    }
}

/// Parse a Session from markdown content
fn session_from_markdown(content: &str) -> Result<Session> {
    use chrono::{DateTime, Utc};

    let doc =
        MarkdownDocument::parse(content).context("Failed to parse session markdown document")?;

    let id = doc
        .get_frontmatter("id")
        .ok_or_else(|| anyhow!("Missing 'id' in session frontmatter"))?
        .to_string();

    let stage_id = doc
        .get_frontmatter("stage_id")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let tmux_session = doc
        .get_frontmatter("tmux_session")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let worktree_path = doc
        .get_frontmatter("worktree_path")
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());

    let pid = doc
        .get_frontmatter("pid")
        .and_then(|s| s.parse::<u32>().ok());

    let status_str = doc
        .get_frontmatter("status")
        .ok_or_else(|| anyhow!("Missing 'status' in session frontmatter"))?;

    let status = match status_str.as_str() {
        "spawning" => SessionStatus::Spawning,
        "running" => SessionStatus::Running,
        "paused" => SessionStatus::Paused,
        "completed" => SessionStatus::Completed,
        "crashed" => SessionStatus::Crashed,
        "context_exhausted" => SessionStatus::ContextExhausted,
        _ => bail!("Invalid session status: {status_str}"),
    };

    let context_tokens = doc
        .get_frontmatter("context_tokens")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let context_limit = doc
        .get_frontmatter("context_limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(200_000);

    let created_at = doc
        .get_frontmatter("created_at")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .ok_or_else(|| anyhow!("Missing or invalid 'created_at' in session frontmatter"))?;

    let last_active = doc
        .get_frontmatter("last_active")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .ok_or_else(|| anyhow!("Missing or invalid 'last_active' in session frontmatter"))?;

    Ok(Session {
        id,
        stage_id,
        tmux_session,
        worktree_path,
        pid,
        status,
        context_tokens,
        context_limit,
        created_at,
        last_active,
    })
}

/// Parse a Stage from markdown content
fn stage_from_markdown(content: &str) -> Result<Stage> {
    use crate::models::stage::StageStatus;
    use chrono::{DateTime, Utc};

    let doc =
        MarkdownDocument::parse(content).context("Failed to parse stage markdown document")?;

    let id = doc
        .get_frontmatter("id")
        .ok_or_else(|| anyhow!("Missing 'id' in stage frontmatter"))?
        .to_string();

    let name = doc
        .get_frontmatter("name")
        .ok_or_else(|| anyhow!("Missing 'name' in stage frontmatter"))?
        .to_string();

    let description = doc
        .get_frontmatter("description")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let status_str = doc
        .get_frontmatter("status")
        .ok_or_else(|| anyhow!("Missing 'status' in stage frontmatter"))?;

    let status = match status_str.as_str() {
        "pending" => StageStatus::Pending,
        "ready" => StageStatus::Ready,
        "executing" => StageStatus::Executing,
        "blocked" => StageStatus::Blocked,
        "completed" => StageStatus::Completed,
        "needs_handoff" => StageStatus::NeedsHandoff,
        "verified" => StageStatus::Verified,
        _ => bail!("Invalid stage status: {status_str}"),
    };

    let session = doc
        .get_frontmatter("session")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let created_at = doc
        .get_frontmatter("created_at")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(chrono::Utc::now);

    let updated_at = doc
        .get_frontmatter("updated_at")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(chrono::Utc::now);

    let completed_at = doc
        .get_frontmatter("completed_at")
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    let close_reason = doc
        .get_frontmatter("close_reason")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    Ok(Stage {
        id,
        name,
        description,
        status,
        dependencies: Vec::new(),
        parallel_group: None,
        acceptance: Vec::new(),
        files: Vec::new(),
        plan_id: None,
        worktree: None,
        session,
        parent_stage: None,
        child_stages: Vec::new(),
        created_at,
        updated_at,
        completed_at,
        close_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_attachable_list() {
        let sessions = vec![
            AttachableSession {
                session_id: "session-1".to_string(),
                stage_id: Some("stage-1".to_string()),
                stage_name: Some("models".to_string()),
                tmux_session: "flux-session-1".to_string(),
                status: SessionStatus::Running,
                context_percent: 45.0,
            },
            AttachableSession {
                session_id: "session-2".to_string(),
                stage_id: Some("stage-2".to_string()),
                stage_name: Some("api".to_string()),
                tmux_session: "flux-session-2".to_string(),
                status: SessionStatus::Paused,
                context_percent: 23.5,
            },
        ];

        let output = format_attachable_list(&sessions);

        assert!(output.contains("SESSION"));
        assert!(output.contains("STAGE"));
        assert!(output.contains("STATUS"));
        assert!(output.contains("CONTEXT"));
        assert!(output.contains("session-1"));
        assert!(output.contains("session-2"));
        assert!(output.contains("models"));
        assert!(output.contains("api"));
        assert!(output.contains("running"));
        assert!(output.contains("paused"));
        assert!(output.contains("45%"));
        assert!(output.contains("24%"));
    }

    #[test]
    fn test_format_attachable_list_long_names() {
        let sessions = vec![AttachableSession {
            session_id: "very-long-session-identifier-name".to_string(),
            stage_id: Some("stage-1".to_string()),
            stage_name: Some("very-long-stage-name-that-exceeds-limit".to_string()),
            tmux_session: "flux-session-1".to_string(),
            status: SessionStatus::Running,
            context_percent: 75.8,
        }];

        let output = format_attachable_list(&sessions);

        assert!(output.contains("very-long-ses..."));
        assert!(output.contains("very-long-stage..."));
        assert!(output.contains("76%"));
    }

    #[test]
    fn test_print_attach_instructions() {
        print_attach_instructions("test-session");
    }

    #[test]
    fn test_context_percent_calculation() {
        let session = AttachableSession {
            session_id: "test".to_string(),
            stage_id: None,
            stage_name: None,
            tmux_session: "flux-test".to_string(),
            status: SessionStatus::Running,
            context_percent: 75.5,
        };

        assert_eq!(session.context_percent, 75.5);
    }

    #[test]
    fn test_attachable_filter() {
        use crate::models::session::Session;

        let mut running_session = Session::new();
        running_session.status = SessionStatus::Running;
        running_session.tmux_session = Some("tmux-1".to_string());

        let mut paused_session = Session::new();
        paused_session.status = SessionStatus::Paused;
        paused_session.tmux_session = Some("tmux-2".to_string());

        let mut completed_session = Session::new();
        completed_session.status = SessionStatus::Completed;
        completed_session.tmux_session = Some("tmux-3".to_string());

        let mut spawning_session = Session::new();
        spawning_session.status = SessionStatus::Spawning;
        spawning_session.tmux_session = Some("tmux-4".to_string());

        let mut no_tmux_session = Session::new();
        no_tmux_session.status = SessionStatus::Running;
        no_tmux_session.tmux_session = None;

        assert!(is_attachable(&running_session));
        assert!(is_attachable(&paused_session));
        assert!(!is_attachable(&completed_session));
        assert!(!is_attachable(&spawning_session));
        assert!(!is_attachable(&no_tmux_session));
    }

    #[test]
    fn test_format_status() {
        assert_eq!(format_status(&SessionStatus::Running), "running");
        assert_eq!(format_status(&SessionStatus::Paused), "paused");
        assert_eq!(format_status(&SessionStatus::Completed), "completed");
        assert_eq!(format_status(&SessionStatus::Crashed), "crashed");
        assert_eq!(format_status(&SessionStatus::ContextExhausted), "exhausted");
        assert_eq!(format_status(&SessionStatus::Spawning), "spawning");
    }

    #[test]
    fn test_session_from_markdown() {
        let markdown = r#"---
id: session-123
stage_id: stage-456
tmux_session: flux-session-123
status: running
context_tokens: 45000
context_limit: 200000
created_at: 2026-01-06T12:00:00Z
last_active: 2026-01-06T13:30:00Z
---

# Session: session-123
"#;

        let session = session_from_markdown(markdown).unwrap();
        assert_eq!(session.id, "session-123");
        assert_eq!(session.stage_id, Some("stage-456".to_string()));
        assert_eq!(session.tmux_session, Some("flux-session-123".to_string()));
        assert_eq!(session.status, SessionStatus::Running);
        assert_eq!(session.context_tokens, 45000);
        assert_eq!(session.context_limit, 200000);
    }

    #[test]
    fn test_stage_from_markdown() {
        let markdown = r#"---
id: stage-123
name: Test Stage
description: A test stage
status: executing
session: session-456
created_at: 2026-01-06T12:00:00Z
updated_at: 2026-01-06T13:30:00Z
---

# Stage: Test Stage
"#;

        let stage = stage_from_markdown(markdown).unwrap();
        assert_eq!(stage.id, "stage-123");
        assert_eq!(stage.name, "Test Stage");
        assert_eq!(stage.description, Some("A test stage".to_string()));
        assert_eq!(stage.status, crate::models::stage::StageStatus::Executing);
        assert_eq!(stage.session, Some("session-456".to_string()));
    }
}
