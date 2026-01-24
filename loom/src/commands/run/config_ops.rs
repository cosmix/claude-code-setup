//! Plan configuration operations
//!
//! This module handles reading and writing plan source path configuration
//! in config.toml.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::fs::work_dir::WorkDir;

/// Get the plan source path from config.toml
pub fn get_plan_source_path(work_dir: &WorkDir) -> Result<Option<PathBuf>> {
    match work_dir.load_config()? {
        Some(config) => Ok(config.source_path()),
        None => Ok(None),
    }
}

/// Update the plan source path in config.toml
pub fn update_plan_source_path(work_dir: &WorkDir, new_path: &Path) -> Result<()> {
    let mut config = work_dir.load_config_required()?;

    if let Some(plan) = config.as_toml_mut().get_mut("plan") {
        if let Some(table) = plan.as_table_mut() {
            table.insert(
                "source_path".to_string(),
                toml::Value::String(new_path.display().to_string()),
            );
        }
    }

    // Serialize back to TOML with proper formatting
    let new_content = config.to_toml_string()?;
    fs::write(work_dir.config_path(), new_content).context("Failed to write config.toml")?;

    Ok(())
}
