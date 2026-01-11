//! Workspace symbols implementation for Stratum LSP
//!
//! This module provides "workspace symbols" functionality, allowing users to
//! search for symbols across all open documents in the workspace.

use stratum_core::ast::{
    EnumDef, ImplDef, InterfaceDef, Item, ItemKind, Module, PatternKind, StructDef, TopLevelItem,
    TypeKind,
};
use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{Location, Position, Range, SymbolInformation, SymbolKind, Url};

/// Information about a workspace symbol
#[derive(Debug)]
struct SymbolInfo {
    name: String,
    kind: SymbolKind,
    container_name: Option<String>,
    span: Span,
}

/// Compute workspace symbols matching a query across all documents
pub fn compute_workspace_symbols(
    query: &str,
    documents: &[(Url, String)],
) -> Vec<SymbolInformation> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for (uri, content) in documents {
        let line_index = LineIndex::new(content);

        // Parse the module
        let Ok(module) = Parser::parse_module(content) else {
            continue;
        };

        // Collect all symbols from this document
        let symbols = collect_all_symbols(&module);

        // Filter by query and convert to LSP format
        for symbol in symbols {
            // Fuzzy match: symbol name contains query (case insensitive)
            if query.is_empty() || symbol.name.to_lowercase().contains(&query_lower) {
                #[allow(deprecated)]
                results.push(SymbolInformation {
                    name: symbol.name,
                    kind: symbol.kind,
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: span_to_range(symbol.span, &line_index),
                    },
                    container_name: symbol.container_name,
                });
            }
        }
    }

    // Sort by relevance: exact prefix matches first, then by name
    results.sort_by(|a, b| {
        let a_prefix = a.name.to_lowercase().starts_with(&query_lower);
        let b_prefix = b.name.to_lowercase().starts_with(&query_lower);
        match (a_prefix, b_prefix) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    results
}

/// Collect all symbols from a module
fn collect_all_symbols(module: &Module) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    for item in &module.top_level {
        collect_top_level_symbols(item, &mut symbols, None);
    }

    symbols
}

/// Collect symbols from a top-level item
fn collect_top_level_symbols(
    item: &TopLevelItem,
    symbols: &mut Vec<SymbolInfo>,
    container: Option<&str>,
) {
    match item {
        TopLevelItem::Item(item) => collect_item_symbols(item, symbols, container),
        TopLevelItem::Let(let_decl) => {
            if let Some(name) = extract_pattern_name(&let_decl.pattern) {
                symbols.push(SymbolInfo {
                    name,
                    kind: SymbolKind::VARIABLE,
                    container_name: container.map(String::from),
                    span: let_decl.span,
                });
            }
        }
        TopLevelItem::Statement(_) => {}
    }
}

/// Collect symbols from an item
fn collect_item_symbols(item: &Item, symbols: &mut Vec<SymbolInfo>, container: Option<&str>) {
    match &item.kind {
        ItemKind::Function(func) => {
            symbols.push(SymbolInfo {
                name: func.name.name.clone(),
                kind: SymbolKind::FUNCTION,
                container_name: container.map(String::from),
                span: func.span,
            });
        }
        ItemKind::Struct(struct_def) => {
            collect_struct_symbols(struct_def, symbols);
        }
        ItemKind::Enum(enum_def) => {
            collect_enum_symbols(enum_def, symbols);
        }
        ItemKind::Interface(interface_def) => {
            collect_interface_symbols(interface_def, symbols);
        }
        ItemKind::Impl(impl_def) => {
            collect_impl_symbols(impl_def, symbols);
        }
        ItemKind::Import(_) => {}
    }
}

/// Collect struct symbols including fields
fn collect_struct_symbols(struct_def: &StructDef, symbols: &mut Vec<SymbolInfo>) {
    let struct_name = struct_def.name.name.clone();

    symbols.push(SymbolInfo {
        name: struct_name.clone(),
        kind: SymbolKind::STRUCT,
        container_name: None,
        span: struct_def.span,
    });

    // Add fields as child symbols
    for field in &struct_def.fields {
        symbols.push(SymbolInfo {
            name: field.name.name.clone(),
            kind: SymbolKind::FIELD,
            container_name: Some(struct_name.clone()),
            span: field.span,
        });
    }
}

/// Collect enum symbols including variants
fn collect_enum_symbols(enum_def: &EnumDef, symbols: &mut Vec<SymbolInfo>) {
    let enum_name = enum_def.name.name.clone();

    symbols.push(SymbolInfo {
        name: enum_name.clone(),
        kind: SymbolKind::ENUM,
        container_name: None,
        span: enum_def.span,
    });

    // Add variants as child symbols
    for variant in &enum_def.variants {
        symbols.push(SymbolInfo {
            name: variant.name.name.clone(),
            kind: SymbolKind::ENUM_MEMBER,
            container_name: Some(enum_name.clone()),
            span: variant.span,
        });
    }
}

/// Collect interface symbols including methods
fn collect_interface_symbols(interface_def: &InterfaceDef, symbols: &mut Vec<SymbolInfo>) {
    let interface_name = interface_def.name.name.clone();

    symbols.push(SymbolInfo {
        name: interface_name.clone(),
        kind: SymbolKind::INTERFACE,
        container_name: None,
        span: interface_def.span,
    });

    // Add methods as child symbols
    for method in &interface_def.methods {
        symbols.push(SymbolInfo {
            name: method.name.name.clone(),
            kind: SymbolKind::METHOD,
            container_name: Some(interface_name.clone()),
            span: method.span,
        });
    }
}

/// Collect impl block symbols
fn collect_impl_symbols(impl_def: &ImplDef, symbols: &mut Vec<SymbolInfo>) {
    let container_name = type_annotation_to_string(&impl_def.target);

    // Add methods as symbols with the impl target as container
    for method in &impl_def.methods {
        symbols.push(SymbolInfo {
            name: method.name.name.clone(),
            kind: SymbolKind::METHOD,
            container_name: Some(container_name.clone()),
            span: method.span,
        });
    }
}

/// Extract the name from a pattern
fn extract_pattern_name(pattern: &stratum_core::ast::Pattern) -> Option<String> {
    match &pattern.kind {
        PatternKind::Ident(ident) => Some(ident.name.clone()),
        _ => None, // Skip complex patterns for workspace symbols
    }
}

/// Convert a type annotation to a string representation
fn type_annotation_to_string(ty: &stratum_core::ast::TypeAnnotation) -> String {
    match &ty.kind {
        TypeKind::Named { name, args } => {
            if args.is_empty() {
                name.name.clone()
            } else {
                let args_str: Vec<String> = args.iter().map(type_annotation_to_string).collect();
                format!("{}<{}>", name.name, args_str.join(", "))
            }
        }
        TypeKind::Nullable(inner) => {
            format!("{}?", type_annotation_to_string(inner))
        }
        TypeKind::Function { params, ret } => {
            let params_str: Vec<String> = params.iter().map(type_annotation_to_string).collect();
            format!(
                "({}) -> {}",
                params_str.join(", "),
                type_annotation_to_string(ret)
            )
        }
        TypeKind::Tuple(types) => {
            let types_str: Vec<String> = types.iter().map(type_annotation_to_string).collect();
            format!("({})", types_str.join(", "))
        }
        TypeKind::List(inner) => {
            format!("[{}]", type_annotation_to_string(inner))
        }
        TypeKind::Unit => "()".to_string(),
        TypeKind::Never => "!".to_string(),
        TypeKind::Inferred => "_".to_string(),
    }
}

/// Convert a Span to an LSP Range
fn span_to_range(span: Span, line_index: &LineIndex) -> Range {
    let start_loc = line_index.location(span.start);
    let end_loc = line_index.location(span.end);

    Range {
        start: Position {
            line: start_loc.line.saturating_sub(1),
            character: start_loc.column.saturating_sub(1),
        },
        end: Position {
            line: end_loc.line.saturating_sub(1),
            character: end_loc.column.saturating_sub(1),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_function_by_name() {
        let uri = Url::parse("file:///test.strat").unwrap();
        let source = r#"
fx greet(name: String) -> String {
    "Hello, {name}!"
}

fx goodbye() {
    print("Goodbye!")
}
"#;
        let documents = vec![(uri, source.to_string())];
        let results = compute_workspace_symbols("greet", &documents);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "greet");
        assert_eq!(results[0].kind, SymbolKind::FUNCTION);
    }

    #[test]
    fn test_find_struct_and_fields() {
        let uri = Url::parse("file:///test.strat").unwrap();
        let source = r#"
struct Point {
    x: Int,
    y: Int
}
"#;
        let documents = vec![(uri, source.to_string())];

        // Search for struct
        let results = compute_workspace_symbols("Point", &documents);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, SymbolKind::STRUCT);

        // Search for field
        let results = compute_workspace_symbols("x", &documents);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, SymbolKind::FIELD);
        assert_eq!(results[0].container_name, Some("Point".to_string()));
    }

    #[test]
    fn test_find_enum_and_variants() {
        let uri = Url::parse("file:///test.strat").unwrap();
        let source = r#"
enum Color {
    Red,
    Green,
    Blue
}
"#;
        let documents = vec![(uri, source.to_string())];

        // Search for variant
        let results = compute_workspace_symbols("Red", &documents);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, SymbolKind::ENUM_MEMBER);
        assert_eq!(results[0].container_name, Some("Color".to_string()));
    }

    #[test]
    fn test_empty_query_returns_all() {
        let uri = Url::parse("file:///test.strat").unwrap();
        let source = r#"
fx foo() {}
fx bar() {}
struct Baz {}
"#;
        let documents = vec![(uri, source.to_string())];
        let results = compute_workspace_symbols("", &documents);

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_case_insensitive_search() {
        let uri = Url::parse("file:///test.strat").unwrap();
        let source = r#"
fx MyFunction() {}
"#;
        let documents = vec![(uri, source.to_string())];

        let results = compute_workspace_symbols("myfunction", &documents);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "MyFunction");
    }

    #[test]
    fn test_multiple_documents() {
        let uri1 = Url::parse("file:///file1.strat").unwrap();
        let uri2 = Url::parse("file:///file2.strat").unwrap();

        let source1 = "fx foo() {}";
        let source2 = "fx foobar() {}";

        let documents = vec![(uri1, source1.to_string()), (uri2, source2.to_string())];

        let results = compute_workspace_symbols("foo", &documents);
        assert_eq!(results.len(), 2);
    }
}
