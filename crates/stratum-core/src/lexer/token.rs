//! Token types for the Stratum lexer

use logos::Logos;

/// The kind of token produced by the lexer
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r]+")]
pub enum TokenKind {
    // ========== Keywords ==========
    #[token("fx")]
    Fx,
    #[token("let")]
    Let,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("match")]
    Match,
    #[token("return")]
    Return,
    #[token("import")]
    Import,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("interface")]
    Interface,
    #[token("impl")]
    Impl,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("try")]
    Try,
    #[token("catch")]
    Catch,
    #[token("throw")]
    Throw,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("in")]
    In,

    // ========== Literals ==========
    /// Integer literal (parsed value stored separately)
    #[regex(r"[0-9][0-9_]*")]
    Int,

    /// Hexadecimal integer literal
    #[regex(r"0[xX][0-9a-fA-F][0-9a-fA-F_]*")]
    HexInt,

    /// Binary integer literal
    #[regex(r"0[bB][01][01_]*")]
    BinaryInt,

    /// Octal integer literal
    #[regex(r"0[oO][0-7][0-7_]*")]
    OctalInt,

    /// Float literal (including scientific notation)
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9][0-9_]*)?")]
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*")]
    Float,

    /// Boolean true
    #[token("true")]
    True,

    /// Boolean false
    #[token("false")]
    False,

    /// Null literal
    #[token("null")]
    Null,

    // ========== String tokens (for interpolation support) ==========
    /// Start of a multiline string: """ (must match before StringStart)
    #[token("\"\"\"", priority = 10)]
    MultiLineStringStart,

    /// End of a multiline string: """ (produced by lexer's multiline string mode)
    MultiLineStringEnd,

    /// Start of a regular string: "
    #[token("\"")]
    StringStart,

    /// Part of a string literal (between interpolations)
    /// Not matched by logos - produced by the lexer's string mode
    StringPart,

    /// End of a string: " (produced by lexer's string mode)
    StringEnd,

    /// Start of string interpolation: {
    InterpolationStart,

    /// End of string interpolation: }
    InterpolationEnd,

    // ========== Identifiers ==========
    /// Regular identifier (ASCII start, allows Unicode continuation)
    /// Higher priority than `UnicodeIdent` since ASCII identifiers are more common
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 3)]
    Ident,

    /// Unicode identifier (starts with Unicode letter)
    /// Matches non-ASCII identifiers like emoji or CJK characters
    #[regex(r"[\p{XID_Start}][\p{XID_Continue}]*", priority = 2)]
    UnicodeIdent,

    // ========== Operators ==========
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,

    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,

    #[token("|")]
    Pipe,
    #[token("|>")]
    PipeGt,

    /// Ampersand for state binding (&state.field)
    #[token("&")]
    Ampersand,

    #[token("?")]
    Question,
    #[token("?.")]
    QuestionDot,
    #[token("??")]
    DoubleQuestion,

    #[token("=>")]
    FatArrow,
    #[token("->")]
    Arrow,

    // ========== Delimiters ==========
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    /// Hash for attributes: #
    #[token("#")]
    Hash,

    #[token(",")]
    Comma,
    #[token("::")]
    ColonColon,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,

    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token("..=")]
    DotDotEq,

    // ========== Comments (skipped but tracked for potential doc comments) ==========
    /// Line comment: // ...
    #[regex(r"//[^\n]*")]
    LineComment,

    /// Block comment: /* ... */
    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    BlockComment,

    // ========== Special ==========
    #[token("\n")]
    Newline,

    /// End of file (added by lexer, not matched by logos)
    Eof,

    /// Lexer error - invalid character
    Error,
}

impl TokenKind {
    /// Returns true if this token is a keyword
    #[must_use]
    pub const fn is_keyword(&self) -> bool {
        matches!(
            self,
            Self::Fx
                | Self::Let
                | Self::If
                | Self::Else
                | Self::For
                | Self::While
                | Self::Match
                | Self::Return
                | Self::Import
                | Self::Struct
                | Self::Enum
                | Self::Interface
                | Self::Impl
                | Self::Async
                | Self::Await
                | Self::Try
                | Self::Catch
                | Self::Break
                | Self::Continue
                | Self::In
                | Self::True
                | Self::False
                | Self::Null
        )
    }

    /// Returns true if this token is a literal
    #[must_use]
    pub const fn is_literal(&self) -> bool {
        matches!(
            self,
            Self::Int
                | Self::HexInt
                | Self::BinaryInt
                | Self::OctalInt
                | Self::Float
                | Self::True
                | Self::False
                | Self::Null
        )
    }

    /// Returns true if this token is an operator
    #[must_use]
    pub const fn is_operator(&self) -> bool {
        matches!(
            self,
            Self::Plus
                | Self::Minus
                | Self::Star
                | Self::Slash
                | Self::Percent
                | Self::Eq
                | Self::EqEq
                | Self::NotEq
                | Self::Lt
                | Self::Gt
                | Self::LtEq
                | Self::GtEq
                | Self::And
                | Self::Or
                | Self::Not
                | Self::Pipe
                | Self::PipeGt
                | Self::Ampersand
                | Self::Question
                | Self::QuestionDot
                | Self::DoubleQuestion
                | Self::FatArrow
                | Self::Arrow
        )
    }

    /// Returns true if this token should typically be skipped
    #[must_use]
    pub const fn is_trivia(&self) -> bool {
        matches!(self, Self::LineComment | Self::BlockComment | Self::Newline)
    }
}

impl std::fmt::Display for TokenKind {
    #[allow(clippy::match_same_arms)] // Each token type is intentionally separate for clarity
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fx => write!(f, "fx"),
            Self::Let => write!(f, "let"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::For => write!(f, "for"),
            Self::While => write!(f, "while"),
            Self::Match => write!(f, "match"),
            Self::Return => write!(f, "return"),
            Self::Import => write!(f, "import"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Interface => write!(f, "interface"),
            Self::Impl => write!(f, "impl"),
            Self::Async => write!(f, "async"),
            Self::Await => write!(f, "await"),
            Self::Try => write!(f, "try"),
            Self::Catch => write!(f, "catch"),
            Self::Throw => write!(f, "throw"),
            Self::Break => write!(f, "break"),
            Self::Continue => write!(f, "continue"),
            Self::In => write!(f, "in"),
            Self::Int => write!(f, "integer"),
            Self::HexInt => write!(f, "hex integer"),
            Self::BinaryInt => write!(f, "binary integer"),
            Self::OctalInt => write!(f, "octal integer"),
            Self::Float => write!(f, "float"),
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Null => write!(f, "null"),
            Self::MultiLineStringStart => write!(f, "\"\"\""),
            Self::MultiLineStringEnd => write!(f, "\"\"\""),
            Self::StringStart => write!(f, "\""),
            Self::StringPart => write!(f, "string"),
            Self::StringEnd => write!(f, "\""),
            Self::InterpolationStart => write!(f, "{{"),
            Self::InterpolationEnd => write!(f, "}}"),
            Self::Ident => write!(f, "identifier"),
            Self::UnicodeIdent => write!(f, "identifier"),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Percent => write!(f, "%"),
            Self::Eq => write!(f, "="),
            Self::EqEq => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::Gt => write!(f, ">"),
            Self::LtEq => write!(f, "<="),
            Self::GtEq => write!(f, ">="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Not => write!(f, "!"),
            Self::Pipe => write!(f, "|"),
            Self::PipeGt => write!(f, "|>"),
            Self::Ampersand => write!(f, "&"),
            Self::Question => write!(f, "?"),
            Self::QuestionDot => write!(f, "?."),
            Self::DoubleQuestion => write!(f, "??"),
            Self::FatArrow => write!(f, "=>"),
            Self::Arrow => write!(f, "->"),
            Self::LParen => write!(f, "("),
            Self::RParen => write!(f, ")"),
            Self::LBrace => write!(f, "{{"),
            Self::RBrace => write!(f, "}}"),
            Self::LBracket => write!(f, "["),
            Self::RBracket => write!(f, "]"),
            Self::Hash => write!(f, "#"),
            Self::Comma => write!(f, ","),
            Self::ColonColon => write!(f, "::"),
            Self::Colon => write!(f, ":"),
            Self::Semicolon => write!(f, ";"),
            Self::Dot => write!(f, "."),
            Self::DotDot => write!(f, ".."),
            Self::DotDotEq => write!(f, "..="),
            Self::LineComment => write!(f, "// comment"),
            Self::BlockComment => write!(f, "/* comment */"),
            Self::Newline => write!(f, "newline"),
            Self::Eof => write!(f, "end of file"),
            Self::Error => write!(f, "error"),
        }
    }
}
