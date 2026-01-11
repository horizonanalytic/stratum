//! Top-level item AST nodes for the Stratum programming language

use crate::lexer::Span;

use super::{Block, Expr, Ident, Param, Spanned, Trivia, TypeAnnotation, TypeParam};

/// Execution mode for a function or module
///
/// Controls whether code should be interpreted, compiled, or JIT-compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    /// Interpret the code using the bytecode VM (default)
    #[default]
    Interpret,
    /// Compile the code to native code ahead of time
    Compile,
    /// Compile to native code when the function becomes "hot" (JIT)
    CompileHot,
}

/// CLI override for execution mode
///
/// When set, this overrides all function-level and module-level directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModeOverride {
    /// Force interpret all functions (--interpret-all)
    InterpretAll,
    /// Force compile all functions with JIT (--compile-all)
    CompileAll,
}

/// An attribute on a function or other item
/// Syntax: #[name] or #[name(args)]
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The attribute name
    pub name: Ident,
    /// Optional arguments to the attribute
    pub args: Vec<AttributeArg>,
    /// Source location
    pub span: Span,
}

impl Attribute {
    /// Create a new attribute
    #[must_use]
    pub fn new(name: Ident, args: Vec<AttributeArg>, span: Span) -> Self {
        Self { name, args, span }
    }

    /// Create a simple attribute with no arguments
    #[must_use]
    pub fn simple(name: Ident, span: Span) -> Self {
        Self {
            name,
            args: Vec::new(),
            span,
        }
    }

    /// Check if this is a test attribute
    #[must_use]
    pub fn is_test(&self) -> bool {
        self.name.name == "test"
    }

    /// Check if this test should expect a panic
    #[must_use]
    pub fn should_panic(&self) -> bool {
        self.args.iter().any(|arg| match arg {
            AttributeArg::Ident(ident) => ident.name == "should_panic",
            AttributeArg::NameValue { name, .. } => name.name == "should_panic",
        })
    }

    /// Check if this is an interpret directive
    #[must_use]
    pub fn is_interpret(&self) -> bool {
        self.name.name == "interpret"
    }

    /// Check if this is a compile directive
    #[must_use]
    pub fn is_compile(&self) -> bool {
        self.name.name == "compile"
    }

    /// Check if this compile directive has the "hot" argument
    #[must_use]
    pub fn is_compile_hot(&self) -> bool {
        self.is_compile()
            && self.args.iter().any(|arg| match arg {
                AttributeArg::Ident(ident) => ident.name == "hot",
                _ => false,
            })
    }

    /// Get the execution mode specified by this attribute, if any
    #[must_use]
    pub fn execution_mode(&self) -> Option<ExecutionMode> {
        if self.is_interpret() {
            Some(ExecutionMode::Interpret)
        } else if self.is_compile_hot() {
            Some(ExecutionMode::CompileHot)
        } else if self.is_compile() {
            Some(ExecutionMode::Compile)
        } else {
            None
        }
    }
}

impl Spanned for Attribute {
    fn span(&self) -> Span {
        self.span
    }
}

/// An argument to an attribute
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeArg {
    /// Just an identifier: #[test(should_panic)]
    Ident(Ident),
    /// Name = value pair: #[test(expected = "error message")]
    NameValue { name: Ident, value: Box<Expr> },
}

/// A complete source file / module
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Inner attributes (file-level directives like `#![interpret]`)
    pub inner_attributes: Vec<Attribute>,
    /// The top-level items in this module (functions, structs, lets, statements)
    pub top_level: Vec<TopLevelItem>,
    /// Source location of the entire module
    pub span: Span,
    /// Leading comments at the start of the file
    pub trivia: Trivia,
}

impl Module {
    /// Create a new module from top-level items
    #[must_use]
    pub fn new(inner_attributes: Vec<Attribute>, top_level: Vec<TopLevelItem>, span: Span) -> Self {
        Self {
            inner_attributes,
            top_level,
            span,
            trivia: Trivia::empty(),
        }
    }

    /// Create a new module with trivia
    #[must_use]
    pub fn with_trivia(
        inner_attributes: Vec<Attribute>,
        top_level: Vec<TopLevelItem>,
        span: Span,
        trivia: Trivia,
    ) -> Self {
        Self {
            inner_attributes,
            top_level,
            span,
            trivia,
        }
    }

    /// Create a new module from just items (for backwards compatibility)
    #[must_use]
    pub fn from_items(items: Vec<Item>, span: Span) -> Self {
        let top_level = items.into_iter().map(TopLevelItem::Item).collect();
        Self {
            inner_attributes: Vec::new(),
            top_level,
            span,
            trivia: Trivia::empty(),
        }
    }

    /// Get all items in the module (excluding top-level lets and statements)
    #[must_use]
    pub fn items(&self) -> Vec<&Item> {
        self.top_level
            .iter()
            .filter_map(|tl| match tl {
                TopLevelItem::Item(item) => Some(item),
                _ => None,
            })
            .collect()
    }

    /// Get all top-level let declarations
    #[must_use]
    pub fn top_level_lets(&self) -> Vec<&TopLevelLet> {
        self.top_level
            .iter()
            .filter_map(|tl| match tl {
                TopLevelItem::Let(let_decl) => Some(let_decl),
                _ => None,
            })
            .collect()
    }

    /// Check if this module has any top-level statements (not lets or items)
    #[must_use]
    pub fn has_top_level_statements(&self) -> bool {
        self.top_level
            .iter()
            .any(|tl| matches!(tl, TopLevelItem::Statement(_)))
    }

    /// Get the default execution mode for this module from inner attributes
    ///
    /// Returns `None` if no execution mode directive is specified.
    #[must_use]
    pub fn execution_mode(&self) -> Option<ExecutionMode> {
        self.inner_attributes
            .iter()
            .find_map(Attribute::execution_mode)
    }
}

/// A top-level item in a module (preserves source order)
#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelItem {
    /// A definition (function, struct, enum, etc.)
    Item(Item),
    /// A top-level let binding
    Let(TopLevelLet),
    /// A top-level statement (only allowed in entry files)
    Statement(super::Stmt),
}

impl TopLevelItem {
    /// Get the span of this top-level item
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            TopLevelItem::Item(item) => item.span,
            TopLevelItem::Let(let_decl) => let_decl.span,
            TopLevelItem::Statement(stmt) => stmt.span,
        }
    }
}

/// A top-level let declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TopLevelLet {
    /// Variable name or destructuring pattern
    pub pattern: super::Pattern,
    /// Optional type annotation
    pub ty: Option<super::TypeAnnotation>,
    /// Initial value
    pub value: super::Expr,
    /// Source location
    pub span: Span,
    /// Comments associated with this let
    pub trivia: Trivia,
}

impl TopLevelLet {
    /// Create a new top-level let declaration
    #[must_use]
    pub fn new(
        pattern: super::Pattern,
        ty: Option<super::TypeAnnotation>,
        value: super::Expr,
        span: Span,
    ) -> Self {
        Self {
            pattern,
            ty,
            value,
            span,
            trivia: Trivia::empty(),
        }
    }

    /// Create a new top-level let declaration with trivia
    #[must_use]
    pub fn with_trivia(
        pattern: super::Pattern,
        ty: Option<super::TypeAnnotation>,
        value: super::Expr,
        span: Span,
        trivia: Trivia,
    ) -> Self {
        Self {
            pattern,
            ty,
            value,
            span,
            trivia,
        }
    }
}

impl Spanned for TopLevelLet {
    fn span(&self) -> Span {
        self.span
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
    /// Attributes on this function (e.g., #[test])
    pub attributes: Vec<Attribute>,
    /// Source location
    pub span: Span,
    /// Comments associated with this function
    pub trivia: Trivia,
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
        attributes: Vec<Attribute>,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            attributes,
            span,
            trivia: Trivia::empty(),
        }
    }

    /// Create a new function with trivia
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn with_trivia(
        name: Ident,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Block,
        is_async: bool,
        attributes: Vec<Attribute>,
        span: Span,
        trivia: Trivia,
    ) -> Self {
        Self {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            attributes,
            span,
            trivia,
        }
    }

    /// Check if this function has a #[test] attribute
    #[must_use]
    pub fn is_test(&self) -> bool {
        self.attributes.iter().any(Attribute::is_test)
    }

    /// Check if this test function should expect a panic
    #[must_use]
    pub fn should_panic(&self) -> bool {
        self.attributes
            .iter()
            .filter(|a| a.is_test())
            .any(Attribute::should_panic)
    }

    /// Get the execution mode specified by this function's attributes
    ///
    /// Returns `None` if no execution mode directive is specified on this function.
    #[must_use]
    pub fn execution_mode(&self) -> Option<ExecutionMode> {
        self.attributes.iter().find_map(Attribute::execution_mode)
    }

    /// Resolve the execution mode for this function, considering module defaults
    ///
    /// Function-level directives override module-level defaults.
    /// If neither is specified, returns the provided default.
    #[must_use]
    pub fn resolve_execution_mode(
        &self,
        module_mode: Option<ExecutionMode>,
        default: ExecutionMode,
    ) -> ExecutionMode {
        // Function directive takes precedence
        if let Some(mode) = self.execution_mode() {
            return mode;
        }
        // Then module directive
        if let Some(mode) = module_mode {
            return mode;
        }
        // Finally, use the default
        default
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
    /// Comments associated with this struct
    pub trivia: Trivia,
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
            trivia: Trivia::empty(),
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
    /// Comments associated with this enum
    pub trivia: Trivia,
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
            trivia: Trivia::empty(),
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
    /// Comments associated with this interface
    pub trivia: Trivia,
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
            trivia: Trivia::empty(),
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
    /// Comments associated with this impl
    pub trivia: Trivia,
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
            trivia: Trivia::empty(),
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
