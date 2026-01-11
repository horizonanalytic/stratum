//! Type inference engine for the Stratum type checker
//!
//! Implements Hindley-Milner style type inference with unification.

use std::collections::HashMap;

use crate::lexer::Span;

use super::error::{TypeError, TypeErrorKind};
use super::{Type, TypeVarId};

/// Type inference engine
///
/// Manages type variable generation, constraint collection, and unification.
#[derive(Debug, Clone)]
pub struct TypeInference {
    /// Substitution mapping type variables to their resolved types
    substitution: HashMap<TypeVarId, Type>,

    /// Errors collected during inference
    errors: Vec<TypeError>,
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInference {
    /// Create a new type inference engine
    #[must_use]
    pub fn new() -> Self {
        Self {
            substitution: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Generate a fresh type variable
    #[must_use]
    pub fn fresh_var(&mut self) -> Type {
        Type::TypeVar(TypeVarId::fresh())
    }

    /// Get collected errors
    #[must_use]
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// Take collected errors
    pub fn take_errors(&mut self) -> Vec<TypeError> {
        std::mem::take(&mut self.errors)
    }

    /// Add an error
    pub fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    /// Unify two types, adding to the substitution if successful
    ///
    /// Returns true if unification succeeded, false otherwise.
    pub fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> bool {
        let t1 = self.apply(t1);
        let t2 = self.apply(t2);

        self.unify_impl(&t1, &t2, span)
    }

    /// Internal unification implementation
    fn unify_impl(&mut self, t1: &Type, t2: &Type, span: Span) -> bool {
        // Error types unify with anything (to prevent cascading errors)
        if t1.is_error() || t2.is_error() {
            return true;
        }

        // Never type unifies with anything (it's the bottom type)
        if t1.is_never() || t2.is_never() {
            return true;
        }

        // Any type unifies with anything (it's the top type for polymorphic builtins)
        if t1.is_any() || t2.is_any() {
            return true;
        }

        match (t1, t2) {
            // Same primitive types
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Null, Type::Null)
            | (Type::Unit, Type::Unit) => true,

            // Type variables
            (Type::TypeVar(id), t) | (t, Type::TypeVar(id)) => self.bind(*id, t.clone(), span),

            // Nullable types
            (Type::Nullable(inner1), Type::Nullable(inner2)) => {
                self.unify_impl(inner1, inner2, span)
            }

            // Null can unify with any nullable type
            (Type::Null, Type::Nullable(_)) | (Type::Nullable(_), Type::Null) => true,

            // List types
            (Type::List(elem1), Type::List(elem2)) => self.unify_impl(elem1, elem2, span),

            // Future types
            (Type::Future(inner1), Type::Future(inner2)) => self.unify_impl(inner1, inner2, span),

            // Map types
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                self.unify_impl(k1, k2, span) && self.unify_impl(v1, v2, span)
            }

            // Tuple types
            (Type::Tuple(elems1), Type::Tuple(elems2)) => {
                if elems1.len() != elems2.len() {
                    self.add_error(TypeError::new(
                        TypeErrorKind::CannotUnify {
                            t1: t1.clone(),
                            t2: t2.clone(),
                        },
                        span,
                    ));
                    return false;
                }
                elems1
                    .iter()
                    .zip(elems2.iter())
                    .all(|(e1, e2)| self.unify_impl(e1, e2, span))
            }

            // Function types
            (
                Type::Function {
                    params: params1,
                    ret: ret1,
                },
                Type::Function {
                    params: params2,
                    ret: ret2,
                },
            ) => {
                if params1.len() != params2.len() {
                    self.add_error(TypeError::new(
                        TypeErrorKind::CannotUnify {
                            t1: t1.clone(),
                            t2: t2.clone(),
                        },
                        span,
                    ));
                    return false;
                }
                let params_ok = params1
                    .iter()
                    .zip(params2.iter())
                    .all(|(p1, p2)| self.unify_impl(p1, p2, span));
                params_ok && self.unify_impl(ret1, ret2, span)
            }

            // Struct types
            (
                Type::Struct {
                    id: id1,
                    type_args: args1,
                    ..
                },
                Type::Struct {
                    id: id2,
                    type_args: args2,
                    ..
                },
            ) => {
                if id1 != id2 {
                    self.add_error(TypeError::new(
                        TypeErrorKind::CannotUnify {
                            t1: t1.clone(),
                            t2: t2.clone(),
                        },
                        span,
                    ));
                    return false;
                }
                if args1.len() != args2.len() {
                    return false;
                }
                args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| self.unify_impl(a1, a2, span))
            }

            // Enum types
            (
                Type::Enum {
                    id: id1,
                    type_args: args1,
                    ..
                },
                Type::Enum {
                    id: id2,
                    type_args: args2,
                    ..
                },
            ) => {
                if id1 != id2 {
                    self.add_error(TypeError::new(
                        TypeErrorKind::CannotUnify {
                            t1: t1.clone(),
                            t2: t2.clone(),
                        },
                        span,
                    ));
                    return false;
                }
                if args1.len() != args2.len() {
                    return false;
                }
                args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| self.unify_impl(a1, a2, span))
            }

            // Types don't match
            _ => {
                self.add_error(TypeError::new(
                    TypeErrorKind::CannotUnify {
                        t1: t1.clone(),
                        t2: t2.clone(),
                    },
                    span,
                ));
                false
            }
        }
    }

    /// Bind a type variable to a type (with occurs check)
    fn bind(&mut self, var: TypeVarId, ty: Type, span: Span) -> bool {
        // Check if already bound
        if let Some(existing) = self.substitution.get(&var).cloned() {
            return self.unify_impl(&existing, &ty, span);
        }

        // Self-reference is fine
        if let Type::TypeVar(id) = &ty {
            if *id == var {
                return true;
            }
        }

        // Occurs check: prevent infinite types
        if self.occurs_in(var, &ty) {
            self.add_error(TypeError::new(
                TypeErrorKind::OccursCheck {
                    var: format!("{var}"),
                    ty: ty.clone(),
                },
                span,
            ));
            return false;
        }

        self.substitution.insert(var, ty);
        true
    }

    /// Check if a type variable occurs in a type (for occurs check)
    fn occurs_in(&self, var: TypeVarId, ty: &Type) -> bool {
        let ty = self.apply(ty);
        match &ty {
            Type::TypeVar(id) => *id == var,
            Type::List(elem) | Type::Nullable(elem) | Type::Future(elem) => {
                self.occurs_in(var, elem)
            }
            Type::Map(k, v) => self.occurs_in(var, k) || self.occurs_in(var, v),
            Type::Tuple(elems) => elems.iter().any(|e| self.occurs_in(var, e)),
            Type::Function { params, ret } => {
                params.iter().any(|p| self.occurs_in(var, p)) || self.occurs_in(var, ret)
            }
            Type::Struct { type_args, .. } | Type::Enum { type_args, .. } => {
                type_args.iter().any(|a| self.occurs_in(var, a))
            }
            _ => false,
        }
    }

    /// Apply the current substitution to a type
    #[must_use]
    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(id) => {
                if let Some(bound) = self.substitution.get(id) {
                    self.apply(bound)
                } else {
                    ty.clone()
                }
            }
            Type::List(elem) => Type::List(Box::new(self.apply(elem))),
            Type::Map(k, v) => Type::Map(Box::new(self.apply(k)), Box::new(self.apply(v))),
            Type::Nullable(inner) => Type::Nullable(Box::new(self.apply(inner))),
            Type::Future(inner) => Type::Future(Box::new(self.apply(inner))),
            Type::Tuple(elems) => Type::Tuple(elems.iter().map(|e| self.apply(e)).collect()),
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(ret)),
            },
            Type::Struct {
                id,
                name,
                type_args,
            } => Type::Struct {
                id: *id,
                name: name.clone(),
                type_args: type_args.iter().map(|a| self.apply(a)).collect(),
            },
            Type::Enum {
                id,
                name,
                type_args,
            } => Type::Enum {
                id: *id,
                name: name.clone(),
                type_args: type_args.iter().map(|a| self.apply(a)).collect(),
            },
            _ => ty.clone(),
        }
    }

    /// Resolve a type fully, replacing all type variables with their bound types
    /// Returns Type::Error if any type variable is unbound
    #[must_use]
    pub fn resolve(&self, ty: &Type) -> Type {
        let applied = self.apply(ty);

        // Check for unresolved type variables
        if applied.has_type_vars() {
            // For now, default unresolved type variables to Error
            // In practice, we might want to default to specific types or report errors
            self.default_type_vars(&applied)
        } else {
            applied
        }
    }

    /// Replace unresolved type variables with default types
    fn default_type_vars(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(_) => Type::Error,
            Type::List(elem) => Type::List(Box::new(self.default_type_vars(elem))),
            Type::Map(k, v) => Type::Map(
                Box::new(self.default_type_vars(k)),
                Box::new(self.default_type_vars(v)),
            ),
            Type::Nullable(inner) => Type::Nullable(Box::new(self.default_type_vars(inner))),
            Type::Future(inner) => Type::Future(Box::new(self.default_type_vars(inner))),
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| self.default_type_vars(e)).collect())
            }
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.default_type_vars(p)).collect(),
                ret: Box::new(self.default_type_vars(ret)),
            },
            Type::Struct {
                id,
                name,
                type_args,
            } => Type::Struct {
                id: *id,
                name: name.clone(),
                type_args: type_args
                    .iter()
                    .map(|a| self.default_type_vars(a))
                    .collect(),
            },
            Type::Enum {
                id,
                name,
                type_args,
            } => Type::Enum {
                id: *id,
                name: name.clone(),
                type_args: type_args
                    .iter()
                    .map(|a| self.default_type_vars(a))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }

    /// Check if two types are compatible (can be unified without modifying state)
    #[must_use]
    pub fn can_unify(&self, t1: &Type, t2: &Type) -> bool {
        let mut copy = self.clone();
        copy.unify(t1, t2, Span::dummy())
    }

    /// Get the current substitution
    #[must_use]
    pub fn substitution(&self) -> &HashMap<TypeVarId, Type> {
        &self.substitution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        TypeVarId::reset_counter();
    }

    #[test]
    fn test_unify_primitives() {
        setup();
        let mut inf = TypeInference::new();

        assert!(inf.unify(&Type::Int, &Type::Int, Span::dummy()));
        assert!(inf.unify(&Type::String, &Type::String, Span::dummy()));
        assert!(!inf.unify(&Type::Int, &Type::String, Span::dummy()));
    }

    #[test]
    fn test_unify_type_var() {
        setup();
        let mut inf = TypeInference::new();

        let var = inf.fresh_var();
        assert!(inf.unify(&var, &Type::Int, Span::dummy()));
        assert_eq!(inf.apply(&var), Type::Int);
    }

    #[test]
    fn test_unify_two_type_vars() {
        setup();
        let mut inf = TypeInference::new();

        let v1 = inf.fresh_var();
        let v2 = inf.fresh_var();

        assert!(inf.unify(&v1, &v2, Span::dummy()));
        assert!(inf.unify(&v2, &Type::Bool, Span::dummy()));

        assert_eq!(inf.apply(&v1), Type::Bool);
        assert_eq!(inf.apply(&v2), Type::Bool);
    }

    #[test]
    fn test_unify_lists() {
        setup();
        let mut inf = TypeInference::new();

        let var = inf.fresh_var();
        let list_var = Type::list(var.clone());
        let list_int = Type::list(Type::Int);

        assert!(inf.unify(&list_var, &list_int, Span::dummy()));
        assert_eq!(inf.apply(&var), Type::Int);
    }

    #[test]
    fn test_unify_functions() {
        setup();
        let mut inf = TypeInference::new();

        let var = inf.fresh_var();
        let func_var = Type::function(vec![Type::Int], var.clone());
        let func_concrete = Type::function(vec![Type::Int], Type::String);

        assert!(inf.unify(&func_var, &func_concrete, Span::dummy()));
        assert_eq!(inf.apply(&var), Type::String);
    }

    #[test]
    fn test_unify_function_arity_mismatch() {
        setup();
        let mut inf = TypeInference::new();

        let f1 = Type::function(vec![Type::Int], Type::Bool);
        let f2 = Type::function(vec![Type::Int, Type::Int], Type::Bool);

        assert!(!inf.unify(&f1, &f2, Span::dummy()));
    }

    #[test]
    fn test_occurs_check() {
        setup();
        let mut inf = TypeInference::new();

        let var = inf.fresh_var();
        let recursive = Type::list(var.clone());

        // This should fail the occurs check
        assert!(!inf.unify(&var, &recursive, Span::dummy()));
        assert!(!inf.errors().is_empty());
    }

    #[test]
    fn test_nullable_unification() {
        setup();
        let mut inf = TypeInference::new();

        // Null unifies with nullable types
        assert!(inf.unify(&Type::Null, &Type::nullable(Type::Int), Span::dummy()));

        // Nullable<Int> unifies with Nullable<Int>
        assert!(inf.unify(
            &Type::nullable(Type::Int),
            &Type::nullable(Type::Int),
            Span::dummy()
        ));

        // Int does not unify with Nullable<Int>
        assert!(!inf.unify(&Type::Int, &Type::nullable(Type::Int), Span::dummy()));
    }

    #[test]
    fn test_never_unifies_with_anything() {
        setup();
        let mut inf = TypeInference::new();

        assert!(inf.unify(&Type::Never, &Type::Int, Span::dummy()));
        assert!(inf.unify(&Type::String, &Type::Never, Span::dummy()));
        assert!(inf.unify(&Type::Never, &Type::Never, Span::dummy()));
    }

    #[test]
    fn test_error_unifies_with_anything() {
        setup();
        let mut inf = TypeInference::new();

        assert!(inf.unify(&Type::Error, &Type::Int, Span::dummy()));
        assert!(inf.unify(&Type::String, &Type::Error, Span::dummy()));
    }

    #[test]
    fn test_can_unify_nondestructive() {
        setup();
        let mut inf = TypeInference::new();

        let var = inf.fresh_var();

        // This shouldn't modify the inference state
        assert!(inf.can_unify(&var, &Type::Int));

        // var should still be unbound
        assert!(inf.apply(&var).is_type_var());
    }
}
