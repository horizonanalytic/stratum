//! Parser error types for the Stratum programming language

use crate::lexer::{Span, TokenKind};
use thiserror::Error;

/// A parser error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The kind of error
    pub kind: ParseErrorKind,
    /// Source location where the error occurred
    pub span: Span,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    #[must_use]
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            hint: None,
        }
    }

    /// Add a hint to this error
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.kind, self.span)?;
        if let Some(hint) = &self.hint {
            write!(f, " (hint: {hint})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// The kind of parse error
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    #[error("unexpected token: found {found}, expected {expected}")]
    UnexpectedToken {
        found: TokenKind,
        expected: ExpectedToken,
    },

    #[error("unexpected end of file")]
    UnexpectedEof,

    #[error("expected expression")]
    ExpectedExpression,

    #[error("expected statement")]
    ExpectedStatement,

    #[error("expected identifier")]
    ExpectedIdentifier,

    #[error("expected type")]
    ExpectedType,

    #[error("expected pattern")]
    ExpectedPattern,

    #[error("expected '{expected}' after {context}")]
    ExpectedAfter {
        expected: &'static str,
        context: &'static str,
    },

    #[error("invalid assignment target")]
    InvalidAssignmentTarget,

    #[error("invalid number literal: {0}")]
    InvalidNumber(String),

    #[error("duplicate parameter name: {0}")]
    DuplicateParameter(String),

    #[error("break outside of loop")]
    BreakOutsideLoop,

    #[error("continue outside of loop")]
    ContinueOutsideLoop,

    #[error("return outside of function")]
    ReturnOutsideFunction,
}

/// What token was expected
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedToken {
    /// A specific token kind
    Token(TokenKind),
    /// One of several possible tokens
    OneOf(Vec<TokenKind>),
    /// A description of what was expected
    Description(String),
}

impl std::fmt::Display for ExpectedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpectedToken::Token(kind) => write!(f, "{kind}"),
            ExpectedToken::OneOf(kinds) => {
                let names: Vec<String> = kinds.iter().map(|k| format!("{k}")).collect();
                write!(f, "one of: {}", names.join(", "))
            }
            ExpectedToken::Description(desc) => write!(f, "{desc}"),
        }
    }
}
