//! Code execution for Stratum Workshop
//!
//! This module provides the infrastructure for running Stratum code from the IDE,
//! capturing output, and handling errors.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use stratum_core::aot::{AotCompiler, Linker, LinkerConfig};
use stratum_core::ast::ExecutionMode;
use stratum_core::bytecode::Value;
use stratum_core::types::TypeChecker;
use stratum_core::{with_output_capture, Compiler, Parser, VM};

/// Result of executing Stratum code
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Captured stdout lines
    pub stdout: Vec<String>,
    /// Error messages (if any)
    pub errors: Vec<String>,
    /// The final return value (as string representation)
    pub return_value: Option<String>,
}

impl ExecutionResult {
    /// Create a successful execution result
    pub fn success(stdout: Vec<String>, return_value: Option<String>) -> Self {
        Self {
            success: true,
            stdout,
            errors: Vec::new(),
            return_value,
        }
    }

    /// Create a failed execution result
    pub fn failure(stdout: Vec<String>, errors: Vec<String>) -> Self {
        Self {
            success: false,
            stdout,
            errors,
            return_value: None,
        }
    }
}

/// Result of building Stratum code to an executable
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// Whether the build succeeded
    pub success: bool,
    /// Path to the generated executable (if successful)
    pub output_path: Option<PathBuf>,
    /// Build log messages
    pub messages: Vec<String>,
    /// Error messages (if any)
    pub errors: Vec<String>,
}

impl BuildResult {
    /// Create a successful build result
    pub fn success(output_path: PathBuf, messages: Vec<String>) -> Self {
        Self {
            success: true,
            output_path: Some(output_path),
            messages,
            errors: Vec::new(),
        }
    }

    /// Create a failed build result
    pub fn failure(messages: Vec<String>, errors: Vec<String>) -> Self {
        Self {
            success: false,
            output_path: None,
            messages,
            errors,
        }
    }
}

/// Cancellation token for stopping execution
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    /// Create a new cancellation token
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Execute Stratum source code and capture output
///
/// # Arguments
/// * `source` - The Stratum source code to execute
/// * `file_path` - Optional file path for error reporting
/// * `args` - Arguments to pass to main() function (as a space-separated string)
/// * `_cancellation` - Cancellation token (for future use)
///
/// # Returns
/// An `ExecutionResult` containing stdout, errors, and the return value
pub fn execute_source(
    source: &str,
    file_path: Option<&Path>,
    args: &str,
    _cancellation: &CancellationToken,
) -> ExecutionResult {
    let file_name = file_path
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("<untitled>");

    // Parse the module
    let module = match Parser::parse_module(source) {
        Ok(module) => module,
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format_parse_error(e, file_name))
                .collect();
            return ExecutionResult::failure(Vec::new(), error_messages);
        }
    };

    // Compile the module
    let function = match Compiler::with_source(file_name.to_string()).compile_module(&module) {
        Ok(function) => function,
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format_compile_error(e, file_name))
                .collect();
            return ExecutionResult::failure(Vec::new(), error_messages);
        }
    };

    // Parse arguments
    let parsed_args = parse_args(args);

    // Execute with output capture
    let (result, output) = with_output_capture(|| {
        let mut vm = VM::new();

        // Run the module (registers functions, executes top-level code)
        match vm.run(function) {
            Ok(_) => {
                // If there's a main() function, call it
                if vm.globals().contains_key("main") {
                    call_main(&mut vm, file_name, &parsed_args)
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(format_runtime_error(&e, file_name)),
        }
    });

    match result {
        Ok(return_value) => ExecutionResult::success(output.stdout, return_value),
        Err(error) => ExecutionResult::failure(output.stdout, vec![error]),
    }
}

/// Parse command-line style arguments
/// Handles quoted strings and escapes
fn parse_args(args: &str) -> Vec<String> {
    let args = args.trim();
    if args.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in args.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => escape_next = true,
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

/// Call the main() function if it exists
fn call_main(vm: &mut VM, file_name: &str, args: &[String]) -> Result<Option<String>, String> {
    // Build the main() call expression with arguments
    let args_str = if args.is_empty() {
        String::new()
    } else {
        args.iter()
            .map(|a| format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let main_call_str = format!("main({})", args_str);

    // Parse the main() call expression
    let main_call = Parser::parse_expression(&main_call_str)
        .map_err(|_| "Internal error: failed to parse main() call".to_string())?;

    // Compile the call
    let main_fn = Compiler::new()
        .compile_expression(&main_call)
        .map_err(|_| "Internal error: failed to compile main() call".to_string())?;

    // Execute main()
    match vm.run(main_fn) {
        Ok(value) => {
            // Return the value if it's not null
            if matches!(value, Value::Null) {
                Ok(None)
            } else {
                Ok(Some(format!("{value}")))
            }
        }
        Err(e) => Err(format_runtime_error(&e, file_name)),
    }
}

/// Format a parse error with source location
fn format_parse_error(error: &stratum_core::parser::ParseError, file_name: &str) -> String {
    // ParseError includes span information
    format!("Parse error in {}: {}", file_name, error)
}

/// Format a compile error with source location
fn format_compile_error(
    error: &stratum_core::bytecode::CompileError,
    file_name: &str,
) -> String {
    format!("Compile error in {}: {}", file_name, error)
}

/// Format a runtime error with source location
fn format_runtime_error(error: &stratum_core::vm::RuntimeError, file_name: &str) -> String {
    // RuntimeError includes stack trace
    let mut msg = format!("Runtime error in {}:\n{}", file_name, error);

    // Add stack trace if available
    if !error.stack_trace.is_empty() {
        msg.push_str("\n\nStack trace:");
        for frame in &error.stack_trace {
            let source = frame.source.as_deref().unwrap_or("<unknown>");
            msg.push_str(&format!(
                "\n  at {} ({}:{})",
                frame.function_name, source, frame.line
            ));
        }
    }

    msg
}

/// Execute source code asynchronously (for use with iced Tasks)
pub async fn execute_source_async(
    source: String,
    file_path: Option<std::path::PathBuf>,
    args: String,
    cancellation: CancellationToken,
) -> ExecutionResult {
    // Run execution in a blocking thread to not block the UI
    tokio::task::spawn_blocking(move || {
        execute_source(&source, file_path.as_deref(), &args, &cancellation)
    })
    .await
    .unwrap_or_else(|e| {
        ExecutionResult::failure(Vec::new(), vec![format!("Execution task panicked: {e}")])
    })
}

/// Build Stratum source code to a standalone executable
///
/// # Arguments
/// * `source` - The Stratum source code to compile
/// * `file_path` - The source file path (used for output path and error reporting)
/// * `release` - Whether to build with optimizations
///
/// # Returns
/// A `BuildResult` containing the output path or errors
pub fn build_source(source: &str, file_path: &Path, release: bool) -> BuildResult {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<untitled>");

    let mut messages = Vec::new();
    messages.push(format!("Compiling {}...", file_name));

    // Parse the module
    let module = match Parser::parse_module(source) {
        Ok(module) => module,
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format_parse_error(e, file_name))
                .collect();
            return BuildResult::failure(messages, error_messages);
        }
    };

    // Type check
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check_module(&module);
    if !type_result.errors.is_empty() {
        let error_messages: Vec<String> = type_result
            .errors
            .iter()
            .map(|e| format!("Type error in {}: {}", file_name, e))
            .collect();
        return BuildResult::failure(messages, error_messages);
    }
    messages.push("Type checking passed".to_string());

    // Compile to bytecode
    let bytecode_fn = match Compiler::with_source(file_name.to_string()).compile_module(&module) {
        Ok(function) => function,
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format_compile_error(e, file_name))
                .collect();
            return BuildResult::failure(messages, error_messages);
        }
    };

    // Create AOT compiler
    let mut aot = match AotCompiler::new() {
        Ok(aot) => aot,
        Err(e) => {
            return BuildResult::failure(messages, vec![format!("Failed to create AOT compiler: {e}")]);
        }
    };

    // Find all functions in the module and compile them
    let mut has_main = false;
    let mut compiled_count = 0;

    for constant in bytecode_fn.chunk.constants() {
        if let Value::Function(func) = constant {
            // Compile all functions for build
            let should_compile = matches!(
                func.execution_mode,
                ExecutionMode::Compile | ExecutionMode::CompileHot
            ) || true; // For build, compile all functions

            if should_compile {
                if let Err(e) = aot.compile_function(func) {
                    return BuildResult::failure(
                        messages,
                        vec![format!("Failed to compile function '{}': {}", func.name, e)],
                    );
                }
                compiled_count += 1;
                if func.name == "main" {
                    has_main = true;
                }
            }
        }
    }

    if !has_main {
        return BuildResult::failure(
            messages,
            vec!["No main function found in module".to_string()],
        );
    }
    messages.push(format!("Compiled {} function(s) to native code", compiled_count));

    // Generate entry point
    if let Err(e) = aot.generate_entry_point() {
        return BuildResult::failure(
            messages,
            vec![format!("Failed to generate entry point: {e}")],
        );
    }

    // Finish AOT compilation
    let product = aot.finish();

    // Determine output path (same directory as source, with executable extension)
    let output_path = file_path.with_extension(if cfg!(windows) { "exe" } else { "" });

    // Link into executable
    let linker = Linker::new(LinkerConfig {
        output: output_path.clone(),
        optimize: release,
        extra_flags: Vec::new(),
    });

    if let Err(e) = linker.link(product) {
        return BuildResult::failure(messages, vec![format!("Linking failed: {e}")]);
    }

    messages.push(format!(
        "Linked executable: {}",
        output_path.display()
    ));

    BuildResult::success(output_path, messages)
}

/// Build source code asynchronously (for use with iced Tasks)
pub async fn build_source_async(
    source: String,
    file_path: PathBuf,
    release: bool,
) -> BuildResult {
    // Run build in a blocking thread to not block the UI
    tokio::task::spawn_blocking(move || build_source(&source, &file_path, release))
        .await
        .unwrap_or_else(|e| {
            BuildResult::failure(Vec::new(), vec![format!("Build task panicked: {e}")])
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_simple_expression() {
        let source = r#"
            fx main() {
                println("Hello, Workshop!");
                42
            }
        "#;

        let result = execute_source(source, None, "", &CancellationToken::new());

        assert!(result.success, "Execution should succeed: {:?}", result.errors);
        assert_eq!(result.stdout.len(), 1);
        assert_eq!(result.stdout[0], "Hello, Workshop!");
        assert_eq!(result.return_value, Some("42".to_string()));
    }

    #[test]
    fn test_execute_parse_error() {
        let source = "fx main() { let x = }"; // Invalid syntax

        let result = execute_source(source, None, "", &CancellationToken::new());

        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_execute_runtime_error() {
        let source = r#"
            fx main() {
                let x = 1 / 0;
            }
        "#;

        let result = execute_source(source, None, "", &CancellationToken::new());

        // Division by zero should cause a runtime error
        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_execute_no_main() {
        let source = r#"
            fx helper() {
                42
            }
        "#;

        let result = execute_source(source, None, "", &CancellationToken::new());

        // Should succeed but return_value should be None (no main called)
        assert!(result.success);
        assert!(result.return_value.is_none());
    }

    #[test]
    fn test_execute_with_args() {
        let source = r#"
            fx main(name: String) {
                println("Hello, " + name + "!");
                name
            }
        "#;

        let result = execute_source(source, None, "World", &CancellationToken::new());

        assert!(result.success, "Execution should succeed: {:?}", result.errors);
        assert_eq!(result.stdout.len(), 1);
        assert_eq!(result.stdout[0], "Hello, World!");
        assert_eq!(result.return_value, Some("World".to_string()));
    }

    #[test]
    fn test_parse_args_empty() {
        assert_eq!(parse_args(""), Vec::<String>::new());
        assert_eq!(parse_args("   "), Vec::<String>::new());
    }

    #[test]
    fn test_parse_args_simple() {
        assert_eq!(parse_args("foo"), vec!["foo"]);
        assert_eq!(parse_args("foo bar baz"), vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_parse_args_quoted() {
        assert_eq!(parse_args("\"hello world\""), vec!["hello world"]);
        assert_eq!(parse_args("foo \"hello world\" bar"), vec!["foo", "hello world", "bar"]);
    }

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_build_result_success() {
        let path = PathBuf::from("/test/output");
        let messages = vec!["Compiling...".to_string(), "Linking...".to_string()];
        let result = BuildResult::success(path.clone(), messages.clone());

        assert!(result.success);
        assert_eq!(result.output_path, Some(path));
        assert_eq!(result.messages, messages);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_build_result_failure() {
        let messages = vec!["Compiling...".to_string()];
        let errors = vec!["No main function found".to_string()];
        let result = BuildResult::failure(messages.clone(), errors.clone());

        assert!(!result.success);
        assert!(result.output_path.is_none());
        assert_eq!(result.messages, messages);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_build_parse_error() {
        let source = "fx main() { let x = }"; // Invalid syntax
        let file_path = PathBuf::from("test.strat");

        let result = build_source(source, &file_path, false);

        assert!(!result.success);
        assert!(!result.errors.is_empty());
        // Should have at least the "Compiling..." message
        assert!(!result.messages.is_empty());
    }

    #[test]
    fn test_build_no_main() {
        let source = r#"
            fx helper() -> Int {
                42
            }
        "#;
        let file_path = PathBuf::from("test.strat");

        let result = build_source(source, &file_path, false);

        // Build should fail because there's no main function
        assert!(!result.success, "Build should fail without main function");
        // Check the error message mentions missing main
        let has_main_error = result.errors.iter().any(|e| e.to_lowercase().contains("main"));
        assert!(
            has_main_error,
            "Expected 'main' in error messages. Messages: {:?}, Errors: {:?}",
            result.messages,
            result.errors
        );
    }
}
