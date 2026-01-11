//! Comment handling for the Stratum AST
//!
//! Comments are attached to AST nodes to preserve them during formatting.

use crate::lexer::Span;

/// A comment in the source code
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// The comment text (including // or /* */)
    pub text: String,
    /// Source location
    pub span: Span,
    /// The kind of comment
    pub kind: CommentKind,
}

impl Comment {
    /// Create a new line comment
    #[must_use]
    pub fn line(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            span,
            kind: CommentKind::Line,
        }
    }

    /// Create a new block comment
    #[must_use]
    pub fn block(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            span,
            kind: CommentKind::Block,
        }
    }

    /// Get the comment content without the delimiters
    #[must_use]
    pub fn content(&self) -> &str {
        match self.kind {
            CommentKind::Line => {
                // Strip // prefix
                self.text
                    .strip_prefix("//")
                    .unwrap_or(&self.text)
                    .trim_start()
            }
            CommentKind::Block => {
                // Strip /* prefix and */ suffix
                let s = self.text.strip_prefix("/*").unwrap_or(&self.text);
                s.strip_suffix("*/").unwrap_or(s).trim()
            }
        }
    }

    /// Check if this is a doc comment (/// or /** */)
    #[must_use]
    pub fn is_doc_comment(&self) -> bool {
        match self.kind {
            CommentKind::Line => self.text.starts_with("///"),
            CommentKind::Block => self.text.starts_with("/**"),
        }
    }

    /// Get doc comment content without the delimiters
    /// Returns None if not a doc comment
    #[must_use]
    pub fn doc_content(&self) -> Option<&str> {
        if !self.is_doc_comment() {
            return None;
        }
        match self.kind {
            CommentKind::Line => {
                // Strip /// prefix
                Some(
                    self.text
                        .strip_prefix("///")
                        .unwrap_or(&self.text)
                        .trim_start(),
                )
            }
            CommentKind::Block => {
                // Strip /** prefix and */ suffix
                let s = self.text.strip_prefix("/**").unwrap_or(&self.text);
                Some(s.strip_suffix("*/").unwrap_or(s).trim())
            }
        }
    }
}

/// The kind of comment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    /// Line comment: // ...
    Line,
    /// Block comment: /* ... */
    Block,
}

/// Leading and trailing trivia (comments, blank lines) attached to an AST node
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trivia {
    /// Comments before this node
    pub leading: Vec<Comment>,
    /// Comments after this node on the same line
    pub trailing: Option<Comment>,
}

impl Trivia {
    /// Create empty trivia
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create trivia with leading comments
    #[must_use]
    pub fn with_leading(leading: Vec<Comment>) -> Self {
        Self {
            leading,
            trailing: None,
        }
    }

    /// Check if there are no comments
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leading.is_empty() && self.trailing.is_none()
    }

    /// Add a leading comment
    pub fn add_leading(&mut self, comment: Comment) {
        self.leading.push(comment);
    }

    /// Set the trailing comment
    pub fn set_trailing(&mut self, comment: Comment) {
        self.trailing = Some(comment);
    }

    /// Get all doc comments from leading comments
    #[must_use]
    pub fn doc_comments(&self) -> Vec<&Comment> {
        self.leading.iter().filter(|c| c.is_doc_comment()).collect()
    }

    /// Get combined doc comment text from all leading doc comments
    #[must_use]
    pub fn doc_text(&self) -> Option<String> {
        let doc_lines: Vec<&str> = self
            .leading
            .iter()
            .filter_map(Comment::doc_content)
            .collect();

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }
}
