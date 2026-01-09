//! Acceptance Criteria Execution Module
//!
//! This module executes shell commands defined as acceptance criteria in loom plans.
//!
//! # Trust Model
//!
//! Plan files (containing acceptance criteria and setup commands) follow the same trust
//! model as Makefiles, shell scripts, or CI/CD configuration files. They are considered
//! trusted project artifacts that are:
//!
//! - Version controlled alongside application code
//! - Reviewed as part of the normal code review process
//! - Authored by project maintainers or approved contributors
//!
//! Users should treat plan files with the same caution as any executable script:
//! do not run plans from untrusted sources without reviewing their contents.
//!
//! # Security Controls
//!
//! While plan files are trusted, this module implements the following controls to
//! limit the impact of runaway or misbehaving commands:
//!
//! - **Command Timeout**: All commands have a configurable timeout (default 5 minutes)
//!   to prevent indefinite hangs from blocking the orchestration pipeline.
//!
//! - **Explicit Shell Invocation**: Commands are executed via `sh -c` (Unix) or
//!   `cmd /C` (Windows) with the command passed as a single argument, avoiding
//!   shell injection through improper argument splitting.
//!
//! - **Isolated Working Directory**: Commands can be scoped to a specific worktree
//!   directory, limiting their filesystem context.
//!
//! # Timeout Behavior
//!
//! When a command exceeds its timeout:
//! - The process is terminated (SIGKILL on Unix, TerminateProcess on Windows)
//! - The criterion is marked as failed with a timeout-specific error message
//! - Subsequent criteria continue to execute (fail-fast is not the default)

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

use super::context::CriteriaContext;
use crate::models::stage::Stage;

/// Default timeout for command execution (5 minutes)
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

/// Configuration for acceptance criteria execution
#[derive(Debug, Clone)]
pub struct CriteriaConfig {
    /// Maximum time to wait for a single command to complete
    pub command_timeout: Duration,
}

impl Default for CriteriaConfig {
    fn default() -> Self {
        Self {
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
        }
    }
}

impl CriteriaConfig {
    /// Create a new configuration with a custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            command_timeout: timeout,
        }
    }
}

/// Result of executing a single acceptance criterion (shell command)
#[derive(Debug, Clone)]
pub struct CriterionResult {
    pub command: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    /// Whether the command was terminated due to timeout
    pub timed_out: bool,
}

impl CriterionResult {
    /// Create a new criterion result
    pub fn new(
        command: String,
        success: bool,
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
        duration: Duration,
        timed_out: bool,
    ) -> Self {
        Self {
            command,
            success,
            stdout,
            stderr,
            exit_code,
            duration,
            timed_out,
        }
    }

    /// Check if the criterion passed
    pub fn passed(&self) -> bool {
        self.success
    }

    /// Get a summary of the result
    pub fn summary(&self) -> String {
        let status = if self.timed_out {
            "TIMEOUT"
        } else if self.success {
            "PASSED"
        } else {
            "FAILED"
        };
        let duration_ms = self.duration.as_millis();
        format!(
            "{} - {} ({}ms, exit code: {:?})",
            status, self.command, duration_ms, self.exit_code
        )
    }
}

/// Result of running all acceptance criteria for a stage
#[derive(Debug)]
pub enum AcceptanceResult {
    /// All acceptance criteria passed
    AllPassed { results: Vec<CriterionResult> },
    /// One or more acceptance criteria failed
    Failed {
        results: Vec<CriterionResult>,
        failures: Vec<String>,
    },
}

impl AcceptanceResult {
    /// Check if all criteria passed
    pub fn all_passed(&self) -> bool {
        matches!(self, AcceptanceResult::AllPassed { .. })
    }

    /// Get all criterion results
    pub fn results(&self) -> &[CriterionResult] {
        match self {
            AcceptanceResult::AllPassed { results } => results,
            AcceptanceResult::Failed { results, .. } => results,
        }
    }

    /// Get failure messages if any
    pub fn failures(&self) -> Vec<String> {
        match self {
            AcceptanceResult::AllPassed { .. } => Vec::new(),
            AcceptanceResult::Failed { failures, .. } => failures.clone(),
        }
    }

    /// Get total duration of all criteria
    pub fn total_duration(&self) -> Duration {
        self.results().iter().map(|r| r.duration).sum()
    }

    /// Get count of passed criteria
    pub fn passed_count(&self) -> usize {
        self.results().iter().filter(|r| r.passed()).count()
    }

    /// Get count of failed criteria
    pub fn failed_count(&self) -> usize {
        self.results().iter().filter(|r| !r.passed()).count()
    }
}

/// Run all acceptance criteria for a stage with default configuration
///
/// This is a convenience wrapper around `run_acceptance_with_config` that uses
/// the default timeout settings.
pub fn run_acceptance(stage: &Stage, working_dir: Option<&Path>) -> Result<AcceptanceResult> {
    run_acceptance_with_config(stage, working_dir, &CriteriaConfig::default())
}

/// Run all acceptance criteria for a stage with custom configuration
///
/// Executes each shell command sequentially and collects results.
/// Returns AllPassed if all commands exit with code 0, Failed otherwise.
///
/// If `working_dir` is provided, commands will be executed in that directory.
/// This is typically used to run criteria in a worktree directory.
///
/// Context variables (like `${PROJECT_ROOT}`, `${WORKTREE}`) in criteria
/// are automatically expanded before execution.
///
/// If the stage has setup commands defined, they will be prepended to each
/// criterion command using `&&` to ensure environment preparation runs first.
///
/// Each command is subject to the timeout specified in `config`. Commands that
/// exceed the timeout are terminated and marked as failed.
pub fn run_acceptance_with_config(
    stage: &Stage,
    working_dir: Option<&Path>,
    config: &CriteriaConfig,
) -> Result<AcceptanceResult> {
    if stage.acceptance.is_empty() {
        return Ok(AcceptanceResult::AllPassed {
            results: Vec::new(),
        });
    }

    // Build context for variable expansion
    let default_dir = PathBuf::from(".");
    let ctx_path = working_dir.unwrap_or(&default_dir);
    let context = CriteriaContext::with_stage_id(ctx_path, &stage.id);

    let mut results = Vec::new();
    let mut failures = Vec::new();

    // Build setup prefix if setup commands are defined (also expand variables in setup)
    let setup_prefix = if stage.setup.is_empty() {
        None
    } else {
        let expanded_setup: Vec<String> = stage.setup.iter().map(|s| context.expand(s)).collect();
        Some(expanded_setup.join(" && "))
    };

    for command in &stage.acceptance {
        // Expand context variables in the command
        let expanded_command = context.expand(command);

        // Combine setup commands with criterion if setup is defined
        let full_command = match &setup_prefix {
            Some(prefix) => format!("{prefix} && {expanded_command}"),
            None => expanded_command,
        };

        let result =
            run_single_criterion_with_timeout(&full_command, working_dir, config.command_timeout)
                .with_context(|| format!("Failed to execute criterion: {command}"))?;

        if !result.success {
            let failure_reason = if result.timed_out {
                format!(
                    "Command '{}' timed out after {}s",
                    command,
                    config.command_timeout.as_secs()
                )
            } else {
                format!(
                    "Command '{}' failed with exit code {:?}",
                    command, result.exit_code
                )
            };
            failures.push(failure_reason);
        }

        // Store result with original command for cleaner output
        let mut result_with_original = result;
        result_with_original.command = command.clone();
        results.push(result_with_original);
    }

    if failures.is_empty() {
        Ok(AcceptanceResult::AllPassed { results })
    } else {
        Ok(AcceptanceResult::Failed { results, failures })
    }
}

/// Run a single acceptance criterion (shell command) with default timeout
///
/// This is a convenience wrapper around `run_single_criterion_with_timeout` that uses
/// the default timeout setting.
pub fn run_single_criterion(command: &str, working_dir: Option<&Path>) -> Result<CriterionResult> {
    run_single_criterion_with_timeout(command, working_dir, DEFAULT_COMMAND_TIMEOUT)
}

/// Run a single acceptance criterion (shell command) with specified timeout
///
/// Executes the command using the system shell and captures all output.
/// Returns a CriterionResult with execution details.
///
/// If `working_dir` is provided, the command will be executed in that directory.
///
/// The command will be terminated if it exceeds the specified `timeout` duration.
/// When this happens, the result will have `timed_out` set to true and `success`
/// set to false.
pub fn run_single_criterion_with_timeout(
    command: &str,
    working_dir: Option<&Path>,
    timeout: Duration,
) -> Result<CriterionResult> {
    let start = Instant::now();

    // Spawn the child process using the appropriate shell
    let mut child = spawn_shell_command(command, working_dir)?;

    // Wait for completion with timeout
    let wait_result = child
        .wait_timeout(timeout)
        .with_context(|| format!("Failed to wait for command: {command}"))?;

    let duration = start.elapsed();

    match wait_result {
        Some(status) => {
            // Command completed within timeout
            let (stdout, stderr) = collect_child_output(&mut child)?;
            let success = status.success();
            let exit_code = status.code();

            Ok(CriterionResult::new(
                command.to_string(),
                success,
                stdout,
                stderr,
                exit_code,
                duration,
                false, // not timed out
            ))
        }
        None => {
            // Command timed out - kill the process
            kill_child_process(&mut child);

            // Collect any partial output that was captured
            let (stdout, stderr) = collect_child_output(&mut child).unwrap_or_default();

            Ok(CriterionResult::new(
                command.to_string(),
                false, // failed due to timeout
                stdout,
                format!(
                    "{}\n[Process killed after {}s timeout]",
                    stderr,
                    timeout.as_secs()
                ),
                None, // no exit code for killed process
                duration,
                true, // timed out
            ))
        }
    }
}

/// Spawn a shell command as a child process
///
/// Uses `sh -c` on Unix and `cmd /C` on Windows to execute the command.
/// The command string is passed as a single argument to avoid shell injection
/// through improper argument splitting.
fn spawn_shell_command(command: &str, working_dir: Option<&Path>) -> Result<Child> {
    let mut cmd = if cfg!(target_family = "unix") {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    } else {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(command);
        c
    };

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    cmd.spawn()
        .with_context(|| format!("Failed to spawn command: {command}"))
}

/// Collect stdout and stderr from a child process
fn collect_child_output(child: &mut Child) -> Result<(String, String)> {
    let stdout = child
        .stdout
        .take()
        .map(|mut s| {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut buf).ok();
            String::from_utf8_lossy(&buf).to_string()
        })
        .unwrap_or_default();

    let stderr = child
        .stderr
        .take()
        .map(|mut s| {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut buf).ok();
            String::from_utf8_lossy(&buf).to_string()
        })
        .unwrap_or_default();

    Ok((stdout, stderr))
}

/// Terminate a child process
///
/// Attempts to kill the process. On Unix, this sends SIGKILL.
/// On Windows, this calls TerminateProcess.
fn kill_child_process(child: &mut Child) {
    // Attempt to kill - ignore errors since the process may have already exited
    let _ = child.kill();
    // Wait to reap the zombie process
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criterion_result_new() {
        let result = CriterionResult::new(
            "echo test".to_string(),
            true,
            "test\n".to_string(),
            String::new(),
            Some(0),
            Duration::from_millis(100),
            false,
        );

        assert!(result.passed());
        assert_eq!(result.command, "echo test");
        assert_eq!(result.stdout, "test\n");
        assert_eq!(result.stderr, "");
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.duration, Duration::from_millis(100));
        assert!(!result.timed_out);
    }

    #[test]
    fn test_criterion_result_summary() {
        let result = CriterionResult::new(
            "cargo test".to_string(),
            false,
            String::new(),
            "error".to_string(),
            Some(1),
            Duration::from_millis(500),
            false,
        );

        let summary = result.summary();
        assert!(summary.contains("FAILED"));
        assert!(summary.contains("cargo test"));
        assert!(summary.contains("500ms"));
        assert!(summary.contains("exit code: Some(1)"));
    }

    #[test]
    fn test_criterion_result_summary_timeout() {
        let result = CriterionResult::new(
            "sleep 1000".to_string(),
            false,
            String::new(),
            "[Process killed after 5s timeout]".to_string(),
            None,
            Duration::from_secs(5),
            true,
        );

        let summary = result.summary();
        assert!(summary.contains("TIMEOUT"));
        assert!(summary.contains("sleep 1000"));
    }

    #[test]
    fn test_acceptance_result_all_passed() {
        let results = vec![
            CriterionResult::new(
                "test1".to_string(),
                true,
                "ok".to_string(),
                String::new(),
                Some(0),
                Duration::from_millis(100),
                false,
            ),
            CriterionResult::new(
                "test2".to_string(),
                true,
                "ok".to_string(),
                String::new(),
                Some(0),
                Duration::from_millis(200),
                false,
            ),
        ];

        let acceptance = AcceptanceResult::AllPassed {
            results: results.clone(),
        };

        assert!(acceptance.all_passed());
        assert_eq!(acceptance.passed_count(), 2);
        assert_eq!(acceptance.failed_count(), 0);
        assert_eq!(acceptance.failures().len(), 0);
        assert_eq!(acceptance.total_duration(), Duration::from_millis(300));
    }

    #[test]
    fn test_acceptance_result_failed() {
        let results = vec![
            CriterionResult::new(
                "test1".to_string(),
                true,
                "ok".to_string(),
                String::new(),
                Some(0),
                Duration::from_millis(100),
                false,
            ),
            CriterionResult::new(
                "test2".to_string(),
                false,
                String::new(),
                "error".to_string(),
                Some(1),
                Duration::from_millis(200),
                false,
            ),
        ];

        let failures = vec!["test2 failed".to_string()];
        let acceptance = AcceptanceResult::Failed {
            results: results.clone(),
            failures,
        };

        assert!(!acceptance.all_passed());
        assert_eq!(acceptance.passed_count(), 1);
        assert_eq!(acceptance.failed_count(), 1);
        assert_eq!(acceptance.failures().len(), 1);
        assert_eq!(acceptance.total_duration(), Duration::from_millis(300));
    }

    #[test]
    fn test_run_single_criterion_success() {
        let command = if cfg!(target_family = "unix") {
            "echo 'hello world'"
        } else {
            "echo hello world"
        };

        let result = run_single_criterion(command, None).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("hello world"));
        assert!(result.duration > Duration::from_nanos(0));
        assert!(!result.timed_out);
    }

    #[test]
    fn test_run_single_criterion_failure() {
        let command = if cfg!(target_family = "unix") {
            "exit 42"
        } else {
            "exit /b 42"
        };

        let result = run_single_criterion(command, None).unwrap();

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(42));
        assert!(!result.timed_out);
    }

    #[test]
    fn test_run_single_criterion_timeout() {
        // Only run on Unix - sleep command behavior differs on Windows
        if cfg!(target_family = "unix") {
            // Use a very short timeout (100ms) with a command that sleeps for 10 seconds
            let result =
                run_single_criterion_with_timeout("sleep 10", None, Duration::from_millis(100))
                    .unwrap();

            assert!(!result.success);
            assert!(result.timed_out);
            assert!(result.exit_code.is_none()); // killed process has no exit code
            assert!(result.stderr.contains("timeout"));
            // Duration should be close to the timeout, not 10 seconds
            assert!(result.duration < Duration::from_secs(1));
        }
    }

    #[test]
    fn test_criteria_config_default() {
        let config = CriteriaConfig::default();
        assert_eq!(config.command_timeout, DEFAULT_COMMAND_TIMEOUT);
        assert_eq!(config.command_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_criteria_config_with_timeout() {
        let config = CriteriaConfig::with_timeout(Duration::from_secs(60));
        assert_eq!(config.command_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_run_acceptance_empty() {
        use crate::models::stage::Stage;

        let stage = Stage::new("test".to_string(), None);
        let result = run_acceptance(&stage, None).unwrap();

        assert!(result.all_passed());
        assert_eq!(result.results().len(), 0);
    }

    #[test]
    fn test_run_acceptance_all_pass() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);
        let command = if cfg!(target_family = "unix") {
            "true"
        } else {
            "exit /b 0"
        };
        stage.add_acceptance_criterion(command.to_string());
        stage.add_acceptance_criterion(command.to_string());

        let result = run_acceptance(&stage, None).unwrap();

        assert!(result.all_passed());
        assert_eq!(result.results().len(), 2);
        assert_eq!(result.passed_count(), 2);
        assert_eq!(result.failed_count(), 0);
    }

    #[test]
    fn test_run_acceptance_some_fail() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            stage.add_acceptance_criterion("true".to_string());
            stage.add_acceptance_criterion("false".to_string());
        } else {
            stage.add_acceptance_criterion("exit /b 0".to_string());
            stage.add_acceptance_criterion("exit /b 1".to_string());
        }

        let result = run_acceptance(&stage, None).unwrap();

        assert!(!result.all_passed());
        assert_eq!(result.results().len(), 2);
        assert_eq!(result.passed_count(), 1);
        assert_eq!(result.failed_count(), 1);
        assert_eq!(result.failures().len(), 1);
    }

    #[test]
    fn test_run_single_criterion_with_working_dir() {
        use std::path::PathBuf;

        // Create temp dir and verify command runs in it
        let temp_dir = std::env::temp_dir();
        let command = if cfg!(target_family = "unix") {
            "pwd"
        } else {
            "cd"
        };

        let result = run_single_criterion(command, Some(&temp_dir)).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        // The output should contain the temp directory path
        let canonical_temp = temp_dir.canonicalize().unwrap_or(temp_dir.clone());
        let stdout_path = PathBuf::from(result.stdout.trim());
        let canonical_stdout = stdout_path.canonicalize().unwrap_or(stdout_path);
        assert_eq!(canonical_stdout, canonical_temp);
    }

    #[test]
    fn test_run_acceptance_with_setup_commands() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Setup creates an environment variable, criterion checks it exists
            stage.setup.push("export TEST_VAR=hello".to_string());
            stage.add_acceptance_criterion("test -n \"$TEST_VAR\"".to_string());
        } else {
            // Windows: set var and check it
            stage.setup.push("set TEST_VAR=hello".to_string());
            stage.add_acceptance_criterion(
                "if defined TEST_VAR (exit /b 0) else (exit /b 1)".to_string(),
            );
        }

        let result = run_acceptance(&stage, None).unwrap();

        assert!(result.all_passed());
        assert_eq!(result.passed_count(), 1);
        // Verify the result stores original command, not the combined one
        assert!(!result.results()[0].command.contains("export"));
    }

    #[test]
    fn test_run_acceptance_setup_failure_fails_criterion() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Setup command that fails
            stage.setup.push("false".to_string());
            stage.add_acceptance_criterion("true".to_string());
        } else {
            stage.setup.push("exit /b 1".to_string());
            stage.add_acceptance_criterion("exit /b 0".to_string());
        }

        let result = run_acceptance(&stage, None).unwrap();

        // Even though the criterion itself would pass, setup failure causes failure
        assert!(!result.all_passed());
        assert_eq!(result.failed_count(), 1);
    }

    #[test]
    fn test_run_acceptance_multiple_setup_commands() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Multiple setup commands chained
            stage.setup.push("export A=1".to_string());
            stage.setup.push("export B=2".to_string());
            stage.add_acceptance_criterion("test -n \"$A\" && test -n \"$B\"".to_string());
        } else {
            stage.setup.push("set A=1".to_string());
            stage.setup.push("set B=2".to_string());
            stage.add_acceptance_criterion(
                "if defined A if defined B (exit /b 0) else (exit /b 1)".to_string(),
            );
        }

        let result = run_acceptance(&stage, None).unwrap();

        assert!(result.all_passed());
    }

    #[test]
    fn test_run_acceptance_with_worktree_variable() {
        use crate::models::stage::Stage;
        use tempfile::tempdir;

        // Create a temp directory to use as the working dir
        let temp_dir = tempdir().expect("failed to create temp dir");

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Use ${WORKTREE} variable in criterion - it should expand to working_dir
            stage.add_acceptance_criterion("test -d \"${WORKTREE}\"".to_string());
        } else {
            stage.add_acceptance_criterion("if exist \"${WORKTREE}\" (exit /b 0)".to_string());
        }

        let result = run_acceptance(&stage, Some(temp_dir.path())).unwrap();

        assert!(result.all_passed());
        // The stored command should be the original, not expanded
        assert!(result.results()[0].command.contains("${WORKTREE}"));
    }

    #[test]
    fn test_run_acceptance_with_project_root_variable() {
        use crate::models::stage::Stage;
        use std::fs;
        use tempfile::tempdir;

        // Create a temp directory with a Cargo.toml to trigger PROJECT_ROOT detection
        let temp_dir = tempdir().expect("failed to create temp dir");
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]")
            .expect("failed to write Cargo.toml");

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Use ${PROJECT_ROOT} variable - should be the dir with Cargo.toml
            stage.add_acceptance_criterion("test -f \"${PROJECT_ROOT}/Cargo.toml\"".to_string());
        } else {
            stage.add_acceptance_criterion(
                "if exist \"${PROJECT_ROOT}\\Cargo.toml\" (exit /b 0)".to_string(),
            );
        }

        let result = run_acceptance(&stage, Some(temp_dir.path())).unwrap();

        assert!(result.all_passed());
    }

    #[test]
    fn test_run_acceptance_with_stage_id_variable() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Use ${STAGE_ID} variable - should expand to the stage's id
            stage.add_acceptance_criterion("test -n \"${STAGE_ID}\"".to_string());
        } else {
            stage.add_acceptance_criterion("echo %STAGE_ID%".to_string());
        }

        let result = run_acceptance(&stage, None).unwrap();

        assert!(result.all_passed());
    }

    #[test]
    fn test_run_acceptance_variables_in_setup() {
        use crate::models::stage::Stage;
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("failed to create temp dir");

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Setup uses ${WORKTREE} variable
            stage.setup.push("cd ${WORKTREE}".to_string());
            stage.add_acceptance_criterion("pwd".to_string());
        } else {
            stage.setup.push("cd ${WORKTREE}".to_string());
            stage.add_acceptance_criterion("cd".to_string());
        }

        let result = run_acceptance(&stage, Some(temp_dir.path())).unwrap();

        assert!(result.all_passed());
    }

    #[test]
    fn test_run_acceptance_unknown_variable_unchanged() {
        use crate::models::stage::Stage;

        let mut stage = Stage::new("test".to_string(), None);

        if cfg!(target_family = "unix") {
            // Unknown variable should remain unchanged, and the echo should succeed
            stage.add_acceptance_criterion("echo \"${UNKNOWN_VAR}\"".to_string());
        } else {
            stage.add_acceptance_criterion("echo ${UNKNOWN_VAR}".to_string());
        }

        let result = run_acceptance(&stage, None).unwrap();

        // Command should pass (echo always succeeds)
        assert!(result.all_passed());
    }
}
