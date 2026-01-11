//! Document symbols implementation for Stratum LSP
//!
//! This module provides "document symbols" functionality, which gives an outline
//! view of all symbols defined in a file.

use stratum_core::ast::{
    EnumDef, EnumVariantData, Function, ImplDef, InterfaceDef, Item, ItemKind, Module, PatternKind,
    StructDef, TopLevelItem, TopLevelLet, TypeKind,
};
use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

use crate::cache::CachedData;

/// Compute document symbols using cached data
pub fn compute_document_symbols_cached(data: &CachedData<'_>) -> Vec<DocumentSymbol> {
    let Some(module) = data.ast() else {
        return vec![];
    };

    collect_module_symbols(module, data.line_index)
}

/// Compute all document symbols for a source file (non-cached)
#[allow(dead_code)] // Standalone API used by tests
pub fn compute_document_symbols(source: &str) -> Vec<DocumentSymbol> {
    let line_index = LineIndex::new(source);

    // Parse the module
    let Ok(module) = Parser::parse_module(source) else {
        return vec![];
    };

    collect_module_symbols(&module, &line_index)
}

/// Collect symbols from a module
fn collect_module_symbols(module: &Module, line_index: &LineIndex) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    for item in &module.top_level {
        if let Some(symbol) = collect_top_level_symbol(item, line_index) {
            symbols.push(symbol);
        }
    }

    symbols
}

/// Collect a symbol from a top-level item
fn collect_top_level_symbol(item: &TopLevelItem, line_index: &LineIndex) -> Option<DocumentSymbol> {
    match item {
        TopLevelItem::Item(item) => collect_item_symbol(item, line_index),
        TopLevelItem::Let(let_decl) => collect_top_level_let_symbol(let_decl, line_index),
        TopLevelItem::Statement(_) => None,
    }
}

/// Collect a symbol from an item
fn collect_item_symbol(item: &Item, line_index: &LineIndex) -> Option<DocumentSymbol> {
    match &item.kind {
        ItemKind::Function(func) => Some(collect_function_symbol(func, line_index)),
        ItemKind::Struct(struct_def) => Some(collect_struct_symbol(struct_def, line_index)),
        ItemKind::Enum(enum_def) => Some(collect_enum_symbol(enum_def, line_index)),
        ItemKind::Interface(interface_def) => {
            Some(collect_interface_symbol(interface_def, line_index))
        }
        ItemKind::Impl(impl_def) => Some(collect_impl_symbol(impl_def, line_index)),
        ItemKind::Import(_) => None,
    }
}

/// Collect a function symbol
fn collect_function_symbol(func: &Function, line_index: &LineIndex) -> DocumentSymbol {
    // Build detail string showing parameters and return type
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            if let Some(ty) = &p.ty {
                format!("{}: {}", p.name.name, type_annotation_to_string(ty))
            } else {
                p.name.name.clone()
            }
        })
        .collect();

    let return_type = func
        .return_type
        .as_ref()
        .map(|ty| format!(" -> {}", type_annotation_to_string(ty)))
        .unwrap_or_default();

    let detail = format!("({}){}", params.join(", "), return_type);

    #[allow(deprecated)]
    DocumentSymbol {
        name: func.name.name.clone(),
        detail: Some(detail),
        kind: if func.is_async {
            SymbolKind::FUNCTION
        } else {
            SymbolKind::FUNCTION
        },
        tags: None,
        deprecated: None,
        range: span_to_range(func.span, line_index),
        selection_range: span_to_range(func.name.span, line_index),
        children: None,
    }
}

/// Collect a struct symbol with field children
fn collect_struct_symbol(struct_def: &StructDef, line_index: &LineIndex) -> DocumentSymbol {
    let children: Vec<DocumentSymbol> = struct_def
        .fields
        .iter()
        .map(|field| {
            let detail = type_annotation_to_string(&field.ty);
            #[allow(deprecated)]
            DocumentSymbol {
                name: field.name.name.clone(),
                detail: Some(detail),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range: span_to_range(field.span, line_index),
                selection_range: span_to_range(field.name.span, line_index),
                children: None,
            }
        })
        .collect();

    #[allow(deprecated)]
    DocumentSymbol {
        name: struct_def.name.name.clone(),
        detail: None,
        kind: SymbolKind::STRUCT,
        tags: None,
        deprecated: None,
        range: span_to_range(struct_def.span, line_index),
        selection_range: span_to_range(struct_def.name.span, line_index),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

/// Collect an enum symbol with variant children
fn collect_enum_symbol(enum_def: &EnumDef, line_index: &LineIndex) -> DocumentSymbol {
    let children: Vec<DocumentSymbol> = enum_def
        .variants
        .iter()
        .map(|variant| {
            let detail = variant
                .data
                .as_ref()
                .map(|data| enum_variant_data_to_string(data));
            #[allow(deprecated)]
            DocumentSymbol {
                name: variant.name.name.clone(),
                detail,
                kind: SymbolKind::ENUM_MEMBER,
                tags: None,
                deprecated: None,
                range: span_to_range(variant.span, line_index),
                selection_range: span_to_range(variant.name.span, line_index),
                children: None,
            }
        })
        .collect();

    #[allow(deprecated)]
    DocumentSymbol {
        name: enum_def.name.name.clone(),
        detail: None,
        kind: SymbolKind::ENUM,
        tags: None,
        deprecated: None,
        range: span_to_range(enum_def.span, line_index),
        selection_range: span_to_range(enum_def.name.span, line_index),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

/// Collect an interface symbol with method children
fn collect_interface_symbol(
    interface_def: &InterfaceDef,
    line_index: &LineIndex,
) -> DocumentSymbol {
    let children: Vec<DocumentSymbol> = interface_def
        .methods
        .iter()
        .map(|method| {
            let params: Vec<String> = method
                .params
                .iter()
                .map(|p| {
                    if let Some(ty) = &p.ty {
                        format!("{}: {}", p.name.name, type_annotation_to_string(ty))
                    } else {
                        p.name.name.clone()
                    }
                })
                .collect();

            let return_type = method
                .return_type
                .as_ref()
                .map(|ty| format!(" -> {}", type_annotation_to_string(ty)))
                .unwrap_or_default();

            let detail = format!("({}){}", params.join(", "), return_type);

            #[allow(deprecated)]
            DocumentSymbol {
                name: method.name.name.clone(),
                detail: Some(detail),
                kind: SymbolKind::METHOD,
                tags: None,
                deprecated: None,
                range: span_to_range(method.span, line_index),
                selection_range: span_to_range(method.name.span, line_index),
                children: None,
            }
        })
        .collect();

    #[allow(deprecated)]
    DocumentSymbol {
        name: interface_def.name.name.clone(),
        detail: None,
        kind: SymbolKind::INTERFACE,
        tags: None,
        deprecated: None,
        range: span_to_range(interface_def.span, line_index),
        selection_range: span_to_range(interface_def.name.span, line_index),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

/// Collect an impl block symbol with method children
fn collect_impl_symbol(impl_def: &ImplDef, line_index: &LineIndex) -> DocumentSymbol {
    let children: Vec<DocumentSymbol> = impl_def
        .methods
        .iter()
        .map(|method| collect_function_symbol(method, line_index))
        .collect();

    // Build the impl name from type and optional interface
    let impl_name = if let Some(interface) = &impl_def.interface {
        format!(
            "impl {} for {}",
            type_annotation_to_string(interface),
            type_annotation_to_string(&impl_def.target)
        )
    } else {
        format!("impl {}", type_annotation_to_string(&impl_def.target))
    };

    #[allow(deprecated)]
    DocumentSymbol {
        name: impl_name,
        detail: None,
        kind: SymbolKind::CLASS,
        tags: None,
        deprecated: None,
        range: span_to_range(impl_def.span, line_index),
        selection_range: span_to_range(impl_def.span, line_index), // No specific name span for impl
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

/// Collect a top-level let symbol
fn collect_top_level_let_symbol(
    let_decl: &TopLevelLet,
    line_index: &LineIndex,
) -> Option<DocumentSymbol> {
    // Extract the variable name from the pattern
    let name = extract_pattern_name(&let_decl.pattern)?;

    let detail = let_decl.ty.as_ref().map(|ty| type_annotation_to_string(ty));

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: SymbolKind::VARIABLE,
        tags: None,
        deprecated: None,
        range: span_to_range(let_decl.span, line_index),
        selection_range: span_to_range(let_decl.pattern.span, line_index),
        children: None,
    })
}

/// Extract the name from a pattern (for simple ident patterns)
fn extract_pattern_name(pattern: &stratum_core::ast::Pattern) -> Option<String> {
    match &pattern.kind {
        PatternKind::Ident(ident) => Some(ident.name.clone()),
        _ => Some("<pattern>".to_string()), // Complex patterns
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

/// Convert enum variant data to a string representation
fn enum_variant_data_to_string(data: &EnumVariantData) -> String {
    match data {
        EnumVariantData::Tuple(types) => {
            let types_str: Vec<String> = types.iter().map(type_annotation_to_string).collect();
            format!("({})", types_str.join(", "))
        }
        EnumVariantData::Struct(fields) => {
            let fields_str: Vec<String> = fields
                .iter()
                .map(|f| format!("{}: {}", f.name.name, type_annotation_to_string(&f.ty)))
                .collect();
            format!("{{ {} }}", fields_str.join(", "))
        }
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
    fn test_function_symbol() {
        let source = r#"
fx greet(name: String) -> String {
    "Hello, {name}!"
}
"#;
        let symbols = compute_document_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
        assert!(symbols[0].detail.as_ref().unwrap().contains("name: String"));
    }

    #[test]
    fn test_struct_with_fields() {
        let source = r#"
struct Point {
    x: Int,
    y: Int
}
"#;
        let symbols = compute_document_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Point");
        assert_eq!(symbols[0].kind, SymbolKind::STRUCT);

        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "x");
        assert_eq!(children[0].kind, SymbolKind::FIELD);
    }

    #[test]
    fn test_enum_with_variants() {
        let source = r#"
enum Color {
    Red,
    Green,
    Blue
}
"#;
        let symbols = compute_document_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, SymbolKind::ENUM);

        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].name, "Red");
        assert_eq!(children[0].kind, SymbolKind::ENUM_MEMBER);
    }

    #[test]
    fn test_top_level_variable() {
        let source = r#"
let PI = 3.14159
"#;
        let symbols = compute_document_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "PI");
        assert_eq!(symbols[0].kind, SymbolKind::VARIABLE);
    }

    #[test]
    fn test_multiple_symbols() {
        let source = r#"
struct Point {
    x: Int,
    y: Int
}

fx distance(p1: Point, p2: Point) -> Float {
    0.0
}

enum Shape {
    Circle,
    Rectangle
}
"#;
        let symbols = compute_document_symbols(source);
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "Point");
        assert_eq!(symbols[1].name, "distance");
        assert_eq!(symbols[2].name, "Shape");
    }
}
