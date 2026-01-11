//! Callback system for bridging Stratum closures to GUI events
//!
//! This module provides the infrastructure for registering Stratum closures
//! as event handlers and executing them when GUI events occur.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use stratum_core::bytecode::Value;
use stratum_core::VM;

use crate::error::{GuiError, GuiResult};
use crate::state::ReactiveState;

/// Unique identifier for a registered callback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackId(u64);

impl CallbackId {
    /// Create a new callback ID from a raw value
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// A registered callback that can be invoked when events occur
#[derive(Clone)]
pub struct Callback {
    /// The Stratum closure or native function to invoke
    handler: Value,
    /// Optional description for debugging
    description: Option<String>,
}

impl Callback {
    /// Create a new callback from a Stratum value
    ///
    /// # Errors
    /// Returns an error if the value is not callable (Closure or NativeFunction)
    pub fn new(handler: Value) -> GuiResult<Self> {
        match &handler {
            Value::Closure(_) | Value::NativeFunction(_) => Ok(Self {
                handler,
                description: None,
            }),
            other => Err(GuiError::EventHandling(format!(
                "Expected callable, got {}",
                other.type_name()
            ))),
        }
    }

    /// Create a callback with a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Get the underlying handler value
    #[must_use]
    pub fn handler(&self) -> &Value {
        &self.handler
    }

    /// Get the description if set
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callback")
            .field("handler_type", &self.handler.type_name())
            .field("description", &self.description)
            .finish()
    }
}

/// Registry for managing callbacks
///
/// This stores Stratum closures indexed by unique IDs, allowing the GUI
/// to dispatch events to Stratum code.
#[derive(Debug, Default)]
pub struct CallbackRegistry {
    callbacks: HashMap<CallbackId, Callback>,
    next_id: u64,
}

impl CallbackRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a callback and return its ID
    pub fn register(&mut self, callback: Callback) -> CallbackId {
        let id = CallbackId(self.next_id);
        self.next_id += 1;
        self.callbacks.insert(id, callback);
        id
    }

    /// Register a Stratum value as a callback
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn register_value(&mut self, handler: Value) -> GuiResult<CallbackId> {
        let callback = Callback::new(handler)?;
        Ok(self.register(callback))
    }

    /// Get a callback by ID
    #[must_use]
    pub fn get(&self, id: CallbackId) -> Option<&Callback> {
        self.callbacks.get(&id)
    }

    /// Remove a callback by ID
    pub fn unregister(&mut self, id: CallbackId) -> Option<Callback> {
        self.callbacks.remove(&id)
    }

    /// Check if a callback exists
    #[must_use]
    pub fn contains(&self, id: CallbackId) -> bool {
        self.callbacks.contains_key(&id)
    }

    /// Get the number of registered callbacks
    #[must_use]
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    /// Check if the registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// Clear all callbacks
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }
}

/// Executor for invoking Stratum callbacks
///
/// This wraps a VM instance and provides methods to execute callbacks
/// in response to GUI events.
pub struct CallbackExecutor {
    /// The VM instance for executing closures
    vm: Rc<RefCell<VM>>,
    /// The callback registry
    registry: Rc<RefCell<CallbackRegistry>>,
}

impl CallbackExecutor {
    /// Create a new executor with a VM and registry
    #[must_use]
    pub fn new(vm: Rc<RefCell<VM>>, registry: Rc<RefCell<CallbackRegistry>>) -> Self {
        Self { vm, registry }
    }

    /// Execute a callback by ID with the given arguments
    ///
    /// # Errors
    /// Returns an error if:
    /// - The callback ID is not found
    /// - The callback execution fails
    pub fn execute(&self, id: CallbackId, args: Vec<Value>) -> GuiResult<Value> {
        let registry = self.registry.borrow();
        let callback = registry
            .get(id)
            .ok_or_else(|| GuiError::EventHandling(format!("Callback {:?} not found", id)))?;

        let handler = callback.handler().clone();
        drop(registry); // Release borrow before VM execution

        let mut vm = self.vm.borrow_mut();
        vm.invoke_callback(&handler, args)
            .map_err(|e| GuiError::EventHandling(format!("Callback execution failed: {}", e)))
    }

    /// Execute a callback with no arguments
    pub fn execute_no_args(&self, id: CallbackId) -> GuiResult<Value> {
        self.execute(id, Vec::new())
    }

    /// Execute a callback with the current state as the argument
    pub fn execute_with_state(&self, id: CallbackId, state: &ReactiveState) -> GuiResult<Value> {
        let state_value = state.get().clone();
        self.execute(id, vec![state_value])
    }

    /// Execute a closure directly (not via registered callback ID)
    ///
    /// This is used for dynamic closures like ForEach item templates.
    ///
    /// # Errors
    /// Returns an error if the closure execution fails
    pub fn execute_closure(&self, closure: &Value, args: Vec<Value>) -> GuiResult<Value> {
        let mut vm = self.vm.borrow_mut();
        vm.invoke_callback(closure, args)
            .map_err(|e| GuiError::EventHandling(format!("Closure execution failed: {}", e)))
    }

    /// Get a reference to the registry
    #[must_use]
    pub fn registry(&self) -> &Rc<RefCell<CallbackRegistry>> {
        &self.registry
    }

    /// Get a reference to the VM
    #[must_use]
    pub fn vm(&self) -> &Rc<RefCell<VM>> {
        &self.vm
    }
}

impl Clone for CallbackExecutor {
    fn clone(&self) -> Self {
        Self {
            vm: self.vm.clone(),
            registry: self.registry.clone(),
        }
    }
}

impl std::fmt::Debug for CallbackExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackExecutor")
            .field("registry_size", &self.registry.borrow().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_core::bytecode::NativeFunction;

    fn make_native_callback(f: fn(&[Value]) -> Result<Value, String>) -> Value {
        Value::NativeFunction(NativeFunction::new("test_callback", 0, f))
    }

    #[test]
    fn test_callback_id() {
        let id1 = CallbackId::new(1);
        let id2 = CallbackId::new(2);
        assert_ne!(id1, id2);
        assert_eq!(id1.raw(), 1);
    }

    #[test]
    fn test_callback_creation() {
        let handler = make_native_callback(|_| Ok(Value::Null));
        let callback = Callback::new(handler).unwrap();
        assert!(callback.description().is_none());

        let callback = callback.with_description("test callback");
        assert_eq!(callback.description(), Some("test callback"));
    }

    #[test]
    fn test_callback_creation_rejects_non_callable() {
        let result = Callback::new(Value::Int(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_basic_operations() {
        let mut registry = CallbackRegistry::new();
        assert!(registry.is_empty());

        let handler = make_native_callback(|_| Ok(Value::Null));
        let callback = Callback::new(handler).unwrap();
        let id = registry.register(callback);

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(id));
        assert!(registry.get(id).is_some());

        let removed = registry.unregister(id);
        assert!(removed.is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_unique_ids() {
        let mut registry = CallbackRegistry::new();

        let handler1 = make_native_callback(|_| Ok(Value::Int(1)));
        let handler2 = make_native_callback(|_| Ok(Value::Int(2)));

        let id1 = registry.register(Callback::new(handler1).unwrap());
        let id2 = registry.register(Callback::new(handler2).unwrap());

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_executor_creation() {
        let vm = Rc::new(RefCell::new(VM::new()));
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let executor = CallbackExecutor::new(vm, registry);

        assert!(executor.registry().borrow().is_empty());
    }

    #[test]
    fn test_executor_execute_native() {
        let vm = Rc::new(RefCell::new(VM::new()));
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let executor = CallbackExecutor::new(vm, registry.clone());

        // Register a native callback that returns 42
        let handler = make_native_callback(|_| Ok(Value::Int(42)));
        let id = registry
            .borrow_mut()
            .register(Callback::new(handler).unwrap());

        let result = executor.execute_no_args(id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_executor_missing_callback() {
        let vm = Rc::new(RefCell::new(VM::new()));
        let registry = Rc::new(RefCell::new(CallbackRegistry::new()));
        let executor = CallbackExecutor::new(vm, registry);

        let result = executor.execute_no_args(CallbackId::new(999));
        assert!(result.is_err());
    }
}
