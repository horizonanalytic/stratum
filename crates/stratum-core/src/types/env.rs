//! Type environment / symbol table for the Stratum type checker
//!
//! Manages scopes, variable bindings, and type definitions.

use std::collections::HashMap;

use super::{EnumId, InterfaceId, StructId, Type};

/// Type environment managing scopes and type definitions
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Stack of scopes (innermost last)
    scopes: Vec<Scope>,

    /// Struct definitions by ID
    structs: HashMap<StructId, StructInfo>,

    /// Enum definitions by ID
    enums: HashMap<EnumId, EnumInfo>,

    /// Interface definitions by ID
    interfaces: HashMap<InterfaceId, InterfaceInfo>,

    /// Map from struct names to IDs
    struct_names: HashMap<String, StructId>,

    /// Map from enum names to IDs
    enum_names: HashMap<String, EnumId>,

    /// Map from interface names to IDs
    interface_names: HashMap<String, InterfaceId>,

    /// Impl blocks: (target_type, interface_name) -> ImplInfo
    /// For inherent impls, interface_name is None
    impls: HashMap<(String, Option<String>), ImplInfo>,

    /// Methods available on each type (collected from impls)
    /// Maps type_name -> method_name -> ImplMethodInfo
    type_methods: HashMap<String, HashMap<String, ImplMethodInfo>>,

    /// Counter for generating struct IDs
    next_struct_id: u32,

    /// Counter for generating enum IDs
    next_enum_id: u32,

    /// Counter for generating interface IDs
    next_interface_id: u32,

    /// Current function return type (None if not in a function)
    current_return_type: Option<Type>,

    /// Loop depth (for checking break/continue)
    loop_depth: u32,
}

/// A single scope containing variable bindings
#[derive(Debug, Clone, Default)]
struct Scope {
    /// Variables in this scope
    variables: HashMap<String, VarInfo>,

    /// Type aliases in this scope
    type_aliases: HashMap<String, Type>,
}

/// Information about a variable binding
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// The variable's type
    pub ty: Type,

    /// Whether this variable is mutable
    pub mutable: bool,
}

/// Information about a struct definition
#[derive(Debug, Clone)]
pub struct StructInfo {
    /// Struct name
    pub name: String,

    /// Type parameters (generic parameters)
    pub type_params: Vec<String>,

    /// TypeVarIds used for type parameters during registration
    /// Maps position to the TypeVarId used (for substitution during instantiation)
    pub type_param_vars: Vec<super::TypeVarId>,

    /// Fields with their types
    pub fields: HashMap<String, FieldInfo>,

    /// Order of fields (for construction)
    pub field_order: Vec<String>,
}

/// Information about a struct field
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Field type
    pub ty: Type,

    /// Whether the field is public
    pub public: bool,
}

/// Information about an enum definition
#[derive(Debug, Clone)]
pub struct EnumInfo {
    /// Enum name
    pub name: String,

    /// Type parameters (generic parameters)
    pub type_params: Vec<String>,

    /// TypeVarIds used for type parameters during registration
    pub type_param_vars: Vec<super::TypeVarId>,

    /// Variants
    pub variants: HashMap<String, VariantInfo>,
}

/// Information about an enum variant
#[derive(Debug, Clone)]
pub struct VariantInfo {
    /// Variant name
    pub name: String,

    /// Associated data (None for unit variants)
    pub data: Option<VariantData>,
}

/// Data associated with an enum variant
#[derive(Debug, Clone)]
pub enum VariantData {
    /// Tuple variant: Variant(T1, T2, ...)
    Tuple(Vec<Type>),

    /// Struct variant: Variant { field: T, ... }
    Struct(HashMap<String, Type>),
}

/// Information about an interface definition
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    /// Interface name
    pub name: String,

    /// Type parameters
    pub type_params: Vec<String>,

    /// Method signatures
    pub methods: HashMap<String, MethodInfo>,
}

/// Information about an interface method
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Parameter types (excluding self)
    pub params: Vec<Type>,

    /// Return type
    pub ret: Type,

    /// Whether this method has a default implementation
    pub has_default: bool,
}

/// Information about an impl block
#[derive(Debug, Clone)]
pub struct ImplInfo {
    /// The target type name (e.g., "Circle")
    pub target_type: String,

    /// The interface being implemented (None for inherent impls)
    pub interface_name: Option<String>,

    /// Methods defined in this impl
    pub methods: HashMap<String, ImplMethodInfo>,
}

/// Information about a method in an impl block
#[derive(Debug, Clone)]
pub struct ImplMethodInfo {
    /// Parameter types (excluding self)
    pub params: Vec<Type>,

    /// Return type
    pub ret: Type,

    /// Whether the method is async
    pub is_async: bool,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeEnv {
    /// Create a new type environment with built-in types
    #[must_use]
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![Scope::default()], // Start with global scope
            structs: HashMap::new(),
            enums: HashMap::new(),
            interfaces: HashMap::new(),
            struct_names: HashMap::new(),
            enum_names: HashMap::new(),
            interface_names: HashMap::new(),
            impls: HashMap::new(),
            type_methods: HashMap::new(),
            next_struct_id: 0,
            next_enum_id: 0,
            next_interface_id: 0,
            current_return_type: None,
            loop_depth: 0,
        };

        // Register built-in type aliases
        env.define_type_alias("Int", Type::Int);
        env.define_type_alias("Float", Type::Float);
        env.define_type_alias("Bool", Type::Bool);
        env.define_type_alias("String", Type::String);
        env.define_type_alias("Null", Type::Null);

        env
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Get the current scope depth
    #[must_use]
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Define a variable in the current scope
    pub fn define_var(&mut self, name: impl Into<String>, ty: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.into(), VarInfo { ty, mutable });
        }
    }

    /// Look up a variable by name (searches from innermost to outermost scope)
    #[must_use]
    pub fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.variables.get(name) {
                return Some(info);
            }
        }
        None
    }

    /// Check if a variable exists in the current scope (not parent scopes)
    #[must_use]
    pub fn var_exists_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map_or(false, |s| s.variables.contains_key(name))
    }

    /// Define a type alias in the current scope
    pub fn define_type_alias(&mut self, name: impl Into<String>, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.type_aliases.insert(name.into(), ty);
        }
    }

    /// Look up a type by name
    #[must_use]
    pub fn lookup_type(&self, name: &str) -> Option<Type> {
        // Check type aliases first
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.type_aliases.get(name) {
                return Some(ty.clone());
            }
        }

        // Check struct names
        if let Some(&id) = self.struct_names.get(name) {
            if let Some(info) = self.structs.get(&id) {
                return Some(Type::struct_type(id, &info.name, Vec::new()));
            }
        }

        // Check enum names
        if let Some(&id) = self.enum_names.get(name) {
            if let Some(info) = self.enums.get(&id) {
                return Some(Type::enum_type(id, &info.name, Vec::new()));
            }
        }

        None
    }

    /// Register a new struct definition
    pub fn define_struct(&mut self, info: StructInfo) -> StructId {
        let id = StructId(self.next_struct_id);
        self.next_struct_id += 1;

        self.struct_names.insert(info.name.clone(), id);
        self.structs.insert(id, info);

        id
    }

    /// Look up a struct by ID
    #[must_use]
    pub fn get_struct(&self, id: StructId) -> Option<&StructInfo> {
        self.structs.get(&id)
    }

    /// Look up a struct by name
    #[must_use]
    pub fn lookup_struct(&self, name: &str) -> Option<(StructId, &StructInfo)> {
        self.struct_names
            .get(name)
            .and_then(|&id| self.structs.get(&id).map(|info| (id, info)))
    }

    /// Register a new enum definition
    pub fn define_enum(&mut self, info: EnumInfo) -> EnumId {
        let id = EnumId(self.next_enum_id);
        self.next_enum_id += 1;

        self.enum_names.insert(info.name.clone(), id);
        self.enums.insert(id, info);

        id
    }

    /// Look up an enum by ID
    #[must_use]
    pub fn get_enum(&self, id: EnumId) -> Option<&EnumInfo> {
        self.enums.get(&id)
    }

    /// Look up an enum by name
    #[must_use]
    pub fn lookup_enum(&self, name: &str) -> Option<(EnumId, &EnumInfo)> {
        self.enum_names
            .get(name)
            .and_then(|&id| self.enums.get(&id).map(|info| (id, info)))
    }

    /// Register a new interface definition
    pub fn define_interface(&mut self, info: InterfaceInfo) -> InterfaceId {
        let id = InterfaceId(self.next_interface_id);
        self.next_interface_id += 1;

        self.interface_names.insert(info.name.clone(), id);
        self.interfaces.insert(id, info);

        id
    }

    /// Look up an interface by ID
    #[must_use]
    pub fn get_interface(&self, id: InterfaceId) -> Option<&InterfaceInfo> {
        self.interfaces.get(&id)
    }

    /// Look up an interface by name
    #[must_use]
    pub fn lookup_interface(&self, name: &str) -> Option<(InterfaceId, &InterfaceInfo)> {
        self.interface_names
            .get(name)
            .and_then(|&id| self.interfaces.get(&id).map(|info| (id, info)))
    }

    /// Register an impl block
    /// Returns false if this would be a duplicate impl
    pub fn register_impl(&mut self, info: ImplInfo) -> bool {
        let key = (info.target_type.clone(), info.interface_name.clone());

        // Check for duplicate
        if self.impls.contains_key(&key) {
            return false;
        }

        // Add methods to the type's method map
        let type_methods = self
            .type_methods
            .entry(info.target_type.clone())
            .or_default();

        for (method_name, method_info) in &info.methods {
            type_methods.insert(method_name.clone(), method_info.clone());
        }

        self.impls.insert(key, info);
        true
    }

    /// Check if an impl exists for a type and interface
    #[must_use]
    pub fn has_impl(&self, target_type: &str, interface_name: Option<&str>) -> bool {
        let key = (target_type.to_string(), interface_name.map(String::from));
        self.impls.contains_key(&key)
    }

    /// Get an impl by target type and interface
    #[must_use]
    pub fn get_impl(&self, target_type: &str, interface_name: Option<&str>) -> Option<&ImplInfo> {
        let key = (target_type.to_string(), interface_name.map(String::from));
        self.impls.get(&key)
    }

    /// Look up a method on a type (from impl blocks)
    #[must_use]
    pub fn lookup_method(&self, type_name: &str, method_name: &str) -> Option<&ImplMethodInfo> {
        self.type_methods
            .get(type_name)
            .and_then(|methods| methods.get(method_name))
    }

    /// Get all methods available on a type
    #[must_use]
    pub fn get_type_methods(&self, type_name: &str) -> Option<&HashMap<String, ImplMethodInfo>> {
        self.type_methods.get(type_name)
    }

    /// Set the current function's return type
    pub fn set_return_type(&mut self, ty: Option<Type>) {
        self.current_return_type = ty;
    }

    /// Get the current function's return type
    #[must_use]
    pub fn get_return_type(&self) -> Option<&Type> {
        self.current_return_type.as_ref()
    }

    /// Check if we're currently inside a function
    #[must_use]
    pub fn in_function(&self) -> bool {
        self.current_return_type.is_some()
    }

    /// Enter a loop
    pub fn enter_loop(&mut self) {
        self.loop_depth += 1;
    }

    /// Exit a loop
    pub fn exit_loop(&mut self) {
        if self.loop_depth > 0 {
            self.loop_depth -= 1;
        }
    }

    /// Check if we're currently inside a loop
    #[must_use]
    pub fn in_loop(&self) -> bool {
        self.loop_depth > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_management() {
        let mut env = TypeEnv::new();
        assert_eq!(env.scope_depth(), 1); // Global scope

        env.enter_scope();
        assert_eq!(env.scope_depth(), 2);

        env.enter_scope();
        assert_eq!(env.scope_depth(), 3);

        env.exit_scope();
        assert_eq!(env.scope_depth(), 2);

        env.exit_scope();
        assert_eq!(env.scope_depth(), 1);

        // Can't exit global scope
        env.exit_scope();
        assert_eq!(env.scope_depth(), 1);
    }

    #[test]
    fn test_variable_lookup() {
        let mut env = TypeEnv::new();

        // Define in global scope
        env.define_var("x", Type::Int, false);

        assert!(env.lookup_var("x").is_some());
        assert!(env.lookup_var("y").is_none());

        // Enter new scope and define shadowing variable
        env.enter_scope();
        env.define_var("x", Type::String, true);
        env.define_var("y", Type::Bool, false);

        // Should find the shadowing variable
        let x = env.lookup_var("x").unwrap();
        assert_eq!(x.ty, Type::String);
        assert!(x.mutable);

        // y only exists in inner scope
        assert!(env.lookup_var("y").is_some());

        // Exit scope
        env.exit_scope();

        // Now x should be the global one again
        let x = env.lookup_var("x").unwrap();
        assert_eq!(x.ty, Type::Int);
        assert!(!x.mutable);

        // y should no longer exist
        assert!(env.lookup_var("y").is_none());
    }

    #[test]
    fn test_builtin_types() {
        let env = TypeEnv::new();

        assert_eq!(env.lookup_type("Int"), Some(Type::Int));
        assert_eq!(env.lookup_type("Float"), Some(Type::Float));
        assert_eq!(env.lookup_type("Bool"), Some(Type::Bool));
        assert_eq!(env.lookup_type("String"), Some(Type::String));
        assert!(env.lookup_type("Unknown").is_none());
    }

    #[test]
    fn test_struct_registration() {
        let mut env = TypeEnv::new();

        let info = StructInfo {
            name: "Point".into(),
            type_params: Vec::new(),
            type_param_vars: Vec::new(),
            fields: {
                let mut f = HashMap::new();
                f.insert(
                    "x".into(),
                    FieldInfo {
                        ty: Type::Float,
                        public: true,
                    },
                );
                f.insert(
                    "y".into(),
                    FieldInfo {
                        ty: Type::Float,
                        public: true,
                    },
                );
                f
            },
            field_order: vec!["x".into(), "y".into()],
        };

        let id = env.define_struct(info);

        // Lookup by name
        let (found_id, found_info) = env.lookup_struct("Point").unwrap();
        assert_eq!(found_id, id);
        assert_eq!(found_info.name, "Point");
        assert_eq!(found_info.fields.len(), 2);

        // Type lookup
        let ty = env.lookup_type("Point").unwrap();
        assert!(matches!(ty, Type::Struct { name, .. } if name == "Point"));
    }

    #[test]
    fn test_loop_tracking() {
        let mut env = TypeEnv::new();

        assert!(!env.in_loop());

        env.enter_loop();
        assert!(env.in_loop());

        env.enter_loop();
        assert!(env.in_loop());

        env.exit_loop();
        assert!(env.in_loop());

        env.exit_loop();
        assert!(!env.in_loop());
    }

    #[test]
    fn test_function_return_type() {
        let mut env = TypeEnv::new();

        assert!(!env.in_function());
        assert!(env.get_return_type().is_none());

        env.set_return_type(Some(Type::Int));
        assert!(env.in_function());
        assert_eq!(env.get_return_type(), Some(&Type::Int));

        env.set_return_type(None);
        assert!(!env.in_function());
    }
}
