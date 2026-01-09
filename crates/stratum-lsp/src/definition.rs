//! Go to definition implementation for Stratum LSP
//!
//! This module provides "go to definition" functionality, allowing users to
//! jump from an identifier usage to its definition location.

use std::collections::HashMap;

use stratum_core::ast::{
    Block, CallArg, EnumDef, Expr, ExprKind, Function, Item, ItemKind, Module, Pattern,
    PatternKind, Stmt, StmtKind, StructDef, TopLevelItem, TopLevelLet, InterfaceDef, ImplDef,
};
use stratum_core::lexer::{LineIndex, Span};
use stratum_core::parser::Parser;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::cache::CachedData;

/// Information about a symbol definition
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DefinitionInfo {
    /// The name of the symbol
    pub name: String,
    /// The span of the symbol's name (not the whole definition)
    pub name_span: Span,
    /// The kind of symbol
    pub kind: SymbolKind,
    /// The scope in which this symbol is visible (None for top-level)
    pub scope_span: Option<Span>,
}

/// The kind of symbol being defined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Interface,
    Variable,
    Parameter,
    #[allow(dead_code)] // Used in completions for field completion
    Field,
    EnumVariant,
}

/// Index of all symbols in a module for definition lookup
#[derive(Debug, Default)]
pub struct SymbolIndex {
    /// Top-level symbols (functions, structs, enums, interfaces, top-level lets)
    top_level: HashMap<String, DefinitionInfo>,
    /// Scoped symbols indexed by scope span then by name
    /// This allows looking up local variables within their defining scope
    scoped: Vec<DefinitionInfo>,
}

impl SymbolIndex {
    /// Create a new empty symbol index
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a symbol index from a parsed module
    pub fn from_module(module: &Module) -> Self {
        let mut index = Self::new();
        index.collect_module(module);
        index
    }

    /// Add a top-level symbol to the index
    fn add_top_level(&mut self, info: DefinitionInfo) {
        self.top_level.insert(info.name.clone(), info);
    }

    /// Add a scoped symbol to the index
    fn add_scoped(&mut self, info: DefinitionInfo) {
        self.scoped.push(info);
    }

    /// Look up a symbol by name at a given position
    /// Returns the definition info if found
    pub fn lookup(&self, name: &str, position: u32) -> Option<&DefinitionInfo> {
        // First, check scoped symbols (more specific)
        // Find the most specific scope that contains the position and has this symbol
        let mut best_scoped: Option<&DefinitionInfo> = None;
        let mut best_scope_size = u32::MAX;

        for info in &self.scoped {
            if info.name != name {
                continue;
            }

            if let Some(scope) = info.scope_span {
                // Check if position is within scope and after the definition
                if position >= info.name_span.start && position < scope.end {
                    let scope_size = scope.end - scope.start;
                    if scope_size < best_scope_size {
                        best_scoped = Some(info);
                        best_scope_size = scope_size;
                    }
                }
            }
        }

        if best_scoped.is_some() {
            return best_scoped;
        }

        // Fall back to top-level symbols
        self.top_level.get(name)
    }

    /// Get all symbols that are visible at a given position
    /// Returns an iterator of (name, kind) pairs
    pub fn all_symbols_matching(&self, prefix: &str, position: u32) -> Vec<(String, SymbolKind)> {
        let prefix_lower = prefix.to_lowercase();
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Collect scoped symbols that are visible at this position
        for info in &self.scoped {
            if let Some(scope) = info.scope_span {
                // Check if position is within scope and after the definition
                if position >= info.name_span.start && position < scope.end {
                    if info.name.to_lowercase().starts_with(&prefix_lower) && !seen.contains(&info.name) {
                        seen.insert(info.name.clone());
                        results.push((info.name.clone(), info.kind));
                    }
                }
            }
        }

        // Collect top-level symbols
        for (name, info) in &self.top_level {
            if name.to_lowercase().starts_with(&prefix_lower) && !seen.contains(name) {
                seen.insert(name.clone());
                results.push((name.clone(), info.kind));
            }
        }

        results
    }

    /// Collect symbols from a module
    fn collect_module(&mut self, module: &Module) {
        for item in &module.top_level {
            self.collect_top_level_item(item);
        }
    }

    /// Collect symbols from a top-level item
    fn collect_top_level_item(&mut self, item: &TopLevelItem) {
        match item {
            TopLevelItem::Item(item) => self.collect_item(item),
            TopLevelItem::Let(let_decl) => self.collect_top_level_let(let_decl),
            TopLevelItem::Statement(_) => {}
        }
    }

    /// Collect symbols from an item
    fn collect_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function(func) => self.collect_function(func),
            ItemKind::Struct(struct_def) => self.collect_struct(struct_def),
            ItemKind::Enum(enum_def) => self.collect_enum(enum_def),
            ItemKind::Interface(interface_def) => self.collect_interface(interface_def),
            ItemKind::Impl(impl_def) => self.collect_impl(impl_def),
            ItemKind::Import(_) => {}
        }
    }

    /// Collect symbols from a function definition
    fn collect_function(&mut self, func: &Function) {
        // Add the function itself
        self.add_top_level(DefinitionInfo {
            name: func.name.name.clone(),
            name_span: func.name.span,
            kind: SymbolKind::Function,
            scope_span: None,
        });

        // Add parameters as scoped symbols
        for param in &func.params {
            self.add_scoped(DefinitionInfo {
                name: param.name.name.clone(),
                name_span: param.name.span,
                kind: SymbolKind::Parameter,
                scope_span: Some(func.body.span),
            });
        }

        // Collect local variables from the function body
        self.collect_block(&func.body, func.body.span);
    }

    /// Collect symbols from a struct definition
    fn collect_struct(&mut self, struct_def: &StructDef) {
        self.add_top_level(DefinitionInfo {
            name: struct_def.name.name.clone(),
            name_span: struct_def.name.span,
            kind: SymbolKind::Struct,
            scope_span: None,
        });

        // Add fields (these are accessible anywhere struct is used, not scoped)
        // We track them but they're looked up differently (via type info)
    }

    /// Collect symbols from an enum definition
    fn collect_enum(&mut self, enum_def: &EnumDef) {
        self.add_top_level(DefinitionInfo {
            name: enum_def.name.name.clone(),
            name_span: enum_def.name.span,
            kind: SymbolKind::Enum,
            scope_span: None,
        });

        // Add enum variants as top-level accessible
        for variant in &enum_def.variants {
            // Variants are often accessed as EnumName::Variant or just Variant
            // For now, add them as top-level with the variant name
            self.add_top_level(DefinitionInfo {
                name: variant.name.name.clone(),
                name_span: variant.name.span,
                kind: SymbolKind::EnumVariant,
                scope_span: None,
            });
        }
    }

    /// Collect symbols from an interface definition
    fn collect_interface(&mut self, interface_def: &InterfaceDef) {
        self.add_top_level(DefinitionInfo {
            name: interface_def.name.name.clone(),
            name_span: interface_def.name.span,
            kind: SymbolKind::Interface,
            scope_span: None,
        });
    }

    /// Collect symbols from an impl block
    fn collect_impl(&mut self, impl_def: &ImplDef) {
        // Methods in impl blocks are added as functions
        for method in &impl_def.methods {
            // Don't add methods to top-level - they're accessed via type
            // But we do need to collect their parameters and local variables
            for param in &method.params {
                self.add_scoped(DefinitionInfo {
                    name: param.name.name.clone(),
                    name_span: param.name.span,
                    kind: SymbolKind::Parameter,
                    scope_span: Some(method.body.span),
                });
            }
            self.collect_block(&method.body, method.body.span);
        }
    }

    /// Collect symbols from a top-level let declaration
    fn collect_top_level_let(&mut self, let_decl: &TopLevelLet) {
        self.collect_pattern_top_level(&let_decl.pattern);
    }

    /// Collect top-level symbols from a pattern
    fn collect_pattern_top_level(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Ident(ident) => {
                self.add_top_level(DefinitionInfo {
                    name: ident.name.clone(),
                    name_span: ident.span,
                    kind: SymbolKind::Variable,
                    scope_span: None,
                });
            }
            PatternKind::Struct { fields, .. } => {
                for field in fields {
                    if let Some(pat) = &field.pattern {
                        self.collect_pattern_top_level(pat);
                    } else {
                        // Shorthand: field name is the variable name
                        self.add_top_level(DefinitionInfo {
                            name: field.name.name.clone(),
                            name_span: field.name.span,
                            kind: SymbolKind::Variable,
                            scope_span: None,
                        });
                    }
                }
            }
            PatternKind::List { elements, rest } => {
                for elem in elements {
                    self.collect_pattern_top_level(elem);
                }
                if let Some(rest_pat) = rest {
                    self.collect_pattern_top_level(rest_pat);
                }
            }
            PatternKind::Or(patterns) => {
                // For or-patterns, all branches should bind the same names
                if let Some(first) = patterns.first() {
                    self.collect_pattern_top_level(first);
                }
            }
            PatternKind::Variant { data, .. } => {
                if let Some(d) = data {
                    self.collect_pattern_top_level(d);
                }
            }
            PatternKind::Wildcard | PatternKind::Literal(_) => {}
        }
    }

    /// Collect scoped symbols from a pattern
    fn collect_pattern_scoped(&mut self, pattern: &Pattern, scope_span: Span) {
        match &pattern.kind {
            PatternKind::Ident(ident) => {
                self.add_scoped(DefinitionInfo {
                    name: ident.name.clone(),
                    name_span: ident.span,
                    kind: SymbolKind::Variable,
                    scope_span: Some(scope_span),
                });
            }
            PatternKind::Struct { fields, .. } => {
                for field in fields {
                    if let Some(pat) = &field.pattern {
                        self.collect_pattern_scoped(pat, scope_span);
                    } else {
                        // Shorthand
                        self.add_scoped(DefinitionInfo {
                            name: field.name.name.clone(),
                            name_span: field.name.span,
                            kind: SymbolKind::Variable,
                            scope_span: Some(scope_span),
                        });
                    }
                }
            }
            PatternKind::List { elements, rest } => {
                for elem in elements {
                    self.collect_pattern_scoped(elem, scope_span);
                }
                if let Some(rest_pat) = rest {
                    self.collect_pattern_scoped(rest_pat, scope_span);
                }
            }
            PatternKind::Or(patterns) => {
                if let Some(first) = patterns.first() {
                    self.collect_pattern_scoped(first, scope_span);
                }
            }
            PatternKind::Variant { data, .. } => {
                if let Some(d) = data {
                    self.collect_pattern_scoped(d, scope_span);
                }
            }
            PatternKind::Wildcard | PatternKind::Literal(_) => {}
        }
    }

    /// Collect symbols from a block
    fn collect_block(&mut self, block: &Block, scope_span: Span) {
        for stmt in &block.stmts {
            self.collect_stmt(stmt, scope_span);
        }

        if let Some(expr) = &block.expr {
            self.collect_expr(expr, scope_span);
        }
    }

    /// Collect symbols from a statement
    fn collect_stmt(&mut self, stmt: &Stmt, scope_span: Span) {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                // Collect from value first (variables aren't visible in their own init)
                self.collect_expr(value, scope_span);
                // Then add the variables
                self.collect_pattern_scoped(pattern, scope_span);
            }
            StmtKind::Expr(expr) => {
                self.collect_expr(expr, scope_span);
            }
            StmtKind::Assign { target, value } => {
                self.collect_expr(target, scope_span);
                self.collect_expr(value, scope_span);
            }
            StmtKind::CompoundAssign { target, value, .. } => {
                self.collect_expr(target, scope_span);
                self.collect_expr(value, scope_span);
            }
            StmtKind::Return(Some(expr)) => {
                self.collect_expr(expr, scope_span);
            }
            StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
            StmtKind::For { pattern, iter, body } => {
                self.collect_expr(iter, scope_span);
                // For loop variable pattern is scoped to the body
                self.collect_pattern_scoped(pattern, body.span);
                self.collect_block(body, body.span);
            }
            StmtKind::While { cond, body } => {
                self.collect_expr(cond, scope_span);
                self.collect_block(body, body.span);
            }
            StmtKind::Loop { body } => {
                self.collect_block(body, body.span);
            }
            StmtKind::TryCatch { try_block, catches, finally } => {
                self.collect_block(try_block, try_block.span);
                for catch in catches {
                    // Catch binding is scoped to the catch body
                    if let Some(binding) = &catch.binding {
                        self.add_scoped(DefinitionInfo {
                            name: binding.name.clone(),
                            name_span: binding.span,
                            kind: SymbolKind::Variable,
                            scope_span: Some(catch.body.span),
                        });
                    }
                    self.collect_block(&catch.body, catch.body.span);
                }
                if let Some(finally_block) = finally {
                    self.collect_block(finally_block, finally_block.span);
                }
            }
            StmtKind::Throw(expr) => {
                self.collect_expr(expr, scope_span);
            }
        }
    }

    /// Collect symbols from an expression (mainly for nested scopes)
    fn collect_expr(&mut self, expr: &Expr, scope_span: Span) {
        match &expr.kind {
            ExprKind::Lambda { params, body, .. } => {
                // Lambda parameters are scoped to the lambda body
                // We need to determine the body span
                let body_span = body.span;
                for param in params {
                    self.add_scoped(DefinitionInfo {
                        name: param.name.name.clone(),
                        name_span: param.name.span,
                        kind: SymbolKind::Parameter,
                        scope_span: Some(body_span),
                    });
                }
                self.collect_expr(body, body_span);
            }
            ExprKind::Block(block) => {
                self.collect_block(block, block.span);
            }
            ExprKind::If { cond, then_branch, else_branch } => {
                self.collect_expr(cond, scope_span);
                self.collect_block(then_branch, then_branch.span);
                if let Some(else_br) = else_branch {
                    match else_br {
                        stratum_core::ast::ElseBranch::Block(block) => {
                            self.collect_block(block, block.span);
                        }
                        stratum_core::ast::ElseBranch::ElseIf(e) => {
                            self.collect_expr(e, scope_span);
                        }
                    }
                }
            }
            ExprKind::Match { expr: e, arms } => {
                self.collect_expr(e, scope_span);
                for arm in arms {
                    // Match arm patterns introduce variables scoped to the arm body
                    // The body is an expression, so we use its span
                    let arm_body_span = arm.body.span;
                    self.collect_pattern_scoped(&arm.pattern, arm_body_span);
                    if let Some(guard) = &arm.guard {
                        self.collect_expr(guard, arm_body_span);
                    }
                    self.collect_expr(&arm.body, arm_body_span);
                }
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_expr(left, scope_span);
                self.collect_expr(right, scope_span);
            }
            ExprKind::Unary { expr: e, .. } => {
                self.collect_expr(e, scope_span);
            }
            ExprKind::Call { callee, args, trailing_closure } => {
                self.collect_expr(callee, scope_span);
                for arg in args {
                    match arg {
                        CallArg::Positional(e) | CallArg::Named { value: e, .. } => {
                            self.collect_expr(e, scope_span);
                        }
                    }
                }
                if let Some(closure) = trailing_closure {
                    self.collect_expr(closure, scope_span);
                }
            }
            ExprKind::Index { expr: e, index } => {
                self.collect_expr(e, scope_span);
                self.collect_expr(index, scope_span);
            }
            ExprKind::Field { expr: e, .. } | ExprKind::NullSafeField { expr: e, .. } => {
                self.collect_expr(e, scope_span);
            }
            ExprKind::NullSafeIndex { expr: e, index } => {
                self.collect_expr(e, scope_span);
                self.collect_expr(index, scope_span);
            }
            ExprKind::List(elements) => {
                for elem in elements {
                    self.collect_expr(elem, scope_span);
                }
            }
            ExprKind::Map(pairs) => {
                for (k, v) in pairs {
                    self.collect_expr(k, scope_span);
                    self.collect_expr(v, scope_span);
                }
            }
            ExprKind::StructInit { fields, .. } => {
                for field in fields {
                    if let Some(value) = &field.value {
                        self.collect_expr(value, scope_span);
                    }
                }
            }
            ExprKind::EnumVariant { data, .. } => {
                if let Some(d) = data {
                    self.collect_expr(d, scope_span);
                }
            }
            ExprKind::Paren(inner) | ExprKind::Await(inner) | ExprKind::Try(inner)
            | ExprKind::StateBinding(inner) => {
                self.collect_expr(inner, scope_span);
            }
            ExprKind::StringInterp { parts } => {
                for part in parts {
                    if let stratum_core::ast::StringPart::Expr(e) = part {
                        self.collect_expr(e, scope_span);
                    }
                }
            }
            // Leaf expressions - no nested symbols
            ExprKind::Literal(_) | ExprKind::Ident(_) | ExprKind::Placeholder
            | ExprKind::ColumnShorthand(_) => {}
        }
    }
}

/// Result of a go-to-definition request
#[derive(Debug)]
pub struct DefinitionResult {
    /// The location of the definition
    pub location: Location,
}

/// Compute definition using cached data
pub fn compute_definition_cached(
    uri: &Url,
    data: &CachedData<'_>,
    position: Position,
) -> Option<DefinitionResult> {
    // Convert LSP position to byte offset
    let offset = position_to_offset(data.line_index, position)?;

    // Get cached AST and symbol index
    let module = data.ast()?;
    let index = data.symbol_index.as_ref()?;

    // Find the identifier at the position
    let ident_info = find_ident_at_position(module, offset)?;

    // Look up the definition
    let def_info = index.lookup(&ident_info.name, offset)?;

    // Convert to LSP location
    let range = span_to_range(def_info.name_span, data.line_index);
    Some(DefinitionResult {
        location: Location {
            uri: uri.clone(),
            range,
        },
    })
}

/// Compute definition (non-cached version for compatibility)
#[allow(dead_code)] // Standalone API used by tests
pub fn compute_definition(
    uri: &Url,
    source: &str,
    position: Position,
) -> Option<DefinitionResult> {
    let line_index = LineIndex::new(source);

    // Convert LSP position to byte offset
    let offset = position_to_offset(&line_index, position)?;

    // Parse the module
    let module = Parser::parse_module(source).ok()?;

    // Build symbol index
    let index = SymbolIndex::from_module(&module);

    // Find the identifier at the position
    let ident_info = find_ident_at_position(&module, offset)?;

    // Look up the definition
    let def_info = index.lookup(&ident_info.name, offset)?;

    // Convert to LSP location
    let range = span_to_range(def_info.name_span, &line_index);
    Some(DefinitionResult {
        location: Location {
            uri: uri.clone(),
            range,
        },
    })
}

/// Information about an identifier at a position
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
        PatternKind::Variant { enum_name, variant, data } => {
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
        StmtKind::For { pattern, iter, body } => {
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
        StmtKind::TryCatch { try_block, catches, finally } => {
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
        ExprKind::Call { callee, args, trailing_closure } => {
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
                // Field access - can't go to definition without type info
                // For now, we don't handle this case
                return None;
            }
            return find_ident_in_expr(e, offset);
        }
        ExprKind::NullSafeField { expr: e, field } => {
            if span_contains(field.span, offset) {
                return None;
            }
            return find_ident_in_expr(e, offset);
        }
        ExprKind::NullSafeIndex { expr: e, index } => {
            if let Some(info) = find_ident_in_expr(e, offset) {
                return Some(info);
            }
            return find_ident_in_expr(index, offset);
        }
        ExprKind::If { cond, then_branch, else_branch } => {
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
                // Field name in struct init can go to struct field definition
                // but we'd need type info for that
                if let Some(value) = &field.value {
                    if let Some(info) = find_ident_in_expr(value, offset) {
                        return Some(info);
                    }
                }
            }
        }
        ExprKind::EnumVariant { enum_name, variant, data } => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goto_function_definition() {
        let source = r#"
fx greet(name: String) -> String {
    "Hello, {name}!"
}

fx main() {
    let msg = greet("World")
    print(msg)
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "greet" in the call (line 7, after "let msg = ")
        let position = Position { line: 6, character: 14 };

        let result = compute_definition(&uri, source, position);
        assert!(result.is_some());

        let def = result.unwrap();
        // Should point to the function name definition on line 1
        assert_eq!(def.location.range.start.line, 1);
    }

    #[test]
    fn test_goto_local_variable_definition() {
        let source = r#"
fx main() {
    let x = 42
    let y = x + 1
    print(y)
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "x" in "x + 1" (line 3)
        let position = Position { line: 3, character: 12 };

        let result = compute_definition(&uri, source, position);
        assert!(result.is_some());

        let def = result.unwrap();
        // Should point to "let x" on line 2
        assert_eq!(def.location.range.start.line, 2);
    }

    #[test]
    fn test_goto_parameter_definition() {
        let source = r#"
fx add(a: Int, b: Int) -> Int {
    a + b
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "a" in "a + b" (line 2)
        let position = Position { line: 2, character: 4 };

        let result = compute_definition(&uri, source, position);
        assert!(result.is_some());

        let def = result.unwrap();
        // Should point to parameter "a" on line 1
        assert_eq!(def.location.range.start.line, 1);
    }

    #[test]
    fn test_goto_struct_definition() {
        let source = r#"
struct Point {
    x: Int,
    y: Int
}

fx main() {
    let p = Point { x: 1, y: 2 }
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "Point" in struct init (line 7)
        let position = Position { line: 7, character: 12 };

        let result = compute_definition(&uri, source, position);
        assert!(result.is_some());

        let def = result.unwrap();
        // Should point to struct definition on line 1
        assert_eq!(def.location.range.start.line, 1);
    }

    #[test]
    fn test_goto_for_loop_variable() {
        let source = r#"
fx main() {
    for i in [1, 2, 3] {
        print(i)
    }
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "i" in print(i) (line 3)
        let position = Position { line: 3, character: 14 };

        let result = compute_definition(&uri, source, position);
        assert!(result.is_some());

        let def = result.unwrap();
        // Should point to "i" in for loop on line 2
        assert_eq!(def.location.range.start.line, 2);
    }

    #[test]
    fn test_no_definition_for_unknown() {
        let source = r#"
fx main() {
    print(unknown_var)
}
"#;
        let uri = Url::parse("file:///test.strat").unwrap();

        // Position on "unknown_var"
        let position = Position { line: 2, character: 10 };

        let result = compute_definition(&uri, source, position);
        // Should not find a definition
        assert!(result.is_none());
    }
}
