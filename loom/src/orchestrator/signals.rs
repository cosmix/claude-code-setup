use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::handoff::git_handoff::{format_git_history_markdown, GitHistory};
use crate::models::session::Session;
use crate::models::stage::Stage;
use crate::models::worktree::Worktree;

/// Embedded context to include directly in signals so agents don't need to read from main repo
#[derive(Debug, Clone, Default)]
pub struct EmbeddedContext {
    /// Content of the handoff file (if resuming from a previous session)
    pub handoff_content: Option<String>,
    /// Content of structure.md (codebase structure map)
    pub structure_content: Option<String>,
    /// Plan overview extracted from the plan file
    pub plan_overview: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub stage_id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct SignalContent {
    pub session_id: String,
    pub stage_id: String,
    pub plan_id: Option<String>,
    pub stage_name: String,
    pub description: String,
    pub tasks: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub context_files: Vec<String>,
    pub files_to_modify: Vec<String>,
    pub git_history: Option<GitHistory>,
}

/// Content for a merge conflict resolution signal
#[derive(Debug, Clone)]
pub struct MergeSignalContent {
    pub session_id: String,
    pub stage_id: String,
    pub source_branch: String,
    pub target_branch: String,
    pub conflicting_files: Vec<String>,
}

#[derive(Debug, Default)]
pub struct SignalUpdates {
    pub add_tasks: Option<Vec<String>>,
    pub update_dependencies: Option<Vec<DependencyStatus>>,
    pub add_context_files: Option<Vec<String>>,
}

pub fn generate_signal(
    session: &Session,
    stage: &Stage,
    worktree: &Worktree,
    dependencies_status: &[DependencyStatus],
    handoff_file: Option<&str>,
    git_history: Option<&GitHistory>,
    work_dir: &Path,
) -> Result<PathBuf> {
    let signals_dir = work_dir.join("signals");

    if !signals_dir.exists() {
        fs::create_dir_all(&signals_dir).context("Failed to create signals directory")?;
    }

    // Build embedded context by reading files
    let embedded_context = build_embedded_context(work_dir, handoff_file);

    let signal_path = signals_dir.join(format!("{}.md", session.id));
    let content = format_signal_content(
        session,
        stage,
        worktree,
        dependencies_status,
        handoff_file,
        git_history,
        &embedded_context,
    );

    fs::write(&signal_path, content)
        .with_context(|| format!("Failed to write signal file: {}", signal_path.display()))?;

    Ok(signal_path)
}

/// Build embedded context by reading handoff, structure.md, and plan overview files
fn build_embedded_context(work_dir: &Path, handoff_file: Option<&str>) -> EmbeddedContext {
    let mut context = EmbeddedContext::default();

    // Read handoff content if specified
    if let Some(handoff_name) = handoff_file {
        let handoff_path = work_dir.join("handoffs").join(format!("{handoff_name}.md"));
        if handoff_path.exists() {
            context.handoff_content = fs::read_to_string(&handoff_path).ok();
        }
    }

    // Read structure.md if it exists
    let structure_path = work_dir.join("structure.md");
    if structure_path.exists() {
        context.structure_content = fs::read_to_string(&structure_path).ok();
    }

    // Read plan overview from config.toml and the plan file
    context.plan_overview = read_plan_overview(work_dir);

    context
}

/// Read the plan overview from the plan file referenced in config.toml
fn read_plan_overview(work_dir: &Path) -> Option<String> {
    let config_path = work_dir.join("config.toml");
    if !config_path.exists() {
        return None;
    }

    let config_content = fs::read_to_string(&config_path).ok()?;
    let config: toml::Value = config_content.parse().ok()?;

    let source_path = config.get("plan")?.get("source_path")?.as_str()?;

    let plan_path = PathBuf::from(source_path);
    if !plan_path.exists() {
        return None;
    }

    let plan_content = fs::read_to_string(&plan_path).ok()?;

    // Extract overview section from plan markdown
    extract_plan_overview(&plan_content)
}

/// Extract overview and proposed changes sections from plan markdown
fn extract_plan_overview(plan_content: &str) -> Option<String> {
    let mut overview = String::new();
    let mut in_relevant_section = false;
    let mut current_section = String::new();

    for line in plan_content.lines() {
        // Detect section headers
        if line.starts_with("## ") {
            let section_name = line.trim_start_matches("## ").trim().to_lowercase();

            // Save accumulated content from previous relevant section
            if in_relevant_section && !current_section.is_empty() {
                overview.push_str(&current_section);
                overview.push_str("\n\n");
                current_section.clear();
            }

            // Check if entering a relevant section
            in_relevant_section = section_name.contains("overview")
                || section_name.contains("proposed changes")
                || section_name.contains("summary")
                || section_name.contains("current state");

            if in_relevant_section {
                current_section.push_str(line);
                current_section.push('\n');
            }
        } else if line.starts_with("# ") && overview.is_empty() {
            // Capture plan title
            overview.push_str(line);
            overview.push_str("\n\n");
        } else if in_relevant_section {
            // Stop at next major section (Stages, metadata, etc.)
            let trimmed = line.trim().to_lowercase();
            if trimmed.starts_with("## stages")
                || trimmed.starts_with("```yaml")
                || trimmed.starts_with("<!-- loom")
            {
                in_relevant_section = false;
                if !current_section.is_empty() {
                    overview.push_str(&current_section);
                    overview.push_str("\n\n");
                    current_section.clear();
                }
            } else {
                current_section.push_str(line);
                current_section.push('\n');
            }
        }
    }

    // Capture any remaining content
    if in_relevant_section && !current_section.is_empty() {
        overview.push_str(&current_section);
    }

    let trimmed = overview.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Generate a signal file for a merge conflict resolution session.
///
/// Unlike regular stage signals that run in worktrees, merge signals direct
/// the session to work in the main repository to resolve merge conflicts.
pub fn generate_merge_signal(
    session: &Session,
    stage: &Stage,
    source_branch: &str,
    target_branch: &str,
    conflicting_files: &[String],
    work_dir: &Path,
) -> Result<PathBuf> {
    let signals_dir = work_dir.join("signals");

    if !signals_dir.exists() {
        fs::create_dir_all(&signals_dir).context("Failed to create signals directory")?;
    }

    let signal_path = signals_dir.join(format!("{}.md", session.id));
    let content = format_merge_signal_content(
        session,
        stage,
        source_branch,
        target_branch,
        conflicting_files,
    );

    fs::write(&signal_path, &content).with_context(|| {
        format!(
            "Failed to write merge signal file: {}",
            signal_path.display()
        )
    })?;

    Ok(signal_path)
}

/// Read and parse a merge signal file.
///
/// Returns `None` if the signal file doesn't exist or isn't a merge signal.
pub fn read_merge_signal(session_id: &str, work_dir: &Path) -> Result<Option<MergeSignalContent>> {
    let signal_path = work_dir.join("signals").join(format!("{session_id}.md"));

    if !signal_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&signal_path).context("Failed to read signal file")?;

    // Check if this is a merge signal by looking for the merge-specific header
    if !content.contains("# Merge Signal:") {
        return Ok(None);
    }

    let parsed = parse_merge_signal_content(session_id, &content)?;
    Ok(Some(parsed))
}

pub fn update_signal(session_id: &str, updates: SignalUpdates, work_dir: &Path) -> Result<()> {
    let signal_path = work_dir.join("signals").join(format!("{session_id}.md"));

    if !signal_path.exists() {
        bail!("Signal file does not exist: {}", signal_path.display());
    }

    let content = fs::read_to_string(&signal_path).context("Failed to read signal file")?;

    let mut updated_content = content;

    if let Some(tasks) = updates.add_tasks {
        if !tasks.is_empty() {
            let task_section = tasks
                .iter()
                .enumerate()
                .map(|(i, task)| format!("{}. {}", i + 1, task))
                .collect::<Vec<_>>()
                .join("\n");

            if let Some(pos) = updated_content.find("## Immediate Tasks") {
                if let Some(next_section) = updated_content[pos..].find("\n\n## ") {
                    let insert_pos = pos + next_section;
                    updated_content.insert_str(insert_pos, &format!("\n{task_section}"));
                }
            }
        }
    }

    if let Some(deps) = updates.update_dependencies {
        if !deps.is_empty() {
            let dep_table = format_dependency_table(&deps);
            if let Some(start) = updated_content.find("## Dependencies Status") {
                if let Some(table_start) = updated_content[start..].find("| Dependency") {
                    let abs_table_start = start + table_start;
                    if let Some(next_section) = updated_content[abs_table_start..].find("\n\n## ") {
                        let end_pos = abs_table_start + next_section;
                        updated_content.replace_range(abs_table_start..end_pos, &dep_table);
                    }
                }
            }
        }
    }

    if let Some(files) = updates.add_context_files {
        if !files.is_empty() {
            let file_list = files
                .iter()
                .map(|f| format!("- {f}"))
                .collect::<Vec<_>>()
                .join("\n");

            if let Some(pos) = updated_content.find("## Context Restoration") {
                if let Some(next_section) = updated_content[pos..].find("\n\n## ") {
                    let insert_pos = pos + next_section;
                    updated_content.insert_str(insert_pos, &format!("\n{file_list}"));
                }
            }
        }
    }

    fs::write(&signal_path, updated_content).context("Failed to update signal file")?;

    Ok(())
}

pub fn remove_signal(session_id: &str, work_dir: &Path) -> Result<()> {
    let signal_path = work_dir.join("signals").join(format!("{session_id}.md"));

    if signal_path.exists() {
        fs::remove_file(&signal_path)
            .with_context(|| format!("Failed to remove signal file: {}", signal_path.display()))?;
    }

    Ok(())
}

pub fn read_signal(session_id: &str, work_dir: &Path) -> Result<Option<SignalContent>> {
    let signal_path = work_dir.join("signals").join(format!("{session_id}.md"));

    if !signal_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&signal_path).context("Failed to read signal file")?;

    let parsed = parse_signal_content(session_id, &content)?;
    Ok(Some(parsed))
}

pub fn list_signals(work_dir: &Path) -> Result<Vec<String>> {
    let signals_dir = work_dir.join("signals");

    if !signals_dir.exists() {
        return Ok(Vec::new());
    }

    let mut signals = Vec::new();

    for entry in fs::read_dir(signals_dir).context("Failed to read signals directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                signals.push(name.to_string());
            }
        }
    }

    signals.sort();
    Ok(signals)
}

fn format_signal_content(
    session: &Session,
    stage: &Stage,
    worktree: &Worktree,
    dependencies_status: &[DependencyStatus],
    handoff_file: Option<&str>,
    git_history: Option<&GitHistory>,
    embedded_context: &EmbeddedContext,
) -> String {
    let mut content = String::new();

    content.push_str(&format!("# Signal: {}\n\n", session.id));

    // Worktree context - self-contained signal
    content.push_str("## Worktree Context\n\n");
    content.push_str(
        "You are in an **isolated git worktree**. This signal contains everything you need:\n\n",
    );
    content.push_str("- **Your stage assignment and acceptance criteria are below** - this file is self-contained\n");
    content.push_str("- **All context (plan overview, handoff, structure map) is embedded below** - no need to read from main repo\n");
    content.push_str(
        "- **Commit to your worktree branch** - it will be merged after verification\n\n",
    );

    // Add reminder to follow CLAUDE.md rules
    content.push_str("## Execution Rules\n\n");
    content.push_str("Follow your `~/.claude/CLAUDE.md` and project `CLAUDE.md` rules (both are symlinked into this worktree). Key reminders:\n");
    content.push_str(
        "- **Delegate work to subagents** - use Task tool with appropriate agent types\n",
    );
    content.push_str("- **Use TodoWrite** to plan and track progress\n");
    content.push_str("- **Verify acceptance criteria** before marking stage complete\n");
    content.push_str("- **Create handoff** if context exceeds 75%\n\n");

    content.push_str("## Target\n\n");
    content.push_str(&format!("- **Session**: {}\n", session.id));
    content.push_str(&format!("- **Stage**: {}\n", stage.id));
    if let Some(plan_id) = &stage.plan_id {
        content.push_str(&format!(
            "- **Plan**: {plan_id} (overview embedded below)\n"
        ));
    }
    content.push_str(&format!("- **Worktree**: {}\n", worktree.path.display()));
    content.push_str(&format!("- **Branch**: {}\n", worktree.branch));
    content.push('\n');

    // Embed plan overview if available
    if let Some(plan_overview) = &embedded_context.plan_overview {
        content.push_str("## Plan Overview\n\n");
        content.push_str("<plan-overview>\n");
        content.push_str(plan_overview);
        content.push_str("\n</plan-overview>\n\n");
    }

    content.push_str("## Assignment\n\n");
    content.push_str(&format!("{}: ", stage.name));
    if let Some(desc) = &stage.description {
        content.push_str(desc);
    } else {
        content.push_str("(no description provided)");
    }
    content.push_str("\n\n");

    content.push_str("## Immediate Tasks\n\n");
    let tasks = extract_tasks_from_stage(stage);
    if tasks.is_empty() {
        content.push_str("1. Review stage acceptance criteria below\n");
        content.push_str("2. Implement required changes\n");
        content.push_str("3. Verify all acceptance criteria are met\n");
    } else {
        for (i, task) in tasks.iter().enumerate() {
            content.push_str(&format!("{}. {task}\n", i + 1));
        }
    }
    content.push('\n');

    if !dependencies_status.is_empty() {
        content.push_str("## Dependencies Status\n\n");
        content.push_str(&format_dependency_table(dependencies_status));
        content.push('\n');
    }

    // Embed handoff content if available (previous session context)
    if let Some(handoff_content) = &embedded_context.handoff_content {
        content.push_str("## Previous Session Handoff\n\n");
        content.push_str(
            "**READ THIS CAREFULLY** - This contains context from the previous session:\n\n",
        );
        content.push_str("<handoff>\n");
        content.push_str(handoff_content);
        content.push_str("\n</handoff>\n\n");
    } else if let Some(handoff) = handoff_file {
        // Fallback reference if content couldn't be read
        content.push_str("## Context Restoration\n\n");
        content.push_str(&format!(
            "- `.work/handoffs/{handoff}.md` - **READ THIS FIRST** - Previous session handoff\n\n"
        ));
    }

    // Embed structure.md content if available
    if let Some(structure_content) = &embedded_context.structure_content {
        content.push_str("## Codebase Structure\n\n");
        content.push_str("<structure-map>\n");
        content.push_str(structure_content);
        content.push_str("\n</structure-map>\n\n");
    }

    // Git History from previous session (if resuming)
    if let Some(history) = git_history {
        content.push_str(&format_git_history_markdown(history));
        content.push('\n');
    }

    content.push_str("## Acceptance Criteria\n\n");
    if stage.acceptance.is_empty() {
        content.push_str("- [ ] Implementation complete\n");
        content.push_str("- [ ] Code reviewed and tested\n");
    } else {
        for criterion in &stage.acceptance {
            content.push_str(&format!("- [ ] {criterion}\n"));
        }
    }
    content.push('\n');

    if !stage.files.is_empty() {
        content.push_str("## Files to Modify\n\n");
        for file in &stage.files {
            content.push_str(&format!("- {file}\n"));
        }
        content.push('\n');
    }

    content
}

fn format_dependency_table(deps: &[DependencyStatus]) -> String {
    let mut table = String::new();
    table.push_str("| Dependency | Status |\n");
    table.push_str("|------------|--------|\n");

    for dep in deps {
        let name = &dep.name;
        let status = &dep.status;
        table.push_str(&format!("| {name} | {status} |\n"));
    }

    table
}

fn extract_tasks_from_stage(stage: &Stage) -> Vec<String> {
    let mut tasks = Vec::new();

    if let Some(desc) = &stage.description {
        tasks.extend(extract_tasks_from_description(desc));
    }

    if tasks.is_empty() && !stage.acceptance.is_empty() {
        for criterion in &stage.acceptance {
            tasks.push(criterion.clone());
        }
    }

    tasks
}

fn extract_tasks_from_description(description: &str) -> Vec<String> {
    let mut tasks = Vec::new();

    for line in description.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            tasks.push(trimmed[2..].trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            if let Some(task) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
                tasks.push(task.trim().to_string());
            }
        }
    }

    tasks
}

fn parse_signal_content(session_id: &str, content: &str) -> Result<SignalContent> {
    let mut stage_id = String::new();
    let mut plan_id = None;
    let mut stage_name = String::new();
    let mut description = String::new();
    let mut tasks = Vec::new();
    let mut acceptance_criteria = Vec::new();
    let mut context_files = Vec::new();
    let mut files_to_modify = Vec::new();

    let mut current_section = "";

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("## ") {
            current_section = trimmed.trim_start_matches("## ");
            continue;
        }

        match current_section {
            "Target" => {
                if let Some(id) = trimmed.strip_prefix("- **Stage**: ") {
                    stage_id = id.to_string();
                } else if let Some(pid) = trimmed.strip_prefix("- **Plan**: ") {
                    // Strip the plan ID suffix if present (handles both old and new formats)
                    let clean_pid = pid
                        .strip_suffix(" (reference only - content embedded below)")
                        .or_else(|| pid.strip_suffix(" (overview embedded below)"))
                        .unwrap_or(pid);
                    plan_id = Some(clean_pid.to_string());
                }
            }
            "Assignment" => {
                if !trimmed.is_empty() && !description.is_empty() {
                    description.push('\n');
                }
                if let Some((name, desc)) = trimmed.split_once(": ") {
                    if stage_name.is_empty() {
                        stage_name = name.to_string();
                        description = desc.to_string();
                    } else {
                        description.push_str(trimmed);
                    }
                } else if !trimmed.is_empty() {
                    description.push_str(trimmed);
                }
            }
            "Immediate Tasks" => {
                if let Some(task) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                    if let Some(t) = task.strip_prefix(". ") {
                        tasks.push(t.to_string());
                    }
                }
            }
            "Acceptance Criteria" => {
                if let Some(criterion) = trimmed.strip_prefix("- [ ] ") {
                    acceptance_criteria.push(criterion.to_string());
                }
            }
            "Context Restoration" => {
                if let Some(file) = trimmed.strip_prefix("- `") {
                    if let Some(f) = file
                        .strip_suffix("` - Stage definition")
                        .or_else(|| {
                            file.strip_suffix("` - **READ THIS FIRST** - Previous session handoff")
                        })
                        .or_else(|| file.strip_suffix("` - Previous handoff"))
                        .or_else(|| file.strip_suffix("` - Codebase structure map (if exists)"))
                        .or_else(|| file.strip_suffix("` - Relevant code to modify"))
                        .or_else(|| file.strip_suffix("` - Relevant code"))
                        .or_else(|| file.strip_suffix('`'))
                    {
                        context_files.push(f.to_string());
                    }
                }
            }
            "Files to Modify" => {
                if let Some(file) = trimmed.strip_prefix("- ") {
                    files_to_modify.push(file.to_string());
                }
            }
            _ => {}
        }
    }

    if stage_id.is_empty() {
        bail!("Signal file is missing stage_id");
    }

    Ok(SignalContent {
        session_id: session_id.to_string(),
        stage_id,
        plan_id,
        stage_name,
        description,
        tasks,
        acceptance_criteria,
        context_files,
        files_to_modify,
        git_history: None, // Git history is informational, not parsed back
    })
}

fn format_merge_signal_content(
    session: &Session,
    stage: &Stage,
    source_branch: &str,
    target_branch: &str,
    conflicting_files: &[String],
) -> String {
    let mut content = String::new();

    content.push_str(&format!("# Merge Signal: {}\n\n", session.id));

    // Merge context - explain the situation
    content.push_str("## Merge Context\n\n");
    content.push_str("You are resolving a **merge conflict** in the main repository.\n\n");
    content.push_str("- This is NOT a regular stage execution - you are fixing conflicts\n");
    content.push_str("- Work directly in the main repository (not a worktree)\n");
    content.push_str("- Follow the merge instructions below carefully\n\n");

    // Execution rules for merge sessions
    content.push_str("## Execution Rules\n\n");
    content.push_str("Follow your `~/.claude/CLAUDE.md` rules. Key reminders:\n");
    content.push_str("- **Do NOT modify code** beyond what's needed for conflict resolution\n");
    content.push_str("- **Preserve intent from BOTH branches** where possible\n");
    content.push_str("- **Ask the user** if unclear how to resolve a conflict\n");
    content.push_str("- **Use TodoWrite** to track resolution progress\n\n");

    // Target information
    content.push_str("## Target\n\n");
    content.push_str(&format!("- **Session**: {}\n", session.id));
    content.push_str(&format!("- **Stage**: {}\n", stage.id));
    content.push_str(&format!("- **Source Branch**: {source_branch}\n"));
    content.push_str(&format!("- **Target Branch**: {target_branch}\n"));
    content.push('\n');

    // Stage context (if available)
    if let Some(desc) = &stage.description {
        content.push_str("## Stage Context\n\n");
        content.push_str(&format!("**{0}**: {1}\n\n", stage.name, desc));
    }

    // Conflicting files
    content.push_str("## Conflicting Files\n\n");
    if conflicting_files.is_empty() {
        content
            .push_str("_No specific files listed - run `git status` to see current conflicts_\n");
    } else {
        for file in conflicting_files {
            content.push_str(&format!("- `{file}`\n"));
        }
    }
    content.push('\n');

    // Task instructions
    content.push_str("## Your Task\n\n");
    content.push_str(&format!(
        "1. Run: `git merge {source_branch}` (if not already in merge state)\n"
    ));
    content.push_str("2. Resolve conflicts in the files listed above\n");
    content.push_str("3. Stage resolved files: `git add <resolved-files>`\n");
    content.push_str("4. Review changes and complete the merge: `git commit`\n\n");

    // Important notes
    content.push_str("## Important\n\n");
    content.push_str("- Do NOT modify code beyond what's needed for conflict resolution\n");
    content.push_str("- Preserve intent from BOTH branches where possible\n");
    content.push_str("- If unclear how to resolve, ask the user for guidance\n");
    content.push_str("- After completing the merge, loom will automatically detect and clean up\n");

    content
}

fn parse_merge_signal_content(session_id: &str, content: &str) -> Result<MergeSignalContent> {
    let mut stage_id = String::new();
    let mut source_branch = String::new();
    let mut target_branch = String::new();
    let mut conflicting_files = Vec::new();

    let mut current_section = "";

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("## ") {
            current_section = trimmed.trim_start_matches("## ");
            continue;
        }

        match current_section {
            "Target" => {
                if let Some(id) = trimmed.strip_prefix("- **Stage**: ") {
                    stage_id = id.to_string();
                } else if let Some(branch) = trimmed.strip_prefix("- **Source Branch**: ") {
                    source_branch = branch.to_string();
                } else if let Some(branch) = trimmed.strip_prefix("- **Target Branch**: ") {
                    target_branch = branch.to_string();
                }
            }
            "Conflicting Files" => {
                if let Some(file) = trimmed.strip_prefix("- `") {
                    if let Some(f) = file.strip_suffix('`') {
                        conflicting_files.push(f.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if stage_id.is_empty() {
        bail!("Merge signal file is missing stage_id");
    }

    Ok(MergeSignalContent {
        session_id: session_id.to_string(),
        stage_id,
        source_branch,
        target_branch,
        conflicting_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::session::Session;
    use crate::models::stage::{Stage, StageStatus};
    use crate::models::worktree::Worktree;
    use tempfile::TempDir;

    fn create_test_session() -> Session {
        let mut session = Session::new();
        session.id = "session-test-123".to_string();
        session.assign_to_stage("stage-1".to_string());
        session
    }

    fn create_test_stage() -> Stage {
        let mut stage = Stage::new(
            "Implement signals module".to_string(),
            Some("Create signal file generation logic".to_string()),
        );
        stage.id = "stage-1".to_string();
        stage.status = StageStatus::Executing;
        stage.add_acceptance_criterion("Signal files are generated correctly".to_string());
        stage.add_acceptance_criterion("All tests pass".to_string());
        stage.add_file_pattern("src/orchestrator/signals.rs".to_string());
        stage
    }

    fn create_test_worktree() -> Worktree {
        Worktree::new(
            "stage-1".to_string(),
            PathBuf::from("/repo/.worktrees/stage-1"),
            "loom/stage-1".to_string(),
        )
    }

    #[test]
    fn test_generate_signal_basic() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        let result = generate_signal(&session, &stage, &worktree, &[], None, None, &work_dir);

        assert!(result.is_ok());
        let signal_path = result.unwrap();
        assert!(signal_path.exists());

        let content = fs::read_to_string(&signal_path).unwrap();
        assert!(content.contains("# Signal: session-test-123"));
        assert!(content.contains("- **Session**: session-test-123"));
        assert!(content.contains("- **Stage**: stage-1"));
        assert!(content.contains("## Assignment"));
        assert!(content.contains("Implement signals module"));
    }

    #[test]
    fn test_generate_signal_with_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        let deps = vec![DependencyStatus {
            stage_id: "stage-0".to_string(),
            name: "Setup models".to_string(),
            status: "completed".to_string(),
        }];

        let result = generate_signal(&session, &stage, &worktree, &deps, None, None, &work_dir);

        assert!(result.is_ok());
        let signal_path = result.unwrap();
        let content = fs::read_to_string(&signal_path).unwrap();

        assert!(content.contains("## Dependencies Status"));
        assert!(content.contains("Setup models"));
        assert!(content.contains("completed"));
    }

    #[test]
    fn test_generate_signal_with_handoff() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        let result = generate_signal(
            &session,
            &stage,
            &worktree,
            &[],
            Some("2026-01-06-previous-work"),
            None,
            &work_dir,
        );

        assert!(result.is_ok());
        let signal_path = result.unwrap();
        let content = fs::read_to_string(&signal_path).unwrap();

        assert!(content.contains("## Context Restoration"));
        assert!(content.contains("2026-01-06-previous-work.md"));
    }

    #[test]
    fn test_format_signal_content() {
        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();
        let embedded_context = EmbeddedContext::default();

        let content = format_signal_content(
            &session,
            &stage,
            &worktree,
            &[],
            None,
            None,
            &embedded_context,
        );

        assert!(content.contains("# Signal: session-test-123"));
        assert!(content.contains("## Worktree Context"));
        assert!(content.contains("This signal contains everything you need"));
        assert!(content.contains("## Target"));
        assert!(content.contains("## Assignment"));
        assert!(content.contains("## Immediate Tasks"));
        assert!(content.contains("## Acceptance Criteria"));
        assert!(content.contains("## Files to Modify"));
        assert!(content.contains("src/orchestrator/signals.rs"));
    }

    #[test]
    fn test_format_signal_content_with_embedded_context() {
        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();
        let embedded_context = EmbeddedContext {
            handoff_content: Some(
                "# Handoff\nPrevious session completed tasks A and B.".to_string(),
            ),
            structure_content: Some("# Structure\nsrc/\n  main.rs\n  lib.rs".to_string()),
            plan_overview: Some("# Plan Title\n\n## Overview\nThis plan does X.".to_string()),
        };

        let content = format_signal_content(
            &session,
            &stage,
            &worktree,
            &[],
            None,
            None,
            &embedded_context,
        );

        // Verify embedded content is present
        assert!(content.contains("## Plan Overview"));
        assert!(content.contains("<plan-overview>"));
        assert!(content.contains("This plan does X."));
        assert!(content.contains("</plan-overview>"));

        assert!(content.contains("## Previous Session Handoff"));
        assert!(content.contains("<handoff>"));
        assert!(content.contains("Previous session completed tasks A and B."));
        assert!(content.contains("</handoff>"));

        assert!(content.contains("## Codebase Structure"));
        assert!(content.contains("<structure-map>"));
        assert!(content.contains("src/"));
        assert!(content.contains("</structure-map>"));
    }

    #[test]
    fn test_extract_plan_overview() {
        let plan_content = r#"# PLAN: Test Feature

## Overview

This is the overview section.
It has multiple lines.

## Current State

Current state description.

## Proposed Changes

Proposed changes here.

## Stages

### Stage 1: First Stage

Implementation details.

```yaml
loom:
  version: 1
```
"#;

        let overview = extract_plan_overview(plan_content).unwrap();
        assert!(overview.contains("# PLAN: Test Feature"));
        assert!(overview.contains("## Overview"));
        assert!(overview.contains("This is the overview section."));
        assert!(overview.contains("## Current State"));
        assert!(overview.contains("## Proposed Changes"));
        // Should NOT contain Stages section
        assert!(!overview.contains("### Stage 1"));
        assert!(!overview.contains("```yaml"));
    }

    #[test]
    fn test_extract_tasks_from_description() {
        let desc1 = "- First task\n- Second task\n- Third task";
        let tasks1 = extract_tasks_from_description(desc1);
        assert_eq!(tasks1.len(), 3);
        assert_eq!(tasks1[0], "First task");

        let desc2 = "1. First task\n2. Second task\n3. Third task";
        let tasks2 = extract_tasks_from_description(desc2);
        assert_eq!(tasks2.len(), 3);
        assert_eq!(tasks2[1], "Second task");

        let desc3 = "* Task one\n* Task two";
        let tasks3 = extract_tasks_from_description(desc3);
        assert_eq!(tasks3.len(), 2);
        assert_eq!(tasks3[0], "Task one");

        let desc4 = "No tasks here";
        let tasks4 = extract_tasks_from_description(desc4);
        assert_eq!(tasks4.len(), 0);
    }

    #[test]
    fn test_remove_signal() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(work_dir.join("signals")).unwrap();

        let signal_path = work_dir.join("signals").join("session-test-123.md");
        fs::write(&signal_path, "test content").unwrap();
        assert!(signal_path.exists());

        let result = remove_signal("session-test-123", &work_dir);
        assert!(result.is_ok());
        assert!(!signal_path.exists());

        let result2 = remove_signal("nonexistent", &work_dir);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_list_signals() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        let signals_dir = work_dir.join("signals");
        fs::create_dir_all(&signals_dir).unwrap();

        fs::write(signals_dir.join("session-1.md"), "").unwrap();
        fs::write(signals_dir.join("session-2.md"), "").unwrap();
        fs::write(signals_dir.join("session-3.md"), "").unwrap();
        fs::write(signals_dir.join("not-a-signal.txt"), "").unwrap();

        let signals = list_signals(&work_dir).unwrap();
        assert_eq!(signals.len(), 3);
        assert!(signals.contains(&"session-1".to_string()));
        assert!(signals.contains(&"session-2".to_string()));
        assert!(signals.contains(&"session-3".to_string()));
        assert!(!signals.contains(&"not-a-signal".to_string()));
    }

    #[test]
    fn test_read_signal() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        generate_signal(&session, &stage, &worktree, &[], None, None, &work_dir).unwrap();

        let result = read_signal("session-test-123", &work_dir);
        assert!(result.is_ok());

        let signal_content = result.unwrap();
        assert!(signal_content.is_some());

        let content = signal_content.unwrap();
        assert_eq!(content.session_id, "session-test-123");
        assert_eq!(content.stage_id, "stage-1");
        assert_eq!(content.stage_name, "Implement signals module");
        assert!(!content.acceptance_criteria.is_empty());
    }

    #[test]
    fn test_update_signal_add_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        generate_signal(&session, &stage, &worktree, &[], None, None, &work_dir).unwrap();

        let updates = SignalUpdates {
            add_tasks: Some(vec!["New task 1".to_string(), "New task 2".to_string()]),
            ..Default::default()
        };

        let result = update_signal("session-test-123", updates, &work_dir);
        assert!(result.is_ok());

        let signal_path = work_dir.join("signals").join("session-test-123.md");
        let content = fs::read_to_string(signal_path).unwrap();
        assert!(content.contains("New task 1"));
        assert!(content.contains("New task 2"));
    }

    #[test]
    fn test_generate_signal_with_git_history() {
        use crate::handoff::git_handoff::{CommitInfo, GitHistory};

        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        let git_history = GitHistory {
            branch: "loom/stage-1".to_string(),
            base_branch: "main".to_string(),
            commits: vec![CommitInfo {
                hash: "abc1234".to_string(),
                message: "Add feature".to_string(),
            }],
            uncommitted_changes: vec!["M src/test.rs".to_string()],
        };

        let result = generate_signal(
            &session,
            &stage,
            &worktree,
            &[],
            None,
            Some(&git_history),
            &work_dir,
        );

        assert!(result.is_ok());
        let signal_path = result.unwrap();
        let content = fs::read_to_string(&signal_path).unwrap();

        assert!(content.contains("## Git History"));
        assert!(content.contains("**Branch**: loom/stage-1 (from main)"));
        assert!(content.contains("abc1234"));
        assert!(content.contains("Add feature"));
        assert!(content.contains("M src/test.rs"));
    }

    #[test]
    fn test_generate_merge_signal_basic() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let conflicting_files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];

        let result = generate_merge_signal(
            &session,
            &stage,
            "loom/stage-1",
            "main",
            &conflicting_files,
            &work_dir,
        );

        assert!(result.is_ok());
        let signal_path = result.unwrap();
        assert!(signal_path.exists());

        let content = fs::read_to_string(&signal_path).unwrap();
        assert!(content.contains("# Merge Signal: session-test-123"));
        assert!(content.contains("- **Session**: session-test-123"));
        assert!(content.contains("- **Stage**: stage-1"));
        assert!(content.contains("- **Source Branch**: loom/stage-1"));
        assert!(content.contains("- **Target Branch**: main"));
        assert!(content.contains("## Conflicting Files"));
        assert!(content.contains("- `src/main.rs`"));
        assert!(content.contains("- `src/lib.rs`"));
    }

    #[test]
    fn test_generate_merge_signal_empty_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();

        let result =
            generate_merge_signal(&session, &stage, "loom/stage-1", "main", &[], &work_dir);

        assert!(result.is_ok());
        let signal_path = result.unwrap();
        let content = fs::read_to_string(&signal_path).unwrap();

        assert!(content.contains("## Conflicting Files"));
        assert!(content.contains("_No specific files listed"));
    }

    #[test]
    fn test_format_merge_signal_content_sections() {
        let session = create_test_session();
        let stage = create_test_stage();
        let conflicting_files = vec!["src/test.rs".to_string()];

        let content = format_merge_signal_content(
            &session,
            &stage,
            "loom/stage-1",
            "main",
            &conflicting_files,
        );

        // Check all required sections are present
        assert!(content.contains("# Merge Signal:"));
        assert!(content.contains("## Merge Context"));
        assert!(content.contains("## Execution Rules"));
        assert!(content.contains("## Target"));
        assert!(content.contains("## Stage Context"));
        assert!(content.contains("## Conflicting Files"));
        assert!(content.contains("## Your Task"));
        assert!(content.contains("## Important"));

        // Check key instructions
        assert!(content.contains("git merge loom/stage-1"));
        assert!(content.contains("Resolve conflicts"));
        assert!(content.contains("git add"));
        assert!(content.contains("git commit"));
    }

    #[test]
    fn test_read_merge_signal() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let conflicting_files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];

        generate_merge_signal(
            &session,
            &stage,
            "loom/stage-1",
            "main",
            &conflicting_files,
            &work_dir,
        )
        .unwrap();

        let result = read_merge_signal("session-test-123", &work_dir);
        assert!(result.is_ok());

        let signal_content = result.unwrap();
        assert!(signal_content.is_some());

        let content = signal_content.unwrap();
        assert_eq!(content.session_id, "session-test-123");
        assert_eq!(content.stage_id, "stage-1");
        assert_eq!(content.source_branch, "loom/stage-1");
        assert_eq!(content.target_branch, "main");
        assert_eq!(content.conflicting_files.len(), 2);
        assert!(content
            .conflicting_files
            .contains(&"src/main.rs".to_string()));
        assert!(content
            .conflicting_files
            .contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn test_read_merge_signal_returns_none_for_regular_signal() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let session = create_test_session();
        let stage = create_test_stage();
        let worktree = create_test_worktree();

        // Generate a regular signal (not a merge signal)
        generate_signal(&session, &stage, &worktree, &[], None, None, &work_dir).unwrap();

        // read_merge_signal should return None for regular signals
        let result = read_merge_signal("session-test-123", &work_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_read_merge_signal_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join(".work");
        fs::create_dir_all(&work_dir).unwrap();

        let result = read_merge_signal("nonexistent-session", &work_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_merge_signal_content() {
        let content = r#"# Merge Signal: session-merge-123

## Merge Context

You are resolving a **merge conflict** in the main repository.

## Target

- **Session**: session-merge-123
- **Stage**: feature-stage
- **Source Branch**: loom/feature-stage
- **Target Branch**: develop

## Conflicting Files

- `src/app.rs`
- `src/config.rs`

## Your Task

1. Run: `git merge loom/feature-stage`
"#;

        let result = parse_merge_signal_content("session-merge-123", content);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.session_id, "session-merge-123");
        assert_eq!(parsed.stage_id, "feature-stage");
        assert_eq!(parsed.source_branch, "loom/feature-stage");
        assert_eq!(parsed.target_branch, "develop");
        assert_eq!(parsed.conflicting_files.len(), 2);
        assert_eq!(parsed.conflicting_files[0], "src/app.rs");
        assert_eq!(parsed.conflicting_files[1], "src/config.rs");
    }
}
