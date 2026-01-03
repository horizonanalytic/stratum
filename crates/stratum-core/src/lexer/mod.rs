//! Lexer for the Stratum programming language
//!
//! The lexer converts source code into a stream of tokens, handling:
//! - Keywords, identifiers, and operators
//! - Numeric literals (int, float, hex, binary, octal)
//! - String literals with interpolation support
//! - Comments (line and block)
//! - Source location tracking

#![allow(clippy::cast_possible_truncation)] // We intentionally use u32 for spans; files > 4GB are unsupported

mod span;
mod token;

pub use span::{LineIndex, Location, Span};
pub use token::TokenKind;

use logos::Logos;
use thiserror::Error;

/// A token with its kind, span, and source text
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The span in the source code
    pub span: Span,
    /// The source text of the token
    pub lexeme: String,
}

impl Token {
    /// Create a new token
    #[must_use]
    pub fn new(kind: TokenKind, span: Span, lexeme: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            lexeme: lexeme.into(),
        }
    }
}

/// String-related tokens emitted during string lexing
#[derive(Debug, Clone, PartialEq)]
pub enum StringToken {
    /// A literal part of the string (no interpolation)
    StringPart(String),
    /// Start of an interpolation: {
    InterpolationStart,
    /// End of an interpolation: }
    InterpolationEnd,
    /// End of the string: "
    StringEnd,
}

/// Lexer error types
#[derive(Error, Debug, Clone, PartialEq)]
pub enum LexError {
    #[error("unexpected character")]
    UnexpectedChar,
    #[error("unterminated string literal")]
    UnterminatedString,
    #[error("unterminated block comment")]
    UnterminatedBlockComment,
    #[error("invalid escape sequence: \\{0}")]
    InvalidEscape(char),
    #[error("unmatched closing brace in string interpolation")]
    UnmatchedCloseBrace,
}

/// A lexer error with location information
#[derive(Debug, Clone)]
pub struct SpannedError {
    pub error: LexError,
    pub span: Span,
}

impl SpannedError {
    #[must_use]
    pub fn new(error: LexError, span: Span) -> Self {
        Self { error, span }
    }
}

impl std::fmt::Display for SpannedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.error, self.span)
    }
}

impl std::error::Error for SpannedError {}

/// Lexer state for handling string interpolation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerMode {
    /// Normal code lexing
    Normal,
    /// Inside a string literal
    String,
    /// Inside an interpolation expression within a string
    Interpolation { depth: u32 },
}

/// The Stratum lexer
pub struct Lexer<'source> {
    source: &'source str,
    /// Current position in the source (byte offset)
    position: usize,
    /// Current lexer mode
    mode: LexerMode,
    /// Stack of modes for nested interpolations
    mode_stack: Vec<LexerMode>,
    /// Collected errors during lexing
    errors: Vec<SpannedError>,
}

impl<'source> Lexer<'source> {
    /// Create a new lexer for the given source code
    #[must_use]
    pub fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
            mode: LexerMode::Normal,
            mode_stack: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Tokenize the entire source, returning all tokens and any errors
    #[must_use]
    pub fn tokenize(source: &str) -> (Vec<Token>, Vec<SpannedError>) {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.collect_all();
        (tokens, lexer.errors)
    }

    /// Collect all tokens from the source
    pub fn collect_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Option<Token> {
        match self.mode {
            LexerMode::Normal => self.lex_normal(),
            LexerMode::String => self.lex_string(),
            LexerMode::Interpolation { .. } => self.lex_interpolation(),
        }
    }

    /// Lex in normal mode using logos
    fn lex_normal(&mut self) -> Option<Token> {
        if self.position >= self.source.len() {
            return Some(Token::new(
                TokenKind::Eof,
                Span::new(self.position as u32, self.position as u32),
                "",
            ));
        }

        let remaining = &self.source[self.position..];
        let mut logos_lexer = TokenKind::lexer(remaining);

        match logos_lexer.next() {
            Some(Ok(kind)) => {
                let span_range = logos_lexer.span();
                let lexeme = logos_lexer.slice();
                // span_range is relative to remaining slice, accounting for skipped whitespace
                let start = self.position + span_range.start;
                let end = self.position + span_range.end;
                self.position = end;

                // Handle string start - switch to string mode
                if kind == TokenKind::StringStart {
                    self.mode = LexerMode::String;
                }

                Some(Token::new(
                    kind,
                    Span::new(start as u32, end as u32),
                    lexeme,
                ))
            }
            Some(Err(())) => {
                // Error recovery: skip the invalid character
                let start = self.position;
                let invalid_char = remaining.chars().next()?;
                let char_len = invalid_char.len_utf8();
                self.position += char_len;

                self.errors.push(SpannedError::new(
                    LexError::UnexpectedChar,
                    Span::new(start as u32, self.position as u32),
                ));

                Some(Token::new(
                    TokenKind::Error,
                    Span::new(start as u32, self.position as u32),
                    &self.source[start..self.position],
                ))
            }
            None => Some(Token::new(
                TokenKind::Eof,
                Span::new(self.position as u32, self.position as u32),
                "",
            )),
        }
    }

    /// Lex inside a string literal
    #[allow(clippy::too_many_lines)] // String lexing is inherently complex with escape handling
    #[allow(clippy::unnecessary_wraps)] // Consistent return type with other lex_* methods
    fn lex_string(&mut self) -> Option<Token> {
        let start = self.position;
        let mut content = String::new();
        let chars: Vec<char> = self.source[self.position..].chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            match c {
                '"' => {
                    // End of string
                    if !content.is_empty() {
                        // First emit the string part
                        let token = Token::new(
                            TokenKind::StringPart,
                            Span::new(start as u32, self.position as u32),
                            &content,
                        );
                        return Some(token);
                    }
                    // Emit string end
                    self.position += 1;
                    self.mode = LexerMode::Normal;
                    return Some(Token::new(
                        TokenKind::StringEnd,
                        Span::new((self.position - 1) as u32, self.position as u32),
                        "\"",
                    ));
                }
                '{' => {
                    // Start of interpolation
                    if !content.is_empty() {
                        // First emit the string part
                        let token = Token::new(
                            TokenKind::StringPart,
                            Span::new(start as u32, self.position as u32),
                            &content,
                        );
                        return Some(token);
                    }
                    // Emit interpolation start
                    let interp_start = self.position;
                    self.position += 1;
                    self.mode_stack.push(LexerMode::String);
                    self.mode = LexerMode::Interpolation { depth: 1 };
                    return Some(Token::new(
                        TokenKind::InterpolationStart,
                        Span::new(interp_start as u32, self.position as u32),
                        "{",
                    ));
                }
                '\\' => {
                    // Escape sequence
                    i += 1;
                    if i >= chars.len() {
                        self.errors.push(SpannedError::new(
                            LexError::UnterminatedString,
                            Span::new(start as u32, self.source.len() as u32),
                        ));
                        self.mode = LexerMode::Normal;
                        return Some(Token::new(
                            TokenKind::Error,
                            Span::new(start as u32, self.source.len() as u32),
                            &self.source[start..],
                        ));
                    }
                    let escaped = chars[i];
                    let escape_char = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        '{' => '{',
                        '}' => '}',
                        '0' => '\0',
                        _ => {
                            self.errors.push(SpannedError::new(
                                LexError::InvalidEscape(escaped),
                                Span::new(
                                    (self.position + i - 1) as u32,
                                    (self.position + i + 1) as u32,
                                ),
                            ));
                            escaped
                        }
                    };
                    content.push(escape_char);
                    self.position += 1 + escaped.len_utf8();
                    i += 1;
                }
                '\n' => {
                    // Unterminated string (newline inside)
                    self.errors.push(SpannedError::new(
                        LexError::UnterminatedString,
                        Span::new(start as u32, self.position as u32),
                    ));
                    self.mode = LexerMode::Normal;
                    if !content.is_empty() {
                        return Some(Token::new(
                            TokenKind::StringPart,
                            Span::new(start as u32, self.position as u32),
                            &content,
                        ));
                    }
                    return Some(Token::new(
                        TokenKind::Error,
                        Span::new(start as u32, self.position as u32),
                        "",
                    ));
                }
                _ => {
                    content.push(c);
                    self.position += c.len_utf8();
                    i += 1;
                }
            }
        }

        // Reached end of input while in string
        self.errors.push(SpannedError::new(
            LexError::UnterminatedString,
            Span::new(start as u32, self.source.len() as u32),
        ));
        self.mode = LexerMode::Normal;
        if content.is_empty() {
            Some(Token::new(
                TokenKind::Error,
                Span::new(start as u32, self.source.len() as u32),
                "",
            ))
        } else {
            Some(Token::new(
                TokenKind::StringPart,
                Span::new(start as u32, self.source.len() as u32),
                &content,
            ))
        }
    }

    /// Lex inside an interpolation expression
    fn lex_interpolation(&mut self) -> Option<Token> {
        if self.position >= self.source.len() {
            self.errors.push(SpannedError::new(
                LexError::UnterminatedString,
                Span::new(self.position as u32, self.position as u32),
            ));
            return Some(Token::new(
                TokenKind::Eof,
                Span::new(self.position as u32, self.position as u32),
                "",
            ));
        }

        let remaining = &self.source[self.position..];

        // Check for closing brace
        if remaining.starts_with('}') {
            if let LexerMode::Interpolation { depth } = self.mode {
                if depth == 1 {
                    // End of interpolation
                    let start = self.position;
                    self.position += 1;
                    self.mode = self.mode_stack.pop().unwrap_or(LexerMode::Normal);
                    return Some(Token::new(
                        TokenKind::InterpolationEnd,
                        Span::new(start as u32, self.position as u32),
                        "}",
                    ));
                }
            }
        }

        // Use normal lexing for the interpolation content
        let mut logos_lexer = TokenKind::lexer(remaining);

        match logos_lexer.next() {
            Some(Ok(kind)) => {
                let span_range = logos_lexer.span();
                let lexeme = logos_lexer.slice();
                // span_range is relative to remaining slice, accounting for skipped whitespace
                let start = self.position + span_range.start;
                let end = self.position + span_range.end;
                self.position = end;

                // Track brace depth
                if let LexerMode::Interpolation { depth } = &mut self.mode {
                    match kind {
                        TokenKind::LBrace => *depth += 1,
                        TokenKind::RBrace => {
                            *depth -= 1;
                            if *depth == 0 {
                                // This shouldn't happen due to the check above, but handle it
                                self.mode = self.mode_stack.pop().unwrap_or(LexerMode::Normal);
                                return Some(Token::new(
                                    TokenKind::InterpolationEnd,
                                    Span::new(start as u32, end as u32),
                                    "}",
                                ));
                            }
                        }
                        TokenKind::StringStart => {
                            // Nested string in interpolation
                            self.mode_stack.push(self.mode);
                            self.mode = LexerMode::String;
                        }
                        _ => {}
                    }
                }

                Some(Token::new(
                    kind,
                    Span::new(start as u32, end as u32),
                    lexeme,
                ))
            }
            Some(Err(())) => {
                // Error recovery
                let start = self.position;
                let invalid_char = remaining.chars().next()?;
                let char_len = invalid_char.len_utf8();
                self.position += char_len;

                self.errors.push(SpannedError::new(
                    LexError::UnexpectedChar,
                    Span::new(start as u32, self.position as u32),
                ));

                Some(Token::new(
                    TokenKind::Error,
                    Span::new(start as u32, self.position as u32),
                    &self.source[start..self.position],
                ))
            }
            None => Some(Token::new(
                TokenKind::Eof,
                Span::new(self.position as u32, self.position as u32),
                "",
            )),
        }
    }

    /// Get all errors collected during lexing
    #[must_use]
    pub fn errors(&self) -> &[SpannedError] {
        &self.errors
    }

    /// Check if any errors occurred
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token()?;
        if token.kind == TokenKind::Eof {
            None
        } else {
            Some(token)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<Token> {
        let (tokens, _) = Lexer::tokenize(source);
        tokens
    }

    fn lex_kinds(source: &str) -> Vec<TokenKind> {
        lex(source).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lex_keywords() {
        assert_eq!(
            lex_kinds("fx let if else for while"),
            vec![
                TokenKind::Fx,
                TokenKind::Let,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::For,
                TokenKind::While,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_identifiers() {
        assert_eq!(
            lex_kinds("foo bar_baz _private camelCase"),
            vec![
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_integers() {
        let tokens = lex("42 0xFF 0b1010 0o777");
        assert_eq!(tokens[0].kind, TokenKind::Int);
        assert_eq!(tokens[0].lexeme, "42");
        assert_eq!(tokens[1].kind, TokenKind::HexInt);
        assert_eq!(tokens[1].lexeme, "0xFF");
        assert_eq!(tokens[2].kind, TokenKind::BinaryInt);
        assert_eq!(tokens[2].lexeme, "0b1010");
        assert_eq!(tokens[3].kind, TokenKind::OctalInt);
        assert_eq!(tokens[3].lexeme, "0o777");
    }

    #[test]
    fn lex_floats() {
        let tokens = lex("3.14 1.0e10 2.5e-3 1e6");
        assert_eq!(tokens[0].kind, TokenKind::Float);
        assert_eq!(tokens[0].lexeme, "3.14");
        assert_eq!(tokens[1].kind, TokenKind::Float);
        assert_eq!(tokens[1].lexeme, "1.0e10");
        assert_eq!(tokens[2].kind, TokenKind::Float);
        assert_eq!(tokens[2].lexeme, "2.5e-3");
        assert_eq!(tokens[3].kind, TokenKind::Float);
        assert_eq!(tokens[3].lexeme, "1e6");
    }

    #[test]
    fn lex_operators() {
        assert_eq!(
            lex_kinds("+ - * / % == != < > <= >= && || !"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Not,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_special_operators() {
        assert_eq!(
            lex_kinds("|> ?? ?. => -> .. ..="),
            vec![
                TokenKind::PipeGt,
                TokenKind::DoubleQuestion,
                TokenKind::QuestionDot,
                TokenKind::FatArrow,
                TokenKind::Arrow,
                TokenKind::DotDot,
                TokenKind::DotDotEq,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_delimiters() {
        assert_eq!(
            lex_kinds("( ) { } [ ] , : ;"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Semicolon,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_comments() {
        assert_eq!(
            lex_kinds("foo // comment\nbar"),
            vec![
                TokenKind::Ident,
                TokenKind::LineComment,
                TokenKind::Newline,
                TokenKind::Ident,
                TokenKind::Eof
            ]
        );

        assert_eq!(
            lex_kinds("foo /* block */ bar"),
            vec![
                TokenKind::Ident,
                TokenKind::BlockComment,
                TokenKind::Ident,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lex_simple_string() {
        let tokens = lex(r#""hello""#);
        assert_eq!(tokens[0].kind, TokenKind::StringStart);
        assert_eq!(tokens[1].kind, TokenKind::StringPart);
        assert_eq!(tokens[1].lexeme, "hello");
        assert_eq!(tokens[2].kind, TokenKind::StringEnd);
    }

    #[test]
    fn lex_string_with_escapes() {
        let tokens = lex(r#""hello\nworld""#);
        assert_eq!(tokens[0].kind, TokenKind::StringStart);
        assert_eq!(tokens[1].kind, TokenKind::StringPart);
        assert_eq!(tokens[1].lexeme, "hello\nworld");
        assert_eq!(tokens[2].kind, TokenKind::StringEnd);
    }

    #[test]
    fn lex_string_interpolation() {
        let tokens = lex(r#""Hello, {name}!""#);
        assert_eq!(tokens[0].kind, TokenKind::StringStart);
        assert_eq!(tokens[1].kind, TokenKind::StringPart);
        assert_eq!(tokens[1].lexeme, "Hello, ");
        assert_eq!(tokens[2].kind, TokenKind::InterpolationStart);
        assert_eq!(tokens[3].kind, TokenKind::Ident);
        assert_eq!(tokens[3].lexeme, "name");
        assert_eq!(tokens[4].kind, TokenKind::InterpolationEnd);
        assert_eq!(tokens[5].kind, TokenKind::StringPart);
        assert_eq!(tokens[5].lexeme, "!");
        assert_eq!(tokens[6].kind, TokenKind::StringEnd);
    }

    #[test]
    fn lex_complex_interpolation() {
        let tokens = lex(r#""result: {a + b}""#);
        assert_eq!(tokens[0].kind, TokenKind::StringStart);
        assert_eq!(tokens[1].kind, TokenKind::StringPart);
        assert_eq!(tokens[2].kind, TokenKind::InterpolationStart);
        assert_eq!(tokens[3].kind, TokenKind::Ident); // a
        assert_eq!(tokens[4].kind, TokenKind::Plus);
        assert_eq!(tokens[5].kind, TokenKind::Ident); // b
        assert_eq!(tokens[6].kind, TokenKind::InterpolationEnd);
        assert_eq!(tokens[7].kind, TokenKind::StringEnd);
    }

    #[test]
    fn lex_function_definition() {
        let source = "fx add(a: Int, b: Int) -> Int { return a + b }";
        let tokens = lex(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();

        assert!(kinds.contains(&TokenKind::Fx));
        assert!(kinds.contains(&TokenKind::Ident));
        assert!(kinds.contains(&TokenKind::Arrow));
        assert!(kinds.contains(&TokenKind::Return));
    }

    #[test]
    fn lex_spans_are_correct() {
        let source = "let x = 42";
        let tokens = lex(source);

        assert_eq!(tokens[0].span, Span::new(0, 3)); // "let"
        assert_eq!(tokens[1].span, Span::new(4, 5)); // "x"
        assert_eq!(tokens[2].span, Span::new(6, 7)); // "="
        assert_eq!(tokens[3].span, Span::new(8, 10)); // "42"
    }

    #[test]
    fn error_recovery_continues() {
        let (tokens, errors) = Lexer::tokenize("let @ x = 5");
        assert!(!errors.is_empty());
        // Should still have parsed the valid tokens
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert!(kinds.contains(&TokenKind::Let));
        assert!(kinds.contains(&TokenKind::Error));
        assert!(kinds.contains(&TokenKind::Ident));
        assert!(kinds.contains(&TokenKind::Eq));
        assert!(kinds.contains(&TokenKind::Int));
    }

    #[test]
    fn lex_empty_string() {
        let tokens = lex(r#""""#);
        assert_eq!(tokens[0].kind, TokenKind::StringStart);
        assert_eq!(tokens[1].kind, TokenKind::StringEnd);
    }

    #[test]
    fn lex_unicode_identifiers() {
        // Unicode identifiers should be recognized
        let tokens = lex("日本語 变量 αβγ");
        assert_eq!(tokens[0].kind, TokenKind::UnicodeIdent);
        assert_eq!(tokens[0].lexeme, "日本語");
        assert_eq!(tokens[1].kind, TokenKind::UnicodeIdent);
        assert_eq!(tokens[1].lexeme, "变量");
        assert_eq!(tokens[2].kind, TokenKind::UnicodeIdent);
        assert_eq!(tokens[2].lexeme, "αβγ");
    }

    #[test]
    fn lex_integers_with_underscores() {
        let tokens = lex("1_000_000 0xFF_FF 0b1010_1010");
        assert_eq!(tokens[0].kind, TokenKind::Int);
        assert_eq!(tokens[0].lexeme, "1_000_000");
        assert_eq!(tokens[1].kind, TokenKind::HexInt);
        assert_eq!(tokens[1].lexeme, "0xFF_FF");
        assert_eq!(tokens[2].kind, TokenKind::BinaryInt);
        assert_eq!(tokens[2].lexeme, "0b1010_1010");
    }

    #[test]
    fn lex_nested_braces_in_interpolation() {
        // Interpolation with nested braces: "value: {map[key]}"
        let tokens = lex(r#""value: {map[key]}""#);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert!(kinds.contains(&TokenKind::InterpolationStart));
        assert!(kinds.contains(&TokenKind::LBracket));
        assert!(kinds.contains(&TokenKind::RBracket));
        assert!(kinds.contains(&TokenKind::InterpolationEnd));
    }

    #[test]
    fn lex_all_keywords() {
        let source = "fx let if else for while match return import struct enum interface impl async await try catch break continue in true false null";
        let tokens = lex(source);
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();

        assert!(kinds.contains(&TokenKind::Fx));
        assert!(kinds.contains(&TokenKind::Let));
        assert!(kinds.contains(&TokenKind::If));
        assert!(kinds.contains(&TokenKind::Else));
        assert!(kinds.contains(&TokenKind::For));
        assert!(kinds.contains(&TokenKind::While));
        assert!(kinds.contains(&TokenKind::Match));
        assert!(kinds.contains(&TokenKind::Return));
        assert!(kinds.contains(&TokenKind::Import));
        assert!(kinds.contains(&TokenKind::Struct));
        assert!(kinds.contains(&TokenKind::Enum));
        assert!(kinds.contains(&TokenKind::Interface));
        assert!(kinds.contains(&TokenKind::Impl));
        assert!(kinds.contains(&TokenKind::Async));
        assert!(kinds.contains(&TokenKind::Await));
        assert!(kinds.contains(&TokenKind::Try));
        assert!(kinds.contains(&TokenKind::Catch));
        assert!(kinds.contains(&TokenKind::Break));
        assert!(kinds.contains(&TokenKind::Continue));
        assert!(kinds.contains(&TokenKind::In));
        assert!(kinds.contains(&TokenKind::True));
        assert!(kinds.contains(&TokenKind::False));
        assert!(kinds.contains(&TokenKind::Null));
    }

    #[test]
    fn lex_multiline_block_comment() {
        let tokens = lex("foo /* multi\nline\ncomment */ bar");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(kinds[0], TokenKind::Ident);
        assert_eq!(kinds[1], TokenKind::BlockComment);
        assert_eq!(kinds[2], TokenKind::Ident);
    }

    #[test]
    fn lex_range_expressions() {
        let tokens = lex("0..10 0..=10");
        assert_eq!(tokens[0].kind, TokenKind::Int);
        assert_eq!(tokens[1].kind, TokenKind::DotDot);
        assert_eq!(tokens[2].kind, TokenKind::Int);
        assert_eq!(tokens[3].kind, TokenKind::Int);
        assert_eq!(tokens[4].kind, TokenKind::DotDotEq);
        assert_eq!(tokens[5].kind, TokenKind::Int);
    }

    #[test]
    fn lex_nullable_operators() {
        let tokens = lex("x?.field ?? default");
        assert_eq!(tokens[0].kind, TokenKind::Ident);
        assert_eq!(tokens[1].kind, TokenKind::QuestionDot);
        assert_eq!(tokens[2].kind, TokenKind::Ident);
        assert_eq!(tokens[3].kind, TokenKind::DoubleQuestion);
        assert_eq!(tokens[4].kind, TokenKind::Ident);
    }

    #[test]
    fn line_index_works_with_lexer() {
        let source = "let x = 42\nlet y = 10";
        let index = LineIndex::new(source);

        let tokens = lex(source);
        // "let" on line 1
        assert_eq!(index.location(tokens[0].span.start), Location::new(1, 1));
        // "y" on line 2
        let y_token = tokens.iter().find(|t| t.lexeme == "y").unwrap();
        assert_eq!(index.location(y_token.span.start), Location::new(2, 5));
    }
}
