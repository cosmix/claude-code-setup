//! Hooks command implementation
//!
//! Provides commands for managing loom hooks independently of full orchestration.
//! Useful for developers who want to use loom hooks without running a full plan.

use anyhow::{Context, Result};
use std::env;

use crate::fs::permissions::{ensure_loom_permissions, install_loom_hooks, loom_hooks_config};

/// Install loom hooks to the current project
///
/// This command:
/// 1. Installs hook scripts to ~/.claude/hooks/loom/
/// 2. Configures .claude/settings.local.json with hooks and permissions
///
/// After running this command, hooks will be active for all Claude Code
/// sessions in this project without needing to run `loom init` with a plan.
pub fn install() -> Result<()> {
    println!("Installing loom hooks...\n");

    // Find repository root (where .git is)
    let repo_root = find_repo_root().context("Not in a git repository")?;

    // Install hooks to ~/.claude/hooks/loom/
    let scripts_installed = install_loom_hooks()?;
    if scripts_installed > 0 {
        println!("  Installed {scripts_installed} hook script(s) to ~/.claude/hooks/loom/");
    } else {
        println!("  Hook scripts already up to date in ~/.claude/hooks/loom/");
    }

    // Configure .claude/settings.local.json with hooks and permissions
    ensure_loom_permissions(&repo_root)?;

    println!("\nHooks installed successfully!");
    println!("\nActive hooks:");
    println!("  - prefer-modern-tools.sh  Guides grep/find usage toward native tools or fd/rg");
    println!("  - commit-guard.sh         Enforces commit before session end (in worktrees)");
    println!("  - ask-user-pre/post.sh    Manages stage waiting state on user questions");
    println!("  - post-tool-use.sh        Updates heartbeat after tool usage");
    println!();
    println!("Hooks are now active for all Claude Code sessions in this project.");

    Ok(())
}

/// List available loom hooks and their status
pub fn list() -> Result<()> {
    let hooks_config = loom_hooks_config();

    println!("Loom hooks configuration:\n");

    if let Some(obj) = hooks_config.as_object() {
        for (event, rules) in obj {
            println!("{event}:");
            if let Some(rules_arr) = rules.as_array() {
                for rule in rules_arr {
                    if let (Some(matcher), Some(hooks)) =
                        (rule.get("matcher"), rule.get("hooks"))
                    {
                        let matcher_str = matcher.as_str().unwrap_or("*");
                        if let Some(hooks_arr) = hooks.as_array() {
                            for hook in hooks_arr {
                                if let Some(cmd) = hook.get("command").and_then(|c| c.as_str()) {
                                    let script_name = std::path::Path::new(cmd)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or(cmd);
                                    println!("  [{matcher_str}] -> {script_name}");
                                }
                            }
                        }
                    }
                }
            }
            println!();
        }
    }

    // Check if hooks are installed
    let home_dir = dirs::home_dir();
    if let Some(home) = home_dir {
        let hooks_dir = home.join(".claude/hooks/loom");
        if hooks_dir.exists() {
            println!("Hook scripts installed at: {}", hooks_dir.display());
        } else {
            println!("Hook scripts not installed. Run 'loom hooks install' to install them.");
        }
    }

    Ok(())
}

/// Find the repository root directory (containing .git)
fn find_repo_root() -> Result<std::path::PathBuf> {
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    let mut dir = current_dir.as_path();

    loop {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => anyhow::bail!("Not in a git repository"),
        }
    }
}
