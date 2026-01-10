//! Worktree operations
//!
//! Core CRUD operations for git worktrees: create, remove, list, get_or_create.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::models::worktree::Worktree;

use super::checks::is_valid_git_worktree;
use super::parser::{parse_worktree_list, WorktreeInfo};
use super::settings::{
    cleanup_worktree_settings, ensure_work_symlink, setup_claude_directory, setup_root_claude_md,
};

/// Create a new worktree for a stage
///
/// Creates: .worktrees/{stage_id}/ with branch loom/{stage_id}
/// Also creates symlink .worktrees/{stage_id}/.work -> main .work/
pub fn create_worktree(stage_id: &str, repo_root: &Path) -> Result<Worktree> {
    let worktree_path = repo_root.join(".worktrees").join(stage_id);
    let branch_name = format!("loom/{stage_id}");

    // Ensure .worktrees directory exists
    let worktrees_dir = repo_root.join(".worktrees");
    if !worktrees_dir.exists() {
        std::fs::create_dir_all(&worktrees_dir)
            .with_context(|| "Failed to create .worktrees directory")?;
    }

    // Check if worktree already exists
    if worktree_path.exists() {
        bail!("Worktree already exists at {}", worktree_path.display());
    }

    // Create the worktree with a new branch
    // git worktree add .worktrees/{stage_id} -b loom/{stage_id}
    let output = Command::new("git")
        .args(["worktree", "add", "-b", &branch_name])
        .arg(&worktree_path)
        .current_dir(repo_root)
        .output()
        .with_context(|| "Failed to execute git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // If branch already exists, try without -b
        if stderr.contains("already exists") {
            let output = Command::new("git")
                .args(["worktree", "add"])
                .arg(&worktree_path)
                .arg(&branch_name)
                .current_dir(repo_root)
                .output()
                .with_context(|| "Failed to execute git worktree add")?;

            if !output.status.success() {
                let stderr_msg = String::from_utf8_lossy(&output.stderr);
                bail!("git worktree add failed: {stderr_msg}");
            }
        } else {
            bail!("git worktree add failed: {stderr}");
        }
    }

    // Create symlink to main .work/ directory
    ensure_work_symlink(&worktree_path, repo_root)?;

    // Set up .claude/ directory for worktree
    setup_claude_directory(&worktree_path, repo_root)?;

    // Symlink project-root CLAUDE.md
    setup_root_claude_md(&worktree_path, repo_root)?;

    let mut worktree = Worktree::new(stage_id.to_string(), worktree_path, branch_name);
    worktree.mark_active();

    Ok(worktree)
}

/// Remove a worktree
///
/// Runs: git worktree remove .worktrees/{stage_id}
pub fn remove_worktree(stage_id: &str, repo_root: &Path, force: bool) -> Result<()> {
    let worktree_path = repo_root.join(".worktrees").join(stage_id);

    if !worktree_path.exists() {
        bail!("Worktree does not exist: {}", worktree_path.display());
    }

    // Clean up settings and symlinks first
    cleanup_worktree_settings(&worktree_path);

    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }

    let output = Command::new("git")
        .args(&args)
        .arg(&worktree_path)
        .current_dir(repo_root)
        .output()
        .with_context(|| "Failed to execute git worktree remove")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree remove failed: {stderr}");
    }

    Ok(())
}

/// List all worktrees
pub fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .with_context(|| "Failed to execute git worktree list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree list failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_worktree_list(&stdout)
}

/// Clean orphaned worktrees (prune)
pub fn clean_worktrees(repo_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(repo_root)
        .output()
        .with_context(|| "Failed to execute git worktree prune")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree prune failed: {stderr}");
    }

    Ok(())
}

/// Get an existing worktree or create a new one
///
/// If a valid worktree exists at .worktrees/{stage_id}/, reuses it.
/// If the directory exists but is not a valid worktree, removes it and recreates.
/// Otherwise, creates a new worktree.
///
/// This function is idempotent and safe to call multiple times for the same stage.
pub fn get_or_create_worktree(stage_id: &str, repo_root: &Path) -> Result<Worktree> {
    let worktree_path = repo_root.join(".worktrees").join(stage_id);
    let branch_name = format!("loom/{stage_id}");

    if worktree_path.exists() {
        // Check if it's a valid git worktree by looking for the .git file
        // Git worktrees have a .git file (not directory) that points to the main repo
        let git_file = worktree_path.join(".git");
        if git_file.exists() {
            // Verify it's actually tracked by git worktree list
            if is_valid_git_worktree(&worktree_path, repo_root)? {
                // Valid worktree exists, return it
                let mut worktree = Worktree::new(stage_id.to_string(), worktree_path, branch_name);
                worktree.mark_active();
                return Ok(worktree);
            }
        }

        // Directory exists but is not a valid worktree - remove it
        // First try to prune any stale worktree references
        let _ = clean_worktrees(repo_root);

        // Now remove the directory
        std::fs::remove_dir_all(&worktree_path).with_context(|| {
            format!(
                "Failed to remove invalid worktree directory: {}",
                worktree_path.display()
            )
        })?;
    }

    // Create new worktree
    create_worktree(stage_id, repo_root)
}
