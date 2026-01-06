//! Core orchestrator for coordinating stage execution
//!
//! The orchestrator is the heart of `flux run`. It:
//! - Creates worktrees for ready stages
//! - Spawns Claude sessions in tmux
//! - Monitors stage completion and session health
//! - Handles crashes and context exhaustion
//! - Manages the execution graph

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::git;
use crate::models::session::Session;
use crate::models::stage::{Stage, StageStatus};
use crate::models::worktree::Worktree;
use crate::orchestrator::monitor::{Monitor, MonitorConfig, MonitorEvent};
use crate::orchestrator::signals::{generate_signal, remove_signal, DependencyStatus};
use crate::orchestrator::spawner::{
    check_tmux_available, kill_session, spawn_session, SpawnerConfig,
};
use crate::plan::ExecutionGraph;

/// Configuration for the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_parallel_sessions: usize,
    pub poll_interval: Duration,
    pub manual_mode: bool,
    pub tmux_prefix: String,
    pub work_dir: PathBuf,
    pub repo_root: PathBuf,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_parallel_sessions: 4,
            poll_interval: Duration::from_secs(5),
            manual_mode: false,
            tmux_prefix: "flux".to_string(),
            work_dir: PathBuf::from(".work"),
            repo_root: PathBuf::from("."),
        }
    }
}

/// Main orchestrator coordinating stage execution
pub struct Orchestrator {
    config: OrchestratorConfig,
    graph: ExecutionGraph,
    active_sessions: HashMap<String, Session>,
    active_worktrees: HashMap<String, Worktree>,
    monitor: Monitor,
}

impl Orchestrator {
    /// Create a new orchestrator from config and execution graph
    pub fn new(config: OrchestratorConfig, graph: ExecutionGraph) -> Self {
        let monitor_config = MonitorConfig {
            poll_interval: config.poll_interval,
            work_dir: config.work_dir.clone(),
            ..Default::default()
        };

        let monitor = Monitor::new(monitor_config);

        Self {
            config,
            graph,
            active_sessions: HashMap::new(),
            active_worktrees: HashMap::new(),
            monitor,
        }
    }

    /// Main run loop - executes until all stages complete or error
    pub fn run(&mut self) -> Result<OrchestratorResult> {
        if !self.config.manual_mode {
            check_tmux_available()
                .context("tmux is required for automatic session spawning. Use --manual to set up sessions yourself.")?;
        }

        let mut total_sessions_spawned = 0;
        let mut completed_stages = Vec::new();
        let mut failed_stages = Vec::new();
        let mut needs_handoff = Vec::new();

        loop {
            let started = self
                .start_ready_stages()
                .context("Failed to start ready stages")?;
            total_sessions_spawned += started;

            if !self.config.manual_mode {
                let events = self
                    .monitor
                    .poll()
                    .context("Failed to poll monitor for events")?;

                self.handle_events(events)
                    .context("Failed to handle monitor events")?;

                for stage_id in self.active_sessions.keys() {
                    if let Ok(stage) = self.load_stage(stage_id) {
                        match stage.status {
                            StageStatus::Completed => {
                                if !completed_stages.contains(&stage_id.clone()) {
                                    completed_stages.push(stage_id.clone());
                                }
                            }
                            StageStatus::Blocked => {
                                if !failed_stages.contains(&stage_id.clone()) {
                                    failed_stages.push(stage_id.clone());
                                }
                            }
                            StageStatus::NeedsHandoff => {
                                if !needs_handoff.contains(&stage_id.clone()) {
                                    needs_handoff.push(stage_id.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if self.graph.is_complete() {
                break;
            }

            if !failed_stages.is_empty() && self.running_session_count() == 0 {
                break;
            }

            if self.config.manual_mode {
                break;
            }

            std::thread::sleep(self.config.poll_interval);
        }

        Ok(OrchestratorResult {
            completed_stages,
            failed_stages,
            needs_handoff,
            total_sessions_spawned,
        })
    }

    /// Run a single stage by ID (for `flux run --stage <id>`)
    pub fn run_single(&mut self, stage_id: &str) -> Result<OrchestratorResult> {
        let node = self
            .graph
            .get_node(stage_id)
            .ok_or_else(|| anyhow::anyhow!("Stage not found: {stage_id}"))?;

        if node.status != crate::plan::NodeStatus::Ready {
            bail!(
                "Stage '{}' is not ready for execution. Current status: {:?}",
                stage_id,
                node.status
            );
        }

        self.start_stage(stage_id)
            .context("Failed to start stage")?;

        if self.config.manual_mode {
            return Ok(OrchestratorResult {
                completed_stages: Vec::new(),
                failed_stages: Vec::new(),
                needs_handoff: Vec::new(),
                total_sessions_spawned: 1,
            });
        }

        check_tmux_available().context("tmux is required for single stage execution")?;

        let mut completed = false;
        let mut failed = false;
        let mut needs_handoff = false;

        loop {
            let events = self.monitor.poll().context("Failed to poll monitor")?;

            for event in events {
                match event {
                    MonitorEvent::StageCompleted { stage_id: sid } if sid == stage_id => {
                        completed = true;
                    }
                    MonitorEvent::StageBlocked { stage_id: sid, .. } if sid == stage_id => {
                        failed = true;
                    }
                    MonitorEvent::SessionNeedsHandoff { stage_id: sid, .. } if sid == stage_id => {
                        needs_handoff = true;
                    }
                    MonitorEvent::SessionCrashed {
                        stage_id: Some(sid),
                        ..
                    } if sid == stage_id => {
                        failed = true;
                    }
                    _ => {}
                }
            }

            if completed || failed || needs_handoff {
                break;
            }

            std::thread::sleep(self.config.poll_interval);
        }

        Ok(OrchestratorResult {
            completed_stages: if completed {
                vec![stage_id.to_string()]
            } else {
                Vec::new()
            },
            failed_stages: if failed {
                vec![stage_id.to_string()]
            } else {
                Vec::new()
            },
            needs_handoff: if needs_handoff {
                vec![stage_id.to_string()]
            } else {
                Vec::new()
            },
            total_sessions_spawned: 1,
        })
    }

    /// Start ready stages (create worktrees, spawn sessions)
    fn start_ready_stages(&mut self) -> Result<usize> {
        let ready_stages = self.graph.ready_stages();
        let available_slots = self
            .config
            .max_parallel_sessions
            .saturating_sub(self.running_session_count());
        let mut started = 0;

        // Collect stage IDs first to avoid borrow checker issues
        let stage_ids: Vec<String> = ready_stages
            .iter()
            .take(available_slots)
            .map(|node| node.id.clone())
            .collect();

        for stage_id in stage_ids {
            self.start_stage(&stage_id)
                .with_context(|| format!("Failed to start stage: {stage_id}"))?;
            started += 1;
        }

        Ok(started)
    }

    /// Process a single ready stage
    fn start_stage(&mut self, stage_id: &str) -> Result<()> {
        let stage = self.load_stage(stage_id)?;

        let worktree = git::create_worktree(stage_id, &self.config.repo_root)
            .with_context(|| format!("Failed to create worktree for stage: {stage_id}"))?;

        let session = Session::new();

        let deps = self.get_dependency_status(&stage);

        generate_signal(
            &session,
            &stage,
            &worktree,
            &deps,
            None,
            &self.config.work_dir,
        )
        .context("Failed to generate signal file")?;

        let spawned_session = if !self.config.manual_mode {
            let spawner_config = SpawnerConfig {
                max_parallel_sessions: self.config.max_parallel_sessions,
                tmux_prefix: self.config.tmux_prefix.clone(),
            };

            spawn_session(&stage, &worktree, &spawner_config)
                .with_context(|| format!("Failed to spawn session for stage: {stage_id}"))?
        } else {
            println!("Manual mode: Session setup for stage '{stage_id}'");
            println!("  Worktree: {}", worktree.path.display());
            println!("  Signal: .work/signals/{}.md", session.id);
            println!("  To start: cd {} && claude", worktree.path.display());
            session
        };

        self.save_session(&spawned_session)?;

        let mut updated_stage = stage;
        updated_stage.assign_session(spawned_session.id.clone());
        updated_stage.set_worktree(Some(worktree.id.clone()));
        updated_stage.mark_executing();
        self.save_stage(&updated_stage)?;

        self.graph
            .mark_executing(stage_id)
            .context("Failed to mark stage as executing in graph")?;

        self.active_sessions
            .insert(stage_id.to_string(), spawned_session);
        self.active_worktrees.insert(stage_id.to_string(), worktree);

        Ok(())
    }

    /// Handle monitor events
    fn handle_events(&mut self, events: Vec<MonitorEvent>) -> Result<()> {
        for event in events {
            match event {
                MonitorEvent::StageCompleted { stage_id } => {
                    self.on_stage_completed(&stage_id)?;
                }
                MonitorEvent::StageBlocked { stage_id, reason } => {
                    eprintln!("Stage '{stage_id}' blocked: {reason}");
                    self.graph.mark_blocked(&stage_id)?;
                }
                MonitorEvent::SessionContextWarning {
                    session_id,
                    usage_percent,
                } => {
                    eprintln!(
                        "Warning: Session '{session_id}' context at {usage_percent:.1}%"
                    );
                }
                MonitorEvent::SessionContextCritical {
                    session_id,
                    usage_percent,
                } => {
                    eprintln!(
                        "Critical: Session '{session_id}' context at {usage_percent:.1}%"
                    );
                }
                MonitorEvent::SessionCrashed {
                    session_id,
                    stage_id,
                } => {
                    self.on_session_crashed(&session_id, stage_id)?;
                }
                MonitorEvent::SessionNeedsHandoff {
                    session_id,
                    stage_id,
                } => {
                    self.on_needs_handoff(&session_id, &stage_id)?;
                }
            }
        }
        Ok(())
    }

    /// Handle stage completion
    fn on_stage_completed(&mut self, stage_id: &str) -> Result<()> {
        self.graph.mark_completed(stage_id)?;

        if let Some(session) = self.active_sessions.remove(stage_id) {
            remove_signal(&session.id, &self.config.work_dir)?;
            let _ = kill_session(&session);
        }

        self.active_worktrees.remove(stage_id);

        Ok(())
    }

    /// Handle session crash
    fn on_session_crashed(&mut self, session_id: &str, stage_id: Option<String>) -> Result<()> {
        eprintln!("Session '{session_id}' crashed");

        if let Some(sid) = stage_id {
            self.active_sessions.remove(&sid);

            let mut stage = self.load_stage(&sid)?;
            stage.status = StageStatus::Blocked;
            stage.close_reason = Some("Session crashed".to_string());
            self.save_stage(&stage)?;

            self.graph.mark_blocked(&sid)?;
        }

        Ok(())
    }

    /// Handle context exhaustion (needs handoff)
    fn on_needs_handoff(&mut self, session_id: &str, stage_id: &str) -> Result<()> {
        eprintln!("Session '{session_id}' needs handoff for stage '{stage_id}'");

        let mut stage = self.load_stage(stage_id)?;
        stage.mark_needs_handoff();
        self.save_stage(&stage)?;

        Ok(())
    }

    /// Load stage definition from .work/stages/
    fn load_stage(&self, stage_id: &str) -> Result<Stage> {
        let stage_path = self
            .config
            .work_dir
            .join("stages")
            .join(format!("{stage_id}.md"));

        if !stage_path.exists() {
            let node = self
                .graph
                .get_node(stage_id)
                .ok_or_else(|| anyhow::anyhow!("Stage not found in graph: {stage_id}"))?;

            let mut stage = Stage::new(node.name.clone(), None);
            stage.id = stage_id.to_string();
            stage.dependencies = node.dependencies.clone();
            stage.parallel_group = node.parallel_group.clone();

            return Ok(stage);
        }

        let content = std::fs::read_to_string(&stage_path)
            .with_context(|| format!("Failed to read stage file: {}", stage_path.display()))?;

        let frontmatter = extract_yaml_frontmatter(&content)?;
        let stage: Stage = serde_yaml::from_value(frontmatter)
            .context("Failed to deserialize Stage from frontmatter")?;

        Ok(stage)
    }

    /// Save stage state to .work/stages/
    fn save_stage(&self, stage: &Stage) -> Result<()> {
        let stages_dir = self.config.work_dir.join("stages");
        if !stages_dir.exists() {
            std::fs::create_dir_all(&stages_dir).context("Failed to create stages directory")?;
        }

        let stage_path = stages_dir.join(format!("{}.md", stage.id));

        let yaml = serde_yaml::to_string(stage).context("Failed to serialize stage to YAML")?;

        let content = format!(
            "---\n{}---\n\n# Stage: {}\n\n{}\n",
            yaml,
            stage.name,
            stage
                .description
                .as_deref()
                .unwrap_or("No description provided.")
        );

        std::fs::write(&stage_path, content)
            .with_context(|| format!("Failed to write stage file: {}", stage_path.display()))?;

        Ok(())
    }

    /// Save session state to .work/sessions/
    fn save_session(&self, session: &Session) -> Result<()> {
        let sessions_dir = self.config.work_dir.join("sessions");
        if !sessions_dir.exists() {
            std::fs::create_dir_all(&sessions_dir)
                .context("Failed to create sessions directory")?;
        }

        let session_path = sessions_dir.join(format!("{}.md", session.id));

        let yaml = serde_yaml::to_string(session).context("Failed to serialize session to YAML")?;

        let content = format!(
            "---\n{}---\n\n# Session: {}\n\nStatus: {:?}\n",
            yaml, session.id, session.status
        );

        std::fs::write(&session_path, content)
            .with_context(|| format!("Failed to write session file: {}", session_path.display()))?;

        Ok(())
    }

    /// Get dependency status for signal generation
    fn get_dependency_status(&self, stage: &Stage) -> Vec<DependencyStatus> {
        stage
            .dependencies
            .iter()
            .map(|dep_id| {
                let status = if let Some(node) = self.graph.get_node(dep_id) {
                    format!("{:?}", node.status)
                } else {
                    "Unknown".to_string()
                };

                DependencyStatus {
                    stage_id: dep_id.clone(),
                    name: dep_id.clone(),
                    status,
                }
            })
            .collect()
    }

    /// Count currently running sessions
    fn running_session_count(&self) -> usize {
        self.active_sessions.len()
    }
}

/// Result of orchestrator run
#[derive(Debug)]
pub struct OrchestratorResult {
    pub completed_stages: Vec<String>,
    pub failed_stages: Vec<String>,
    pub needs_handoff: Vec<String>,
    pub total_sessions_spawned: usize,
}

impl OrchestratorResult {
    pub fn is_success(&self) -> bool {
        self.failed_stages.is_empty() && self.needs_handoff.is_empty()
    }
}

/// Extract YAML frontmatter from markdown content
fn extract_yaml_frontmatter(content: &str) -> Result<serde_yaml::Value> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || !lines[0].trim().starts_with("---") {
        bail!("No frontmatter delimiter found");
    }

    let mut end_idx = None;
    for (idx, line) in lines.iter().enumerate().skip(1) {
        if line.trim().starts_with("---") {
            end_idx = Some(idx);
            break;
        }
    }

    let end_idx = end_idx.ok_or_else(|| anyhow::anyhow!("Frontmatter not properly closed"))?;

    let yaml_content = lines[1..end_idx].join("\n");

    serde_yaml::from_str(&yaml_content).context("Failed to parse YAML frontmatter")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::schema::StageDefinition;

    fn create_test_config() -> OrchestratorConfig {
        OrchestratorConfig {
            max_parallel_sessions: 2,
            poll_interval: Duration::from_millis(100),
            manual_mode: true,
            tmux_prefix: "test".to_string(),
            work_dir: PathBuf::from("/tmp/test-work"),
            repo_root: PathBuf::from("/tmp/test-repo"),
        }
    }

    fn create_simple_graph() -> ExecutionGraph {
        let stages = vec![StageDefinition {
            id: "stage-1".to_string(),
            name: "Stage 1".to_string(),
            description: None,
            dependencies: vec![],
            parallel_group: None,
            acceptance: vec![],
            files: vec![],
        }];

        ExecutionGraph::build(stages).unwrap()
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_parallel_sessions, 4);
        assert_eq!(config.poll_interval, Duration::from_secs(5));
        assert!(!config.manual_mode);
        assert_eq!(config.tmux_prefix, "flux");
    }

    #[test]
    fn test_orchestrator_result_success() {
        let result = OrchestratorResult {
            completed_stages: vec!["stage-1".to_string()],
            failed_stages: vec![],
            needs_handoff: vec![],
            total_sessions_spawned: 1,
        };

        assert!(result.is_success());
    }

    #[test]
    fn test_orchestrator_result_failure() {
        let result = OrchestratorResult {
            completed_stages: vec![],
            failed_stages: vec!["stage-1".to_string()],
            needs_handoff: vec![],
            total_sessions_spawned: 1,
        };

        assert!(!result.is_success());
    }

    #[test]
    fn test_orchestrator_result_needs_handoff() {
        let result = OrchestratorResult {
            completed_stages: vec![],
            failed_stages: vec![],
            needs_handoff: vec!["stage-1".to_string()],
            total_sessions_spawned: 1,
        };

        assert!(!result.is_success());
    }

    #[test]
    fn test_running_session_count() {
        let config = create_test_config();
        let graph = create_simple_graph();
        let orchestrator = Orchestrator::new(config, graph);

        assert_eq!(orchestrator.running_session_count(), 0);
    }

    #[test]
    fn test_extract_yaml_frontmatter() {
        let content = r#"---
id: stage-1
name: Test Stage
status: Pending
---

# Stage Details
Test content
"#;

        let result = extract_yaml_frontmatter(content);
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value.get("id").is_some());
        assert!(value.get("name").is_some());
    }

    #[test]
    fn test_extract_yaml_frontmatter_no_delimiter() {
        let content = "No frontmatter here";
        let result = extract_yaml_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_yaml_frontmatter_not_closed() {
        let content = r#"---
id: stage-1
name: Test Stage
"#;
        let result = extract_yaml_frontmatter(content);
        assert!(result.is_err());
    }
}
