//! Statement AST nodes for the Stratum programming language

use crate::lexer::Span;

use super::{Block, Expr, Ident, Pattern, Spanned, TypeAnnotation};

/// A statement with source location
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// The kind of statement
    pub kind: StmtKind,
    /// Source location
    pub span: Span,
}

impl Stmt {
    /// Create a new statement
    #[must_use]
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Create an expression statement
    #[must_use]
    pub fn expr(expr: Expr, span: Span) -> Self {
        Self::new(StmtKind::Expr(expr), span)
    }
}

impl Spanned for Stmt {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of statement
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// Variable declaration (let x = value, let x: Type = value)
    Let {
        /// Variable name or destructuring pattern
        pattern: Pattern,
        /// Optional type annotation
        ty: Option<TypeAnnotation>,
        /// Initial value
        value: Expr,
    },

    /// Expression statement (expr;)
    Expr(Expr),

    /// Assignment statement (x = value, arr[0] = value, obj.field = value)
    Assign {
        /// Assignment target
        target: Expr,
        /// New value
        value: Expr,
    },

    /// Compound assignment (x += 1, x -= 1, etc.)
    CompoundAssign {
        /// Assignment target
        target: Expr,
        /// Operator (+, -, *, /, %)
        op: CompoundOp,
        /// Value to apply
        value: Expr,
    },

    /// Return statement (return, return value)
    Return(Option<Expr>),

    /// For loop (for x in iter { body })
    For {
        /// Loop variable pattern
        pattern: Pattern,
        /// Iterator expression
        iter: Expr,
        /// Loop body
        body: Block,
    },

    /// While loop (while cond { body })
    While {
        /// Loop condition
        cond: Expr,
        /// Loop body
        body: Block,
    },

    /// Loop (infinite loop, use break to exit)
    Loop {
        /// Loop body
        body: Block,
    },

    /// Break statement (exits loop)
    Break,

    /// Continue statement (next iteration)
    Continue,

    /// Try-catch statement
    TryCatch {
        /// The block to try
        try_block: Block,
        /// The catch clauses
        catches: Vec<CatchClause>,
        /// Optional finally block
        finally: Option<Block>,
    },

    /// Throw statement
    Throw(Expr),
}

/// Compound assignment operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompoundOp {
    /// +=
    Add,
    /// -=
    Sub,
    /// *=
    Mul,
    /// /=
    Div,
    /// %=
    Mod,
}

impl CompoundOp {
    /// Returns the symbol representation of the operator
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CompoundOp::Add => "+=",
            CompoundOp::Sub => "-=",
            CompoundOp::Mul => "*=",
            CompoundOp::Div => "/=",
            CompoundOp::Mod => "%=",
        }
    }
}

/// A catch clause in a try-catch statement
#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    /// The exception type to catch (optional, None catches all)
    pub exception_type: Option<TypeAnnotation>,
    /// Binding name for the caught exception
    pub binding: Option<Ident>,
    /// Catch body
    pub body: Block,
    /// Source location
    pub span: Span,
}

impl CatchClause {
    /// Create a new catch clause
    #[must_use]
    pub fn new(
        exception_type: Option<TypeAnnotation>,
        binding: Option<Ident>,
        body: Block,
        span: Span,
    ) -> Self {
        Self {
            exception_type,
            binding,
            body,
            span,
        }
    }
}

impl Spanned for CatchClause {
    fn span(&self) -> Span {
        self.span
    }
}
