//! Output panel
//!
//! Displays stdout/stderr from program execution with timestamps,
//! color-coded output, and clickable error source locations.

use chrono::Local;
use iced::widget::{button, container, row, scrollable, text, Column, Row};
use iced::{Color, Element, Length, Theme};
use regex::Regex;

/// A source location in code (file:line:column)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

impl SourceLocation {
    /// Parse a source location from an error message
    /// Supports formats like:
    /// - "file.strat:10:5" (file:line:column)
    /// - "file.strat:10" (file:line)
    /// - "at main.strat:15:8"
    /// - "Error at file.strat:10:5:"
    pub fn parse(text: &str) -> Option<Self> {
        // Look for patterns like "file.strat:10:5" or "file.strat:10"
        let patterns = [
            // "at file.strat:10:5" or "at file.strat:10"
            Regex::new(r"at\s+([^\s:]+\.strat):(\d+)(?::(\d+))?").ok()?,
            // "file.strat:10:5:" or "file.strat:10:"
            Regex::new(r"([^\s:]+\.strat):(\d+)(?::(\d+))?:?").ok()?,
        ];

        for re in &patterns {
            if let Some(caps) = re.captures(text) {
                let file = caps.get(1)?.as_str().to_string();
                let line: u32 = caps.get(2)?.as_str().parse().ok()?;
                let column: Option<u32> = caps.get(3).map(|m| m.as_str().parse().ok()).flatten();
                return Some(Self { file, line, column });
            }
        }
        None
    }

    /// Format as "file:line:column" or "file:line"
    pub fn to_string(&self) -> String {
        match self.column {
            Some(col) => format!("{}:{}:{}", self.file, self.line, col),
            None => format!("{}:{}", self.file, self.line),
        }
    }
}

/// A line of output with optional styling
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub text: String,
    pub kind: OutputKind,
    pub timestamp: String,
    /// Parsed source location for error messages
    pub source_location: Option<SourceLocation>,
}

impl OutputLine {
    /// Create a new output line with current timestamp
    pub fn new(text: String, kind: OutputKind) -> Self {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        let source_location = if kind == OutputKind::Stderr {
            SourceLocation::parse(&text)
        } else {
            None
        };
        Self {
            text,
            kind,
            timestamp,
            source_location,
        }
    }
}

/// Type of output line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Stdout,
    Stderr,
    Info,
    Success,
}

impl OutputKind {
    /// Get the color for this output kind
    pub fn color(&self) -> Color {
        match self {
            Self::Stdout => Color::WHITE,
            Self::Stderr => Color::from_rgb(1.0, 0.4, 0.4),
            Self::Info => Color::from_rgb(0.6, 0.6, 0.6),
            Self::Success => Color::from_rgb(0.4, 1.0, 0.4),
        }
    }

    /// Get the prefix label for this output kind
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Stdout => "",
            Self::Stderr => "[ERR] ",
            Self::Info => "[INF] ",
            Self::Success => "[OK] ",
        }
    }
}

/// Messages for the output panel
#[derive(Debug, Clone)]
pub enum OutputMessage {
    /// Clear all output
    Clear,
    /// Copy all output to clipboard
    CopyToClipboard,
    /// Toggle timestamp display
    ToggleTimestamps,
    /// User clicked on an error with a source location
    JumpToSource(SourceLocation),
}

/// Output panel for displaying program output
#[derive(Debug)]
pub struct OutputPanel {
    pub lines: Vec<OutputLine>,
    pub show_timestamps: bool,
}

impl Default for OutputPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputPanel {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            show_timestamps: false,
        }
    }

    /// Handle output panel messages
    /// Returns Some(SourceLocation) if user clicked on an error to jump to
    pub fn update(&mut self, message: OutputMessage) -> Option<SourceLocation> {
        match message {
            OutputMessage::Clear => {
                self.lines.clear();
                None
            }
            OutputMessage::CopyToClipboard => {
                let content = self.to_clipboard_text();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(content);
                }
                None
            }
            OutputMessage::ToggleTimestamps => {
                self.show_timestamps = !self.show_timestamps;
                None
            }
            OutputMessage::JumpToSource(location) => Some(location),
        }
    }

    /// Add a line of output (splits on newlines)
    pub fn push(&mut self, text: String, kind: OutputKind) {
        // Split multi-line output into separate lines for proper display
        // This is important for DataFrames and Cubes which produce table output
        for line in text.lines() {
            self.lines.push(OutputLine::new(line.to_string(), kind));
        }
        // If the text ends with a newline, the last line is empty - don't add it
        // But if the text is completely empty, add one empty line
        if text.is_empty() {
            self.lines.push(OutputLine::new(String::new(), kind));
        }
    }

    /// Add a single line of output (no newline splitting)
    pub fn push_line(&mut self, text: String, kind: OutputKind) {
        self.lines.push(OutputLine::new(text, kind));
    }

    /// Add stdout output (supports multi-line)
    pub fn stdout(&mut self, text: String) {
        self.push(text, OutputKind::Stdout);
    }

    /// Add stderr output (supports multi-line)
    pub fn stderr(&mut self, text: String) {
        self.push(text, OutputKind::Stderr);
    }

    /// Add info message (supports multi-line)
    pub fn info(&mut self, text: String) {
        self.push(text, OutputKind::Info);
    }

    /// Add success message (supports multi-line)
    pub fn success(&mut self, text: String) {
        self.push(text, OutputKind::Success);
    }

    /// Clear all output
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Get all output as plain text for clipboard
    fn to_clipboard_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                if self.show_timestamps {
                    format!("[{}] {}{}", line.timestamp, line.kind.prefix(), line.text)
                } else {
                    format!("{}{}", line.kind.prefix(), line.text)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render the output panel header with action buttons
    fn view_header(&self) -> Element<'_, OutputMessage> {
        let timestamp_button = button(text(if self.show_timestamps { "Hide Time" } else { "Show Time" }).size(10))
            .on_press(OutputMessage::ToggleTimestamps)
            .padding([2, 6])
            .style(button::text);

        let copy_button = button(text("Copy").size(10))
            .on_press(OutputMessage::CopyToClipboard)
            .padding([2, 6])
            .style(button::text);

        let clear_button = button(text("Clear").size(10))
            .on_press(OutputMessage::Clear)
            .padding([2, 6])
            .style(button::text);

        let line_count = text(format!("{} lines", self.lines.len())).size(10);

        container(
            row![line_count, Row::new(), timestamp_button, copy_button, clear_button]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        )
        .padding([2, 6])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                ..Default::default()
            }
        })
        .into()
    }

    /// Render a single output line
    fn view_line(&self, line: &OutputLine) -> Element<'_, OutputMessage> {
        let color = line.kind.color();

        // Build the display text
        let display = if self.show_timestamps {
            format!("[{}] {}{}", line.timestamp, line.kind.prefix(), line.text)
        } else {
            format!("{}{}", line.kind.prefix(), line.text)
        };

        // If this line has a source location, make it clickable
        if let Some(ref location) = line.source_location {
            let loc = location.clone();
            button(text(display).size(12).color(color).font(iced::Font::MONOSPACE))
                .on_press(OutputMessage::JumpToSource(loc))
                .padding(0)
                .style(move |theme: &Theme, status| {
                    let palette = theme.extended_palette();
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(palette.background.weak.color.into()),
                            text_color: Color::from_rgb(0.4, 0.6, 1.0), // Blue underline effect
                            ..Default::default()
                        },
                        _ => button::Style {
                            background: None,
                            text_color: color,
                            ..Default::default()
                        },
                    }
                })
                .into()
        } else {
            text(display)
                .size(12)
                .color(color)
                .font(iced::Font::MONOSPACE)
                .into()
        }
    }

    /// Render the output panel
    pub fn view(&self) -> Element<'_, OutputMessage> {
        let header = self.view_header();

        let content: Element<'_, OutputMessage> = if self.lines.is_empty() {
            container(
                text("Output will appear here...")
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            )
            .padding(10)
            .into()
        } else {
            let items: Vec<Element<'_, OutputMessage>> =
                self.lines.iter().map(|line| self.view_line(line)).collect();

            scrollable(Column::with_children(items).spacing(2).padding(4))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let body = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.strong.color.into()),
                    ..Default::default()
                }
            });

        Column::new()
            .push(header)
            .push(body)
            .width(Length::Fill)
            .height(Length::FillPortion(2))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_panel_creation() {
        let panel = OutputPanel::new();
        assert!(panel.lines.is_empty());
        assert!(!panel.show_timestamps);
    }

    #[test]
    fn test_push_stdout() {
        let mut panel = OutputPanel::new();
        panel.stdout("Hello, world!".to_string());
        assert_eq!(panel.lines.len(), 1);
        assert_eq!(panel.lines[0].text, "Hello, world!");
        assert_eq!(panel.lines[0].kind, OutputKind::Stdout);
    }

    #[test]
    fn test_push_stderr() {
        let mut panel = OutputPanel::new();
        panel.stderr("Error occurred".to_string());
        assert_eq!(panel.lines.len(), 1);
        assert_eq!(panel.lines[0].kind, OutputKind::Stderr);
    }

    #[test]
    fn test_multiline_output() {
        let mut panel = OutputPanel::new();
        // Simulate DataFrame table output
        let table = "| col1 | col2 |\n|------|------|\n| val1 | val2 |";
        panel.stdout(table.to_string());
        assert_eq!(panel.lines.len(), 3);
        assert_eq!(panel.lines[0].text, "| col1 | col2 |");
        assert_eq!(panel.lines[1].text, "|------|------|");
        assert_eq!(panel.lines[2].text, "| val1 | val2 |");
    }

    #[test]
    fn test_push_line_no_split() {
        let mut panel = OutputPanel::new();
        panel.push_line("line1\nline2".to_string(), OutputKind::Stdout);
        assert_eq!(panel.lines.len(), 1);
        assert!(panel.lines[0].text.contains('\n'));
    }

    #[test]
    fn test_clear() {
        let mut panel = OutputPanel::new();
        panel.stdout("Line 1".to_string());
        panel.stdout("Line 2".to_string());
        assert_eq!(panel.lines.len(), 2);

        panel.update(OutputMessage::Clear);
        assert!(panel.lines.is_empty());
    }

    #[test]
    fn test_toggle_timestamps() {
        let mut panel = OutputPanel::new();
        assert!(!panel.show_timestamps);

        panel.update(OutputMessage::ToggleTimestamps);
        assert!(panel.show_timestamps);

        panel.update(OutputMessage::ToggleTimestamps);
        assert!(!panel.show_timestamps);
    }

    #[test]
    fn test_source_location_parse() {
        // Test "file.strat:10:5" format
        let loc = SourceLocation::parse("Error at main.strat:15:8");
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.file, "main.strat");
        assert_eq!(loc.line, 15);
        assert_eq!(loc.column, Some(8));

        // Test "file.strat:10" format (no column)
        let loc = SourceLocation::parse("main.strat:20: undefined variable");
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.file, "main.strat");
        assert_eq!(loc.line, 20);
        assert_eq!(loc.column, None);

        // Test no match
        let loc = SourceLocation::parse("Just a regular error message");
        assert!(loc.is_none());
    }

    #[test]
    fn test_error_with_source_location() {
        let mut panel = OutputPanel::new();
        panel.stderr("Error at main.strat:15:8: undefined variable 'x'".to_string());

        assert_eq!(panel.lines.len(), 1);
        assert!(panel.lines[0].source_location.is_some());
        let loc = panel.lines[0].source_location.as_ref().unwrap();
        assert_eq!(loc.file, "main.strat");
        assert_eq!(loc.line, 15);
    }

    #[test]
    fn test_clipboard_text() {
        let mut panel = OutputPanel::new();
        panel.stdout("Line 1".to_string());
        panel.stderr("Error".to_string());
        panel.info("Info".to_string());
        panel.success("Done".to_string());

        let text = panel.to_clipboard_text();
        assert!(text.contains("Line 1"));
        assert!(text.contains("[ERR] Error"));
        assert!(text.contains("[INF] Info"));
        assert!(text.contains("[OK] Done"));
    }

    #[test]
    fn test_output_kind_colors() {
        assert_eq!(OutputKind::Stdout.color(), Color::WHITE);
        assert_eq!(OutputKind::Stderr.color(), Color::from_rgb(1.0, 0.4, 0.4));
        assert_eq!(OutputKind::Success.color(), Color::from_rgb(0.4, 1.0, 0.4));
    }
}
