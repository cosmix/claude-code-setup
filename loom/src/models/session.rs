use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::constants::{CONTEXT_WARNING_THRESHOLD, DEFAULT_CONTEXT_LIMIT};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub stage_id: Option<String>,
    pub tmux_session: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub pid: Option<u32>,
    pub status: SessionStatus,
    pub context_tokens: u32,
    pub context_limit: u32,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Spawning,
    Running,
    Paused,
    Completed,
    Crashed,
    ContextExhausted,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Spawning => write!(f, "Spawning"),
            SessionStatus::Running => write!(f, "Running"),
            SessionStatus::Paused => write!(f, "Paused"),
            SessionStatus::Completed => write!(f, "Completed"),
            SessionStatus::Crashed => write!(f, "Crashed"),
            SessionStatus::ContextExhausted => write!(f, "ContextExhausted"),
        }
    }
}

impl SessionStatus {
    /// Check if transitioning from the current status to the new status is valid.
    ///
    /// Valid transitions:
    /// - `Spawning` -> `Running`
    /// - `Running` -> `Completed` | `Paused` | `Crashed` | `ContextExhausted`
    /// - `Paused` -> `Running`
    ///
    /// Terminal states (no outgoing transitions):
    /// - `Completed`
    /// - `Crashed`
    /// - `ContextExhausted`
    ///
    /// # Arguments
    /// * `new_status` - The target status to transition to
    ///
    /// # Returns
    /// `true` if the transition is valid, `false` otherwise
    pub fn can_transition_to(&self, new_status: &SessionStatus) -> bool {
        // Same status is always valid (no-op)
        if self == new_status {
            return true;
        }

        match self {
            SessionStatus::Spawning => matches!(new_status, SessionStatus::Running),
            SessionStatus::Running => matches!(
                new_status,
                SessionStatus::Completed
                    | SessionStatus::Paused
                    | SessionStatus::Crashed
                    | SessionStatus::ContextExhausted
            ),
            SessionStatus::Paused => matches!(new_status, SessionStatus::Running),
            // Terminal states
            SessionStatus::Completed | SessionStatus::Crashed | SessionStatus::ContextExhausted => {
                false
            }
        }
    }

    /// Attempt to transition to a new status, returning an error if invalid.
    ///
    /// # Arguments
    /// * `new_status` - The target status to transition to
    ///
    /// # Returns
    /// `Ok(new_status)` if the transition is valid, `Err` otherwise
    pub fn try_transition(&self, new_status: SessionStatus) -> Result<SessionStatus> {
        if self.can_transition_to(&new_status) {
            Ok(new_status)
        } else {
            bail!("Invalid session status transition: {self} -> {new_status}")
        }
    }

    /// Returns the list of valid statuses this status can transition to.
    pub fn valid_transitions(&self) -> Vec<SessionStatus> {
        match self {
            SessionStatus::Spawning => vec![SessionStatus::Running],
            SessionStatus::Running => vec![
                SessionStatus::Completed,
                SessionStatus::Paused,
                SessionStatus::Crashed,
                SessionStatus::ContextExhausted,
            ],
            SessionStatus::Paused => vec![SessionStatus::Running],
            SessionStatus::Completed | SessionStatus::Crashed | SessionStatus::ContextExhausted => {
                vec![]
            }
        }
    }

    /// Returns true if this is a terminal state (no valid outgoing transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SessionStatus::Completed | SessionStatus::Crashed | SessionStatus::ContextExhausted
        )
    }
}

impl Session {
    pub fn new() -> Self {
        let now = Utc::now();
        let id = Self::generate_id();

        Self {
            id,
            stage_id: None,
            tmux_session: None,
            worktree_path: None,
            pid: None,
            status: SessionStatus::Spawning,
            context_tokens: 0,
            context_limit: DEFAULT_CONTEXT_LIMIT,
            created_at: now,
            last_active: now,
        }
    }

    fn generate_id() -> String {
        let timestamp = Utc::now().timestamp();
        let uuid_short = uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("")
            .to_string();
        format!("session-{uuid_short}-{timestamp}")
    }

    pub fn assign_to_stage(&mut self, stage_id: String) {
        self.stage_id = Some(stage_id);
        self.last_active = Utc::now();
    }

    pub fn release_from_stage(&mut self) {
        self.stage_id = None;
        self.last_active = Utc::now();
    }

    pub fn set_tmux_session(&mut self, session_name: String) {
        self.tmux_session = Some(session_name);
    }

    pub fn set_worktree_path(&mut self, path: PathBuf) {
        self.worktree_path = Some(path);
    }

    pub fn set_pid(&mut self, pid: u32) {
        self.pid = Some(pid);
    }

    pub fn clear_pid(&mut self) {
        self.pid = None;
    }

    pub fn update_context(&mut self, tokens: u32) {
        self.context_tokens = tokens;
        self.last_active = Utc::now();
    }

    pub fn context_health(&self) -> f32 {
        if self.context_limit == 0 {
            return 0.0;
        }
        (self.context_tokens as f32 / self.context_limit as f32) * 100.0
    }

    pub fn is_context_exhausted(&self) -> bool {
        if self.context_limit == 0 {
            return false;
        }
        let usage_fraction = self.context_tokens as f32 / self.context_limit as f32;
        usage_fraction >= CONTEXT_WARNING_THRESHOLD
    }

    /// Attempt to transition the session to a new status with validation.
    ///
    /// This is the primary method for changing session status. It validates
    /// that the transition is allowed before applying it.
    ///
    /// # Arguments
    /// * `new_status` - The target status to transition to
    ///
    /// # Returns
    /// `Ok(())` if the transition succeeded, `Err` if the transition is invalid
    pub fn try_transition(&mut self, new_status: SessionStatus) -> Result<()> {
        let validated_status = self.status.try_transition(new_status)?;
        self.status = validated_status;
        self.last_active = Utc::now();
        Ok(())
    }

    /// Mark the session as running with validation.
    ///
    /// # Returns
    /// `Ok(())` if the transition succeeded, `Err` if invalid
    pub fn try_mark_running(&mut self) -> Result<()> {
        self.try_transition(SessionStatus::Running)
    }

    /// Mark the session as paused with validation.
    ///
    /// # Returns
    /// `Ok(())` if the transition succeeded, `Err` if invalid
    pub fn try_mark_paused(&mut self) -> Result<()> {
        self.try_transition(SessionStatus::Paused)
    }

    /// Mark the session as completed with validation.
    ///
    /// # Returns
    /// `Ok(())` if the transition succeeded, `Err` if invalid
    pub fn try_mark_completed(&mut self) -> Result<()> {
        self.try_transition(SessionStatus::Completed)
    }

    /// Mark the session as crashed with validation.
    ///
    /// # Returns
    /// `Ok(())` if the transition succeeded, `Err` if invalid
    pub fn try_mark_crashed(&mut self) -> Result<()> {
        self.try_transition(SessionStatus::Crashed)
    }

    /// Mark the session as context exhausted with validation.
    ///
    /// # Returns
    /// `Ok(())` if the transition succeeded, `Err` if invalid
    pub fn try_mark_context_exhausted(&mut self) -> Result<()> {
        self.try_transition(SessionStatus::ContextExhausted)
    }

    // Deprecated: Use try_mark_running for validated transitions
    #[deprecated(
        since = "0.2.0",
        note = "Use try_mark_running for validated transitions"
    )]
    pub fn mark_running(&mut self) {
        self.status = SessionStatus::Running;
    }

    // Deprecated: Use try_mark_paused for validated transitions
    #[deprecated(
        since = "0.2.0",
        note = "Use try_mark_paused for validated transitions"
    )]
    pub fn mark_paused(&mut self) {
        self.status = SessionStatus::Paused;
    }

    // Deprecated: Use try_mark_completed for validated transitions
    #[deprecated(
        since = "0.2.0",
        note = "Use try_mark_completed for validated transitions"
    )]
    pub fn mark_completed(&mut self) {
        self.status = SessionStatus::Completed;
    }

    // Deprecated: Use try_mark_crashed for validated transitions
    #[deprecated(
        since = "0.2.0",
        note = "Use try_mark_crashed for validated transitions"
    )]
    pub fn mark_crashed(&mut self) {
        self.status = SessionStatus::Crashed;
    }

    // Deprecated: Use try_mark_context_exhausted for validated transitions
    #[deprecated(
        since = "0.2.0",
        note = "Use try_mark_context_exhausted for validated transitions"
    )]
    pub fn mark_context_exhausted(&mut self) {
        self.status = SessionStatus::ContextExhausted;
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session(status: SessionStatus) -> Session {
        let mut session = Session::new();
        session.status = status;
        session
    }

    // =========================================================================
    // SessionStatus::can_transition_to tests
    // =========================================================================

    #[test]
    fn test_spawning_can_transition_to_running() {
        let status = SessionStatus::Spawning;
        assert!(status.can_transition_to(&SessionStatus::Running));
    }

    #[test]
    fn test_spawning_cannot_transition_to_other_states() {
        let status = SessionStatus::Spawning;
        assert!(!status.can_transition_to(&SessionStatus::Paused));
        assert!(!status.can_transition_to(&SessionStatus::Completed));
        assert!(!status.can_transition_to(&SessionStatus::Crashed));
        assert!(!status.can_transition_to(&SessionStatus::ContextExhausted));
    }

    #[test]
    fn test_running_can_transition_to_valid_states() {
        let status = SessionStatus::Running;
        assert!(status.can_transition_to(&SessionStatus::Completed));
        assert!(status.can_transition_to(&SessionStatus::Paused));
        assert!(status.can_transition_to(&SessionStatus::Crashed));
        assert!(status.can_transition_to(&SessionStatus::ContextExhausted));
    }

    #[test]
    fn test_running_cannot_transition_to_spawning() {
        let status = SessionStatus::Running;
        assert!(!status.can_transition_to(&SessionStatus::Spawning));
    }

    #[test]
    fn test_paused_can_transition_to_running() {
        let status = SessionStatus::Paused;
        assert!(status.can_transition_to(&SessionStatus::Running));
    }

    #[test]
    fn test_paused_cannot_transition_to_other_states() {
        let status = SessionStatus::Paused;
        assert!(!status.can_transition_to(&SessionStatus::Spawning));
        assert!(!status.can_transition_to(&SessionStatus::Completed));
        assert!(!status.can_transition_to(&SessionStatus::Crashed));
        assert!(!status.can_transition_to(&SessionStatus::ContextExhausted));
    }

    #[test]
    fn test_completed_is_terminal_state() {
        let status = SessionStatus::Completed;
        assert!(!status.can_transition_to(&SessionStatus::Spawning));
        assert!(!status.can_transition_to(&SessionStatus::Running));
        assert!(!status.can_transition_to(&SessionStatus::Paused));
        assert!(!status.can_transition_to(&SessionStatus::Crashed));
        assert!(!status.can_transition_to(&SessionStatus::ContextExhausted));
    }

    #[test]
    fn test_crashed_is_terminal_state() {
        let status = SessionStatus::Crashed;
        assert!(!status.can_transition_to(&SessionStatus::Spawning));
        assert!(!status.can_transition_to(&SessionStatus::Running));
        assert!(!status.can_transition_to(&SessionStatus::Paused));
        assert!(!status.can_transition_to(&SessionStatus::Completed));
        assert!(!status.can_transition_to(&SessionStatus::ContextExhausted));
    }

    #[test]
    fn test_context_exhausted_is_terminal_state() {
        let status = SessionStatus::ContextExhausted;
        assert!(!status.can_transition_to(&SessionStatus::Spawning));
        assert!(!status.can_transition_to(&SessionStatus::Running));
        assert!(!status.can_transition_to(&SessionStatus::Paused));
        assert!(!status.can_transition_to(&SessionStatus::Completed));
        assert!(!status.can_transition_to(&SessionStatus::Crashed));
    }

    #[test]
    fn test_same_status_transition_is_valid() {
        let statuses = vec![
            SessionStatus::Spawning,
            SessionStatus::Running,
            SessionStatus::Paused,
            SessionStatus::Completed,
            SessionStatus::Crashed,
            SessionStatus::ContextExhausted,
        ];

        for status in statuses {
            assert!(
                status.can_transition_to(&status.clone()),
                "Same-state transition should be valid for {status:?}"
            );
        }
    }

    // =========================================================================
    // SessionStatus::try_transition tests
    // =========================================================================

    #[test]
    fn test_try_transition_valid_spawning_to_running() {
        let status = SessionStatus::Spawning;
        let result = status.try_transition(SessionStatus::Running);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SessionStatus::Running);
    }

    #[test]
    fn test_try_transition_invalid_completed_to_running() {
        let status = SessionStatus::Completed;
        let result = status.try_transition(SessionStatus::Running);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid session status transition"));
        assert!(err.contains("Completed"));
        assert!(err.contains("Running"));
    }

    // =========================================================================
    // SessionStatus::valid_transitions tests
    // =========================================================================

    #[test]
    fn test_valid_transitions_spawning() {
        let transitions = SessionStatus::Spawning.valid_transitions();
        assert_eq!(transitions, vec![SessionStatus::Running]);
    }

    #[test]
    fn test_valid_transitions_running() {
        let transitions = SessionStatus::Running.valid_transitions();
        assert_eq!(transitions.len(), 4);
        assert!(transitions.contains(&SessionStatus::Completed));
        assert!(transitions.contains(&SessionStatus::Paused));
        assert!(transitions.contains(&SessionStatus::Crashed));
        assert!(transitions.contains(&SessionStatus::ContextExhausted));
    }

    #[test]
    fn test_valid_transitions_terminal_states() {
        assert!(SessionStatus::Completed.valid_transitions().is_empty());
        assert!(SessionStatus::Crashed.valid_transitions().is_empty());
        assert!(SessionStatus::ContextExhausted
            .valid_transitions()
            .is_empty());
    }

    // =========================================================================
    // SessionStatus::is_terminal tests
    // =========================================================================

    #[test]
    fn test_is_terminal_true_for_terminal_states() {
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Crashed.is_terminal());
        assert!(SessionStatus::ContextExhausted.is_terminal());
    }

    #[test]
    fn test_is_terminal_false_for_non_terminal_states() {
        assert!(!SessionStatus::Spawning.is_terminal());
        assert!(!SessionStatus::Running.is_terminal());
        assert!(!SessionStatus::Paused.is_terminal());
    }

    // =========================================================================
    // Session::try_transition tests
    // =========================================================================

    #[test]
    fn test_session_try_transition_valid() {
        let mut session = create_test_session(SessionStatus::Spawning);
        let result = session.try_transition(SessionStatus::Running);
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_session_try_transition_invalid() {
        let mut session = create_test_session(SessionStatus::Completed);
        let result = session.try_transition(SessionStatus::Running);
        assert!(result.is_err());
        assert_eq!(session.status, SessionStatus::Completed); // Status unchanged
    }

    #[test]
    fn test_session_try_mark_running_from_spawning() {
        let mut session = create_test_session(SessionStatus::Spawning);
        let result = session.try_mark_running();
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_session_try_mark_running_from_paused() {
        let mut session = create_test_session(SessionStatus::Paused);
        let result = session.try_mark_running();
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_session_try_mark_running_invalid() {
        let mut session = create_test_session(SessionStatus::Completed);
        let result = session.try_mark_running();
        assert!(result.is_err());
    }

    #[test]
    fn test_session_try_mark_paused_valid() {
        let mut session = create_test_session(SessionStatus::Running);
        let result = session.try_mark_paused();
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::Paused);
    }

    #[test]
    fn test_session_try_mark_paused_invalid() {
        let mut session = create_test_session(SessionStatus::Spawning);
        let result = session.try_mark_paused();
        assert!(result.is_err());
    }

    #[test]
    fn test_session_try_mark_completed_valid() {
        let mut session = create_test_session(SessionStatus::Running);
        let result = session.try_mark_completed();
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_session_try_mark_crashed_valid() {
        let mut session = create_test_session(SessionStatus::Running);
        let result = session.try_mark_crashed();
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::Crashed);
    }

    #[test]
    fn test_session_try_mark_context_exhausted_valid() {
        let mut session = create_test_session(SessionStatus::Running);
        let result = session.try_mark_context_exhausted();
        assert!(result.is_ok());
        assert_eq!(session.status, SessionStatus::ContextExhausted);
    }

    // =========================================================================
    // Full workflow tests
    // =========================================================================

    #[test]
    fn test_full_happy_path_workflow() {
        let mut session = create_test_session(SessionStatus::Spawning);

        // Spawning -> Running
        assert!(session.try_mark_running().is_ok());
        assert_eq!(session.status, SessionStatus::Running);

        // Running -> Completed
        assert!(session.try_mark_completed().is_ok());
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_pause_resume_workflow() {
        let mut session = create_test_session(SessionStatus::Running);

        // Running -> Paused
        assert!(session.try_mark_paused().is_ok());
        assert_eq!(session.status, SessionStatus::Paused);

        // Paused -> Running
        assert!(session.try_mark_running().is_ok());
        assert_eq!(session.status, SessionStatus::Running);
    }

    #[test]
    fn test_crash_workflow() {
        let mut session = create_test_session(SessionStatus::Running);

        // Running -> Crashed
        assert!(session.try_mark_crashed().is_ok());
        assert_eq!(session.status, SessionStatus::Crashed);

        // Crashed is terminal - cannot recover
        assert!(session.try_mark_running().is_err());
    }

    #[test]
    fn test_context_exhausted_workflow() {
        let mut session = create_test_session(SessionStatus::Running);

        // Running -> ContextExhausted
        assert!(session.try_mark_context_exhausted().is_ok());
        assert_eq!(session.status, SessionStatus::ContextExhausted);

        // ContextExhausted is terminal - cannot recover
        assert!(session.try_mark_running().is_err());
    }

    #[test]
    fn test_display_implementation() {
        assert_eq!(format!("{}", SessionStatus::Spawning), "Spawning");
        assert_eq!(format!("{}", SessionStatus::Running), "Running");
        assert_eq!(format!("{}", SessionStatus::Paused), "Paused");
        assert_eq!(format!("{}", SessionStatus::Completed), "Completed");
        assert_eq!(format!("{}", SessionStatus::Crashed), "Crashed");
        assert_eq!(
            format!("{}", SessionStatus::ContextExhausted),
            "ContextExhausted"
        );
    }
}
