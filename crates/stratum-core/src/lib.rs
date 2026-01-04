//! Stratum Core - Language engine for the Stratum programming language
//!
//! This crate provides the core functionality:
//! - Lexer: Tokenization of source code
//! - AST: Abstract syntax tree definitions
//! - Parser: AST construction from token stream
//! - Type Checker: Static type analysis
//! - Bytecode: Instruction set and compiler
//! - VM: Bytecode execution

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
}
