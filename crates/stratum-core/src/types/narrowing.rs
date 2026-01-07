//! Type narrowing for nullable types
//!
//! Analyzes conditions to determine which variables can have their types
//! narrowed in different branches. For example, `x != null` allows `x`
//! to be treated as non-nullable in the then-branch.

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, ExprKind, Literal, UnaryOp};

/// Information about type narrowing extracted from a condition
#[derive(Debug, Clone, Default)]
pub struct NarrowingInfo {
    /// Variables to narrow in the "true" branch (e.g., then-branch of if)
    /// Maps variable name to whether it should be unwrapped from nullable
    pub then_narrowings: HashMap<String, Narrowing>,

    /// Variables to narrow in the "false" branch (e.g., else-branch of if)
    pub else_narrowings: HashMap<String, Narrowing>,
}

/// A single narrowing action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Narrowing {
    /// Narrow from T? to T (unwrap nullable)
    UnwrapNullable,
}

impl NarrowingInfo {
    /// Create empty narrowing info
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Merge two narrowing infos with AND semantics
    /// Both conditions must be true, so we union the then-narrowings
    pub fn and(mut self, other: Self) -> Self {
        // For then-branch: both must be true, so we can narrow both
        self.then_narrowings.extend(other.then_narrowings);
        // For else-branch: at least one is false, so we can only narrow
        // if both would narrow the same variable (intersection)
        self.else_narrowings = self
            .else_narrowings
            .into_iter()
            .filter(|(k, v)| other.else_narrowings.get(k) == Some(v))
            .collect();
        self
    }

    /// Merge two narrowing infos with OR semantics
    /// At least one condition is true, so we intersect the then-narrowings
    pub fn or(mut self, other: Self) -> Self {
        // For then-branch: at least one is true, so we can only narrow
        // if both would narrow the same variable (intersection)
        self.then_narrowings = self
            .then_narrowings
            .into_iter()
            .filter(|(k, v)| other.then_narrowings.get(k) == Some(v))
            .collect();
        // For else-branch: both are false, so we can narrow both
        self.else_narrowings.extend(other.else_narrowings);
        self
    }

    /// Negate the narrowing (swap then and else)
    pub fn negate(self) -> Self {
        Self {
            then_narrowings: self.else_narrowings,
            else_narrowings: self.then_narrowings,
        }
    }
}

/// Extract narrowing information from a condition expression
pub fn extract_narrowing(cond: &Expr) -> NarrowingInfo {
    match &cond.kind {
        // x != null  ->  narrow x in then-branch
        ExprKind::Binary {
            left,
            op: BinOp::Ne,
            right,
        } => {
            if let Some(name) = extract_null_check(left, right) {
                let mut info = NarrowingInfo::empty();
                info.then_narrowings.insert(name, Narrowing::UnwrapNullable);
                return info;
            }
            if let Some(name) = extract_null_check(right, left) {
                let mut info = NarrowingInfo::empty();
                info.then_narrowings.insert(name, Narrowing::UnwrapNullable);
                return info;
            }
            NarrowingInfo::empty()
        }

        // x == null  ->  narrow x in else-branch
        ExprKind::Binary {
            left,
            op: BinOp::Eq,
            right,
        } => {
            if let Some(name) = extract_null_check(left, right) {
                let mut info = NarrowingInfo::empty();
                info.else_narrowings.insert(name, Narrowing::UnwrapNullable);
                return info;
            }
            if let Some(name) = extract_null_check(right, left) {
                let mut info = NarrowingInfo::empty();
                info.else_narrowings.insert(name, Narrowing::UnwrapNullable);
                return info;
            }
            NarrowingInfo::empty()
        }

        // x && y  ->  combine with AND semantics
        ExprKind::Binary {
            left,
            op: BinOp::And,
            right,
        } => {
            let left_info = extract_narrowing(left);
            let right_info = extract_narrowing(right);
            left_info.and(right_info)
        }

        // x || y  ->  combine with OR semantics
        ExprKind::Binary {
            left,
            op: BinOp::Or,
            right,
        } => {
            let left_info = extract_narrowing(left);
            let right_info = extract_narrowing(right);
            left_info.or(right_info)
        }

        // !x  ->  negate the narrowing
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => extract_narrowing(expr).negate(),

        // (x)  ->  extract from inner
        ExprKind::Paren(inner) => extract_narrowing(inner),

        // Bare identifier (truthiness check)
        // if x { ... }  where x: T?  ->  narrow x in then-branch
        ExprKind::Ident(ident) => {
            let mut info = NarrowingInfo::empty();
            info.then_narrowings
                .insert(ident.name.clone(), Narrowing::UnwrapNullable);
            info
        }

        _ => NarrowingInfo::empty(),
    }
}

/// Check if this is a pattern like `x` compared to `null`, returning the variable name
fn extract_null_check(var_side: &Expr, null_side: &Expr) -> Option<String> {
    // Check if null_side is a null literal
    if !matches!(null_side.kind, ExprKind::Literal(Literal::Null)) {
        return None;
    }

    // Check if var_side is an identifier
    if let ExprKind::Ident(ident) = &var_side.kind {
        return Some(ident.name.clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn parse_expr(source: &str) -> Expr {
        // Wrap in a function to make it valid syntax
        let full = format!("fx test() {{ {source} }}");
        let module = Parser::parse_module(&full).expect("parse failed");
        let items = module.items();
        if let crate::ast::ItemKind::Function(f) = &items[0].kind {
            if let Some(expr) = &f.body.expr {
                return (**expr).clone();
            }
        }
        panic!("Expected expression");
    }

    #[test]
    fn test_ne_null_narrows_then() {
        let expr = parse_expr("x != null");
        let info = extract_narrowing(&expr);

        assert!(info.then_narrowings.contains_key("x"));
        assert!(info.else_narrowings.is_empty());
    }

    #[test]
    fn test_eq_null_narrows_else() {
        let expr = parse_expr("x == null");
        let info = extract_narrowing(&expr);

        assert!(info.then_narrowings.is_empty());
        assert!(info.else_narrowings.contains_key("x"));
    }

    #[test]
    fn test_and_combines_then() {
        let expr = parse_expr("x != null && y != null");
        let info = extract_narrowing(&expr);

        assert!(info.then_narrowings.contains_key("x"));
        assert!(info.then_narrowings.contains_key("y"));
    }

    #[test]
    fn test_or_combines_else() {
        let expr = parse_expr("x == null || y == null");
        let info = extract_narrowing(&expr);

        assert!(info.else_narrowings.contains_key("x"));
        assert!(info.else_narrowings.contains_key("y"));
    }

    #[test]
    fn test_negation_swaps_branches() {
        let expr = parse_expr("!(x == null)");
        let info = extract_narrowing(&expr);

        // !(x == null) is like x != null
        assert!(info.then_narrowings.contains_key("x"));
        assert!(info.else_narrowings.is_empty());
    }

    #[test]
    fn test_bare_ident_narrows_then() {
        let expr = parse_expr("x");
        let info = extract_narrowing(&expr);

        assert!(info.then_narrowings.contains_key("x"));
    }
}
