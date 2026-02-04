//! Type definitions for baseline capture and change impact analysis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A captured test baseline representing the state before stage changes.
///
/// This captures the output of running a baseline command (typically a test suite)
/// and categorizes the results into passed, failed, and warning counts based on
/// configured patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBaseline {
    /// Stage ID this baseline belongs to
    pub stage_id: String,
    /// The command that was run to generate this baseline
    pub command: String,
    /// Raw stdout from the command
    pub stdout: String,
    /// Raw stderr from the command
    pub stderr: String,
    /// Exit code of the command
    pub exit_code: Option<i32>,
    /// Count of failures detected using failure_patterns
    pub failure_count: usize,
    /// Lines matching failure patterns
    pub failure_lines: Vec<String>,
    /// Count of warnings detected using warning_patterns
    pub warning_count: usize,
    /// Lines matching warning patterns
    pub warning_lines: Vec<String>,
    /// When this baseline was captured
    pub captured_at: DateTime<Utc>,
}

impl TestBaseline {
    /// Create a new baseline from command output
    pub fn new(
        stage_id: impl Into<String>,
        command: impl Into<String>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        exit_code: Option<i32>,
        failure_lines: Vec<String>,
        warning_lines: Vec<String>,
    ) -> Self {
        let failure_count = failure_lines.len();
        let warning_count = warning_lines.len();
        Self {
            stage_id: stage_id.into(),
            command: command.into(),
            stdout: stdout.into(),
            stderr: stderr.into(),
            exit_code,
            failure_count,
            failure_lines,
            warning_count,
            warning_lines,
            captured_at: Utc::now(),
        }
    }
}

/// The impact of changes detected by comparing baseline to current state.
///
/// This tracks what changed between the baseline capture (before stage work)
/// and the comparison run (after stage work).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeImpact {
    /// New failures introduced (not in baseline, now failing)
    pub new_failures: Vec<String>,
    /// Failures that were fixed (in baseline, now passing)
    pub fixed_failures: Vec<String>,
    /// New warnings introduced
    pub new_warnings: Vec<String>,
    /// Warnings that were fixed
    pub fixed_warnings: Vec<String>,
    /// Exit code of the comparison command
    pub exit_code: Option<i32>,
    /// Whether the comparison itself succeeded (command ran)
    pub comparison_succeeded: bool,
}

impl ChangeImpact {
    /// Create a successful comparison result
    pub fn new(
        new_failures: Vec<String>,
        fixed_failures: Vec<String>,
        new_warnings: Vec<String>,
        fixed_warnings: Vec<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self {
            new_failures,
            fixed_failures,
            new_warnings,
            fixed_warnings,
            exit_code,
            comparison_succeeded: true,
        }
    }

    /// Create a failed comparison result (command couldn't run)
    pub fn failed() -> Self {
        Self {
            comparison_succeeded: false,
            ..Default::default()
        }
    }

    /// Check if any new failures were introduced
    pub fn has_new_failures(&self) -> bool {
        !self.new_failures.is_empty()
    }

    /// Check if any new warnings were introduced
    pub fn has_new_warnings(&self) -> bool {
        !self.new_warnings.is_empty()
    }

    /// Check if the change is "clean" (no new failures or warnings)
    pub fn is_clean(&self) -> bool {
        self.comparison_succeeded && self.new_failures.is_empty() && self.new_warnings.is_empty()
    }

    /// Get a summary of the impact
    pub fn summary(&self) -> String {
        if !self.comparison_succeeded {
            return "Comparison command failed to run".to_string();
        }

        let mut parts = Vec::new();

        if !self.new_failures.is_empty() {
            parts.push(format!("{} new failure(s)", self.new_failures.len()));
        }
        if !self.fixed_failures.is_empty() {
            parts.push(format!("{} fixed failure(s)", self.fixed_failures.len()));
        }
        if !self.new_warnings.is_empty() {
            parts.push(format!("{} new warning(s)", self.new_warnings.len()));
        }
        if !self.fixed_warnings.is_empty() {
            parts.push(format!("{} fixed warning(s)", self.fixed_warnings.len()));
        }

        if parts.is_empty() {
            "No change impact detected".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_creation() {
        let baseline = TestBaseline::new(
            "test-stage",
            "cargo test",
            "test output",
            "",
            Some(0),
            vec!["FAILED: test_foo".to_string()],
            vec!["warning: unused variable".to_string()],
        );

        assert_eq!(baseline.stage_id, "test-stage");
        assert_eq!(baseline.failure_count, 1);
        assert_eq!(baseline.warning_count, 1);
    }

    #[test]
    fn test_change_impact_clean() {
        let impact = ChangeImpact::new(vec![], vec![], vec![], vec![], Some(0));
        assert!(impact.is_clean());
        assert!(!impact.has_new_failures());
        assert!(!impact.has_new_warnings());
    }

    #[test]
    fn test_change_impact_with_new_failures() {
        let impact = ChangeImpact::new(
            vec!["FAILED: new_test".to_string()],
            vec![],
            vec![],
            vec![],
            Some(1),
        );
        assert!(!impact.is_clean());
        assert!(impact.has_new_failures());
    }

    #[test]
    fn test_change_impact_summary() {
        let impact = ChangeImpact::new(
            vec!["FAILED: new_test".to_string()],
            vec!["FAILED: old_test".to_string()],
            vec![],
            vec![],
            Some(1),
        );
        let summary = impact.summary();
        assert!(summary.contains("1 new failure"));
        assert!(summary.contains("1 fixed failure"));
    }
}
