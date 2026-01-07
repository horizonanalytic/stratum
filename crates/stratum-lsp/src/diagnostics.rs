//! Diagnostics computation for Stratum source files
//!
//! This module handles parsing and type-checking source code,
//! then converts errors to LSP diagnostics format.

use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::{ParseError, Parser};
use stratum_core::types::{TypeError, TypeChecker};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::cache::CachedData;

/// Compute diagnostics using cached data
///
/// This uses the pre-parsed AST and type check results from the cache.
pub fn compute_diagnostics_cached(data: &CachedData<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check for parse errors
    if let Some(errors) = data.parse_errors() {
        for error in errors {
            diagnostics.push(parse_error_to_diagnostic(error, data.line_index));
        }
        return diagnostics;
    }

    // Check for type errors
    if let Some(type_result) = data.type_result {
        for error in &type_result.errors {
            diagnostics.push(type_error_to_diagnostic(error, data.line_index));
        }
    }

    diagnostics
}

/// Compute diagnostics for a source file (non-cached version for compatibility)
///
/// This runs the parser and type checker, collecting all errors.
pub fn compute_diagnostics(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let line_index = LineIndex::new(source);

    // Try to parse the module
    match Parser::parse_module(source) {
        Ok(module) => {
            // Parsing succeeded, now type check
            let mut type_checker = TypeChecker::new();
            let result = type_checker.check_module(&module);

            for error in result.errors {
                diagnostics.push(type_error_to_diagnostic(&error, &line_index));
            }
        }
        Err(parse_errors) => {
            // Add all parse errors
            for error in parse_errors {
                diagnostics.push(parse_error_to_diagnostic(&error, &line_index));
            }
        }
    }

    diagnostics
}

/// Convert a Stratum Span to an LSP Range
fn span_to_range(span: Span, line_index: &LineIndex) -> Range {
    let start_loc = line_index.location(span.start);
    let end_loc = line_index.location(span.end);

    Range {
        start: Position {
            // LSP uses 0-indexed lines and columns, LineIndex uses 1-indexed
            line: start_loc.line.saturating_sub(1),
            character: start_loc.column.saturating_sub(1),
        },
        end: Position {
            line: end_loc.line.saturating_sub(1),
            character: end_loc.column.saturating_sub(1),
        },
    }
}

/// Convert a parse error to an LSP diagnostic
fn parse_error_to_diagnostic(error: &ParseError, line_index: &LineIndex) -> Diagnostic {
    let range = span_to_range(error.span, line_index);
    let message = if let Some(hint) = &error.hint {
        format!("{}\nhint: {}", error.kind, hint)
    } else {
        error.kind.to_string()
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("stratum".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert a type error to an LSP diagnostic
fn type_error_to_diagnostic(error: &TypeError, line_index: &LineIndex) -> Diagnostic {
    let range = span_to_range(error.span, line_index);

    let message = if let Some(hint) = &error.hint {
        format!("{}\nhint: {}", error.kind, hint)
    } else {
        error.kind.to_string()
    };

    // Convert related locations to LSP related information
    // Note: Related information requires a URI, but we only have spans.
    // For now, we skip related information since we don't track file URIs here.
    // This could be enhanced later by passing the URI into the diagnostic computation.
    let related_information = if error.related.is_empty() {
        None
    } else {
        // We can't create proper related info without the document URI
        // For now, append related info to the message
        None
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("stratum".to_string()),
        message,
        related_information,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_errors_for_valid_code() {
        let source = r#"
            fx add(a: Int, b: Int) -> Int {
                a + b
            }
        "#;
        let diagnostics = compute_diagnostics(source);
        assert!(diagnostics.is_empty(), "Expected no diagnostics, got: {:?}", diagnostics);
    }

    #[test]
    fn test_parse_error_detected() {
        let source = "fx broken(";
        let diagnostics = compute_diagnostics(source);
        assert!(!diagnostics.is_empty(), "Expected parse error diagnostic");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_type_error_detected() {
        let source = r#"
            fx main() {
                let x: Int = "hello"
            }
        "#;
        let diagnostics = compute_diagnostics(source);
        assert!(!diagnostics.is_empty(), "Expected type error diagnostic");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_undefined_variable_error() {
        let source = r#"
            fx main() {
                unknown_var
            }
        "#;
        let diagnostics = compute_diagnostics(source);
        assert!(!diagnostics.is_empty(), "Expected undefined variable error");
        assert!(
            diagnostics[0].message.contains("undefined"),
            "Message should mention 'undefined': {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn test_span_to_range_single_line() {
        let source = "let x = 42";
        let line_index = LineIndex::new(source);
        let span = Span::new(4, 5); // 'x'
        let range = span_to_range(span, &line_index);

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 4);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_span_to_range_multiline() {
        let source = "line1\nline2\nline3";
        let line_index = LineIndex::new(source);

        // Span covering "line2" (bytes 6-11)
        let span = Span::new(6, 11);
        let range = span_to_range(span, &line_index);

        assert_eq!(range.start.line, 1); // 0-indexed, so line 2 is index 1
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 5);
    }
}
