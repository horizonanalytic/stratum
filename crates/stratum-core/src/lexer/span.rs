//! Source location tracking for the Stratum lexer

#![allow(clippy::cast_possible_truncation)] // We intentionally use u32 for spans; files > 4GB are unsupported

use std::ops::Range;

/// A span representing a range in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Byte offset of the start of the span
    pub start: u32,
    /// Byte offset of the end of the span (exclusive)
    pub end: u32,
}

impl Span {
    /// Create a new span from start and end byte offsets
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Create a span from a Range<usize>
    #[must_use]
    pub fn from_range(range: Range<usize>) -> Self {
        Self {
            start: range.start as u32,
            end: range.end as u32,
        }
    }

    /// Length of the span in bytes
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Returns true if the span is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Create a span that encompasses both self and other
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        let start = if self.start < other.start {
            self.start
        } else {
            other.start
        };
        let end = if self.end > other.end {
            self.end
        } else {
            other.end
        };
        Self { start, end }
    }

    /// Create a dummy span for synthesized tokens
    #[must_use]
    pub const fn dummy() -> Self {
        Self {
            start: u32::MAX,
            end: u32::MAX,
        }
    }

    /// Check if this is a dummy span
    #[must_use]
    pub const fn is_dummy(&self) -> bool {
        self.start == u32::MAX && self.end == u32::MAX
    }

    /// Convert to a Range<usize> for slicing
    #[must_use]
    pub const fn as_range(&self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Self::from_range(range)
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.as_range()
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::dummy()
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Source location with line and column information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// 1-indexed line number
    pub line: u32,
    /// 1-indexed column number (in characters, not bytes)
    pub column: u32,
}

impl Location {
    /// Create a new location
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Maps byte offsets to line/column locations
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offsets where each line starts
    line_starts: Vec<u32>,
}

impl LineIndex {
    /// Build a line index from source code
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { line_starts }
    }

    /// Convert a byte offset to a line/column location
    #[must_use]
    pub fn location(&self, offset: u32) -> Location {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line];
        Location {
            line: (line + 1) as u32,
            column: (offset - line_start + 1),
        }
    }

    /// Get the byte offset where a line starts (0-indexed line number)
    #[must_use]
    pub fn line_start(&self, line: usize) -> Option<u32> {
        self.line_starts.get(line).copied()
    }

    /// Get the number of lines
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_basics() {
        let span = Span::new(5, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
        assert_eq!(span.as_range(), 5..10);
    }

    #[test]
    fn span_merge() {
        let a = Span::new(5, 10);
        let b = Span::new(8, 15);
        let merged = a.merge(b);
        assert_eq!(merged, Span::new(5, 15));
    }

    #[test]
    fn line_index_single_line() {
        let source = "hello world";
        let index = LineIndex::new(source);
        assert_eq!(index.line_count(), 1);
        assert_eq!(index.location(0), Location::new(1, 1));
        assert_eq!(index.location(6), Location::new(1, 7));
    }

    #[test]
    fn line_index_multiple_lines() {
        let source = "line1\nline2\nline3";
        let index = LineIndex::new(source);
        assert_eq!(index.line_count(), 3);
        assert_eq!(index.location(0), Location::new(1, 1));
        assert_eq!(index.location(5), Location::new(1, 6)); // newline char
        assert_eq!(index.location(6), Location::new(2, 1)); // start of line2
        assert_eq!(index.location(12), Location::new(3, 1)); // start of line3
    }
}
