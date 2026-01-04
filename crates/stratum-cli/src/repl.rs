//! REPL (Read-Eval-Print Loop) for Stratum
//!
//! Provides an interactive environment for evaluating Stratum expressions.

use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};
use stratum_core::bytecode::Value;
use stratum_core::{Compiler, Parser, VM};

/// REPL prompt shown at the start of each line
const PROMPT: &str = ">>> ";
/// Continuation prompt for multi-line input
const CONTINUATION_PROMPT: &str = "... ";
/// History file name
const HISTORY_FILE: &str = ".stratum_history";

/// Result of processing a REPL command
enum CommandResult {
    /// Continue the REPL loop
    Continue,
    /// Exit the REPL
    Exit,
    /// Input was handled as a command (no further evaluation needed)
    Handled,
}

/// The Stratum REPL
pub struct Repl {
    vm: VM,
    editor: Editor<(), DefaultHistory>,
}

impl Repl {
    /// Create a new REPL instance
    pub fn new() -> Result<Self> {
        let vm = VM::new();
        let mut editor = DefaultEditor::new()?;

        // Load history if available
        if let Some(home) = home_dir() {
            let history_path = home.join(HISTORY_FILE);
            let _ = editor.load_history(&history_path);
        }

        Ok(Self { vm, editor })
    }

    /// Run the REPL loop
    pub fn run(&mut self) -> Result<()> {
        println!("Stratum v{}", stratum_core::VERSION);
        println!("Type :help for help, :quit to exit");
        println!();

        loop {
            match self.read_input() {
                Ok(Some(input)) => {
                    // Check for REPL commands first
                    match self.handle_command(&input) {
                        CommandResult::Exit => break,
                        CommandResult::Handled => continue,
                        CommandResult::Continue => {}
                    }

                    // Evaluate the input
                    self.eval_and_print(&input);
                }
                Ok(None) => {
                    // Empty input, continue
                    continue;
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    eprintln!("Error reading input: {err}");
                    break;
                }
            }
        }

        // Save history
        if let Some(home) = home_dir() {
            let history_path = home.join(HISTORY_FILE);
            let _ = self.editor.save_history(&history_path);
        }

        Ok(())
    }

    /// Read input from the user, handling multi-line input
    fn read_input(&mut self) -> Result<Option<String>, ReadlineError> {
        let mut input = String::new();
        let mut prompt = PROMPT;

        loop {
            let line = self.editor.readline(prompt)?;

            if input.is_empty() && line.trim().is_empty() {
                return Ok(None);
            }

            if !input.is_empty() {
                input.push('\n');
            }
            input.push_str(&line);

            // Check if we need more input
            if is_complete(&input) {
                // Add to history
                let _ = self.editor.add_history_entry(&input);
                return Ok(Some(input));
            }

            prompt = CONTINUATION_PROMPT;
        }
    }

    /// Handle REPL commands (starting with :)
    fn handle_command(&self, input: &str) -> CommandResult {
        let trimmed = input.trim();

        if !trimmed.starts_with(':') {
            return CommandResult::Continue;
        }

        let cmd = trimmed.trim_start_matches(':').trim();
        let (cmd_name, args) = cmd.split_once(' ').unwrap_or((cmd, ""));

        match cmd_name.to_lowercase().as_str() {
            "quit" | "q" | "exit" => CommandResult::Exit,

            "help" | "h" | "?" => {
                print_help();
                CommandResult::Handled
            }

            "clear" | "cls" => {
                // Clear screen using ANSI escape codes
                print!("\x1B[2J\x1B[1;1H");
                CommandResult::Handled
            }

            "type" | "t" => {
                if args.is_empty() {
                    println!("Usage: :type <expression>");
                } else {
                    self.show_type(args);
                }
                CommandResult::Handled
            }

            "reset" => {
                println!("Note: :reset creates a new VM instance. Use :quit and restart for now.");
                CommandResult::Handled
            }

            _ => {
                println!("Unknown command: :{cmd_name}");
                println!("Type :help for available commands");
                CommandResult::Handled
            }
        }
    }

    /// Evaluate input and print the result
    fn eval_and_print(&mut self, input: &str) {
        // Try to parse and compile
        let result = self.eval(input);

        match result {
            Ok(value) => {
                // Don't print null for statements that don't produce a value
                if !matches!(value, Value::Null) {
                    println!("{}", pretty_print(&value));
                }
            }
            Err(err) => {
                eprintln!("{err}");
            }
        }
    }

    /// Evaluate a string of Stratum code
    fn eval(&mut self, input: &str) -> Result<Value, String> {
        // First, try parsing as an expression
        let expr = Parser::parse_expression(input).map_err(|errors| {
            errors
                .iter()
                .map(|e| format!("Parse error: {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        })?;

        // Compile the expression
        let function = Compiler::new()
            .compile_expression(&expr)
            .map_err(|errors| {
                errors
                    .iter()
                    .map(|e| format!("Compile error: {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;

        // Run in the VM
        self.vm.run(function).map_err(|e| format!("Runtime error: {e}"))
    }

    /// Show the type of an expression without evaluating it
    fn show_type(&self, input: &str) {
        match Parser::parse_expression(input) {
            Ok(_expr) => {
                // For now, just show that it parsed successfully
                // Full type inference would require the type checker
                println!("Expression is syntactically valid");
            }
            Err(errors) => {
                for e in errors {
                    eprintln!("Parse error: {e}");
                }
            }
        }
    }
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
                let formatted: Vec<String> = items.iter().map(|v| pretty_print(v)).collect();
                format!("[{}]", formatted.join(", "))
            } else {
                // Truncate long lists
                let formatted: Vec<String> =
                    items.iter().take(10).map(|v| pretty_print(v)).collect();
                format!("[{}, ... ({} more)]", formatted.join(", "), items.len() - 10)
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
                            stratum_core::bytecode::HashableValue::String(s) => {
                                format!("\"{s}\"")
                            }
                            stratum_core::bytecode::HashableValue::Int(i) => format!("{i}"),
                            stratum_core::bytecode::HashableValue::Bool(b) => format!("{b}"),
                            stratum_core::bytecode::HashableValue::Null => "null".to_string(),
                        };
                        format!("{key_str}: {}", pretty_print(v))
                    })
                    .collect();
                if entries.len() > 10 {
                    format!("{{{}, ... ({} more)}}", formatted.join(", "), entries.len() - 10)
                } else {
                    format!("{{{}}}", formatted.join(", "))
                }
            }
        }
        _ => format!("{value}"),
    }
}

/// Print help information
fn print_help() {
    println!(
        r#"
Stratum REPL Commands:
  :help, :h, :?    Show this help message
  :quit, :q        Exit the REPL
  :clear, :cls     Clear the screen
  :type <expr>     Check if an expression is valid

Tips:
  - Variables persist across inputs
  - Use blocks for multiple statements: {{ let x = 5; x + 1 }}
  - Press Ctrl+C to cancel current input
  - Press Ctrl+D to exit
  - Use up/down arrows for history

Examples:
  >>> 1 + 2 * 3
  7
  >>> {{ let x = 5; x * 2 }}
  10
  >>> [1, 2, 3].map(|n| n * 2)
  [2, 4, 6]
"#
    );
}

/// Get the user's home directory
fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(is_complete("fn(x) { x + 1 }"));
    }

    #[test]
    fn test_is_complete_unbalanced() {
        assert!(!is_complete("(1 + 2"));
        assert!(!is_complete("[1, 2, 3"));
        assert!(!is_complete("{ let x = 5"));
        assert!(!is_complete("fn(x) { x + 1"));
    }

    #[test]
    fn test_is_complete_strings() {
        assert!(is_complete(r#""hello""#));
        assert!(is_complete(r#""hello { world }""#)); // braces in string don't count
        assert!(!is_complete(r#""hello"#)); // unclosed string
    }

    #[test]
    fn test_is_complete_comments() {
        assert!(is_complete("1 + 2 // comment"));
        assert!(is_complete("1 + 2 /* block comment */"));
        assert!(is_complete("{ x /* nested { braces } */ }"));
    }

    #[test]
    fn test_pretty_print() {
        assert_eq!(pretty_print(&Value::Int(42)), "42");
        assert_eq!(pretty_print(&Value::Bool(true)), "true");
        assert_eq!(pretty_print(&Value::Null), "null");

        let s = Value::String(std::rc::Rc::new("hello".to_string()));
        assert_eq!(pretty_print(&s), "\"hello\"");
    }
}
