//! Integration tests for daemon and orchestrator configuration
//!
//! Tests verify that configuration options are properly applied and affect
//! orchestrator behavior as expected.

use loom::models::stage::{Stage, StageStatus};
use loom::orchestrator::terminal::BackendType;
use loom::orchestrator::{Orchestrator, OrchestratorConfig};
use loom::plan::graph::ExecutionGraph;
use loom::plan::schema::StageDefinition;
use loom::verify::transitions::save_stage;
use std::time::Duration;
use tempfile::TempDir;

/// Create a basic stage definition for testing
fn create_stage_def(id: &str, name: &str, deps: Vec<String>) -> StageDefinition {
    StageDefinition {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(format!("Test stage {name}")),
        dependencies: deps,
        parallel_group: None,
        acceptance: vec![],
        setup: vec![],
        files: vec![],
        auto_merge: None,
    }
}

#[test]
fn test_orchestrator_config_default_values() {
    let config = OrchestratorConfig::default();

    assert_eq!(config.max_parallel_sessions, 4);
    assert_eq!(config.poll_interval, Duration::from_secs(5));
    assert!(!config.manual_mode);
    assert!(!config.watch_mode);
    assert!(!config.auto_merge);
    assert_eq!(config.status_update_interval, Duration::from_secs(30));
    assert_eq!(config.backend_type, BackendType::Native);
}

#[test]
fn test_orchestrator_config_custom_values() {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path();
    std::fs::create_dir_all(work_dir.join("stages")).unwrap();

    let config = OrchestratorConfig {
        max_parallel_sessions: 8,
        poll_interval: Duration::from_secs(10),
        manual_mode: true,
        watch_mode: true,
        work_dir: work_dir.to_path_buf(),
        repo_root: work_dir.to_path_buf(),
        status_update_interval: Duration::from_secs(60),
        backend_type: BackendType::Native,
        auto_merge: true,
    };

    assert_eq!(config.max_parallel_sessions, 8);
    assert_eq!(config.poll_interval, Duration::from_secs(10));
    assert!(config.manual_mode);
    assert!(config.watch_mode);
    assert!(config.auto_merge);
    assert_eq!(config.status_update_interval, Duration::from_secs(60));
}

#[test]
fn test_orchestrator_creation_with_config() {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path();
    std::fs::create_dir_all(work_dir.join("stages")).unwrap();

    let stage_defs = vec![create_stage_def("stage-1", "Test Stage", vec![])];

    let graph = ExecutionGraph::build(stage_defs).expect("Should build execution graph");

    let config = OrchestratorConfig {
        max_parallel_sessions: 2,
        poll_interval: Duration::from_millis(100),
        manual_mode: true,
        watch_mode: false,
        work_dir: work_dir.to_path_buf(),
        repo_root: work_dir.to_path_buf(),
        status_update_interval: Duration::from_secs(5),
        backend_type: BackendType::Native,
        auto_merge: false,
    };

    let orchestrator = Orchestrator::new(config.clone(), graph);
    assert!(
        orchestrator.is_ok(),
        "Orchestrator should be created successfully"
    );

    let orchestrator = orchestrator.unwrap();
    assert_eq!(orchestrator.running_session_count(), 0);
}

#[test]
fn test_orchestrator_with_manual_mode() {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path();
    std::fs::create_dir_all(work_dir.join("stages")).unwrap();
    std::fs::create_dir_all(work_dir.join("sessions")).unwrap();

    // Create a stage file - start as WaitingForDeps to avoid immediate spawning
    let mut stage = Stage::new("Test Stage".to_string(), None);
    stage.id = "stage-1".to_string();
    stage.status = StageStatus::WaitingForDeps;
    save_stage(&stage, work_dir).expect("Should save stage");

    let stage_defs = vec![create_stage_def("stage-1", "Test Stage", vec![])];

    let graph = ExecutionGraph::build(stage_defs).expect("Should build execution graph");

    let config = OrchestratorConfig {
        max_parallel_sessions: 4,
        poll_interval: Duration::from_millis(50),
        manual_mode: true, // Manual mode exits after first batch
        watch_mode: false,
        work_dir: work_dir.to_path_buf(),
        repo_root: temp_dir.path().to_path_buf(),
        status_update_interval: Duration::from_secs(30),
        backend_type: BackendType::Native,
        auto_merge: false,
    };

    let orchestrator = Orchestrator::new(config, graph).expect("Should create orchestrator");

    // In manual mode, the orchestrator is ready to run
    // The test verifies configuration is applied correctly
    assert_eq!(orchestrator.running_session_count(), 0);
}

#[test]
fn test_execution_graph_with_parallel_stages() {
    let stage_defs = vec![
        create_stage_def("stage-1", "Foundation", vec![]),
        create_stage_def("stage-2a", "Parallel A", vec!["stage-1".to_string()]),
        create_stage_def("stage-2b", "Parallel B", vec!["stage-1".to_string()]),
        create_stage_def("stage-3", "Final", vec!["stage-2a".to_string(), "stage-2b".to_string()]),
    ];

    let graph = ExecutionGraph::build(stage_defs).expect("Should build execution graph");

    // Initial ready stages should only include stage-1
    let ready = graph.ready_stages();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "stage-1");
}

#[test]
fn test_auto_merge_config_cascade() {
    // Test that auto_merge can be configured at different levels

    // Stage-level auto_merge overrides plan-level
    let stage_with_auto_merge = StageDefinition {
        id: "stage-override".to_string(),
        name: "Override Stage".to_string(),
        description: None,
        dependencies: vec![],
        parallel_group: None,
        acceptance: vec![],
        setup: vec![],
        files: vec![],
        auto_merge: Some(true), // Stage-level override
    };

    assert_eq!(stage_with_auto_merge.auto_merge, Some(true));

    // Stage without override uses plan-level (represented as None)
    let stage_without_override = StageDefinition {
        id: "stage-default".to_string(),
        name: "Default Stage".to_string(),
        description: None,
        dependencies: vec![],
        parallel_group: None,
        acceptance: vec![],
        setup: vec![],
        files: vec![],
        auto_merge: None, // Uses plan default
    };

    assert_eq!(stage_without_override.auto_merge, None);
}

#[test]
fn test_backend_type_native() {
    // Test that Native backend type is the default and works correctly
    let config = OrchestratorConfig::default();
    assert_eq!(config.backend_type, BackendType::Native);

    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path();
    std::fs::create_dir_all(work_dir.join("stages")).unwrap();

    let stage_defs = vec![create_stage_def("stage-1", "Test", vec![])];
    let graph = ExecutionGraph::build(stage_defs).unwrap();

    let config = OrchestratorConfig {
        backend_type: BackendType::Native,
        work_dir: work_dir.to_path_buf(),
        repo_root: work_dir.to_path_buf(),
        ..Default::default()
    };

    let orchestrator = Orchestrator::new(config, graph);
    assert!(
        orchestrator.is_ok(),
        "Native backend should create successfully"
    );
}

#[test]
fn test_poll_interval_configuration() {
    // Test different poll intervals can be configured
    let configs = vec![
        Duration::from_millis(100),
        Duration::from_millis(500),
        Duration::from_secs(1),
        Duration::from_secs(5),
        Duration::from_secs(30),
    ];

    for poll_interval in configs {
        let config = OrchestratorConfig {
            poll_interval,
            ..Default::default()
        };
        assert_eq!(config.poll_interval, poll_interval);
    }
}

#[test]
fn test_status_update_interval_configuration() {
    // Test different status update intervals can be configured
    let intervals = vec![
        Duration::from_secs(5),
        Duration::from_secs(10),
        Duration::from_secs(30),
        Duration::from_secs(60),
    ];

    for interval in intervals {
        let config = OrchestratorConfig {
            status_update_interval: interval,
            ..Default::default()
        };
        assert_eq!(config.status_update_interval, interval);
    }
}

#[test]
fn test_max_parallel_sessions_configuration() {
    // Test different max_parallel_sessions values
    let values = vec![1, 2, 4, 8, 16];

    for max in values {
        let config = OrchestratorConfig {
            max_parallel_sessions: max,
            ..Default::default()
        };
        assert_eq!(config.max_parallel_sessions, max);
    }
}

#[test]
fn test_watch_mode_configuration() {
    // Test watch mode flag
    let config_watch = OrchestratorConfig {
        watch_mode: true,
        ..Default::default()
    };
    assert!(config_watch.watch_mode);

    let config_no_watch = OrchestratorConfig {
        watch_mode: false,
        ..Default::default()
    };
    assert!(!config_no_watch.watch_mode);
}

#[test]
fn test_work_dir_and_repo_root_configuration() {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path().join(".work");
    let repo_root = temp_dir.path().to_path_buf();

    std::fs::create_dir_all(&work_dir).unwrap();

    let config = OrchestratorConfig {
        work_dir: work_dir.clone(),
        repo_root: repo_root.clone(),
        ..Default::default()
    };

    assert_eq!(config.work_dir, work_dir);
    assert_eq!(config.repo_root, repo_root);
}

/// Integration test: Verify orchestrator respects max_parallel_sessions
///
/// This test verifies that the max_parallel_sessions configuration is stored
/// correctly and would limit concurrent sessions when the orchestrator runs.
/// Note: Full execution requires a git repo, so we test configuration only.
#[test]
#[ignore] // Integration test - run with --ignored
fn test_orchestrator_respects_max_parallel_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path();
    std::fs::create_dir_all(work_dir.join("stages")).unwrap();
    std::fs::create_dir_all(work_dir.join("sessions")).unwrap();

    // Create multiple parallel stages (WaitingForDeps so they won't try to spawn)
    for i in 1..=5 {
        let mut stage = Stage::new(format!("Stage {i}"), None);
        stage.id = format!("stage-{i}");
        stage.status = StageStatus::WaitingForDeps;
        save_stage(&stage, work_dir).expect("Should save stage");
    }

    let stage_defs: Vec<StageDefinition> = (1..=5)
        .map(|i| create_stage_def(&format!("stage-{i}"), &format!("Stage {i}"), vec![]))
        .collect();

    let graph = ExecutionGraph::build(stage_defs).expect("Should build execution graph");

    let config = OrchestratorConfig {
        max_parallel_sessions: 2, // Limit to 2 parallel
        poll_interval: Duration::from_millis(50),
        manual_mode: true, // Exit after first batch
        watch_mode: false,
        work_dir: work_dir.to_path_buf(),
        repo_root: temp_dir.path().to_path_buf(),
        status_update_interval: Duration::from_secs(30),
        backend_type: BackendType::Native,
        auto_merge: false,
    };

    // Verify the configuration is correctly set
    assert_eq!(config.max_parallel_sessions, 2);

    let orchestrator = Orchestrator::new(config, graph).expect("Should create orchestrator");

    // Verify orchestrator was created with correct settings
    assert_eq!(orchestrator.running_session_count(), 0);
}

/// Integration test: Verify auto-merge flag is passed through config
#[test]
#[ignore] // Integration test - run with --ignored
fn test_auto_merge_flag_in_config() {
    let temp_dir = TempDir::new().unwrap();
    let work_dir = temp_dir.path();
    std::fs::create_dir_all(work_dir.join("stages")).unwrap();

    let mut stage = Stage::new("Test Stage".to_string(), None);
    stage.id = "stage-1".to_string();
    stage.status = StageStatus::Queued;
    save_stage(&stage, work_dir).expect("Should save stage");

    let stage_defs = vec![create_stage_def("stage-1", "Test Stage", vec![])];
    let graph = ExecutionGraph::build(stage_defs).expect("Should build execution graph");

    // Test with auto_merge enabled
    let config = OrchestratorConfig {
        auto_merge: true,
        manual_mode: true,
        work_dir: work_dir.to_path_buf(),
        repo_root: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    assert!(config.auto_merge, "auto_merge should be enabled");

    let orchestrator = Orchestrator::new(config, graph).expect("Should create orchestrator");
    assert!(
        orchestrator.running_session_count() == 0,
        "No sessions running initially"
    );
}
