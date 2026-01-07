use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::models::stage::Stage;

/// Result of executing a single acceptance criterion (shell command)
#[derive(Debug, Clone)]
pub struct CriterionResult {
    pub command: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
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
    ) -> Self {
        Self {
            command,
            success,
            stdout,
            stderr,
            exit_code,
            duration,
        }
    }

    /// Check if the criterion passed
    pub fn passed(&self) -> bool {
        self.success
    }

    /// Get a summary of the result
    pub fn summary(&self) -> String {
        let status = if self.success { "PASSED" } else { "FAILED" };
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

/// Run all acceptance criteria for a stage
///
/// Executes each shell command sequentially and collects results.
/// Returns AllPassed if all commands exit with code 0, Failed otherwise.
///
/// If `working_dir` is provided, commands will be executed in that directory.
/// This is typically used to run criteria in a worktree directory.
///
/// If the stage has setup commands defined, they will be prepended to each
/// criterion command using `&&` to ensure environment preparation runs first.
pub fn run_acceptance(stage: &Stage, working_dir: Option<&Path>) -> Result<AcceptanceResult> {
    if stage.acceptance.is_empty() {
        return Ok(AcceptanceResult::AllPassed {
            results: Vec::new(),
        });
    }

    let mut results = Vec::new();
    let mut failures = Vec::new();

    // Build setup prefix if setup commands are defined
    let setup_prefix = if stage.setup.is_empty() {
        None
    } else {
        Some(stage.setup.join(" && "))
    };

    for command in &stage.acceptance {
        // Combine setup commands with criterion if setup is defined
        let full_command = match &setup_prefix {
            Some(prefix) => format!("{prefix} && {command}"),
            None => command.clone(),
        };

        let result = run_single_criterion(&full_command, working_dir)
            .with_context(|| format!("Failed to execute criterion: {command}"))?;

        if !result.success {
            failures.push(format!(
                "Command '{}' failed with exit code {:?}",
                command, result.exit_code
            ));
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

/// Run a single acceptance criterion (shell command)
///
/// Executes the command using the system shell and captures all output.
/// Returns a CriterionResult with execution details.
///
/// If `working_dir` is provided, the command will be executed in that directory.
pub fn run_single_criterion(command: &str, working_dir: Option<&Path>) -> Result<CriterionResult> {
    let start = Instant::now();

    // Use sh -c on Unix systems to execute the command in a shell
    let output = if cfg!(target_family = "unix") {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        cmd.output()
            .with_context(|| format!("Failed to spawn command: {command}"))?
    } else {
        // Windows: use cmd /C
        let mut cmd = Command::new("cmd");
        cmd.arg("/C")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        cmd.output()
            .with_context(|| format!("Failed to spawn command: {command}"))?
    };

    let duration = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();
    let exit_code = output.status.code();

    Ok(CriterionResult::new(
        command.to_string(),
        success,
        stdout,
        stderr,
        exit_code,
        duration,
    ))
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
        );

        assert!(result.passed());
        assert_eq!(result.command, "echo test");
        assert_eq!(result.stdout, "test\n");
        assert_eq!(result.stderr, "");
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.duration, Duration::from_millis(100));
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
        );

        let summary = result.summary();
        assert!(summary.contains("FAILED"));
        assert!(summary.contains("cargo test"));
        assert!(summary.contains("500ms"));
        assert!(summary.contains("exit code: Some(1)"));
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
            ),
            CriterionResult::new(
                "test2".to_string(),
                true,
                "ok".to_string(),
                String::new(),
                Some(0),
                Duration::from_millis(200),
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
            ),
            CriterionResult::new(
                "test2".to_string(),
                false,
                String::new(),
                "error".to_string(),
                Some(1),
                Duration::from_millis(200),
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
}
