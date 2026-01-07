//! Compiler error types

use crate::lexer::Span;
use std::fmt;

/// A compilation error
#[derive(Debug, Clone)]
pub struct CompileError {
    /// The kind of error
    pub kind: CompileErrorKind,

    /// Source location
    pub span: Span,

    /// Optional hint for fixing the error
    pub hint: Option<String>,
}

impl CompileError {
    /// Create a new compile error
    #[must_use]
    pub fn new(kind: CompileErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            hint: None,
        }
    }

    /// Add a hint to the error
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CompileError {}

/// The kind of compilation error
#[derive(Debug, Clone)]
pub enum CompileErrorKind {
    /// Too many constants in a chunk (> 65535)
    TooManyConstants,

    /// Too many local variables in a scope (> 65535)
    TooManyLocals,

    /// Too many upvalues in a closure (> 256)
    TooManyUpvalues,

    /// Variable not found
    UndefinedVariable(String),

    /// Duplicate variable in same scope
    DuplicateVariable(String),

    /// Break outside of loop
    BreakOutsideLoop,

    /// Continue outside of loop
    ContinueOutsideLoop,

    /// Return outside of function
    ReturnOutsideFunction,

    /// Jump too large (> 32767 bytes)
    JumpTooLarge,

    /// Too many function parameters (> 255)
    TooManyParameters,

    /// Too many function arguments (> 255)
    TooManyArguments,

    /// Invalid assignment target
    InvalidAssignmentTarget,

    /// Cannot use 'this' outside of a method
    ThisOutsideMethod,

    /// Cannot use 'super' outside of a subclass
    SuperOutsideSubclass,

    /// Unsupported feature (for features not yet implemented)
    Unsupported(String),

    /// Unsupported pattern in a binding position
    UnsupportedPattern,

    /// Placeholder (_) used outside of pipeline expression
    InvalidPlaceholder,

    /// Column shorthand (.column) used outside of valid context
    InvalidColumnShorthand(String),

    /// Internal compiler error
    Internal(String),
}

impl fmt::Display for CompileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileErrorKind::TooManyConstants => {
                write!(f, "Too many constants in one chunk (max 65535)")
            }
            CompileErrorKind::TooManyLocals => {
                write!(f, "Too many local variables in one scope (max 65535)")
            }
            CompileErrorKind::TooManyUpvalues => {
                write!(f, "Too many captured variables in one closure (max 256)")
            }
            CompileErrorKind::UndefinedVariable(name) => {
                write!(f, "Undefined variable '{name}'")
            }
            CompileErrorKind::DuplicateVariable(name) => {
                write!(f, "Variable '{name}' already declared in this scope")
            }
            CompileErrorKind::BreakOutsideLoop => {
                write!(f, "'break' can only be used inside a loop")
            }
            CompileErrorKind::ContinueOutsideLoop => {
                write!(f, "'continue' can only be used inside a loop")
            }
            CompileErrorKind::ReturnOutsideFunction => {
                write!(f, "'return' can only be used inside a function")
            }
            CompileErrorKind::JumpTooLarge => {
                write!(f, "Jump offset too large (code too far apart)")
            }
            CompileErrorKind::TooManyParameters => {
                write!(f, "Too many parameters (max 255)")
            }
            CompileErrorKind::TooManyArguments => {
                write!(f, "Too many arguments (max 255)")
            }
            CompileErrorKind::InvalidAssignmentTarget => {
                write!(f, "Invalid assignment target")
            }
            CompileErrorKind::ThisOutsideMethod => {
                write!(f, "'this' can only be used inside a method")
            }
            CompileErrorKind::SuperOutsideSubclass => {
                write!(f, "'super' can only be used inside a subclass method")
            }
            CompileErrorKind::Unsupported(feature) => {
                write!(f, "Unsupported feature: {feature}")
            }
            CompileErrorKind::UnsupportedPattern => {
                write!(
                    f,
                    "Complex patterns not supported in top-level let bindings"
                )
            }
            CompileErrorKind::InvalidPlaceholder => {
                write!(
                    f,
                    "Placeholder '_' can only be used inside pipeline expressions (|>)"
                )
            }
            CompileErrorKind::InvalidColumnShorthand(name) => {
                write!(
                    f,
                    "Column shorthand '.{name}' can only be used as a function argument in DataFrame operations"
                )
            }
            CompileErrorKind::Internal(msg) => {
                write!(f, "Internal compiler error: {msg}")
            }
        }
    }
}

/// Result type for compilation operations
pub type CompileResult<T> = Result<T, CompileError>;
