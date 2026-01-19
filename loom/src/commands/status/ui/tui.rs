//! TUI application for live status dashboard
//!
//! This module provides the ratatui-based terminal UI for displaying
//! live status updates from the loom daemon.
//!
//! Layout (unified design):
//! - Compact header with spinner, title, and inline progress
//! - Execution graph (scrollable DAG visualization)
//! - Unified stage table with all columns (status, name, merged, deps, elapsed)
//! - Simplified footer with keybinds and errors

use std::io::{self, Stdout};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame, Terminal,
};

use super::theme::{StatusColors, Theme};
use super::tree_widget::TreeWidget;
use super::widgets::{status_indicator, status_text};
use crate::daemon::{read_message, write_message, Request, Response, StageInfo};
use crate::models::stage::{Stage, StageStatus};

/// Connection timeout for daemon socket
const SOCKET_TIMEOUT: Duration = Duration::from_secs(2);

/// Poll timeout for event loop (100ms for responsive UI)
const POLL_TIMEOUT: Duration = Duration::from_millis(100);

/// Scroll step for arrow key navigation
const SCROLL_STEP: i32 = 2;

/// Page scroll multiplier (viewport size * this factor)
const PAGE_SCROLL_FACTOR: f64 = 0.8;

/// Fixed height for the graph area (prevents jerking from dynamic resizing)
const GRAPH_AREA_HEIGHT: u16 = 12;

/// Graph state tracking for scroll position
#[derive(Default)]
struct GraphState {
    /// Vertical scroll offset for the tree
    scroll_y: u16,
    /// Total number of lines in the tree
    total_lines: u16,
    /// Viewport height for scrolling bounds
    viewport_height: u16,
}

impl GraphState {
    /// Scroll by a delta, clamping to bounds
    fn scroll_by(&mut self, delta: i16) {
        if delta < 0 {
            self.scroll_y = self.scroll_y.saturating_sub((-delta) as u16);
        } else {
            let max_scroll = self.total_lines.saturating_sub(self.viewport_height);
            self.scroll_y = (self.scroll_y + delta as u16).min(max_scroll);
        }
    }

    /// Jump to start
    fn scroll_to_start(&mut self) {
        self.scroll_y = 0;
    }

    /// Jump to end
    fn scroll_to_end(&mut self) {
        self.scroll_y = self.total_lines.saturating_sub(self.viewport_height);
    }
}

/// Unified stage entry for the table display
#[derive(Clone)]
struct UnifiedStage {
    id: String,
    status: StageStatus,
    merged: bool,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    level: usize,
    dependencies: Vec<String>,
}

/// Live status data received from daemon
#[derive(Default)]
struct LiveStatus {
    executing: Vec<StageInfo>,
    pending: Vec<StageInfo>,
    completed: Vec<StageInfo>,
    blocked: Vec<StageInfo>,
}

impl LiveStatus {
    fn total(&self) -> usize {
        self.executing.len() + self.pending.len() + self.completed.len() + self.blocked.len()
    }

    fn progress_pct(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.completed.len() as f64 / total as f64
        }
    }

    /// Compute execution levels for all stages based on dependencies
    fn compute_levels(&self) -> std::collections::HashMap<String, usize> {
        use std::collections::{HashMap, HashSet};

        // Collect all stages into a map
        let all_stages: Vec<&StageInfo> = self
            .executing
            .iter()
            .chain(self.pending.iter())
            .chain(self.completed.iter())
            .chain(self.blocked.iter())
            .collect();

        let stage_map: HashMap<&str, &StageInfo> =
            all_stages.iter().map(|s| (s.id.as_str(), *s)).collect();

        let mut levels: HashMap<String, usize> = HashMap::new();

        fn get_level(
            stage_id: &str,
            stage_map: &HashMap<&str, &StageInfo>,
            levels: &mut HashMap<String, usize>,
            visiting: &mut HashSet<String>,
        ) -> usize {
            if let Some(&level) = levels.get(stage_id) {
                return level;
            }

            // Cycle detection - treat as level 0 to avoid infinite recursion
            if visiting.contains(stage_id) {
                return 0;
            }
            visiting.insert(stage_id.to_string());

            let stage = match stage_map.get(stage_id) {
                Some(s) => s,
                None => return 0,
            };

            let level = if stage.dependencies.is_empty() {
                0
            } else {
                stage
                    .dependencies
                    .iter()
                    .map(|dep| get_level(dep, stage_map, levels, visiting))
                    .max()
                    .unwrap_or(0)
                    + 1
            };

            visiting.remove(stage_id);
            levels.insert(stage_id.to_string(), level);
            level
        }

        for stage in &all_stages {
            let mut visiting = HashSet::new();
            get_level(&stage.id, &stage_map, &mut levels, &mut visiting);
        }

        levels
    }

    /// Build unified list of all stages for table display, sorted by execution order
    fn unified_stages(&self) -> Vec<UnifiedStage> {
        use std::collections::HashSet;

        let levels = self.compute_levels();
        let mut stages = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // Helper to convert StageInfo to UnifiedStage with level
        let to_unified =
            |stage: &StageInfo, levels: &std::collections::HashMap<String, usize>| UnifiedStage {
                id: stage.id.clone(),
                status: stage.status.clone(),
                merged: stage.merged,
                started_at: Some(stage.started_at),
                completed_at: stage.completed_at,
                level: levels.get(&stage.id).copied().unwrap_or(0),
                dependencies: stage.dependencies.clone(),
            };

        // Add all stages from each category
        for stage in &self.executing {
            if seen.insert(stage.id.clone()) {
                stages.push(to_unified(stage, &levels));
            }
        }

        for stage in &self.completed {
            if seen.insert(stage.id.clone()) {
                stages.push(to_unified(stage, &levels));
            }
        }

        for stage in &self.pending {
            if seen.insert(stage.id.clone()) {
                stages.push(to_unified(stage, &levels));
            }
        }

        for stage in &self.blocked {
            if seen.insert(stage.id.clone()) {
                stages.push(to_unified(stage, &levels));
            }
        }

        // Sort by level (execution order), then by id for consistency
        stages.sort_by(|a, b| a.level.cmp(&b.level).then_with(|| a.id.cmp(&b.id)));

        stages
    }
}

/// TUI application state
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    running: Arc<AtomicBool>,
    status: LiveStatus,
    spinner_frame: usize,
    last_error: Option<String>,
    /// Graph scrolling and caching state
    graph_state: GraphState,
    /// Mouse support enabled
    mouse_enabled: bool,
    /// Exiting flag - set when user requests exit to show immediate feedback
    exiting: bool,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> Result<Self> {
        // Set up terminal
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

        // Enable mouse capture for minimap interaction
        let mouse_enabled = crossterm::execute!(stdout, crossterm::event::EnableMouseCapture).is_ok();

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;

        Ok(Self {
            terminal,
            running: Arc::new(AtomicBool::new(true)),
            status: LiveStatus::default(),
            spinner_frame: 0,
            last_error: None,
            graph_state: GraphState::default(),
            mouse_enabled,
            exiting: false,
        })
    }

    /// Run the TUI event loop
    pub fn run(&mut self, work_path: &Path) -> Result<()> {
        let socket_path = work_path.join("orchestrator.sock");
        let mut stream = self.connect(&socket_path)?;
        self.subscribe(&mut stream)?;

        // Set stream to non-blocking for event loop
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .ok();

        let result = self.run_event_loop(&mut stream);

        // Cleanup: unsubscribe from daemon (best effort)
        let _ = write_message(&mut stream, &Request::Unsubscribe);

        result
    }

    /// Main event loop - returns on quit or daemon disconnect
    fn run_event_loop(&mut self, stream: &mut UnixStream) -> Result<()> {
        while self.running.load(Ordering::SeqCst) {
            // Check if user requested exit - show feedback and break immediately
            if self.exiting {
                self.last_error = Some("Exiting...".to_string());
                self.render()?; // Show exit message immediately
                break;
            }

            // Handle daemon messages (non-blocking)
            match read_message::<Response, _>(stream) {
                Ok(response) => {
                    self.handle_response(response);
                }
                Err(e) => {
                    // Check if this is a socket disconnect or fatal error
                    if self.is_socket_disconnected(&e) {
                        // Daemon disconnected - exit gracefully
                        self.last_error = Some("Daemon exited".to_string());
                        self.render()?; // Show final error state
                        std::thread::sleep(Duration::from_millis(500)); // Brief pause for user to see message
                        break;
                    }
                    // For other errors (timeout is expected with non-blocking reads), continue
                }
            }

            // Handle input events (keyboard and mouse)
            if event::poll(POLL_TIMEOUT)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key_event(key.code, key.modifiers);
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_event(mouse);
                    }
                    _ => {}
                }
            }

            // Update spinner
            self.spinner_frame = (self.spinner_frame + 1) % 10;

            // Render
            self.render()?;
        }

        Ok(())
    }

    /// Check if an error indicates socket disconnection
    ///
    /// Returns true only for actual disconnection errors (EOF, broken pipe, etc.)
    /// NOT for timeouts (WouldBlock, TimedOut) which are expected in non-blocking reads.
    fn is_socket_disconnected(&self, error: &anyhow::Error) -> bool {
        // First, check if the underlying error is a timeout - these are NOT disconnects
        for cause in error.chain() {
            if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
                match io_err.kind() {
                    // Timeout errors are expected with non-blocking reads - NOT a disconnect
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                        return false;
                    }
                    // These are actual disconnection errors
                    std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionAborted => {
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // Fallback: check error message for patterns that indicate disconnection
        // but NOT patterns that could indicate timeouts
        let err_str = error.to_string().to_lowercase();

        // These patterns indicate actual disconnection
        (err_str.contains("unexpectedeof")
            || err_str.contains("connection reset")
            || err_str.contains("broken pipe")
            || err_str.contains("os error 9") // EBADF - bad file descriptor
            || err_str.contains("os error 104") // ECONNRESET
            || err_str.contains("os error 32")) // EPIPE
            // Exclude timeout patterns
            && !err_str.contains("would block")
            && !err_str.contains("timed out")
            && !err_str.contains("os error 11") // EAGAIN/EWOULDBLOCK
    }

    /// Handle keyboard events for navigation and control
    fn handle_key_event(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            // Quit - set exiting flag for immediate feedback
            KeyCode::Char('q') | KeyCode::Esc => {
                self.exiting = true;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.exiting = true;
            }

            // Arrow key navigation (vertical only for tree)
            KeyCode::Up => {
                self.graph_state.scroll_by(-SCROLL_STEP as i16);
            }
            KeyCode::Down => {
                self.graph_state.scroll_by(SCROLL_STEP as i16);
            }

            // Home/End: jump to start/end
            KeyCode::Home => {
                self.graph_state.scroll_to_start();
            }
            KeyCode::End => {
                self.graph_state.scroll_to_end();
            }

            // Page Up/Down: scroll by page
            KeyCode::PageUp => {
                let page_step = (self.graph_state.viewport_height as f64 * PAGE_SCROLL_FACTOR) as i16;
                self.graph_state.scroll_by(-page_step);
            }
            KeyCode::PageDown => {
                let page_step = (self.graph_state.viewport_height as f64 * PAGE_SCROLL_FACTOR) as i16;
                self.graph_state.scroll_by(page_step);
            }

            // Ignore horizontal keys for tree view
            KeyCode::Left | KeyCode::Right => {}

            _ => {}
        }
    }

    /// Handle mouse events for scrolling
    fn handle_mouse_event(&mut self, mouse: crossterm::event::MouseEvent) {
        match mouse.kind {
            // Scroll wheel to scroll tree
            MouseEventKind::ScrollUp => {
                self.graph_state.scroll_by(-(SCROLL_STEP as i16) * 2);
            }
            MouseEventKind::ScrollDown => {
                self.graph_state.scroll_by((SCROLL_STEP as i16) * 2);
            }
            _ => {}
        }
    }

    /// Connect to daemon socket
    fn connect(&self, socket_path: &Path) -> Result<UnixStream> {
        let mut stream =
            UnixStream::connect(socket_path).context("Failed to connect to daemon socket")?;

        stream
            .set_read_timeout(Some(SOCKET_TIMEOUT))
            .context("Failed to set read timeout")?;
        stream
            .set_write_timeout(Some(SOCKET_TIMEOUT))
            .context("Failed to set write timeout")?;

        // Ping to verify daemon is responsive
        write_message(&mut stream, &Request::Ping).context("Failed to send Ping")?;

        let response: Response =
            read_message(&mut stream).context("Failed to read Ping response")?;

        match response {
            Response::Pong => {}
            Response::Error { message } => {
                anyhow::bail!("Daemon returned error: {message}");
            }
            _ => {
                anyhow::bail!("Unexpected response from daemon");
            }
        }

        Ok(stream)
    }

    /// Subscribe to status updates
    fn subscribe(&self, stream: &mut UnixStream) -> Result<()> {
        write_message(stream, &Request::SubscribeStatus)
            .context("Failed to send SubscribeStatus")?;

        let response: Response =
            read_message(stream).context("Failed to read subscription response")?;

        match response {
            Response::Ok => Ok(()),
            Response::Error { message } => {
                anyhow::bail!("Subscription failed: {message}");
            }
            _ => {
                anyhow::bail!("Unexpected subscription response");
            }
        }
    }

    /// Handle a response from the daemon
    fn handle_response(&mut self, response: Response) {
        match response {
            Response::StatusUpdate {
                stages_executing,
                stages_pending,
                stages_completed,
                stages_blocked,
            } => {
                self.status = LiveStatus {
                    executing: stages_executing,
                    pending: stages_pending,
                    completed: stages_completed,
                    blocked: stages_blocked,
                };
                self.last_error = None;
            }
            Response::Error { message } => {
                self.last_error = Some(message);
            }
            _ => {}
        }
    }

    /// Render the UI
    fn render(&mut self) -> Result<()> {
        // Extract all data we need before entering the closure
        let spinner = self.spinner_char();
        let status = &self.status;
        let last_error = self.last_error.clone();

        // Pre-compute values for rendering
        let pct = status.progress_pct();
        let total = status.total();
        let completed_count = status.completed.len();

        // Clone the data we need for rendering
        let unified_stages = status.unified_stages();

        // Convert UnifiedStages to Stages for the tree widget
        let stages_for_graph: Vec<Stage> = unified_stages
            .iter()
            .map(unified_stage_to_stage)
            .collect();

        // Estimate total lines (stages + base branch lines for executing/queued)
        let total_lines = unified_stages.iter().fold(0_u16, |acc, s| {
            let base = 1;
            let extra = if matches!(s.status, StageStatus::Executing | StageStatus::Queued) {
                1
            } else {
                0
            };
            acc + base + extra
        });
        self.graph_state.total_lines = total_lines;

        // Extract scroll position for the closure
        let scroll_y = self.graph_state.scroll_y;

        // Build context and elapsed time maps
        let context_pcts = std::collections::HashMap::new();
        let mut elapsed_times = std::collections::HashMap::new();
        for stage in &unified_stages {
            if let (Some(start), StageStatus::Executing) = (stage.started_at, &stage.status) {
                let elapsed = chrono::Utc::now()
                    .signed_duration_since(start)
                    .num_seconds();
                elapsed_times.insert(stage.id.clone(), elapsed);
            }
        }

        self.terminal.draw(|frame| {
            let area = frame.area();

            // Layout with breathing room:
            // - Header with logo (5 lines: 4 logo + 1 progress)
            // - Spacer (1 line)
            // - Execution graph (fixed height for stability)
            // - Spacer (1 line)
            // - Unified stage table (remaining space)
            // - Footer (1 line for keybinds)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),                // Header with logo and progress
                    Constraint::Length(1),                // Spacer
                    Constraint::Length(GRAPH_AREA_HEIGHT), // Execution graph (fixed)
                    Constraint::Length(1),                // Spacer
                    Constraint::Min(6),                   // Unified stage table
                    Constraint::Length(1),                // Footer with keybinds
                ])
                .split(area);

            render_compact_header(frame, chunks[0], spinner, pct, completed_count, total);
            // chunks[1] is spacer - left empty

            // Render tree-based graph area
            render_tree_graph(
                frame,
                chunks[2],
                &stages_for_graph,
                scroll_y,
                &context_pcts,
                &elapsed_times,
            );

            // chunks[3] is spacer - left empty
            render_unified_table(frame, chunks[4], &unified_stages);
            render_compact_footer(frame, chunks[5], &last_error);
        })?;

        // Update viewport height for scroll bounds
        self.graph_state.viewport_height = GRAPH_AREA_HEIGHT.saturating_sub(2); // Account for borders

        Ok(())
    }

    /// Get spinner character for current frame
    fn spinner_char(&self) -> char {
        const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER[self.spinner_frame % SPINNER.len()]
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        // Restore terminal state
        let _ = disable_raw_mode();
        // Disable mouse capture if it was enabled
        if self.mouse_enabled {
            let _ = crossterm::execute!(
                self.terminal.backend_mut(),
                crossterm::event::DisableMouseCapture
            );
        }
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Render compact header with logo and inline progress
fn render_compact_header(
    frame: &mut Frame,
    area: Rect,
    spinner: char,
    pct: f64,
    completed_count: usize,
    total: usize,
) {
    let progress_str = format!("{completed_count}/{total} ({:.0}%)", pct * 100.0);

    // Build lines: logo lines + progress line
    let mut lines: Vec<Line> = crate::LOGO
        .lines()
        .map(|l| Line::from(Span::styled(l, Theme::header())))
        .collect();

    // Add progress line after logo
    lines.push(Line::from(vec![
        Span::styled(format!("   {spinner} "), Theme::header()),
        Span::styled(progress_str, Style::default().fg(StatusColors::COMPLETED)),
        Span::raw(" "),
        Span::styled(progress_bar_compact(pct, 20), Theme::status_completed()),
    ]));

    let header = Paragraph::new(lines);
    frame.render_widget(header, area);
}

/// Create a compact progress bar string
fn progress_bar_compact(pct: f64, width: usize) -> String {
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Convert UnifiedStage to Stage for graph widget compatibility
fn unified_stage_to_stage(us: &UnifiedStage) -> Stage {
    use chrono::Utc;

    Stage {
        id: us.id.clone(),
        name: us.id.clone(),
        description: None,
        status: us.status.clone(),
        dependencies: us.dependencies.clone(),
        parallel_group: None,
        acceptance: vec![],
        setup: vec![],
        files: vec![],
        stage_type: Default::default(),
        plan_id: None,
        worktree: None,
        session: None,
        held: false,
        parent_stage: None,
        child_stages: vec![],
        created_at: us.started_at.unwrap_or_else(Utc::now),
        updated_at: Utc::now(),
        completed_at: us.completed_at,
        close_reason: None,
        auto_merge: None,
        working_dir: None,
        retry_count: 0,
        max_retries: None,
        last_failure_at: None,
        failure_info: None,
        resolved_base: None,
        base_branch: None,
        base_merged_from: vec![],
        outputs: vec![],
        completed_commit: None,
        merged: us.merged,
        merge_conflict: false,
    }
}

/// Render the tree-based execution graph
fn render_tree_graph(
    frame: &mut Frame,
    area: Rect,
    stages: &[Stage],
    scroll_y: u16,
    context_pcts: &std::collections::HashMap<String, f32>,
    elapsed_times: &std::collections::HashMap<String, i64>,
) {
    let graph_block = Block::default()
        .title(" Execution Graph ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(StatusColors::BORDER));

    let inner_area = graph_block.inner(area);
    frame.render_widget(graph_block, area);

    if stages.is_empty() {
        let empty = Paragraph::new(Span::styled("(no stages)", Theme::dimmed()));
        frame.render_widget(empty, inner_area);
        return;
    }

    // Create tree widget with context and elapsed time data
    let tree_widget = TreeWidget::new(stages)
        .context_percentages(context_pcts.clone())
        .elapsed_times(elapsed_times.clone());

    // Build lines and apply scroll offset
    let lines = tree_widget.build_lines();
    let visible_lines: Vec<_> = lines.into_iter().skip(scroll_y as usize).collect();
    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner_area);
}

/// Render unified stage table with all columns
fn render_unified_table(frame: &mut Frame, area: Rect, stages: &[UnifiedStage]) {
    let block = Block::default()
        .title(format!(" Stages ({}) ", stages.len()))
        .title_style(Theme::header())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(StatusColors::BORDER));

    if stages.is_empty() {
        let empty = Paragraph::new("No stages")
            .style(Theme::dimmed())
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["", "Lvl", "ID", "Deps", "Status", "Merged", "Elapsed"])
        .style(Theme::header())
        .bottom_margin(1);

    let rows: Vec<Row> = stages
        .iter()
        .map(|stage| {
            let icon = status_indicator(&stage.status);
            let status_str = status_text(&stage.status);
            let merged_str = if stage.merged { "✓" } else { "○" };

            let level_str = stage.level.to_string();

            // Show elapsed time: live for executing, final duration for completed
            let elapsed_str = match (&stage.status, stage.started_at, stage.completed_at) {
                // Executing: show live elapsed time
                (StageStatus::Executing, Some(start), _) => {
                    let elapsed = chrono::Utc::now()
                        .signed_duration_since(start)
                        .num_seconds();
                    format_elapsed(elapsed)
                }
                // Completed/blocked/etc with completed_at: show final duration
                (_, Some(start), Some(end)) => {
                    let elapsed = end.signed_duration_since(start).num_seconds();
                    format_elapsed(elapsed)
                }
                // No timing info available
                _ => "-".to_string(),
            };

            let style = match stage.status {
                StageStatus::Executing => Theme::status_executing(),
                StageStatus::Completed => Theme::status_completed(),
                StageStatus::Blocked | StageStatus::MergeConflict | StageStatus::MergeBlocked => {
                    Theme::status_blocked()
                }
                StageStatus::NeedsHandoff
                | StageStatus::WaitingForInput
                | StageStatus::CompletedWithFailures => Theme::status_warning(),
                StageStatus::Queued => Theme::status_queued(),
                _ => Theme::dimmed(),
            };

            let deps_str = format_dependencies(&stage.dependencies, 20);

            Row::new(vec![
                icon.content.to_string(),
                level_str,
                stage.id.clone(),
                deps_str,
                status_str.to_string(),
                merged_str.to_string(),
                elapsed_str,
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(2),  // Icon
        Constraint::Length(3),  // Level
        Constraint::Min(15),    // ID
        Constraint::Length(20), // Deps
        Constraint::Length(10), // Status
        Constraint::Length(6),  // Merged
        Constraint::Length(8),  // Elapsed
    ];

    let table = Table::new(rows, widths).block(block).header(header);
    frame.render_widget(table, area);
}

/// Render compact footer with keybinds
fn render_compact_footer(frame: &mut Frame, area: Rect, last_error: &Option<String>) {
    let line = if let Some(ref err) = last_error {
        Line::from(vec![
            Span::styled("Error: ", Style::default().fg(StatusColors::BLOCKED)),
            Span::styled(err.as_str(), Style::default().fg(StatusColors::BLOCKED)),
        ])
    } else {
        Line::from(vec![
            Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" quit │ "),
            Span::styled("↑↓←→", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" scroll │ "),
            Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" page │ "),
            Span::styled("Home/End", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" jump"),
        ])
    };

    let footer = Paragraph::new(line);
    frame.render_widget(footer, area);
}

/// Format elapsed time in human-readable format
fn format_elapsed(seconds: i64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m{}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h{}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

/// Format dependencies as "(dep1, dep2, ...)" with middle truncation if too long
fn format_dependencies(deps: &[String], max_width: usize) -> String {
    if deps.is_empty() {
        return "-".to_string();
    }

    let inner = deps.join(", ");
    let full = format!("({inner})");

    if full.len() <= max_width {
        return full;
    }

    // Need to truncate - use middle truncation with "..."
    // Reserve space for "(" + "..." + ")" = 5 chars
    if max_width <= 5 {
        return "...".to_string();
    }

    let available = max_width - 5; // for "(" + "..." + ")"
    let left_len = available.div_ceil(2); // slightly favor left side
    let right_len = available / 2;

    let left: String = inner.chars().take(left_len).collect();
    let right: String = inner.chars().skip(inner.len().saturating_sub(right_len)).collect();

    format!("({left}...{right})")
}

/// Entry point for TUI live mode
pub fn run_tui(work_path: &Path) -> Result<()> {
    let mut app = TuiApp::new()?;
    app.run(work_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_state_default() {
        let state = GraphState::default();
        assert_eq!(state.scroll_y, 0);
        assert_eq!(state.total_lines, 0);
        assert_eq!(state.viewport_height, 0);
    }

    #[test]
    fn test_graph_state_scroll_by() {
        let mut state = GraphState {
            scroll_y: 5,
            total_lines: 20,
            viewport_height: 10,
        };

        // Scroll down
        state.scroll_by(3);
        assert_eq!(state.scroll_y, 8);

        // Scroll up
        state.scroll_by(-3);
        assert_eq!(state.scroll_y, 5);

        // Scroll beyond bounds should clamp
        state.scroll_by(100);
        assert_eq!(state.scroll_y, 10); // total_lines - viewport_height = 20 - 10 = 10

        // Scroll negative beyond bounds
        state.scroll_by(-100);
        assert_eq!(state.scroll_y, 0);
    }

    #[test]
    fn test_graph_state_scroll_to_start_end() {
        let mut state = GraphState {
            scroll_y: 5,
            total_lines: 20,
            viewport_height: 10,
        };

        state.scroll_to_end();
        assert_eq!(state.scroll_y, 10); // 20 - 10 = 10

        state.scroll_to_start();
        assert_eq!(state.scroll_y, 0);
    }

    #[test]
    fn test_unified_stage_to_stage_conversion() {
        let unified = UnifiedStage {
            id: "test-stage".to_string(),
            status: StageStatus::Executing,
            merged: true,
            started_at: Some(chrono::Utc::now()),
            completed_at: None,
            level: 2,
            dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        };

        let stage = unified_stage_to_stage(&unified);

        assert_eq!(stage.id, "test-stage");
        assert_eq!(stage.status, StageStatus::Executing);
        assert!(stage.merged);
        assert_eq!(stage.dependencies, vec!["dep1".to_string(), "dep2".to_string()]);
    }

    #[test]
    fn test_live_status_progress() {
        let mut status = LiveStatus::default();
        assert_eq!(status.total(), 0);
        assert_eq!(status.progress_pct(), 0.0);

        status.pending = vec![
            StageInfo {
                id: "a".to_string(),
                name: "Stage A".to_string(),
                session_pid: None,
                started_at: chrono::Utc::now(),
                completed_at: None,
                worktree_status: None,
                status: StageStatus::WaitingForDeps,
                merged: false,
                dependencies: vec![],
            },
        ];
        status.completed = vec![
            StageInfo {
                id: "b".to_string(),
                name: "Stage B".to_string(),
                session_pid: None,
                started_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
                worktree_status: None,
                status: StageStatus::Completed,
                merged: true,
                dependencies: vec![],
            },
        ];

        assert_eq!(status.total(), 2);
        assert_eq!(status.progress_pct(), 0.5); // 1/2 completed
    }

    #[test]
    fn test_live_status_compute_levels() {
        let status = LiveStatus {
            executing: vec![],
            pending: vec![
                StageInfo {
                    id: "a".to_string(),
                    name: "A".to_string(),
                    session_pid: None,
                    started_at: chrono::Utc::now(),
                    completed_at: None,
                    worktree_status: None,
                    status: StageStatus::WaitingForDeps,
                    merged: false,
                    dependencies: vec![],
                },
                StageInfo {
                    id: "b".to_string(),
                    name: "B".to_string(),
                    session_pid: None,
                    started_at: chrono::Utc::now(),
                    completed_at: None,
                    worktree_status: None,
                    status: StageStatus::WaitingForDeps,
                    merged: false,
                    dependencies: vec!["a".to_string()],
                },
                StageInfo {
                    id: "c".to_string(),
                    name: "C".to_string(),
                    session_pid: None,
                    started_at: chrono::Utc::now(),
                    completed_at: None,
                    worktree_status: None,
                    status: StageStatus::WaitingForDeps,
                    merged: false,
                    dependencies: vec!["a".to_string(), "b".to_string()],
                },
            ],
            completed: vec![],
            blocked: vec![],
        };

        let levels = status.compute_levels();

        assert_eq!(levels.get("a"), Some(&0)); // no deps
        assert_eq!(levels.get("b"), Some(&1)); // depends on a
        assert_eq!(levels.get("c"), Some(&2)); // depends on a and b
    }

    #[test]
    fn test_format_elapsed() {
        assert_eq!(format_elapsed(30), "30s");
        assert_eq!(format_elapsed(90), "1m30s");
        assert_eq!(format_elapsed(3661), "1h1m");
    }

    #[test]
    fn test_format_dependencies() {
        // Empty deps
        let empty: Vec<String> = vec![];
        assert_eq!(format_dependencies(&empty, 20), "-");

        // Single dep that fits
        let single = vec!["stage-a".to_string()];
        assert_eq!(format_dependencies(&single, 20), "(stage-a)");

        // Multiple deps that fit
        let multi = vec!["a".to_string(), "b".to_string()];
        assert_eq!(format_dependencies(&multi, 20), "(a, b)");

        // Deps that need truncation
        let long = vec![
            "knowledge-bootstrap".to_string(),
            "implement-feature".to_string(),
        ];
        let result = format_dependencies(&long, 20);
        assert!(result.starts_with('('));
        assert!(result.ends_with(')'));
        assert!(result.contains("..."));
        assert!(result.len() <= 20);

        // Very small max_width
        let tiny_result = format_dependencies(&long, 5);
        assert_eq!(tiny_result, "...");
    }

    /// Helper to check is_socket_disconnected logic without creating a full TuiApp
    fn check_disconnect_for_io_error(kind: std::io::ErrorKind) -> bool {
        let io_err = std::io::Error::new(kind, "test error");
        let error = anyhow::Error::new(io_err).context("Failed to read message length");

        // Replicate the logic from is_socket_disconnected
        for cause in error.chain() {
            if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
                match io_err.kind() {
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                        return false;
                    }
                    std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionAborted => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    #[test]
    fn test_is_socket_disconnected_timeout_not_disconnect() {
        // WouldBlock (timeout) should NOT be treated as a disconnect
        assert!(!check_disconnect_for_io_error(std::io::ErrorKind::WouldBlock));
        assert!(!check_disconnect_for_io_error(std::io::ErrorKind::TimedOut));
    }

    #[test]
    fn test_is_socket_disconnected_real_disconnect() {
        // Real disconnection errors SHOULD be treated as disconnects
        assert!(check_disconnect_for_io_error(std::io::ErrorKind::UnexpectedEof));
        assert!(check_disconnect_for_io_error(std::io::ErrorKind::ConnectionReset));
        assert!(check_disconnect_for_io_error(std::io::ErrorKind::BrokenPipe));
        assert!(check_disconnect_for_io_error(std::io::ErrorKind::ConnectionAborted));
    }

    #[test]
    fn test_is_socket_disconnected_other_errors() {
        // Other errors should NOT be treated as disconnects (fallthrough case)
        assert!(!check_disconnect_for_io_error(std::io::ErrorKind::PermissionDenied));
        assert!(!check_disconnect_for_io_error(std::io::ErrorKind::NotFound));
    }
}
