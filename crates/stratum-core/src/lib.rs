//! Stratum Core - Language engine for the Stratum programming language
//!
//! This crate provides the core functionality:
//! - Lexer: Tokenization of source code
//! - AST: Abstract syntax tree definitions
//! - Parser: AST construction from token stream
//! - Type Checker: Static type analysis
//! - VM: Bytecode execution (TODO)

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

/// Convenience re-export of lexer
pub use lexer::Lexer;

/// Convenience re-export of parser
pub use parser::Parser;

/// Convenience re-export of type checker
pub use types::TypeChecker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert!(!VERSION.is_empty());
    }
}
