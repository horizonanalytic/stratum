//! Abstract Syntax Tree (AST) for the Stratum programming language
//!
//! This module defines the data structures that represent parsed Stratum source code.
//! All AST nodes include source location information via [`Span`] for error reporting.

mod expr;
mod item;
mod pretty;
mod stmt;
mod types;

pub use expr::*;
pub use item::*;
pub use stmt::*;
pub use types::*;

// Re-export Span from lexer for convenience
pub use crate::lexer::Span;

/// A trait for AST nodes that have associated source location information
pub trait Spanned {
    /// Returns the source span of this node
    fn span(&self) -> Span;
}

/// An identifier with its source location
#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    /// The identifier name
    pub name: String,
    /// Source location
    pub span: Span,
}

impl Ident {
    /// Create a new identifier
    #[must_use]
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

impl Spanned for Ident {
    fn span(&self) -> Span {
        self.span
    }
}

/// A block of statements, optionally with a trailing expression
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The statements in the block
    pub stmts: Vec<Stmt>,
    /// Optional trailing expression (the block's value)
    pub expr: Option<Box<Expr>>,
    /// Source location of the entire block (including braces)
    pub span: Span,
}

impl Block {
    /// Create a new block
    #[must_use]
    pub fn new(stmts: Vec<Stmt>, expr: Option<Expr>, span: Span) -> Self {
        Self {
            stmts,
            expr: expr.map(Box::new),
            span,
        }
    }

    /// Create an empty block
    #[must_use]
    pub fn empty(span: Span) -> Self {
        Self {
            stmts: Vec::new(),
            expr: None,
            span,
        }
    }
}

impl Spanned for Block {
    fn span(&self) -> Span {
        self.span
    }
}
