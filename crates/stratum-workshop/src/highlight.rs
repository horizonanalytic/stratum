//! Syntax highlighting for Stratum source code
//!
//! Implements iced's Highlighter trait using Stratum's lexer for token-based
//! syntax highlighting in the Workshop editor.

use iced::advanced::text::highlighter::{Format, Highlighter};
use iced::{Color, Font};
use std::ops::Range;
use stratum_core::lexer::{Lexer, TokenKind};

/// Highlight category for token coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    /// Keywords (fx, let, if, else, etc.)
    Keyword,
    /// Control flow keywords (return, break, continue, throw)
    ControlFlow,
    /// Type keywords and type names
    Type,
    /// Function names
    Function,
    /// Numeric literals (int, float, hex, etc.)
    Number,
    /// String literals and parts
    String,
    /// Boolean literals (true, false)
    Boolean,
    /// Null literal
    Null,
    /// Operators (+, -, *, /, etc.)
    Operator,
    /// Punctuation and delimiters
    Punctuation,
    /// Comments (line and block)
    Comment,
    /// Identifiers
    Identifier,
    /// Errors
    Error,
}

impl HighlightKind {
    /// Get the color for this highlight kind based on theme
    #[must_use]
    pub fn color(&self, theme: &HighlightTheme) -> Color {
        match self {
            Self::Keyword => theme.keyword,
            Self::ControlFlow => theme.control_flow,
            Self::Type => theme.type_name,
            Self::Function => theme.function,
            Self::Number => theme.number,
            Self::String => theme.string,
            Self::Boolean => theme.boolean,
            Self::Null => theme.null,
            Self::Operator => theme.operator,
            Self::Punctuation => theme.punctuation,
            Self::Comment => theme.comment,
            Self::Identifier => theme.identifier,
            Self::Error => theme.error,
        }
    }

    /// Map a token kind to a highlight kind
    #[must_use]
    pub fn from_token(kind: &TokenKind) -> Self {
        match kind {
            // Keywords
            TokenKind::Fx
            | TokenKind::Let
            | TokenKind::If
            | TokenKind::Else
            | TokenKind::For
            | TokenKind::While
            | TokenKind::Match
            | TokenKind::Import
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Interface
            | TokenKind::Impl
            | TokenKind::Async
            | TokenKind::Await
            | TokenKind::Try
            | TokenKind::Catch
            | TokenKind::In => Self::Keyword,

            // Control flow
            TokenKind::Return
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Throw => Self::ControlFlow,

            // Numbers
            TokenKind::Int
            | TokenKind::HexInt
            | TokenKind::BinaryInt
            | TokenKind::OctalInt
            | TokenKind::Float => Self::Number,

            // Booleans
            TokenKind::True | TokenKind::False => Self::Boolean,

            // Null
            TokenKind::Null => Self::Null,

            // Strings
            TokenKind::StringStart
            | TokenKind::StringPart
            | TokenKind::StringEnd
            | TokenKind::InterpolationStart
            | TokenKind::InterpolationEnd => Self::String,

            // Comments
            TokenKind::LineComment | TokenKind::BlockComment => Self::Comment,

            // Operators
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Eq
            | TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Not
            | TokenKind::Pipe
            | TokenKind::PipeGt
            | TokenKind::Ampersand
            | TokenKind::Question
            | TokenKind::QuestionDot
            | TokenKind::DoubleQuestion
            | TokenKind::FatArrow
            | TokenKind::Arrow
            | TokenKind::DotDot
            | TokenKind::DotDotEq => Self::Operator,

            // Punctuation
            TokenKind::LParen
            | TokenKind::RParen
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::Hash
            | TokenKind::Comma
            | TokenKind::ColonColon
            | TokenKind::Colon
            | TokenKind::Semicolon
            | TokenKind::Dot => Self::Punctuation,

            // Identifiers
            TokenKind::Ident | TokenKind::UnicodeIdent => Self::Identifier,

            // Whitespace and special
            TokenKind::Newline | TokenKind::Eof => Self::Punctuation,

            // Errors
            TokenKind::Error => Self::Error,
        }
    }
}

/// Color theme for syntax highlighting
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightTheme {
    pub keyword: Color,
    pub control_flow: Color,
    pub type_name: Color,
    pub function: Color,
    pub number: Color,
    pub string: Color,
    pub boolean: Color,
    pub null: Color,
    pub operator: Color,
    pub punctuation: Color,
    pub comment: Color,
    pub identifier: Color,
    pub error: Color,
}

impl Default for HighlightTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl HighlightTheme {
    /// Dark theme (default)
    #[must_use]
    pub fn dark() -> Self {
        Self {
            keyword: Color::from_rgb(0.77, 0.47, 0.82),      // Purple
            control_flow: Color::from_rgb(0.86, 0.40, 0.52), // Pink/Red
            type_name: Color::from_rgb(0.38, 0.73, 0.82),    // Cyan
            function: Color::from_rgb(0.60, 0.80, 0.40),     // Green
            number: Color::from_rgb(0.87, 0.73, 0.46),       // Orange/Yellow
            string: Color::from_rgb(0.80, 0.60, 0.40),       // Orange/Brown
            boolean: Color::from_rgb(0.86, 0.40, 0.52),      // Pink/Red
            null: Color::from_rgb(0.86, 0.40, 0.52),         // Pink/Red
            operator: Color::from_rgb(0.67, 0.67, 0.67),     // Gray
            punctuation: Color::from_rgb(0.60, 0.60, 0.60),  // Darker gray
            comment: Color::from_rgb(0.45, 0.55, 0.45),      // Muted green
            identifier: Color::from_rgb(0.85, 0.85, 0.85),   // Light gray
            error: Color::from_rgb(1.0, 0.3, 0.3),           // Red
        }
    }

    /// Light theme
    #[must_use]
    pub fn light() -> Self {
        Self {
            keyword: Color::from_rgb(0.55, 0.20, 0.60),      // Purple
            control_flow: Color::from_rgb(0.70, 0.15, 0.30), // Pink/Red
            type_name: Color::from_rgb(0.10, 0.50, 0.60),    // Teal
            function: Color::from_rgb(0.30, 0.55, 0.20),     // Green
            number: Color::from_rgb(0.70, 0.50, 0.10),       // Orange
            string: Color::from_rgb(0.60, 0.35, 0.15),       // Brown
            boolean: Color::from_rgb(0.70, 0.15, 0.30),      // Pink/Red
            null: Color::from_rgb(0.70, 0.15, 0.30),         // Pink/Red
            operator: Color::from_rgb(0.35, 0.35, 0.35),     // Dark gray
            punctuation: Color::from_rgb(0.40, 0.40, 0.40),  // Gray
            comment: Color::from_rgb(0.35, 0.50, 0.35),      // Muted green
            identifier: Color::from_rgb(0.15, 0.15, 0.15),   // Near black
            error: Color::from_rgb(0.80, 0.10, 0.10),        // Red
        }
    }
}

/// Settings for the Stratum highlighter
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSettings {
    pub theme: HighlightTheme,
}

impl Default for HighlightSettings {
    fn default() -> Self {
        Self {
            theme: HighlightTheme::default(),
        }
    }
}

/// Stratum syntax highlighter
pub struct StratumHighlighter {
    settings: HighlightSettings,
    current_line: usize,
}

impl Highlighter for StratumHighlighter {
    type Settings = HighlightSettings;
    type Highlight = HighlightKind;
    type Iterator<'a> = Box<dyn Iterator<Item = (Range<usize>, Self::Highlight)> + 'a>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            settings: settings.clone(),
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.settings = new_settings.clone();
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;

        // Tokenize the line
        let (tokens, _errors) = Lexer::tokenize(line);

        // Convert tokens to highlight ranges
        let highlights: Vec<(Range<usize>, HighlightKind)> = tokens
            .into_iter()
            .filter(|t| t.kind != TokenKind::Eof && t.kind != TokenKind::Newline)
            .map(|token| {
                let start = token.span.start as usize;
                let end = token.span.end as usize;
                let kind = HighlightKind::from_token(&token.kind);
                (start..end, kind)
            })
            .collect();

        Box::new(highlights.into_iter())
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

/// Convert a highlight kind to a text format
///
/// This function is used as the `to_format` parameter in TextEditor's `highlight_with` method.
/// It receives the highlight kind from our highlighter and the current iced Theme.
pub fn highlight_to_format(kind: &HighlightKind, _theme: &iced::Theme) -> Format<Font> {
    // Use our dark theme colors for now
    // TODO: Could inspect the iced Theme to pick dark/light colors
    let highlight_theme = HighlightTheme::dark();
    Format {
        color: Some(kind.color(&highlight_theme)),
        font: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_keywords() {
        let settings = HighlightSettings::default();
        let mut highlighter = StratumHighlighter::new(&settings);

        let highlights: Vec<_> = highlighter.highlight_line("fx let if").collect();

        assert_eq!(highlights.len(), 3);
        assert_eq!(highlights[0].1, HighlightKind::Keyword);
        assert_eq!(highlights[1].1, HighlightKind::Keyword);
        assert_eq!(highlights[2].1, HighlightKind::Keyword);
    }

    #[test]
    fn test_highlight_mixed() {
        let settings = HighlightSettings::default();
        let mut highlighter = StratumHighlighter::new(&settings);

        let highlights: Vec<_> = highlighter.highlight_line("let x = 42").collect();

        assert_eq!(highlights.len(), 4);
        assert_eq!(highlights[0].1, HighlightKind::Keyword); // let
        assert_eq!(highlights[1].1, HighlightKind::Identifier); // x
        assert_eq!(highlights[2].1, HighlightKind::Operator); // =
        assert_eq!(highlights[3].1, HighlightKind::Number); // 42
    }

    #[test]
    fn test_highlight_string() {
        let settings = HighlightSettings::default();
        let mut highlighter = StratumHighlighter::new(&settings);

        let highlights: Vec<_> = highlighter.highlight_line(r#""hello""#).collect();

        // StringStart, StringPart, StringEnd
        assert!(highlights.iter().all(|(_, k)| *k == HighlightKind::String));
    }

    #[test]
    fn test_highlight_comment() {
        let settings = HighlightSettings::default();
        let mut highlighter = StratumHighlighter::new(&settings);

        let highlights: Vec<_> = highlighter.highlight_line("// comment").collect();

        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].1, HighlightKind::Comment);
    }

    #[test]
    fn test_theme_colors() {
        let dark = HighlightTheme::dark();
        let light = HighlightTheme::light();

        // Dark and light themes should have different colors
        assert_ne!(dark.keyword, light.keyword);
        assert_ne!(dark.identifier, light.identifier);
    }
}
