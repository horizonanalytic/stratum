//! Type annotation AST nodes for the Stratum programming language

use crate::lexer::Span;

use super::{Ident, Spanned};

/// A type annotation in source code
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    /// The kind of type
    pub kind: TypeKind,
    /// Source location
    pub span: Span,
}

impl TypeAnnotation {
    /// Create a new type annotation
    #[must_use]
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Create a simple named type (e.g., Int, String)
    #[must_use]
    pub fn simple(name: impl Into<String>, span: Span) -> Self {
        Self::new(
            TypeKind::Named {
                name: Ident::new(name, span),
                args: Vec::new(),
            },
            span,
        )
    }

    /// Create a nullable type (T?)
    #[must_use]
    pub fn nullable(inner: TypeAnnotation, span: Span) -> Self {
        Self::new(TypeKind::Nullable(Box::new(inner)), span)
    }
}

impl Spanned for TypeAnnotation {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of type annotation
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// A named type, possibly with generic arguments (Int, List<T>, Map<K, V>)
    Named {
        /// Type name
        name: Ident,
        /// Generic type arguments (empty if not generic)
        args: Vec<TypeAnnotation>,
    },

    /// A nullable type (T?)
    Nullable(Box<TypeAnnotation>),

    /// A function type ((A, B) -> C)
    Function {
        /// Parameter types
        params: Vec<TypeAnnotation>,
        /// Return type
        ret: Box<TypeAnnotation>,
    },

    /// A tuple type ((A, B, C))
    Tuple(Vec<TypeAnnotation>),

    /// A list type using bracket syntax ([T] as shorthand for List<T>)
    List(Box<TypeAnnotation>),

    /// The unit type (())
    Unit,

    /// The never type (!) - for functions that don't return
    Never,

    /// An inferred type (_) - placeholder for type inference
    Inferred,
}

impl TypeKind {
    /// Returns true if this type is nullable
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        matches!(self, TypeKind::Nullable(_))
    }

    /// Returns true if this is a function type
    #[must_use]
    pub const fn is_function(&self) -> bool {
        matches!(self, TypeKind::Function { .. })
    }
}

/// A generic type parameter declaration (e.g., T, K: Hashable)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    /// The parameter name
    pub name: Ident,
    /// Optional constraint (interface bounds)
    pub bounds: Vec<Ident>,
    /// Source location
    pub span: Span,
}

impl TypeParam {
    /// Create a new unconstrained type parameter
    #[must_use]
    pub fn new(name: Ident, span: Span) -> Self {
        Self {
            name,
            bounds: Vec::new(),
            span,
        }
    }

    /// Create a type parameter with bounds
    #[must_use]
    pub fn with_bounds(name: Ident, bounds: Vec<Ident>, span: Span) -> Self {
        Self { name, bounds, span }
    }
}

impl Spanned for TypeParam {
    fn span(&self) -> Span {
        self.span
    }
}
