//! Mini-map widget for compressed graph overview.
//!
//! Displays a scaled-down view of the entire execution graph with:
//! - Single-character status-colored node representation
//! - Simple edge lines (no boxes)
//! - Viewport rectangle overlay showing visible area

use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::sugiyama::{LayoutResult, NodePosition};
use super::theme::StatusColors;
use crate::models::stage::StageStatus;

/// Minimap compression scale (1/4 to 1/8 of original).
const DEFAULT_SCALE: f64 = 0.125; // 1/8 scale
const MIN_SCALE: f64 = 0.125;
const MAX_SCALE: f64 = 0.25;

/// Mini-map widget showing compressed graph overview.
pub struct MiniMap<'a> {
    /// Reference to the layout result
    layout: &'a LayoutResult,
    /// Current visible area in the main graph view
    viewport: Rect,
    /// Compression factor (1/4 to 1/8 of original size)
    scale: f64,
    /// Node statuses keyed by stage ID
    statuses: HashMap<String, StageStatus>,
}

impl<'a> MiniMap<'a> {
    /// Create a new minimap widget.
    ///
    /// # Arguments
    /// * `layout` - Reference to the computed layout result
    pub fn new(layout: &'a LayoutResult) -> Self {
        Self {
            layout,
            viewport: Rect::default(),
            scale: DEFAULT_SCALE,
            statuses: HashMap::new(),
        }
    }

    /// Set the viewport rectangle (visible area in main graph).
    pub fn set_viewport(&mut self, rect: Rect) {
        self.viewport = rect;
    }

    /// Set the compression scale factor.
    ///
    /// Value is clamped to [MIN_SCALE, MAX_SCALE] range.
    pub fn set_scale(&mut self, scale: f64) -> &mut Self {
        self.scale = scale.clamp(MIN_SCALE, MAX_SCALE);
        self
    }

    /// Set node statuses for coloring.
    pub fn set_statuses(&mut self, statuses: HashMap<String, StageStatus>) -> &mut Self {
        self.statuses = statuses;
        self
    }

    /// Convert a point in the minimap to scroll coordinates in the main graph.
    ///
    /// # Arguments
    /// * `x` - X coordinate within the minimap area
    /// * `y` - Y coordinate within the minimap area
    /// * `minimap_area` - The Rect where the minimap is rendered
    ///
    /// # Returns
    /// Scroll coordinates (scroll_x, scroll_y) for the main graph view
    pub fn point_to_scroll(&self, x: u16, y: u16, minimap_area: Rect) -> (f64, f64) {
        let bounds = self.layout.bounds();

        // Calculate relative position within minimap
        let rel_x = x.saturating_sub(minimap_area.x) as f64;
        let rel_y = y.saturating_sub(minimap_area.y) as f64;

        // Convert to graph coordinates
        let minimap_width = minimap_area.width as f64;
        let minimap_height = minimap_area.height as f64;

        if minimap_width == 0.0 || minimap_height == 0.0 {
            return (0.0, 0.0);
        }

        let scroll_x = bounds.min_x + (rel_x / minimap_width) * bounds.width();
        let scroll_y = bounds.min_y + (rel_y / minimap_height) * bounds.height();

        (scroll_x, scroll_y)
    }

    /// Get the character representation for a node status.
    fn status_char(status: &StageStatus) -> char {
        match status {
            StageStatus::Completed => '✓',
            StageStatus::Executing => '●',
            StageStatus::Queued => '▶',
            StageStatus::WaitingForDeps => '○',
            StageStatus::Blocked => '✗',
            StageStatus::NeedsHandoff => '⟳',
            StageStatus::WaitingForInput => '?',
            StageStatus::Skipped => '⊘',
            StageStatus::MergeConflict => '⚡',
            StageStatus::CompletedWithFailures => '⚠',
            StageStatus::MergeBlocked => '⊗',
        }
    }

    /// Get the color for a node status.
    fn status_color(status: &StageStatus) -> Color {
        match status {
            StageStatus::Completed => StatusColors::COMPLETED,
            StageStatus::Executing => StatusColors::EXECUTING,
            StageStatus::Queued => StatusColors::QUEUED,
            StageStatus::WaitingForDeps => StatusColors::PENDING,
            StageStatus::Blocked | StageStatus::MergeBlocked => StatusColors::BLOCKED,
            StageStatus::NeedsHandoff => StatusColors::HANDOFF,
            StageStatus::WaitingForInput => StatusColors::WARNING,
            StageStatus::Skipped => StatusColors::DIMMED,
            StageStatus::MergeConflict => StatusColors::CONFLICT,
            StageStatus::CompletedWithFailures => StatusColors::WARNING,
        }
    }

    /// Scale a position from graph coordinates to minimap coordinates.
    fn scale_position(&self, pos: &NodePosition, area: Rect) -> (u16, u16) {
        let bounds = self.layout.bounds();

        if bounds.width() == 0.0 || bounds.height() == 0.0 {
            return (area.x, area.y);
        }

        // Calculate center of node
        let center_x = pos.center_x();
        let center_y = pos.center_y();

        // Normalize to [0, 1]
        let norm_x = (center_x - bounds.min_x) / bounds.width();
        let norm_y = (center_y - bounds.min_y) / bounds.height();

        // Scale to minimap area
        let scaled_x = area.x + (norm_x * (area.width.saturating_sub(1)) as f64) as u16;
        let scaled_y = area.y + (norm_y * (area.height.saturating_sub(1)) as f64) as u16;

        (scaled_x, scaled_y)
    }

    /// Render a node at the given buffer position.
    fn render_node(&self, buf: &mut Buffer, x: u16, y: u16, node_id: &str) {
        let status = self
            .statuses
            .get(node_id)
            .cloned()
            .unwrap_or(StageStatus::WaitingForDeps);

        let ch = Self::status_char(&status);
        let color = Self::status_color(&status);
        let style = Style::default().fg(color);

        if x < buf.area().right() && y < buf.area().bottom() {
            buf[(x, y)].set_char(ch).set_style(style);
        }
    }

    /// Render edges as simple lines.
    fn render_edges(&self, buf: &mut Buffer, area: Rect) {
        let bounds = self.layout.bounds();

        if bounds.width() == 0.0 || bounds.height() == 0.0 {
            return;
        }

        let edge_style = Style::default().fg(StatusColors::GRAPH_EDGE);

        for edge in self.layout.edges() {
            for segment in &edge.segments {
                // Scale segment endpoints
                let x1 = ((segment.x1 - bounds.min_x) / bounds.width()
                    * (area.width.saturating_sub(1)) as f64) as u16
                    + area.x;
                let y1 = ((segment.y1 - bounds.min_y) / bounds.height()
                    * (area.height.saturating_sub(1)) as f64) as u16
                    + area.y;
                let x2 = ((segment.x2 - bounds.min_x) / bounds.width()
                    * (area.width.saturating_sub(1)) as f64) as u16
                    + area.x;
                let y2 = ((segment.y2 - bounds.min_y) / bounds.height()
                    * (area.height.saturating_sub(1)) as f64) as u16
                    + area.y;

                // Draw line using Bresenham-style approach
                self.draw_line(buf, x1, y1, x2, y2, edge_style, area);
            }
        }
    }

    /// Draw a line between two points using simple character drawing.
    #[allow(clippy::too_many_arguments)]
    fn draw_line(&self, buf: &mut Buffer, x1: u16, y1: u16, x2: u16, y2: u16, style: Style, area: Rect) {
        // Simple line drawing - use | for vertical, - for horizontal, and corners
        if x1 == x2 {
            // Vertical line
            let (start_y, end_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            for y in start_y..=end_y {
                if x1 >= area.x && x1 < area.right() && y >= area.y && y < area.bottom() {
                    let cell = &mut buf[(x1, y)];
                    if cell.symbol() == " " {
                        cell.set_char('│').set_style(style);
                    }
                }
            }
        } else if y1 == y2 {
            // Horizontal line
            let (start_x, end_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            for x in start_x..=end_x {
                if x >= area.x && x < area.right() && y1 >= area.y && y1 < area.bottom() {
                    let cell = &mut buf[(x, y1)];
                    if cell.symbol() == " " {
                        cell.set_char('─').set_style(style);
                    }
                }
            }
        } else {
            // Diagonal - draw as dot for simplicity in minimap
            let mid_x = (x1 + x2) / 2;
            let mid_y = (y1 + y2) / 2;
            if mid_x >= area.x && mid_x < area.right() && mid_y >= area.y && mid_y < area.bottom() {
                let cell = &mut buf[(mid_x, mid_y)];
                if cell.symbol() == " " {
                    cell.set_char('·').set_style(style);
                }
            }
        }
    }

    /// Render the viewport rectangle overlay.
    fn render_viewport_overlay(&self, buf: &mut Buffer, area: Rect) {
        if self.viewport.width == 0 || self.viewport.height == 0 {
            return;
        }

        let bounds = self.layout.bounds();
        if bounds.width() == 0.0 || bounds.height() == 0.0 {
            return;
        }

        // Calculate viewport position in minimap coordinates
        let viewport_x = self.viewport.x as f64;
        let viewport_y = self.viewport.y as f64;
        let viewport_w = self.viewport.width as f64;
        let viewport_h = self.viewport.height as f64;

        // Scale viewport to minimap
        let scaled_x = area.x
            + ((viewport_x - bounds.min_x) / bounds.width() * area.width as f64)
                .clamp(0.0, area.width as f64 - 1.0) as u16;
        let scaled_y = area.y
            + ((viewport_y - bounds.min_y) / bounds.height() * area.height as f64)
                .clamp(0.0, area.height as f64 - 1.0) as u16;
        let scaled_w =
            ((viewport_w / bounds.width()) * area.width as f64).clamp(1.0, area.width as f64) as u16;
        let scaled_h = ((viewport_h / bounds.height()) * area.height as f64)
            .clamp(1.0, area.height as f64) as u16;

        let viewport_style = Style::default().fg(StatusColors::QUEUED); // Cyan

        // Draw top border
        for x in scaled_x..scaled_x.saturating_add(scaled_w).min(area.right()) {
            if scaled_y >= area.y && scaled_y < area.bottom() {
                let ch = if x == scaled_x {
                    '┌'
                } else if x == scaled_x.saturating_add(scaled_w).saturating_sub(1) {
                    '┐'
                } else {
                    '─'
                };
                buf[(x, scaled_y)].set_char(ch).set_style(viewport_style);
            }
        }

        // Draw bottom border
        let bottom_y = scaled_y.saturating_add(scaled_h).saturating_sub(1);
        if bottom_y != scaled_y {
            for x in scaled_x..scaled_x.saturating_add(scaled_w).min(area.right()) {
                if bottom_y >= area.y && bottom_y < area.bottom() {
                    let ch = if x == scaled_x {
                        '└'
                    } else if x == scaled_x.saturating_add(scaled_w).saturating_sub(1) {
                        '┘'
                    } else {
                        '─'
                    };
                    buf[(x, bottom_y)].set_char(ch).set_style(viewport_style);
                }
            }
        }

        // Draw left border
        for y in scaled_y.saturating_add(1)..bottom_y {
            if scaled_x >= area.x && scaled_x < area.right() && y >= area.y && y < area.bottom() {
                buf[(scaled_x, y)].set_char('│').set_style(viewport_style);
            }
        }

        // Draw right border
        let right_x = scaled_x.saturating_add(scaled_w).saturating_sub(1);
        if right_x != scaled_x {
            for y in scaled_y.saturating_add(1)..bottom_y {
                if right_x >= area.x && right_x < area.right() && y >= area.y && y < area.bottom() {
                    buf[(right_x, y)].set_char('│').set_style(viewport_style);
                }
            }
        }
    }
}

impl Widget for MiniMap<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.layout.is_empty() {
            return;
        }

        // Render edges first (background layer)
        self.render_edges(buf, area);

        // Render nodes on top of edges
        for (node_id, pos) in self.layout.nodes() {
            let (x, y) = self.scale_position(pos, area);
            self.render_node(buf, x, y, node_id);
        }

        // Render viewport overlay last (foreground)
        self.render_viewport_overlay(buf, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::status::ui::sugiyama::{EdgePath, LayoutBounds, LineSegment};

    fn make_layout(nodes: Vec<(&str, f64, f64)>, edges: Vec<(&str, &str)>) -> LayoutResult {
        let mut node_map = HashMap::new();
        for (id, x, y) in &nodes {
            node_map.insert(
                id.to_string(),
                NodePosition::new(*x, *y, 80.0, 30.0),
            );
        }

        let edge_paths: Vec<EdgePath> = edges
            .iter()
            .map(|(src, tgt)| {
                let src_pos = node_map.get(*src).unwrap();
                let tgt_pos = node_map.get(*tgt).unwrap();
                EdgePath::new(
                    src.to_string(),
                    tgt.to_string(),
                    vec![LineSegment::new(
                        src_pos.center_x(),
                        src_pos.y + src_pos.height,
                        tgt_pos.center_x(),
                        tgt_pos.y,
                    )],
                )
            })
            .collect();

        let bounds = if node_map.is_empty() {
            LayoutBounds::new(0.0, 0.0, 0.0, 0.0)
        } else {
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;
            for pos in node_map.values() {
                min_x = min_x.min(pos.x);
                min_y = min_y.min(pos.y);
                max_x = max_x.max(pos.x + pos.width);
                max_y = max_y.max(pos.y + pos.height);
            }
            LayoutBounds::new(min_x, min_y, max_x, max_y)
        };

        LayoutResult::new(node_map, edge_paths, bounds)
    }

    #[test]
    fn test_minimap_new() {
        let layout = make_layout(vec![("a", 0.0, 0.0)], vec![]);
        let minimap = MiniMap::new(&layout);

        assert_eq!(minimap.scale, DEFAULT_SCALE);
        assert_eq!(minimap.viewport, Rect::default());
    }

    #[test]
    fn test_minimap_set_viewport() {
        let layout = make_layout(vec![("a", 0.0, 0.0)], vec![]);
        let mut minimap = MiniMap::new(&layout);

        let viewport = Rect::new(10, 20, 100, 50);
        minimap.set_viewport(viewport);

        assert_eq!(minimap.viewport, viewport);
    }

    #[test]
    fn test_minimap_set_scale() {
        let layout = make_layout(vec![("a", 0.0, 0.0)], vec![]);
        let mut minimap = MiniMap::new(&layout);

        minimap.set_scale(0.5);
        assert_eq!(minimap.scale, MAX_SCALE); // Clamped to max

        minimap.set_scale(0.05);
        assert_eq!(minimap.scale, MIN_SCALE); // Clamped to min

        minimap.set_scale(0.2);
        assert_eq!(minimap.scale, 0.2); // Within range
    }

    #[test]
    fn test_scaling_accuracy() {
        let layout = make_layout(
            vec![
                ("a", 0.0, 0.0),
                ("b", 100.0, 70.0),
            ],
            vec![("a", "b")],
        );
        let minimap = MiniMap::new(&layout);

        // Test area: 20x10 minimap
        let area = Rect::new(0, 0, 20, 10);

        // Scale node "a" at (0, 0)
        let pos_a = layout.get_node("a").unwrap();
        let (x_a, y_a) = minimap.scale_position(pos_a, area);

        // Node a center is at (40, 15), bounds min is at (0, 0)
        // Should be at relative (0, 0) scaled
        assert!(x_a < 10, "node a should be in left half");
        assert!(y_a < 5, "node a should be in top half");

        // Scale node "b" at (100, 70)
        let pos_b = layout.get_node("b").unwrap();
        let (x_b, y_b) = minimap.scale_position(pos_b, area);

        // Node b center is at (140, 85), which is at right-bottom
        assert!(x_b > 10, "node b should be in right half");
        assert!(y_b > 5, "node b should be in bottom half");
    }

    #[test]
    fn test_viewport_rectangle_positioning() {
        let layout = make_layout(
            vec![
                ("a", 0.0, 0.0),
                ("b", 200.0, 200.0),
            ],
            vec![],
        );
        let mut minimap = MiniMap::new(&layout);

        // Set viewport to cover roughly half the graph
        let viewport = Rect::new(0, 0, 100, 100);
        minimap.set_viewport(viewport);

        // The viewport rectangle should be rendered in the top-left
        // portion of the minimap when rendered
        assert_eq!(minimap.viewport, viewport);
    }

    #[test]
    fn test_point_to_scroll_coordinate_mapping() {
        let layout = make_layout(
            vec![
                ("a", 0.0, 0.0),
                ("b", 100.0, 100.0),
            ],
            vec![],
        );
        let minimap = MiniMap::new(&layout);

        let minimap_area = Rect::new(0, 0, 20, 10);

        // Click at top-left of minimap should map to top-left of graph
        let (scroll_x, scroll_y) = minimap.point_to_scroll(0, 0, minimap_area);
        assert!(scroll_x >= 0.0, "scroll_x should be near graph origin");
        assert!(scroll_y >= 0.0, "scroll_y should be near graph origin");

        // Click at center of minimap
        let (scroll_x, scroll_y) = minimap.point_to_scroll(10, 5, minimap_area);
        let bounds = layout.bounds();
        let mid_x = bounds.min_x + bounds.width() / 2.0;
        let mid_y = bounds.min_y + bounds.height() / 2.0;
        assert!(
            (scroll_x - mid_x).abs() < bounds.width() * 0.2,
            "center click should map near graph center"
        );
        assert!(
            (scroll_y - mid_y).abs() < bounds.height() * 0.2,
            "center click should map near graph center"
        );
    }

    #[test]
    fn test_point_to_scroll_empty_area() {
        let layout = make_layout(vec![("a", 0.0, 0.0)], vec![]);
        let minimap = MiniMap::new(&layout);

        // Zero-size minimap area should return (0, 0)
        let minimap_area = Rect::new(0, 0, 0, 0);
        let (scroll_x, scroll_y) = minimap.point_to_scroll(5, 5, minimap_area);
        assert_eq!(scroll_x, 0.0);
        assert_eq!(scroll_y, 0.0);
    }

    #[test]
    fn test_status_char_mapping() {
        assert_eq!(MiniMap::status_char(&StageStatus::Completed), '✓');
        assert_eq!(MiniMap::status_char(&StageStatus::Executing), '●');
        assert_eq!(MiniMap::status_char(&StageStatus::WaitingForDeps), '○');
        assert_eq!(MiniMap::status_char(&StageStatus::Blocked), '✗');
        assert_eq!(MiniMap::status_char(&StageStatus::Queued), '▶');
    }

    #[test]
    fn test_status_color_mapping() {
        assert_eq!(
            MiniMap::status_color(&StageStatus::Completed),
            StatusColors::COMPLETED
        );
        assert_eq!(
            MiniMap::status_color(&StageStatus::Executing),
            StatusColors::EXECUTING
        );
        assert_eq!(
            MiniMap::status_color(&StageStatus::Blocked),
            StatusColors::BLOCKED
        );
    }

    #[test]
    fn test_minimap_with_statuses() {
        let layout = make_layout(vec![("a", 0.0, 0.0), ("b", 100.0, 70.0)], vec![("a", "b")]);
        let mut minimap = MiniMap::new(&layout);

        let mut statuses = HashMap::new();
        statuses.insert("a".to_string(), StageStatus::Completed);
        statuses.insert("b".to_string(), StageStatus::Executing);
        minimap.set_statuses(statuses);

        assert_eq!(minimap.statuses.len(), 2);
        assert_eq!(
            minimap.statuses.get("a"),
            Some(&StageStatus::Completed)
        );
        assert_eq!(
            minimap.statuses.get("b"),
            Some(&StageStatus::Executing)
        );
    }

    #[test]
    fn test_empty_layout() {
        let layout = make_layout(vec![], vec![]);
        let minimap = MiniMap::new(&layout);

        assert!(minimap.layout.is_empty());
    }
}
