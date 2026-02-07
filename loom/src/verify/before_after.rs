//! Before/after stage verification
//!
//! Verifies pre-conditions (before implementation) and post-conditions (after implementation)
//! using TruthCheck definitions from the plan.
//!
//! - Before-stage: Plan author writes TruthChecks describing the expected "before" state
//!   (e.g., exit_code: 1 for a test that should fail before the feature exists)
//! - After-stage: Plan author writes TruthChecks describing the expected "after" state
//!   (e.g., exit_code: 0 for a test that should pass after the feature is built)
//!
//! Both use verify_truth_checks() internally - the verification logic is identical.
//! The difference is semantic: when the checks run in the stage lifecycle.

use anyhow::Result;
use std::path::Path;

use crate::plan::schema::TruthCheck;
use crate::verify::goal_backward::{verify_truth_checks, VerificationGap};

/// Run before-stage checks to verify pre-conditions.
///
/// Executes TruthChecks that describe the expected state BEFORE implementation.
/// If any check fails, the pre-conditions are not met and the stage should not proceed.
pub fn run_before_stage_checks(
    checks: &[TruthCheck],
    working_dir: &Path,
) -> Result<Vec<VerificationGap>> {
    verify_truth_checks(checks, working_dir)
}

/// Run after-stage checks to verify post-conditions.
///
/// Executes TruthChecks that describe the expected state AFTER implementation.
/// If any check fails, the post-conditions are not met and the stage completion should fail.
pub fn run_after_stage_checks(
    checks: &[TruthCheck],
    working_dir: &Path,
) -> Result<Vec<VerificationGap>> {
    verify_truth_checks(checks, working_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_before_after_stage_checks_pass() {
        // Simulate a "before" check: command exits 1 (feature doesn't exist yet)
        let before_checks = vec![TruthCheck {
            command: "exit 1".to_string(),
            stdout_contains: vec![],
            stdout_not_contains: vec![],
            stderr_empty: None,
            exit_code: Some(1),
            description: Some("Feature test should fail before implementation".to_string()),
        }];

        let working_dir = env::temp_dir();
        let gaps = run_before_stage_checks(&before_checks, &working_dir).unwrap();
        assert!(
            gaps.is_empty(),
            "Before-stage checks should pass when pre-conditions match"
        );
    }

    #[test]
    fn test_before_stage_checks_fail_when_precondition_not_met() {
        // Before check expects exit 1, but command exits 0 (feature already exists)
        let before_checks = vec![TruthCheck {
            command: "exit 0".to_string(),
            stdout_contains: vec![],
            stdout_not_contains: vec![],
            stderr_empty: None,
            exit_code: Some(1),
            description: Some("Feature should not exist yet".to_string()),
        }];

        let working_dir = env::temp_dir();
        let gaps = run_before_stage_checks(&before_checks, &working_dir).unwrap();
        assert_eq!(
            gaps.len(),
            1,
            "Should report gap when pre-condition not met"
        );
    }

    #[test]
    fn test_after_stage_checks_pass() {
        // Simulate an "after" check: command exits 0 (feature works)
        let after_checks = vec![TruthCheck {
            command: "echo 'feature works'".to_string(),
            stdout_contains: vec!["feature works".to_string()],
            stdout_not_contains: vec![],
            stderr_empty: None,
            exit_code: Some(0),
            description: Some("Feature should work after implementation".to_string()),
        }];

        let working_dir = env::temp_dir();
        let gaps = run_after_stage_checks(&after_checks, &working_dir).unwrap();
        assert!(
            gaps.is_empty(),
            "After-stage checks should pass when post-conditions match"
        );
    }

    #[test]
    fn test_after_stage_checks_fail_when_postcondition_not_met() {
        // After check expects stdout to contain "feature works", but it doesn't
        let after_checks = vec![TruthCheck {
            command: "echo 'something else'".to_string(),
            stdout_contains: vec!["feature works".to_string()],
            stdout_not_contains: vec![],
            stderr_empty: None,
            exit_code: None,
            description: Some("Feature output check".to_string()),
        }];

        let working_dir = env::temp_dir();
        let gaps = run_after_stage_checks(&after_checks, &working_dir).unwrap();
        assert_eq!(
            gaps.len(),
            1,
            "Should report gap when post-condition not met"
        );
    }

    #[test]
    fn test_before_after_empty_checks() {
        let working_dir = env::temp_dir();

        let gaps = run_before_stage_checks(&[], &working_dir).unwrap();
        assert!(
            gaps.is_empty(),
            "Empty before checks should produce no gaps"
        );

        let gaps = run_after_stage_checks(&[], &working_dir).unwrap();
        assert!(gaps.is_empty(), "Empty after checks should produce no gaps");
    }

    #[test]
    fn test_before_stage_stdout_not_contains() {
        // Before check: stdout should NOT contain "FeatureX" (feature doesn't exist)
        let checks = vec![TruthCheck {
            command: "echo 'no features here'".to_string(),
            stdout_contains: vec![],
            stdout_not_contains: vec!["FeatureX".to_string()],
            stderr_empty: None,
            exit_code: None,
            description: Some("FeatureX should not appear before implementation".to_string()),
        }];

        let working_dir = env::temp_dir();
        let gaps = run_before_stage_checks(&checks, &working_dir).unwrap();
        assert!(
            gaps.is_empty(),
            "Check should pass when forbidden pattern is absent"
        );
    }

    #[test]
    fn test_after_stage_multiple_checks() {
        let checks = vec![
            TruthCheck {
                command: "echo 'test passed'".to_string(),
                stdout_contains: vec!["test passed".to_string()],
                stdout_not_contains: vec![],
                stderr_empty: None,
                exit_code: Some(0),
                description: Some("First post-condition".to_string()),
            },
            TruthCheck {
                command: "echo 'integration ok'".to_string(),
                stdout_contains: vec!["integration ok".to_string()],
                stdout_not_contains: vec![],
                stderr_empty: None,
                exit_code: Some(0),
                description: Some("Second post-condition".to_string()),
            },
        ];

        let working_dir = env::temp_dir();
        let gaps = run_after_stage_checks(&checks, &working_dir).unwrap();
        assert!(gaps.is_empty(), "All after-stage checks should pass");
    }
}
