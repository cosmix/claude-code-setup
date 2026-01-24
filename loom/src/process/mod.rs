//! Process utilities for loom
//!
//! This module provides common process management functions used across the codebase.

use std::process::Command;

/// Check if a process with the given PID is alive
///
/// Uses `kill -0` to check if the process exists and can receive signals.
/// This doesn't actually send a signal to the process, it only checks if
/// it exists and is owned by the current user (or we have permission to signal it).
///
/// # Arguments
/// * `pid` - The process ID to check
///
/// # Returns
/// * `true` - The process exists and we can signal it
/// * `false` - The process doesn't exist or we can't signal it
///
/// # Example
/// ```ignore
/// use loom::process::is_process_alive;
///
/// let our_pid = std::process::id();
/// assert!(is_process_alive(our_pid));
///
/// // Non-existent PID
/// assert!(!is_process_alive(999999999));
/// ```
pub fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_process_is_alive() {
        // Our own process should be alive
        let our_pid = std::process::id();
        assert!(is_process_alive(our_pid));
    }

    #[test]
    fn test_nonexistent_process_is_not_alive() {
        // A very high PID is unlikely to exist
        assert!(!is_process_alive(999999999));
    }

    #[test]
    fn test_pid_one_behavior() {
        // PID 1 is init/systemd, we may or may not be able to signal it
        // depending on permissions, so we just test it doesn't panic
        let _ = is_process_alive(1);
    }
}
