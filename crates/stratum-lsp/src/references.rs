//! Find references implementation for Stratum LSP
//!
//! This module provides "find all references" functionality, allowing users to
//! find all usages of a symbol throughout the code.

use stratum_core::ast::{
    Block, CallArg, EnumDef, Expr, ExprKind, Function, ImplDef, InterfaceDef, Item, ItemKind,
    Module, Pattern, PatternKind, Stmt, StmtKind, StructDef, TopLevelItem, TopLevelLet,
};
use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::cache::CachedData;
use crate::definition::{DefinitionInfo, SymbolIndex};

/// Compute all references using cached data
pub fn compute_references_cached(
    uri: &Url,
    data: &CachedData<'_>,
    position: Position,
    include_declaration: bool,
) -> Vec<Location> {
    // Convert LSP position to byte offset
    let Some(offset) = position_to_offset(data.line_index, position) else {
        return vec![];
    };

    // Get cached AST and symbol index
    let Some(module) = data.ast() else {
        return vec![];
    };
    let Some(index) = data.symbol_index else {
        return vec![];
    };

    // Find the identifier at the position
    let Some(ident_info) = find_ident_at_position(module, offset) else {
        return vec![];
    };

    // Look up the definition to get scope information
    let Some(def_info) = index.lookup(&ident_info.name, offset) else {
        return collect_all_references(
            module,
            &ident_info.name,
            None,
            uri,
            data.line_index,
            include_declaration,
        );
    };

    // Collect all references
    collect_all_references(
        module,
        &def_info.name,
        Some(def_info),
        uri,
        data.line_index,
        include_declaration,
    )
}

/// Compute all references for a symbol at the given position (non-cached)
#[allow(dead_code)] // Standalone API used by tests
pub fn compute_references(
    uri: &Url,
    source: &str,
    position: Position,
    include_declaration: bool,
) -> Vec<Location> {
    let line_index = LineIndex::new(source);

    // Convert LSP position to byte offset
    let Some(offset) = position_to_offset(&line_index, position) else {
        return vec![];
    };

    // Parse the module
    let Ok(module) = Parser::parse_module(source) else {
        return vec![];
    };

    // Build symbol index
    let index = SymbolIndex::from_module(&module);

    // Find the identifier at the position
    let Some(ident_info) = find_ident_at_position(&module, offset) else {
        return vec![];
    };

    // Look up the definition to get scope information
    let Some(def_info) = index.lookup(&ident_info.name, offset) else {
        // No definition found - symbol might be undefined
        // Still collect references in case it's a built-in or external
        return collect_all_references(
            &module,
            &ident_info.name,
            None,
            uri,
            &line_index,
            include_declaration,
        );
    };

    // Collect all references
    collect_all_references(
        &module,
        &def_info.name,
        Some(def_info),
        uri,
        &line_index,
        include_declaration,
    )
}

/// Collect all references to a symbol in the module
fn collect_all_references(
    module: &Module,
    name: &str,
    def_info: Option<&DefinitionInfo>,
    uri: &Url,
    line_index: &LineIndex,
    include_declaration: bool,
) -> Vec<Location> {
    let mut references = Vec::new();

    // Determine scope for collection
    let scope_span = def_info.and_then(|d| d.scope_span);

    // Collect references from the module
    for item in &module.top_level {
        collect_refs_in_top_level_item(item, name, scope_span, &mut references);
    }

    // Get the definition span if available
    let def_span = def_info.map(|d| d.name_span);

    // Convert spans to locations, filtering out the definition if not requested
    let mut locations: Vec<Location> = references
        .into_iter()
        .filter(|span| {
            // If we have a definition and include_declaration is false, filter it out
            if !include_declaration {
                if let Some(def) = def_span {
                    if span.start == def.start && span.end == def.end {
                        return false;
                    }
                }
            }
            true
        })
        .map(|span| Location {
            uri: uri.clone(),
            range: span_to_range(span, line_index),
        })
        .collect();

    // If include_declaration, ensure the declaration is included and at the front
    if include_declaration {
        if let Some(def) = def_info {
            let def_location = Location {
                uri: uri.clone(),
                range: span_to_range(def.name_span, line_index),
            };
            // Add if not already present
            if !locations.iter().any(|loc| loc.range == def_location.range) {
                locations.insert(0, def_location);
            }
        }
    }

    locations
}

/// Collect references in a top-level item
fn collect_refs_in_top_level_item(
    item: &TopLevelItem,
    name: &str,
    scope: Option<Span>,
    refs: &mut Vec<Span>,
) {
    match item {
        TopLevelItem::Item(item) => collect_refs_in_item(item, name, scope, refs),
        TopLevelItem::Let(let_decl) => collect_refs_in_top_level_let(let_decl, name, scope, refs),
        TopLevelItem::Statement(stmt) => collect_refs_in_stmt(stmt, name, scope, refs),
    }
}

/// Collect references in an item
fn collect_refs_in_item(item: &Item, name: &str, scope: Option<Span>, refs: &mut Vec<Span>) {
    match &item.kind {
        ItemKind::Function(func) => collect_refs_in_function(func, name, scope, refs),
        ItemKind::Struct(struct_def) => collect_refs_in_struct(struct_def, name, scope, refs),
        ItemKind::Enum(enum_def) => collect_refs_in_enum(enum_def, name, scope, refs),
        ItemKind::Interface(interface_def) => {
            collect_refs_in_interface(interface_def, name, scope, refs);
        }
        ItemKind::Impl(impl_def) => collect_refs_in_impl(impl_def, name, scope, refs),
        ItemKind::Import(_) => {}
    }
}

/// Collect references in a function
fn collect_refs_in_function(
    func: &Function,
    name: &str,
    scope: Option<Span>,
    refs: &mut Vec<Span>,
) {
    // Check if we're in scope (for scoped symbols)
    if let Some(s) = scope {
        if !spans_overlap(func.span, s) {
            return;
        }
    }

    // Check function name (for function references)
    if func.name.name == name {
        refs.push(func.name.span);
    }

    // Check parameters
    for param in &func.params {
        if param.name.name == name {
            refs.push(param.name.span);
        }
    }

    // Check body
    collect_refs_in_block(&func.body, name, scope, refs);
}

/// Collect references in a struct definition
fn collect_refs_in_struct(
    struct_def: &StructDef,
    name: &str,
    _scope: Option<Span>,
    refs: &mut Vec<Span>,
) {
    if struct_def.name.name == name {
        refs.push(struct_def.name.span);
    }
}

/// Collect references in an enum definition
fn collect_refs_in_enum(
    enum_def: &EnumDef,
    name: &str,
    _scope: Option<Span>,
    refs: &mut Vec<Span>,
) {
    if enum_def.name.name == name {
        refs.push(enum_def.name.span);
    }

    for variant in &enum_def.variants {
        if variant.name.name == name {
            refs.push(variant.name.span);
        }
    }
}

/// Collect references in an interface definition
fn collect_refs_in_interface(
    interface_def: &InterfaceDef,
    name: &str,
    _scope: Option<Span>,
    refs: &mut Vec<Span>,
) {
    if interface_def.name.name == name {
        refs.push(interface_def.name.span);
    }
}

/// Collect references in an impl block
fn collect_refs_in_impl(impl_def: &ImplDef, name: &str, scope: Option<Span>, refs: &mut Vec<Span>) {
    for method in &impl_def.methods {
        collect_refs_in_function(method, name, scope, refs);
    }
}

/// Collect references in a top-level let declaration
fn collect_refs_in_top_level_let(
    let_decl: &TopLevelLet,
    name: &str,
    scope: Option<Span>,
    refs: &mut Vec<Span>,
) {
    collect_refs_in_pattern(&let_decl.pattern, name, refs);
    collect_refs_in_expr(&let_decl.value, name, scope, refs);
}

/// Collect references in a block
fn collect_refs_in_block(block: &Block, name: &str, scope: Option<Span>, refs: &mut Vec<Span>) {
    // Check scope constraint
    if let Some(s) = scope {
        if !spans_overlap(block.span, s) {
            return;
        }
    }

    for stmt in &block.stmts {
        collect_refs_in_stmt(stmt, name, scope, refs);
    }

    if let Some(expr) = &block.expr {
        collect_refs_in_expr(expr, name, scope, refs);
    }
}

/// Collect references in a statement
fn collect_refs_in_stmt(stmt: &Stmt, name: &str, scope: Option<Span>, refs: &mut Vec<Span>) {
    // Check scope constraint
    if let Some(s) = scope {
        if !spans_overlap(stmt.span, s) {
            return;
        }
    }

    match &stmt.kind {
        StmtKind::Let { pattern, value, .. } => {
            collect_refs_in_pattern(pattern, name, refs);
            collect_refs_in_expr(value, name, scope, refs);
        }
        StmtKind::Expr(expr) => {
            collect_refs_in_expr(expr, name, scope, refs);
        }
        StmtKind::Assign { target, value } => {
            collect_refs_in_expr(target, name, scope, refs);
            collect_refs_in_expr(value, name, scope, refs);
        }
        StmtKind::CompoundAssign { target, value, .. } => {
            collect_refs_in_expr(target, name, scope, refs);
            collect_refs_in_expr(value, name, scope, refs);
        }
        StmtKind::Return(Some(expr)) => {
            collect_refs_in_expr(expr, name, scope, refs);
        }
        StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
        StmtKind::For {
            pattern,
            iter,
            body,
        } => {
            collect_refs_in_pattern(pattern, name, refs);
            collect_refs_in_expr(iter, name, scope, refs);
            collect_refs_in_block(body, name, scope, refs);
        }
        StmtKind::While { cond, body } => {
            collect_refs_in_expr(cond, name, scope, refs);
            collect_refs_in_block(body, name, scope, refs);
        }
        StmtKind::Loop { body } => {
            collect_refs_in_block(body, name, scope, refs);
        }
        StmtKind::TryCatch {
            try_block,
            catches,
            finally,
        } => {
            collect_refs_in_block(try_block, name, scope, refs);
            for catch in catches {
                if let Some(binding) = &catch.binding {
                    if binding.name == name {
                        refs.push(binding.span);
                    }
                }
                collect_refs_in_block(&catch.body, name, scope, refs);
            }
            if let Some(finally_block) = finally {
                collect_refs_in_block(finally_block, name, scope, refs);
            }
        }
        StmtKind::Throw(expr) => {
            collect_refs_in_expr(expr, name, scope, refs);
        }
    }
}

/// Collect references in a pattern
fn collect_refs_in_pattern(pattern: &Pattern, name: &str, refs: &mut Vec<Span>) {
    match &pattern.kind {
        PatternKind::Ident(ident) => {
            if ident.name == name {
                refs.push(ident.span);
            }
        }
        PatternKind::Struct {
            name: struct_name,
            fields,
            ..
        } => {
            if struct_name.name == name {
                refs.push(struct_name.span);
            }
            for field in fields {
                if field.name.name == name {
                    refs.push(field.name.span);
                }
                if let Some(pat) = &field.pattern {
                    collect_refs_in_pattern(pat, name, refs);
                }
            }
        }
        PatternKind::Variant {
            enum_name,
            variant,
            data,
        } => {
            if let Some(enum_n) = enum_name {
                if enum_n.name == name {
                    refs.push(enum_n.span);
                }
            }
            if variant.name == name {
                refs.push(variant.span);
            }
            if let Some(d) = data {
                collect_refs_in_pattern(d, name, refs);
            }
        }
        PatternKind::List { elements, rest } => {
            for elem in elements {
                collect_refs_in_pattern(elem, name, refs);
            }
            if let Some(rest_pat) = rest {
                collect_refs_in_pattern(rest_pat, name, refs);
            }
        }
        PatternKind::Or(patterns) => {
            for pat in patterns {
                collect_refs_in_pattern(pat, name, refs);
            }
        }
        PatternKind::Wildcard | PatternKind::Literal(_) => {}
    }
}

/// Collect references in an expression
fn collect_refs_in_expr(expr: &Expr, name: &str, scope: Option<Span>, refs: &mut Vec<Span>) {
    // Check scope constraint
    if let Some(s) = scope {
        if !spans_overlap(expr.span, s) {
            return;
        }
    }

    match &expr.kind {
        ExprKind::Ident(ident) => {
            if ident.name == name {
                refs.push(ident.span);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_refs_in_expr(left, name, scope, refs);
            collect_refs_in_expr(right, name, scope, refs);
        }
        ExprKind::Unary { expr: e, .. } => {
            collect_refs_in_expr(e, name, scope, refs);
        }
        ExprKind::Paren(inner) => {
            collect_refs_in_expr(inner, name, scope, refs);
        }
        ExprKind::Call {
            callee,
            args,
            trailing_closure,
        } => {
            collect_refs_in_expr(callee, name, scope, refs);
            for arg in args {
                match arg {
                    CallArg::Positional(e) => {
                        collect_refs_in_expr(e, name, scope, refs);
                    }
                    CallArg::Named {
                        name: arg_name,
                        value,
                        ..
                    } => {
                        if arg_name.name == name {
                            refs.push(arg_name.span);
                        }
                        collect_refs_in_expr(value, name, scope, refs);
                    }
                }
            }
            if let Some(closure) = trailing_closure {
                collect_refs_in_expr(closure, name, scope, refs);
            }
        }
        ExprKind::Index { expr: e, index } => {
            collect_refs_in_expr(e, name, scope, refs);
            collect_refs_in_expr(index, name, scope, refs);
        }
        ExprKind::Field { expr: e, field } => {
            collect_refs_in_expr(e, name, scope, refs);
            // Field names are not variable references in the same sense
            // but we could add them for completeness in field lookups
            if field.name == name {
                refs.push(field.span);
            }
        }
        ExprKind::NullSafeField { expr: e, field } => {
            collect_refs_in_expr(e, name, scope, refs);
            if field.name == name {
                refs.push(field.span);
            }
        }
        ExprKind::NullSafeIndex { expr: e, index } => {
            collect_refs_in_expr(e, name, scope, refs);
            collect_refs_in_expr(index, name, scope, refs);
        }
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_refs_in_expr(cond, name, scope, refs);
            collect_refs_in_block(then_branch, name, scope, refs);
            if let Some(else_br) = else_branch {
                match else_br {
                    stratum_core::ast::ElseBranch::Block(block) => {
                        collect_refs_in_block(block, name, scope, refs);
                    }
                    stratum_core::ast::ElseBranch::ElseIf(e) => {
                        collect_refs_in_expr(e, name, scope, refs);
                    }
                }
            }
        }
        ExprKind::Match { expr: e, arms } => {
            collect_refs_in_expr(e, name, scope, refs);
            for arm in arms {
                collect_refs_in_pattern(&arm.pattern, name, refs);
                if let Some(guard) = &arm.guard {
                    collect_refs_in_expr(guard, name, scope, refs);
                }
                collect_refs_in_expr(&arm.body, name, scope, refs);
            }
        }
        ExprKind::Lambda { params, body, .. } => {
            for param in params {
                if param.name.name == name {
                    refs.push(param.name.span);
                }
            }
            collect_refs_in_expr(body, name, scope, refs);
        }
        ExprKind::Block(block) => {
            collect_refs_in_block(block, name, scope, refs);
        }
        ExprKind::List(elements) => {
            for elem in elements {
                collect_refs_in_expr(elem, name, scope, refs);
            }
        }
        ExprKind::Map(pairs) => {
            for (k, v) in pairs {
                collect_refs_in_expr(k, name, scope, refs);
                collect_refs_in_expr(v, name, scope, refs);
            }
        }
        ExprKind::StringInterp { parts } => {
            for part in parts {
                if let stratum_core::ast::StringPart::Expr(e) = part {
                    collect_refs_in_expr(e, name, scope, refs);
                }
            }
        }
        ExprKind::StructInit {
            name: struct_name,
            fields,
        } => {
            if struct_name.name == name {
                refs.push(struct_name.span);
            }
            for field in fields {
                // Field name in struct init
                if field.name.name == name {
                    refs.push(field.name.span);
                }
                if let Some(value) = &field.value {
                    collect_refs_in_expr(value, name, scope, refs);
                }
            }
        }
        ExprKind::EnumVariant {
            enum_name,
            variant,
            data,
        } => {
            if let Some(enum_n) = enum_name {
                if enum_n.name == name {
                    refs.push(enum_n.span);
                }
            }
            if variant.name == name {
                refs.push(variant.span);
            }
            if let Some(d) = data {
                collect_refs_in_expr(d, name, scope, refs);
            }
        }
        ExprKind::Await(inner) | ExprKind::Try(inner) | ExprKind::StateBinding(inner) => {
            collect_refs_in_expr(inner, name, scope, refs);
        }
        ExprKind::Literal(_) | ExprKind::Placeholder | ExprKind::ColumnShorthand(_) => {}
    }
}

/// Information about an identifier at a position (matches definition.rs)
struct IdentAtPosition {
    name: String,
    #[allow(dead_code)]
    span: Span,
}

/// Find the identifier at a given byte offset
fn find_ident_at_position(module: &Module, offset: u32) -> Option<IdentAtPosition> {
    for item in &module.top_level {
        if let Some(info) = find_ident_in_top_level_item(item, offset) {
            return Some(info);
        }
    }
    None
}

fn find_ident_in_top_level_item(item: &TopLevelItem, offset: u32) -> Option<IdentAtPosition> {
    match item {
        TopLevelItem::Item(item) => find_ident_in_item(item, offset),
        TopLevelItem::Let(let_decl) => find_ident_in_top_level_let(let_decl, offset),
        TopLevelItem::Statement(stmt) => find_ident_in_stmt(stmt, offset),
    }
}

fn find_ident_in_item(item: &Item, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(item.span, offset) {
        return None;
    }

    match &item.kind {
        ItemKind::Function(func) => find_ident_in_function(func, offset),
        ItemKind::Struct(struct_def) => find_ident_in_struct(struct_def, offset),
        ItemKind::Enum(enum_def) => find_ident_in_enum(enum_def, offset),
        ItemKind::Interface(interface_def) => find_ident_in_interface(interface_def, offset),
        ItemKind::Impl(impl_def) => find_ident_in_impl(impl_def, offset),
        ItemKind::Import(_) => None,
    }
}

fn find_ident_in_function(func: &Function, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(func.span, offset) {
        return None;
    }

    // Check function name
    if span_contains(func.name.span, offset) {
        return Some(IdentAtPosition {
            name: func.name.name.clone(),
            span: func.name.span,
        });
    }

    // Check parameters
    for param in &func.params {
        if span_contains(param.name.span, offset) {
            return Some(IdentAtPosition {
                name: param.name.name.clone(),
                span: param.name.span,
            });
        }
    }

    // Check body
    find_ident_in_block(&func.body, offset)
}

fn find_ident_in_struct(struct_def: &StructDef, offset: u32) -> Option<IdentAtPosition> {
    if span_contains(struct_def.name.span, offset) {
        return Some(IdentAtPosition {
            name: struct_def.name.name.clone(),
            span: struct_def.name.span,
        });
    }
    None
}

fn find_ident_in_enum(enum_def: &EnumDef, offset: u32) -> Option<IdentAtPosition> {
    if span_contains(enum_def.name.span, offset) {
        return Some(IdentAtPosition {
            name: enum_def.name.name.clone(),
            span: enum_def.name.span,
        });
    }

    for variant in &enum_def.variants {
        if span_contains(variant.name.span, offset) {
            return Some(IdentAtPosition {
                name: variant.name.name.clone(),
                span: variant.name.span,
            });
        }
    }

    None
}

fn find_ident_in_interface(interface_def: &InterfaceDef, offset: u32) -> Option<IdentAtPosition> {
    if span_contains(interface_def.name.span, offset) {
        return Some(IdentAtPosition {
            name: interface_def.name.name.clone(),
            span: interface_def.name.span,
        });
    }
    None
}

fn find_ident_in_impl(impl_def: &ImplDef, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(impl_def.span, offset) {
        return None;
    }

    for method in &impl_def.methods {
        if let Some(info) = find_ident_in_function(method, offset) {
            return Some(info);
        }
    }

    None
}

fn find_ident_in_top_level_let(let_decl: &TopLevelLet, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(let_decl.span, offset) {
        return None;
    }

    if let Some(info) = find_ident_in_pattern(&let_decl.pattern, offset) {
        return Some(info);
    }

    find_ident_in_expr(&let_decl.value, offset)
}

fn find_ident_in_pattern(pattern: &Pattern, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(pattern.span, offset) {
        return None;
    }

    match &pattern.kind {
        PatternKind::Ident(ident) => {
            if span_contains(ident.span, offset) {
                return Some(IdentAtPosition {
                    name: ident.name.clone(),
                    span: ident.span,
                });
            }
        }
        PatternKind::Struct { name, fields, .. } => {
            if span_contains(name.span, offset) {
                return Some(IdentAtPosition {
                    name: name.name.clone(),
                    span: name.span,
                });
            }
            for field in fields {
                if span_contains(field.name.span, offset) {
                    return Some(IdentAtPosition {
                        name: field.name.name.clone(),
                        span: field.name.span,
                    });
                }
                if let Some(pat) = &field.pattern {
                    if let Some(info) = find_ident_in_pattern(pat, offset) {
                        return Some(info);
                    }
                }
            }
        }
        PatternKind::Variant {
            enum_name,
            variant,
            data,
        } => {
            if let Some(enum_n) = enum_name {
                if span_contains(enum_n.span, offset) {
                    return Some(IdentAtPosition {
                        name: enum_n.name.clone(),
                        span: enum_n.span,
                    });
                }
            }
            if span_contains(variant.span, offset) {
                return Some(IdentAtPosition {
                    name: variant.name.clone(),
                    span: variant.span,
                });
            }
            if let Some(d) = data {
                if let Some(info) = find_ident_in_pattern(d, offset) {
                    return Some(info);
                }
            }
        }
        PatternKind::List { elements, rest } => {
            for elem in elements {
                if let Some(info) = find_ident_in_pattern(elem, offset) {
                    return Some(info);
                }
            }
            if let Some(rest_pat) = rest {
                if let Some(info) = find_ident_in_pattern(rest_pat, offset) {
                    return Some(info);
                }
            }
        }
        PatternKind::Or(patterns) => {
            for pat in patterns {
                if let Some(info) = find_ident_in_pattern(pat, offset) {
                    return Some(info);
                }
            }
        }
        PatternKind::Wildcard | PatternKind::Literal(_) => {}
    }

    None
}

fn find_ident_in_block(block: &Block, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(block.span, offset) {
        return None;
    }

    for stmt in &block.stmts {
        if let Some(info) = find_ident_in_stmt(stmt, offset) {
            return Some(info);
        }
    }

    if let Some(expr) = &block.expr {
        return find_ident_in_expr(expr, offset);
    }

    None
}

fn find_ident_in_stmt(stmt: &Stmt, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(stmt.span, offset) {
        return None;
    }

    match &stmt.kind {
        StmtKind::Let { pattern, value, .. } => {
            if let Some(info) = find_ident_in_pattern(pattern, offset) {
                return Some(info);
            }
            find_ident_in_expr(value, offset)
        }
        StmtKind::Expr(expr) => find_ident_in_expr(expr, offset),
        StmtKind::Assign { target, value } => {
            if let Some(info) = find_ident_in_expr(target, offset) {
                return Some(info);
            }
            find_ident_in_expr(value, offset)
        }
        StmtKind::CompoundAssign { target, value, .. } => {
            if let Some(info) = find_ident_in_expr(target, offset) {
                return Some(info);
            }
            find_ident_in_expr(value, offset)
        }
        StmtKind::Return(Some(expr)) => find_ident_in_expr(expr, offset),
        StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => None,
        StmtKind::For {
            pattern,
            iter,
            body,
        } => {
            if let Some(info) = find_ident_in_pattern(pattern, offset) {
                return Some(info);
            }
            if let Some(info) = find_ident_in_expr(iter, offset) {
                return Some(info);
            }
            find_ident_in_block(body, offset)
        }
        StmtKind::While { cond, body } => {
            if let Some(info) = find_ident_in_expr(cond, offset) {
                return Some(info);
            }
            find_ident_in_block(body, offset)
        }
        StmtKind::Loop { body } => find_ident_in_block(body, offset),
        StmtKind::TryCatch {
            try_block,
            catches,
            finally,
        } => {
            if let Some(info) = find_ident_in_block(try_block, offset) {
                return Some(info);
            }
            for catch in catches {
                if let Some(binding) = &catch.binding {
                    if span_contains(binding.span, offset) {
                        return Some(IdentAtPosition {
                            name: binding.name.clone(),
                            span: binding.span,
                        });
                    }
                }
                if let Some(info) = find_ident_in_block(&catch.body, offset) {
                    return Some(info);
                }
            }
            if let Some(finally_block) = finally {
                return find_ident_in_block(finally_block, offset);
            }
            None
        }
        StmtKind::Throw(expr) => find_ident_in_expr(expr, offset),
    }
}

fn find_ident_in_expr(expr: &Expr, offset: u32) -> Option<IdentAtPosition> {
    if !span_contains(expr.span, offset) {
        return None;
    }

    match &expr.kind {
        ExprKind::Ident(ident) => {
            if span_contains(ident.span, offset) {
                return Some(IdentAtPosition {
                    name: ident.name.clone(),
                    span: ident.span,
                });
            }
        }
        ExprKind::Binary { left, right, .. } => {
            if let Some(info) = find_ident_in_expr(left, offset) {
                return Some(info);
            }
            return find_ident_in_expr(right, offset);
        }
        ExprKind::Unary { expr: e, .. } => {
            return find_ident_in_expr(e, offset);
        }
        ExprKind::Paren(inner) => {
            return find_ident_in_expr(inner, offset);
        }
        ExprKind::Call {
            callee,
            args,
            trailing_closure,
        } => {
            if let Some(info) = find_ident_in_expr(callee, offset) {
                return Some(info);
            }
            for arg in args {
                match arg {
                    CallArg::Positional(e) => {
                        if let Some(info) = find_ident_in_expr(e, offset) {
                            return Some(info);
                        }
                    }
                    CallArg::Named { name, value, .. } => {
                        if span_contains(name.span, offset) {
                            return Some(IdentAtPosition {
                                name: name.name.clone(),
                                span: name.span,
                            });
                        }
                        if let Some(info) = find_ident_in_expr(value, offset) {
                            return Some(info);
                        }
                    }
                }
            }
            if let Some(closure) = trailing_closure {
                return find_ident_in_expr(closure, offset);
            }
        }
        ExprKind::Index { expr: e, index } => {
            if let Some(info) = find_ident_in_expr(e, offset) {
                return Some(info);
            }
            return find_ident_in_expr(index, offset);
        }
        ExprKind::Field { expr: e, field } => {
            if span_contains(field.span, offset) {
                return Some(IdentAtPosition {
                    name: field.name.clone(),
                    span: field.span,
                });
            }
            return find_ident_in_expr(e, offset);
        }
        ExprKind::NullSafeField { expr: e, field } => {
            if span_contains(field.span, offset) {
                return Some(IdentAtPosition {
                    name: field.name.clone(),
                    span: field.span,
                });
            }
            return find_ident_in_expr(e, offset);
        }
        ExprKind::NullSafeIndex { expr: e, index } => {
            if let Some(info) = find_ident_in_expr(e, offset) {
                return Some(info);
            }
            return find_ident_in_expr(index, offset);
        }
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            if let Some(info) = find_ident_in_expr(cond, offset) {
                return Some(info);
            }
            if let Some(info) = find_ident_in_block(then_branch, offset) {
                return Some(info);
            }
            if let Some(else_br) = else_branch {
                match else_br {
                    stratum_core::ast::ElseBranch::Block(block) => {
                        return find_ident_in_block(block, offset);
                    }
                    stratum_core::ast::ElseBranch::ElseIf(e) => {
                        return find_ident_in_expr(e, offset);
                    }
                }
            }
        }
        ExprKind::Match { expr: e, arms } => {
            if let Some(info) = find_ident_in_expr(e, offset) {
                return Some(info);
            }
            for arm in arms {
                if let Some(info) = find_ident_in_pattern(&arm.pattern, offset) {
                    return Some(info);
                }
                if let Some(guard) = &arm.guard {
                    if let Some(info) = find_ident_in_expr(guard, offset) {
                        return Some(info);
                    }
                }
                if let Some(info) = find_ident_in_expr(&arm.body, offset) {
                    return Some(info);
                }
            }
        }
        ExprKind::Lambda { params, body, .. } => {
            for param in params {
                if span_contains(param.name.span, offset) {
                    return Some(IdentAtPosition {
                        name: param.name.name.clone(),
                        span: param.name.span,
                    });
                }
            }
            return find_ident_in_expr(body, offset);
        }
        ExprKind::Block(block) => {
            return find_ident_in_block(block, offset);
        }
        ExprKind::List(elements) => {
            for elem in elements {
                if let Some(info) = find_ident_in_expr(elem, offset) {
                    return Some(info);
                }
            }
        }
        ExprKind::Map(pairs) => {
            for (k, v) in pairs {
                if let Some(info) = find_ident_in_expr(k, offset) {
                    return Some(info);
                }
                if let Some(info) = find_ident_in_expr(v, offset) {
                    return Some(info);
                }
            }
        }
        ExprKind::StringInterp { parts } => {
            for part in parts {
                if let stratum_core::ast::StringPart::Expr(e) = part {
                    if let Some(info) = find_ident_in_expr(e, offset) {
                        return Some(info);
                    }
                }
            }
        }
        ExprKind::StructInit { name, fields } => {
            if span_contains(name.span, offset) {
                return Some(IdentAtPosition {
                    name: name.name.clone(),
                    span: name.span,
                });
            }
            for field in fields {
                if span_contains(field.name.span, offset) {
                    return Some(IdentAtPosition {
                        name: field.name.name.clone(),
                        span: field.name.span,
                    });
                }
                if let Some(value) = &field.value {
                    if let Some(info) = find_ident_in_expr(value, offset) {
                        return Some(info);
                    }
                }
            }
        }
        ExprKind::EnumVariant {
            enum_name,
            variant,
            data,
        } => {
            if let Some(enum_n) = enum_name {
                if span_contains(enum_n.span, offset) {
                    return Some(IdentAtPosition {
                        name: enum_n.name.clone(),
                        span: enum_n.span,
                    });
                }
            }
            if span_contains(variant.span, offset) {
                return Some(IdentAtPosition {
                    name: variant.name.clone(),
                    span: variant.span,
                });
            }
            if let Some(d) = data {
                return find_ident_in_expr(d, offset);
            }
        }
        ExprKind::Await(inner) | ExprKind::Try(inner) | ExprKind::StateBinding(inner) => {
            return find_ident_in_expr(inner, offset);
        }
        ExprKind::Literal(_) | ExprKind::Placeholder | ExprKind::ColumnShorthand(_) => {}
    }

    None
}

/// Convert an LSP Position to a byte offset
fn position_to_offset(line_index: &LineIndex, position: Position) -> Option<u32> {
    let line = position.line as usize;
    let character = position.character as usize;
    let line_start = line_index.line_start(line)?;
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

/// Check if a span contains the given offset
fn span_contains(span: Span, offset: u32) -> bool {
    offset >= span.start && offset < span.end
}

/// Check if two spans overlap (for scope checking)
fn spans_overlap(a: Span, b: Span) -> bool {
    a.start < b.end && b.start < a.end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_function_references() {
        let source = r#"
fx greet(name: String) -> String {
    "Hello, {name}!"
}

fx main() {
    let msg = greet("World")
    print(greet("Again"))
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "greet" function definition (line 1)
        let position = Position {
            line: 1,
            character: 3,
        };

        let refs = compute_references(&uri, source, position, true);
        // Should find: definition + 2 calls
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_find_local_variable_references() {
        let source = r#"
fx main() {
    let x = 42
    let y = x + 1
    print(x)
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "x" definition (line 2)
        let position = Position {
            line: 2,
            character: 8,
        };

        let refs = compute_references(&uri, source, position, true);
        // Should find: definition + 2 usages
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_find_parameter_references() {
        let source = r#"
fx add(a: Int, b: Int) -> Int {
    let sum = a + b
    a * sum
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on parameter "a" (line 1)
        let position = Position {
            line: 1,
            character: 7,
        };

        let refs = compute_references(&uri, source, position, true);
        // Should find: parameter definition + 2 usages
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_find_struct_references() {
        let source = r#"
struct Point {
    x: Int,
    y: Int
}

fx main() {
    let p = Point { x: 1, y: 2 }
    let q = Point { x: 3, y: 4 }
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "Point" struct definition (line 1)
        let position = Position {
            line: 1,
            character: 7,
        };

        let refs = compute_references(&uri, source, position, true);
        // Should find: definition + 2 struct init usages
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_find_for_loop_variable_references() {
        let source = r#"
fx main() {
    for i in [1, 2, 3] {
        let j = i + 1
        print(i)
    }
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "i" in print(i) - a usage inside the loop body (line 4)
        let position = Position {
            line: 4,
            character: 14,
        };

        let refs = compute_references(&uri, source, position, true);
        // Should find: loop variable declaration + 2 usages
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_exclude_declaration() {
        let source = r#"
fx greet(name: String) -> String {
    "Hello, {name}!"
}

fx main() {
    greet("World")
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "greet" function definition
        let position = Position {
            line: 1,
            character: 3,
        };

        let refs_with_decl = compute_references(&uri, source, position, true);
        let refs_without_decl = compute_references(&uri, source, position, false);

        // With declaration should have one more
        assert_eq!(refs_with_decl.len(), refs_without_decl.len() + 1);
    }

    #[test]
    fn test_no_references_for_unknown_symbol() {
        let source = r#"
fx main() {
    print(unknown_var)
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "unknown_var"
        let position = Position {
            line: 2,
            character: 10,
        };

        let refs = compute_references(&uri, source, position, true);
        // Should still find the usage itself (as best effort)
        assert!(!refs.is_empty());
    }
}
