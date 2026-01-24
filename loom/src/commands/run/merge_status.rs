//! Stage merge status checking
//!
//! This module provides functionality to check if all stages in a plan
//! have been merged by reading stage files and their frontmatter.

use anyhow::{Context, Result};
use std::fs;

use crate::fs::work_dir::WorkDir;
use crate::parser::frontmatter::extract_frontmatter_field;

/// Check if all stages are merged by reading stage files.
pub fn all_stages_merged(work_dir: &WorkDir) -> Result<bool> {
    let stages_dir = work_dir.root().join("stages");

    if !stages_dir.exists() {
        return Ok(false);
    }

    let entries = fs::read_dir(&stages_dir).context("Failed to read stages directory")?;

    let mut found_any_stage = false;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        found_any_stage = true;

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read stage file: {}", path.display()))?;

        // Parse YAML frontmatter to check merged status
        match extract_frontmatter_field(&content, "merged") {
            Ok(Some(value)) if value == "true" => {
                // Stage is merged, continue checking others
            }
            _ => {
                // Not merged or error parsing
                return Ok(false);
            }
        }
    }

    // Must have at least one stage to be considered "all merged"
    Ok(found_any_stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_work_dir(temp_dir: &TempDir) -> WorkDir {
        let work_dir = WorkDir::new(temp_dir.path()).unwrap();
        work_dir.initialize().unwrap();
        work_dir
    }

    fn create_stage_file(work_dir: &WorkDir, stage_id: &str, merged: bool) {
        let stages_dir = work_dir.root().join("stages");
        fs::create_dir_all(&stages_dir).unwrap();
        let content = format!(
            "---\nid: {stage_id}\nname: Test Stage\nstatus: Completed\nmerged: {merged}\n---\n# Stage\n"
        );
        fs::write(stages_dir.join(format!("0-{stage_id}.md")), content).unwrap();
    }

    #[test]
    fn test_extract_frontmatter_field() {
        let content = "---\nstatus: completed\nmerged: true\n---\n# Content";
        let merged = extract_frontmatter_field(content, "merged");
        assert_eq!(merged.unwrap(), Some("true".to_string()));

        let no_fm = "# Just content";
        assert!(extract_frontmatter_field(no_fm, "merged").is_err());
    }

    #[test]
    fn test_all_stages_merged_empty_stages_dir() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = create_test_work_dir(&temp_dir);
        // No stages directory

        let result = all_stages_merged(&work_dir).unwrap();

        assert!(!result); // Empty = not merged
    }

    #[test]
    fn test_all_stages_merged_ignores_non_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = create_test_work_dir(&temp_dir);

        let stages_dir = work_dir.root().join("stages");
        fs::create_dir_all(&stages_dir).unwrap();
        fs::write(stages_dir.join("readme.txt"), "Not a stage").unwrap();

        // With only non-markdown files, returns false (no stages found)
        let result = all_stages_merged(&work_dir).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_all_stages_merged_true() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = create_test_work_dir(&temp_dir);

        create_stage_file(&work_dir, "stage-1", true);
        create_stage_file(&work_dir, "stage-2", true);

        let result = all_stages_merged(&work_dir).unwrap();
        assert!(result);
    }

    #[test]
    fn test_all_stages_merged_false_when_one_not_merged() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = create_test_work_dir(&temp_dir);

        create_stage_file(&work_dir, "stage-1", true);
        create_stage_file(&work_dir, "stage-2", false);

        let result = all_stages_merged(&work_dir).unwrap();
        assert!(!result);
    }
}
