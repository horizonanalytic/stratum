//! Test utilities for Stratum
//!
//! This module provides common helpers for testing Stratum code,
//! including expression evaluation, program execution, and assertions.

use crate::bytecode::{Compiler, Value};
use crate::parser::Parser;
use crate::types::TypeChecker;
use crate::vm::VM;

/// Result type for test helpers
pub type TestResult<T> = Result<T, String>;

/// Evaluate a Stratum expression and return the result
///
/// # Errors
/// Returns error if parsing, type checking, compilation, or execution fails
pub fn eval_expr(source: &str) -> TestResult<Value> {
    let expr = Parser::parse_expression(source).map_err(|e| format!("Parse error: {e:?}"))?;
    let function = Compiler::new()
        .compile_expression(&expr)
        .map_err(|e| format!("Compile error: {e:?}"))?;
    let mut vm = VM::new();
    vm.run(function).map_err(|e| format!("Runtime error: {e}"))
}

/// Evaluate a Stratum expression without type checking
///
/// This is useful for testing dynamic features like heterogeneous maps
/// for DataFrame creation.
///
/// # Errors
/// Returns error if parsing, compilation, or execution fails
pub fn eval_expr_dynamic(source: &str) -> TestResult<Value> {
    let expr = Parser::parse_expression(source).map_err(|e| format!("Parse error: {e:?}"))?;
    let function = Compiler::new()
        .compile_expression(&expr)
        .map_err(|e| format!("Compile error: {e:?}"))?;
    let mut vm = VM::new();
    vm.run(function).map_err(|e| format!("Runtime error: {e}"))
}

/// Evaluate a Stratum expression and expect an integer result
///
/// # Errors
/// Returns error if evaluation fails or result is not an integer
pub fn eval_int(source: &str) -> TestResult<i64> {
    match eval_expr(source)? {
        Value::Int(n) => Ok(n),
        other => Err(format!("Expected Int, got {}", other.type_name())),
    }
}

/// Evaluate a Stratum expression and expect a float result
///
/// # Errors
/// Returns error if evaluation fails or result is not a float
pub fn eval_float(source: &str) -> TestResult<f64> {
    match eval_expr(source)? {
        Value::Float(n) => Ok(n),
        other => Err(format!("Expected Float, got {}", other.type_name())),
    }
}

/// Evaluate a Stratum expression and expect a boolean result
///
/// # Errors
/// Returns error if evaluation fails or result is not a boolean
pub fn eval_bool(source: &str) -> TestResult<bool> {
    match eval_expr(source)? {
        Value::Bool(b) => Ok(b),
        other => Err(format!("Expected Bool, got {}", other.type_name())),
    }
}

/// Evaluate a Stratum expression and expect a string result
///
/// # Errors
/// Returns error if evaluation fails or result is not a string
pub fn eval_string(source: &str) -> TestResult<String> {
    match eval_expr(source)? {
        Value::String(s) => Ok((*s).clone()),
        other => Err(format!("Expected String, got {}", other.type_name())),
    }
}

/// Run a complete Stratum program
///
/// # Errors
/// Returns error if parsing, type checking, compilation, or execution fails
pub fn run_program(source: &str) -> TestResult<Value> {
    let module = Parser::parse_module(source).map_err(|e| format!("Parse error: {e:?}"))?;

    let mut checker = TypeChecker::new();
    let result = checker.check_module(&module);
    if !result.errors.is_empty() {
        return Err(format!("Type error: {}", result.errors[0]));
    }

    let function = Compiler::new()
        .compile_module(&module)
        .map_err(|e| format!("Compile error: {e:?}"))?;

    let mut vm = VM::new();
    vm.run(function).map_err(|e| format!("Runtime error: {e}"))
}

/// Run a complete Stratum program without type checking
///
/// This is useful for testing dynamic features that the type checker
/// may not fully support (e.g., heterogeneous maps for DataFrame creation).
///
/// # Errors
/// Returns error if parsing, compilation, or execution fails
pub fn run_program_dynamic(source: &str) -> TestResult<Value> {
    let module = Parser::parse_module(source).map_err(|e| format!("Parse error: {e:?}"))?;

    // Skip type checking - useful for dynamic data structures
    let function = Compiler::new()
        .compile_module(&module)
        .map_err(|e| format!("Compile error: {e:?}"))?;

    let mut vm = VM::new();
    vm.run(function).map_err(|e| format!("Runtime error: {e}"))
}

/// Check that a Stratum program type-checks successfully
///
/// # Errors
/// Returns error if parsing or type checking fails
pub fn typecheck(source: &str) -> TestResult<()> {
    let module = Parser::parse_module(source).map_err(|e| format!("Parse error: {e:?}"))?;

    let mut checker = TypeChecker::new();
    let result = checker.check_module(&module);
    if !result.errors.is_empty() {
        return Err(format!("Type error: {}", result.errors[0]));
    }

    Ok(())
}

/// Check that a Stratum program fails type checking
///
/// # Errors
/// Returns error if the program unexpectedly type-checks successfully
pub fn expect_type_error(source: &str) -> TestResult<String> {
    let module = match Parser::parse_module(source) {
        Ok(module) => module,
        Err(e) => return Ok(format!("Parse error: {e:?}")),
    };

    let mut checker = TypeChecker::new();
    let result = checker.check_module(&module);
    if result.errors.is_empty() {
        Err("Expected type error, but program type-checked successfully".to_string())
    } else {
        Ok(result.errors[0].to_string())
    }
}

/// Check that a Stratum program fails at runtime
///
/// # Errors
/// Returns error if the program unexpectedly succeeds
pub fn expect_runtime_error(source: &str) -> TestResult<String> {
    match run_program(source) {
        Ok(value) => Err(format!(
            "Expected runtime error, but got value: {}",
            value
        )),
        Err(e) => Ok(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_expr_basic() {
        assert_eq!(eval_int("1 + 2").unwrap(), 3);
        assert_eq!(eval_float("1.5 * 2.0").unwrap(), 3.0);
        assert!(eval_bool("true and false").is_ok());
        assert_eq!(eval_string(r#""hello""#).unwrap(), "hello");
    }

    #[test]
    fn test_eval_expr_block() {
        let result = eval_int("{ let x = 10; x * 2 }").unwrap();
        assert_eq!(result, 20);
    }
}
