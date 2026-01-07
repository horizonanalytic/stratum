//! Document caching for improved LSP performance
//!
//! This module provides caching of parsed ASTs, type check results,
//! symbol indices, and line indices to avoid redundant computation.

use std::sync::Arc;

use stratum_core::ast::Module;
use stratum_core::lexer::LineIndex;
use stratum_core::parser::{ParseError, Parser};
use stratum_core::types::{TypeCheckResult, TypeChecker};

use crate::definition::SymbolIndex;

/// Cached data for a single document
#[derive(Debug, Clone)]
pub struct DocumentCache {
    /// The document content
    content: String,
    /// The document version (for invalidation)
    version: i32,
    /// Cached line index
    line_index: Arc<LineIndex>,
    /// Cached parse result (None if not yet parsed)
    parse_result: Option<ParseResult>,
    /// Cached type check result (None if not yet type-checked)
    type_result: Option<Arc<TypeCheckResult>>,
    /// Cached symbol index (None if not yet built)
    symbol_index: Option<Arc<SymbolIndex>>,
}

/// Result of parsing - either success with AST or failure with errors
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// Successfully parsed module
    Ok(Arc<Module>),
    /// Parse errors
    Err(Vec<ParseError>),
}

impl DocumentCache {
    /// Create a new document cache with the given content
    pub fn new(content: String, version: i32) -> Self {
        let line_index = Arc::new(LineIndex::new(&content));
        Self {
            content,
            version,
            line_index,
            parse_result: None,
            type_result: None,
            symbol_index: None,
        }
    }

    /// Get the document content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the document version
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Get the cached line index
    pub fn line_index(&self) -> &Arc<LineIndex> {
        &self.line_index
    }

    /// Apply an incremental text change to the document
    /// This invalidates all cached analysis results
    pub fn apply_change(&mut self, range: Option<tower_lsp::lsp_types::Range>, text: String, version: i32) {
        if let Some(range) = range {
            // Incremental change - apply the edit
            let start_offset = self.position_to_offset(range.start);
            let end_offset = self.position_to_offset(range.end);

            if let (Some(start), Some(end)) = (start_offset, end_offset) {
                let start = start as usize;
                let end = end as usize;

                // Replace the range with the new text
                let mut new_content = String::with_capacity(
                    self.content.len() - (end - start) + text.len()
                );
                new_content.push_str(&self.content[..start]);
                new_content.push_str(&text);
                new_content.push_str(&self.content[end..]);
                self.content = new_content;
            } else {
                // Fallback to full replacement if offset calculation fails
                self.content = text;
            }
        } else {
            // Full document replacement
            self.content = text;
        }

        // Update version and rebuild line index
        self.version = version;
        self.line_index = Arc::new(LineIndex::new(&self.content));

        // Invalidate cached analysis
        self.parse_result = None;
        self.type_result = None;
        self.symbol_index = None;
    }

    /// Convert an LSP position to a byte offset
    fn position_to_offset(&self, position: tower_lsp::lsp_types::Position) -> Option<u32> {
        let line_start = self.line_index.line_start(position.line as usize)?;
        Some(line_start + position.character)
    }

    /// Get or compute the parsed AST
    /// Returns Ok with the AST if parsing succeeded, or Err with parse errors
    pub fn get_or_parse(&mut self) -> &ParseResult {
        if self.parse_result.is_none() {
            let result = match Parser::parse_module(&self.content) {
                Ok(module) => ParseResult::Ok(Arc::new(module)),
                Err(errors) => ParseResult::Err(errors),
            };
            self.parse_result = Some(result);
        }
        self.parse_result.as_ref().unwrap()
    }

    /// Get the parsed AST if available (without parsing)
    pub fn parsed_ast(&self) -> Option<&Arc<Module>> {
        match &self.parse_result {
            Some(ParseResult::Ok(module)) => Some(module),
            _ => None,
        }
    }

    /// Get or compute the type check result
    /// Returns None if parsing failed
    pub fn get_or_type_check(&mut self) -> Option<&Arc<TypeCheckResult>> {
        // Ensure we have a parsed AST first
        let has_ast = matches!(self.get_or_parse(), ParseResult::Ok(_));

        if !has_ast {
            return None;
        }

        if self.type_result.is_none() {
            if let Some(ParseResult::Ok(module)) = &self.parse_result {
                let mut checker = TypeChecker::new();
                let result = checker.check_module(module);
                self.type_result = Some(Arc::new(result));
            }
        }

        self.type_result.as_ref()
    }

    /// Get or compute the symbol index
    /// Returns None if parsing failed
    pub fn get_or_build_symbol_index(&mut self) -> Option<&Arc<SymbolIndex>> {
        // Ensure we have a parsed AST first
        let has_ast = matches!(self.get_or_parse(), ParseResult::Ok(_));

        if !has_ast {
            return None;
        }

        if self.symbol_index.is_none() {
            if let Some(ParseResult::Ok(module)) = &self.parse_result {
                let index = SymbolIndex::from_module(module);
                self.symbol_index = Some(Arc::new(index));
            }
        }

        self.symbol_index.as_ref()
    }

    /// Get all cached data at once for operations that need multiple pieces
    /// This is more efficient than calling each getter separately
    pub fn get_all_cached(&mut self) -> CachedData<'_> {
        // Parse first
        let _ = self.get_or_parse();

        // Then type check and build symbol index
        let _ = self.get_or_type_check();
        let _ = self.get_or_build_symbol_index();

        CachedData {
            content: &self.content,
            line_index: &self.line_index,
            parse_result: self.parse_result.as_ref(),
            type_result: self.type_result.as_ref(),
            symbol_index: self.symbol_index.as_ref(),
        }
    }
}

/// Borrowed references to all cached data
#[derive(Debug)]
pub struct CachedData<'a> {
    pub content: &'a str,
    pub line_index: &'a Arc<LineIndex>,
    pub parse_result: Option<&'a ParseResult>,
    pub type_result: Option<&'a Arc<TypeCheckResult>>,
    pub symbol_index: Option<&'a Arc<SymbolIndex>>,
}

impl<'a> CachedData<'a> {
    /// Get the AST if parsing succeeded
    pub fn ast(&self) -> Option<&Arc<Module>> {
        match self.parse_result {
            Some(ParseResult::Ok(module)) => Some(module),
            _ => None,
        }
    }

    /// Get parse errors if parsing failed
    pub fn parse_errors(&self) -> Option<&Vec<ParseError>> {
        match self.parse_result {
            Some(ParseResult::Err(errors)) => Some(errors),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{Position, Range};

    #[test]
    fn test_cache_creation() {
        let cache = DocumentCache::new("let x = 42".to_string(), 1);
        assert_eq!(cache.content(), "let x = 42");
        assert_eq!(cache.version(), 1);
    }

    #[test]
    fn test_cache_parsing() {
        let mut cache = DocumentCache::new("fx add(a: Int) -> Int { a }".to_string(), 1);

        // First access triggers parsing
        let result = cache.get_or_parse();
        assert!(matches!(result, ParseResult::Ok(_)));

        // Second access returns cached result (should not re-parse)
        let result2 = cache.get_or_parse();
        assert!(matches!(result2, ParseResult::Ok(_)));
    }

    #[test]
    fn test_cache_invalidation_on_change() {
        let mut cache = DocumentCache::new("let x = 42".to_string(), 1);

        // Parse the document
        let _ = cache.get_or_parse();
        assert!(cache.parse_result.is_some());

        // Apply a change - should invalidate cache
        cache.apply_change(None, "let y = 100".to_string(), 2);
        assert!(cache.parse_result.is_none());
        assert_eq!(cache.content(), "let y = 100");
        assert_eq!(cache.version(), 2);
    }

    #[test]
    fn test_incremental_change() {
        let mut cache = DocumentCache::new("let x = 42".to_string(), 1);

        // Apply an incremental change - change "42" to "100"
        let range = Range {
            start: Position { line: 0, character: 8 },
            end: Position { line: 0, character: 10 },
        };
        cache.apply_change(Some(range), "100".to_string(), 2);

        assert_eq!(cache.content(), "let x = 100");
    }

    #[test]
    fn test_symbol_index_caching() {
        let mut cache = DocumentCache::new(
            "fx hello() { 42 }\nstruct Point { x: Int }".to_string(),
            1,
        );

        // Build symbol index
        let index = cache.get_or_build_symbol_index();
        assert!(index.is_some());

        // Should be cached
        assert!(cache.symbol_index.is_some());
    }

    #[test]
    fn test_parse_error_caching() {
        let mut cache = DocumentCache::new("fx broken(".to_string(), 1);

        let result = cache.get_or_parse();
        assert!(matches!(result, ParseResult::Err(_)));

        // Type check should return None for failed parse
        let type_result = cache.get_or_type_check();
        assert!(type_result.is_none());
    }
}
