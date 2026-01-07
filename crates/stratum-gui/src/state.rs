//! State binding and reactivity for Stratum GUI
//!
//! This module provides the bridge between Stratum's Value types and
//! iced's state management. It enables two-way binding where:
//! - UI widgets can read from Stratum state
//! - User interactions update Stratum state
//! - State changes trigger automatic re-renders

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use stratum_core::bytecode::Value;

/// A computed property that derives its value from other state fields.
///
/// Computed properties are lazily evaluated and cached. They are automatically
/// invalidated when any of their dependencies change.
#[derive(Debug, Clone)]
pub struct ComputedProperty {
    /// The name/key of this computed property
    pub name: String,
    /// Fields this property depends on
    pub dependencies: HashSet<String>,
    /// The computation function (a Stratum closure)
    pub compute_fn: Value,
    /// Cached result
    cached_value: Rc<RefCell<Option<Value>>>,
    /// Generation when the cache was last computed
    cached_generation: Rc<RefCell<u64>>,
}

impl ComputedProperty {
    /// Create a new computed property
    #[must_use]
    pub fn new(name: impl Into<String>, dependencies: Vec<String>, compute_fn: Value) -> Self {
        Self {
            name: name.into(),
            dependencies: dependencies.into_iter().collect(),
            compute_fn,
            cached_value: Rc::new(RefCell::new(None)),
            cached_generation: Rc::new(RefCell::new(0)),
        }
    }

    /// Check if the cached value is still valid for the given generation
    #[must_use]
    pub fn is_valid(&self, current_generation: u64) -> bool {
        *self.cached_generation.borrow() == current_generation && self.cached_value.borrow().is_some()
    }

    /// Get the cached value if valid
    #[must_use]
    pub fn get_cached(&self) -> Option<Value> {
        self.cached_value.borrow().clone()
    }

    /// Update the cache with a new value
    pub fn set_cached(&self, value: Value, generation: u64) {
        *self.cached_value.borrow_mut() = Some(value);
        *self.cached_generation.borrow_mut() = generation;
    }

    /// Invalidate the cache
    pub fn invalidate(&self) {
        *self.cached_value.borrow_mut() = None;
    }

    /// Check if this computed property depends on a given field
    #[must_use]
    pub fn depends_on(&self, field: &str) -> bool {
        self.dependencies.contains(field) || self.dependencies.iter().any(|d| field.starts_with(&format!("{d}.")))
    }
}

/// A reactive state container that wraps a Stratum Value.
///
/// This provides the bridge between Stratum's dynamic value system
/// and iced's typed state management.
#[derive(Debug, Clone)]
pub struct ReactiveState {
    /// The underlying Stratum value
    value: Rc<RefCell<Value>>,
    /// Generation counter for change detection
    generation: Rc<RefCell<u64>>,
    /// Set of changed field paths since last check
    dirty_fields: Rc<RefCell<HashSet<String>>>,
    /// Computed properties registry
    computed: Rc<RefCell<HashMap<String, ComputedProperty>>>,
}

impl ReactiveState {
    /// Create a new reactive state from a Stratum value
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            generation: Rc::new(RefCell::new(0)),
            dirty_fields: Rc::new(RefCell::new(HashSet::new())),
            computed: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Get the current value (read-only borrow)
    #[must_use]
    pub fn get(&self) -> std::cell::Ref<'_, Value> {
        self.value.borrow()
    }

    /// Update the value and increment generation
    pub fn set(&self, new_value: Value) {
        *self.value.borrow_mut() = new_value;
        *self.generation.borrow_mut() += 1;
        // Mark all fields as dirty when replacing entire value
        self.dirty_fields.borrow_mut().insert("*".to_string());
    }

    /// Get the current generation (for change detection)
    #[must_use]
    pub fn generation(&self) -> u64 {
        *self.generation.borrow()
    }

    /// Check if any changes have occurred since the given generation
    #[must_use]
    pub fn has_changed_since(&self, gen: u64) -> bool {
        self.generation() > gen
    }

    /// Update a field in a struct value
    pub fn update_field(&self, field: &str, new_value: Value) -> bool {
        let value = self.value.borrow();
        if let Value::Struct(struct_val) = &*value {
            let mut instance = struct_val.borrow_mut();
            if instance.fields.contains_key(field) {
                instance.fields.insert(field.to_string(), new_value);
                drop(instance);
                drop(value);
                *self.generation.borrow_mut() += 1;
                self.dirty_fields.borrow_mut().insert(field.to_string());
                // Invalidate computed properties that depend on this field
                self.invalidate_dependents(field);
                return true;
            }
        }
        false
    }

    /// Invalidate computed properties that depend on a given field
    fn invalidate_dependents(&self, field: &str) {
        let computed = self.computed.borrow();
        for prop in computed.values() {
            if prop.depends_on(field) {
                prop.invalidate();
            }
        }
    }

    /// Get a field from a struct value
    #[must_use]
    pub fn get_field(&self, field: &str) -> Option<Value> {
        let value = self.value.borrow();
        if let Value::Struct(struct_val) = &*value {
            let instance = struct_val.borrow();
            instance.fields.get(field).cloned()
        } else {
            None
        }
    }

    /// Get a nested field using dot notation (e.g., "user.profile.name")
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let mut current = self.get_field(parts[0])?;

        for part in &parts[1..] {
            current = match current {
                Value::Struct(struct_val) => {
                    let instance = struct_val.borrow();
                    instance.fields.get(*part).cloned()?
                }
                Value::Map(map_val) => {
                    let map = map_val.borrow();
                    let key = stratum_core::bytecode::HashableValue::String(
                        Rc::new((*part).to_string())
                    );
                    map.get(&key).cloned()?
                }
                _ => return None,
            };
        }

        Some(current)
    }

    /// Update a nested field using dot notation
    pub fn update_path(&self, path: &str, new_value: Value) -> bool {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return false;
        }

        if parts.len() == 1 {
            return self.update_field(parts[0], new_value);
        }

        // For nested paths, we need to navigate to the parent and update
        let parent_path = parts[..parts.len() - 1].join(".");
        let field_name = parts[parts.len() - 1];

        if let Some(parent) = self.get_path(&parent_path) {
            if let Value::Struct(struct_val) = parent {
                let mut instance = struct_val.borrow_mut();
                if instance.fields.contains_key(field_name) {
                    instance.fields.insert(field_name.to_string(), new_value);
                    *self.generation.borrow_mut() += 1;
                    self.dirty_fields.borrow_mut().insert(path.to_string());
                    return true;
                }
            }
        }

        false
    }

    /// Check if a specific field has changed since last clear
    #[must_use]
    pub fn is_field_dirty(&self, field: &str) -> bool {
        let dirty = self.dirty_fields.borrow();
        dirty.contains(field) || dirty.contains("*")
    }

    /// Clear all dirty field markers
    pub fn clear_dirty(&self) {
        self.dirty_fields.borrow_mut().clear();
    }

    /// Get all dirty field names
    #[must_use]
    pub fn dirty_fields(&self) -> Vec<String> {
        self.dirty_fields.borrow().iter().cloned().collect()
    }

    /// Get a list value as a Vec
    #[must_use]
    pub fn get_list(&self, field: &str) -> Option<Vec<Value>> {
        let value = self.get_field(field)?;
        if let Value::List(list) = value {
            Some(list.borrow().clone())
        } else {
            None
        }
    }

    /// Iterate over a list field with indices
    pub fn iter_list(&self, field: &str) -> Option<impl Iterator<Item = (usize, Value)>> {
        let list = self.get_list(field)?;
        Some(list.into_iter().enumerate())
    }

    /// Resolve a StateBinding value to its current value.
    ///
    /// If the value is a `Value::StateBinding(path)`, returns the value at that path.
    /// If not a StateBinding, returns the value as-is.
    ///
    /// # Example
    /// ```ignore
    /// // Given state with field "count" = 42
    /// let binding = Value::StateBinding("count".to_string());
    /// let resolved = state.resolve_binding(&binding);
    /// assert_eq!(resolved, Value::Int(42));
    /// ```
    #[must_use]
    pub fn resolve_binding(&self, value: &Value) -> Value {
        match value {
            Value::StateBinding(path) => {
                self.get_path(path).unwrap_or(Value::Null)
            }
            other => other.clone(),
        }
    }

    /// Check if a value is a StateBinding and extract its path if so
    #[must_use]
    pub fn binding_path(value: &Value) -> Option<&str> {
        match value {
            Value::StateBinding(path) => Some(path.as_str()),
            _ => None,
        }
    }

    /// Create a FieldBinding for the given path
    #[must_use]
    pub fn bind(&self, path: &str) -> FieldBinding {
        FieldBinding::new(self.clone(), path.to_string())
    }

    // ==================== Computed Properties ====================

    /// Register a computed property
    ///
    /// The computed property will automatically invalidate when any of its
    /// dependencies change. The computation function should be a Stratum closure.
    pub fn register_computed(&self, name: impl Into<String>, dependencies: Vec<String>, compute_fn: Value) {
        let name = name.into();
        let prop = ComputedProperty::new(name.clone(), dependencies, compute_fn);
        self.computed.borrow_mut().insert(name, prop);
    }

    /// Get a computed property by name
    ///
    /// Returns the cached value if valid, or the compute function for evaluation.
    /// The caller is responsible for executing the function and caching the result.
    #[must_use]
    pub fn get_computed(&self, name: &str) -> Option<ComputedPropertyAccess> {
        let computed = self.computed.borrow();
        let prop = computed.get(name)?;
        let generation = self.generation();

        if prop.is_valid(generation) {
            Some(ComputedPropertyAccess::Cached(prop.get_cached().unwrap()))
        } else {
            Some(ComputedPropertyAccess::NeedsCompute {
                compute_fn: prop.compute_fn.clone(),
                name: name.to_string(),
            })
        }
    }

    /// Cache a computed property value after evaluation
    pub fn cache_computed(&self, name: &str, value: Value) {
        let generation = self.generation();
        if let Some(prop) = self.computed.borrow().get(name) {
            prop.set_cached(value, generation);
        }
    }

    /// Check if a computed property exists
    #[must_use]
    pub fn has_computed(&self, name: &str) -> bool {
        self.computed.borrow().contains_key(name)
    }

    /// Get all computed property names
    #[must_use]
    pub fn computed_names(&self) -> Vec<String> {
        self.computed.borrow().keys().cloned().collect()
    }
}

/// Result of accessing a computed property
#[derive(Debug, Clone)]
pub enum ComputedPropertyAccess {
    /// The cached value is still valid
    Cached(Value),
    /// The value needs to be recomputed
    NeedsCompute {
        /// The computation function to execute
        compute_fn: Value,
        /// The name of the property (for caching the result)
        name: String,
    },
}

/// A binding to a specific field in a struct state.
///
/// This enables two-way binding syntax like `&state.field` in Stratum.
#[derive(Debug, Clone)]
pub struct FieldBinding {
    state: ReactiveState,
    field_path: String,
}

impl FieldBinding {
    /// Create a new field binding
    #[must_use]
    pub fn new(state: ReactiveState, field_path: String) -> Self {
        Self { state, field_path }
    }

    /// Get the current field value
    #[must_use]
    pub fn get(&self) -> Option<Value> {
        if self.field_path.contains('.') {
            self.state.get_path(&self.field_path)
        } else {
            self.state.get_field(&self.field_path)
        }
    }

    /// Set the field value
    pub fn set(&self, value: Value) -> bool {
        if self.field_path.contains('.') {
            self.state.update_path(&self.field_path, value)
        } else {
            self.state.update_field(&self.field_path, value)
        }
    }

    /// Get the field path
    #[must_use]
    pub fn path(&self) -> &str {
        &self.field_path
    }

    /// Check if the field is dirty
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.state.is_field_dirty(&self.field_path)
    }
}

/// Subscription to state changes for specific fields
#[derive(Debug)]
pub struct StateSubscription {
    /// Fields to watch
    watched_fields: HashSet<String>,
    /// Last seen generation
    last_generation: u64,
}

impl StateSubscription {
    /// Create a new subscription
    #[must_use]
    pub fn new() -> Self {
        Self {
            watched_fields: HashSet::new(),
            last_generation: 0,
        }
    }

    /// Watch a specific field
    pub fn watch(&mut self, field: impl Into<String>) {
        self.watched_fields.insert(field.into());
    }

    /// Watch multiple fields
    pub fn watch_all(&mut self, fields: impl IntoIterator<Item = impl Into<String>>) {
        for field in fields {
            self.watched_fields.insert(field.into());
        }
    }

    /// Check if any watched fields have changed
    pub fn has_updates(&mut self, state: &ReactiveState) -> bool {
        let current_gen = state.generation();
        if current_gen <= self.last_generation {
            return false;
        }

        // Check if any watched field is dirty
        let dirty = state.dirty_fields.borrow();
        let has_updates = dirty.contains("*")
            || self.watched_fields.iter().any(|f| dirty.contains(f));

        if has_updates {
            self.last_generation = current_gen;
        }

        has_updates
    }

    /// Reset the subscription
    pub fn reset(&mut self) {
        self.last_generation = 0;
    }
}

impl Default for StateSubscription {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use stratum_core::bytecode::StructInstance;

    fn create_struct(name: &str, fields: HashMap<String, Value>) -> Value {
        let mut instance = StructInstance::new(name.to_string());
        instance.fields = fields;
        Value::Struct(Rc::new(RefCell::new(instance)))
    }

    fn create_nested_struct() -> Value {
        let mut inner_fields = HashMap::new();
        inner_fields.insert("name".to_string(), Value::String(Rc::new("test".to_string())));
        inner_fields.insert("age".to_string(), Value::Int(25));

        let mut outer_fields = HashMap::new();
        outer_fields.insert("user".to_string(), create_struct("User", inner_fields));
        outer_fields.insert("count".to_string(), Value::Int(0));

        create_struct("AppState", outer_fields)
    }

    #[test]
    fn test_reactive_state_basic() {
        let state = ReactiveState::new(Value::Int(42));
        assert_eq!(*state.get(), Value::Int(42));
        assert_eq!(state.generation(), 0);

        state.set(Value::Int(100));
        assert_eq!(*state.get(), Value::Int(100));
        assert_eq!(state.generation(), 1);
    }

    #[test]
    fn test_reactive_state_struct_field() {
        let mut fields = HashMap::new();
        fields.insert("count".to_string(), Value::Int(0));
        fields.insert("name".to_string(), Value::String(Rc::new("test".to_string())));

        let state = ReactiveState::new(create_struct("AppState", fields));

        // Read field
        let count = state.get_field("count");
        assert_eq!(count, Some(Value::Int(0)));

        // Update field
        assert!(state.update_field("count", Value::Int(5)));
        assert_eq!(state.get_field("count"), Some(Value::Int(5)));
        assert_eq!(state.generation(), 1);
    }

    #[test]
    fn test_nested_path_access() {
        let state = ReactiveState::new(create_nested_struct());

        // Access nested field
        let name = state.get_path("user.name");
        assert_eq!(name, Some(Value::String(Rc::new("test".to_string()))));

        let age = state.get_path("user.age");
        assert_eq!(age, Some(Value::Int(25)));

        // Non-existent path
        assert!(state.get_path("user.nonexistent").is_none());
        assert!(state.get_path("nonexistent.path").is_none());
    }

    #[test]
    fn test_field_binding() {
        let mut fields = HashMap::new();
        fields.insert("value".to_string(), Value::Int(10));

        let state = ReactiveState::new(create_struct("State", fields));
        let binding = FieldBinding::new(state, "value".to_string());

        assert_eq!(binding.get(), Some(Value::Int(10)));
        assert!(binding.set(Value::Int(20)));
        assert_eq!(binding.get(), Some(Value::Int(20)));
    }

    #[test]
    fn test_dirty_tracking() {
        let mut fields = HashMap::new();
        fields.insert("a".to_string(), Value::Int(1));
        fields.insert("b".to_string(), Value::Int(2));

        let state = ReactiveState::new(create_struct("State", fields));

        assert!(!state.is_field_dirty("a"));
        assert!(!state.is_field_dirty("b"));

        state.update_field("a", Value::Int(10));

        assert!(state.is_field_dirty("a"));
        assert!(!state.is_field_dirty("b"));

        state.clear_dirty();
        assert!(!state.is_field_dirty("a"));
    }

    #[test]
    fn test_state_subscription() {
        let mut fields = HashMap::new();
        fields.insert("a".to_string(), Value::Int(1));
        fields.insert("b".to_string(), Value::Int(2));

        let state = ReactiveState::new(create_struct("State", fields));
        let mut sub = StateSubscription::new();
        sub.watch("a");

        // No updates initially
        assert!(!sub.has_updates(&state));

        // Update watched field
        state.update_field("a", Value::Int(10));
        assert!(sub.has_updates(&state));

        // Already processed
        assert!(!sub.has_updates(&state));

        // Update unwatched field - subscription still sees it due to generation
        state.update_field("b", Value::Int(20));
        state.clear_dirty();
        assert!(!sub.has_updates(&state));
    }

    #[test]
    fn test_has_changed_since() {
        let state = ReactiveState::new(Value::Int(0));
        let gen0 = state.generation();

        assert!(!state.has_changed_since(gen0));

        state.set(Value::Int(1));
        assert!(state.has_changed_since(gen0));
        assert!(!state.has_changed_since(state.generation()));
    }

    #[test]
    fn test_get_list() {
        let list = Value::List(Rc::new(RefCell::new(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ])));

        let mut fields = HashMap::new();
        fields.insert("items".to_string(), list);

        let state = ReactiveState::new(create_struct("State", fields));

        let items = state.get_list("items").unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], Value::Int(1));
    }

    #[test]
    fn test_resolve_binding() {
        let mut fields = HashMap::new();
        fields.insert("count".to_string(), Value::Int(42));
        fields.insert("name".to_string(), Value::string("test"));

        let state = ReactiveState::new(create_struct("State", fields));

        // Resolving a StateBinding returns the value at that path
        let binding = Value::StateBinding("count".to_string());
        let resolved = state.resolve_binding(&binding);
        assert_eq!(resolved, Value::Int(42));

        // Resolving a non-StateBinding returns the value as-is
        let direct = Value::Int(100);
        let resolved = state.resolve_binding(&direct);
        assert_eq!(resolved, Value::Int(100));

        // Resolving a non-existent path returns Null
        let bad_binding = Value::StateBinding("nonexistent".to_string());
        let resolved = state.resolve_binding(&bad_binding);
        assert_eq!(resolved, Value::Null);
    }

    #[test]
    fn test_binding_path() {
        let binding = Value::StateBinding("state.count".to_string());
        assert_eq!(ReactiveState::binding_path(&binding), Some("state.count"));

        let non_binding = Value::Int(42);
        assert_eq!(ReactiveState::binding_path(&non_binding), None);
    }

    #[test]
    fn test_bind_creates_field_binding() {
        let mut fields = HashMap::new();
        fields.insert("value".to_string(), Value::Int(0));

        let state = ReactiveState::new(create_struct("State", fields));
        let binding = state.bind("value");

        assert_eq!(binding.get(), Some(Value::Int(0)));

        binding.set(Value::Int(42));
        assert_eq!(binding.get(), Some(Value::Int(42)));
    }

    #[test]
    fn test_computed_property_registration() {
        let mut fields = HashMap::new();
        fields.insert("count".to_string(), Value::Int(10));

        let state = ReactiveState::new(create_struct("State", fields));

        // Register a computed property
        let compute_fn = Value::Null; // Placeholder - real closure would go here
        state.register_computed("doubled", vec!["count".to_string()], compute_fn);

        assert!(state.has_computed("doubled"));
        assert!(!state.has_computed("nonexistent"));

        let names = state.computed_names();
        assert!(names.contains(&"doubled".to_string()));
    }

    #[test]
    fn test_computed_property_invalidation() {
        let mut fields = HashMap::new();
        fields.insert("a".to_string(), Value::Int(1));
        fields.insert("b".to_string(), Value::Int(2));

        let state = ReactiveState::new(create_struct("State", fields));

        // Register a computed property that depends on "a"
        let compute_fn = Value::Null;
        state.register_computed("computed_a", vec!["a".to_string()], compute_fn.clone());

        // Cache a value
        state.cache_computed("computed_a", Value::Int(100));

        // Get computed - should return cached
        if let Some(ComputedPropertyAccess::Cached(val)) = state.get_computed("computed_a") {
            assert_eq!(val, Value::Int(100));
        } else {
            panic!("Expected cached value");
        }

        // Update field "a" - should invalidate the computed property
        state.update_field("a", Value::Int(5));

        // Now get_computed should indicate it needs recomputation
        if let Some(ComputedPropertyAccess::NeedsCompute { name, .. }) = state.get_computed("computed_a") {
            assert_eq!(name, "computed_a");
        } else {
            panic!("Expected NeedsCompute after dependency changed");
        }
    }
}
