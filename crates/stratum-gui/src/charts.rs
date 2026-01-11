//! Chart widgets for Stratum GUI
//!
//! This module provides chart components (BarChart, LineChart, PieChart) that
//! use iced's native canvas widget for rendering.

use std::f32::consts::PI;

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::callback::CallbackId;

/// Default colors for chart series
pub const CHART_COLORS: [(u8, u8, u8); 10] = [
    (66, 133, 244),  // Blue
    (234, 67, 53),   // Red
    (251, 188, 5),   // Yellow
    (52, 168, 83),   // Green
    (154, 160, 166), // Gray
    (255, 112, 67),  // Orange
    (156, 39, 176),  // Purple
    (0, 188, 212),   // Cyan
    (139, 195, 74),  // Light Green
    (121, 85, 72),   // Brown
];

/// Get a consistent color index for a given label string.
/// Uses a simple hash to ensure the same label always gets the same color.
#[must_use]
pub fn color_index_for_label(label: &str) -> usize {
    // Simple string hash - consistent across renders
    let hash: usize = label.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    hash % CHART_COLORS.len()
}

/// Get a color for a given label string.
#[must_use]
pub fn color_for_label(label: &str) -> Color {
    let idx = color_index_for_label(label);
    let (r, g, b) = CHART_COLORS[idx];
    Color::from_rgb8(r, g, b)
}

/// A single data point with a label and value
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// Label for this data point (e.g., category name, x-axis label)
    pub label: String,
    /// Numeric value
    pub value: f64,
}

impl DataPoint {
    /// Create a new data point
    #[must_use]
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

/// A series of data points for multi-series charts
#[derive(Debug, Clone)]
pub struct DataSeries {
    /// Name of this series
    pub name: String,
    /// Values in this series
    pub values: Vec<f64>,
}

impl DataSeries {
    /// Create a new data series
    #[must_use]
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}

/// Bar chart configuration
#[derive(Debug, Clone)]
pub struct BarChartConfig {
    /// Chart title
    pub title: Option<String>,
    /// Data points (label, value)
    pub data: Vec<DataPoint>,
    /// Chart width in pixels
    pub width: f32,
    /// Chart height in pixels
    pub height: f32,
    /// Whether to show the legend
    pub show_legend: bool,
    /// Whether to show grid lines
    pub show_grid: bool,
    /// Whether to show value labels on bars
    pub show_values: bool,
    /// Bar color (uses default if None)
    pub bar_color: Option<(u8, u8, u8)>,
    /// Callback when a bar is clicked
    pub on_bar_click: Option<CallbackId>,
    /// X-axis label
    pub x_label: Option<String>,
    /// Y-axis label
    pub y_label: Option<String>,
}

impl Default for BarChartConfig {
    fn default() -> Self {
        Self {
            title: None,
            data: Vec::new(),
            width: 400.0,
            height: 300.0,
            show_legend: false,
            show_grid: true,
            show_values: true,
            bar_color: None,
            on_bar_click: None,
            x_label: None,
            y_label: None,
        }
    }
}

/// Line chart configuration
#[derive(Debug, Clone)]
pub struct LineChartConfig {
    /// Chart title
    pub title: Option<String>,
    /// X-axis labels
    pub labels: Vec<String>,
    /// Data series
    pub series: Vec<DataSeries>,
    /// Chart width in pixels
    pub width: f32,
    /// Chart height in pixels
    pub height: f32,
    /// Whether to show the legend
    pub show_legend: bool,
    /// Whether to show grid lines
    pub show_grid: bool,
    /// Whether to show data points
    pub show_points: bool,
    /// Whether to fill area under the line
    pub fill_area: bool,
    /// Custom series colors
    pub series_colors: Vec<(u8, u8, u8)>,
    /// Callback when a point is clicked
    pub on_point_click: Option<CallbackId>,
    /// X-axis label
    pub x_label: Option<String>,
    /// Y-axis label
    pub y_label: Option<String>,
}

impl Default for LineChartConfig {
    fn default() -> Self {
        Self {
            title: None,
            labels: Vec::new(),
            series: Vec::new(),
            width: 400.0,
            height: 300.0,
            show_legend: true,
            show_grid: true,
            show_points: true,
            fill_area: false,
            series_colors: Vec::new(),
            on_point_click: None,
            x_label: None,
            y_label: None,
        }
    }
}

/// Pie chart configuration
#[derive(Debug, Clone)]
pub struct PieChartConfig {
    /// Chart title
    pub title: Option<String>,
    /// Data points (label, value)
    pub data: Vec<DataPoint>,
    /// Chart width in pixels
    pub width: f32,
    /// Chart height in pixels
    pub height: f32,
    /// Whether to show the legend
    pub show_legend: bool,
    /// Whether to show percentage labels
    pub show_percentages: bool,
    /// Whether to show value labels
    pub show_values: bool,
    /// Custom slice colors
    pub slice_colors: Vec<(u8, u8, u8)>,
    /// Callback when a slice is clicked
    pub on_slice_click: Option<CallbackId>,
    /// Inner radius for donut chart (0.0 for regular pie)
    pub inner_radius_ratio: f32,
}

impl Default for PieChartConfig {
    fn default() -> Self {
        Self {
            title: None,
            data: Vec::new(),
            width: 400.0,
            height: 300.0,
            show_legend: true,
            show_percentages: true,
            show_values: false,
            slice_colors: Vec::new(),
            on_slice_click: None,
            inner_radius_ratio: 0.0,
        }
    }
}

/// Canvas program for rendering bar charts
#[derive(Debug)]
pub struct BarChartProgram {
    pub config: BarChartConfig,
}

impl canvas::Program<crate::runtime::Message> for BarChartProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let config = &self.config;
        let data = &config.data;

        if data.is_empty() {
            // Draw "No data" text
            let text = Text {
                content: "No data".to_string(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: Color::from_rgb(0.5, 0.5, 0.5),
                size: 16.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
            return vec![frame.into_geometry()];
        }

        // Chart margins
        let margin_left = 60.0;
        let margin_right = 20.0;
        let margin_top = if config.title.is_some() { 40.0 } else { 20.0 };
        let margin_bottom = 50.0;

        let chart_width = bounds.width - margin_left - margin_right;
        let chart_height = bounds.height - margin_top - margin_bottom;

        // Draw title
        if let Some(ref title) = config.title {
            let text = Text {
                content: title.clone(),
                position: Point::new(bounds.width / 2.0, 20.0),
                color: Color::BLACK,
                size: 18.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        // Find data range
        let max_value = data.iter().map(|d| d.value).fold(0.0_f64, f64::max);
        let max_value = if max_value <= 0.0 { 1.0 } else { max_value };

        // Draw grid lines if enabled
        if config.show_grid {
            let grid_color = Color::from_rgb(0.9, 0.9, 0.9);
            let num_grid_lines = 5;

            for i in 0..=num_grid_lines {
                let y = margin_top + chart_height * (1.0 - i as f32 / num_grid_lines as f32);
                let line = Path::line(
                    Point::new(margin_left, y),
                    Point::new(margin_left + chart_width, y),
                );
                frame.stroke(
                    &line,
                    Stroke::default().with_color(grid_color).with_width(1.0),
                );

                // Y-axis labels
                let value = max_value * i as f64 / num_grid_lines as f64;
                let label = if value >= 1000.0 {
                    format!("{:.1}k", value / 1000.0)
                } else {
                    format!("{:.0}", value)
                };
                let text = Text {
                    content: label,
                    position: Point::new(margin_left - 10.0, y),
                    color: Color::from_rgb(0.4, 0.4, 0.4),
                    size: 12.0.into(),
                    align_x: Horizontal::Right.into(),
                    align_y: Vertical::Center.into(),
                    ..Text::default()
                };
                frame.fill_text(text);
            }
        }

        // Draw bars
        let num_bars = data.len();
        let bar_spacing = 10.0;
        let total_spacing = bar_spacing * (num_bars + 1) as f32;
        let bar_width = (chart_width - total_spacing) / num_bars as f32;

        let bar_color = config
            .bar_color
            .map(|(r, g, b)| Color::from_rgb8(r, g, b))
            .unwrap_or_else(|| {
                Color::from_rgb8(CHART_COLORS[0].0, CHART_COLORS[0].1, CHART_COLORS[0].2)
            });

        for (i, point) in data.iter().enumerate() {
            let x = margin_left + bar_spacing + (bar_width + bar_spacing) * i as f32;
            let bar_height = (point.value / max_value) as f32 * chart_height;
            let y = margin_top + chart_height - bar_height;

            // Draw bar
            let bar = Path::rectangle(Point::new(x, y), Size::new(bar_width, bar_height));
            frame.fill(&bar, bar_color);

            // Draw value label if enabled
            if config.show_values && bar_height > 20.0 {
                let value_text = if point.value >= 1000.0 {
                    format!("{:.1}k", point.value / 1000.0)
                } else {
                    format!("{:.0}", point.value)
                };
                let text = Text {
                    content: value_text,
                    position: Point::new(x + bar_width / 2.0, y - 5.0),
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    size: 11.0.into(),
                    align_x: Horizontal::Center.into(),
                    align_y: Vertical::Bottom.into(),
                    ..Text::default()
                };
                frame.fill_text(text);
            }

            // Draw x-axis label
            let label = if point.label.len() > 10 {
                format!("{}...", &point.label[..8])
            } else {
                point.label.clone()
            };
            let text = Text {
                content: label,
                position: Point::new(x + bar_width / 2.0, margin_top + chart_height + 15.0),
                color: Color::from_rgb(0.3, 0.3, 0.3),
                size: 11.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Top.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        // Draw axes
        let axis_color = Color::from_rgb(0.3, 0.3, 0.3);

        // Y-axis
        let y_axis = Path::line(
            Point::new(margin_left, margin_top),
            Point::new(margin_left, margin_top + chart_height),
        );
        frame.stroke(
            &y_axis,
            Stroke::default().with_color(axis_color).with_width(1.5),
        );

        // X-axis
        let x_axis = Path::line(
            Point::new(margin_left, margin_top + chart_height),
            Point::new(margin_left + chart_width, margin_top + chart_height),
        );
        frame.stroke(
            &x_axis,
            Stroke::default().with_color(axis_color).with_width(1.5),
        );

        // Draw axis labels
        if let Some(ref x_label) = config.x_label {
            let text = Text {
                content: x_label.clone(),
                position: Point::new(margin_left + chart_width / 2.0, bounds.height - 5.0),
                color: Color::from_rgb(0.3, 0.3, 0.3),
                size: 12.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Bottom.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        if let Some(ref y_label) = config.y_label {
            let text = Text {
                content: y_label.clone(),
                position: Point::new(15.0, margin_top + chart_height / 2.0),
                color: Color::from_rgb(0.3, 0.3, 0.3),
                size: 12.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        vec![frame.into_geometry()]
    }
}

/// Canvas program for rendering line charts
#[derive(Debug)]
pub struct LineChartProgram {
    pub config: LineChartConfig,
}

impl canvas::Program<crate::runtime::Message> for LineChartProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let config = &self.config;

        if config.series.is_empty() || config.labels.is_empty() {
            let text = Text {
                content: "No data".to_string(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: Color::from_rgb(0.5, 0.5, 0.5),
                size: 16.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
            return vec![frame.into_geometry()];
        }

        // Chart margins
        let margin_left = 60.0;
        let margin_right = if config.show_legend { 120.0 } else { 20.0 };
        let margin_top = if config.title.is_some() { 40.0 } else { 20.0 };
        let margin_bottom = 50.0;

        let chart_width = bounds.width - margin_left - margin_right;
        let chart_height = bounds.height - margin_top - margin_bottom;

        // Draw title
        if let Some(ref title) = config.title {
            let text = Text {
                content: title.clone(),
                position: Point::new(bounds.width / 2.0, 20.0),
                color: Color::BLACK,
                size: 18.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        // Find data range
        let max_value = config
            .series
            .iter()
            .flat_map(|s| s.values.iter())
            .fold(0.0_f64, |acc, &v| f64::max(acc, v));
        let max_value = if max_value <= 0.0 { 1.0 } else { max_value };

        let min_value = config
            .series
            .iter()
            .flat_map(|s| s.values.iter())
            .fold(f64::MAX, |acc, &v| f64::min(acc, v));
        let min_value = min_value.min(0.0);

        let value_range = max_value - min_value;

        // Draw grid lines
        if config.show_grid {
            let grid_color = Color::from_rgb(0.9, 0.9, 0.9);
            let num_grid_lines = 5;

            for i in 0..=num_grid_lines {
                let y = margin_top + chart_height * (1.0 - i as f32 / num_grid_lines as f32);
                let line = Path::line(
                    Point::new(margin_left, y),
                    Point::new(margin_left + chart_width, y),
                );
                frame.stroke(
                    &line,
                    Stroke::default().with_color(grid_color).with_width(1.0),
                );

                // Y-axis labels
                let value = min_value + value_range * i as f64 / num_grid_lines as f64;
                let label = if value.abs() >= 1000.0 {
                    format!("{:.1}k", value / 1000.0)
                } else {
                    format!("{:.0}", value)
                };
                let text = Text {
                    content: label,
                    position: Point::new(margin_left - 10.0, y),
                    color: Color::from_rgb(0.4, 0.4, 0.4),
                    size: 12.0.into(),
                    align_x: Horizontal::Right.into(),
                    align_y: Vertical::Center.into(),
                    ..Text::default()
                };
                frame.fill_text(text);
            }
        }

        let num_points = config.labels.len();
        let point_spacing = if num_points > 1 {
            chart_width / (num_points - 1) as f32
        } else {
            chart_width
        };

        // Draw each series
        for (series_idx, series) in config.series.iter().enumerate() {
            // Use series name for consistent color across re-renders
            let color = if series_idx < config.series_colors.len() {
                let (r, g, b) = config.series_colors[series_idx];
                Color::from_rgb8(r, g, b)
            } else {
                color_for_label(&series.name)
            };

            // Collect points
            let points: Vec<Point> = series
                .values
                .iter()
                .enumerate()
                .map(|(i, &value)| {
                    let x = margin_left
                        + if num_points > 1 {
                            point_spacing * i as f32
                        } else {
                            chart_width / 2.0
                        };
                    let normalized = (value - min_value) / value_range;
                    let y = margin_top + chart_height * (1.0 - normalized as f32);
                    Point::new(x, y)
                })
                .collect();

            // Draw area fill if enabled
            if config.fill_area && points.len() >= 2 {
                let mut fill_color = color;
                fill_color.a = 0.2;

                let fill_path = Path::new(|builder| {
                    builder.move_to(Point::new(points[0].x, margin_top + chart_height));
                    for point in &points {
                        builder.line_to(*point);
                    }
                    builder.line_to(Point::new(
                        points.last().unwrap().x,
                        margin_top + chart_height,
                    ));
                    builder.close();
                });
                frame.fill(&fill_path, fill_color);
            }

            // Draw line
            if points.len() >= 2 {
                for i in 0..points.len() - 1 {
                    let line = Path::line(points[i], points[i + 1]);
                    frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
                }
            }

            // Draw points if enabled
            if config.show_points {
                for point in &points {
                    let circle = Path::circle(*point, 4.0);
                    frame.fill(&circle, color);
                }
            }
        }

        // Draw x-axis labels
        let label_step = if num_points > 10 { num_points / 10 } else { 1 };
        for (i, label) in config.labels.iter().enumerate() {
            if i % label_step != 0 && i != num_points - 1 {
                continue;
            }
            let x = margin_left
                + if num_points > 1 {
                    point_spacing * i as f32
                } else {
                    chart_width / 2.0
                };
            let display_label = if label.len() > 8 {
                format!("{}...", &label[..6])
            } else {
                label.clone()
            };
            let text = Text {
                content: display_label,
                position: Point::new(x, margin_top + chart_height + 15.0),
                color: Color::from_rgb(0.3, 0.3, 0.3),
                size: 11.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Top.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        // Draw axes
        let axis_color = Color::from_rgb(0.3, 0.3, 0.3);

        let y_axis = Path::line(
            Point::new(margin_left, margin_top),
            Point::new(margin_left, margin_top + chart_height),
        );
        frame.stroke(
            &y_axis,
            Stroke::default().with_color(axis_color).with_width(1.5),
        );

        let x_axis = Path::line(
            Point::new(margin_left, margin_top + chart_height),
            Point::new(margin_left + chart_width, margin_top + chart_height),
        );
        frame.stroke(
            &x_axis,
            Stroke::default().with_color(axis_color).with_width(1.5),
        );

        // Draw legend if enabled
        if config.show_legend && !config.series.is_empty() {
            let legend_x = bounds.width - margin_right + 10.0;
            let legend_y = margin_top + 20.0;

            for (i, series) in config.series.iter().enumerate() {
                // Use series name for consistent color across re-renders
                let color = if i < config.series_colors.len() {
                    let (r, g, b) = config.series_colors[i];
                    Color::from_rgb8(r, g, b)
                } else {
                    color_for_label(&series.name)
                };

                let y = legend_y + i as f32 * 20.0;

                // Color box
                let box_path =
                    Path::rectangle(Point::new(legend_x, y - 5.0), Size::new(12.0, 12.0));
                frame.fill(&box_path, color);

                // Series name
                let name = if series.name.len() > 12 {
                    format!("{}...", &series.name[..10])
                } else {
                    series.name.clone()
                };
                let text = Text {
                    content: name,
                    position: Point::new(legend_x + 18.0, y),
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    size: 11.0.into(),
                    align_x: Horizontal::Left.into(),
                    align_y: Vertical::Center.into(),
                    ..Text::default()
                };
                frame.fill_text(text);
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Canvas program for rendering pie charts
#[derive(Debug)]
pub struct PieChartProgram {
    pub config: PieChartConfig,
}

impl canvas::Program<crate::runtime::Message> for PieChartProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let config = &self.config;
        let data = &config.data;

        if data.is_empty() {
            let text = Text {
                content: "No data".to_string(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: Color::from_rgb(0.5, 0.5, 0.5),
                size: 16.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
            return vec![frame.into_geometry()];
        }

        // Chart layout
        let title_height = if config.title.is_some() { 40.0 } else { 10.0 };
        let legend_width = if config.show_legend { 150.0 } else { 0.0 };

        let available_width = bounds.width - legend_width - 20.0;
        let available_height = bounds.height - title_height - 20.0;
        let radius = (available_width.min(available_height) / 2.0 - 20.0).max(50.0);

        let center_x = 20.0 + available_width / 2.0;
        let center_y = title_height + available_height / 2.0;
        let center = Point::new(center_x, center_y);

        // Draw title
        if let Some(ref title) = config.title {
            let text = Text {
                content: title.clone(),
                position: Point::new(center_x, 20.0),
                color: Color::BLACK,
                size: 18.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
        }

        // Calculate total and percentages
        let total: f64 = data.iter().map(|d| d.value.max(0.0)).sum();
        if total <= 0.0 {
            let text = Text {
                content: "No positive values".to_string(),
                position: center,
                color: Color::from_rgb(0.5, 0.5, 0.5),
                size: 16.0.into(),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center.into(),
                ..Text::default()
            };
            frame.fill_text(text);
            return vec![frame.into_geometry()];
        }

        let inner_radius = radius * config.inner_radius_ratio;

        // Draw slices
        let mut start_angle = -PI / 2.0; // Start from top

        for (i, point) in data.iter().enumerate() {
            if point.value <= 0.0 {
                continue;
            }

            let percentage = point.value / total;
            let sweep_angle = (percentage * 2.0 * PI as f64) as f32;

            // Use label-based color for consistency across re-renders
            let color = if i < config.slice_colors.len() {
                let (r, g, b) = config.slice_colors[i];
                Color::from_rgb8(r, g, b)
            } else {
                color_for_label(&point.label)
            };

            // Draw slice using arc path
            let slice = Path::new(|builder| {
                if inner_radius > 0.0 {
                    // Donut chart
                    let outer_start = Point::new(
                        center.x + radius * start_angle.cos(),
                        center.y + radius * start_angle.sin(),
                    );
                    // inner_start/outer_end calculated for arc geometry reference but drawing uses draw_arc
                    let _inner_start = Point::new(
                        center.x + inner_radius * start_angle.cos(),
                        center.y + inner_radius * start_angle.sin(),
                    );
                    let _outer_end = Point::new(
                        center.x + radius * (start_angle + sweep_angle).cos(),
                        center.y + radius * (start_angle + sweep_angle).sin(),
                    );
                    let inner_end = Point::new(
                        center.x + inner_radius * (start_angle + sweep_angle).cos(),
                        center.y + inner_radius * (start_angle + sweep_angle).sin(),
                    );

                    builder.move_to(outer_start);
                    builder.draw_arc(center, radius, start_angle, sweep_angle);
                    builder.line_to(inner_end);
                    builder.draw_arc(
                        center,
                        inner_radius,
                        start_angle + sweep_angle,
                        -sweep_angle,
                    );
                    builder.close();
                } else {
                    // Regular pie
                    builder.move_to(center);
                    builder.line_to(Point::new(
                        center.x + radius * start_angle.cos(),
                        center.y + radius * start_angle.sin(),
                    ));
                    builder.draw_arc(center, radius, start_angle, sweep_angle);
                    builder.close();
                }
            });
            frame.fill(&slice, color);

            // Draw slice border
            frame.stroke(
                &slice,
                Stroke::default().with_color(Color::WHITE).with_width(1.5),
            );

            // Draw label if percentage is significant enough
            if percentage > 0.03 {
                let mid_angle = start_angle + sweep_angle / 2.0;
                let label_radius = if inner_radius > 0.0 {
                    (radius + inner_radius) / 2.0
                } else {
                    radius * 0.65
                };
                let label_pos = Point::new(
                    center.x + label_radius * mid_angle.cos(),
                    center.y + label_radius * mid_angle.sin(),
                );

                let label_text = if config.show_percentages {
                    format!("{:.1}%", percentage * 100.0)
                } else if config.show_values {
                    if point.value >= 1000.0 {
                        format!("{:.1}k", point.value / 1000.0)
                    } else {
                        format!("{:.0}", point.value)
                    }
                } else {
                    String::new()
                };

                if !label_text.is_empty() {
                    let text = Text {
                        content: label_text,
                        position: label_pos,
                        color: Color::WHITE,
                        size: 12.0.into(),
                        align_x: Horizontal::Center.into(),
                        align_y: Vertical::Center.into(),
                        ..Text::default()
                    };
                    frame.fill_text(text);
                }
            }

            start_angle += sweep_angle;
        }

        // Draw legend
        if config.show_legend {
            let legend_x = bounds.width - legend_width + 10.0;
            let legend_y = title_height + 20.0;

            for (i, point) in data.iter().enumerate() {
                if point.value <= 0.0 {
                    continue;
                }

                // Use label-based color for consistency across re-renders
                let color = if i < config.slice_colors.len() {
                    let (r, g, b) = config.slice_colors[i];
                    Color::from_rgb8(r, g, b)
                } else {
                    color_for_label(&point.label)
                };

                let y = legend_y + i as f32 * 22.0;

                // Color box
                let box_path =
                    Path::rectangle(Point::new(legend_x, y - 6.0), Size::new(14.0, 14.0));
                frame.fill(&box_path, color);

                // Label with value
                let percentage = point.value / total * 100.0;
                let label = if point.label.len() > 15 {
                    format!("{}...", &point.label[..12])
                } else {
                    point.label.clone()
                };
                let legend_text = format!("{label} ({percentage:.1}%)");

                let text = Text {
                    content: legend_text,
                    position: Point::new(legend_x + 20.0, y),
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    size: 11.0.into(),
                    align_x: Horizontal::Left.into(),
                    align_y: Vertical::Center.into(),
                    ..Text::default()
                };
                frame.fill_text(text);
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Helper trait for arc drawing in Path builder
trait PathBuilderExt {
    fn draw_arc(&mut self, center: Point, radius: f32, start_angle: f32, sweep_angle: f32);
}

impl PathBuilderExt for canvas::path::Builder {
    fn draw_arc(&mut self, center: Point, radius: f32, start_angle: f32, sweep_angle: f32) {
        // Approximate arc with bezier curves
        // For small angles, use fewer segments
        let num_segments = ((sweep_angle.abs() / (PI / 4.0)).ceil() as usize).max(1);
        let segment_angle = sweep_angle / num_segments as f32;

        for i in 0..num_segments {
            let angle1 = start_angle + segment_angle * i as f32;
            let angle2 = angle1 + segment_angle;

            // Calculate control points for cubic bezier approximation of arc
            let k = 4.0 / 3.0 * (segment_angle / 4.0).tan();

            let p1 = Point::new(
                center.x + radius * angle1.cos(),
                center.y + radius * angle1.sin(),
            );
            let p2 = Point::new(
                center.x + radius * angle2.cos(),
                center.y + radius * angle2.sin(),
            );

            let c1 = Point::new(
                p1.x - k * radius * angle1.sin(),
                p1.y + k * radius * angle1.cos(),
            );
            let c2 = Point::new(
                p2.x + k * radius * angle2.sin(),
                p2.y - k * radius * angle2.cos(),
            );

            self.bezier_curve_to(c1, c2, p2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_point_creation() {
        let point = DataPoint::new("Sales", 100.0);
        assert_eq!(point.label, "Sales");
        assert!((point.value - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_data_series_creation() {
        let series = DataSeries::new("Revenue", vec![10.0, 20.0, 30.0]);
        assert_eq!(series.name, "Revenue");
        assert_eq!(series.values.len(), 3);
    }

    #[test]
    fn test_bar_chart_config_default() {
        let config = BarChartConfig::default();
        assert!(config.title.is_none());
        assert!(config.data.is_empty());
        assert!((config.width - 400.0).abs() < f32::EPSILON);
        assert!((config.height - 300.0).abs() < f32::EPSILON);
        assert!(config.show_grid);
        assert!(config.show_values);
    }

    #[test]
    fn test_line_chart_config_default() {
        let config = LineChartConfig::default();
        assert!(config.title.is_none());
        assert!(config.series.is_empty());
        assert!(config.labels.is_empty());
        assert!(config.show_legend);
        assert!(config.show_grid);
        assert!(config.show_points);
        assert!(!config.fill_area);
    }

    #[test]
    fn test_pie_chart_config_default() {
        let config = PieChartConfig::default();
        assert!(config.title.is_none());
        assert!(config.data.is_empty());
        assert!(config.show_legend);
        assert!(config.show_percentages);
        assert!(!config.show_values);
        assert!((config.inner_radius_ratio - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_chart_colors_count() {
        assert_eq!(CHART_COLORS.len(), 10);
    }
}
