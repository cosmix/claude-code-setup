//! Desktop notification support for orchestrator events.
//!
//! Sends desktop notifications for events that need human attention,
//! using notify-send on Linux and osascript on macOS.

use crate::utils::truncate;
use std::process::Command;

/// Send a desktop notification.
///
/// Uses platform-appropriate notification tools:
/// - Linux: `notify-send`
/// - macOS: `osascript` with display notification
///
/// Failures are logged but never propagated - notifications are best-effort.
pub fn send_desktop_notification(title: &str, body: &str) {
    let result = if cfg!(target_os = "macos") {
        send_macos_notification(title, body)
    } else {
        send_linux_notification(title, body)
    };

    if let Err(e) = result {
        eprintln!("Desktop notification failed: {e}");
    }
}

fn send_linux_notification(title: &str, body: &str) -> Result<(), String> {
    Command::new("notify-send")
        .arg("--urgency=critical")
        .arg("--app-name=loom")
        .arg(title)
        .arg(body)
        .output()
        .map_err(|e| format!("notify-send failed: {e}"))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!("notify-send exited with: {}", output.status))
            }
        })
}

fn send_macos_notification(title: &str, body: &str) -> Result<(), String> {
    use crate::orchestrator::terminal::emulator::escape_applescript_string;

    let script = format!(
        r#"display notification "{}" with title "{}""#,
        escape_applescript_string(body),
        escape_applescript_string(title)
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("osascript failed: {e}"))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!("osascript exited with: {}", output.status))
            }
        })
}

/// Notify the user that a stage needs human review.
pub fn notify_needs_human_review(stage_id: &str, review_reason: Option<&str>) {
    let title = format!("loom: Stage '{}' needs review", stage_id);
    let body = review_reason
        .map(|r| truncate(r, 200))
        .unwrap_or_else(|| "A stage requires human review.".to_string());

    send_desktop_notification(&title, &body);
}
