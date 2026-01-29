//! Bash command validation for worktree isolation.
//!
//! Validates that bash commands don't violate worktree isolation boundaries.

use super::{BlockedReason, ValidationResult};
use regex::Regex;
use std::sync::LazyLock;

/// Error type for bash validation failures
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BashValidationError {
    /// The blocked reason
    pub reason: BlockedReason,
    /// The offending pattern found in the command
    pub pattern: String,
}

impl BashValidationError {
    /// Create a new bash validation error
    pub fn new(reason: BlockedReason, pattern: impl Into<String>) -> Self {
        Self {
            reason,
            pattern: pattern.into(),
        }
    }
}

// Compiled regex patterns for performance
static GIT_DASH_C_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"git\s+-C\s+").expect("Invalid regex"));
static GIT_WORK_TREE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"git\s+--work-tree").expect("Invalid regex"));
static PATH_TRAVERSAL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.\.[\\/]\.\.").expect("Invalid regex"));
static WORKTREES_ACCESS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.worktrees/([^/\s]+)").expect("Invalid regex"));

/// Validate a bash command for worktree isolation violations.
///
/// # Arguments
/// * `command` - The bash command to validate
/// * `current_stage` - The current stage ID (used to allow access to own worktree)
///
/// # Returns
/// * `ValidationResult::Allowed` if the command is safe
/// * `ValidationResult::Blocked(reason)` if the command violates isolation
///
/// # Examples
/// ```
/// use loom::hooks::validators::validate_bash_command;
///
/// // Safe command
/// let result = validate_bash_command("cargo build", "my-stage");
/// assert!(result.is_allowed());
///
/// // Blocked: git -C
/// let result = validate_bash_command("git -C ../other commit", "my-stage");
/// assert!(result.is_blocked());
/// ```
pub fn validate_bash_command(command: &str, current_stage: &str) -> ValidationResult {
    // Check for git -C (directory override)
    if GIT_DASH_C_PATTERN.is_match(command) {
        return ValidationResult::Blocked(BlockedReason::GitDirectoryOverride);
    }

    // Check for git --work-tree (directory override)
    if GIT_WORK_TREE_PATTERN.is_match(command) {
        return ValidationResult::Blocked(BlockedReason::GitDirectoryOverride);
    }

    // Check for ../../ path traversal
    if PATH_TRAVERSAL_PATTERN.is_match(command) {
        return ValidationResult::Blocked(BlockedReason::PathTraversal);
    }

    // Check for .worktrees/ access (allow current stage only)
    if let Some(captures) = WORKTREES_ACCESS_PATTERN.captures(command) {
        let accessed_stage = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        if accessed_stage != current_stage {
            return ValidationResult::Blocked(BlockedReason::CrossWorktreeAccess {
                target_stage: Some(accessed_stage.to_string()),
            });
        }
    }

    ValidationResult::Allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_normal_commands() {
        let commands = [
            "cargo build",
            "cargo test",
            "git status",
            "git add src/main.rs",
            "git commit -m 'test'",
            "ls -la",
            "pwd",
            "cat file.txt",
            "rg pattern src/",
        ];

        for cmd in &commands {
            let result = validate_bash_command(cmd, "test-stage");
            assert!(
                result.is_allowed(),
                "Expected '{}' to be allowed, but got blocked",
                cmd
            );
        }
    }

    #[test]
    fn test_blocks_git_dash_c() {
        let commands = [
            "git -C ../other status",
            "git -C /path/to/other commit",
            "git -C . status", // Even current dir is suspicious
        ];

        for cmd in &commands {
            let result = validate_bash_command(cmd, "test-stage");
            assert!(result.is_blocked(), "Expected '{}' to be blocked", cmd);
            assert_eq!(
                result.blocked_reason(),
                Some(&BlockedReason::GitDirectoryOverride)
            );
        }
    }

    #[test]
    fn test_blocks_git_work_tree() {
        let commands = [
            "git --work-tree=/other status",
            "git --work-tree=../parent status",
        ];

        for cmd in &commands {
            let result = validate_bash_command(cmd, "test-stage");
            assert!(result.is_blocked(), "Expected '{}' to be blocked", cmd);
            assert_eq!(
                result.blocked_reason(),
                Some(&BlockedReason::GitDirectoryOverride)
            );
        }
    }

    #[test]
    fn test_blocks_path_traversal() {
        let commands = [
            "cat ../../file.txt",
            "ls ../../../",
            r"cat ..\..\file.txt", // Windows-style
            "cd ../../other && ls",
        ];

        for cmd in &commands {
            let result = validate_bash_command(cmd, "test-stage");
            assert!(result.is_blocked(), "Expected '{}' to be blocked", cmd);
            assert_eq!(result.blocked_reason(), Some(&BlockedReason::PathTraversal));
        }
    }

    #[test]
    fn test_allows_single_parent() {
        // Single .. is generally OK (within worktree)
        let result = validate_bash_command("cat ../file.txt", "test-stage");
        assert!(result.is_allowed());
    }

    #[test]
    fn test_blocks_cross_worktree_access() {
        let result = validate_bash_command("ls .worktrees/other-stage/", "my-stage");
        assert!(result.is_blocked());
        assert!(matches!(
            result.blocked_reason(),
            Some(BlockedReason::CrossWorktreeAccess { .. })
        ));
    }

    #[test]
    fn test_allows_own_worktree_access() {
        // Should allow access to own worktree
        let result = validate_bash_command("ls .worktrees/my-stage/", "my-stage");
        assert!(result.is_allowed());
    }

    #[test]
    fn test_cross_worktree_captures_target() {
        let result = validate_bash_command("cat .worktrees/other-stage/file.txt", "my-stage");
        if let ValidationResult::Blocked(BlockedReason::CrossWorktreeAccess { target_stage }) =
            result
        {
            assert_eq!(target_stage, Some("other-stage".to_string()));
        } else {
            panic!("Expected CrossWorktreeAccess block");
        }
    }
}
