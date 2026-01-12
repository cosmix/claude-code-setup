//! Knowledge command - manage curated codebase knowledge.
//!
//! Design principle: Claude Code already has Glob, Grep, Read, LSP tools.
//! We curate high-level knowledge that helps agents know WHERE to look,
//! not raw indexing.

use crate::fs::knowledge::{KnowledgeDir, KnowledgeFile, MigrationResult};
use crate::fs::work_dir::WorkDir;
use anyhow::{bail, Context, Result};
use colored::Colorize;

/// Show the knowledge summary or a specific knowledge file
pub fn show(file: Option<String>) -> Result<()> {
    let work_dir = WorkDir::new(".")?;
    work_dir.load()?;

    let project_root = work_dir
        .project_root()
        .context("Could not determine project root")?;
    let knowledge = KnowledgeDir::new(project_root);

    if !knowledge.exists() {
        println!(
            "{} Knowledge directory not found. Run 'loom knowledge init' to create it.",
            "─".dimmed()
        );
        return Ok(());
    }

    match file {
        Some(file_name) => {
            // Show specific file
            let file_type = parse_file_type(&file_name)?;
            let content = knowledge.read(file_type)?;
            println!("{content}");
        }
        None => {
            // Show summary
            let summary = knowledge.generate_summary()?;
            if summary.is_empty() {
                println!("{} No knowledge files have content yet.", "─".dimmed());
            } else {
                println!("{summary}");
            }
        }
    }

    Ok(())
}

/// Update (append to) a knowledge file
pub fn update(file: String, content: String) -> Result<()> {
    let work_dir = WorkDir::new(".")?;
    work_dir.load()?;

    let project_root = work_dir
        .project_root()
        .context("Could not determine project root")?;
    let knowledge = KnowledgeDir::new(project_root);

    if !knowledge.exists() {
        knowledge
            .initialize()
            .context("Failed to initialize knowledge directory")?;
    }

    let file_type = parse_file_type(&file)?;
    knowledge.append(file_type, &content)?;

    println!(
        "{} Appended to {}",
        "✓".green().bold(),
        file_type.filename()
    );

    Ok(())
}

/// Initialize the knowledge directory with default files
pub fn init() -> Result<()> {
    let work_dir = WorkDir::new(".")?;
    work_dir.load()?;

    let project_root = work_dir
        .project_root()
        .context("Could not determine project root")?;
    let knowledge = KnowledgeDir::new(&project_root);

    if knowledge.exists() {
        println!(
            "{} Knowledge directory already exists at {}",
            "─".dimmed(),
            knowledge.root().display()
        );
        return Ok(());
    }

    // Check for migration from old .work/knowledge/ location
    let migration_result = KnowledgeDir::migrate_from_work_dir(&project_root)?;

    match migration_result {
        MigrationResult::Migrated { files_copied } => {
            println!(
                "{} Migrated {} knowledge file{} from .work/knowledge/ to doc/loom/knowledge/",
                "✓".green().bold(),
                files_copied,
                if files_copied == 1 { "" } else { "s" }
            );
            println!();
            println!(
                "{} Old .work/knowledge/ directory preserved. You can delete it manually after verifying migration.",
                "─".dimmed()
            );

            // Initialize any missing default files after migration
            knowledge.initialize()?;
        }
        MigrationResult::NotNeeded => {
            // No migration needed, just initialize normally
            knowledge.initialize()?;

            println!("{} Initialized knowledge directory", "✓".green().bold());
            println!();
            println!("Created files:");
            for file_type in KnowledgeFile::all() {
                println!("  {} - {}", file_type.filename(), file_type.description());
            }
        }
    }

    Ok(())
}

/// List all knowledge files
pub fn list() -> Result<()> {
    let work_dir = WorkDir::new(".")?;
    work_dir.load()?;

    let project_root = work_dir
        .project_root()
        .context("Could not determine project root")?;
    let knowledge = KnowledgeDir::new(project_root);

    if !knowledge.exists() {
        println!(
            "{} Knowledge directory not found. Run 'loom knowledge init' to create it.",
            "─".dimmed()
        );
        return Ok(());
    }

    let files = knowledge.list_files()?;

    if files.is_empty() {
        println!("{} No knowledge files found.", "─".dimmed());
        return Ok(());
    }

    println!("{}", "Knowledge Files".bold());
    println!();

    for (file_type, path) in files {
        let content = std::fs::read_to_string(&path).ok();
        let line_count = content.as_ref().map(|c| c.lines().count()).unwrap_or(0);

        println!(
            "  {} {} ({} lines)",
            "─".dimmed(),
            file_type.filename().cyan(),
            line_count
        );
        println!("    {}", file_type.description().dimmed());
    }

    Ok(())
}

/// Parse a file type from a string argument
fn parse_file_type(file: &str) -> Result<KnowledgeFile> {
    // Try exact filename match first
    if let Some(file_type) = KnowledgeFile::from_filename(file) {
        return Ok(file_type);
    }

    // Try matching without .md extension
    let with_ext = format!("{file}.md");
    if let Some(file_type) = KnowledgeFile::from_filename(&with_ext) {
        return Ok(file_type);
    }

    // Try common aliases
    match file.to_lowercase().as_str() {
        "entry" | "entries" | "entry-point" | "entrypoints" => Ok(KnowledgeFile::EntryPoints),
        "pattern" | "arch" | "architecture" => Ok(KnowledgeFile::Patterns),
        "convention" | "conventions" | "code" | "coding" => Ok(KnowledgeFile::Conventions),
        _ => {
            let valid_files: Vec<_> = KnowledgeFile::all().iter().map(|f| f.filename()).collect();
            bail!(
                "Unknown knowledge file: '{}'. Valid files: {}",
                file,
                valid_files.join(", ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_dir = temp_dir.path().to_path_buf();

        // Create minimal .work directory structure
        let work_dir_path = test_dir.join(".work");
        fs::create_dir_all(&work_dir_path).expect("Failed to create .work dir");

        // Create required subdirectories
        for subdir in &[
            "runners", "tracks", "signals", "handoffs", "archive", "stages", "sessions", "logs",
            "crashes",
        ] {
            fs::create_dir(work_dir_path.join(subdir)).expect("Failed to create subdir");
        }

        (temp_dir, test_dir)
    }

    #[test]
    fn test_parse_file_type() {
        assert_eq!(
            parse_file_type("entry-points.md").unwrap(),
            KnowledgeFile::EntryPoints
        );
        assert_eq!(
            parse_file_type("entry-points").unwrap(),
            KnowledgeFile::EntryPoints
        );
        assert_eq!(
            parse_file_type("patterns").unwrap(),
            KnowledgeFile::Patterns
        );
        assert_eq!(
            parse_file_type("conventions").unwrap(),
            KnowledgeFile::Conventions
        );
        assert_eq!(
            parse_file_type("entry").unwrap(),
            KnowledgeFile::EntryPoints
        );
        assert!(parse_file_type("unknown").is_err());
    }

    #[test]
    #[serial]
    fn test_knowledge_init() {
        let (_temp_dir, test_dir) = setup_test_env();

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(&test_dir).expect("Failed to change dir");

        let result = init();
        assert!(result.is_ok());

        // Verify files were created at doc/loom/knowledge (not .work/knowledge)
        let knowledge_dir = test_dir.join("doc/loom/knowledge");
        assert!(knowledge_dir.exists());
        assert!(knowledge_dir.join("entry-points.md").exists());
        assert!(knowledge_dir.join("patterns.md").exists());
        assert!(knowledge_dir.join("conventions.md").exists());

        std::env::set_current_dir(original_dir).expect("Failed to restore dir");
    }

    #[test]
    #[serial]
    fn test_knowledge_update() {
        let (_temp_dir, test_dir) = setup_test_env();

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(&test_dir).expect("Failed to change dir");

        // Initialize first
        init().expect("Failed to init knowledge");

        // Update entry-points
        let result = update(
            "entry-points".to_string(),
            "## New Section\n\n- New entry".to_string(),
        );
        assert!(result.is_ok());

        // Verify content was appended at doc/loom/knowledge (not .work/knowledge)
        let content =
            fs::read_to_string(test_dir.join("doc/loom/knowledge/entry-points.md")).unwrap();
        assert!(content.contains("## New Section"));
        assert!(content.contains("- New entry"));

        std::env::set_current_dir(original_dir).expect("Failed to restore dir");
    }

    #[test]
    #[serial]
    fn test_knowledge_init_with_migration() {
        let (_temp_dir, test_dir) = setup_test_env();

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(&test_dir).expect("Failed to change dir");

        // Create old .work/knowledge/ directory with content
        let old_knowledge_dir = test_dir.join(".work/knowledge");
        fs::create_dir_all(&old_knowledge_dir).expect("Failed to create old knowledge dir");

        // Write some knowledge files to old location
        let old_entry_points = "# Entry Points\n\n## Main\n\n- src/main.rs - entry point";
        fs::write(old_knowledge_dir.join("entry-points.md"), old_entry_points)
            .expect("Failed to write old entry-points");

        let old_patterns = "# Patterns\n\n## Error Handling\n\n- Uses anyhow";
        fs::write(old_knowledge_dir.join("patterns.md"), old_patterns)
            .expect("Failed to write old patterns");

        // Run init - should trigger migration
        let result = init();
        assert!(result.is_ok());

        // Verify files were migrated to new location
        let new_knowledge_dir = test_dir.join("doc/loom/knowledge");
        assert!(new_knowledge_dir.exists());
        assert!(new_knowledge_dir.join("entry-points.md").exists());
        assert!(new_knowledge_dir.join("patterns.md").exists());
        assert!(new_knowledge_dir.join("conventions.md").exists()); // default file created

        // Verify migrated content was preserved
        let new_content =
            fs::read_to_string(new_knowledge_dir.join("entry-points.md")).unwrap();
        assert_eq!(new_content, old_entry_points);

        // Verify old files still exist (not deleted)
        assert!(old_knowledge_dir.join("entry-points.md").exists());
        assert!(old_knowledge_dir.join("patterns.md").exists());

        std::env::set_current_dir(original_dir).expect("Failed to restore dir");
    }
}
