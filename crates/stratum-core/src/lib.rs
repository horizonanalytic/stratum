//! Stratum Core - Language engine for the Stratum programming language
//!
//! This crate provides the core functionality:
//! - Lexer: Tokenization of source code
//! - AST: Abstract syntax tree definitions
//! - Parser: AST construction from token stream
//! - Type Checker: Static type analysis
//! - Bytecode: Instruction set and compiler
//! - VM: Bytecode execution
//! - Formatter: Source code formatting

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Lexer module - tokenization of Stratum source code
pub mod lexer;

/// Abstract Syntax Tree - parsed representation of Stratum source code
pub mod ast;

/// Parser module - converts tokens into AST
pub mod parser;

/// Type system module - type checking and inference
pub mod types;

/// Bytecode module - instruction set and compiler
pub mod bytecode;

/// Virtual Machine module - bytecode execution
pub mod vm;

/// Testing framework - test discovery and execution
pub mod testing;

/// Source code formatter
pub mod formatter;

/// JIT compilation module (Cranelift-based)
/// JIT requires unsafe code for memory management and function pointers
#[allow(unsafe_code, clippy::missing_safety_doc)]
pub mod jit;

/// AOT (Ahead-of-Time) compilation module (Cranelift-based)
/// AOT requires unsafe code for memory management and linking
#[allow(unsafe_code, clippy::missing_safety_doc)]
pub mod aot;

/// Data operations module - DataFrame, Series, and Arrow integration
pub mod data;

/// Documentation generation module
pub mod doc;

/// Garbage collection module - cycle detection and collection
pub mod gc;

/// Test utilities - helpers for testing Stratum code
pub mod testutil;

/// Convenience re-export of lexer
pub use lexer::Lexer;

/// Convenience re-export of parser
pub use parser::Parser;

/// Convenience re-export of type checker
pub use types::TypeChecker;

/// Convenience re-export of bytecode compiler
pub use bytecode::Compiler;

/// Convenience re-export of VM
pub use vm::VM;

/// Convenience re-export of output capture utilities
pub use vm::{with_output_capture, OutputCapture};

/// Convenience re-export of debug types
pub use vm::{
    DebugAction, DebugContext, DebugLocation, DebugStackFrame, DebugState, DebugStepResult,
    DebugVariable, PauseReason,
};

/// Convenience re-export of formatter
pub use formatter::Formatter;

/// Convenience re-export of JIT compiler
pub use jit::JitCompiler;

/// Convenience re-export of AOT compiler
pub use aot::AotCompiler;

/// Convenience re-export of execution mode types
pub use ast::{ExecutionMode, ExecutionModeOverride};

/// Convenience re-export of cycle collector
pub use gc::CycleCollector;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert!(!VERSION.is_empty());
    }

    /// Helper to compile and run a Stratum expression
    fn run_expr(source: &str) -> Result<bytecode::Value, String> {
        let expr = parser::Parser::parse_expression(source).map_err(|e| format!("{:?}", e))?;
        let function = bytecode::Compiler::new()
            .compile_expression(&expr)
            .map_err(|e| format!("{:?}", e))?;
        let mut vm = VM::new();
        vm.run(function).map_err(|e| format!("{}", e))
    }

    #[test]
    fn test_simple_block_return() {
        let result = run_expr("{ 42 }").unwrap();
        assert_eq!(result, bytecode::Value::Int(42));
    }

    #[test]
    fn test_block_with_let() {
        // Block with let then return
        let result = run_expr("{ let x = 5; x }").unwrap();
        eprintln!("Block with let: {:?}", result);
        assert_eq!(result, bytecode::Value::Int(5));
    }

    #[test]
    fn test_inline_method() {
        // Method call without block
        let result = run_expr("[1, 2, 3].len()").unwrap();
        eprintln!("Inline method result: {:?}", result);
        assert_eq!(result, bytecode::Value::Int(3));
    }

    #[test]
    fn test_list_map() {
        let result = run_expr("{ let nums = [1, 2, 3]; nums.map(|x| x * 2) }").unwrap();

        match result {
            bytecode::Value::List(list) => {
                let items = list.borrow();
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], bytecode::Value::Int(2));
                assert_eq!(items[1], bytecode::Value::Int(4));
                assert_eq!(items[2], bytecode::Value::Int(6));
            }
            _ => panic!("Expected list, got {:?}", result),
        }
    }

    #[test]
    fn test_list_filter() {
        let result = run_expr("{ let nums = [1, 2, 3, 4, 5]; nums.filter(|x| x > 2) }").unwrap();

        match result {
            bytecode::Value::List(list) => {
                let items = list.borrow();
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], bytecode::Value::Int(3));
                assert_eq!(items[1], bytecode::Value::Int(4));
                assert_eq!(items[2], bytecode::Value::Int(5));
            }
            _ => panic!("Expected list, got {:?}", result),
        }
    }

    #[test]
    fn test_list_reduce() {
        let result = run_expr("{ let nums = [1, 2, 3, 4]; nums.reduce(|acc, x| acc + x, 0) }").unwrap();
        assert_eq!(result, bytecode::Value::Int(10));
    }

    #[test]
    fn test_list_reduce_no_initial() {
        let result = run_expr("{ let nums = [1, 2, 3, 4]; nums.reduce(|acc, x| acc + x) }").unwrap();
        assert_eq!(result, bytecode::Value::Int(10));
    }

    #[test]
    fn test_list_find() {
        let result = run_expr("{ let nums = [1, 2, 3, 4, 5]; nums.find(|x| x > 3) }").unwrap();
        assert_eq!(result, bytecode::Value::Int(4));
    }

    #[test]
    fn test_list_find_not_found() {
        let result = run_expr("{ let nums = [1, 2, 3]; nums.find(|x| x > 10) }").unwrap();
        assert_eq!(result, bytecode::Value::Null);
    }

    #[test]
    fn test_list_sort() {
        let result = run_expr("{ let nums = [3, 1, 4, 1, 5, 9, 2, 6]; nums.sort() }").unwrap();

        match result {
            bytecode::Value::List(list) => {
                let items = list.borrow();
                assert_eq!(items.len(), 8);
                assert_eq!(items[0], bytecode::Value::Int(1));
                assert_eq!(items[1], bytecode::Value::Int(1));
                assert_eq!(items[2], bytecode::Value::Int(2));
                assert_eq!(items[3], bytecode::Value::Int(3));
                assert_eq!(items[7], bytecode::Value::Int(9));
            }
            _ => panic!("Expected list, got {:?}", result),
        }
    }

    #[test]
    fn test_list_sort_with_comparator() {
        let result = run_expr("{ let nums = [3, 1, 4, 1, 5]; nums.sort(|a, b| b - a) }").unwrap();

        match result {
            bytecode::Value::List(list) => {
                let items = list.borrow();
                assert_eq!(items.len(), 5);
                // Should be sorted descending
                assert_eq!(items[0], bytecode::Value::Int(5));
                assert_eq!(items[1], bytecode::Value::Int(4));
                assert_eq!(items[2], bytecode::Value::Int(3));
                assert_eq!(items[3], bytecode::Value::Int(1));
                assert_eq!(items[4], bytecode::Value::Int(1));
            }
            _ => panic!("Expected list, got {:?}", result),
        }
    }

    #[test]
    fn test_map_entries() {
        let result = run_expr(r#"{ let m = {"a": 1, "b": 2}; m.entries().len() }"#).unwrap();
        assert_eq!(result, bytecode::Value::Int(2));
    }

    #[test]
    fn test_map_set() {
        let result = run_expr(r#"{ let m = {"a": 1}; m.set("b", 2); m.get("b") }"#).unwrap();
        assert_eq!(result, bytecode::Value::Int(2));
    }

    // ===== String Method Tests =====

    #[test]
    fn test_string_trim_start() {
        let result = run_expr(r#""  hello  ".trim_start()"#).unwrap();
        assert_eq!(result, bytecode::Value::string("hello  "));
    }

    #[test]
    fn test_string_trim_end() {
        let result = run_expr(r#""  hello  ".trim_end()"#).unwrap();
        assert_eq!(result, bytecode::Value::string("  hello"));
    }

    #[test]
    fn test_string_chars() {
        let result = run_expr(r#""abc".chars()"#).unwrap();
        match result {
            bytecode::Value::List(list) => {
                let items = list.borrow();
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], bytecode::Value::string("a"));
                assert_eq!(items[1], bytecode::Value::string("b"));
                assert_eq!(items[2], bytecode::Value::string("c"));
            }
            _ => panic!("Expected list, got {:?}", result),
        }
    }

    #[test]
    fn test_string_substring() {
        // Basic substring
        let result = run_expr(r#""hello world".substring(0, 5)"#).unwrap();
        assert_eq!(result, bytecode::Value::string("hello"));

        // Substring from middle
        let result = run_expr(r#""hello world".substring(6, 11)"#).unwrap();
        assert_eq!(result, bytecode::Value::string("world"));

        // Substring to end (no second arg)
        let result = run_expr(r#""hello world".substring(6)"#).unwrap();
        assert_eq!(result, bytecode::Value::string("world"));

        // Negative indices
        let result = run_expr(r#""hello world".substring(-5)"#).unwrap();
        assert_eq!(result, bytecode::Value::string("world"));
    }

    /// Helper to compile and run a Stratum module, calling main()
    fn run_module(source: &str) -> Result<bytecode::Value, String> {
        let module = parser::Parser::parse_module(source).map_err(|e| format!("{:?}", e))?;
        let function = bytecode::Compiler::new()
            .compile_module(&module)
            .map_err(|e| format!("{:?}", e))?;
        let mut vm = VM::new();
        // Run the module (registers functions)
        vm.run(function).map_err(|e| format!("{}", e))?;

        // Call main if it exists
        if vm.globals().contains_key("main") {
            let main_call = parser::Parser::parse_expression("main()")
                .map_err(|e| format!("{:?}", e))?;
            let main_fn = bytecode::Compiler::new()
                .compile_expression(&main_call)
                .map_err(|e| format!("{:?}", e))?;
            vm.run(main_fn).map_err(|e| format!("{}", e))
        } else {
            Ok(bytecode::Value::Null)
        }
    }

    #[test]
    fn test_while_loop_simple() {
        let source = r#"
            fx main() {
                let n = 3;
                while n > 0 {
                    n = n - 1
                }
            }
        "#;

        let module = parser::Parser::parse_module(source).unwrap();
        let function = bytecode::Compiler::new()
            .compile_module(&module)
            .unwrap();

        let script_bc = bytecode::disassemble_chunk(&function.chunk, "<script>");
        eprintln!("{}", script_bc);

        // Look for main function in constants
        for c in function.chunk.constants() {
            if let bytecode::Value::Function(f) = c {
                let main_bc = bytecode::disassemble_chunk(&f.chunk, &f.name);
                eprintln!("{}", main_bc);
            }
        }

        let result = run_module(source);
        eprintln!("While loop result: {:?}", result);
        assert!(result.is_ok());
    }

    // ===== JIT Integration Tests =====

    #[test]
    fn test_jit_simple_arithmetic() {
        // Test a simple function that should be JIT-compilable
        // Note: Functions need #[compile] attribute to use JIT
        use crate::ast::ExecutionMode;

        // Create a simple function with ExecutionMode::Compile
        let mut chunk = bytecode::Chunk::new();
        // Push 10
        chunk.emit_constant(bytecode::Value::Int(10), 1);
        // Push 32
        chunk.emit_constant(bytecode::Value::Int(32), 1);
        // Add
        chunk.write_op(bytecode::OpCode::Add, 1);
        // Return
        chunk.write_op(bytecode::OpCode::Return, 1);

        let mut function = bytecode::Function::new("test_add".to_string(), 0);
        function.chunk = chunk;
        function.execution_mode = ExecutionMode::Compile;

        // Compile with JIT
        let mut jit = jit::JitCompiler::new();
        let result = jit.compile_function(&function);
        assert!(result.is_ok(), "JIT compilation should succeed");

        // Call the compiled function
        let func_ptr = result.unwrap();
        let compiled = jit::CompiledFunction {
            ptr: func_ptr,
            arity: 0,
            name: "test_add".to_string(),
        };

        let result = jit::call_jit_function(&compiled, &[]);
        assert_eq!(result, bytecode::Value::Int(42));
    }

    #[test]
    fn test_jit_with_parameters() {
        use crate::ast::ExecutionMode;

        // Create a function that takes two parameters and adds them
        let mut chunk = bytecode::Chunk::new();
        // Load first parameter (slot 0)
        chunk.write_op(bytecode::OpCode::LoadLocal, 1);
        chunk.code_mut().extend(&[0, 0]); // u16 slot 0
        // Load second parameter (slot 1)
        chunk.write_op(bytecode::OpCode::LoadLocal, 1);
        chunk.code_mut().extend(&[1, 0]); // u16 slot 1
        // Add
        chunk.write_op(bytecode::OpCode::Add, 1);
        // Return
        chunk.write_op(bytecode::OpCode::Return, 1);

        let mut function = bytecode::Function::new("add_params".to_string(), 2);
        function.chunk = chunk;
        function.execution_mode = ExecutionMode::Compile;

        // Compile with JIT
        let mut jit = jit::JitCompiler::new();
        let result = jit.compile_function(&function);
        assert!(result.is_ok(), "JIT compilation should succeed");

        let func_ptr = result.unwrap();
        let compiled = jit::CompiledFunction {
            ptr: func_ptr,
            arity: 2,
            name: "add_params".to_string(),
        };

        // Call with arguments
        let args = vec![bytecode::Value::Int(100), bytecode::Value::Int(23)];
        let result = jit::call_jit_function(&compiled, &args);
        assert_eq!(result, bytecode::Value::Int(123));
    }

    #[test]
    fn test_jit_comparison() {
        use crate::ast::ExecutionMode;

        // Create a simple comparison function: return a > b
        let mut chunk = bytecode::Chunk::new();
        // Load first parameter (slot 0)
        chunk.write_op(bytecode::OpCode::LoadLocal, 1);
        chunk.code_mut().extend(&[0, 0]); // u16 slot 0
        // Load second parameter (slot 1)
        chunk.write_op(bytecode::OpCode::LoadLocal, 1);
        chunk.code_mut().extend(&[1, 0]); // u16 slot 1
        // Compare: a > b
        chunk.write_op(bytecode::OpCode::Gt, 1);
        // Return
        chunk.write_op(bytecode::OpCode::Return, 1);

        let mut function = bytecode::Function::new("greater_than".to_string(), 2);
        function.chunk = chunk;
        function.execution_mode = ExecutionMode::Compile;

        // Compile
        let mut jit = jit::JitCompiler::new();
        let result = jit.compile_function(&function);
        assert!(result.is_ok(), "JIT compilation should succeed");

        let func_ptr = result.unwrap();
        let compiled = jit::CompiledFunction {
            ptr: func_ptr,
            arity: 2,
            name: "greater_than".to_string(),
        };

        // Test: 10 > 5 should be true
        let result = jit::call_jit_function(
            &compiled,
            &[bytecode::Value::Int(10), bytecode::Value::Int(5)],
        );
        assert_eq!(result, bytecode::Value::Bool(true));

        // Test: 5 > 10 should be false
        let result = jit::call_jit_function(
            &compiled,
            &[bytecode::Value::Int(5), bytecode::Value::Int(10)],
        );
        assert_eq!(result, bytecode::Value::Bool(false));

        // Test: 5 > 5 should be false
        let result = jit::call_jit_function(
            &compiled,
            &[bytecode::Value::Int(5), bytecode::Value::Int(5)],
        );
        assert_eq!(result, bytecode::Value::Bool(false));
    }

    #[test]
    fn test_jit_fallback_on_unsupported() {
        use crate::ast::ExecutionMode;

        // Create a function with a Call opcode (unsupported)
        let mut chunk = bytecode::Chunk::new();
        // Call with 0 args
        chunk.write_op(bytecode::OpCode::Call, 1);
        chunk.code_mut().push(0); // arg count
        chunk.write_op(bytecode::OpCode::Return, 1);

        let mut function = bytecode::Function::new("calls_something".to_string(), 0);
        function.chunk = chunk;
        function.execution_mode = ExecutionMode::Compile;

        // JIT compilation should fail with UnsupportedInstruction
        let mut jit = jit::JitCompiler::new();
        let result = jit.compile_function(&function);
        assert!(result.is_err(), "JIT should fail for unsupported opcodes");

        if let Err(e) = result {
            assert!(e.to_string().contains("Unsupported"), "Error should indicate unsupported instruction");
        }
    }

    #[test]
    fn test_jit_caching() {
        use crate::ast::ExecutionMode;

        // Create a simple function
        let mut chunk = bytecode::Chunk::new();
        chunk.emit_constant(bytecode::Value::Int(42), 1);
        chunk.write_op(bytecode::OpCode::Return, 1);

        let mut function = bytecode::Function::new("cached_fn".to_string(), 0);
        function.chunk = chunk;
        function.execution_mode = ExecutionMode::Compile;

        // First compilation
        let mut jit = jit::JitCompiler::new();
        let ptr1 = jit.compile_function(&function).unwrap();

        // Second compilation should return the same pointer (cached)
        let ptr2 = jit.compile_function(&function).unwrap();

        assert_eq!(ptr1, ptr2, "Cached compilation should return same pointer");
    }

    // ===== Hot Path Detection Tests =====

    #[test]
    fn test_hot_path_threshold_configurable() {
        let mut vm = VM::new();

        // Default threshold should be 1000
        assert_eq!(vm.get_hot_threshold(), 1000);

        // Should be configurable
        vm.set_hot_threshold(100);
        assert_eq!(vm.get_hot_threshold(), 100);

        vm.set_hot_threshold(5);
        assert_eq!(vm.get_hot_threshold(), 5);
    }

    #[test]
    fn test_compile_hot_mode_recognized() {
        use crate::ast::ExecutionMode;

        // CompileHot functions should be recognized by the JIT compiler
        let mut chunk1 = bytecode::Chunk::new();
        chunk1.emit_constant(bytecode::Value::Int(1), 1);
        chunk1.write_op(bytecode::OpCode::Return, 1);

        let mut hot_fn = bytecode::Function::new("hot".to_string(), 0);
        hot_fn.chunk = chunk1;
        hot_fn.execution_mode = ExecutionMode::CompileHot;

        // Compile functions SHOULD be immediately JIT-compiled
        let mut chunk2 = bytecode::Chunk::new();
        chunk2.emit_constant(bytecode::Value::Int(2), 1);
        chunk2.write_op(bytecode::OpCode::Return, 1);

        let mut compile_fn = bytecode::Function::new("compile".to_string(), 0);
        compile_fn.chunk = chunk2;
        compile_fn.execution_mode = ExecutionMode::Compile;

        // JIT should compile the Compile function immediately
        let mut jit = jit::JitCompiler::new();
        let result = jit.compile_function(&compile_fn);
        assert!(result.is_ok(), "Compile mode function should JIT compile");

        // CompileHot function should also compile when explicitly requested
        let result = jit.compile_function(&hot_fn);
        assert!(result.is_ok(), "CompileHot function should compile when explicitly requested");
    }

    #[test]
    fn test_hot_path_function_can_be_jit_compiled() {
        use crate::ast::ExecutionMode;

        // Create a simple CompileHot function
        let mut chunk = bytecode::Chunk::new();
        // Load parameter, add 1, return
        chunk.write_op(bytecode::OpCode::LoadLocal, 1);
        chunk.code_mut().extend(&[0, 0]); // slot 0
        chunk.emit_constant(bytecode::Value::Int(1), 1);
        chunk.write_op(bytecode::OpCode::Add, 1);
        chunk.write_op(bytecode::OpCode::Return, 1);

        let mut function = bytecode::Function::new("increment".to_string(), 1);
        function.chunk = chunk;
        function.execution_mode = ExecutionMode::CompileHot;

        // Manually compile and verify it works as JIT function
        let mut jit = jit::JitCompiler::new();
        let result = jit.compile_function(&function);
        assert!(result.is_ok());

        let func_ptr = result.unwrap();
        let compiled = jit::CompiledFunction {
            ptr: func_ptr,
            arity: 1,
            name: "increment".to_string(),
        };

        // Call the JIT-compiled version
        let result = jit::call_jit_function(&compiled, &[bytecode::Value::Int(41)]);
        assert_eq!(result, bytecode::Value::Int(42));
    }

    #[test]
    fn test_hot_path_integration() {
        // Test that a function marked with CompileHot can be executed
        // This tests the full pipeline from source to execution
        let source = r#"
            #[compile(hot)]
            fx add(a: Int, b: Int) -> Int {
                a + b
            }

            fx main() -> Int {
                add(1, 2)
            }
        "#;

        let result = run_module(source);
        assert!(result.is_ok(), "CompileHot function should execute: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(3));
    }

    // ==================== Pipeline Operator Tests ====================

    #[test]
    fn test_pipeline_bare_function() {
        // Test: a |> f -> f(a)
        let source = r#"
            fx double(x: Int) -> Int { x * 2 }
            fx main() -> Int {
                5 |> double
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Pipeline with bare function: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(10));
    }

    #[test]
    fn test_pipeline_with_args() {
        // Test: a |> f(b) -> f(a, b)
        let source = r#"
            fx add(a: Int, b: Int) -> Int { a + b }
            fx main() -> Int {
                5 |> add(3)
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Pipeline with args: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(8));
    }

    #[test]
    fn test_pipeline_with_placeholder() {
        // Test: a |> f(_, b) -> f(a, b)
        let source = r#"
            fx sub(a: Int, b: Int) -> Int { a - b }
            fx main() -> Int {
                10 |> sub(_, 3)
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Pipeline with placeholder: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(7));
    }

    #[test]
    fn test_pipeline_placeholder_second_position() {
        // Test: a |> f(b, _) -> f(b, a)
        let source = r#"
            fx sub(a: Int, b: Int) -> Int { a - b }
            fx main() -> Int {
                3 |> sub(10, _)
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Pipeline with placeholder in second position: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(7)); // 10 - 3 = 7
    }

    #[test]
    fn test_pipeline_chained() {
        // Test: a |> f |> g -> g(f(a))
        let source = r#"
            fx double(x: Int) -> Int { x * 2 }
            fx add_one(x: Int) -> Int { x + 1 }
            fx main() -> Int {
                5 |> double |> add_one
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Chained pipeline: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(11)); // (5 * 2) + 1 = 11
    }

    #[test]
    fn test_pipeline_chained_with_args() {
        // Test: a |> f(b) |> g(c) -> g(f(a, b), c)
        let source = r#"
            fx add(a: Int, b: Int) -> Int { a + b }
            fx mul(a: Int, b: Int) -> Int { a * b }
            fx main() -> Int {
                5 |> add(3) |> mul(2)
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Chained pipeline with args: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(16)); // (5 + 3) * 2 = 16
    }

    #[test]
    fn test_pipeline_multiple_placeholders() {
        // Test: a |> f(_, _) -> f(a, a)
        let source = r#"
            fx add(a: Int, b: Int) -> Int { a + b }
            fx main() -> Int {
                5 |> add(_, _)
            }
        "#;
        let result = run_module(source);
        assert!(result.is_ok(), "Pipeline with multiple placeholders: {:?}", result.err());
        assert_eq!(result.unwrap(), bytecode::Value::Int(10)); // 5 + 5 = 10
    }
}
