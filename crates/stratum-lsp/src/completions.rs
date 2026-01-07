//! Completion implementation for Stratum LSP
//!
//! This module provides code completion functionality, including:
//! - Keyword completions with snippets
//! - Symbol completions (functions, variables, structs, enums)
//! - Struct field completions after `.`

use stratum_core::ast::{
    Expr, ExprKind, ItemKind, Module, StructDef, TopLevelItem,
};
use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, InsertTextFormat, Position,
};

use crate::cache::CachedData;
use crate::definition::{SymbolIndex, SymbolKind};

/// Completion context - what kind of completion is being requested
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// General completion (keywords, symbols in scope)
    General { prefix: String, offset: u32 },
    /// Field access completion (after `.`)
    FieldAccess { receiver_span: Span, field_prefix: String },
}

/// Compute completions using cached data
pub fn compute_completions_cached(data: &CachedData<'_>, position: Position) -> Vec<CompletionItem> {
    let Some(offset) = position_to_offset(data.line_index, position) else {
        return vec![];
    };

    // Determine completion context
    let context = determine_context(data.content, offset);

    // If we don't have a valid AST, fall back to keyword completions
    let Some(module) = data.ast() else {
        return match context {
            CompletionContext::General { prefix, .. } => keyword_completions(&prefix),
            CompletionContext::FieldAccess { .. } => vec![],
        };
    };

    match context {
        CompletionContext::General { prefix, offset } => {
            let mut items = keyword_completions(&prefix);
            items.extend(symbol_completions(module, &prefix, offset));
            items
        }
        CompletionContext::FieldAccess { receiver_span, field_prefix } => {
            field_completions(module, data.content, receiver_span, &field_prefix)
        }
    }
}

/// Compute completions at the given position (non-cached)
pub fn compute_completions(source: &str, position: Position) -> Vec<CompletionItem> {
    let line_index = LineIndex::new(source);
    let Some(offset) = position_to_offset(&line_index, position) else {
        return vec![];
    };

    // Determine completion context
    let context = determine_context(source, offset);

    // Parse the source for symbol information
    let module = match Parser::parse_module(source) {
        Ok(m) => m,
        Err(_) => {
            // Even if parsing fails, we can still provide keyword completions
            return match context {
                CompletionContext::General { prefix, .. } => {
                    keyword_completions(&prefix)
                }
                CompletionContext::FieldAccess { .. } => vec![],
            };
        }
    };

    match context {
        CompletionContext::General { prefix, offset } => {
            let mut items = keyword_completions(&prefix);
            items.extend(symbol_completions(&module, &prefix, offset));
            items
        }
        CompletionContext::FieldAccess { receiver_span, field_prefix } => {
            field_completions(&module, source, receiver_span, &field_prefix)
        }
    }
}

/// Determine the completion context from source and cursor position
fn determine_context(source: &str, offset: u32) -> CompletionContext {
    let offset = (offset as usize).min(source.len());
    let bytes = source.as_bytes();

    // Look backwards from cursor to find context
    let mut pos = offset;

    // Skip any identifier characters we're in the middle of typing
    while pos > 0 && is_ident_char(bytes[pos - 1]) {
        pos -= 1;
    }
    let prefix_start = pos;
    let prefix = source[prefix_start..offset].to_string();

    // Check if there's a `.` before the prefix
    if pos > 0 && bytes[pos - 1] == b'.' {
        // Field access - find the receiver expression
        let dot_pos = pos - 1;
        // Simple heuristic: find the start of the receiver
        let receiver_end = dot_pos;
        let receiver_start = find_receiver_start(source, receiver_end);

        return CompletionContext::FieldAccess {
            receiver_span: Span::new(receiver_start as u32, receiver_end as u32),
            field_prefix: prefix,
        };
    }

    CompletionContext::General { prefix, offset: offset as u32 }
}

/// Find the start of a receiver expression before a dot
fn find_receiver_start(source: &str, end: usize) -> usize {
    let bytes = source.as_bytes();
    let mut pos = end;

    // Skip whitespace
    while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }

    if pos == 0 {
        return 0;
    }

    // Handle closing bracket/paren/brace
    let last = bytes[pos - 1];
    if last == b')' || last == b']' || last == b'}' {
        // Find matching open
        let open = match last {
            b')' => b'(',
            b']' => b'[',
            b'}' => b'{',
            _ => unreachable!(),
        };
        let mut depth = 1;
        pos -= 1;
        while pos > 0 && depth > 0 {
            pos -= 1;
            if bytes[pos] == last {
                depth += 1;
            } else if bytes[pos] == open {
                depth -= 1;
            }
        }
        // Now look for identifier before the open bracket
        while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
    }

    // Collect identifier
    while pos > 0 && is_ident_char(bytes[pos - 1]) {
        pos -= 1;
    }

    pos
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Convert LSP position to byte offset
fn position_to_offset(line_index: &LineIndex, position: Position) -> Option<u32> {
    let line = position.line as usize;
    let character = position.character as usize;
    let line_start = line_index.line_start(line)?;
    Some(line_start + character as u32)
}

/// Generate keyword completions with snippets
fn keyword_completions(prefix: &str) -> Vec<CompletionItem> {
    let keywords = [
        // Declarations with snippets
        ("fx", "fx ${1:name}(${2:params}) {\n\t$0\n}", "Function definition", true),
        ("let", "let ${1:name} = ${0}", "Variable binding", true),
        ("struct", "struct ${1:Name} {\n\t${0}\n}", "Struct definition", true),
        ("enum", "enum ${1:Name} {\n\t${0}\n}", "Enum definition", true),
        ("interface", "interface ${1:Name} {\n\t${0}\n}", "Interface definition", true),
        ("impl", "impl ${1:Type} {\n\t${0}\n}", "Implementation block", true),
        ("import", "import ${0}", "Import statement", true),

        // Control flow with snippets
        ("if", "if ${1:condition} {\n\t${0}\n}", "If statement", true),
        ("else", "else {\n\t${0}\n}", "Else clause", true),
        ("for", "for ${1:item} in ${2:iterable} {\n\t${0}\n}", "For loop", true),
        ("while", "while ${1:condition} {\n\t${0}\n}", "While loop", true),
        ("match", "match ${1:value} {\n\t${0}\n}", "Match expression", true),

        // Error handling with snippets
        ("try", "try {\n\t${1}\n} catch ${2:e} {\n\t${0}\n}", "Try-catch block", true),
        ("catch", "catch ${1:e} {\n\t${0}\n}", "Catch clause", true),
        ("throw", "throw ${0}", "Throw expression", true),

        // Keywords without snippets
        ("return", "return", "Return from function", false),
        ("break", "break", "Break from loop", false),
        ("continue", "continue", "Continue loop", false),
        ("async", "async", "Async modifier", false),
        ("await", "await", "Await expression", false),
        ("in", "in", "In keyword", false),

        // Literals
        ("true", "true", "Boolean true", false),
        ("false", "false", "Boolean false", false),
        ("null", "null", "Null value", false),
    ];

    let prefix_lower = prefix.to_lowercase();
    keywords
        .iter()
        .filter(|(kw, _, _, _)| kw.starts_with(&prefix_lower))
        .map(|(kw, snippet, detail, is_snippet)| {
            CompletionItem {
                label: (*kw).to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some((*detail).to_string()),
                insert_text: Some((*snippet).to_string()),
                insert_text_format: if *is_snippet {
                    Some(InsertTextFormat::SNIPPET)
                } else {
                    Some(InsertTextFormat::PLAIN_TEXT)
                },
                // Sort keywords after symbols for better relevance
                sort_text: Some(format!("1_{}", kw)),
                ..Default::default()
            }
        })
        .collect()
}

/// Generate symbol completions using the symbol index
fn symbol_completions(module: &Module, prefix: &str, position: u32) -> Vec<CompletionItem> {
    let index = SymbolIndex::from_module(module);
    let prefix_lower = prefix.to_lowercase();
    let mut items = Vec::new();

    // Get all symbols that match the prefix
    for (name, kind) in index.all_symbols_matching(prefix, position) {
        let (lsp_kind, detail) = match kind {
            SymbolKind::Function => (CompletionItemKind::FUNCTION, "function"),
            SymbolKind::Struct => (CompletionItemKind::STRUCT, "struct"),
            SymbolKind::Enum => (CompletionItemKind::ENUM, "enum"),
            SymbolKind::Interface => (CompletionItemKind::INTERFACE, "interface"),
            SymbolKind::Variable => (CompletionItemKind::VARIABLE, "variable"),
            SymbolKind::Parameter => (CompletionItemKind::VARIABLE, "parameter"),
            SymbolKind::Field => (CompletionItemKind::FIELD, "field"),
            SymbolKind::EnumVariant => (CompletionItemKind::ENUM_MEMBER, "variant"),
        };

        // Only include if matches prefix (case-insensitive)
        if name.to_lowercase().starts_with(&prefix_lower) {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(lsp_kind),
                detail: Some(detail.to_string()),
                // Sort symbols before keywords
                sort_text: Some(format!("0_{}", name)),
                ..Default::default()
            });
        }
    }

    items
}

/// Generate field completions for struct access
fn field_completions(
    module: &Module,
    source: &str,
    receiver_span: Span,
    field_prefix: &str,
) -> Vec<CompletionItem> {
    // Extract the receiver text and try to infer its type
    let receiver_text = &source[receiver_span.start as usize..receiver_span.end as usize];
    let receiver_text = receiver_text.trim();

    // Simple heuristic: if receiver is an identifier, look for a variable of that name
    // and try to determine its struct type
    let struct_name = infer_struct_type(module, receiver_text);

    let Some(struct_name) = struct_name else {
        return vec![];
    };

    // Find the struct definition and list its fields
    let Some(struct_def) = find_struct_def(module, &struct_name) else {
        return vec![];
    };

    let prefix_lower = field_prefix.to_lowercase();
    struct_def
        .fields
        .iter()
        .filter(|f| f.name.name.to_lowercase().starts_with(&prefix_lower))
        .map(|f| {
            let type_str = format_type(&f.ty);
            CompletionItem {
                label: f.name.name.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(type_str),
                sort_text: Some(format!("0_{}", f.name.name)),
                ..Default::default()
            }
        })
        .collect()
}

/// Try to infer the struct type of an expression
fn infer_struct_type(module: &Module, expr_text: &str) -> Option<String> {
    // Very simple heuristic for now:
    // 1. If it looks like a struct instantiation (Name { ... }), return Name
    // 2. If it's a simple identifier, look for let bindings to find type annotation
    // 3. If it's a known type name, return it

    // Check if it's a type name (starts with uppercase)
    if expr_text.chars().next().is_some_and(|c| c.is_uppercase()) {
        // Could be a struct instantiation or type reference
        let name = expr_text.split(&['{', '(', '<', ' '][..]).next()?;
        if find_struct_def(module, name).is_some() {
            return Some(name.to_string());
        }
    }

    // Look for variable definitions with type annotations
    for item in &module.top_level {
        if let TopLevelItem::Let(let_decl) = item {
            if let stratum_core::ast::PatternKind::Ident(ident) = &let_decl.pattern.kind {
                if ident.name == expr_text {
                    // Check if there's a type annotation
                    if let Some(ty) = &let_decl.ty {
                        return extract_type_name(ty);
                    }
                    // Try to infer from value
                    if let Some(type_name) = infer_type_from_expr(&let_decl.value) {
                        return Some(type_name);
                    }
                }
            }
        }
    }

    // Check function local variables - would need more context
    // For now, we'll rely on explicit type annotations

    None
}

/// Extract the simple type name from a type annotation
fn extract_type_name(ty: &stratum_core::ast::TypeAnnotation) -> Option<String> {
    match &ty.kind {
        stratum_core::ast::TypeKind::Named { name, .. } => Some(name.name.clone()),
        stratum_core::ast::TypeKind::Nullable(inner) => extract_type_name(inner),
        _ => None,
    }
}

/// Try to infer type from an expression (struct instantiation)
fn infer_type_from_expr(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::StructInit { name, .. } => Some(name.name.clone()),
        ExprKind::Call { callee, .. } => {
            // If calling a function named like a type, might be constructor
            if let ExprKind::Ident(ident) = &callee.kind {
                if ident.name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    return Some(ident.name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

/// Find a struct definition by name
fn find_struct_def<'a>(module: &'a Module, name: &str) -> Option<&'a StructDef> {
    for item in &module.top_level {
        if let TopLevelItem::Item(item) = item {
            if let ItemKind::Struct(struct_def) = &item.kind {
                if struct_def.name.name == name {
                    return Some(struct_def);
                }
            }
        }
    }
    None
}

/// Format a type annotation as a string
fn format_type(ty: &stratum_core::ast::TypeAnnotation) -> String {
    use stratum_core::ast::TypeKind;
    match &ty.kind {
        TypeKind::Named { name, args } => {
            if args.is_empty() {
                name.name.clone()
            } else {
                let params: Vec<_> = args.iter().map(format_type).collect();
                format!("{}<{}>", name.name, params.join(", "))
            }
        }
        TypeKind::Function { params, ret } => {
            let param_strs: Vec<_> = params.iter().map(format_type).collect();
            format!("fx({}) -> {}", param_strs.join(", "), format_type(ret))
        }
        TypeKind::Nullable(inner) => format!("{}?", format_type(inner)),
        TypeKind::List(inner) => format!("[{}]", format_type(inner)),
        TypeKind::Tuple(types) => {
            let strs: Vec<_> = types.iter().map(format_type).collect();
            format!("({})", strs.join(", "))
        }
        TypeKind::Unit => "()".to_string(),
        TypeKind::Never => "!".to_string(),
        TypeKind::Inferred => "_".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_completion() {
        let items = keyword_completions("f");
        assert!(items.iter().any(|i| i.label == "fx"));
        assert!(items.iter().any(|i| i.label == "for"));
        assert!(items.iter().any(|i| i.label == "false"));
    }

    #[test]
    fn test_keyword_completion_empty_prefix() {
        let items = keyword_completions("");
        // Should return all keywords
        assert!(items.len() > 20);
        assert!(items.iter().any(|i| i.label == "fx"));
        assert!(items.iter().any(|i| i.label == "if"));
        assert!(items.iter().any(|i| i.label == "struct"));
    }

    #[test]
    fn test_context_detection_general() {
        let source = "let x = fo";
        let ctx = determine_context(source, 10);
        match ctx {
            CompletionContext::General { prefix, .. } => {
                assert_eq!(prefix, "fo");
            }
            _ => panic!("Expected General context"),
        }
    }

    #[test]
    fn test_context_detection_field_access() {
        let source = "point.x";
        let ctx = determine_context(source, 7); // After "point."
        match ctx {
            CompletionContext::FieldAccess { field_prefix, .. } => {
                assert_eq!(field_prefix, "x");
            }
            _ => panic!("Expected FieldAccess context"),
        }
    }

    #[test]
    fn test_context_detection_field_access_empty() {
        let source = "point.";
        let ctx = determine_context(source, 6); // Right after "."
        match ctx {
            CompletionContext::FieldAccess { field_prefix, .. } => {
                assert_eq!(field_prefix, "");
            }
            _ => panic!("Expected FieldAccess context"),
        }
    }

    #[test]
    fn test_full_completion() {
        let source = r#"
struct Point {
    x: Int,
    y: Int,
}

fx main() {
    let p = Point { x: 10, y: 20 }

}
"#;
        // Test keyword completion (line 8 has whitespace so position is valid)
        let position = Position { line: 8, character: 4 };
        let items = compute_completions(source, position);
        // Should have keywords and symbols
        assert!(items.iter().any(|i| i.label == "Point"));
        assert!(items.iter().any(|i| i.label == "main"));
    }

    #[test]
    fn test_symbol_completion() {
        let source = r#"
fx helper() {}
fx main() {
    hel
}
"#;
        let position = Position { line: 3, character: 7 };
        let items = compute_completions(source, position);
        assert!(items.iter().any(|i| i.label == "helper"));
    }
}
