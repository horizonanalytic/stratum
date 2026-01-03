//! Type error definitions for the Stratum type checker

use super::Type;
use crate::lexer::Span;
use std::fmt;

/// A type error with source location
#[derive(Debug, Clone)]
pub struct TypeError {
    /// The kind of error
    pub kind: TypeErrorKind,
    /// Primary source location
    pub span: Span,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
    /// Optional related locations (for showing where types came from)
    pub related: Vec<(Span, String)>,
}

impl TypeError {
    /// Create a new type error
    #[must_use]
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            hint: None,
            related: Vec::new(),
        }
    }

    /// Add a hint to this error
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add a related location with explanation
    #[must_use]
    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related.push((span, message.into()));
        self
    }

    /// Create a type mismatch error
    #[must_use]
    pub fn mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::TypeMismatch { expected, found }, span)
    }

    /// Create an undefined variable error
    #[must_use]
    pub fn undefined_variable(name: impl Into<String>, span: Span) -> Self {
        Self::new(TypeErrorKind::UndefinedVariable(name.into()), span)
    }

    /// Create an undefined type error
    #[must_use]
    pub fn undefined_type(name: impl Into<String>, span: Span) -> Self {
        Self::new(TypeErrorKind::UndefinedType(name.into()), span)
    }

    /// Create a not callable error
    #[must_use]
    pub fn not_callable(ty: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::NotCallable(ty), span)
    }

    /// Create a wrong argument count error
    #[must_use]
    pub fn wrong_arg_count(expected: usize, found: usize, span: Span) -> Self {
        Self::new(TypeErrorKind::WrongArgumentCount { expected, found }, span)
    }

    /// Create a not indexable error
    #[must_use]
    pub fn not_indexable(ty: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::NotIndexable(ty), span)
    }

    /// Create a no such field error
    #[must_use]
    pub fn no_such_field(ty: Type, field: impl Into<String>, span: Span) -> Self {
        Self::new(
            TypeErrorKind::NoSuchField {
                ty,
                field: field.into(),
            },
            span,
        )
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}

impl std::error::Error for TypeError {}

/// The kind of type error
#[derive(Debug, Clone)]
pub enum TypeErrorKind {
    /// Type mismatch: expected one type, found another
    TypeMismatch {
        /// Expected type
        expected: Type,
        /// Actual type found
        found: Type,
    },

    /// Variable not found in scope
    UndefinedVariable(String),

    /// Type name not found
    UndefinedType(String),

    /// Function not found
    UndefinedFunction(String),

    /// Struct not found
    UndefinedStruct(String),

    /// Enum not found
    UndefinedEnum(String),

    /// Attempted to call a non-function
    NotCallable(Type),

    /// Wrong number of arguments in function call
    WrongArgumentCount {
        /// Expected number of arguments
        expected: usize,
        /// Actual number provided
        found: usize,
    },

    /// Attempted to index a non-indexable type
    NotIndexable(Type),

    /// Invalid index type (e.g., using string to index a list)
    InvalidIndexType {
        /// The type being indexed
        container: Type,
        /// The index type used
        index: Type,
    },

    /// Field not found on type
    NoSuchField {
        /// The type being accessed
        ty: Type,
        /// The field name
        field: String,
    },

    /// Attempted to use null-safe operator on non-nullable type
    UnnecessaryNullSafe(Type),

    /// Attempted to use non-nullable value where nullable expected
    NullabilityMismatch {
        /// Expected nullable
        expected_nullable: bool,
        /// Actual type
        found: Type,
    },

    /// Binary operator not supported for types
    InvalidBinaryOp {
        /// The operator
        op: String,
        /// Left operand type
        left: Type,
        /// Right operand type
        right: Type,
    },

    /// Unary operator not supported for type
    InvalidUnaryOp {
        /// The operator
        op: String,
        /// Operand type
        operand: Type,
    },

    /// Return type doesn't match function declaration
    ReturnTypeMismatch {
        /// Expected return type
        expected: Type,
        /// Actual return type
        found: Type,
    },

    /// Cannot assign to this expression
    InvalidAssignmentTarget,

    /// Duplicate field in struct literal
    DuplicateField(String),

    /// Missing field in struct literal
    MissingField {
        /// Struct name
        struct_name: String,
        /// Missing field name
        field: String,
    },

    /// Extra field in struct literal
    ExtraField {
        /// Struct name
        struct_name: String,
        /// Extra field name
        field: String,
    },

    /// Could not infer type
    CannotInfer,

    /// Recursive type without indirection
    RecursiveType(String),

    /// Duplicate definition
    DuplicateDefinition(String),

    /// Break/continue outside loop
    BreakOutsideLoop,
    ContinueOutsideLoop,

    /// Return outside function
    ReturnOutsideFunction,

    /// Branches of if/match have incompatible types
    IncompatibleBranches {
        /// Type of first branch
        first: Type,
        /// Type of other branch
        other: Type,
    },

    /// Generic type argument count mismatch
    WrongTypeArgCount {
        /// Name of generic type
        name: String,
        /// Expected number of type arguments
        expected: usize,
        /// Actual number provided
        found: usize,
    },

    /// Occurs check failure (infinite type)
    OccursCheck {
        /// The type variable
        var: String,
        /// The type it would have to equal
        ty: Type,
    },

    /// Cannot unify two types
    CannotUnify {
        /// First type
        t1: Type,
        /// Second type
        t2: Type,
    },

    /// Interface not found
    UndefinedInterface(String),

    /// Target type of impl not found
    ImplTargetNotFound(String),

    /// Missing method required by interface
    MissingInterfaceMethod {
        /// The interface name
        interface_name: String,
        /// The method name that's missing
        method_name: String,
        /// The type being implemented for
        target_type: String,
    },

    /// Method signature doesn't match interface
    MethodSignatureMismatch {
        /// The interface name
        interface_name: String,
        /// The method name
        method_name: String,
        /// Expected parameter types
        expected_params: Vec<Type>,
        /// Found parameter types
        found_params: Vec<Type>,
        /// Expected return type
        expected_ret: Type,
        /// Found return type
        found_ret: Type,
    },

    /// Duplicate impl for same type and interface
    DuplicateImpl {
        /// The target type
        target_type: String,
        /// The interface (None if inherent impl)
        interface_name: Option<String>,
    },

    /// Method not found on type
    MethodNotFound {
        /// The type
        ty: Type,
        /// The method name
        method_name: String,
    },
}

impl fmt::Display for TypeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeErrorKind::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected `{expected}`, found `{found}`")
            }
            TypeErrorKind::UndefinedVariable(name) => {
                write!(f, "undefined variable `{name}`")
            }
            TypeErrorKind::UndefinedType(name) => {
                write!(f, "undefined type `{name}`")
            }
            TypeErrorKind::UndefinedFunction(name) => {
                write!(f, "undefined function `{name}`")
            }
            TypeErrorKind::UndefinedStruct(name) => {
                write!(f, "undefined struct `{name}`")
            }
            TypeErrorKind::UndefinedEnum(name) => {
                write!(f, "undefined enum `{name}`")
            }
            TypeErrorKind::NotCallable(ty) => {
                write!(f, "type `{ty}` is not callable")
            }
            TypeErrorKind::WrongArgumentCount { expected, found } => {
                write!(
                    f,
                    "wrong number of arguments: expected {expected}, found {found}"
                )
            }
            TypeErrorKind::NotIndexable(ty) => {
                write!(f, "type `{ty}` cannot be indexed")
            }
            TypeErrorKind::InvalidIndexType { container, index } => {
                write!(f, "cannot index `{container}` with `{index}`")
            }
            TypeErrorKind::NoSuchField { ty, field } => {
                write!(f, "type `{ty}` has no field `{field}`")
            }
            TypeErrorKind::UnnecessaryNullSafe(ty) => {
                write!(
                    f,
                    "unnecessary null-safe operator on non-nullable type `{ty}`"
                )
            }
            TypeErrorKind::NullabilityMismatch {
                expected_nullable,
                found,
            } => {
                if *expected_nullable {
                    write!(f, "expected nullable type, found `{found}`")
                } else {
                    write!(f, "expected non-nullable type, found `{found}`")
                }
            }
            TypeErrorKind::InvalidBinaryOp { op, left, right } => {
                write!(f, "cannot apply `{op}` to `{left}` and `{right}`")
            }
            TypeErrorKind::InvalidUnaryOp { op, operand } => {
                write!(f, "cannot apply `{op}` to `{operand}`")
            }
            TypeErrorKind::ReturnTypeMismatch { expected, found } => {
                write!(
                    f,
                    "return type mismatch: expected `{expected}`, found `{found}`"
                )
            }
            TypeErrorKind::InvalidAssignmentTarget => {
                write!(f, "invalid assignment target")
            }
            TypeErrorKind::DuplicateField(name) => {
                write!(f, "duplicate field `{name}`")
            }
            TypeErrorKind::MissingField { struct_name, field } => {
                write!(f, "missing field `{field}` in struct `{struct_name}`")
            }
            TypeErrorKind::ExtraField { struct_name, field } => {
                write!(
                    f,
                    "unknown field `{field}` in struct literal for `{struct_name}`"
                )
            }
            TypeErrorKind::CannotInfer => {
                write!(f, "cannot infer type")
            }
            TypeErrorKind::RecursiveType(name) => {
                write!(f, "recursive type `{name}` has infinite size")
            }
            TypeErrorKind::DuplicateDefinition(name) => {
                write!(f, "duplicate definition of `{name}`")
            }
            TypeErrorKind::BreakOutsideLoop => {
                write!(f, "`break` outside of loop")
            }
            TypeErrorKind::ContinueOutsideLoop => {
                write!(f, "`continue` outside of loop")
            }
            TypeErrorKind::ReturnOutsideFunction => {
                write!(f, "`return` outside of function")
            }
            TypeErrorKind::IncompatibleBranches { first, other } => {
                write!(f, "incompatible types in branches: `{first}` and `{other}`")
            }
            TypeErrorKind::WrongTypeArgCount {
                name,
                expected,
                found,
            } => {
                write!(
                    f,
                    "wrong number of type arguments for `{name}`: expected {expected}, found {found}"
                )
            }
            TypeErrorKind::OccursCheck { var, ty } => {
                write!(f, "infinite type: `{var}` occurs in `{ty}`")
            }
            TypeErrorKind::CannotUnify { t1, t2 } => {
                write!(f, "cannot unify `{t1}` with `{t2}`")
            }
            TypeErrorKind::UndefinedInterface(name) => {
                write!(f, "undefined interface `{name}`")
            }
            TypeErrorKind::ImplTargetNotFound(name) => {
                write!(f, "cannot find type `{name}` for impl")
            }
            TypeErrorKind::MissingInterfaceMethod {
                interface_name,
                method_name,
                target_type,
            } => {
                write!(
                    f,
                    "type `{target_type}` is missing method `{method_name}` required by interface `{interface_name}`"
                )
            }
            TypeErrorKind::MethodSignatureMismatch {
                interface_name,
                method_name,
                expected_params,
                found_params,
                expected_ret,
                found_ret,
            } => {
                let expected_params_str: Vec<_> =
                    expected_params.iter().map(ToString::to_string).collect();
                let found_params_str: Vec<_> =
                    found_params.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "method `{method_name}` has wrong signature for interface `{interface_name}`: \
                     expected ({}) -> {expected_ret}, found ({}) -> {found_ret}",
                    expected_params_str.join(", "),
                    found_params_str.join(", ")
                )
            }
            TypeErrorKind::DuplicateImpl {
                target_type,
                interface_name,
            } => {
                if let Some(iface) = interface_name {
                    write!(
                        f,
                        "duplicate impl of interface `{iface}` for type `{target_type}`"
                    )
                } else {
                    write!(f, "duplicate inherent impl for type `{target_type}`")
                }
            }
            TypeErrorKind::MethodNotFound { ty, method_name } => {
                write!(f, "no method `{method_name}` found for type `{ty}`")
            }
        }
    }
}
