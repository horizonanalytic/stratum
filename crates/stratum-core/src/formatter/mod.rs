//! Source code formatter for the Stratum programming language
//!
//! This module provides source code formatting capabilities with consistent style:
//! - 4-space indentation
//! - Consistent brace placement
//! - Proper operator spacing
//! - Line length limits (100 characters soft limit)
//! - Comment preservation

use crate::ast::{
    Attribute, AttributeArg, Block, CallArg, CatchClause, Comment, ElseBranch, EnumDef, EnumVariant,
    EnumVariantData, Expr, ExprKind, FieldInit, FieldPattern, Function, ImplDef, Import,
    ImportKind, InterfaceDef, InterfaceMethod, Item, ItemKind, Literal, MatchArm, Module, Param,
    Pattern, PatternKind, Stmt, StmtKind, StringPart, StructDef, StructField, TopLevelItem,
    TopLevelLet, Trivia, TypeAnnotation, TypeKind, TypeParam,
};

/// Default indentation: 4 spaces
const INDENT: &str = "    ";

/// Soft line length limit
const LINE_LIMIT: usize = 100;

/// Formatter configuration
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Number of spaces for indentation
    pub indent_size: usize,
    /// Maximum line length (soft limit)
    pub max_line_length: usize,
    /// Whether to add trailing newline
    pub trailing_newline: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_line_length: LINE_LIMIT,
            trailing_newline: true,
        }
    }
}

/// The source code formatter
pub struct Formatter {
    /// Output buffer
    output: String,
    /// Current indentation level
    indent_level: usize,
    /// Configuration
    config: FormatConfig,
    /// Whether we're at the start of a line
    at_line_start: bool,
}

impl Formatter {
    /// Create a new formatter with default config
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(FormatConfig::default())
    }

    /// Create a new formatter with custom config
    #[must_use]
    pub fn with_config(config: FormatConfig) -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            config,
            at_line_start: true,
        }
    }

    /// Format a module and return the formatted source code
    #[must_use]
    pub fn format_module(module: &Module) -> String {
        let mut formatter = Self::new();
        formatter.write_module(module);
        if formatter.config.trailing_newline && !formatter.output.ends_with('\n') {
            formatter.output.push('\n');
        }
        formatter.output
    }

    /// Check if formatting would change the source
    #[must_use]
    pub fn check_module(source: &str, module: &Module) -> bool {
        let formatted = Self::format_module(module);
        source == formatted
    }

    // ==================== Output Helpers ====================

    fn write(&mut self, s: &str) {
        if self.at_line_start && !s.is_empty() && s != "\n" {
            self.write_indent();
            self.at_line_start = false;
        }
        self.output.push_str(s);
    }

    fn writeln(&mut self) {
        self.output.push('\n');
        self.at_line_start = true;
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str(INDENT);
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    fn write_space(&mut self) {
        self.write(" ");
    }

    // ==================== Trivia/Comments ====================

    fn write_leading_trivia(&mut self, trivia: &Trivia) {
        for comment in &trivia.leading {
            self.write_comment(comment);
            self.writeln();
        }
    }

    fn write_trailing_trivia(&mut self, trivia: &Trivia) {
        if let Some(comment) = &trivia.trailing {
            self.write_space();
            self.write(&comment.text);
        }
    }

    fn write_comment(&mut self, comment: &Comment) {
        self.write(&comment.text);
    }

    // ==================== Module ====================

    fn write_module(&mut self, module: &Module) {
        // Write file-level leading comments
        self.write_leading_trivia(&module.trivia);

        // Write inner attributes
        for attr in &module.inner_attributes {
            self.write_inner_attribute(attr);
            self.writeln();
        }

        // Add blank line after inner attributes if present
        if !module.inner_attributes.is_empty() && !module.top_level.is_empty() {
            self.writeln();
        }

        // Write top-level items
        let mut prev_was_function = false;
        for (i, item) in module.top_level.iter().enumerate() {
            // Add blank line between top-level items (except after first)
            if i > 0 {
                // Add extra blank line between functions
                if prev_was_function || matches!(item, TopLevelItem::Item(Item { kind: ItemKind::Function(_), .. })) {
                    self.writeln();
                }
            }

            self.write_top_level_item(item);
            self.writeln();

            prev_was_function = matches!(item, TopLevelItem::Item(Item { kind: ItemKind::Function(_), .. }));
        }
    }

    fn write_inner_attribute(&mut self, attr: &Attribute) {
        self.write("#![");
        self.write(&attr.name.name);
        if !attr.args.is_empty() {
            self.write("(");
            self.write_attribute_args(&attr.args);
            self.write(")");
        }
        self.write("]");
    }

    fn write_attribute(&mut self, attr: &Attribute) {
        self.write("#[");
        self.write(&attr.name.name);
        if !attr.args.is_empty() {
            self.write("(");
            self.write_attribute_args(&attr.args);
            self.write(")");
        }
        self.write("]");
    }

    fn write_attribute_args(&mut self, args: &[AttributeArg]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            match arg {
                AttributeArg::Ident(ident) => self.write(&ident.name),
                AttributeArg::NameValue { name, value } => {
                    self.write(&name.name);
                    self.write(" = ");
                    self.write_expr(value);
                }
            }
        }
    }

    // ==================== Top-Level Items ====================

    fn write_top_level_item(&mut self, item: &TopLevelItem) {
        match item {
            TopLevelItem::Item(item) => self.write_item(item),
            TopLevelItem::Let(let_decl) => self.write_top_level_let(let_decl),
            TopLevelItem::Statement(stmt) => self.write_stmt(stmt),
        }
    }

    fn write_top_level_let(&mut self, let_decl: &TopLevelLet) {
        self.write_leading_trivia(&let_decl.trivia);
        self.write("let ");
        self.write_pattern(&let_decl.pattern);
        if let Some(ty) = &let_decl.ty {
            self.write(": ");
            self.write_type(ty);
        }
        self.write(" = ");
        self.write_expr(&let_decl.value);
    }

    fn write_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function(func) => self.write_function(func),
            ItemKind::Struct(s) => self.write_struct(s),
            ItemKind::Enum(e) => self.write_enum(e),
            ItemKind::Interface(i) => self.write_interface(i),
            ItemKind::Impl(i) => self.write_impl(i),
            ItemKind::Import(i) => self.write_import(i),
        }
    }

    // ==================== Functions ====================

    fn write_function(&mut self, func: &Function) {
        // Leading comments
        self.write_leading_trivia(&func.trivia);

        // Attributes
        for attr in &func.attributes {
            self.write_attribute(attr);
            self.writeln();
        }

        // async fx name
        if func.is_async {
            self.write("async ");
        }
        self.write("fx ");
        self.write(&func.name.name);

        // Type parameters
        if !func.type_params.is_empty() {
            self.write("<");
            self.write_type_params(&func.type_params);
            self.write(">");
        }

        // Parameters
        self.write("(");
        self.write_params(&func.params);
        self.write(")");

        // Return type
        if let Some(ret) = &func.return_type {
            self.write(" -> ");
            self.write_type(ret);
        }

        // Body
        self.write_space();
        self.write_block(&func.body);
    }

    fn write_params(&mut self, params: &[Param]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&param.name.name);
            if let Some(ty) = &param.ty {
                self.write(": ");
                self.write_type(ty);
            }
            if let Some(default) = &param.default {
                self.write(" = ");
                self.write_expr(default);
            }
        }
    }

    fn write_type_params(&mut self, params: &[TypeParam]) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&param.name.name);
            if !param.bounds.is_empty() {
                self.write(": ");
                for (j, bound) in param.bounds.iter().enumerate() {
                    if j > 0 {
                        self.write(" + ");
                    }
                    self.write(&bound.name);
                }
            }
        }
    }

    // ==================== Structs ====================

    fn write_struct(&mut self, s: &StructDef) {
        self.write_leading_trivia(&s.trivia);
        self.write("struct ");
        self.write(&s.name.name);

        if !s.type_params.is_empty() {
            self.write("<");
            self.write_type_params(&s.type_params);
            self.write(">");
        }

        self.write(" {");
        if !s.fields.is_empty() {
            self.writeln();
            self.indent();
            for field in &s.fields {
                self.write_struct_field(field);
                self.writeln();
            }
            self.dedent();
        }
        self.write("}");
    }

    fn write_struct_field(&mut self, field: &StructField) {
        if field.is_public {
            self.write("pub ");
        }
        self.write(&field.name.name);
        self.write(": ");
        self.write_type(&field.ty);
    }

    // ==================== Enums ====================

    fn write_enum(&mut self, e: &EnumDef) {
        self.write_leading_trivia(&e.trivia);
        self.write("enum ");
        self.write(&e.name.name);

        if !e.type_params.is_empty() {
            self.write("<");
            self.write_type_params(&e.type_params);
            self.write(">");
        }

        self.write(" {");
        if !e.variants.is_empty() {
            self.writeln();
            self.indent();
            for variant in &e.variants {
                self.write_enum_variant(variant);
                self.write(",");
                self.writeln();
            }
            self.dedent();
        }
        self.write("}");
    }

    fn write_enum_variant(&mut self, variant: &EnumVariant) {
        self.write(&variant.name.name);
        if let Some(data) = &variant.data {
            match data {
                EnumVariantData::Tuple(types) => {
                    self.write("(");
                    for (i, ty) in types.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write_type(ty);
                    }
                    self.write(")");
                }
                EnumVariantData::Struct(fields) => {
                    self.write(" { ");
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write_struct_field(field);
                    }
                    self.write(" }");
                }
            }
        }
    }

    // ==================== Interfaces ====================

    fn write_interface(&mut self, iface: &InterfaceDef) {
        self.write_leading_trivia(&iface.trivia);
        self.write("interface ");
        self.write(&iface.name.name);

        if !iface.type_params.is_empty() {
            self.write("<");
            self.write_type_params(&iface.type_params);
            self.write(">");
        }

        self.write(" {");
        if !iface.methods.is_empty() {
            self.writeln();
            self.indent();
            for method in &iface.methods {
                self.write_interface_method(method);
                self.writeln();
            }
            self.dedent();
        }
        self.write("}");
    }

    fn write_interface_method(&mut self, method: &InterfaceMethod) {
        if method.is_async {
            self.write("async ");
        }
        self.write("fx ");
        self.write(&method.name.name);

        if !method.type_params.is_empty() {
            self.write("<");
            self.write_type_params(&method.type_params);
            self.write(">");
        }

        self.write("(");
        self.write_params(&method.params);
        self.write(")");

        if let Some(ret) = &method.return_type {
            self.write(" -> ");
            self.write_type(ret);
        }

        if let Some(body) = &method.default_body {
            self.write_space();
            self.write_block(body);
        }
    }

    // ==================== Impl ====================

    fn write_impl(&mut self, imp: &ImplDef) {
        self.write_leading_trivia(&imp.trivia);
        self.write("impl");

        if !imp.type_params.is_empty() {
            self.write("<");
            self.write_type_params(&imp.type_params);
            self.write(">");
        }

        if let Some(iface) = &imp.interface {
            self.write_space();
            self.write_type(iface);
            self.write(" for");
        }

        self.write_space();
        self.write_type(&imp.target);

        self.write(" {");
        if !imp.methods.is_empty() {
            self.writeln();
            self.indent();
            for (i, method) in imp.methods.iter().enumerate() {
                if i > 0 {
                    self.writeln();
                }
                self.write_function(method);
                self.writeln();
            }
            self.dedent();
        }
        self.write("}");
    }

    // ==================== Imports ====================

    fn write_import(&mut self, imp: &Import) {
        self.write("import ");
        for (i, seg) in imp.path.iter().enumerate() {
            if i > 0 {
                self.write("::");
            }
            self.write(&seg.name);
        }
        match &imp.kind {
            ImportKind::Item => {}
            ImportKind::Glob => self.write("::*"),
            ImportKind::List(items) => {
                self.write("::{ ");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&item.name.name);
                    if let Some(alias) = &item.alias {
                        self.write(" as ");
                        self.write(&alias.name);
                    }
                }
                self.write(" }");
            }
            ImportKind::Alias(alias) => {
                self.write(" as ");
                self.write(&alias.name);
            }
        }
    }

    // ==================== Blocks ====================

    fn write_block(&mut self, block: &Block) {
        self.write("{");
        if block.stmts.is_empty() && block.expr.is_none() {
            self.write("}");
            return;
        }

        self.writeln();
        self.indent();

        for stmt in &block.stmts {
            self.write_stmt(stmt);
            self.writeln();
        }

        if let Some(expr) = &block.expr {
            self.write_expr(expr);
            self.writeln();
        }

        self.dedent();
        self.write("}");
    }

    // ==================== Statements ====================

    fn write_stmt(&mut self, stmt: &Stmt) {
        self.write_leading_trivia(&stmt.trivia);

        match &stmt.kind {
            StmtKind::Let { pattern, ty, value } => {
                self.write("let ");
                self.write_pattern(pattern);
                if let Some(ty) = ty {
                    self.write(": ");
                    self.write_type(ty);
                }
                self.write(" = ");
                self.write_expr(value);
            }
            StmtKind::Expr(expr) => {
                self.write_expr(expr);
            }
            StmtKind::Assign { target, value } => {
                self.write_expr(target);
                self.write(" = ");
                self.write_expr(value);
            }
            StmtKind::CompoundAssign { target, op, value } => {
                self.write_expr(target);
                self.write_space();
                self.write(op.as_str());
                self.write_space();
                self.write_expr(value);
            }
            StmtKind::Return(expr) => {
                self.write("return");
                if let Some(e) = expr {
                    self.write_space();
                    self.write_expr(e);
                }
            }
            StmtKind::For { pattern, iter, body } => {
                self.write("for ");
                self.write_pattern(pattern);
                self.write(" in ");
                self.write_expr(iter);
                self.write_space();
                self.write_block(body);
            }
            StmtKind::While { cond, body } => {
                self.write("while ");
                self.write_expr(cond);
                self.write_space();
                self.write_block(body);
            }
            StmtKind::Loop { body } => {
                self.write("loop ");
                self.write_block(body);
            }
            StmtKind::Break => self.write("break"),
            StmtKind::Continue => self.write("continue"),
            StmtKind::TryCatch {
                try_block,
                catches,
                finally,
            } => {
                self.write("try ");
                self.write_block(try_block);
                for catch in catches {
                    self.write_catch_clause(catch);
                }
                if let Some(fin) = finally {
                    self.write(" finally ");
                    self.write_block(fin);
                }
            }
            StmtKind::Throw(expr) => {
                self.write("throw ");
                self.write_expr(expr);
            }
        }
    }

    fn write_catch_clause(&mut self, catch: &CatchClause) {
        self.write(" catch");
        if let Some(ty) = &catch.exception_type {
            self.write_space();
            self.write_type(ty);
        }
        if let Some(binding) = &catch.binding {
            self.write(" as ");
            self.write(&binding.name);
        }
        self.write_space();
        self.write_block(&catch.body);
    }

    // ==================== Expressions ====================

    fn write_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Literal(lit) => self.write_literal(lit),
            ExprKind::Ident(name) => self.write(&name.name),
            ExprKind::Binary { left, op, right } => {
                self.write_expr(left);
                self.write_space();
                self.write(op.as_str());
                self.write_space();
                self.write_expr(right);
            }
            ExprKind::Unary { op, expr } => {
                self.write(op.as_str());
                self.write_expr(expr);
            }
            ExprKind::Paren(inner) => {
                self.write("(");
                self.write_expr(inner);
                self.write(")");
            }
            ExprKind::Call { callee, args, trailing_closure } => {
                self.write_expr(callee);
                self.write("(");
                self.write_call_args(args);
                self.write(")");
                if let Some(closure) = trailing_closure {
                    self.write(" ");
                    self.write_expr(closure);
                }
            }
            ExprKind::Index { expr, index } => {
                self.write_expr(expr);
                self.write("[");
                self.write_expr(index);
                self.write("]");
            }
            ExprKind::Field { expr, field } => {
                self.write_expr(expr);
                self.write(".");
                self.write(&field.name);
            }
            ExprKind::NullSafeField { expr, field } => {
                self.write_expr(expr);
                self.write("?.");
                self.write(&field.name);
            }
            ExprKind::NullSafeIndex { expr, index } => {
                self.write_expr(expr);
                self.write("?.[");
                self.write_expr(index);
                self.write("]");
            }
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.write("if ");
                self.write_expr(cond);
                self.write_space();
                self.write_block(then_branch);
                if let Some(else_br) = else_branch {
                    self.write(" else ");
                    match else_br {
                        ElseBranch::Block(block) => self.write_block(block),
                        ElseBranch::ElseIf(expr) => self.write_expr(expr),
                    }
                }
            }
            ExprKind::Match { expr, arms } => {
                self.write("match ");
                self.write_expr(expr);
                self.write(" {");
                self.writeln();
                self.indent();
                for arm in arms {
                    self.write_match_arm(arm);
                    self.writeln();
                }
                self.dedent();
                self.write("}");
            }
            ExprKind::Lambda {
                params,
                return_type,
                body,
            } => {
                self.write("|");
                self.write_params(params);
                self.write("|");
                if let Some(ret) = return_type {
                    self.write(" -> ");
                    self.write_type(ret);
                }
                self.write_space();
                self.write_lambda_body(body);
            }
            ExprKind::Block(block) => self.write_block(block),
            ExprKind::List(items) => {
                self.write("[");
                self.write_args(items);
                self.write("]");
            }
            ExprKind::Map(entries) => {
                self.write("{");
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write_expr(k);
                    self.write(": ");
                    self.write_expr(v);
                }
                self.write("}");
            }
            ExprKind::StringInterp { parts } => {
                self.write("\"");
                for part in parts {
                    match part {
                        StringPart::Literal(s) => self.write(s),
                        StringPart::Expr(e) => {
                            self.write("{");
                            self.write_expr(e);
                            self.write("}");
                        }
                    }
                }
                self.write("\"");
            }
            ExprKind::Await(inner) => {
                self.write("await ");
                self.write_expr(inner);
            }
            ExprKind::Try(inner) => {
                self.write("try ");
                self.write_expr(inner);
            }
            ExprKind::StructInit { name, fields } => {
                self.write(&name.name);
                self.write(" { ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write_field_init(field);
                }
                self.write(" }");
            }
            ExprKind::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                if let Some(name) = enum_name {
                    self.write(&name.name);
                    self.write("::");
                }
                self.write(&variant.name);
                if let Some(d) = data {
                    self.write("(");
                    self.write_expr(d);
                    self.write(")");
                }
            }
            ExprKind::Placeholder => {
                self.write("_");
            }
            ExprKind::ColumnShorthand(ident) => {
                self.write(".");
                self.write(&ident.name);
            }
            ExprKind::StateBinding(inner) => {
                self.write("&");
                self.write_expr(inner);
            }
        }
    }

    fn write_call_args(&mut self, args: &[CallArg]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            match arg {
                CallArg::Positional(expr) => self.write_expr(expr),
                CallArg::Named { name, value, .. } => {
                    self.write(&name.name);
                    self.write(": ");
                    self.write_expr(value);
                }
            }
        }
    }

    fn write_lambda_body(&mut self, body: &Expr) {
        // For simple expressions, don't wrap in block
        match &body.kind {
            ExprKind::Block(_) => self.write_expr(body),
            _ => self.write_expr(body),
        }
    }

    fn write_args(&mut self, args: &[Expr]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write_expr(arg);
        }
    }

    fn write_match_arm(&mut self, arm: &MatchArm) {
        self.write_pattern(&arm.pattern);
        if let Some(guard) = &arm.guard {
            self.write(" if ");
            self.write_expr(guard);
        }
        self.write(" => ");
        self.write_expr(&arm.body);
    }

    fn write_field_init(&mut self, field: &FieldInit) {
        self.write(&field.name.name);
        if let Some(value) = &field.value {
            self.write(": ");
            self.write_expr(value);
        }
    }

    fn write_literal(&mut self, lit: &Literal) {
        match lit {
            Literal::Int(n) => self.write(&n.to_string()),
            Literal::Float(n) => {
                if n.fract() == 0.0 {
                    self.write(&format!("{n}.0"));
                } else {
                    self.write(&n.to_string());
                }
            }
            Literal::String(s) => {
                self.write("\"");
                self.write(s);
                self.write("\"");
            }
            Literal::Bool(b) => self.write(if *b { "true" } else { "false" }),
            Literal::Null => self.write("null"),
        }
    }

    // ==================== Patterns ====================

    fn write_pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard => self.write("_"),
            PatternKind::Ident(name) => self.write(&name.name),
            PatternKind::Literal(lit) => self.write_literal(lit),
            PatternKind::Variant {
                enum_name,
                variant,
                data,
            } => {
                if let Some(name) = enum_name {
                    self.write(&name.name);
                    self.write("::");
                }
                self.write(&variant.name);
                if let Some(d) = data {
                    self.write("(");
                    self.write_pattern(d);
                    self.write(")");
                }
            }
            PatternKind::Struct { name, fields } => {
                self.write(&name.name);
                self.write(" { ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write_field_pattern(field);
                }
                self.write(" }");
            }
            PatternKind::List { elements, rest } => {
                self.write("[");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write_pattern(elem);
                }
                if let Some(r) = rest {
                    if !elements.is_empty() {
                        self.write(", ");
                    }
                    self.write("..");
                    self.write_pattern(r);
                }
                self.write("]");
            }
            PatternKind::Or(patterns) => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.write(" | ");
                    }
                    self.write_pattern(p);
                }
            }
        }
    }

    fn write_field_pattern(&mut self, field: &FieldPattern) {
        self.write(&field.name.name);
        if let Some(pattern) = &field.pattern {
            self.write(": ");
            self.write_pattern(pattern);
        }
    }

    // ==================== Types ====================

    fn write_type(&mut self, ty: &TypeAnnotation) {
        match &ty.kind {
            TypeKind::Named { name, args } => {
                self.write(&name.name);
                if !args.is_empty() {
                    self.write("<");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.write_type(arg);
                    }
                    self.write(">");
                }
            }
            TypeKind::Nullable(inner) => {
                self.write_type(inner);
                self.write("?");
            }
            TypeKind::Function { params, ret } => {
                self.write("(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write_type(p);
                }
                self.write(") -> ");
                self.write_type(ret);
            }
            TypeKind::Tuple(types) => {
                self.write("(");
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write_type(t);
                }
                self.write(")");
            }
            TypeKind::List(inner) => {
                self.write("[");
                self.write_type(inner);
                self.write("]");
            }
            TypeKind::Unit => self.write("()"),
            TypeKind::Never => self.write("!"),
            TypeKind::Inferred => self.write("_"),
        }
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn format_code(source: &str) -> String {
        let module = Parser::parse_module(source).expect("parse failed");
        Formatter::format_module(&module)
    }

    #[test]
    fn test_format_simple_function() {
        let source = "fx add(a:Int,b:Int)->Int{a+b}";
        let formatted = format_code(source);
        assert!(formatted.contains("fx add(a: Int, b: Int) -> Int {"));
        assert!(formatted.contains("a + b"));
    }

    #[test]
    fn test_format_struct() {
        let source = "struct Point{x:Int,y:Int}";
        let formatted = format_code(source);
        assert!(formatted.contains("struct Point {"), "Should contain 'struct Point {{': {}", formatted);
        // Note: All struct fields are public by default in Stratum
        assert!(formatted.contains("    pub x: Int"), "Should contain '    pub x: Int': {}", formatted);
        assert!(formatted.contains("    pub y: Int"), "Should contain '    pub y: Int': {}", formatted);
    }

    #[test]
    fn test_format_preserves_comments() {
        let source = "// This is a comment\nfx main() {}";
        let formatted = format_code(source);
        assert!(formatted.contains("// This is a comment"));
    }

    #[test]
    fn test_format_idempotent() {
        let source = r#"
fx add(a: Int, b: Int) -> Int {
    a + b
}
"#.trim();
        let formatted1 = format_code(source);
        let formatted2 = format_code(&formatted1);
        assert_eq!(formatted1, formatted2, "Formatting should be idempotent");
    }
}
