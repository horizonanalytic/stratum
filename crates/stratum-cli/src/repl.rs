//! REPL (Read-Eval-Print Loop) for Stratum
//!
//! Provides an interactive environment for evaluating Stratum code.
//!
//! Supports:
//! - Expressions: `1 + 2`, `foo.bar()`
//! - Let statements: `let x = 5`
//! - Function definitions: `fx add(a, b) { a + b }`
//! - Multiple statements: `let x = 5; let y = 6`
//! - Control flow: `for`, `while`, `if`

use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};
use std::collections::HashSet;
use std::path::Path;
use stratum_core::bytecode::Value;
use stratum_core::parser::ReplInput;
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
    /// The VM instance that persists across inputs
    vm: VM,
    /// Line editor with history support
    editor: Editor<(), DefaultHistory>,
    /// Track user-defined function names for :funcs command
    user_functions: HashSet<String>,
    /// Track user-defined variable names for :vars command
    user_variables: HashSet<String>,
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

        Ok(Self {
            vm,
            editor,
            user_functions: HashSet::new(),
            user_variables: HashSet::new(),
        })
    }

    /// Reset the REPL state (creates new VM, clears defined functions/variables)
    fn reset(&mut self) {
        self.vm = VM::new();
        self.user_functions.clear();
        self.user_variables.clear();
        println!("REPL state has been reset.");
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
    fn handle_command(&mut self, input: &str) -> CommandResult {
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

            "vars" | "v" => {
                self.show_vars();
                CommandResult::Handled
            }

            "funcs" | "f" => {
                self.show_funcs();
                CommandResult::Handled
            }

            "reset" | "r" => {
                self.reset();
                CommandResult::Handled
            }

            "load" | "l" => {
                if args.is_empty() {
                    println!("Usage: :load <file>");
                } else {
                    self.load_file(args);
                }
                CommandResult::Handled
            }

            _ => {
                println!("Unknown command: :{cmd_name}");
                println!("Type :help for available commands");
                CommandResult::Handled
            }
        }
    }

    /// Show all user-defined variables and their values
    fn show_vars(&self) {
        if self.user_variables.is_empty() {
            println!("No user-defined variables.");
            return;
        }

        println!("User-defined variables:");
        for name in &self.user_variables {
            if let Some(value) = self.vm.globals().get(name) {
                println!("  {name} = {}", pretty_print(value));
            }
        }
    }

    /// Show all user-defined functions
    fn show_funcs(&self) {
        if self.user_functions.is_empty() {
            println!("No user-defined functions.");
            return;
        }

        println!("User-defined functions:");
        for name in &self.user_functions {
            if let Some(value) = self.vm.globals().get(name) {
                if let Value::Function(func) = value {
                    let params = if func.arity == 0 {
                        String::new()
                    } else {
                        (0..func.arity).map(|i| format!("arg{i}")).collect::<Vec<_>>().join(", ")
                    };
                    println!("  fx {name}({params})");
                } else {
                    println!("  {name}");
                }
            }
        }
    }

    /// Load and execute a Stratum file
    fn load_file(&mut self, path: &str) {
        let path = Path::new(path.trim());

        if !path.exists() {
            eprintln!("File not found: {}", path.display());
            return;
        }

        match std::fs::read_to_string(path) {
            Ok(source) => {
                println!("Loading {}...", path.display());
                // Parse as a module to handle functions and top-level items
                match Parser::parse_module(&source) {
                    Ok(module) => {
                        match Compiler::new().compile_module(&module) {
                            Ok(function) => {
                                match self.vm.run(function) {
                                    Ok(_) => {
                                        // Track any functions defined in the file
                                        for item in module.items() {
                                            if let stratum_core::ast::ItemKind::Function(func) = &item.kind {
                                                self.user_functions.insert(func.name.name.clone());
                                            }
                                        }
                                        println!("File loaded successfully.");
                                    }
                                    Err(e) => eprintln!("Runtime error: {e}"),
                                }
                            }
                            Err(errors) => {
                                for e in errors {
                                    eprintln!("Compile error: {e}");
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for e in errors {
                            eprintln!("Parse error: {e}");
                        }
                    }
                }
            }
            Err(e) => eprintln!("Error reading file: {e}"),
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
        // Parse the input - supports expressions, statements, and function definitions
        let repl_input = Parser::parse_repl_input(input).map_err(|errors| {
            errors
                .iter()
                .map(|e| format!("Parse error: {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        })?;

        // Track defined functions and variables
        self.track_definitions(&repl_input);

        // Compile the REPL input
        let function = Compiler::new()
            .compile_repl_input(&repl_input)
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

    /// Track user-defined functions and variables from REPL input
    fn track_definitions(&mut self, input: &ReplInput) {
        match input {
            ReplInput::Function(func) => {
                self.user_functions.insert(func.name.name.clone());
            }
            ReplInput::Statement(stmt) => {
                self.track_statement_definitions(stmt);
            }
            ReplInput::Statements(stmts) => {
                for stmt in stmts {
                    self.track_statement_definitions(stmt);
                }
            }
            ReplInput::Expression(_) => {
                // Expressions don't define new variables at the top level
            }
        }
    }

    /// Track variable definitions from a statement
    fn track_statement_definitions(&mut self, stmt: &stratum_core::ast::Stmt) {
        use stratum_core::ast::{PatternKind, StmtKind};

        if let StmtKind::Let { pattern, .. } = &stmt.kind {
            // Extract variable name from pattern
            if let PatternKind::Ident(ident) = &pattern.kind {
                self.user_variables.insert(ident.name.clone());
            }
        }
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
  :help, :h, :?     Show this help message
  :quit, :q         Exit the REPL
  :clear, :cls      Clear the screen
  :type <expr>      Check if an expression is valid
  :vars, :v         Show all user-defined variables
  :funcs, :f        Show all user-defined functions
  :reset, :r        Reset REPL state (clear variables and functions)
  :load <file>, :l  Load and execute a Stratum file

Supported Input:
  - Expressions:    1 + 2, foo.bar(), [1,2,3].map(|x| x*2)
  - Let statements: let x = 5
  - Functions:      fx add(a, b) {{ a + b }}
  - Control flow:   for i in range(10) {{ println(i) }}
  - Multiple:       let x = 5; let y = 6; x + y

Tips:
  - Variables and functions persist across inputs
  - Press Ctrl+C to cancel current input
  - Press Ctrl+D to exit
  - Use up/down arrows for history

Examples:
  >>> 1 + 2 * 3
  7
  >>> let x = 5
  >>> x * 2
  10
  >>> fx double(n) {{ n * 2 }}
  >>> double(21)
  42
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

    // Tests for REPL input parsing
    #[test]
    fn test_parse_repl_expression() {
        let input = Parser::parse_repl_input("1 + 2 * 3");
        assert!(input.is_ok());
        assert!(matches!(input.unwrap(), ReplInput::Expression(_)));
    }

    #[test]
    fn test_parse_repl_let_statement() {
        let input = Parser::parse_repl_input("let x = 5");
        assert!(input.is_ok());
        assert!(matches!(input.unwrap(), ReplInput::Statement(_)));
    }

    #[test]
    fn test_parse_repl_function() {
        let input = Parser::parse_repl_input("fx add(a, b) { a + b }");
        assert!(input.is_ok());
        assert!(matches!(input.unwrap(), ReplInput::Function(_)));
    }

    #[test]
    fn test_parse_repl_async_function() {
        let input = Parser::parse_repl_input("async fx fetch() { 42 }");
        assert!(input.is_ok());
        assert!(matches!(input.unwrap(), ReplInput::Function(_)));
    }

    #[test]
    fn test_parse_repl_multiple_statements() {
        let input = Parser::parse_repl_input("let x = 5; let y = 6");
        assert!(input.is_ok());
        assert!(matches!(input.unwrap(), ReplInput::Statements(_)));
    }

    #[test]
    fn test_parse_repl_for_loop() {
        // Use a simpler for loop syntax that doesn't require calling a function
        let input = Parser::parse_repl_input("for i in [1, 2, 3] { i }");
        assert!(input.is_ok(), "Failed to parse for loop: {:?}", input.err());
        assert!(matches!(input.unwrap(), ReplInput::Statement(_)));
    }

    #[test]
    fn test_parse_repl_while_loop() {
        let input = Parser::parse_repl_input("while true { break }");
        assert!(input.is_ok());
        assert!(matches!(input.unwrap(), ReplInput::Statement(_)));
    }

    // Tests for REPL evaluation
    #[test]
    fn test_repl_eval_expression() {
        let mut repl = Repl::new().unwrap();
        let result = repl.eval("1 + 2 * 3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(7));
    }

    #[test]
    fn test_repl_eval_let_and_use() {
        let mut repl = Repl::new().unwrap();

        // Define a variable
        let result = repl.eval("let x = 42");
        assert!(result.is_ok());

        // Use the variable
        let result = repl.eval("x * 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(84));
    }

    #[test]
    fn test_repl_eval_function_def_and_call() {
        let mut repl = Repl::new().unwrap();

        // Define a function
        let result = repl.eval("fx double(n) { n * 2 }");
        assert!(result.is_ok());

        // Call the function
        let result = repl.eval("double(21)");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_repl_tracking_variables() {
        let mut repl = Repl::new().unwrap();

        assert!(repl.user_variables.is_empty());

        repl.eval("let x = 5").unwrap();
        assert!(repl.user_variables.contains("x"));

        repl.eval("let y = 10").unwrap();
        assert!(repl.user_variables.contains("y"));
    }

    #[test]
    fn test_repl_tracking_functions() {
        let mut repl = Repl::new().unwrap();

        assert!(repl.user_functions.is_empty());

        repl.eval("fx add(a, b) { a + b }").unwrap();
        assert!(repl.user_functions.contains("add"));

        repl.eval("fx mul(a, b) { a * b }").unwrap();
        assert!(repl.user_functions.contains("mul"));
    }

    #[test]
    fn test_repl_reset() {
        let mut repl = Repl::new().unwrap();

        // Add some state
        repl.eval("let x = 5").unwrap();
        repl.eval("fx foo() { 1 }").unwrap();
        assert!(!repl.user_variables.is_empty());
        assert!(!repl.user_functions.is_empty());

        // Reset
        repl.reset();

        // State should be cleared
        assert!(repl.user_variables.is_empty());
        assert!(repl.user_functions.is_empty());
    }
}
