//! REPL panel
//!
//! Provides an interactive REPL at the bottom of the window.
//! Implements Phase 6.5 of the Workshop IDE.

use iced::widget::{column, container, row, scrollable, text, text_input, Column};
use iced::{Element, Length};
use std::cell::RefCell;
use stratum_core::bytecode::Value;
use stratum_core::{with_output_capture, Compiler, Parser, VM};

/// Messages for the REPL panel
#[derive(Debug, Clone)]
pub enum ReplMessage {
    /// Input text changed
    InputChanged(String),
    /// User pressed Enter to submit
    Submit,
    /// Navigate history up (previous command)
    HistoryUp,
    /// Navigate history down (next command)
    HistoryDown,
    /// Clear REPL history
    Clear,
    /// Reset VM state
    Reset,
}

/// A history entry in the REPL
#[derive(Debug, Clone)]
pub struct ReplEntry {
    pub input: String,
    pub output: String,
    pub is_error: bool,
}

/// REPL panel for interactive evaluation
pub struct ReplPanel {
    /// History of inputs and outputs
    pub history: Vec<ReplEntry>,
    /// Current input being typed
    pub current_input: String,
    /// Saved input when navigating history (so user doesn't lose their typing)
    saved_input: String,
    /// Current history navigation index (None = editing new input)
    pub history_index: Option<usize>,
    /// The VM instance for persistent state across evaluations
    /// Wrapped in RefCell since we can't derive Debug for VM
    vm: RefCell<VM>,
    /// Whether we're in multi-line input mode
    multi_line_mode: bool,
    /// Accumulated input for multi-line mode
    accumulated_input: String,
}

impl std::fmt::Debug for ReplPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplPanel")
            .field("history", &self.history)
            .field("current_input", &self.current_input)
            .field("history_index", &self.history_index)
            .field("multi_line_mode", &self.multi_line_mode)
            .finish_non_exhaustive()
    }
}

impl Default for ReplPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplPanel {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_input: String::new(),
            saved_input: String::new(),
            history_index: None,
            vm: RefCell::new(VM::new()),
            multi_line_mode: false,
            accumulated_input: String::new(),
        }
    }

    /// Handle a REPL message
    pub fn update(&mut self, message: ReplMessage) {
        match message {
            ReplMessage::InputChanged(input) => {
                self.current_input = input;
                // Reset history navigation when user types
                if self.history_index.is_some() {
                    self.history_index = None;
                }
            }
            ReplMessage::Submit => {
                self.submit_input();
            }
            ReplMessage::HistoryUp => {
                self.history_up();
            }
            ReplMessage::HistoryDown => {
                self.history_down();
            }
            ReplMessage::Clear => {
                self.clear();
            }
            ReplMessage::Reset => {
                self.reset_vm();
            }
        }
    }

    /// Submit the current input for evaluation
    fn submit_input(&mut self) {
        let input = self.current_input.trim().to_string();
        if input.is_empty() {
            return;
        }

        // Check for REPL commands
        if let Some(CommandResult::Handled(output)) = self.handle_command(&input) {
            self.history.push(ReplEntry {
                input: input.clone(),
                output,
                is_error: false,
            });
            self.current_input.clear();
            self.history_index = None;
            return;
        }

        // Handle multi-line input
        let full_input = if self.multi_line_mode {
            self.accumulated_input.push('\n');
            self.accumulated_input.push_str(&input);
            self.accumulated_input.clone()
        } else {
            input.clone()
        };

        // Check if input is complete (balanced brackets)
        if !is_complete(&full_input) {
            // Need more input - enter multi-line mode
            if !self.multi_line_mode {
                self.multi_line_mode = true;
                self.accumulated_input = input;
            }
            self.current_input.clear();
            return;
        }

        // Input is complete - evaluate it
        let input_for_history = if self.multi_line_mode {
            self.accumulated_input.clone()
        } else {
            input.clone()
        };

        // Reset multi-line state
        self.multi_line_mode = false;
        self.accumulated_input.clear();

        // Evaluate the input
        let (output, is_error) = match self.eval(&full_input) {
            Ok((stdout, value)) => {
                // Combine captured stdout with the result value
                let mut output_parts = stdout;
                if !matches!(value, Value::Null) {
                    output_parts.push(pretty_print(&value));
                }
                (output_parts.join("\n"), false)
            }
            Err(err) => (err, true),
        };

        // Add to history
        self.history.push(ReplEntry {
            input: input_for_history,
            output,
            is_error,
        });

        self.current_input.clear();
        self.history_index = None;
    }

    /// Evaluate a string of Stratum code
    /// Returns (captured_stdout, result_value) or error
    fn eval(&self, input: &str) -> Result<(Vec<String>, Value), String> {
        // Parse the input - supports expressions, statements, and function definitions
        let repl_input = Parser::parse_repl_input(input).map_err(|errors| {
            errors
                .iter()
                .map(|e| format!("Parse error: {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        })?;

        // Compile based on input type
        let function = Compiler::new()
            .compile_repl_input(&repl_input)
            .map_err(|errors| {
                errors
                    .iter()
                    .map(|e| format!("Compile error: {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;

        // Run in the VM with output capture (globals are preserved between runs)
        let (result, captured) = with_output_capture(|| {
            self.vm.borrow_mut().run(function)
        });

        result
            .map(|value| (captured.stdout, value))
            .map_err(|e| format!("Runtime error: {e}"))
    }

    /// Handle REPL commands (starting with :)
    fn handle_command(&mut self, input: &str) -> Option<CommandResult> {
        let trimmed = input.trim();

        if !trimmed.starts_with(':') {
            return None;
        }

        let cmd = trimmed.trim_start_matches(':').trim();
        let (cmd_name, _args) = cmd.split_once(' ').unwrap_or((cmd, ""));

        match cmd_name.to_lowercase().as_str() {
            "help" | "h" | "?" => Some(CommandResult::Handled(HELP_TEXT.to_string())),

            "clear" | "cls" => {
                self.history.clear();
                Some(CommandResult::Handled("History cleared.".to_string()))
            }

            "reset" => {
                *self.vm.borrow_mut() = VM::new();
                Some(CommandResult::Handled(
                    "VM state reset. All variables cleared.".to_string(),
                ))
            }

            "vars" | "globals" => {
                let vm = self.vm.borrow();
                let globals = vm.globals();
                if globals.is_empty() {
                    Some(CommandResult::Handled("No variables defined.".to_string()))
                } else {
                    let vars: Vec<String> = globals
                        .iter()
                        .filter(|(name, _)| !name.starts_with("__"))
                        .map(|(name, value)| format!("  {name}: {}", value.type_name()))
                        .collect();
                    Some(CommandResult::Handled(format!(
                        "Variables:\n{}",
                        vars.join("\n")
                    )))
                }
            }

            _ => Some(CommandResult::Handled(format!(
                "Unknown command: :{cmd_name}\nType :help for available commands"
            ))),
        }
    }

    /// Navigate history up (to older entries)
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Starting navigation - save current input
                self.saved_input = self.current_input.clone();
                self.history_index = Some(self.history.len() - 1);
            }
            Some(0) => {
                // Already at oldest entry
                return;
            }
            Some(i) => {
                self.history_index = Some(i - 1);
            }
        }

        if let Some(idx) = self.history_index {
            self.current_input = self.history[idx].input.clone();
        }
    }

    /// Navigate history down (to newer entries)
    pub fn history_down(&mut self) {
        let Some(idx) = self.history_index else {
            // Not in history navigation mode
            return;
        };

        if idx + 1 >= self.history.len() {
            // Going past the end - restore saved input
            self.history_index = None;
            self.current_input = std::mem::take(&mut self.saved_input);
        } else {
            self.history_index = Some(idx + 1);
            self.current_input = self.history[idx + 1].input.clone();
        }
    }

    /// Clear REPL history
    pub fn clear(&mut self) {
        self.history.clear();
        self.history_index = None;
    }

    /// Reset VM state
    pub fn reset_vm(&mut self) {
        *self.vm.borrow_mut() = VM::new();
    }

    /// Get a copy of the current globals from the VM
    pub fn get_globals(&self) -> std::collections::HashMap<String, Value> {
        self.vm.borrow().globals().clone()
    }

    /// Render the REPL panel
    pub fn view(&self) -> Element<'_, ReplMessage> {
        // Build history view
        let history_view: Element<'_, ReplMessage> = if self.history.is_empty() && !self.multi_line_mode
        {
            container(text("REPL ready. Type expressions to evaluate. Type :help for commands.").size(12))
                .padding(4)
                .into()
        } else {
            let items: Vec<Element<'_, ReplMessage>> = self
                .history
                .iter()
                .flat_map(|entry| {
                    let mut elements: Vec<Element<'_, ReplMessage>> = Vec::new();

                    // Input line(s) with prompt
                    for (i, line) in entry.input.lines().enumerate() {
                        let prompt = if i == 0 { ">>> " } else { "... " };
                        elements.push(
                            text(format!("{prompt}{line}"))
                                .size(12)
                                .font(iced::Font::MONOSPACE)
                                .into(),
                        );
                    }

                    // Output lines (if any)
                    if !entry.output.is_empty() {
                        let output_color = if entry.is_error {
                            iced::Color::from_rgb(1.0, 0.4, 0.4) // Red for errors
                        } else {
                            iced::Color::from_rgb(0.6, 0.8, 0.6) // Green for success
                        };

                        for line in entry.output.lines() {
                            elements.push(
                                text(line)
                                    .size(12)
                                    .font(iced::Font::MONOSPACE)
                                    .color(output_color)
                                    .into(),
                            );
                        }
                    }

                    elements
                })
                .collect();

            scrollable(
                Column::with_children(items)
                    .spacing(1)
                    .padding(4)
                    .width(Length::Fill),
            )
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
        };

        // Show accumulated multi-line input if in multi-line mode
        let pending_lines: Element<'_, ReplMessage> = if self.multi_line_mode && !self.accumulated_input.is_empty() {
            let lines: Vec<Element<'_, ReplMessage>> = self
                .accumulated_input
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    let prompt = if i == 0 { ">>> " } else { "... " };
                    text(format!("{prompt}{line}"))
                        .size(12)
                        .font(iced::Font::MONOSPACE)
                        .into()
                })
                .collect();
            Column::with_children(lines).spacing(1).padding([0, 4]).into()
        } else {
            column![].into()
        };

        // Determine prompt based on mode
        let prompt = if self.multi_line_mode { "... " } else { ">>> " };

        // Input row with prompt and text input
        let input_row: Element<'_, ReplMessage> = row![
            text(prompt).size(12).font(iced::Font::MONOSPACE),
            text_input("", &self.current_input)
                .on_input(ReplMessage::InputChanged)
                .on_submit(ReplMessage::Submit)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .width(Length::Fill)
                .padding(2)
                .style(|theme: &iced::Theme, status| {
                    let palette = theme.extended_palette();
                    let mut style = text_input::default(theme, status);
                    style.background = palette.background.strong.color.into();
                    style.border = iced::Border::default();
                    style
                }),
        ]
        .spacing(0)
        .padding(4)
        .into();

        container(column![history_view, pending_lines, input_row].spacing(2))
            .width(Length::Fill)
            .height(Length::FillPortion(1))
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.strong.color.into()),
                    ..Default::default()
                }
            })
            .into()
    }
}

/// Result of handling a REPL command
enum CommandResult {
    /// Command was handled, with this output
    Handled(String),
    // Note: Continue variant was removed since handle_command always returns
    // Some(Handled(...)) for commands, and None for non-commands
}

/// Check if the input is complete (balanced brackets/braces/parens)
fn is_complete(input: &str) -> bool {
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_string = false;
    let mut in_raw_string = false;
    let mut escape_next = false;

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escape_next {
            escape_next = false;
            i += 1;
            continue;
        }

        if c == '\\' && in_string && !in_raw_string {
            escape_next = true;
            i += 1;
            continue;
        }

        // Check for raw string start (r" or r#")
        if !in_string && !in_raw_string && c == 'r' {
            if i + 1 < chars.len() && chars[i + 1] == '"' {
                in_raw_string = true;
                in_string = true;
                i += 2;
                continue;
            }
        }

        if c == '"' && !in_raw_string {
            in_string = !in_string;
            i += 1;
            continue;
        }

        if c == '"' && in_raw_string {
            in_raw_string = false;
            in_string = false;
            i += 1;
            continue;
        }

        if in_string {
            i += 1;
            continue;
        }

        // Handle line comments
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            // Skip to end of line
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Handle block comments
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            let mut depth = 1;
            while i + 1 < chars.len() && depth > 0 {
                if chars[i] == '/' && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        match c {
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            _ => {}
        }

        i += 1;
    }

    // Input is complete if all brackets are balanced and we're not in a string
    !in_string && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0
}

/// Pretty-print a value for REPL output
fn pretty_print(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{s}\""),
        Value::List(list) => {
            let items = list.borrow();
            if items.is_empty() {
                "[]".to_string()
            } else if items.len() <= 10 {
                let formatted: Vec<String> = items.iter().map(pretty_print).collect();
                format!("[{}]", formatted.join(", "))
            } else {
                // Truncate long lists
                let formatted: Vec<String> = items.iter().take(10).map(pretty_print).collect();
                format!(
                    "[{}, ... ({} more)]",
                    formatted.join(", "),
                    items.len() - 10
                )
            }
        }
        Value::Map(map) => {
            let entries = map.borrow();
            if entries.is_empty() {
                "{}".to_string()
            } else {
                let formatted: Vec<String> = entries
                    .iter()
                    .take(10)
                    .map(|(k, v)| {
                        let key_str = match k {
                            stratum_core::bytecode::HashableValue::String(s) => format!("\"{s}\""),
                            stratum_core::bytecode::HashableValue::Int(i) => format!("{i}"),
                            stratum_core::bytecode::HashableValue::Bool(b) => format!("{b}"),
                            stratum_core::bytecode::HashableValue::Null => "null".to_string(),
                        };
                        format!("{key_str}: {}", pretty_print(v))
                    })
                    .collect();
                if entries.len() > 10 {
                    format!(
                        "{{{}, ... ({} more)}}",
                        formatted.join(", "),
                        entries.len() - 10
                    )
                } else {
                    format!("{{{}}}", formatted.join(", "))
                }
            }
        }
        Value::DataFrame(df) => {
            format!(
                "<DataFrame [{} cols x {} rows]>",
                df.num_columns(),
                df.num_rows()
            )
        }
        Value::Series(series) => {
            format!("<Series '{}' [{} rows]>", series.name(), series.len())
        }
        Value::Cube(cube) => {
            format!(
                "<Cube '{}' [{} dims x {} measures]>",
                cube.name().unwrap_or("unnamed"),
                cube.dimension_names().len(),
                cube.measure_names().len()
            )
        }
        _ => format!("{value}"),
    }
}

const HELP_TEXT: &str = r#"REPL Commands:
  :help, :h, :?    Show this help message
  :clear, :cls     Clear history
  :reset           Reset VM state (clear all variables)
  :vars            Show defined variables

Tips:
  - Variables persist across inputs
  - Supports expressions: 1 + 2, "hello".len()
  - Supports statements: let x = 5
  - Supports functions: fx add(a, b) { a + b }
  - Press Enter to evaluate
  - Use up/down arrows for history"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_creation() {
        let repl = ReplPanel::new();
        assert!(repl.history.is_empty());
        assert!(repl.current_input.is_empty());
        assert!(repl.history_index.is_none());
    }

    #[test]
    fn test_is_complete_simple() {
        assert!(is_complete("1 + 2"));
        assert!(is_complete("hello"));
        assert!(is_complete(""));
    }

    #[test]
    fn test_is_complete_balanced() {
        assert!(is_complete("(1 + 2)"));
        assert!(is_complete("[1, 2, 3]"));
        assert!(is_complete("{ let x = 5 }"));
    }

    #[test]
    fn test_is_complete_unbalanced() {
        assert!(!is_complete("(1 + 2"));
        assert!(!is_complete("[1, 2, 3"));
        assert!(!is_complete("{ let x = 5"));
    }

    #[test]
    fn test_is_complete_strings() {
        assert!(is_complete(r#""hello""#));
        assert!(is_complete(r#""hello { world }""#));
        assert!(!is_complete(r#""hello"#));
    }

    #[test]
    fn test_history_navigation() {
        let mut repl = ReplPanel::new();

        // Add some history
        repl.history.push(ReplEntry {
            input: "first".to_string(),
            output: "1".to_string(),
            is_error: false,
        });
        repl.history.push(ReplEntry {
            input: "second".to_string(),
            output: "2".to_string(),
            is_error: false,
        });

        // Navigate up
        repl.current_input = "current".to_string();
        repl.history_up();
        assert_eq!(repl.current_input, "second");
        assert_eq!(repl.history_index, Some(1));

        repl.history_up();
        assert_eq!(repl.current_input, "first");
        assert_eq!(repl.history_index, Some(0));

        // Navigate down
        repl.history_down();
        assert_eq!(repl.current_input, "second");
        assert_eq!(repl.history_index, Some(1));

        repl.history_down();
        assert_eq!(repl.current_input, "current"); // Restored saved input
        assert!(repl.history_index.is_none());
    }

    #[test]
    fn test_command_help() {
        let mut repl = ReplPanel::new();
        let result = repl.handle_command(":help");
        assert!(matches!(result, Some(CommandResult::Handled(_))));
    }

    #[test]
    fn test_command_clear() {
        let mut repl = ReplPanel::new();
        repl.history.push(ReplEntry {
            input: "test".to_string(),
            output: "ok".to_string(),
            is_error: false,
        });

        repl.handle_command(":clear");
        // Note: :clear is handled in the match, we need to call update
        repl.update(ReplMessage::Clear);
        assert!(repl.history.is_empty());
    }

    #[test]
    fn test_not_a_command() {
        let mut repl = ReplPanel::new();
        let result = repl.handle_command("1 + 2");
        assert!(result.is_none());
    }

    #[test]
    fn test_eval_simple_expression() {
        let repl = ReplPanel::new();
        let result = repl.eval("1 + 2");
        assert!(result.is_ok());
        let (_stdout, value) = result.unwrap();
        assert_eq!(format!("{}", value), "3");
    }

    #[test]
    fn test_eval_multiple_expressions() {
        // Test that the VM is reused between evaluations
        let repl = ReplPanel::new();

        // First expression
        let result = repl.eval("1 + 2");
        assert!(result.is_ok());
        let (_stdout, value) = result.unwrap();
        assert_eq!(format!("{}", value), "3");

        // Second expression on same VM instance
        let result = repl.eval("10 * 5");
        assert!(result.is_ok());
        let (_stdout, value) = result.unwrap();
        assert_eq!(format!("{}", value), "50");

        // Third expression - test that VM is still alive
        let result = repl.eval("[1, 2, 3].len()");
        assert!(result.is_ok());
        let (_stdout, value) = result.unwrap();
        assert_eq!(format!("{}", value), "3");
    }

    #[test]
    fn test_eval_with_print_capture() {
        let repl = ReplPanel::new();
        let result = repl.eval("println(\"hello\")");
        assert!(result.is_ok());
        let (stdout, _value) = result.unwrap();
        assert_eq!(stdout, vec!["hello"]);
    }

    #[test]
    fn test_pretty_print() {
        assert_eq!(pretty_print(&Value::Int(42)), "42");
        assert_eq!(pretty_print(&Value::Bool(true)), "true");
        assert_eq!(pretty_print(&Value::Null), "null");

        let s = Value::String(std::rc::Rc::new("hello".to_string()));
        assert_eq!(pretty_print(&s), "\"hello\"");
    }

    #[test]
    fn test_for_loop_with_println() {
        let repl = ReplPanel::new();
        let result = repl.eval("for ii in 1..=3 { println(ii) }");
        assert!(result.is_ok());
        let (stdout, _value) = result.unwrap();
        // Should have captured 3 lines: "1", "2", "3"
        assert_eq!(stdout, vec!["1", "2", "3"]);
    }
}
