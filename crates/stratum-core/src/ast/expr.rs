//! Expression AST nodes for the Stratum programming language

use crate::lexer::Span;

use super::{Block, Ident, Spanned, TypeAnnotation};

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic
    /// Addition (+)
    Add,
    /// Subtraction (-)
    Sub,
    /// Multiplication (*)
    Mul,
    /// Division (/)
    Div,
    /// Modulo (%)
    Mod,

    // Comparison
    /// Equal (==)
    Eq,
    /// Not equal (!=)
    Ne,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    Le,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    Ge,

    // Logical
    /// Logical AND (&&)
    And,
    /// Logical OR (||)
    Or,

    // Pipeline
    /// Pipeline operator (|>)
    Pipe,

    // Null handling
    /// Null coalescing (??)
    NullCoalesce,

    // Range
    /// Exclusive range (..)
    Range,
    /// Inclusive range (..=)
    RangeInclusive,
}

impl BinOp {
    /// Returns the precedence of the operator (higher = binds tighter)
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            // Pipeline has lowest precedence
            BinOp::Pipe => 1,

            // Null coalescing
            BinOp::NullCoalesce => 2,

            // Logical OR
            BinOp::Or => 3,

            // Logical AND
            BinOp::And => 4,

            // Comparison
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 5,

            // Range (between comparison and addition)
            BinOp::Range | BinOp::RangeInclusive => 6,

            // Addition and subtraction
            BinOp::Add | BinOp::Sub => 7,

            // Multiplication, division, modulo
            BinOp::Mul | BinOp::Div | BinOp::Mod => 8,
        }
    }

    /// Returns true if the operator is left-associative
    #[must_use]
    pub const fn is_left_associative(self) -> bool {
        // Pipeline is right-associative for chaining: a |> b |> c = a |> (b |> c)
        // Null coalescing is right-associative: a ?? b ?? c = a ?? (b ?? c)
        !matches!(self, BinOp::Pipe | BinOp::NullCoalesce)
    }

    /// Returns the symbol representation of the operator
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Pipe => "|>",
            BinOp::NullCoalesce => "??",
            BinOp::Range => "..",
            BinOp::RangeInclusive => "..=",
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// Negation (-)
    Neg,
    /// Logical NOT (!)
    Not,
}

impl UnaryOp {
    /// Returns the symbol representation of the operator
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
    }
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer literal (e.g., 42, 0xFF, 0b1010)
    Int(i64),
    /// Floating-point literal (e.g., 3.14, 1.0e10)
    Float(f64),
    /// String literal (simple, no interpolation)
    String(String),
    /// Boolean literal (true/false)
    Bool(bool),
    /// Null literal
    Null,
}

/// An expression with source location
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// The kind of expression
    pub kind: ExprKind,
    /// Source location
    pub span: Span,
}

impl Expr {
    /// Create a new expression
    #[must_use]
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Create a literal expression
    #[must_use]
    pub fn literal(lit: Literal, span: Span) -> Self {
        Self::new(ExprKind::Literal(lit), span)
    }

    /// Create an identifier expression
    #[must_use]
    pub fn ident(name: impl Into<String>, span: Span) -> Self {
        Self::new(ExprKind::Ident(Ident::new(name, span)), span)
    }
}

impl Spanned for Expr {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of expression (without source location)
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Literal value (42, 3.14, "hello", true, null)
    Literal(Literal),

    /// Identifier reference
    Ident(Ident),

    /// Binary operation (a + b, x == y, etc.)
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    /// Unary operation (-x, !flag)
    Unary { op: UnaryOp, expr: Box<Expr> },

    /// Parenthesized expression
    Paren(Box<Expr>),

    /// Function call (callee(args...))
    Call { callee: Box<Expr>, args: Vec<Expr> },

    /// Index access (expr[index])
    Index { expr: Box<Expr>, index: Box<Expr> },

    /// Field access (expr.field)
    Field { expr: Box<Expr>, field: Ident },

    /// Null-safe field access (expr?.field)
    NullSafeField { expr: Box<Expr>, field: Ident },

    /// Null-safe index access (expr?.[index])
    NullSafeIndex { expr: Box<Expr>, index: Box<Expr> },

    /// If expression (if cond { then } else { else_ })
    If {
        cond: Box<Expr>,
        then_branch: Block,
        else_branch: Option<ElseBranch>,
    },

    /// Match expression
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Lambda expression (|params| body or |params| -> Type { body })
    Lambda {
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Box<Expr>,
    },

    /// Block expression ({ stmts; expr })
    Block(Block),

    /// List literal ([1, 2, 3])
    List(Vec<Expr>),

    /// Map literal ({"key": value})
    Map(Vec<(Expr, Expr)>),

    /// String interpolation ("Hello, {name}!")
    StringInterp {
        /// Alternating string parts and expressions
        /// Always starts and ends with a string (possibly empty)
        parts: Vec<StringPart>,
    },

    /// Await expression (await expr)
    Await(Box<Expr>),

    /// Try expression (try expr)
    Try(Box<Expr>),

    /// Struct instantiation (Foo { field: value })
    StructInit { name: Ident, fields: Vec<FieldInit> },

    /// Enum variant with data (Some(value))
    EnumVariant {
        enum_name: Option<Ident>,
        variant: Ident,
        data: Option<Box<Expr>>,
    },
}

/// A part of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal string content
    Literal(String),
    /// Interpolated expression
    Expr(Expr),
}

/// Else branch of an if expression
#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    /// else { block }
    Block(Block),
    /// else if ...
    ElseIf(Box<Expr>),
}

/// A match arm (pattern => expression)
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// The pattern to match
    pub pattern: Pattern,
    /// Optional guard condition
    pub guard: Option<Expr>,
    /// The expression to evaluate if matched
    pub body: Expr,
    /// Source location
    pub span: Span,
}

/// A pattern for matching
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// The kind of pattern
    pub kind: PatternKind,
    /// Source location
    pub span: Span,
}

impl Pattern {
    /// Create a new pattern
    #[must_use]
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl Spanned for Pattern {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of pattern
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    /// Wildcard pattern (_)
    Wildcard,
    /// Identifier binding (x, name)
    Ident(Ident),
    /// Literal pattern (42, "hello", true)
    Literal(Literal),
    /// Enum variant pattern (Some(x), None)
    Variant {
        enum_name: Option<Ident>,
        variant: Ident,
        data: Option<Box<Pattern>>,
    },
    /// Struct pattern (Point { x, y })
    Struct {
        name: Ident,
        fields: Vec<FieldPattern>,
    },
    /// List pattern ([a, b, c], [head, ..tail])
    List {
        elements: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
    },
    /// Or pattern (A | B)
    Or(Vec<Pattern>),
}

/// A field pattern in a struct pattern
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    /// Field name
    pub name: Ident,
    /// Optional binding pattern (if different from field name)
    pub pattern: Option<Pattern>,
    /// Source location
    pub span: Span,
}

/// A function/lambda parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Parameter name
    pub name: Ident,
    /// Optional type annotation
    pub ty: Option<TypeAnnotation>,
    /// Optional default value
    pub default: Option<Expr>,
    /// Source location
    pub span: Span,
}

impl Param {
    /// Create a new parameter
    #[must_use]
    pub fn new(name: Ident, ty: Option<TypeAnnotation>, default: Option<Expr>, span: Span) -> Self {
        Self {
            name,
            ty,
            default,
            span,
        }
    }

    /// Create a simple parameter with just a name
    #[must_use]
    pub fn simple(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: Ident::new(name, span),
            ty: None,
            default: None,
            span,
        }
    }
}

impl Spanned for Param {
    fn span(&self) -> Span {
        self.span
    }
}

/// A field initializer in a struct instantiation
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    /// Field name
    pub name: Ident,
    /// Field value (if None, uses shorthand: { x } means { x: x })
    pub value: Option<Expr>,
    /// Source location
    pub span: Span,
}
