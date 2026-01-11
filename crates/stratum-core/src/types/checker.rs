//! Type checker for the Stratum programming language
//!
//! Performs static type analysis on the AST, inferring types where needed
//! and reporting type errors.

use std::collections::HashMap;

use crate::ast::{
    BinOp, Block, CompoundOp, ElseBranch, EnumDef, EnumVariant, EnumVariantData, Expr, ExprKind,
    FieldInit, Function, Ident, ImplDef, InterfaceDef, Item, ItemKind, Literal, Module, Param,
    Pattern, PatternKind, Stmt, StmtKind, StringPart, StructDef, TopLevelItem, TopLevelLet,
    TypeAnnotation, TypeKind, UnaryOp,
};
use crate::lexer::Span;

use super::env::{
    EnumInfo, FieldInfo, ImplInfo, ImplMethodInfo, InterfaceInfo, MethodInfo, StructInfo,
    VariantData, VariantInfo,
};
use super::error::{TypeError, TypeErrorKind};
use super::inference::TypeInference;
use super::narrowing::{extract_narrowing, Narrowing};
use super::{EnumId, StructId, Type, TypeVarId};

/// Type checker for Stratum programs
pub struct TypeChecker {
    /// Type environment (symbol table)
    env: super::TypeEnv,

    /// Type inference engine
    inference: TypeInference,

    /// Collected type errors
    errors: Vec<TypeError>,

    /// Type parameters currently in scope (for generic definitions)
    /// Maps type parameter names (e.g., "T") to their corresponding type variables
    type_params_in_scope: HashMap<String, Type>,

    /// Whether we are currently inside an async function
    in_async_context: bool,
}

/// Result of type checking
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// Collected errors
    pub errors: Vec<TypeError>,

    /// Whether type checking succeeded (no errors)
    pub success: bool,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Create a new type checker
    #[must_use]
    pub fn new() -> Self {
        let mut checker = Self {
            env: super::TypeEnv::new(),
            inference: TypeInference::new(),
            errors: Vec::new(),
            type_params_in_scope: HashMap::new(),
            in_async_context: false,
        };
        checker.register_builtins();
        checker
    }

    /// Register built-in functions and types
    fn register_builtins(&mut self) {
        // Built-in functions that are always available

        // print/println: accept any argument
        // Using a type variable means it accepts any type
        let any_type = self.inference.fresh_var();
        self.env.define_var(
            "print",
            Type::function(vec![any_type.clone()], Type::Unit),
            false,
        );
        let any_type2 = self.inference.fresh_var();
        self.env.define_var(
            "println",
            Type::function(vec![any_type2], Type::Unit),
            false,
        );

        // type_of: returns the type name as a string
        let any_type3 = self.inference.fresh_var();
        self.env.define_var(
            "type_of",
            Type::function(vec![any_type3], Type::String),
            false,
        );

        // assert: accepts a boolean
        self.env.define_var(
            "assert",
            Type::function(vec![Type::Bool], Type::Unit),
            false,
        );

        // assert_eq: accepts two values of the same type
        let assert_type = self.inference.fresh_var();
        self.env.define_var(
            "assert_eq",
            Type::function(vec![assert_type.clone(), assert_type], Type::Unit),
            false,
        );

        // to_string: converts any value to string
        let any_type4 = self.inference.fresh_var();
        self.env.define_var(
            "to_string",
            Type::function(vec![any_type4], Type::String),
            false,
        );

        // parse_int: parses a string to an optional int
        self.env.define_var(
            "parse_int",
            Type::function(vec![Type::String], Type::nullable(Type::Int)),
            false,
        );

        // parse_float: parses a string to an optional float
        self.env.define_var(
            "parse_float",
            Type::function(vec![Type::String], Type::nullable(Type::Float)),
            false,
        );

        // range: creates an exclusive range from start to end
        self.env.define_var(
            "range",
            Type::function(vec![Type::Int, Type::Int], Type::Range),
            false,
        );

        // len: returns the length of strings, lists, and maps
        // Uses Type::Any so it can accept any iterable type
        self.env
            .define_var("len", Type::function(vec![Type::Any], Type::Int), false);

        // str: converts any value to string
        // Uses Type::Any so it can accept Int, Bool, Float, etc.
        self.env
            .define_var("str", Type::function(vec![Type::Any], Type::String), false);

        // int: converts a value to integer
        // Uses Type::Any so it can accept String, Float, Bool, etc.
        self.env
            .define_var("int", Type::function(vec![Type::Any], Type::Int), false);

        // float: converts a value to float
        // Uses Type::Any so it can accept String, Int, Bool, etc.
        self.env
            .define_var("float", Type::function(vec![Type::Any], Type::Float), false);

        // Register native namespace modules
        // These are built-in modules accessed via dot notation (e.g., Random.int(1, 10))
        let namespaces = [
            "File",
            "Dir",
            "Path",
            "Env",
            "Args",
            "Shell",
            "Http",
            "Json",
            "Toml",
            "Yaml",
            "Base64",
            "Url",
            "Gzip",
            "Zip",
            "DateTime",
            "Duration",
            "Time",
            "Regex",
            "Hash",
            "Uuid",
            "Random",
            "Crypto",
            "Math",
            "Input",
            "Log",
            "System",
            "Db",
            "Tcp",
            "Udp",
            "WebSocket",
            "Data",
            "Agg",
            "Join",
            "Cube",
            "Async",
            "Gui",
        ];
        for ns in namespaces {
            self.env
                .define_var(ns, Type::Namespace(ns.to_string()), false);
        }
    }

    /// Type check a complete module
    pub fn check_module(&mut self, module: &Module) -> TypeCheckResult {
        // First pass: collect all type definitions (functions, structs, enums, interfaces)
        // We hoist these so they're available throughout the module
        for tl_item in &module.top_level {
            if let TopLevelItem::Item(item) = tl_item {
                self.register_item(item);
            }
        }

        // Second pass: type check all top-level items in order
        // This ensures top-to-bottom evaluation for lets and statements
        for tl_item in &module.top_level {
            self.check_top_level_item(tl_item);
        }

        // Collect inference errors
        self.errors.extend(self.inference.take_errors());

        TypeCheckResult {
            success: self.errors.is_empty(),
            errors: std::mem::take(&mut self.errors),
        }
    }

    /// Type check a top-level item
    fn check_top_level_item(&mut self, tl_item: &TopLevelItem) {
        match tl_item {
            TopLevelItem::Item(item) => self.check_item(item),
            TopLevelItem::Let(let_decl) => self.check_top_level_let(let_decl),
            TopLevelItem::Statement(stmt) => {
                // Type check the statement (currently in module-level scope)
                self.check_stmt(stmt);
            }
        }
    }

    /// Type check a top-level let declaration
    fn check_top_level_let(&mut self, let_decl: &TopLevelLet) {
        // Get the type from the annotation or infer from value
        let declared_type = let_decl
            .ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t));
        let value_type = self.check_expr(&let_decl.value);

        // If there's a type annotation, ensure the value matches
        if let Some(ref expected) = declared_type {
            if !self
                .inference
                .unify(expected, &value_type, let_decl.value.span)
            {
                self.errors.push(TypeError::new(
                    TypeErrorKind::TypeMismatch {
                        expected: self.inference.apply(expected),
                        found: self.inference.apply(&value_type),
                    },
                    let_decl.value.span,
                ));
            }
        }

        // Bind the pattern to the type
        let ty = declared_type.unwrap_or(value_type);
        self.check_pattern(&let_decl.pattern, &ty);
    }

    /// Register an item's type (first pass)
    fn register_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function(func) => self.register_function(func),
            ItemKind::Struct(s) => {
                self.register_struct(s);
            }
            ItemKind::Enum(e) => {
                self.register_enum(e);
            }
            ItemKind::Interface(i) => {
                self.register_interface(i);
            }
            ItemKind::Impl(_) | ItemKind::Import(_) => {
                // Handled in second pass or separately
            }
        }
    }

    /// Register a function's type signature
    fn register_function(&mut self, func: &Function) {
        let param_types: Vec<Type> = func
            .params
            .iter()
            .map(|p| self.resolve_param_type(&p.ty))
            .collect();

        let ret_type = func
            .return_type
            .as_ref()
            .map_or(Type::Unit, |t| self.resolve_type_annotation(t));

        let func_type = Type::function(param_types, ret_type);
        self.env.define_var(&func.name.name, func_type, false);
    }

    /// Resolve parameter type, using fresh var if none provided
    fn resolve_param_type(&mut self, ty: &Option<TypeAnnotation>) -> Type {
        match ty {
            Some(t) => self.resolve_type_annotation(t),
            None => self.inference.fresh_var(),
        }
    }

    /// Register a struct definition
    fn register_struct(&mut self, s: &StructDef) -> StructId {
        let type_params: Vec<String> = s.type_params.iter().map(|p| p.name.name.clone()).collect();

        // Set up type parameters in scope for resolving field types
        // Also collect the TypeVarIds used for each type parameter
        let mut type_param_vars = Vec::new();
        for param_name in &type_params {
            let type_var = self.inference.fresh_var();
            // Extract the TypeVarId from the Type::TypeVar
            if let Type::TypeVar(id) = &type_var {
                type_param_vars.push(*id);
            }
            self.type_params_in_scope
                .insert(param_name.clone(), type_var);
        }

        let mut fields = HashMap::new();
        let mut field_order = Vec::new();

        for field in &s.fields {
            let field_type = self.resolve_type_annotation(&field.ty);
            fields.insert(
                field.name.name.clone(),
                FieldInfo {
                    ty: field_type,
                    public: field.is_public,
                },
            );
            field_order.push(field.name.name.clone());
        }

        // Clear type parameters from scope
        self.type_params_in_scope.clear();

        let info = StructInfo {
            name: s.name.name.clone(),
            type_params,
            type_param_vars,
            fields,
            field_order,
        };

        self.env.define_struct(info)
    }

    /// Register an enum definition
    fn register_enum(&mut self, e: &EnumDef) -> EnumId {
        let type_params: Vec<String> = e.type_params.iter().map(|p| p.name.name.clone()).collect();

        // Set up type parameters in scope for resolving variant types
        // Also collect the TypeVarIds used for each type parameter
        let mut type_param_vars = Vec::new();
        for param_name in &type_params {
            let type_var = self.inference.fresh_var();
            if let Type::TypeVar(id) = &type_var {
                type_param_vars.push(*id);
            }
            self.type_params_in_scope
                .insert(param_name.clone(), type_var);
        }

        let mut variants = HashMap::new();

        for variant in &e.variants {
            let data = self.resolve_variant_data(variant);
            variants.insert(
                variant.name.name.clone(),
                VariantInfo {
                    name: variant.name.name.clone(),
                    data,
                },
            );
        }

        // Clear type parameters from scope
        self.type_params_in_scope.clear();

        let info = EnumInfo {
            name: e.name.name.clone(),
            type_params,
            type_param_vars,
            variants,
        };

        self.env.define_enum(info)
    }

    /// Resolve enum variant data
    fn resolve_variant_data(&mut self, variant: &EnumVariant) -> Option<VariantData> {
        match &variant.data {
            Some(EnumVariantData::Tuple(types)) => {
                let resolved: Vec<Type> = types
                    .iter()
                    .map(|t| self.resolve_type_annotation(t))
                    .collect();
                Some(VariantData::Tuple(resolved))
            }
            Some(EnumVariantData::Struct(fields)) => {
                let mut field_types = HashMap::new();
                for field in fields {
                    let ty = self.resolve_type_annotation(&field.ty);
                    field_types.insert(field.name.name.clone(), ty);
                }
                Some(VariantData::Struct(field_types))
            }
            None => None,
        }
    }

    /// Register an interface definition
    fn register_interface(&mut self, i: &InterfaceDef) {
        let type_params: Vec<String> = i.type_params.iter().map(|p| p.name.name.clone()).collect();

        let mut methods = HashMap::new();

        for method in &i.methods {
            let param_types: Vec<Type> = method
                .params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map_or(Type::Error, |t| self.resolve_type_annotation(t))
                })
                .collect();

            let ret_type = method
                .return_type
                .as_ref()
                .map_or(Type::Unit, |t| self.resolve_type_annotation(t));

            methods.insert(
                method.name.name.clone(),
                MethodInfo {
                    params: param_types,
                    ret: ret_type,
                    has_default: method.default_body.is_some(),
                },
            );
        }

        let info = InterfaceInfo {
            name: i.name.name.clone(),
            type_params,
            methods,
        };

        self.env.define_interface(info);
    }

    /// Type check an item (second pass)
    fn check_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function(func) => self.check_function(func),
            ItemKind::Struct(_) | ItemKind::Enum(_) | ItemKind::Interface(_) => {
                // Already validated during registration
            }
            ItemKind::Impl(imp) => self.check_impl(imp),
            ItemKind::Import(_) => {}
        }
    }

    /// Type check a function
    fn check_function(&mut self, func: &Function) {
        self.env.enter_scope();

        // Save and set async context
        let was_async = self.in_async_context;
        self.in_async_context = func.is_async;

        let declared_ret_type = func
            .return_type
            .as_ref()
            .map_or(Type::Unit, |t| self.resolve_type_annotation(t));

        // For async functions, the actual return type is Future<T>
        // but the body should produce T (the wrapped type)
        let expected_body_type = declared_ret_type.clone();
        let actual_ret_type = if func.is_async {
            Type::future(declared_ret_type)
        } else {
            declared_ret_type
        };

        self.env.set_return_type(Some(expected_body_type.clone()));

        for param in &func.params {
            let param_type = self.resolve_param_type(&param.ty);
            self.env.define_var(&param.name.name, param_type, false);
        }

        let body_type = self.check_block(&func.body);

        if !self
            .inference
            .unify(&body_type, &expected_body_type, func.body.span)
        {
            self.errors.push(TypeError::new(
                TypeErrorKind::ReturnTypeMismatch {
                    expected: actual_ret_type,
                    found: body_type,
                },
                func.body.span,
            ));
        }

        self.env.set_return_type(None);
        self.in_async_context = was_async;
        self.env.exit_scope();
    }

    /// Check an impl block
    fn check_impl(&mut self, imp: &ImplDef) {
        // 1. Resolve the target type
        let target_type_name = self.extract_type_name(&imp.target);
        let target_type = self.resolve_type_annotation(&imp.target);

        // Verify the target type exists
        if matches!(target_type, Type::Error) {
            self.errors.push(TypeError::new(
                TypeErrorKind::ImplTargetNotFound(target_type_name.clone()),
                imp.target.span,
            ));
            return;
        }

        // 2. Build method info for this impl
        let mut impl_methods = HashMap::new();
        for method in &imp.methods {
            let param_types: Vec<Type> = method
                .params
                .iter()
                .map(|p| self.resolve_param_type(&p.ty))
                .collect();

            let ret_type = method
                .return_type
                .as_ref()
                .map_or(Type::Unit, |t| self.resolve_type_annotation(t));

            impl_methods.insert(
                method.name.name.clone(),
                ImplMethodInfo {
                    params: param_types,
                    ret: ret_type,
                    is_async: method.is_async,
                },
            );
        }

        // 3. If implementing an interface, validate compliance
        let interface_name = if let Some(interface_annotation) = &imp.interface {
            let iface_name = self.extract_type_name(interface_annotation);

            // Look up the interface
            if let Some((_, interface_info)) = self.env.lookup_interface(&iface_name) {
                let interface_info = interface_info.clone(); // Clone to avoid borrow issues

                // Check all required methods are implemented
                for (method_name, method_info) in &interface_info.methods {
                    // Skip methods with default implementations that aren't overridden
                    if method_info.has_default && !impl_methods.contains_key(method_name) {
                        continue;
                    }

                    if let Some(impl_method) = impl_methods.get(method_name) {
                        // Verify signature matches
                        if !self.signatures_match(&method_info.params, &impl_method.params)
                            || !self.types_equal(&method_info.ret, &impl_method.ret)
                        {
                            self.errors.push(TypeError::new(
                                TypeErrorKind::MethodSignatureMismatch {
                                    interface_name: iface_name.clone(),
                                    method_name: method_name.clone(),
                                    expected_params: method_info.params.clone(),
                                    found_params: impl_method.params.clone(),
                                    expected_ret: method_info.ret.clone(),
                                    found_ret: impl_method.ret.clone(),
                                },
                                imp.span,
                            ));
                        }
                    } else if !method_info.has_default {
                        // Required method is missing
                        self.errors.push(TypeError::new(
                            TypeErrorKind::MissingInterfaceMethod {
                                interface_name: iface_name.clone(),
                                method_name: method_name.clone(),
                                target_type: target_type_name.clone(),
                            },
                            imp.span,
                        ));
                    }
                }
                Some(iface_name)
            } else {
                self.errors.push(TypeError::new(
                    TypeErrorKind::UndefinedInterface(iface_name.clone()),
                    interface_annotation.span,
                ));
                return;
            }
        } else {
            None
        };

        // 4. Register the impl
        let impl_info = ImplInfo {
            target_type: target_type_name.clone(),
            interface_name: interface_name.clone(),
            methods: impl_methods,
        };

        if !self.env.register_impl(impl_info) {
            self.errors.push(TypeError::new(
                TypeErrorKind::DuplicateImpl {
                    target_type: target_type_name.clone(),
                    interface_name,
                },
                imp.span,
            ));
        }

        // 5. Type check each method with `self` bound to the target type
        for method in &imp.methods {
            self.check_impl_method(method, &target_type);
        }
    }

    /// Type check a method within an impl block
    fn check_impl_method(&mut self, func: &Function, self_type: &Type) {
        self.env.enter_scope();

        // Bind `self` to the target type
        self.env.define_var("self", self_type.clone(), false);

        let ret_type = func
            .return_type
            .as_ref()
            .map_or(Type::Unit, |t| self.resolve_type_annotation(t));
        self.env.set_return_type(Some(ret_type.clone()));

        for param in &func.params {
            let param_type = self.resolve_param_type(&param.ty);
            self.env.define_var(&param.name.name, param_type, false);
        }

        let body_type = self.check_block(&func.body);

        if !self.inference.unify(&body_type, &ret_type, func.body.span) {
            self.errors.push(TypeError::new(
                TypeErrorKind::ReturnTypeMismatch {
                    expected: ret_type,
                    found: body_type,
                },
                func.body.span,
            ));
        }

        self.env.set_return_type(None);
        self.env.exit_scope();
    }

    /// Extract the type name from a type annotation
    fn extract_type_name(&self, annotation: &TypeAnnotation) -> String {
        match &annotation.kind {
            TypeKind::Named { name, .. } => name.name.clone(),
            TypeKind::Nullable(inner) => self.extract_type_name(inner),
            _ => String::from("<unknown>"),
        }
    }

    /// Check if two parameter lists match
    fn signatures_match(&self, expected: &[Type], found: &[Type]) -> bool {
        if expected.len() != found.len() {
            return false;
        }
        expected
            .iter()
            .zip(found.iter())
            .all(|(e, f)| self.types_equal(e, f))
    }

    /// Check if two types are equal (structural equality)
    fn types_equal(&self, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Null, Type::Null)
            | (Type::Unit, Type::Unit)
            | (Type::Never, Type::Never) => true,
            (Type::List(a), Type::List(b)) => self.types_equal(a, b),
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                self.types_equal(k1, k2) && self.types_equal(v1, v2)
            }
            (Type::Nullable(a), Type::Nullable(b)) => self.types_equal(a, b),
            (Type::Tuple(a), Type::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| self.types_equal(x, y))
            }
            (
                Type::Function {
                    params: p1,
                    ret: r1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                },
            ) => self.signatures_match(p1, p2) && self.types_equal(r1, r2),
            (
                Type::Struct {
                    id: id1, name: n1, ..
                },
                Type::Struct {
                    id: id2, name: n2, ..
                },
            ) => id1 == id2 || n1 == n2,
            (
                Type::Enum {
                    id: id1, name: n1, ..
                },
                Type::Enum {
                    id: id2, name: n2, ..
                },
            ) => id1 == id2 || n1 == n2,
            _ => false,
        }
    }

    /// Type check a block
    fn check_block(&mut self, block: &Block) -> Type {
        self.env.enter_scope();

        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }

        let result = block
            .expr
            .as_ref()
            .map_or(Type::Unit, |e| self.check_expr(e));

        self.env.exit_scope();
        result
    }

    /// Check a block with type narrowing applied
    fn check_block_with_narrowing(
        &mut self,
        block: &Block,
        narrowings: &HashMap<String, Narrowing>,
    ) -> Type {
        self.env.enter_scope();

        // Apply narrowings by shadowing variables with narrowed types
        self.apply_narrowing(narrowings);

        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }

        let result = block
            .expr
            .as_ref()
            .map_or(Type::Unit, |e| self.check_expr(e));

        self.env.exit_scope();
        result
    }

    /// Apply type narrowing by shadowing variables with their narrowed types
    fn apply_narrowing(&mut self, narrowings: &HashMap<String, Narrowing>) {
        for (name, narrowing) in narrowings {
            if let Some(info) = self.env.lookup_var(name) {
                let current_type = self.inference.apply(&info.ty);
                let mutable = info.mutable;

                match narrowing {
                    Narrowing::UnwrapNullable => {
                        // Only narrow if the type is actually nullable
                        if let Type::Nullable(inner) = &current_type {
                            self.env.define_var(name, (**inner).clone(), mutable);
                        }
                    }
                }
            }
        }
    }

    /// Type check a statement
    fn check_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, ty, value } => {
                let value_type = self.check_expr(value);

                let declared_type = ty.as_ref().map(|t| self.resolve_type_annotation(t));

                let final_type = if let Some(declared) = declared_type {
                    if !self.inference.unify(&value_type, &declared, stmt.span) {
                        self.errors.push(TypeError::mismatch(
                            declared.clone(),
                            value_type.clone(),
                            stmt.span,
                        ));
                    }
                    declared
                } else {
                    value_type
                };

                self.bind_pattern(pattern, &final_type);
            }

            StmtKind::Expr(expr) => {
                self.check_expr(expr);
            }

            StmtKind::Assign { target, value } => {
                let target_type = self.check_expr(target);
                let value_type = self.check_expr(value);

                if !self.inference.unify(&target_type, &value_type, stmt.span) {
                    self.errors
                        .push(TypeError::mismatch(target_type, value_type, stmt.span));
                }
            }

            StmtKind::CompoundAssign { target, op, value } => {
                let target_type = self.check_expr(target);
                let value_type = self.check_expr(value);
                let result_type = self.check_compound_op(*op, &target_type, &value_type, stmt.span);

                if !self.inference.unify(&target_type, &result_type, stmt.span) {
                    self.errors
                        .push(TypeError::mismatch(target_type, result_type, stmt.span));
                }
            }

            StmtKind::Return(expr) => {
                let return_type = expr.as_ref().map_or(Type::Unit, |e| self.check_expr(e));

                if let Some(expected) = self.env.get_return_type().cloned() {
                    if !self.inference.unify(&return_type, &expected, stmt.span) {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::ReturnTypeMismatch {
                                expected,
                                found: return_type,
                            },
                            stmt.span,
                        ));
                    }
                } else {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::ReturnOutsideFunction,
                        stmt.span,
                    ));
                }
            }

            StmtKind::For {
                pattern,
                iter,
                body,
            } => {
                let iter_type = self.check_expr(iter);
                let elem_type = self.get_iterator_element_type(&iter_type, stmt.span);

                self.env.enter_scope();
                self.env.enter_loop();
                self.bind_pattern(pattern, &elem_type);
                self.check_block(body);
                self.env.exit_loop();
                self.env.exit_scope();
            }

            StmtKind::While { cond, body } => {
                let cond_type = self.check_expr(cond);

                if !self.inference.unify(&cond_type, &Type::Bool, stmt.span) {
                    self.errors
                        .push(TypeError::mismatch(Type::Bool, cond_type, stmt.span));
                }

                self.env.enter_loop();
                self.check_block(body);
                self.env.exit_loop();
            }

            StmtKind::Loop { body } => {
                self.env.enter_loop();
                self.check_block(body);
                self.env.exit_loop();
            }

            StmtKind::Break => {
                if !self.env.in_loop() {
                    self.errors
                        .push(TypeError::new(TypeErrorKind::BreakOutsideLoop, stmt.span));
                }
            }

            StmtKind::Continue => {
                if !self.env.in_loop() {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::ContinueOutsideLoop,
                        stmt.span,
                    ));
                }
            }

            StmtKind::TryCatch {
                try_block,
                catches,
                finally,
            } => {
                self.check_block(try_block);

                for catch in catches {
                    self.env.enter_scope();
                    if let Some(binding) = &catch.binding {
                        self.env.define_var(&binding.name, Type::Error, false);
                    }
                    self.check_block(&catch.body);
                    self.env.exit_scope();
                }

                if let Some(finally_block) = finally {
                    self.check_block(finally_block);
                }
            }

            StmtKind::Throw(expr) => {
                self.check_expr(expr);
            }
        }
    }

    /// Type check an expression
    fn check_expr(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Literal(lit) => self.check_literal(lit),

            ExprKind::Ident(name) => {
                if let Some(info) = self.env.lookup_var(&name.name) {
                    info.ty.clone()
                } else {
                    self.errors
                        .push(TypeError::undefined_variable(&name.name, expr.span));
                    Type::Error
                }
            }

            ExprKind::Binary { left, op, right } => {
                // Pipeline operator needs special handling for Call expressions on RHS
                if *op == BinOp::Pipe {
                    return self.check_pipe_expr(left, right, expr.span);
                }
                let left_type = self.check_expr(left);
                let right_type = self.check_expr(right);
                self.check_binary_op(*op, &left_type, &right_type, expr.span)
            }

            ExprKind::Unary { op, expr: operand } => {
                let operand_type = self.check_expr(operand);
                self.check_unary_op(*op, &operand_type, expr.span)
            }

            ExprKind::Paren(inner) => self.check_expr(inner),

            ExprKind::Call {
                callee,
                args,
                trailing_closure,
            } => {
                let callee_type = self.check_expr(callee);
                let arg_types: Vec<Type> =
                    args.iter().map(|a| self.check_expr(a.value())).collect();
                // If there's a trailing closure, check its type too
                if let Some(closure) = trailing_closure {
                    let _closure_type = self.check_expr(closure);
                    // For now, trailing closures are not checked against function signature
                }
                self.check_call(&callee_type, &arg_types, expr.span)
            }

            ExprKind::Index {
                expr: container,
                index,
            } => {
                let container_type = self.check_expr(container);
                let index_type = self.check_expr(index);
                self.check_index(&container_type, &index_type, expr.span)
            }

            ExprKind::Field { expr: obj, field } => {
                let obj_type = self.check_expr(obj);
                self.check_field_access(&obj_type, &field.name, expr.span)
            }

            ExprKind::NullSafeField { expr: obj, field } => {
                let obj_type = self.check_expr(obj);
                self.check_null_safe_field(&obj_type, &field.name, expr.span)
            }

            ExprKind::NullSafeIndex {
                expr: container,
                index,
            } => {
                let container_type = self.check_expr(container);
                let index_type = self.check_expr(index);
                self.check_null_safe_index(&container_type, &index_type, expr.span)
            }

            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.check_expr(cond);

                if !self.inference.unify(&cond_type, &Type::Bool, expr.span) {
                    self.errors
                        .push(TypeError::mismatch(Type::Bool, cond_type, expr.span));
                }

                // Extract narrowing info from condition
                let narrowing = extract_narrowing(cond);

                // Check then-branch with narrowed types
                let then_type =
                    self.check_block_with_narrowing(then_branch, &narrowing.then_narrowings);

                if let Some(else_br) = else_branch {
                    let else_type = match else_br {
                        ElseBranch::Block(block) => {
                            self.check_block_with_narrowing(block, &narrowing.else_narrowings)
                        }
                        ElseBranch::ElseIf(if_expr) => {
                            // For else-if, apply narrowing in a new scope
                            self.env.enter_scope();
                            self.apply_narrowing(&narrowing.else_narrowings);
                            let ty = self.check_expr(if_expr);
                            self.env.exit_scope();
                            ty
                        }
                    };

                    if !self.inference.unify(&then_type, &else_type, expr.span) {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::IncompatibleBranches {
                                first: then_type.clone(),
                                other: else_type,
                            },
                            expr.span,
                        ));
                    }

                    then_type
                } else {
                    Type::Unit
                }
            }

            ExprKind::Match {
                expr: scrutinee,
                arms,
            } => {
                let scrutinee_type = self.check_expr(scrutinee);
                let mut result_type: Option<Type> = None;

                for arm in arms {
                    self.env.enter_scope();
                    self.check_pattern(&arm.pattern, &scrutinee_type);

                    if let Some(guard) = &arm.guard {
                        let guard_type = self.check_expr(guard);
                        if !self.inference.unify(&guard_type, &Type::Bool, guard.span) {
                            self.errors.push(TypeError::mismatch(
                                Type::Bool,
                                guard_type,
                                guard.span,
                            ));
                        }
                    }

                    let arm_type = self.check_expr(&arm.body);

                    if let Some(ref prev_type) = result_type {
                        if !self.inference.unify(&arm_type, prev_type, arm.body.span) {
                            self.errors.push(TypeError::new(
                                TypeErrorKind::IncompatibleBranches {
                                    first: prev_type.clone(),
                                    other: arm_type.clone(),
                                },
                                arm.body.span,
                            ));
                        }
                    } else {
                        result_type = Some(arm_type);
                    }

                    self.env.exit_scope();
                }

                result_type.unwrap_or(Type::Never)
            }

            ExprKind::Lambda {
                params,
                body,
                return_type,
            } => self.check_lambda(params, body, return_type.as_ref(), expr.span),

            ExprKind::Block(block) => self.check_block(block),

            ExprKind::List(elements) => {
                if elements.is_empty() {
                    Type::list(self.inference.fresh_var())
                } else {
                    let elem_type = self.check_expr(&elements[0]);

                    for elem in &elements[1..] {
                        let t = self.check_expr(elem);
                        if !self.inference.unify(&elem_type, &t, elem.span) {
                            self.errors
                                .push(TypeError::mismatch(elem_type.clone(), t, elem.span));
                        }
                    }

                    Type::list(elem_type)
                }
            }

            ExprKind::Map(entries) => {
                if entries.is_empty() {
                    Type::map(self.inference.fresh_var(), self.inference.fresh_var())
                } else {
                    let (first_key, first_value) = &entries[0];
                    let key_type = self.check_expr(first_key);
                    let value_type = self.check_expr(first_value);

                    for (k, v) in &entries[1..] {
                        let kt = self.check_expr(k);
                        let vt = self.check_expr(v);

                        if !self.inference.unify(&key_type, &kt, k.span) {
                            self.errors
                                .push(TypeError::mismatch(key_type.clone(), kt, k.span));
                        }
                        if !self.inference.unify(&value_type, &vt, v.span) {
                            self.errors
                                .push(TypeError::mismatch(value_type.clone(), vt, v.span));
                        }
                    }

                    Type::map(key_type, value_type)
                }
            }

            ExprKind::StringInterp { parts } => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.check_expr(e);
                    }
                }
                Type::String
            }

            ExprKind::StructInit { name, fields } => {
                self.check_struct_init(name, fields, expr.span)
            }

            ExprKind::EnumVariant {
                enum_name,
                variant,
                data,
            } => self.check_enum_variant(enum_name.as_ref(), variant, data.as_deref(), expr.span),

            ExprKind::Await(inner) => {
                // Validate we're in an async context
                if !self.in_async_context {
                    self.errors
                        .push(TypeError::new(TypeErrorKind::AwaitOutsideAsync, expr.span));
                    return Type::Error;
                }

                // Check the inner expression
                let inner_type = self.check_expr(inner);

                // The inner type must be a Future<T>
                match &inner_type {
                    Type::Future(inner_ty) => (**inner_ty).clone(),
                    Type::Error => Type::Error,
                    Type::TypeVar(_) => {
                        // Create a fresh type variable for the result and constrain
                        // the inner type to be Future<result>
                        let result_type = Type::fresh_var();
                        let expected_future = Type::future(result_type.clone());
                        if self
                            .inference
                            .unify(&inner_type, &expected_future, expr.span)
                        {
                            result_type
                        } else {
                            self.errors.push(TypeError::new(
                                TypeErrorKind::AwaitNonFuture(inner_type),
                                expr.span,
                            ));
                            Type::Error
                        }
                    }
                    _ => {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::AwaitNonFuture(inner_type),
                            expr.span,
                        ));
                        Type::Error
                    }
                }
            }

            ExprKind::Try(inner) => self.check_expr(inner),

            ExprKind::Placeholder => {
                // Placeholder (_) should only appear inside pipeline expressions
                // If we reach here, it means _ was used outside of a |> context
                self.errors.push(TypeError::new(
                    TypeErrorKind::PlaceholderOutsidePipeline,
                    expr.span,
                ));
                Type::Error
            }

            ExprKind::ColumnShorthand(_) => {
                // Column shorthand (.column_name) represents a dynamic column access
                // Its type depends on the row context, so we use a fresh type variable
                // The bytecode compiler will wrap expressions containing this in a lambda
                Type::fresh_var()
            }

            ExprKind::StateBinding(inner) => {
                // State binding (&state.field) creates a reactive binding
                // The type is a StateBinding<T> where T is the inner expression's type
                // For now, we just return the inner type - proper GUI type checking can come later
                self.check_expr(inner)
            }
        }
    }

    /// Check a pipeline expression (a |> f or a |> f(b))
    fn check_pipe_expr(&mut self, left: &Expr, right: &Expr, span: Span) -> Type {
        let left_type = self.check_expr(left);

        match &right.kind {
            ExprKind::Call { callee, args, .. } => {
                // Check the callee type
                let callee_type = self.check_expr(callee);

                // Check if any argument is a placeholder
                let has_placeholder = args
                    .iter()
                    .any(|arg| matches!(arg.value().kind, ExprKind::Placeholder));

                // Build the argument types
                let arg_types: Vec<Type> = if has_placeholder {
                    // Placeholder mode: replace placeholders with left type
                    args.iter()
                        .map(|arg| {
                            if matches!(arg.value().kind, ExprKind::Placeholder) {
                                left_type.clone()
                            } else {
                                self.check_expr(arg.value())
                            }
                        })
                        .collect()
                } else {
                    // No placeholder: prepend left type to args
                    std::iter::once(left_type.clone())
                        .chain(args.iter().map(|arg| self.check_expr(arg.value())))
                        .collect()
                };

                // Check the call with the constructed argument types
                self.check_call(&callee_type, &arg_types, span)
            }
            _ => {
                // Bare function reference: a |> f -> f(a)
                let right_type = self.check_expr(right);
                self.check_call(&right_type, &[left_type], span)
            }
        }
    }

    /// Check a literal expression
    fn check_literal(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Int(_) => Type::Int,
            Literal::Float(_) => Type::Float,
            Literal::String(_) => Type::String,
            Literal::Bool(_) => Type::Bool,
            Literal::Null => Type::Null,
        }
    }

    /// Check a binary operation
    fn check_binary_op(&mut self, op: BinOp, left: &Type, right: &Type, span: Span) -> Type {
        let left = self.inference.apply(left);
        let right = self.inference.apply(right);

        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // Try to unify with numeric types for type inference
                let left_numeric = left.is_numeric()
                    || left.is_type_var() && self.inference.can_unify(&left, &Type::Int);
                let right_numeric = right.is_numeric()
                    || right.is_type_var() && self.inference.can_unify(&right, &Type::Int);

                if left_numeric && right_numeric {
                    // If either is a type variable, try to infer from the other side
                    if left.is_type_var() && right.is_numeric() {
                        self.inference.unify(&left, &right, span);
                    } else if right.is_type_var() && left.is_numeric() {
                        self.inference.unify(&right, &left, span);
                    } else if left.is_type_var() && right.is_type_var() {
                        // Both are type vars, unify them and assume Int
                        self.inference.unify(&left, &right, span);
                        self.inference.unify(&left, &Type::Int, span);
                    }

                    let left = self.inference.apply(&left);
                    let right = self.inference.apply(&right);

                    if matches!(left, Type::Float) || matches!(right, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else if op == BinOp::Add
                    && matches!(left, Type::String)
                    && matches!(right, Type::String)
                {
                    Type::String
                } else if op == BinOp::Add && (left.is_type_var() || right.is_type_var()) {
                    // For Add with type var, try string if the other side is string
                    if matches!(left, Type::String) && right.is_type_var() {
                        self.inference.unify(&right, &Type::String, span);
                        Type::String
                    } else if matches!(right, Type::String) && left.is_type_var() {
                        self.inference.unify(&left, &Type::String, span);
                        Type::String
                    } else {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::InvalidBinaryOp {
                                op: op.as_str().to_string(),
                                left: left.clone(),
                                right: right.clone(),
                            },
                            span,
                        ));
                        Type::Error
                    }
                } else {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidBinaryOp {
                            op: op.as_str().to_string(),
                            left: left.clone(),
                            right: right.clone(),
                        },
                        span,
                    ));
                    Type::Error
                }
            }

            BinOp::Eq | BinOp::Ne => {
                if !self.inference.unify(&left, &right, span) {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidBinaryOp {
                            op: op.as_str().to_string(),
                            left: left.clone(),
                            right: right.clone(),
                        },
                        span,
                    ));
                }
                Type::Bool
            }

            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                // Handle type variables by trying to unify with the other operand
                let left_comparable =
                    left.is_numeric() || matches!(left, Type::String) || left.is_type_var();
                let right_comparable =
                    right.is_numeric() || matches!(right, Type::String) || right.is_type_var();

                if left_comparable && right_comparable {
                    // Try to unify type variables with the other side
                    if left.is_type_var() && right.is_numeric() {
                        self.inference.unify(&left, &right, span);
                    } else if right.is_type_var() && left.is_numeric() {
                        self.inference.unify(&right, &left, span);
                    } else if left.is_type_var() && matches!(right, Type::String) {
                        self.inference.unify(&left, &Type::String, span);
                    } else if right.is_type_var() && matches!(left, Type::String) {
                        self.inference.unify(&right, &Type::String, span);
                    } else if left.is_type_var() && right.is_type_var() {
                        // Both are type vars, unify them and assume Int
                        self.inference.unify(&left, &right, span);
                        self.inference.unify(&left, &Type::Int, span);
                    }
                    Type::Bool
                } else {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidBinaryOp {
                            op: op.as_str().to_string(),
                            left: left.clone(),
                            right: right.clone(),
                        },
                        span,
                    ));
                    Type::Error
                }
            }

            BinOp::And | BinOp::Or => {
                if !self.inference.unify(&left, &Type::Bool, span) {
                    self.errors
                        .push(TypeError::mismatch(Type::Bool, left.clone(), span));
                }
                if !self.inference.unify(&right, &Type::Bool, span) {
                    self.errors
                        .push(TypeError::mismatch(Type::Bool, right.clone(), span));
                }
                Type::Bool
            }

            BinOp::Pipe => {
                if let Type::Function { params, ret } = &right {
                    if params.is_empty() {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::WrongArgumentCount {
                                expected: 1,
                                found: 0,
                            },
                            span,
                        ));
                        Type::Error
                    } else {
                        if !self.inference.unify(&left, &params[0], span) {
                            self.errors.push(TypeError::mismatch(
                                params[0].clone(),
                                left.clone(),
                                span,
                            ));
                        }
                        *ret.clone()
                    }
                } else {
                    self.errors
                        .push(TypeError::not_callable(right.clone(), span));
                    Type::Error
                }
            }

            BinOp::NullCoalesce => match &left {
                Type::Nullable(inner) => {
                    if !self.inference.unify(inner, &right, span) {
                        self.errors
                            .push(TypeError::mismatch(*inner.clone(), right.clone(), span));
                    }
                    right
                }
                Type::Error => Type::Error,
                _ => {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::UnnecessaryNullSafe(left.clone()),
                        span,
                    ));
                    right
                }
            },

            BinOp::Range | BinOp::RangeInclusive => Type::list(Type::Int),
        }
    }

    /// Check a unary operation
    fn check_unary_op(&mut self, op: UnaryOp, operand: &Type, span: Span) -> Type {
        let operand = self.inference.apply(operand);

        match op {
            UnaryOp::Neg => {
                if operand.is_numeric() {
                    operand
                } else {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidUnaryOp {
                            op: "-".to_string(),
                            operand: operand.clone(),
                        },
                        span,
                    ));
                    Type::Error
                }
            }
            UnaryOp::Not => {
                if matches!(operand, Type::Bool) {
                    Type::Bool
                } else {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidUnaryOp {
                            op: "!".to_string(),
                            operand: operand.clone(),
                        },
                        span,
                    ));
                    Type::Error
                }
            }
        }
    }

    /// Check a compound assignment operation
    fn check_compound_op(
        &mut self,
        op: CompoundOp,
        target: &Type,
        value: &Type,
        span: Span,
    ) -> Type {
        let bin_op = match op {
            CompoundOp::Add => BinOp::Add,
            CompoundOp::Sub => BinOp::Sub,
            CompoundOp::Mul => BinOp::Mul,
            CompoundOp::Div => BinOp::Div,
            CompoundOp::Mod => BinOp::Mod,
        };
        self.check_binary_op(bin_op, target, value, span)
    }

    /// Check a function call
    fn check_call(&mut self, callee: &Type, args: &[Type], span: Span) -> Type {
        let callee = self.inference.apply(callee);

        match &callee {
            Type::Function { params, ret } => {
                if params.len() != args.len() {
                    self.errors
                        .push(TypeError::wrong_arg_count(params.len(), args.len(), span));
                    return Type::Error;
                }

                for (param, arg) in params.iter().zip(args.iter()) {
                    if !self.inference.unify(param, arg, span) {
                        self.errors
                            .push(TypeError::mismatch(param.clone(), arg.clone(), span));
                    }
                }

                *ret.clone()
            }
            Type::Error => Type::Error,
            // Type variables might be functions - unify with expected function type
            Type::TypeVar(_) => {
                // Create a function type with the actual argument types and fresh return
                let ret = self.inference.fresh_var();
                let expected_fn = Type::function(args.to_vec(), ret.clone());
                // Try to unify the type variable with a function type
                if !self.inference.unify(&callee, &expected_fn, span) {
                    self.errors
                        .push(TypeError::not_callable(callee.clone(), span));
                    return Type::Error;
                }
                ret
            }
            _ => {
                self.errors
                    .push(TypeError::not_callable(callee.clone(), span));
                Type::Error
            }
        }
    }

    /// Check index access
    fn check_index(&mut self, container: &Type, index: &Type, span: Span) -> Type {
        let container = self.inference.apply(container);
        let index = self.inference.apply(index);

        match &container {
            Type::List(elem) => {
                if !self.inference.unify(&index, &Type::Int, span) {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidIndexType {
                            container: container.clone(),
                            index: index.clone(),
                        },
                        span,
                    ));
                }
                *elem.clone()
            }
            Type::Map(key, value) => {
                if !self.inference.unify(&index, key, span) {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidIndexType {
                            container: container.clone(),
                            index: index.clone(),
                        },
                        span,
                    ));
                }
                *value.clone()
            }
            Type::String => {
                if !self.inference.unify(&index, &Type::Int, span) {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::InvalidIndexType {
                            container: container.clone(),
                            index: index.clone(),
                        },
                        span,
                    ));
                }
                Type::String
            }
            Type::Error => Type::Error,
            // Type variables (from dynamic sources like Json.parse) - constrain to list
            Type::TypeVar(_) => {
                let elem_type = self.inference.fresh_var();
                let list_type = Type::list(elem_type.clone());
                // Try to unify with list type - if it fails, allow anyway for dynamic typing
                self.inference.unify(&container, &list_type, span);
                elem_type
            }
            _ => {
                self.errors
                    .push(TypeError::not_indexable(container.clone(), span));
                Type::Error
            }
        }
    }

    /// Check field access
    fn check_field_access(&mut self, obj: &Type, field: &str, span: Span) -> Type {
        let obj = self.inference.apply(obj);

        match &obj {
            Type::Struct { id, name: _, .. } => {
                // Clone the field type to avoid borrow issues
                let field_type = self
                    .env
                    .get_struct(*id)
                    .and_then(|info| info.fields.get(field))
                    .map(|f| f.ty.clone());

                if let Some(ty) = field_type {
                    ty
                } else {
                    self.errors
                        .push(TypeError::no_such_field(obj.clone(), field, span));
                    Type::Error
                }
            }
            Type::Tuple(elems) => {
                if let Ok(index) = field.parse::<usize>() {
                    if index < elems.len() {
                        elems[index].clone()
                    } else {
                        self.errors
                            .push(TypeError::no_such_field(obj.clone(), field, span));
                        Type::Error
                    }
                } else {
                    self.errors
                        .push(TypeError::no_such_field(obj.clone(), field, span));
                    Type::Error
                }
            }
            // Built-in String methods
            Type::String => self.check_string_method(field, span),
            // Built-in List methods
            Type::List(elem_type) => self.check_list_method(field, elem_type, span),
            // Built-in Map methods
            Type::Map(key_type, value_type) => {
                self.check_map_method(field, key_type, value_type, span)
            }
            // Native namespace modules (Random, Math, File, etc.)
            // Methods on namespaces are dynamically typed - the VM handles actual dispatch.
            // Return a fresh type variable that will unify with a function type when called.
            Type::Namespace(_) => self.inference.fresh_var(),
            // Type variables can have methods called on them - return fresh type var
            // This enables chaining from dynamically-typed namespace method results
            Type::TypeVar(_) => self.inference.fresh_var(),
            Type::Error => Type::Error,
            _ => {
                self.errors
                    .push(TypeError::no_such_field(obj.clone(), field, span));
                Type::Error
            }
        }
    }

    /// Get the type of a String method (returns function type for methods)
    fn check_string_method(&mut self, method: &str, span: Span) -> Type {
        match method {
            "len" | "length" => Type::function(vec![], Type::Int),
            "is_empty" => Type::function(vec![], Type::Bool),
            "contains" => Type::function(vec![Type::String], Type::Bool),
            "starts_with" => Type::function(vec![Type::String], Type::Bool),
            "ends_with" => Type::function(vec![Type::String], Type::Bool),
            "to_upper" | "to_uppercase" => Type::function(vec![], Type::String),
            "to_lower" | "to_lowercase" => Type::function(vec![], Type::String),
            "trim" => Type::function(vec![], Type::String),
            "trim_start" => Type::function(vec![], Type::String),
            "trim_end" => Type::function(vec![], Type::String),
            "split" => Type::function(vec![Type::String], Type::list(Type::String)),
            "replace" => Type::function(vec![Type::String, Type::String], Type::String),
            "repeat" => Type::function(vec![Type::Int], Type::String),
            "substring" => Type::function(vec![Type::Int, Type::Int], Type::String),
            "chars" => Type::function(vec![], Type::list(Type::String)),
            "index_of" => Type::function(vec![Type::String], Type::nullable(Type::Int)),
            _ => {
                self.errors
                    .push(TypeError::no_such_field(Type::String, method, span));
                Type::Error
            }
        }
    }

    /// Get the type of a List method (returns function type for methods)
    fn check_list_method(&mut self, method: &str, elem_type: &Type, span: Span) -> Type {
        let elem = elem_type.clone();
        match method {
            "len" | "length" => Type::function(vec![], Type::Int),
            "is_empty" => Type::function(vec![], Type::Bool),
            "first" => Type::function(vec![], Type::nullable(elem.clone())),
            "last" => Type::function(vec![], Type::nullable(elem.clone())),
            "push" => Type::function(vec![elem.clone()], Type::Unit),
            "pop" => Type::function(vec![], Type::nullable(elem.clone())),
            "contains" => Type::function(vec![elem.clone()], Type::Bool),
            "reverse" => Type::function(vec![], Type::list(elem.clone())),
            "sort" => Type::function(vec![], Type::list(elem.clone())),
            "join" => Type::function(vec![Type::String], Type::String),
            "get" => Type::function(vec![Type::Int], Type::nullable(elem.clone())),
            // Higher-order methods - use type variables for flexibility
            "map" => {
                let result_type = self.inference.fresh_var();
                let mapper_type = Type::function(vec![elem.clone()], result_type.clone());
                Type::function(vec![mapper_type], Type::list(result_type))
            }
            "filter" => {
                let predicate_type = Type::function(vec![elem.clone()], Type::Bool);
                Type::function(vec![predicate_type], Type::list(elem.clone()))
            }
            "reduce" => {
                let acc_type = self.inference.fresh_var();
                let reducer_type =
                    Type::function(vec![acc_type.clone(), elem.clone()], acc_type.clone());
                // reduce(reducer, initial_value) -> acc_type
                Type::function(vec![reducer_type, acc_type.clone()], acc_type)
            }
            "find" => {
                let predicate_type = Type::function(vec![elem.clone()], Type::Bool);
                Type::function(vec![predicate_type], Type::nullable(elem.clone()))
            }
            _ => {
                self.errors
                    .push(TypeError::no_such_field(Type::list(elem), method, span));
                Type::Error
            }
        }
    }

    /// Get the type of a Map method (returns function type for methods)
    fn check_map_method(
        &mut self,
        method: &str,
        key_type: &Type,
        value_type: &Type,
        span: Span,
    ) -> Type {
        let key = key_type.clone();
        let value = value_type.clone();
        match method {
            "len" => Type::function(vec![], Type::Int),
            "is_empty" => Type::function(vec![], Type::Bool),
            "get" => Type::function(vec![key.clone()], Type::nullable(value.clone())),
            "set" => Type::function(vec![key.clone(), value.clone()], Type::Unit),
            "remove" => Type::function(vec![key.clone()], Type::nullable(value.clone())),
            "contains_key" => Type::function(vec![key.clone()], Type::Bool),
            "keys" => Type::function(vec![], Type::list(key.clone())),
            "values" => Type::function(vec![], Type::list(value.clone())),
            "entries" => Type::function(
                vec![],
                Type::list(Type::Tuple(vec![key.clone(), value.clone()])),
            ),
            "clear" => Type::function(vec![], Type::Unit),
            _ => {
                self.errors.push(TypeError::no_such_field(
                    Type::Map(Box::new(key), Box::new(value)),
                    method,
                    span,
                ));
                Type::Error
            }
        }
    }

    /// Check null-safe field access
    fn check_null_safe_field(&mut self, obj: &Type, field: &str, span: Span) -> Type {
        let obj = self.inference.apply(obj);

        match &obj {
            Type::Nullable(inner) => {
                let field_type = self.check_field_access(inner, field, span);
                Type::nullable(field_type)
            }
            Type::Error => Type::Error,
            _ => {
                self.errors.push(TypeError::new(
                    TypeErrorKind::UnnecessaryNullSafe(obj.clone()),
                    span,
                ));
                self.check_field_access(&obj, field, span)
            }
        }
    }

    /// Check null-safe index access (container?.[index])
    fn check_null_safe_index(&mut self, container: &Type, index: &Type, span: Span) -> Type {
        let container = self.inference.apply(container);

        match &container {
            Type::Nullable(inner) => {
                let elem_type = self.check_index(inner, index, span);
                Type::nullable(elem_type)
            }
            Type::Error => Type::Error,
            _ => {
                self.errors.push(TypeError::new(
                    TypeErrorKind::UnnecessaryNullSafe(container.clone()),
                    span,
                ));
                self.check_index(&container, index, span)
            }
        }
    }

    /// Check a lambda expression
    fn check_lambda(
        &mut self,
        params: &[Param],
        body: &Expr,
        return_type: Option<&TypeAnnotation>,
        span: Span,
    ) -> Type {
        self.env.enter_scope();

        let param_types: Vec<Type> = params
            .iter()
            .map(|p| {
                let ty = self.resolve_param_type(&p.ty);
                self.env.define_var(&p.name.name, ty.clone(), false);
                ty
            })
            .collect();

        let expected_ret = return_type.map(|t| self.resolve_type_annotation(t));
        self.env.set_return_type(expected_ret.clone());

        let body_type = self.check_expr(body);

        let ret_type = if let Some(expected) = expected_ret {
            if !self.inference.unify(&body_type, &expected, span) {
                self.errors.push(TypeError::new(
                    TypeErrorKind::ReturnTypeMismatch {
                        expected: expected.clone(),
                        found: body_type,
                    },
                    span,
                ));
            }
            expected
        } else {
            body_type
        };

        self.env.set_return_type(None);
        self.env.exit_scope();

        Type::function(param_types, ret_type)
    }

    /// Check a struct initialization
    fn check_struct_init(&mut self, name: &Ident, fields: &[FieldInit], span: Span) -> Type {
        // First, get struct info and clone what we need
        let struct_data = self.env.lookup_struct(&name.name).map(|(id, info)| {
            (
                id,
                info.name.clone(),
                info.fields.clone(),
                info.type_param_vars.clone(),
            )
        });

        if let Some((id, struct_name, expected_fields, type_param_vars)) = struct_data {
            // Create fresh type variables for each type parameter
            let type_args: Vec<Type> = type_param_vars
                .iter()
                .map(|_| self.inference.fresh_var())
                .collect();

            let mut seen_fields = std::collections::HashSet::new();

            for field_init in fields {
                let field_name = &field_init.name.name;

                if !seen_fields.insert(field_name.clone()) {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::DuplicateField(field_name.clone()),
                        field_init.name.span,
                    ));
                    continue;
                }

                if let Some(field_info) = expected_fields.get(field_name) {
                    // Substitute old type variables (from registration) with fresh ones
                    let expected_type =
                        Self::substitute_type_vars(&field_info.ty, &type_param_vars, &type_args);

                    // Handle shorthand syntax: { x } means { x: x }
                    let value_type = if let Some(ref value) = field_init.value {
                        self.check_expr(value)
                    } else {
                        // Shorthand: look up variable with same name
                        if let Some(var_info) = self.env.lookup_var(field_name) {
                            var_info.ty.clone()
                        } else {
                            self.errors
                                .push(TypeError::undefined_variable(field_name, field_init.span));
                            Type::Error
                        }
                    };

                    if !self
                        .inference
                        .unify(&value_type, &expected_type, field_init.span)
                    {
                        self.errors.push(TypeError::mismatch(
                            expected_type,
                            value_type,
                            field_init.span,
                        ));
                    }
                } else {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::ExtraField {
                            struct_name: name.name.clone(),
                            field: field_name.clone(),
                        },
                        field_init.name.span,
                    ));
                }
            }

            // Check for missing fields
            for field_name in expected_fields.keys() {
                if !seen_fields.contains(field_name) {
                    self.errors.push(TypeError::new(
                        TypeErrorKind::MissingField {
                            struct_name: name.name.clone(),
                            field: field_name.clone(),
                        },
                        span,
                    ));
                }
            }

            Type::struct_type(id, struct_name, type_args)
        } else {
            self.errors.push(TypeError::new(
                TypeErrorKind::UndefinedStruct(name.name.clone()),
                span,
            ));
            Type::Error
        }
    }

    /// Check an enum variant construction
    fn check_enum_variant(
        &mut self,
        enum_name: Option<&Ident>,
        variant: &Ident,
        data: Option<&Expr>,
        span: Span,
    ) -> Type {
        let lookup_name = enum_name.map_or("_", |n| &n.name);

        if let Some(enum_ident) = enum_name {
            // Get enum info and clone what we need
            let enum_data = self.env.lookup_enum(&enum_ident.name).map(|(id, info)| {
                (
                    id,
                    info.name.clone(),
                    info.variants.clone(),
                    info.type_param_vars.clone(),
                )
            });

            if let Some((id, name, variants, type_param_vars)) = enum_data {
                // Create fresh type variables for each type parameter
                let type_args: Vec<Type> = type_param_vars
                    .iter()
                    .map(|_| self.inference.fresh_var())
                    .collect();

                if let Some(variant_info) = variants.get(&variant.name) {
                    match (&variant_info.data, data) {
                        (None, None) => {}
                        (None, Some(_)) => {
                            self.errors.push(TypeError::new(
                                TypeErrorKind::WrongArgumentCount {
                                    expected: 0,
                                    found: 1,
                                },
                                span,
                            ));
                        }
                        (Some(VariantData::Tuple(expected)), Some(actual)) => {
                            let actual_type = self.check_expr(actual);
                            if expected.len() == 1 {
                                // Substitute old type variables with fresh ones
                                let expected_type = Self::substitute_type_vars(
                                    &expected[0],
                                    &type_param_vars,
                                    &type_args,
                                );
                                if !self.inference.unify(&actual_type, &expected_type, span) {
                                    self.errors.push(TypeError::mismatch(
                                        expected_type,
                                        actual_type,
                                        span,
                                    ));
                                }
                            }
                        }
                        (Some(VariantData::Tuple(_)), None) => {
                            self.errors.push(TypeError::new(
                                TypeErrorKind::WrongArgumentCount {
                                    expected: 1,
                                    found: 0,
                                },
                                span,
                            ));
                        }
                        (Some(VariantData::Struct(_)), _) => {
                            // Struct variants handled differently
                        }
                    }

                    return Type::enum_type(id, name, type_args);
                } else {
                    self.errors.push(TypeError::undefined_variable(
                        format!("{}::{}", enum_ident.name, variant.name),
                        span,
                    ));
                    return Type::Error;
                }
            }
        }

        self.errors.push(TypeError::new(
            TypeErrorKind::UndefinedEnum(lookup_name.to_string()),
            span,
        ));
        Type::Error
    }

    /// Check a pattern and bind variables
    fn check_pattern(&mut self, pattern: &Pattern, expected: &Type) {
        match &pattern.kind {
            PatternKind::Wildcard => {}
            PatternKind::Ident(name) => {
                self.env.define_var(&name.name, expected.clone(), false);
            }
            PatternKind::Literal(lit) => {
                let lit_type = self.check_literal(lit);
                if !self.inference.unify(&lit_type, expected, pattern.span) {
                    self.errors.push(TypeError::mismatch(
                        expected.clone(),
                        lit_type,
                        pattern.span,
                    ));
                }
            }
            PatternKind::Variant { variant, data, .. } => {
                if let Type::Enum { id, .. } = self.inference.apply(expected) {
                    // Clone variant info to avoid borrow issues
                    let variant_data = self
                        .env
                        .get_enum(id)
                        .and_then(|info| info.variants.get(&variant.name))
                        .and_then(|v| v.data.clone());

                    if let (Some(VariantData::Tuple(types)), Some(pat)) = (variant_data, data) {
                        if let Some(first_type) = types.first() {
                            self.check_pattern(pat, first_type);
                        }
                    }
                }
            }
            PatternKind::Struct { fields, .. } => {
                if let Type::Struct { id, .. } = self.inference.apply(expected) {
                    // Collect field types first to avoid borrow issues
                    let field_types: Vec<_> = fields
                        .iter()
                        .filter_map(|fp| {
                            self.env
                                .get_struct(id)
                                .and_then(|info| info.fields.get(&fp.name.name))
                                .map(|f| (fp.name.name.clone(), f.ty.clone(), fp.pattern.clone()))
                        })
                        .collect();

                    for (field_name, field_ty, pattern) in field_types {
                        if let Some(pat) = pattern {
                            self.check_pattern(&pat, &field_ty);
                        } else {
                            self.env.define_var(&field_name, field_ty, false);
                        }
                    }
                }
            }
            PatternKind::List { elements, rest } => {
                if let Type::List(elem_type) = self.inference.apply(expected) {
                    for pat in elements {
                        self.check_pattern(pat, &elem_type);
                    }
                    if let Some(rest_pat) = rest {
                        self.check_pattern(rest_pat, expected);
                    }
                }
            }
            PatternKind::Or(patterns) => {
                for pat in patterns {
                    self.check_pattern(pat, expected);
                }
            }
        }
    }

    /// Bind variables from a pattern
    fn bind_pattern(&mut self, pattern: &Pattern, ty: &Type) {
        match &pattern.kind {
            PatternKind::Wildcard => {}
            PatternKind::Ident(name) => {
                self.env.define_var(&name.name, ty.clone(), false);
            }
            PatternKind::Literal(_) => {}
            PatternKind::List { elements, rest } => {
                if let Type::List(elem_ty) = self.inference.apply(ty) {
                    for pat in elements {
                        self.bind_pattern(pat, &elem_ty);
                    }
                    if let Some(rest_pat) = rest {
                        self.bind_pattern(rest_pat, ty);
                    }
                }
            }
            PatternKind::Variant { data, .. } => {
                if let Some(pat) = data {
                    self.bind_pattern(pat, &Type::Error);
                }
            }
            PatternKind::Struct { fields, .. } => {
                if let Type::Struct { id, .. } = self.inference.apply(ty) {
                    // Collect field info first to avoid borrow issues
                    let field_data: Vec<_> = fields
                        .iter()
                        .filter_map(|fp| {
                            self.env
                                .get_struct(id)
                                .and_then(|info| info.fields.get(&fp.name.name))
                                .map(|f| (fp.name.name.clone(), f.ty.clone(), fp.pattern.clone()))
                        })
                        .collect();

                    for (field_name, field_ty, pattern) in field_data {
                        if let Some(pat) = pattern {
                            self.bind_pattern(&pat, &field_ty);
                        } else {
                            self.env.define_var(&field_name, field_ty, false);
                        }
                    }
                }
            }
            PatternKind::Or(patterns) => {
                if let Some(first) = patterns.first() {
                    self.bind_pattern(first, ty);
                }
            }
        }
    }

    /// Get the element type of an iterator
    fn get_iterator_element_type(&mut self, iter_type: &Type, span: Span) -> Type {
        let iter_type = self.inference.apply(iter_type);

        match &iter_type {
            Type::List(elem) => *elem.clone(),
            Type::String => Type::String,
            Type::Map(key, _) => *key.clone(),
            Type::Range => Type::Int,
            Type::Error => Type::Error,
            // Type variables (from dynamic sources like Json.parse) - constrain to list
            Type::TypeVar(_) => {
                let elem_type = self.inference.fresh_var();
                let list_type = Type::list(elem_type.clone());
                self.inference.unify(&iter_type, &list_type, span);
                elem_type
            }
            _ => {
                self.errors.push(TypeError::new(
                    TypeErrorKind::TypeMismatch {
                        expected: Type::list(self.inference.fresh_var()),
                        found: iter_type.clone(),
                    },
                    span,
                ));
                Type::Error
            }
        }
    }

    /// Resolve a type annotation to an internal type
    fn resolve_type_annotation(&mut self, annotation: &TypeAnnotation) -> Type {
        match &annotation.kind {
            TypeKind::Named { name, args } => {
                let type_name = &name.name;

                match type_name.as_str() {
                    "Int" => return Type::Int,
                    "Float" => return Type::Float,
                    "Bool" => return Type::Bool,
                    "String" => return Type::String,
                    "Null" => return Type::Null,
                    "List" if args.len() == 1 => {
                        let elem = self.resolve_type_annotation(&args[0]);
                        return Type::list(elem);
                    }
                    "Map" if args.len() == 2 => {
                        let key = self.resolve_type_annotation(&args[0]);
                        let value = self.resolve_type_annotation(&args[1]);
                        return Type::map(key, value);
                    }
                    _ => {}
                }

                // Check for type parameters in scope (e.g., T in struct Box<T>)
                if let Some(type_var) = self.type_params_in_scope.get(type_name) {
                    // Type parameters should not have type arguments
                    if !args.is_empty() {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::WrongTypeArgCount {
                                name: type_name.clone(),
                                expected: 0,
                                found: args.len(),
                            },
                            annotation.span,
                        ));
                        return Type::Error;
                    }
                    return type_var.clone();
                }

                // Get struct/enum info and clone what we need
                let struct_info = self
                    .env
                    .lookup_struct(type_name)
                    .map(|(id, info)| (id, info.name.clone(), info.type_params.len()));

                if let Some((id, name, expected_type_params)) = struct_info {
                    // Validate type argument count
                    if args.len() != expected_type_params {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::WrongTypeArgCount {
                                name: name.clone(),
                                expected: expected_type_params,
                                found: args.len(),
                            },
                            annotation.span,
                        ));
                        return Type::Error;
                    }

                    let type_args: Vec<Type> = args
                        .iter()
                        .map(|a| self.resolve_type_annotation(a))
                        .collect();
                    return Type::struct_type(id, name, type_args);
                }

                let enum_info = self
                    .env
                    .lookup_enum(type_name)
                    .map(|(id, info)| (id, info.name.clone(), info.type_params.len()));

                if let Some((id, name, expected_type_params)) = enum_info {
                    // Validate type argument count
                    if args.len() != expected_type_params {
                        self.errors.push(TypeError::new(
                            TypeErrorKind::WrongTypeArgCount {
                                name: name.clone(),
                                expected: expected_type_params,
                                found: args.len(),
                            },
                            annotation.span,
                        ));
                        return Type::Error;
                    }

                    let type_args: Vec<Type> = args
                        .iter()
                        .map(|a| self.resolve_type_annotation(a))
                        .collect();
                    return Type::enum_type(id, name, type_args);
                }

                self.errors
                    .push(TypeError::undefined_type(type_name, annotation.span));
                Type::Error
            }

            TypeKind::Nullable(inner) => {
                let inner_type = self.resolve_type_annotation(inner);
                Type::nullable(inner_type)
            }

            TypeKind::Function { params, ret } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                let ret_type = self.resolve_type_annotation(ret);
                Type::function(param_types, ret_type)
            }

            TypeKind::Tuple(elems) => {
                let types: Vec<Type> = elems
                    .iter()
                    .map(|e| self.resolve_type_annotation(e))
                    .collect();
                Type::Tuple(types)
            }

            TypeKind::List(elem) => {
                let elem_type = self.resolve_type_annotation(elem);
                Type::list(elem_type)
            }

            TypeKind::Unit => Type::Unit,

            TypeKind::Never => Type::Never,

            TypeKind::Inferred => self.inference.fresh_var(),
        }
    }

    /// Substitute type variables in a type
    /// Replaces any TypeVar with the given old_ids with the corresponding new types
    fn substitute_type_vars(ty: &Type, old_ids: &[TypeVarId], new_types: &[Type]) -> Type {
        match ty {
            Type::TypeVar(id) => {
                // Check if this TypeVar should be substituted
                if let Some(pos) = old_ids.iter().position(|old_id| old_id == id) {
                    new_types[pos].clone()
                } else {
                    ty.clone()
                }
            }
            Type::List(inner) => Type::list(Self::substitute_type_vars(inner, old_ids, new_types)),
            Type::Map(key, value) => Type::map(
                Self::substitute_type_vars(key, old_ids, new_types),
                Self::substitute_type_vars(value, old_ids, new_types),
            ),
            Type::Nullable(inner) => {
                Type::nullable(Self::substitute_type_vars(inner, old_ids, new_types))
            }
            Type::Future(inner) => {
                Type::future(Self::substitute_type_vars(inner, old_ids, new_types))
            }
            Type::Tuple(elems) => Type::Tuple(
                elems
                    .iter()
                    .map(|e| Self::substitute_type_vars(e, old_ids, new_types))
                    .collect(),
            ),
            Type::Function { params, ret } => Type::function(
                params
                    .iter()
                    .map(|p| Self::substitute_type_vars(p, old_ids, new_types))
                    .collect(),
                Self::substitute_type_vars(ret, old_ids, new_types),
            ),
            Type::Struct {
                id,
                name,
                type_args,
            } => Type::struct_type(
                *id,
                name.clone(),
                type_args
                    .iter()
                    .map(|a| Self::substitute_type_vars(a, old_ids, new_types))
                    .collect(),
            ),
            Type::Enum {
                id,
                name,
                type_args,
            } => Type::enum_type(
                *id,
                name.clone(),
                type_args
                    .iter()
                    .map(|a| Self::substitute_type_vars(a, old_ids, new_types))
                    .collect(),
            ),
            // Primitive types, Any, and namespaces don't need substitution
            Type::Int
            | Type::Float
            | Type::Bool
            | Type::String
            | Type::Null
            | Type::Unit
            | Type::Never
            | Type::Error
            | Type::Any
            | Type::Range
            | Type::Namespace(_) => ty.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn check(source: &str) -> TypeCheckResult {
        let module = Parser::parse_module(source).expect("parse failed");
        let mut checker = TypeChecker::new();
        checker.check_module(&module)
    }

    #[test]
    fn test_literal_types() {
        let result = check("fx main() { let x = 42 }");
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_type_annotation() {
        let result = check("fx main() { let x: Int = 42 }");
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_type_mismatch() {
        let result = check("fx main() { let x: String = 42 }");
        assert!(!result.success);
    }

    #[test]
    fn test_undefined_variable() {
        let result = check("fx main() { let y = x }");
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UndefinedVariable(_))));
    }

    #[test]
    fn test_function_call() {
        let result = check(
            r#"
            fx add(a: Int, b: Int) -> Int { a + b }
            fx main() { let x = add(1, 2) }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_wrong_arg_count() {
        let result = check(
            r#"
            fx greet(name: String) { }
            fx main() { greet() }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::WrongArgumentCount { .. })));
    }

    #[test]
    fn test_if_expression() {
        let result = check(
            r#"
            fx main() {
                let x = if true { 1 } else { 2 }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_binary_ops() {
        let result = check(
            r#"
            fx main() {
                let a = 1 + 2
                let b = 3.0 * 4.0
                let c = "hello" + " world"
                let d = 1 < 2
                let e = true && false
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_binary_op() {
        let result = check(
            r#"
            fx main() {
                let x = "hello" - "world"
            }
        "#,
        );
        assert!(!result.success);
    }

    #[test]
    fn test_list_literal() {
        let result = check(
            r#"
            fx main() {
                let xs = [1, 2, 3]
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_for_loop() {
        let result = check(
            r#"
            fx main() {
                for x in [1, 2, 3] {
                    let y = x + 1
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_while_loop() {
        let result = check(
            r#"
            fx main() {
                while true {
                    break
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_break_in_loop() {
        // Parser already catches break/continue outside loops, so we just test valid usage
        let result = check(
            r#"
            fx main() {
                while true {
                    if true { break }
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_struct_definition() {
        let result = check(
            r#"
            struct Point {
                x: Float,
                y: Float
            }
            fx main() { }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_return_type() {
        let result = check(
            r#"
            fx double(x: Int) -> Int {
                x * 2
            }
            fx main() { }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_return_type_mismatch() {
        let result = check(
            r#"
            fx greet() -> Int {
                "hello"
            }
        "#,
        );
        assert!(!result.success);
    }

    #[test]
    fn test_lambda() {
        let result = check(
            r#"
            fx main() {
                let f = |x| x + 1
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_nullable_type_annotation() {
        let result = check(
            r#"
            fx main() {
                let x: Int? = null
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_null_coalescing() {
        let result = check(
            r#"
            fx get_value() -> Int? { null }
            fx main() {
                let x: Int = get_value() ?? 0
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_null_coalescing_on_non_nullable() {
        let result = check(
            r#"
            fx main() {
                let x: Int = 42
                let y = x ?? 0
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UnnecessaryNullSafe(_))));
    }

    #[test]
    fn test_type_narrowing_ne_null() {
        // After checking x != null, x should be narrowed to Int in the then-branch
        let result = check(
            r#"
            fx get_value() -> Int? { null }
            fx main() {
                let x: Int? = get_value()
                if x != null {
                    let y: Int = x
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_type_narrowing_eq_null() {
        // After checking x == null, x should be narrowed to Int in the else-branch
        let result = check(
            r#"
            fx get_value() -> Int? { null }
            fx main() {
                let x: Int? = get_value()
                if x == null {
                    let y = 0
                } else {
                    let y: Int = x
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_type_narrowing_compound_and() {
        // Both x and y should be narrowed in the then-branch
        let result = check(
            r#"
            fx get_value() -> Int? { null }
            fx main() {
                let x: Int? = get_value()
                let y: Int? = get_value()
                if x != null && y != null {
                    let sum: Int = x + y
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_null_safe_field_access() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx get_point() -> Point? { null }
            fx main() {
                let p: Point? = get_point()
                let x: Int? = p?.x
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_null_safe_field_on_non_nullable() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() {
                let p = Point { x: 1, y: 2 }
                let x = p?.x
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UnnecessaryNullSafe(_))));
    }

    #[test]
    fn test_null_safe_index_access() {
        let result = check(
            r#"
            fx get_list() -> List<Int>? { null }
            fx main() {
                let list: List<Int>? = get_list()
                let x: Int? = list?.[0]
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_null_safe_index_on_non_nullable() {
        let result = check(
            r#"
            fx main() {
                let list = [1, 2, 3]
                let x = list?.[0]
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UnnecessaryNullSafe(_))));
    }

    // === Control Flow Type Checking Tests (2.4.6) ===

    #[test]
    fn test_if_else_incompatible_branches() {
        // If/else branches must have compatible types
        let result = check(
            r#"
            fx main() {
                let x = if true { 42 } else { "hello" }
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::IncompatibleBranches { .. })));
    }

    #[test]
    fn test_if_else_compatible_branches() {
        // Compatible branches should unify
        let result = check(
            r#"
            fx main() {
                let x = if true { 1 } else { 2 }
                let y: Int = x
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_if_else_if_branches_unify() {
        // Multiple else-if branches should all unify
        let result = check(
            r#"
            fx main() {
                let x = if true { 1 } else if false { 2 } else { 3 }
                let y: Int = x
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_if_else_if_incompatible() {
        // Incompatible types in else-if chain
        let result = check(
            r#"
            fx main() {
                let x = if true { 1 } else if false { "two" } else { 3 }
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::IncompatibleBranches { .. })));
    }

    #[test]
    fn test_match_arms_compatible() {
        let result = check(
            r#"
            fx main() {
                let x = 1
                let y = match x {
                    1 => 10,
                    2 => 20,
                    _ => 0
                }
                let z: Int = y
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_match_arms_incompatible() {
        // Match arms must have compatible types
        let result = check(
            r#"
            fx main() {
                let x = 1
                let y = match x {
                    1 => 10,
                    2 => "twenty",
                    _ => 0
                }
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::IncompatibleBranches { .. })));
    }

    #[test]
    fn test_return_statement_type_consistency() {
        // Multiple return points must match declared return type (using expression forms)
        let result = check(
            r#"
            fx get_value(flag: Bool) -> Int {
                if flag { 42 } else { 0 }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_return_statement_type_mismatch() {
        // Return expression type doesn't match declared return type
        let result = check(
            r#"
            fx get_value(flag: Bool) -> Int {
                if flag { "hello" } else { 0 }
            }
        "#,
        );
        assert!(!result.success);
        // This should fail due to incompatible branches or return type mismatch
        assert!(result.errors.iter().any(|e| matches!(
            e.kind,
            TypeErrorKind::ReturnTypeMismatch { .. }
        ) || matches!(
            e.kind,
            TypeErrorKind::IncompatibleBranches { .. }
        )));
    }

    #[test]
    fn test_implicit_return_type_consistency() {
        // Implicit return (last expression) must match declared type
        let result = check(
            r#"
            fx get_value() -> Int {
                42
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_implicit_return_type_mismatch() {
        // Implicit return doesn't match declared type
        let result = check(
            r#"
            fx get_value() -> String {
                42
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::ReturnTypeMismatch { .. })));
    }

    #[test]
    fn test_if_without_else_returns_unit() {
        // If without else should return Unit
        let result = check(
            r#"
            fx main() {
                let x = if true { 42 }
            }
        "#,
        );
        // This should succeed - x will be Unit
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_match_with_guard() {
        // Match arms with guards should type check correctly
        let result = check(
            r#"
            fx main() {
                let x = 5
                let y = match x {
                    n if n > 10 => "big",
                    n if n > 0 => "small",
                    _ => "zero or negative"
                }
                let z: String = y
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_match_guard_must_be_bool() {
        // Match guard must evaluate to Bool
        let result = check(
            r#"
            fx main() {
                let x = 5
                let y = match x {
                    n if "not a bool" => 1,
                    _ => 0
                }
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::TypeMismatch { .. })));
    }

    // === Struct Type Checking Tests (2.4.7) ===

    #[test]
    fn test_struct_init_basic() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() { let p = Point { x: 1, y: 2 } }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_struct_field_access_type() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() {
                let p = Point { x: 1, y: 2 }
                let val: Int = p.x
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_struct_missing_field() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() { let p = Point { x: 1 } }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::MissingField { .. })));
    }

    #[test]
    fn test_struct_extra_field() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() { let p = Point { x: 1, y: 2, z: 3 } }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::ExtraField { .. })));
    }

    #[test]
    fn test_struct_duplicate_field() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() { let p = Point { x: 1, x: 2, y: 3 } }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::DuplicateField(_))));
    }

    #[test]
    fn test_struct_wrong_field_type() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() { let p = Point { x: "hello", y: 2 } }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_struct_invalid_field_access() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() {
                let p = Point { x: 1, y: 2 }
                let z = p.z
            }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::NoSuchField { .. })));
    }

    #[test]
    fn test_struct_undefined_type_in_field() {
        let result = check(
            r#"
            struct Foo { field: UndefinedType }
            fx main() {}
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UndefinedType(_))));
    }

    #[test]
    fn test_struct_shorthand_init() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() {
                let x = 1
                let y = 2
                let p = Point { x, y }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_struct_pattern_basic() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            fx main() {
                let p = Point { x: 1, y: 2 }
                let sum = match p {
                    Point { x, y } => x + y
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_struct_nested() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }
            struct Line { start: Point, end: Point }
            fx main() {
                let line = Line {
                    start: Point { x: 0, y: 0 },
                    end: Point { x: 10, y: 10 }
                }
                let sx: Int = line.start.x
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    // === Enum Type Checking Tests (2.4.7) ===

    #[test]
    fn test_enum_unit_variant() {
        let result = check(
            r#"
            enum Color { Red, Green, Blue }
            fx main() { let c = Color::Red }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_enum_tuple_variant() {
        let result = check(
            r#"
            enum Option { Some(Int), None }
            fx main() { let x = Option::Some(42) }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_enum_tuple_variant_wrong_type() {
        let result = check(
            r#"
            enum Option { Some(Int), None }
            fx main() { let x = Option::Some("hello") }
        "#,
        );
        assert!(!result.success);
    }

    #[test]
    fn test_enum_undefined() {
        let result = check(
            r#"
            fx main() { let x = Unknown::Variant }
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UndefinedEnum(_))));
    }

    #[test]
    fn test_enum_undefined_variant() {
        let result = check(
            r#"
            enum Color { Red, Green, Blue }
            fx main() { let x = Color::Purple }
        "#,
        );
        assert!(!result.success);
    }

    #[test]
    fn test_enum_pattern_unit() {
        let result = check(
            r#"
            enum Color { Red, Green, Blue }
            fx main() {
                let c = Color::Red
                let val = match c {
                    Color::Red => 1,
                    Color::Green => 2,
                    Color::Blue => 3
                }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_enum_pattern_tuple() {
        let result = check(
            r#"
            enum Option { Some(Int), None }
            fx main() {
                let x = Option::Some(42)
                let val = match x {
                    Option::Some(n) => n,
                    Option::None => 0
                }
                let y: Int = val
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_enum_undefined_type_in_variant() {
        let result = check(
            r#"
            enum Result { Ok(UndefinedType), Err(String) }
            fx main() {}
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UndefinedType(_))));
    }

    #[test]
    fn test_enum_with_struct_variant() {
        let result = check(
            r#"
            enum Event {
                Click { x: Int, y: Int },
                KeyPress { key: String }
            }
            fx main() {
                let e = Event::Click { x: 10, y: 20 }
            }
        "#,
        );
        // This tests struct-like enum variants
        assert!(result.success, "errors: {:?}", result.errors);
    }

    // ==================== Interface and Impl Tests ====================

    #[test]
    fn test_interface_definition() {
        let result = check(
            r#"
            interface Drawable {
                fx draw() -> String
            }
            fx main() {}
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_impl_for_struct() {
        let result = check(
            r#"
            struct Circle { radius: Float }

            impl Circle {
                fx area() -> Float {
                    3.14159 * self.radius * self.radius
                }
            }

            fx main() {}
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_impl_interface_for_struct() {
        let result = check(
            r#"
            interface Drawable {
                fx draw() -> String
            }

            struct Circle { radius: Float }

            impl Drawable for Circle {
                fx draw() -> String {
                    "circle"
                }
            }

            fx main() {}
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_impl_missing_method() {
        let result = check(
            r#"
            interface Drawable {
                fx draw() -> String
                fx color() -> String
            }

            struct Circle { radius: Float }

            impl Drawable for Circle {
                fx draw() -> String {
                    "circle"
                }
            }

            fx main() {}
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::MissingInterfaceMethod { .. })));
    }

    #[test]
    fn test_impl_wrong_signature() {
        let result = check(
            r#"
            interface Drawable {
                fx draw() -> String
            }

            struct Circle { radius: Float }

            impl Drawable for Circle {
                fx draw() -> Int {
                    42
                }
            }

            fx main() {}
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::MethodSignatureMismatch { .. })));
    }

    #[test]
    fn test_impl_undefined_interface() {
        let result = check(
            r#"
            struct Circle { radius: Float }

            impl UndefinedInterface for Circle {
                fx draw() -> String {
                    "circle"
                }
            }

            fx main() {}
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::UndefinedInterface(_))));
    }

    #[test]
    fn test_impl_undefined_target_type() {
        let result = check(
            r#"
            interface Drawable {
                fx draw() -> String
            }

            impl Drawable for UndefinedType {
                fx draw() -> String {
                    "unknown"
                }
            }

            fx main() {}
        "#,
        );
        assert!(!result.success);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e.kind, TypeErrorKind::ImplTargetNotFound(_))));
    }

    #[test]
    fn test_impl_self_access() {
        let result = check(
            r#"
            struct Point { x: Int, y: Int }

            impl Point {
                fx magnitude() -> Int {
                    self.x + self.y
                }
            }

            fx main() {}
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_interface_with_default_method() {
        let result = check(
            r#"
            interface Describable {
                fx name() -> String
                fx description() -> String {
                    "default description"
                }
            }

            struct Item { title: String }

            impl Describable for Item {
                fx name() -> String {
                    self.title
                }
            }

            fx main() {}
        "#,
        );
        // Should succeed because description() has a default implementation
        assert!(result.success, "errors: {:?}", result.errors);
    }

    // ========== Generic Type Instantiation Tests ==========

    #[test]
    fn test_generic_struct_definition() {
        // Generic struct with type parameter used in field
        let result = check(
            r#"
            struct Box<T> {
                value: T
            }

            fx main() {}
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_generic_struct_instantiation_correct() {
        // Correct instantiation with one type argument
        let result = check(
            r#"
            struct Box<T> {
                value: T
            }

            fx main() {
                let b: Box<Int> = Box { value: 42 }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_generic_struct_too_many_type_args() {
        // Too many type arguments
        let result = check(
            r#"
            struct Box<T> {
                value: T
            }

            fx main() {
                let b: Box<Int, String> = Box { value: 42 }
            }
        "#,
        );
        assert!(!result.success);
        assert!(result.errors.iter().any(|e| matches!(
            &e.kind,
            TypeErrorKind::WrongTypeArgCount { name, expected: 1, found: 2 } if name == "Box"
        )));
    }

    #[test]
    fn test_generic_struct_too_few_type_args() {
        // Too few type arguments (non-generic struct used with type args)
        let result = check(
            r#"
            struct Point {
                x: Int,
                y: Int
            }

            fx main() {
                let p: Point<Int> = Point { x: 1, y: 2 }
            }
        "#,
        );
        assert!(!result.success);
        assert!(result.errors.iter().any(|e| matches!(
            &e.kind,
            TypeErrorKind::WrongTypeArgCount { name, expected: 0, found: 1 } if name == "Point"
        )));
    }

    #[test]
    fn test_generic_enum_definition() {
        // Generic enum with type parameter
        let result = check(
            r#"
            enum Option<T> {
                Some(T),
                None
            }

            fx main() {}
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_generic_enum_instantiation_correct() {
        // Correct instantiation of generic enum
        let result = check(
            r#"
            enum Result<T, E> {
                Ok(T),
                Err(E)
            }

            fx main() {
                let r: Result<Int, String> = Result::Ok(42)
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_generic_enum_wrong_type_arg_count() {
        // Wrong number of type arguments for enum
        let result = check(
            r#"
            enum Result<T, E> {
                Ok(T),
                Err(E)
            }

            fx main() {
                let r: Result<Int> = Result::Ok(42)
            }
        "#,
        );
        assert!(!result.success);
        assert!(result.errors.iter().any(|e| matches!(
            &e.kind,
            TypeErrorKind::WrongTypeArgCount { name, expected: 2, found: 1 } if name == "Result"
        )));
    }

    #[test]
    fn test_multiple_type_parameters() {
        // Struct with multiple type parameters
        let result = check(
            r#"
            struct Pair<A, B> {
                first: A,
                second: B
            }

            fx main() {
                let p: Pair<Int, String> = Pair { first: 42, second: "hello" }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_nested_generic_types() {
        // Nested generic types
        let result = check(
            r#"
            struct Box<T> {
                value: T
            }

            fx main() {
                let b: Box<Box<Int>> = Box { value: Box { value: 42 } }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }

    #[test]
    fn test_generic_with_list() {
        // Generic struct containing a List
        let result = check(
            r#"
            struct Container<T> {
                items: List<T>
            }

            fx main() {
                let c: Container<Int> = Container { items: [1, 2, 3] }
            }
        "#,
        );
        assert!(result.success, "errors: {:?}", result.errors);
    }
}
