//! Output capture for VM execution
//!
//! This module provides a thread-local output buffer that allows capturing
//! stdout from print/println calls during VM execution.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

// Thread-local output buffer for capturing print output
thread_local! {
    static OUTPUT_BUFFER: RefCell<Option<Arc<Mutex<Vec<String>>>>> = const { RefCell::new(None) };
}

/// Captured output from VM execution
#[derive(Debug, Clone, Default)]
pub struct OutputCapture {
    /// Lines of stdout output
    pub stdout: Vec<String>,
}

impl OutputCapture {
    /// Create a new empty output capture
    pub fn new() -> Self {
        Self::default()
    }
}

/// Execute a function with output capture enabled.
///
/// Any calls to `print` or `println` during the execution of `f` will be
/// captured and returned in the `OutputCapture` struct.
///
/// # Example
/// ```ignore
/// let (result, output) = with_output_capture(|| {
///     vm.run(function)
/// });
/// for line in output.stdout {
///     println!("Captured: {}", line);
/// }
/// ```
pub fn with_output_capture<F, R>(f: F) -> (R, OutputCapture)
where
    F: FnOnce() -> R,
{
    let buffer = Arc::new(Mutex::new(Vec::new()));

    // Set the thread-local buffer
    OUTPUT_BUFFER.with(|cell| {
        *cell.borrow_mut() = Some(buffer.clone());
    });

    // Execute the function
    let result = f();

    // Clear the thread-local buffer and collect output
    OUTPUT_BUFFER.with(|cell| {
        *cell.borrow_mut() = None;
    });

    let stdout = Arc::try_unwrap(buffer)
        .ok()
        .and_then(|m| m.into_inner().ok())
        .unwrap_or_default();

    (result, OutputCapture { stdout })
}

/// Write a line to the output buffer (used by print natives).
/// Returns true if output was captured, false if it should go to stdout.
pub(crate) fn capture_output(text: &str) -> bool {
    OUTPUT_BUFFER.with(|cell| {
        if let Some(buffer) = cell.borrow().as_ref() {
            if let Ok(mut buf) = buffer.lock() {
                buf.push(text.to_string());
                return true;
            }
        }
        false
    })
}

/// Write output without a newline (used by print native).
/// Returns true if output was captured, false if it should go to stdout.
pub(crate) fn capture_print(text: &str) -> bool {
    OUTPUT_BUFFER.with(|cell| {
        if let Some(buffer) = cell.borrow().as_ref() {
            if let Ok(mut buf) = buffer.lock() {
                // Append to last line if exists, otherwise create new line
                if let Some(last) = buf.last_mut() {
                    last.push_str(text);
                } else {
                    buf.push(text.to_string());
                }
                return true;
            }
        }
        false
    })
}

/// Mark the end of a line in captured output (used after print calls with newline).
#[allow(dead_code)]
pub(crate) fn capture_newline() -> bool {
    OUTPUT_BUFFER.with(|cell| {
        if let Some(buffer) = cell.borrow().as_ref() {
            if let Ok(mut buf) = buffer.lock() {
                buf.push(String::new());
                return true;
            }
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Compiler, Parser, VM};

    #[test]
    fn test_output_capture_basic() {
        let ((), output) = with_output_capture(|| {
            if !capture_output("Hello, World!") {
                println!("Hello, World!");
            }
        });

        assert_eq!(output.stdout.len(), 1);
        assert_eq!(output.stdout[0], "Hello, World!");
    }

    #[test]
    fn test_output_capture_multiple_lines() {
        let ((), output) = with_output_capture(|| {
            capture_output("Line 1");
            capture_output("Line 2");
            capture_output("Line 3");
        });

        assert_eq!(output.stdout.len(), 3);
        assert_eq!(output.stdout[0], "Line 1");
        assert_eq!(output.stdout[1], "Line 2");
        assert_eq!(output.stdout[2], "Line 3");
    }

    #[test]
    fn test_no_capture_returns_false() {
        // Outside of with_output_capture, capture_output returns false
        assert!(!capture_output("test"));
    }

    #[test]
    fn test_vm_println_capture() {
        // Test that println in Stratum code gets captured
        let expr = Parser::parse_expression(r#"println("Hello from Stratum!")"#).unwrap();
        let function = Compiler::new().compile_expression(&expr).unwrap();

        let (result, output) = with_output_capture(|| {
            let mut vm = VM::new();
            vm.run(function)
        });

        assert!(result.is_ok());
        assert_eq!(output.stdout.len(), 1);
        assert_eq!(output.stdout[0], "Hello from Stratum!");
    }

    #[test]
    fn test_vm_multiple_println_capture() {
        // Test multiple println calls
        let source = r#"{
            println("Line 1");
            println("Line 2");
            println("Line 3");
            42
        }"#;
        let expr = Parser::parse_expression(source).unwrap();
        let function = Compiler::new().compile_expression(&expr).unwrap();

        let (result, output) = with_output_capture(|| {
            let mut vm = VM::new();
            vm.run(function)
        });

        assert!(result.is_ok());
        assert_eq!(output.stdout.len(), 3);
        assert_eq!(output.stdout[0], "Line 1");
        assert_eq!(output.stdout[1], "Line 2");
        assert_eq!(output.stdout[2], "Line 3");
    }

    #[test]
    fn test_vm_print_without_newline() {
        // Test print (without newline) accumulates on same line
        let source = r#"{
            print("Hello");
            print(" ");
            print("World");
            println("!");
            42
        }"#;
        let expr = Parser::parse_expression(source).unwrap();
        let function = Compiler::new().compile_expression(&expr).unwrap();

        let (result, output) = with_output_capture(|| {
            let mut vm = VM::new();
            vm.run(function)
        });

        assert!(result.is_ok());
        // print calls accumulate, println finishes the line
        assert_eq!(output.stdout.len(), 2);
        assert_eq!(output.stdout[0], "Hello World");
        assert_eq!(output.stdout[1], "!");
    }
}
