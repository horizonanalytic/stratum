//! Top-level item AST nodes for the Stratum programming language

use crate::lexer::Span;

use super::{Block, Ident, Param, Spanned, TypeAnnotation, TypeParam};

/// A complete source file / module
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The items in this module
    pub items: Vec<Item>,
    /// Source location of the entire module
    pub span: Span,
}

impl Module {
    /// Create a new module
    #[must_use]
    pub fn new(items: Vec<Item>, span: Span) -> Self {
        Self { items, span }
    }
}

/// A top-level item with source location
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The kind of item
    pub kind: ItemKind,
    /// Source location
    pub span: Span,
}

impl Item {
    /// Create a new item
    #[must_use]
    pub fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl Spanned for Item {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of top-level item
#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// Function definition (fx name(...) { ... })
    Function(Function),

    /// Struct definition (struct Name { ... })
    Struct(StructDef),

    /// Enum definition (enum Name { ... })
    Enum(EnumDef),

    /// Interface definition (interface Name { ... })
    Interface(InterfaceDef),

    /// Implementation block (impl Interface for Type { ... })
    Impl(ImplDef),

    /// Import statement
    Import(Import),
}

/// A function definition
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Function name
    pub name: Ident,
    /// Generic type parameters
    pub type_params: Vec<TypeParam>,
    /// Function parameters
    pub params: Vec<Param>,
    /// Return type (None means inferred or unit)
    pub return_type: Option<TypeAnnotation>,
    /// Function body
    pub body: Block,
    /// Whether this is an async function
    pub is_async: bool,
    /// Source location
    pub span: Span,
}

impl Function {
    /// Create a new function
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Ident,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Block,
        is_async: bool,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            span,
        }
    }
}

impl Spanned for Function {
    fn span(&self) -> Span {
        self.span
    }
}

/// A struct definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    /// Struct name
    pub name: Ident,
    /// Generic type parameters
    pub type_params: Vec<TypeParam>,
    /// Struct fields
    pub fields: Vec<StructField>,
    /// Source location
    pub span: Span,
}

impl StructDef {
    /// Create a new struct definition
    #[must_use]
    pub fn new(
        name: Ident,
        type_params: Vec<TypeParam>,
        fields: Vec<StructField>,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_params,
            fields,
            span,
        }
    }
}

impl Spanned for StructDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// A field in a struct definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// Field name
    pub name: Ident,
    /// Field type
    pub ty: TypeAnnotation,
    /// Visibility (public if true)
    pub is_public: bool,
    /// Source location
    pub span: Span,
}

impl StructField {
    /// Create a new struct field
    #[must_use]
    pub fn new(name: Ident, ty: TypeAnnotation, is_public: bool, span: Span) -> Self {
        Self {
            name,
            ty,
            is_public,
            span,
        }
    }
}

impl Spanned for StructField {
    fn span(&self) -> Span {
        self.span
    }
}

/// An enum definition
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    /// Enum name
    pub name: Ident,
    /// Generic type parameters
    pub type_params: Vec<TypeParam>,
    /// Enum variants
    pub variants: Vec<EnumVariant>,
    /// Source location
    pub span: Span,
}

impl EnumDef {
    /// Create a new enum definition
    #[must_use]
    pub fn new(
        name: Ident,
        type_params: Vec<TypeParam>,
        variants: Vec<EnumVariant>,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_params,
            variants,
            span,
        }
    }
}

impl Spanned for EnumDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// A variant in an enum definition
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    /// Variant name
    pub name: Ident,
    /// Associated data type (for variants like Some(T))
    pub data: Option<EnumVariantData>,
    /// Source location
    pub span: Span,
}

impl EnumVariant {
    /// Create a new enum variant
    #[must_use]
    pub fn new(name: Ident, data: Option<EnumVariantData>, span: Span) -> Self {
        Self { name, data, span }
    }

    /// Create a unit variant (no data)
    #[must_use]
    pub fn unit(name: Ident, span: Span) -> Self {
        Self {
            name,
            data: None,
            span,
        }
    }
}

impl Spanned for EnumVariant {
    fn span(&self) -> Span {
        self.span
    }
}

/// Data associated with an enum variant
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantData {
    /// Tuple-style data: Some(T) or Pair(A, B)
    Tuple(Vec<TypeAnnotation>),
    /// Struct-style data: Point { x: Int, y: Int }
    Struct(Vec<StructField>),
}

/// An interface definition
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDef {
    /// Interface name
    pub name: Ident,
    /// Generic type parameters
    pub type_params: Vec<TypeParam>,
    /// Required methods
    pub methods: Vec<InterfaceMethod>,
    /// Source location
    pub span: Span,
}

impl InterfaceDef {
    /// Create a new interface definition
    #[must_use]
    pub fn new(
        name: Ident,
        type_params: Vec<TypeParam>,
        methods: Vec<InterfaceMethod>,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_params,
            methods,
            span,
        }
    }
}

impl Spanned for InterfaceDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// A method signature in an interface
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    /// Method name
    pub name: Ident,
    /// Generic type parameters
    pub type_params: Vec<TypeParam>,
    /// Method parameters
    pub params: Vec<Param>,
    /// Return type
    pub return_type: Option<TypeAnnotation>,
    /// Whether this is an async method
    pub is_async: bool,
    /// Default implementation (if any)
    pub default_body: Option<Block>,
    /// Source location
    pub span: Span,
}

impl InterfaceMethod {
    /// Create a new interface method
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Ident,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        is_async: bool,
        default_body: Option<Block>,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_params,
            params,
            return_type,
            is_async,
            default_body,
            span,
        }
    }
}

impl Spanned for InterfaceMethod {
    fn span(&self) -> Span {
        self.span
    }
}

/// An impl block
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    /// Generic type parameters for the impl
    pub type_params: Vec<TypeParam>,
    /// Interface being implemented (if any)
    pub interface: Option<TypeAnnotation>,
    /// Type being implemented for
    pub target: TypeAnnotation,
    /// Method implementations
    pub methods: Vec<Function>,
    /// Source location
    pub span: Span,
}

impl ImplDef {
    /// Create a new impl block
    #[must_use]
    pub fn new(
        type_params: Vec<TypeParam>,
        interface: Option<TypeAnnotation>,
        target: TypeAnnotation,
        methods: Vec<Function>,
        span: Span,
    ) -> Self {
        Self {
            type_params,
            interface,
            target,
            methods,
            span,
        }
    }
}

impl Spanned for ImplDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// An import statement
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// The import path segments
    pub path: Vec<Ident>,
    /// What to import
    pub kind: ImportKind,
    /// Source location
    pub span: Span,
}

impl Import {
    /// Create a new import
    #[must_use]
    pub fn new(path: Vec<Ident>, kind: ImportKind, span: Span) -> Self {
        Self { path, kind, span }
    }
}

impl Spanned for Import {
    fn span(&self) -> Span {
        self.span
    }
}

/// What kind of import
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    /// Import a single item
    Item,
    /// Import everything (glob import)
    Glob,
    /// Import specific items from a module
    List(Vec<ImportItem>),
    /// Import with an alias
    Alias(Ident),
}

/// An item in an import list
#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// The item name
    pub name: Ident,
    /// Optional alias
    pub alias: Option<Ident>,
    /// Source location
    pub span: Span,
}

impl ImportItem {
    /// Create a new import item
    #[must_use]
    pub fn new(name: Ident, alias: Option<Ident>, span: Span) -> Self {
        Self { name, alias, span }
    }
}

impl Spanned for ImportItem {
    fn span(&self) -> Span {
        self.span
    }
}
