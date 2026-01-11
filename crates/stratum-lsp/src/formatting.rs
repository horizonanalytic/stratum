//! Document formatting support for the Stratum LSP
//!
//! This module provides code formatting using the stratum-core formatter.

use stratum_core::formatter::Formatter;
use stratum_core::lexer::LineIndex;
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Compute formatting edits for a document
///
/// Returns a list of text edits that transform the source into formatted code,
/// or None if the source cannot be parsed.
pub fn compute_formatting(source: &str) -> Option<Vec<TextEdit>> {
    // Parse the source
    let module = Parser::parse_module(source).ok()?;

    // Format the module
    let formatted = Formatter::format_module(&module);

    // If the source is already formatted, return empty edits
    if source == formatted {
        return Some(vec![]);
    }

    // Create a single edit that replaces the entire document
    // This is simpler and more reliable than computing minimal diffs
    let line_index = LineIndex::new(source);
    let end_line = line_index.line_count().saturating_sub(1);
    let end_char = if end_line > 0 {
        let last_line_start = line_index.line_start(end_line).unwrap_or(0) as usize;
        source.len().saturating_sub(last_line_start)
    } else {
        source.len()
    };

    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: end_line as u32,
                character: end_char as u32,
            },
        },
        new_text: formatted,
    }])
}

/// Compute formatting edits for a range within a document
///
/// Note: Currently formats the entire document as range formatting
/// requires more sophisticated diffing. Returns None if source cannot be parsed.
pub fn compute_range_formatting(source: &str, _range: Range) -> Option<Vec<TextEdit>> {
    // For now, we format the entire document
    // True range formatting would require:
    // 1. Identifying which AST nodes fall within the range
    // 2. Formatting only those nodes
    // 3. Computing diffs
    compute_formatting(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_simple_function() {
        let source = "fx add(a:Int,b:Int)->Int{a+b}";
        let edits = compute_formatting(source);

        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 1);

        let formatted = &edits[0].new_text;
        assert!(formatted.contains("fx add(a: Int, b: Int) -> Int {"));
        assert!(formatted.contains("a + b"));
    }

    #[test]
    fn test_formatting_already_formatted() {
        let source = "fx add(a: Int, b: Int) -> Int {\n    a + b\n}\n";
        let edits = compute_formatting(source);

        assert!(edits.is_some());
        let edits = edits.unwrap();
        // Already formatted, so no edits needed
        assert!(edits.is_empty());
    }

    #[test]
    fn test_formatting_invalid_source() {
        let source = "fx incomplete(";
        let edits = compute_formatting(source);

        // Should return None for invalid source
        assert!(edits.is_none());
    }

    #[test]
    fn test_formatting_struct() {
        let source = "struct Point{x:Int,y:Int}";
        let edits = compute_formatting(source);

        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 1);

        let formatted = &edits[0].new_text;
        assert!(formatted.contains("struct Point {"));
        assert!(formatted.contains("    pub x: Int"));
    }

    #[test]
    fn test_formatting_preserves_comments() {
        let source = "// Comment\nfx main(){}";
        let edits = compute_formatting(source);

        assert!(edits.is_some());
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 1);

        let formatted = &edits[0].new_text;
        assert!(formatted.contains("// Comment"));
    }
}
