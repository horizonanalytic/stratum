//! Type system for the Stratum programming language
//!
//! This module provides:
//! - Internal type representation (`Type`)
//! - Type environment / symbol table (`TypeEnv`)
//! - Type inference engine (`TypeInference`)
//! - Type checker (`TypeChecker`)

mod checker;
mod env;
mod error;
mod inference;
mod narrowing;

pub use checker::{TypeCheckResult, TypeChecker};
pub use env::TypeEnv;
pub use error::{TypeError, TypeErrorKind};
pub use inference::TypeInference;

use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};

/// Counter for generating unique type variable IDs
static NEXT_TYPE_VAR: AtomicU32 = AtomicU32::new(0);

/// A unique identifier for type variables (used during inference)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVarId(pub u32);

impl TypeVarId {
    /// Generate a fresh type variable ID
    #[must_use]
    pub fn fresh() -> Self {
        Self(NEXT_TYPE_VAR.fetch_add(1, Ordering::Relaxed))
    }

    /// Reset the counter (for testing)
    pub fn reset_counter() {
        NEXT_TYPE_VAR.store(0, Ordering::Relaxed);
    }
}

impl fmt::Display for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as T0, T1, T2, etc.
        write!(f, "T{}", self.0)
    }
}

/// A unique identifier for struct definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructId(pub u32);

impl fmt::Display for StructId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct#{}", self.0)
    }
}

/// A unique identifier for enum definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumId(pub u32);

impl fmt::Display for EnumId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enum#{}", self.0)
    }
}

/// A unique identifier for interface definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterfaceId(pub u32);

impl fmt::Display for InterfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "interface#{}", self.0)
    }
}

/// Internal type representation used by the type checker
///
/// This is distinct from `TypeAnnotation` in the AST, which represents
/// the syntactic form of types as written by the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// 64-bit signed integer
    Int,

    /// 64-bit floating point
    Float,

    /// Boolean
    Bool,

    /// UTF-8 string
    String,

    /// The null value (only valid for nullable types)
    Null,

    /// Homogeneous list type
    List(Box<Type>),

    /// Map from key type to value type
    Map(Box<Type>, Box<Type>),

    /// Nullable type (T?)
    Nullable(Box<Type>),

    /// Function type
    Function {
        /// Parameter types
        params: Vec<Type>,
        /// Return type
        ret: Box<Type>,
    },

    /// Tuple type
    Tuple(Vec<Type>),

    /// A named struct type
    Struct {
        /// Struct definition ID
        id: StructId,
        /// Name for error messages
        name: String,
        /// Type arguments (for generic structs)
        type_args: Vec<Type>,
    },

    /// A named enum type
    Enum {
        /// Enum definition ID
        id: EnumId,
        /// Name for error messages
        name: String,
        /// Type arguments (for generic enums)
        type_args: Vec<Type>,
    },

    /// A type variable (placeholder during inference)
    TypeVar(TypeVarId),

    /// The unit type (void/nothing)
    Unit,

    /// The never type (function never returns, e.g., throws or loops forever)
    Never,

    /// An error type (used to continue type checking after errors)
    Error,

    /// Future type (result of async functions)
    /// Future<T> represents an asynchronous computation that will produce a value of type T
    Future(Box<Type>),

    /// Range type for iterating over integer sequences
    /// Range represents start..end (exclusive end)
    Range,
}

impl Type {
    /// Create a function type
    #[must_use]
    pub fn function(params: Vec<Type>, ret: Type) -> Self {
        Self::Function {
            params,
            ret: Box::new(ret),
        }
    }

    /// Create a list type
    #[must_use]
    pub fn list(element: Type) -> Self {
        Self::List(Box::new(element))
    }

    /// Create a map type
    #[must_use]
    pub fn map(key: Type, value: Type) -> Self {
        Self::Map(Box::new(key), Box::new(value))
    }

    /// Create a nullable type
    #[must_use]
    pub fn nullable(inner: Type) -> Self {
        // Don't double-wrap nullables: T?? -> T?
        if matches!(inner, Type::Nullable(_)) {
            inner
        } else {
            Self::Nullable(Box::new(inner))
        }
    }

    /// Create a future type
    #[must_use]
    pub fn future(inner: Type) -> Self {
        Self::Future(Box::new(inner))
    }

    /// Create a fresh type variable
    #[must_use]
    pub fn fresh_var() -> Self {
        Self::TypeVar(TypeVarId::fresh())
    }

    /// Create a struct type
    #[must_use]
    pub fn struct_type(id: StructId, name: impl Into<String>, type_args: Vec<Type>) -> Self {
        Self::Struct {
            id,
            name: name.into(),
            type_args,
        }
    }

    /// Create an enum type
    #[must_use]
    pub fn enum_type(id: EnumId, name: impl Into<String>, type_args: Vec<Type>) -> Self {
        Self::Enum {
            id,
            name: name.into(),
            type_args,
        }
    }

    /// Returns true if this type is nullable
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        matches!(self, Type::Nullable(_))
    }

    /// Returns true if this is a numeric type (Int or Float)
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    /// Returns true if this is a function type
    #[must_use]
    pub const fn is_function(&self) -> bool {
        matches!(self, Type::Function { .. })
    }

    /// Returns true if this is a type variable
    #[must_use]
    pub const fn is_type_var(&self) -> bool {
        matches!(self, Type::TypeVar(_))
    }

    /// Returns true if this is the error type
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Returns true if this is the never type
    #[must_use]
    pub const fn is_never(&self) -> bool {
        matches!(self, Type::Never)
    }

    /// Returns true if this is a future type
    #[must_use]
    pub const fn is_future(&self) -> bool {
        matches!(self, Type::Future(_))
    }

    /// Get the inner type if this is nullable, otherwise return self
    #[must_use]
    pub fn unwrap_nullable(&self) -> &Type {
        match self {
            Type::Nullable(inner) => inner,
            other => other,
        }
    }

    /// Get the inner type if this is a future, otherwise return self
    #[must_use]
    pub fn unwrap_future(&self) -> &Type {
        match self {
            Type::Future(inner) => inner,
            other => other,
        }
    }

    /// Check if this type contains any type variables
    #[must_use]
    pub fn has_type_vars(&self) -> bool {
        match self {
            Type::TypeVar(_) => true,
            Type::List(t) | Type::Nullable(t) | Type::Future(t) => t.has_type_vars(),
            Type::Map(k, v) => k.has_type_vars() || v.has_type_vars(),
            Type::Tuple(ts) => ts.iter().any(Type::has_type_vars),
            Type::Function { params, ret } => {
                params.iter().any(Type::has_type_vars) || ret.has_type_vars()
            }
            Type::Struct { type_args, .. } | Type::Enum { type_args, .. } => {
                type_args.iter().any(Type::has_type_vars)
            }
            _ => false,
        }
    }

    /// Collect all type variable IDs in this type
    pub fn collect_type_vars(&self, vars: &mut Vec<TypeVarId>) {
        match self {
            Type::TypeVar(id) => {
                if !vars.contains(id) {
                    vars.push(*id);
                }
            }
            Type::List(t) | Type::Nullable(t) | Type::Future(t) => t.collect_type_vars(vars),
            Type::Map(k, v) => {
                k.collect_type_vars(vars);
                v.collect_type_vars(vars);
            }
            Type::Tuple(ts) => {
                for t in ts {
                    t.collect_type_vars(vars);
                }
            }
            Type::Function { params, ret } => {
                for p in params {
                    p.collect_type_vars(vars);
                }
                ret.collect_type_vars(vars);
            }
            Type::Struct { type_args, .. } | Type::Enum { type_args, .. } => {
                for t in type_args {
                    t.collect_type_vars(vars);
                }
            }
            _ => {}
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Null => write!(f, "Null"),
            Type::Unit => write!(f, "()"),
            Type::Never => write!(f, "!"),
            Type::Error => write!(f, "<error>"),
            Type::TypeVar(id) => write!(f, "{id}"),
            Type::List(t) => write!(f, "List<{t}>"),
            Type::Map(k, v) => write!(f, "Map<{k}, {v}>"),
            Type::Nullable(t) => write!(f, "{t}?"),
            Type::Future(t) => write!(f, "Future<{t}>"),
            Type::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{t}")?;
                }
                write!(f, ")")
            }
            Type::Function { params, ret } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::Struct {
                name, type_args, ..
            } => {
                write!(f, "{name}")?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, t) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{t}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Enum {
                name, type_args, ..
            } => {
                write!(f, "{name}")?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, t) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{t}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Range => write!(f, "Range"),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_display() {
        assert_eq!(Type::Int.to_string(), "Int");
        assert_eq!(Type::Float.to_string(), "Float");
        assert_eq!(Type::Bool.to_string(), "Bool");
        assert_eq!(Type::String.to_string(), "String");
        assert_eq!(Type::Unit.to_string(), "()");
        assert_eq!(Type::Never.to_string(), "!");
    }

    #[test]
    fn test_composite_type_display() {
        assert_eq!(Type::list(Type::Int).to_string(), "List<Int>");
        assert_eq!(
            Type::map(Type::String, Type::Int).to_string(),
            "Map<String, Int>"
        );
        assert_eq!(Type::nullable(Type::String).to_string(), "String?");
    }

    #[test]
    fn test_function_type_display() {
        let func = Type::function(vec![Type::Int, Type::String], Type::Bool);
        assert_eq!(func.to_string(), "(Int, String) -> Bool");

        let no_args = Type::function(vec![], Type::Unit);
        assert_eq!(no_args.to_string(), "() -> ()");
    }

    #[test]
    fn test_nullable_no_double_wrap() {
        let nullable = Type::nullable(Type::String);
        let double = Type::nullable(nullable.clone());
        assert_eq!(nullable, double);
    }

    #[test]
    fn test_type_var_fresh() {
        TypeVarId::reset_counter();
        let v1 = TypeVarId::fresh();
        let v2 = TypeVarId::fresh();
        assert_ne!(v1, v2);
        assert_eq!(v1.0 + 1, v2.0);
    }

    #[test]
    fn test_has_type_vars() {
        TypeVarId::reset_counter();
        assert!(!Type::Int.has_type_vars());
        assert!(Type::fresh_var().has_type_vars());
        assert!(Type::list(Type::fresh_var()).has_type_vars());
        assert!(!Type::list(Type::Int).has_type_vars());
    }

    #[test]
    fn test_is_numeric() {
        assert!(Type::Int.is_numeric());
        assert!(Type::Float.is_numeric());
        assert!(!Type::String.is_numeric());
        assert!(!Type::Bool.is_numeric());
    }

    #[test]
    fn test_future_type() {
        let future_int = Type::future(Type::Int);
        assert_eq!(future_int.to_string(), "Future<Int>");
        assert!(future_int.is_future());
        assert!(!Type::Int.is_future());
        assert_eq!(*future_int.unwrap_future(), Type::Int);
    }

    #[test]
    fn test_future_has_type_vars() {
        TypeVarId::reset_counter();
        assert!(!Type::future(Type::Int).has_type_vars());
        assert!(Type::future(Type::fresh_var()).has_type_vars());
    }
}
