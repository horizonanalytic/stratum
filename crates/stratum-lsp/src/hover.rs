//! Hover information for Stratum source files
//!
//! This module provides hover functionality for the LSP server,
//! showing type information when hovering over expressions and identifiers.

use stratum_core::ast::{
    Block, CallArg, Expr, ExprKind, Function, Item, ItemKind, Literal, Module, Param, Pattern,
    PatternKind, Stmt, StmtKind, StructDef, TopLevelItem, TopLevelLet,
};
use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::Parser;
use stratum_core::types::TypeChecker;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

use crate::cache::CachedData;

/// Information about a hover target
#[derive(Debug)]
pub struct HoverInfo {
    /// The hover contents to display
    pub contents: String,
    /// The range of the hovered item
    pub range: Range,
}

/// Compute hover information using cached data
pub fn compute_hover_cached(data: &CachedData<'_>, position: Position) -> Option<HoverInfo> {
    // Convert LSP position to byte offset
    let offset = position_to_offset(data.line_index, position)?;

    // Get the cached AST
    let module = data.ast()?;

    // Create a type checker for hover info (ideally we'd cache this too)
    let mut type_checker = TypeChecker::new();
    let _ = type_checker.check_module(module);

    // Find the node at the position
    let node_info = find_node_at_position(module, offset, &type_checker)?;

    // Convert span to range
    let range = span_to_range(node_info.span, data.line_index);

    Some(HoverInfo {
        contents: node_info.hover_text,
        range,
    })
}

/// Compute hover information for a position in the source (non-cached version)
#[allow(dead_code)] // Standalone API used by tests
pub fn compute_hover(source: &str, position: Position) -> Option<HoverInfo> {
    let line_index = LineIndex::new(source);

    // Convert LSP position to byte offset
    let offset = position_to_offset(&line_index, position)?;

    // Parse the module
    let module = Parser::parse_module(source).ok()?;

    // Run type checker to get type information
    let mut type_checker = TypeChecker::new();
    let _ = type_checker.check_module(&module);

    // Find the node at the position
    let node_info = find_node_at_position(&module, offset, &type_checker)?;

    // Convert span to range
    let range = span_to_range(node_info.span, &line_index);

    Some(HoverInfo {
        contents: node_info.hover_text,
        range,
    })
}

/// Convert hover info to LSP Hover
pub fn hover_info_to_lsp(info: HoverInfo) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: info.contents,
        }),
        range: Some(info.range),
    }
}

/// Information about a node found at a position
struct NodeInfo {
    hover_text: String,
    span: Span,
}

/// Convert an LSP Position to a byte offset
fn position_to_offset(line_index: &LineIndex, position: Position) -> Option<u32> {
    // LSP positions are 0-indexed for both line and character
    let line = position.line as usize;
    let character = position.character as usize;

    // Get the start byte offset of the line
    let line_start = line_index.line_start(line)?;

    // Add the character offset (accounting for potential UTF-8)
    Some(line_start + character as u32)
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

/// Find the AST node at a given byte offset and return hover information
fn find_node_at_position(module: &Module, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    // Check top-level items
    for tl_item in &module.top_level {
        if let Some(info) = find_in_top_level_item(tl_item, offset, checker) {
            return Some(info);
        }
    }
    None
}

fn find_in_top_level_item(
    item: &TopLevelItem,
    offset: u32,
    checker: &TypeChecker,
) -> Option<NodeInfo> {
    match item {
        TopLevelItem::Item(item) => find_in_item(item, offset, checker),
        TopLevelItem::Let(let_decl) => find_in_top_level_let(let_decl, offset, checker),
        TopLevelItem::Statement(stmt) => find_in_stmt(stmt, offset, checker),
    }
}

fn find_in_item(item: &Item, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    if !span_contains(item.span, offset) {
        return None;
    }

    match &item.kind {
        ItemKind::Function(func) => find_in_function(func, offset, checker),
        ItemKind::Struct(struct_def) => find_in_struct(struct_def, offset),
        ItemKind::Enum(enum_def) => {
            if span_contains(enum_def.name.span, offset) {
                Some(NodeInfo {
                    hover_text: format!("```stratum\nenum {}\n```", enum_def.name.name),
                    span: enum_def.name.span,
                })
            } else {
                None
            }
        }
        ItemKind::Interface(interface_def) => {
            if span_contains(interface_def.name.span, offset) {
                Some(NodeInfo {
                    hover_text: format!("```stratum\ninterface {}\n```", interface_def.name.name),
                    span: interface_def.name.span,
                })
            } else {
                None
            }
        }
        ItemKind::Impl(_) | ItemKind::Import(_) => None,
    }
}

fn find_in_function(func: &Function, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    if !span_contains(func.span, offset) {
        return None;
    }

    // Check if hovering over function name
    if span_contains(func.name.span, offset) {
        let params_str = func
            .params
            .iter()
            .map(|p| {
                if let Some(ty) = &p.ty {
                    format!("{}: {}", p.name.name, type_annotation_to_string(ty))
                } else {
                    p.name.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let return_str = func
            .return_type
            .as_ref()
            .map(|ty| format!(" -> {}", type_annotation_to_string(ty)))
            .unwrap_or_default();

        let async_str = if func.is_async { "async " } else { "" };

        return Some(NodeInfo {
            hover_text: format!(
                "```stratum\n{}fx {}({}){}",
                async_str, func.name.name, params_str, return_str
            ) + "\n```",
            span: func.name.span,
        });
    }

    // Check parameters
    for param in &func.params {
        if let Some(info) = find_in_param(param, offset) {
            return Some(info);
        }
    }

    // Check function body
    find_in_block(&func.body, offset, checker)
}

fn find_in_param(param: &Param, offset: u32) -> Option<NodeInfo> {
    if span_contains(param.name.span, offset) {
        let ty_str = param
            .ty
            .as_ref()
            .map(type_annotation_to_string)
            .unwrap_or_else(|| "inferred".to_string());

        return Some(NodeInfo {
            hover_text: format!("```stratum\n{}: {}\n```\n\n(parameter)", param.name.name, ty_str),
            span: param.name.span,
        });
    }
    None
}

fn find_in_struct(struct_def: &StructDef, offset: u32) -> Option<NodeInfo> {
    if span_contains(struct_def.name.span, offset) {
        let type_params = if struct_def.type_params.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                struct_def
                    .type_params
                    .iter()
                    .map(|p| p.name.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        return Some(NodeInfo {
            hover_text: format!(
                "```stratum\nstruct {}{}\n```",
                struct_def.name.name, type_params
            ),
            span: struct_def.name.span,
        });
    }

    // Check fields
    for field in &struct_def.fields {
        if span_contains(field.name.span, offset) {
            return Some(NodeInfo {
                hover_text: format!(
                    "```stratum\n{}: {}\n```\n\n(field of `{}`)",
                    field.name.name,
                    type_annotation_to_string(&field.ty),
                    struct_def.name.name
                ),
                span: field.name.span,
            });
        }
    }

    None
}

fn find_in_top_level_let(let_decl: &TopLevelLet, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    if !span_contains(let_decl.span, offset) {
        return None;
    }

    // Check the pattern (variable name)
    if let Some(info) = find_in_pattern(&let_decl.pattern, offset, let_decl.ty.as_ref()) {
        return Some(info);
    }

    // Check the value expression
    find_in_expr(&let_decl.value, offset, checker)
}

fn find_in_pattern(
    pattern: &Pattern,
    offset: u32,
    ty: Option<&stratum_core::ast::TypeAnnotation>,
) -> Option<NodeInfo> {
    if !span_contains(pattern.span, offset) {
        return None;
    }

    match &pattern.kind {
        PatternKind::Ident(ident) => {
            if span_contains(ident.span, offset) {
                let ty_str = ty
                    .map(type_annotation_to_string)
                    .unwrap_or_else(|| "inferred".to_string());
                return Some(NodeInfo {
                    hover_text: format!("```stratum\nlet {}: {}\n```", ident.name, ty_str),
                    span: ident.span,
                });
            }
        }
        PatternKind::Wildcard
        | PatternKind::Literal(_)
        | PatternKind::Variant { .. }
        | PatternKind::Struct { .. }
        | PatternKind::List { .. }
        | PatternKind::Or(_) => {}
    }
    None
}

fn find_in_block(block: &Block, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    if !span_contains(block.span, offset) {
        return None;
    }

    // Check statements
    for stmt in &block.stmts {
        if let Some(info) = find_in_stmt(stmt, offset, checker) {
            return Some(info);
        }
    }

    // Check trailing expression
    if let Some(expr) = &block.expr {
        return find_in_expr(expr, offset, checker);
    }

    None
}

fn find_in_stmt(stmt: &Stmt, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    if !span_contains(stmt.span, offset) {
        return None;
    }

    match &stmt.kind {
        StmtKind::Let { pattern, ty, value } => {
            // Check pattern
            if let Some(info) = find_in_pattern(pattern, offset, ty.as_ref()) {
                return Some(info);
            }
            // Check value
            return find_in_expr(value, offset, checker);
        }
        StmtKind::Expr(expr) => return find_in_expr(expr, offset, checker),
        StmtKind::Assign { target, value } => {
            if let Some(info) = find_in_expr(target, offset, checker) {
                return Some(info);
            }
            return find_in_expr(value, offset, checker);
        }
        StmtKind::CompoundAssign { target, value, .. } => {
            if let Some(info) = find_in_expr(target, offset, checker) {
                return Some(info);
            }
            return find_in_expr(value, offset, checker);
        }
        StmtKind::Return(Some(expr)) => return find_in_expr(expr, offset, checker),
        StmtKind::For { iter, body, .. } => {
            if let Some(info) = find_in_expr(iter, offset, checker) {
                return Some(info);
            }
            return find_in_block(body, offset, checker);
        }
        StmtKind::While { cond, body } => {
            if let Some(info) = find_in_expr(cond, offset, checker) {
                return Some(info);
            }
            return find_in_block(body, offset, checker);
        }
        StmtKind::Loop { body } => return find_in_block(body, offset, checker),
        StmtKind::TryCatch {
            try_block,
            catches,
            finally,
        } => {
            if let Some(info) = find_in_block(try_block, offset, checker) {
                return Some(info);
            }
            for catch in catches {
                if let Some(info) = find_in_block(&catch.body, offset, checker) {
                    return Some(info);
                }
            }
            if let Some(finally_block) = finally {
                return find_in_block(finally_block, offset, checker);
            }
        }
        StmtKind::Throw(expr) => return find_in_expr(expr, offset, checker),
        StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
    }

    None
}

fn find_in_expr(expr: &Expr, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    if !span_contains(expr.span, offset) {
        return None;
    }

    // First, try to find a more specific node within this expression
    let inner = find_in_expr_inner(expr, offset, checker);
    if inner.is_some() {
        return inner;
    }

    // If no inner node found, return info for this expression
    let ty = infer_expr_type(expr, checker);
    Some(NodeInfo {
        hover_text: format!("```stratum\n{}\n```", ty),
        span: expr.span,
    })
}

fn find_in_expr_inner(expr: &Expr, offset: u32, checker: &TypeChecker) -> Option<NodeInfo> {
    match &expr.kind {
        ExprKind::Literal(lit) => {
            let ty = match lit {
                Literal::Int(_) => "Int",
                Literal::Float(_) => "Float",
                Literal::String(_) => "String",
                Literal::Bool(_) => "Bool",
                Literal::Null => "Null",
            };
            return Some(NodeInfo {
                hover_text: format!("```stratum\n{ty}\n```"),
                span: expr.span,
            });
        }

        ExprKind::Ident(ident) => {
            // For identifiers, infer the type
            let ty = infer_expr_type(expr, checker);
            return Some(NodeInfo {
                hover_text: format!("```stratum\n{}: {}\n```", ident.name, ty),
                span: ident.span,
            });
        }

        ExprKind::Binary { left, right, .. } => {
            if let Some(info) = find_in_expr(left, offset, checker) {
                return Some(info);
            }
            return find_in_expr(right, offset, checker);
        }

        ExprKind::Unary { expr: inner, .. } => {
            return find_in_expr(inner, offset, checker);
        }

        ExprKind::Paren(inner) => {
            return find_in_expr(inner, offset, checker);
        }

        ExprKind::Call {
            callee,
            args,
            trailing_closure,
        } => {
            if let Some(info) = find_in_expr(callee, offset, checker) {
                return Some(info);
            }
            for arg in args {
                match arg {
                    CallArg::Positional(e) | CallArg::Named { value: e, .. } => {
                        if let Some(info) = find_in_expr(e, offset, checker) {
                            return Some(info);
                        }
                    }
                }
            }
            if let Some(closure) = trailing_closure {
                return find_in_expr(closure, offset, checker);
            }
        }

        ExprKind::Index { expr: e, index } => {
            if let Some(info) = find_in_expr(e, offset, checker) {
                return Some(info);
            }
            return find_in_expr(index, offset, checker);
        }

        ExprKind::Field { expr: e, field } => {
            if span_contains(field.span, offset) {
                // Hovering over field name
                let ty = infer_expr_type(expr, checker);
                return Some(NodeInfo {
                    hover_text: format!("```stratum\n{}: {}\n```\n\n(field)", field.name, ty),
                    span: field.span,
                });
            }
            return find_in_expr(e, offset, checker);
        }

        ExprKind::NullSafeField { expr: e, field } => {
            if span_contains(field.span, offset) {
                let ty = infer_expr_type(expr, checker);
                return Some(NodeInfo {
                    hover_text: format!(
                        "```stratum\n{}: {}\n```\n\n(null-safe field access)",
                        field.name, ty
                    ),
                    span: field.span,
                });
            }
            return find_in_expr(e, offset, checker);
        }

        ExprKind::NullSafeIndex { expr: e, index } => {
            if let Some(info) = find_in_expr(e, offset, checker) {
                return Some(info);
            }
            return find_in_expr(index, offset, checker);
        }

        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            if let Some(info) = find_in_expr(cond, offset, checker) {
                return Some(info);
            }
            if let Some(info) = find_in_block(then_branch, offset, checker) {
                return Some(info);
            }
            if let Some(else_br) = else_branch {
                match else_br {
                    stratum_core::ast::ElseBranch::Block(block) => {
                        return find_in_block(block, offset, checker);
                    }
                    stratum_core::ast::ElseBranch::ElseIf(e) => {
                        return find_in_expr(e, offset, checker);
                    }
                }
            }
        }

        ExprKind::Match { expr: e, arms } => {
            if let Some(info) = find_in_expr(e, offset, checker) {
                return Some(info);
            }
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    if let Some(info) = find_in_expr(guard, offset, checker) {
                        return Some(info);
                    }
                }
                if let Some(info) = find_in_expr(&arm.body, offset, checker) {
                    return Some(info);
                }
            }
        }

        ExprKind::Lambda { body, .. } => {
            return find_in_expr(body, offset, checker);
        }

        ExprKind::Block(block) => {
            return find_in_block(block, offset, checker);
        }

        ExprKind::List(elements) => {
            for elem in elements {
                if let Some(info) = find_in_expr(elem, offset, checker) {
                    return Some(info);
                }
            }
        }

        ExprKind::Map(pairs) => {
            for (k, v) in pairs {
                if let Some(info) = find_in_expr(k, offset, checker) {
                    return Some(info);
                }
                if let Some(info) = find_in_expr(v, offset, checker) {
                    return Some(info);
                }
            }
        }

        ExprKind::StringInterp { parts } => {
            for part in parts {
                if let stratum_core::ast::StringPart::Expr(e) = part {
                    if let Some(info) = find_in_expr(e, offset, checker) {
                        return Some(info);
                    }
                }
            }
        }

        ExprKind::Await(inner) | ExprKind::Try(inner) => {
            return find_in_expr(inner, offset, checker);
        }

        ExprKind::StructInit { name, fields } => {
            if span_contains(name.span, offset) {
                return Some(NodeInfo {
                    hover_text: format!("```stratum\nstruct {}\n```", name.name),
                    span: name.span,
                });
            }
            for field in fields {
                if span_contains(field.name.span, offset) {
                    return Some(NodeInfo {
                        hover_text: format!(
                            "```stratum\n{}\n```\n\n(field of `{}`)",
                            field.name.name, name.name
                        ),
                        span: field.name.span,
                    });
                }
                if let Some(value) = &field.value {
                    if let Some(info) = find_in_expr(value, offset, checker) {
                        return Some(info);
                    }
                }
            }
        }

        ExprKind::EnumVariant { variant, data, .. } => {
            if span_contains(variant.span, offset) {
                return Some(NodeInfo {
                    hover_text: format!("```stratum\n{}\n```\n\n(enum variant)", variant.name),
                    span: variant.span,
                });
            }
            if let Some(d) = data {
                return find_in_expr(d, offset, checker);
            }
        }

        ExprKind::StateBinding(inner) => {
            return find_in_expr(inner, offset, checker);
        }

        ExprKind::Placeholder | ExprKind::ColumnShorthand(_) => {}
    }

    None
}

/// Check if a span contains the given offset
fn span_contains(span: Span, offset: u32) -> bool {
    offset >= span.start && offset < span.end
}

/// Infer the type of an expression
fn infer_expr_type(expr: &Expr, _checker: &TypeChecker) -> String {
    // For now, we use a simple approach based on expression kind
    // In the future, this could leverage cached type information from the checker
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            Literal::Int(_) => "Int".to_string(),
            Literal::Float(_) => "Float".to_string(),
            Literal::String(_) => "String".to_string(),
            Literal::Bool(_) => "Bool".to_string(),
            Literal::Null => "Null".to_string(),
        },
        ExprKind::List(_) => "List<...>".to_string(),
        ExprKind::Map(_) => "Map<...>".to_string(),
        ExprKind::Lambda { .. } => "(function)".to_string(),
        ExprKind::StringInterp { .. } => "String".to_string(),
        _ => "...".to_string(),
    }
}

/// Convert a type annotation to a display string
fn type_annotation_to_string(ty: &stratum_core::ast::TypeAnnotation) -> String {
    use stratum_core::ast::TypeKind;

    match &ty.kind {
        TypeKind::Named { name, args } => {
            if args.is_empty() {
                name.name.clone()
            } else {
                let args_str = args
                    .iter()
                    .map(type_annotation_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name.name, args_str)
            }
        }
        TypeKind::Nullable(inner) => format!("{}?", type_annotation_to_string(inner)),
        TypeKind::List(inner) => format!("[{}]", type_annotation_to_string(inner)),
        TypeKind::Tuple(types) => {
            let types_str = types
                .iter()
                .map(type_annotation_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({types_str})")
        }
        TypeKind::Function { params, ret } => {
            let params_str = params
                .iter()
                .map(type_annotation_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({params_str}) -> {}", type_annotation_to_string(ret))
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
    fn test_hover_on_function_name() {
        let source = "fx add(a: Int, b: Int) -> Int { a + b }";
        let position = Position {
            line: 0,
            character: 4, // On 'add'
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("fx add"));
        assert!(info.contents.contains("Int"));
    }

    #[test]
    fn test_hover_on_literal() {
        let source = "fx main() { 42 }";
        let position = Position {
            line: 0,
            character: 12, // On '42'
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("Int"));
    }

    #[test]
    fn test_hover_on_string_literal() {
        let source = r#"fx main() { "hello" }"#;
        let position = Position {
            line: 0,
            character: 13, // On string
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("String"));
    }

    #[test]
    fn test_hover_on_parameter() {
        let source = "fx greet(name: String) { name }";
        let position = Position {
            line: 0,
            character: 9, // On 'name' parameter
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("name"));
        assert!(info.contents.contains("String"));
        assert!(info.contents.contains("parameter"));
    }

    #[test]
    fn test_hover_on_struct_name() {
        let source = "struct Point { x: Int, y: Int }";
        let position = Position {
            line: 0,
            character: 8, // On 'Point'
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("struct Point"));
    }

    #[test]
    fn test_hover_outside_code() {
        let source = "fx main() { 42 }";
        let position = Position {
            line: 10, // Way past the code
            character: 0,
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_none());
    }

    #[test]
    fn test_hover_on_let_binding() {
        let source = "let x: Int = 42";
        let position = Position {
            line: 0,
            character: 4, // On 'x'
        };

        let hover = compute_hover(source, position);
        assert!(hover.is_some());
        let info = hover.unwrap();
        assert!(info.contents.contains("x"));
        assert!(info.contents.contains("Int"));
    }
}
