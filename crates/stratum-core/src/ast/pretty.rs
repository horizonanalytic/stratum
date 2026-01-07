//! Pretty printing for AST nodes
//!
//! Implements Display trait for AST nodes to produce human-readable output.

use std::fmt::{self, Display, Formatter};

use super::{
    BinOp, Block, CallArg, CompoundOp, ElseBranch, EnumDef, EnumVariant, EnumVariantData, Expr,
    ExprKind, FieldInit, FieldPattern, Function, Ident, ImplDef, Import, ImportKind, InterfaceDef,
    InterfaceMethod, Item, ItemKind, Literal, MatchArm, Module, Param, Pattern, PatternKind, Stmt,
    StmtKind, StringPart, StructDef, StructField, TopLevelItem, TopLevelLet, TypeAnnotation,
    TypeKind, TypeParam, UnaryOp,
};

// ============================================================================
// Helpers
// ============================================================================

fn write_comma_separated<T: Display>(f: &mut Formatter<'_>, items: &[T]) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{item}")?;
    }
    Ok(())
}

// ============================================================================
// Basic types
// ============================================================================

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Display for CompoundOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Int(n) => write!(f, "{n}"),
            Literal::Float(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{n}.0")
                } else {
                    write!(f, "{n}")
                }
            }
            Literal::String(s) => write!(f, "\"{s}\""),
            Literal::Bool(b) => write!(f, "{b}"),
            Literal::Null => write!(f, "null"),
        }
    }
}

// ============================================================================
// Types
// ============================================================================

impl Display for TypeAnnotation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Named { name, args } => {
                write!(f, "{name}")?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    write_comma_separated(f, args)?;
                    write!(f, ">")?;
                }
                Ok(())
            }
            TypeKind::Nullable(inner) => write!(f, "{inner}?"),
            TypeKind::Function { params, ret } => {
                write!(f, "(")?;
                write_comma_separated(f, params)?;
                write!(f, ") -> {ret}")
            }
            TypeKind::Tuple(types) => {
                write!(f, "(")?;
                write_comma_separated(f, types)?;
                write!(f, ")")
            }
            TypeKind::List(inner) => write!(f, "[{inner}]"),
            TypeKind::Unit => write!(f, "()"),
            TypeKind::Never => write!(f, "!"),
            TypeKind::Inferred => write!(f, "_"),
        }
    }
}

impl Display for TypeParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.bounds.is_empty() {
            write!(f, ": ")?;
            for (i, bound) in self.bounds.iter().enumerate() {
                if i > 0 {
                    write!(f, " + ")?;
                }
                write!(f, "{bound}")?;
            }
        }
        Ok(())
    }
}

// ============================================================================
// Expressions
// ============================================================================

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Display for ExprKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ExprKind::Literal(lit) => write!(f, "{lit}"),
            ExprKind::Ident(name) => write!(f, "{name}"),
            ExprKind::Binary { left, op, right } => write!(f, "({left} {op} {right})"),
            ExprKind::Unary { op, expr } => write!(f, "{op}{expr}"),
            ExprKind::Paren(expr) => write!(f, "({expr})"),
            ExprKind::Call {
                callee,
                args,
                trailing_closure,
            } => {
                write!(f, "{callee}(")?;
                write_comma_separated(f, args)?;
                write!(f, ")")?;
                if let Some(closure) = trailing_closure {
                    write!(f, " {closure}")?;
                }
                Ok(())
            }
            ExprKind::Index { expr, index } => write!(f, "{expr}[{index}]"),
            ExprKind::Field { expr, field } => write!(f, "{expr}.{field}"),
            ExprKind::NullSafeField { expr, field } => write!(f, "{expr}?.{field}"),
            ExprKind::NullSafeIndex { expr, index } => write!(f, "{expr}?.[{index}]"),
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                write!(f, "if {cond} {then_branch}")?;
                if let Some(else_) = else_branch {
                    write!(f, " else {else_}")?;
                }
                Ok(())
            }
            ExprKind::Match { expr, arms } => {
                writeln!(f, "match {expr} {{")?;
                for arm in arms {
                    writeln!(f, "    {arm}")?;
                }
                write!(f, "}}")
            }
            ExprKind::Lambda {
                params,
                return_type,
                body,
            } => {
                write!(f, "|")?;
                write_comma_separated(f, params)?;
                write!(f, "|")?;
                if let Some(ret) = return_type {
                    write!(f, " -> {ret}")?;
                }
                write!(f, " {body}")
            }
            ExprKind::Block(block) => write!(f, "{block}"),
            ExprKind::List(items) => {
                write!(f, "[")?;
                write_comma_separated(f, items)?;
                write!(f, "]")
            }
            ExprKind::Map(entries) => {
                write!(f, "{{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            ExprKind::StringInterp { parts } => {
                write!(f, "\"")?;
                for part in parts {
                    match part {
                        StringPart::Literal(s) => write!(f, "{s}")?,
                        StringPart::Expr(e) => write!(f, "{{{e}}}")?,
                    }
                }
                write!(f, "\"")
            }
            ExprKind::Await(expr) => write!(f, "await {expr}"),
            ExprKind::Try(expr) => write!(f, "try {expr}"),
            ExprKind::StructInit { name, fields } => {
                write!(f, "{name} {{ ")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{field}")?;
                }
                write!(f, " }}")
            }
            ExprKind::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                if let Some(enum_name) = enum_name {
                    write!(f, "{enum_name}::")?;
                }
                write!(f, "{variant}")?;
                if let Some(data) = data {
                    write!(f, "({data})")?;
                }
                Ok(())
            }
            ExprKind::Placeholder => write!(f, "_"),
            ExprKind::ColumnShorthand(ident) => write!(f, ".{}", ident.name),
            ExprKind::StateBinding(expr) => write!(f, "&{expr}"),
        }
    }
}

impl Display for CallArg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CallArg::Positional(expr) => write!(f, "{expr}"),
            CallArg::Named { name, value, .. } => write!(f, "{name}: {value}"),
        }
    }
}

impl Display for FieldInit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(value) = &self.value {
            write!(f, ": {value}")?;
        }
        Ok(())
    }
}

impl Display for ElseBranch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ElseBranch::Block(block) => write!(f, "{block}"),
            ElseBranch::ElseIf(expr) => write!(f, "{expr}"),
        }
    }
}

impl Display for MatchArm {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pattern)?;
        if let Some(guard) = &self.guard {
            write!(f, " if {guard}")?;
        }
        write!(f, " => {}", self.body)
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(ty) = &self.ty {
            write!(f, ": {ty}")?;
        }
        if let Some(default) = &self.default {
            write!(f, " = {default}")?;
        }
        Ok(())
    }
}

// ============================================================================
// Patterns
// ============================================================================

impl Display for Pattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Display for PatternKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PatternKind::Wildcard => write!(f, "_"),
            PatternKind::Ident(name) => write!(f, "{name}"),
            PatternKind::Literal(lit) => write!(f, "{lit}"),
            PatternKind::Variant {
                enum_name,
                variant,
                data,
            } => {
                if let Some(enum_name) = enum_name {
                    write!(f, "{enum_name}::")?;
                }
                write!(f, "{variant}")?;
                if let Some(data) = data {
                    write!(f, "({data})")?;
                }
                Ok(())
            }
            PatternKind::Struct { name, fields } => {
                write!(f, "{name} {{ ")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{field}")?;
                }
                write!(f, " }}")
            }
            PatternKind::List { elements, rest } => {
                write!(f, "[")?;
                write_comma_separated(f, elements)?;
                if let Some(rest) = rest {
                    if !elements.is_empty() {
                        write!(f, ", ")?;
                    }
                    write!(f, "..{rest}")?;
                }
                write!(f, "]")
            }
            PatternKind::Or(patterns) => {
                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{pattern}")?;
                }
                Ok(())
            }
        }
    }
}

impl Display for FieldPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(pattern) = &self.pattern {
            write!(f, ": {pattern}")?;
        }
        Ok(())
    }
}

// ============================================================================
// Statements
// ============================================================================

impl Display for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Display for StmtKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StmtKind::Let { pattern, ty, value } => {
                write!(f, "let {pattern}")?;
                if let Some(ty) = ty {
                    write!(f, ": {ty}")?;
                }
                write!(f, " = {value}")
            }
            StmtKind::Expr(expr) => write!(f, "{expr}"),
            StmtKind::Assign { target, value } => write!(f, "{target} = {value}"),
            StmtKind::CompoundAssign { target, op, value } => write!(f, "{target} {op} {value}"),
            StmtKind::Return(expr) => {
                write!(f, "return")?;
                if let Some(expr) = expr {
                    write!(f, " {expr}")?;
                }
                Ok(())
            }
            StmtKind::For {
                pattern,
                iter,
                body,
            } => {
                write!(f, "for {pattern} in {iter} {body}")
            }
            StmtKind::While { cond, body } => write!(f, "while {cond} {body}"),
            StmtKind::Loop { body } => write!(f, "loop {body}"),
            StmtKind::Break => write!(f, "break"),
            StmtKind::Continue => write!(f, "continue"),
            StmtKind::TryCatch {
                try_block,
                catches,
                finally,
            } => {
                write!(f, "try {try_block}")?;
                for catch in catches {
                    write!(f, " catch")?;
                    if let Some(ty) = &catch.exception_type {
                        write!(f, " {ty}")?;
                    }
                    if let Some(binding) = &catch.binding {
                        write!(f, " as {binding}")?;
                    }
                    write!(f, " {}", catch.body)?;
                }
                if let Some(finally) = finally {
                    write!(f, " finally {finally}")?;
                }
                Ok(())
            }
            StmtKind::Throw(expr) => write!(f, "throw {expr}"),
        }
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.stmts.is_empty() && self.expr.is_none() {
            return write!(f, "{{ }}");
        }

        writeln!(f, "{{")?;
        for stmt in &self.stmts {
            writeln!(f, "    {stmt}")?;
        }
        if let Some(expr) = &self.expr {
            writeln!(f, "    {expr}")?;
        }
        write!(f, "}}")
    }
}

// ============================================================================
// Items
// ============================================================================

impl Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (i, tl_item) in self.top_level.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{tl_item}")?;
        }
        Ok(())
    }
}

impl Display for TopLevelItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TopLevelItem::Item(item) => write!(f, "{item}"),
            TopLevelItem::Let(let_decl) => write!(f, "{let_decl}"),
            TopLevelItem::Statement(stmt) => write!(f, "{stmt}"),
        }
    }
}

impl Display for TopLevelLet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "let {}", self.pattern)?;
        if let Some(ty) = &self.ty {
            write!(f, ": {ty}")?;
        }
        write!(f, " = {}", self.value)
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Display for ItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ItemKind::Function(func) => write!(f, "{func}"),
            ItemKind::Struct(s) => write!(f, "{s}"),
            ItemKind::Enum(e) => write!(f, "{e}"),
            ItemKind::Interface(i) => write!(f, "{i}"),
            ItemKind::Impl(i) => write!(f, "{i}"),
            ItemKind::Import(i) => write!(f, "{i}"),
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_async {
            write!(f, "async ")?;
        }
        write!(f, "fx {}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            write_comma_separated(f, &self.type_params)?;
            write!(f, ">")?;
        }
        write!(f, "(")?;
        write_comma_separated(f, &self.params)?;
        write!(f, ")")?;
        if let Some(ret) = &self.return_type {
            write!(f, " -> {ret}")?;
        }
        write!(f, " {}", self.body)
    }
}

impl Display for StructDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "struct {}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            write_comma_separated(f, &self.type_params)?;
            write!(f, ">")?;
        }
        writeln!(f, " {{")?;
        for field in &self.fields {
            writeln!(f, "    {field}")?;
        }
        write!(f, "}}")
    }
}

impl Display for StructField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_public {
            write!(f, "pub ")?;
        }
        write!(f, "{}: {}", self.name, self.ty)
    }
}

impl Display for EnumDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "enum {}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            write_comma_separated(f, &self.type_params)?;
            write!(f, ">")?;
        }
        writeln!(f, " {{")?;
        for variant in &self.variants {
            writeln!(f, "    {variant},")?;
        }
        write!(f, "}}")
    }
}

impl Display for EnumVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(data) = &self.data {
            match data {
                EnumVariantData::Tuple(types) => {
                    write!(f, "(")?;
                    write_comma_separated(f, types)?;
                    write!(f, ")")?;
                }
                EnumVariantData::Struct(fields) => {
                    write!(f, " {{ ")?;
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{field}")?;
                    }
                    write!(f, " }}")?;
                }
            }
        }
        Ok(())
    }
}

impl Display for InterfaceDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "interface {}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            write_comma_separated(f, &self.type_params)?;
            write!(f, ">")?;
        }
        writeln!(f, " {{")?;
        for method in &self.methods {
            writeln!(f, "    {method}")?;
        }
        write!(f, "}}")
    }
}

impl Display for InterfaceMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_async {
            write!(f, "async ")?;
        }
        write!(f, "fx {}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            write_comma_separated(f, &self.type_params)?;
            write!(f, ">")?;
        }
        write!(f, "(")?;
        write_comma_separated(f, &self.params)?;
        write!(f, ")")?;
        if let Some(ret) = &self.return_type {
            write!(f, " -> {ret}")?;
        }
        if let Some(body) = &self.default_body {
            write!(f, " {body}")?;
        }
        Ok(())
    }
}

impl Display for ImplDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "impl")?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            write_comma_separated(f, &self.type_params)?;
            write!(f, ">")?;
        }
        if let Some(interface) = &self.interface {
            write!(f, " {interface} for")?;
        }
        write!(f, " {}", self.target)?;
        writeln!(f, " {{")?;
        for method in &self.methods {
            writeln!(f, "    {method}")?;
        }
        write!(f, "}}")
    }
}

impl Display for Import {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "import ")?;
        for (i, segment) in self.path.iter().enumerate() {
            if i > 0 {
                write!(f, "::")?;
            }
            write!(f, "{segment}")?;
        }
        match &self.kind {
            ImportKind::Item => Ok(()),
            ImportKind::Glob => write!(f, "::*"),
            ImportKind::List(items) => {
                write!(f, "::{{ ")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item.name)?;
                    if let Some(alias) = &item.alias {
                        write!(f, " as {alias}")?;
                    }
                }
                write!(f, " }}")
            }
            ImportKind::Alias(alias) => write!(f, " as {alias}"),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    #[test]
    fn display_literals() {
        assert_eq!(format!("{}", Literal::Int(42)), "42");
        assert_eq!(format!("{}", Literal::Float(3.14)), "3.14");
        assert_eq!(format!("{}", Literal::Float(1.0)), "1.0");
        assert_eq!(format!("{}", Literal::String("hello".into())), "\"hello\"");
        assert_eq!(format!("{}", Literal::Bool(true)), "true");
        assert_eq!(format!("{}", Literal::Null), "null");
    }

    #[test]
    fn display_binary_operators() {
        assert_eq!(format!("{}", BinOp::Add), "+");
        assert_eq!(format!("{}", BinOp::Eq), "==");
        assert_eq!(format!("{}", BinOp::Pipe), "|>");
        assert_eq!(format!("{}", BinOp::NullCoalesce), "??");
    }

    #[test]
    fn display_simple_types() {
        let ty = TypeAnnotation::simple("Int", dummy_span());
        assert_eq!(format!("{ty}"), "Int");

        let nullable =
            TypeAnnotation::nullable(TypeAnnotation::simple("String", dummy_span()), dummy_span());
        assert_eq!(format!("{nullable}"), "String?");
    }

    #[test]
    fn display_expressions() {
        let lit = Expr::literal(Literal::Int(42), dummy_span());
        assert_eq!(format!("{lit}"), "42");

        let ident = Expr::ident("foo", dummy_span());
        assert_eq!(format!("{ident}"), "foo");
    }
}
