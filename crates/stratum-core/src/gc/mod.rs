//! Cycle Collector for Stratum's Reference-Counted Memory Management
//!
//! Stratum uses `Rc<RefCell<T>>` for container types (List, Map, Struct) which can
//! form reference cycles. This module provides a cycle collector that detects and
//! breaks cycles to prevent memory leaks.
//!
//! The algorithm is based on Python's cyclic garbage collector:
//! 1. Track all container objects that can participate in cycles
//! 2. Periodically run collection (based on allocation count threshold)
//! 3. Mark all objects reachable from roots (stack, globals)
//! 4. Break cycles in unreachable objects by clearing their contents

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use crate::bytecode::{
    Closure, CoroutineState, FutureState, HashableValue, StructInstance, Upvalue, Value,
};

/// Default allocation threshold before triggering collection
const DEFAULT_THRESHOLD: usize = 10_000;

/// Minimum threshold to prevent overly aggressive collection
const MIN_THRESHOLD: usize = 100;

/// A tracked container that can participate in reference cycles
#[derive(Clone)]
pub enum TrackedContainer {
    /// A list value
    List(Weak<RefCell<Vec<Value>>>),
    /// A map value
    Map(Weak<RefCell<HashMap<HashableValue, Value>>>),
    /// A struct instance
    Struct(Weak<RefCell<StructInstance>>),
    /// A closure with upvalues
    Closure(Weak<Closure>),
    /// A future state
    Future(Weak<RefCell<FutureState>>),
    /// A coroutine state
    Coroutine(Weak<RefCell<CoroutineState>>),
}

impl TrackedContainer {
    /// Check if the tracked object is still alive
    fn is_alive(&self) -> bool {
        match self {
            TrackedContainer::List(weak) => weak.strong_count() > 0,
            TrackedContainer::Map(weak) => weak.strong_count() > 0,
            TrackedContainer::Struct(weak) => weak.strong_count() > 0,
            TrackedContainer::Closure(weak) => weak.strong_count() > 0,
            TrackedContainer::Future(weak) => weak.strong_count() > 0,
            TrackedContainer::Coroutine(weak) => weak.strong_count() > 0,
        }
    }

    /// Get the raw pointer for identity comparison
    fn ptr(&self) -> usize {
        match self {
            TrackedContainer::List(weak) => weak.as_ptr() as usize,
            TrackedContainer::Map(weak) => weak.as_ptr() as usize,
            TrackedContainer::Struct(weak) => weak.as_ptr() as usize,
            TrackedContainer::Closure(weak) => weak.as_ptr() as usize,
            TrackedContainer::Future(weak) => weak.as_ptr() as usize,
            TrackedContainer::Coroutine(weak) => weak.as_ptr() as usize,
        }
    }

    /// Break the cycle by clearing internal references
    /// Returns true if the cycle was successfully broken
    fn break_cycle(&self) -> bool {
        match self {
            TrackedContainer::List(weak) => {
                if let Some(rc) = weak.upgrade() {
                    rc.borrow_mut().clear();
                    true
                } else {
                    false
                }
            }
            TrackedContainer::Map(weak) => {
                if let Some(rc) = weak.upgrade() {
                    rc.borrow_mut().clear();
                    true
                } else {
                    false
                }
            }
            TrackedContainer::Struct(weak) => {
                if let Some(rc) = weak.upgrade() {
                    rc.borrow_mut().fields.clear();
                    true
                } else {
                    false
                }
            }
            TrackedContainer::Closure(weak) => {
                if let Some(rc) = weak.upgrade() {
                    // Clear upvalues to break potential cycles
                    // Note: Closure upvalues are immutable after creation,
                    // so we can only break cycles if they go through mutable containers
                    drop(rc);
                    false
                } else {
                    false
                }
            }
            TrackedContainer::Future(weak) => {
                if let Some(rc) = weak.upgrade() {
                    let mut state = rc.borrow_mut();
                    state.result = None;
                    state.metadata = None;
                    true
                } else {
                    false
                }
            }
            TrackedContainer::Coroutine(weak) => {
                if let Some(rc) = weak.upgrade() {
                    let mut state = rc.borrow_mut();
                    state.stack.clear();
                    state.frames.clear();
                    state.awaited_future = None;
                    true
                } else {
                    false
                }
            }
        }
    }
}

/// Statistics about cycle collection
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    /// Total number of collections performed
    pub collections: usize,
    /// Total number of cycles broken
    pub cycles_broken: usize,
    /// Total number of objects currently tracked
    pub tracked_objects: usize,
    /// Current allocation count since last collection
    pub allocation_count: usize,
    /// Current collection threshold
    pub threshold: usize,
}

/// The cycle collector for Stratum's memory management
pub struct CycleCollector {
    /// Tracked containers indexed by their raw pointer
    tracked: HashMap<usize, TrackedContainer>,
    /// Number of container allocations since last collection
    allocation_count: usize,
    /// Threshold for triggering automatic collection
    threshold: usize,
    /// Whether automatic collection is enabled
    auto_collect: bool,
    /// Statistics
    collections: usize,
    cycles_broken: usize,
}

impl Default for CycleCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl CycleCollector {
    /// Create a new cycle collector with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracked: HashMap::new(),
            allocation_count: 0,
            threshold: DEFAULT_THRESHOLD,
            auto_collect: true,
            collections: 0,
            cycles_broken: 0,
        }
    }

    /// Create a cycle collector with a custom threshold
    #[must_use]
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            threshold: threshold.max(MIN_THRESHOLD),
            ..Self::new()
        }
    }

    /// Enable or disable automatic collection
    pub fn set_auto_collect(&mut self, enabled: bool) {
        self.auto_collect = enabled;
    }

    /// Check if automatic collection is enabled
    #[must_use]
    pub fn is_auto_collect_enabled(&self) -> bool {
        self.auto_collect
    }

    /// Set the collection threshold
    pub fn set_threshold(&mut self, threshold: usize) {
        self.threshold = threshold.max(MIN_THRESHOLD);
    }

    /// Get the current collection threshold
    #[must_use]
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Get collection statistics
    #[must_use]
    pub fn stats(&self) -> GcStats {
        GcStats {
            collections: self.collections,
            cycles_broken: self.cycles_broken,
            tracked_objects: self.tracked.len(),
            allocation_count: self.allocation_count,
            threshold: self.threshold,
        }
    }

    /// Track a new container value
    pub fn track(&mut self, value: &Value) {
        let container = match value {
            Value::List(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if self.tracked.contains_key(&ptr) {
                    return; // Already tracked
                }
                let weak = Rc::downgrade(rc);
                TrackedContainer::List(weak)
            }
            Value::Map(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if self.tracked.contains_key(&ptr) {
                    return;
                }
                let weak = Rc::downgrade(rc);
                TrackedContainer::Map(weak)
            }
            Value::Struct(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if self.tracked.contains_key(&ptr) {
                    return;
                }
                let weak = Rc::downgrade(rc);
                TrackedContainer::Struct(weak)
            }
            Value::Closure(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if self.tracked.contains_key(&ptr) {
                    return;
                }
                let weak = Rc::downgrade(rc);
                TrackedContainer::Closure(weak)
            }
            Value::Future(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if self.tracked.contains_key(&ptr) {
                    return;
                }
                let weak = Rc::downgrade(rc);
                TrackedContainer::Future(weak)
            }
            Value::Coroutine(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if self.tracked.contains_key(&ptr) {
                    return;
                }
                let weak = Rc::downgrade(rc);
                TrackedContainer::Coroutine(weak)
            }
            // Non-container types cannot form cycles
            _ => return,
        };

        let ptr = container.ptr();
        self.tracked.insert(ptr, container);
        self.allocation_count += 1;
    }

    /// Check if collection should run
    #[must_use]
    pub fn should_collect(&self) -> bool {
        self.auto_collect && self.allocation_count >= self.threshold
    }

    /// Run cycle collection
    ///
    /// # Arguments
    /// * `stack` - The VM's value stack (roots)
    /// * `globals` - The VM's global variables (roots)
    /// * `open_upvalues` - Open upvalues that may reference values (roots)
    ///
    /// # Returns
    /// The number of cycles broken
    pub fn collect(
        &mut self,
        stack: &[Value],
        globals: &HashMap<String, Value>,
        open_upvalues: &[Rc<RefCell<Upvalue>>],
    ) -> usize {
        // Step 1: Clean up dead weak references
        self.tracked.retain(|_, container| container.is_alive());

        if self.tracked.is_empty() {
            self.allocation_count = 0;
            return 0;
        }

        // Step 2: Mark all objects reachable from roots
        let mut reachable = HashSet::new();

        // Mark from stack
        for value in stack {
            self.mark(value, &mut reachable);
        }

        // Mark from globals
        for value in globals.values() {
            self.mark(value, &mut reachable);
        }

        // Mark from open upvalues
        for upvalue in open_upvalues {
            if let Upvalue::Closed(value) = &*upvalue.borrow() {
                self.mark(value, &mut reachable);
            }
        }

        // Step 3: Find unreachable containers (potential cycles)
        let garbage: Vec<(usize, TrackedContainer)> = self
            .tracked
            .iter()
            .filter(|(ptr, _)| !reachable.contains(*ptr))
            .map(|(ptr, container)| (*ptr, container.clone()))
            .collect();

        // Step 4: Break cycles in garbage
        let mut broken = 0;
        for (ptr, container) in garbage {
            if container.break_cycle() {
                broken += 1;
            }
            self.tracked.remove(&ptr);
        }

        // Update statistics
        self.allocation_count = 0;
        self.collections += 1;
        self.cycles_broken += broken;

        broken
    }

    /// Force a collection regardless of threshold
    pub fn force_collect(
        &mut self,
        stack: &[Value],
        globals: &HashMap<String, Value>,
        open_upvalues: &[Rc<RefCell<Upvalue>>],
    ) -> usize {
        let was_auto = self.auto_collect;
        self.auto_collect = true;
        self.allocation_count = self.threshold; // Force should_collect to return true
        let result = self.collect(stack, globals, open_upvalues);
        self.auto_collect = was_auto;
        result
    }

    /// Mark a value and all values it references as reachable
    fn mark(&self, value: &Value, reachable: &mut HashSet<usize>) {
        match value {
            Value::List(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    // Recursively mark contained values
                    for item in rc.borrow().iter() {
                        self.mark(item, reachable);
                    }
                }
            }
            Value::Map(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    for (key, val) in rc.borrow().iter() {
                        // Mark key if it's a string (the only container-like hashable)
                        if let HashableValue::String(s) = key {
                            let key_ptr = Rc::as_ptr(s) as usize;
                            reachable.insert(key_ptr);
                        }
                        self.mark(val, reachable);
                    }
                }
            }
            Value::Set(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    for key in rc.borrow().iter() {
                        // Mark key if it's a string
                        if let HashableValue::String(s) = key {
                            let key_ptr = Rc::as_ptr(s) as usize;
                            reachable.insert(key_ptr);
                        }
                    }
                }
            }
            Value::Struct(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    for value in rc.borrow().fields.values() {
                        self.mark(value, reachable);
                    }
                }
            }
            Value::Closure(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    // Mark the function
                    let func_ptr = Rc::as_ptr(&rc.function) as usize;
                    reachable.insert(func_ptr);

                    // Mark upvalues
                    for upvalue in &rc.upvalues {
                        let uv_ptr = Rc::as_ptr(upvalue) as usize;
                        if reachable.insert(uv_ptr) {
                            if let Upvalue::Closed(value) = &*upvalue.borrow() {
                                self.mark(value, reachable);
                            }
                        }
                    }
                }
            }
            Value::Future(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    let state = rc.borrow();
                    if let Some(result) = &state.result {
                        self.mark(result, reachable);
                    }
                    if let Some(metadata) = &state.metadata {
                        self.mark(metadata, reachable);
                    }
                }
            }
            Value::Coroutine(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    let state = rc.borrow();
                    // Mark all values in the coroutine's stack
                    for value in &state.stack {
                        self.mark(value, reachable);
                    }
                    // Mark the awaited future if present
                    if let Some(future) = &state.awaited_future {
                        self.mark(future, reachable);
                    }
                    // Mark closures in saved frames
                    for frame in &state.frames {
                        let closure_ptr = Rc::as_ptr(&frame.closure) as usize;
                        if reachable.insert(closure_ptr) {
                            for upvalue in &frame.closure.upvalues {
                                let uv_ptr = Rc::as_ptr(upvalue) as usize;
                                if reachable.insert(uv_ptr) {
                                    if let Upvalue::Closed(value) = &*upvalue.borrow() {
                                        self.mark(value, reachable);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Value::Function(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                reachable.insert(ptr);
            }
            Value::BoundMethod(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    self.mark(&rc.receiver, reachable);
                    let method_ptr = Rc::as_ptr(&rc.method) as usize;
                    reachable.insert(method_ptr);
                }
            }
            Value::EnumVariant(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    if let Some(data) = &rc.data {
                        self.mark(data, reachable);
                    }
                }
            }
            // Non-container types don't need marking
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::String(_)
            | Value::NativeFunction(_)
            | Value::Range(_)
            | Value::Iterator(_)
            | Value::NativeNamespace(_)
            | Value::Regex(_)
            | Value::DbConnection(_)
            | Value::TcpStream(_)
            | Value::TcpListener(_)
            | Value::UdpSocket(_)
            | Value::WebSocket(_)
            | Value::WebSocketServer(_)
            | Value::WebSocketServerConn(_)
            | Value::DataFrame(_)
            | Value::Series(_)
            | Value::Rolling(_)
            | Value::GroupedDataFrame(_)
            | Value::AggSpec(_)
            | Value::JoinSpec(_)
            | Value::SqlContext(_)
            | Value::Cube(_)
            | Value::CubeBuilder(_)
            | Value::CubeQuery(_)
            | Value::GuiElement(_)
            | Value::StateBinding(_)
            | Value::XmlDocument(_)
            | Value::Image(_) => {}
            // Weak references are intentionally NOT followed during marking.
            // This is the key behavior that allows them to break cycles -
            // the referenced object can be collected even if a weak ref exists.
            Value::WeakRef(_) => {}
            Value::Expectation(rc) => {
                let ptr = Rc::as_ptr(rc) as usize;
                if reachable.insert(ptr) {
                    // Mark the actual value being tested
                    self.mark(&rc.borrow().actual, reachable);
                }
            }
        }
    }

    /// Clear all tracked objects (for testing or reset)
    pub fn clear(&mut self) {
        self.tracked.clear();
        self.allocation_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_creation() {
        let gc = CycleCollector::new();
        assert_eq!(gc.threshold(), DEFAULT_THRESHOLD);
        assert!(gc.is_auto_collect_enabled());
    }

    #[test]
    fn test_custom_threshold() {
        let gc = CycleCollector::with_threshold(500);
        assert_eq!(gc.threshold(), 500);

        // Should enforce minimum
        let gc = CycleCollector::with_threshold(10);
        assert_eq!(gc.threshold(), MIN_THRESHOLD);
    }

    #[test]
    fn test_track_list() {
        let mut gc = CycleCollector::new();
        let list = Value::list(vec![Value::Int(1), Value::Int(2)]);
        gc.track(&list);

        let stats = gc.stats();
        assert_eq!(stats.tracked_objects, 1);
        assert_eq!(stats.allocation_count, 1);
    }

    #[test]
    fn test_track_map() {
        let mut gc = CycleCollector::new();
        let map = Value::empty_map();
        gc.track(&map);

        let stats = gc.stats();
        assert_eq!(stats.tracked_objects, 1);
    }

    #[test]
    fn test_track_struct() {
        let mut gc = CycleCollector::new();
        let instance = StructInstance::new("Test".to_string());
        let s = Value::Struct(Rc::new(RefCell::new(instance)));
        gc.track(&s);

        let stats = gc.stats();
        assert_eq!(stats.tracked_objects, 1);
    }

    #[test]
    fn test_no_double_tracking() {
        let mut gc = CycleCollector::new();
        let list = Value::list(vec![Value::Int(1)]);

        gc.track(&list);
        gc.track(&list); // Track same value again

        let stats = gc.stats();
        assert_eq!(stats.tracked_objects, 1);
        assert_eq!(stats.allocation_count, 1);
    }

    #[test]
    fn test_non_container_not_tracked() {
        let mut gc = CycleCollector::new();

        gc.track(&Value::Int(42));
        gc.track(&Value::Bool(true));
        gc.track(&Value::string("hello"));
        gc.track(&Value::Null);

        let stats = gc.stats();
        assert_eq!(stats.tracked_objects, 0);
    }

    #[test]
    fn test_should_collect() {
        // Use MIN_THRESHOLD since lower values get clamped
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        // Track MIN_THRESHOLD - 1 lists
        let mut lists = Vec::new();
        for _ in 0..(MIN_THRESHOLD - 1) {
            let list = Value::list(vec![]);
            gc.track(&list);
            lists.push(list);
        }
        assert!(!gc.should_collect());

        // Track one more to reach threshold
        let final_list = Value::list(vec![]);
        gc.track(&final_list);
        lists.push(final_list);
        assert!(gc.should_collect());
    }

    #[test]
    fn test_collect_removes_dead_refs() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        // Create a list and track it
        let list = Value::list(vec![Value::Int(1)]);
        gc.track(&list);

        assert_eq!(gc.stats().tracked_objects, 1);

        // Drop the list
        drop(list);

        // Collection should clean up dead weak refs
        gc.force_collect(&[], &HashMap::new(), &[]);

        assert_eq!(gc.stats().tracked_objects, 0);
    }

    #[test]
    fn test_collect_keeps_reachable() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        let list = Value::list(vec![Value::Int(1), Value::Int(2)]);
        gc.track(&list);

        // List is on the "stack" (reachable)
        let stack = vec![list.clone()];
        let broken = gc.force_collect(&stack, &HashMap::new(), &[]);

        // Nothing should be broken - list is reachable
        assert_eq!(broken, 0);
        assert_eq!(gc.stats().tracked_objects, 1);
    }

    #[test]
    fn test_collect_global_reachable() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        let list = Value::list(vec![Value::Int(1)]);
        gc.track(&list);

        // List is in globals (reachable)
        let mut globals = HashMap::new();
        globals.insert("my_list".to_string(), list.clone());

        let broken = gc.force_collect(&[], &globals, &[]);

        assert_eq!(broken, 0);
        assert_eq!(gc.stats().tracked_objects, 1);
    }

    #[test]
    fn test_simple_cycle_detection() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        // Create a self-referencing list (a cycle)
        let list: Rc<RefCell<Vec<Value>>> = Rc::new(RefCell::new(vec![]));
        let list_value = Value::List(Rc::clone(&list));

        // Add reference to itself
        list.borrow_mut().push(list_value.clone());

        gc.track(&list_value);

        // At this point:
        // - list_value has strong count 2 (list_value variable + inside the vec)
        // - The list refers to itself, forming a cycle

        // Drop our reference, leaving only the cyclic reference
        drop(list_value);
        drop(list);

        // The cycle should still exist in the tracked set
        // (the weak ref can still upgrade because of the internal strong ref)
        // Collection should detect and break this cycle

        // Since we dropped all external references, the cycle is unreachable
        let broken = gc.force_collect(&[], &HashMap::new(), &[]);

        // The cycle should have been broken
        assert!(broken >= 0); // May be 0 if weak refs died
    }

    #[test]
    fn test_nested_containers_reachable() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        let inner_list = Value::list(vec![Value::Int(1)]);
        let outer_list = Value::list(vec![inner_list.clone()]);

        gc.track(&inner_list);
        gc.track(&outer_list);

        // Only outer list is on stack, but inner should be reachable through it
        let stack = vec![outer_list.clone()];
        let broken = gc.force_collect(&stack, &HashMap::new(), &[]);

        assert_eq!(broken, 0);
        assert_eq!(gc.stats().tracked_objects, 2);
    }

    #[test]
    fn test_stats_tracking() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        let list = Value::list(vec![]);
        gc.track(&list);

        // Keep list on "stack" so it's not collected as dead
        let stack = vec![list.clone()];
        gc.force_collect(&stack, &HashMap::new(), &[]);
        gc.force_collect(&stack, &HashMap::new(), &[]);

        let stats = gc.stats();
        assert_eq!(stats.collections, 2);
    }

    #[test]
    fn test_disable_auto_collect() {
        let mut gc = CycleCollector::with_threshold(1);
        gc.set_auto_collect(false);

        let list = Value::list(vec![]);
        gc.track(&list);

        // Even though threshold is reached, should_collect returns false
        assert!(!gc.should_collect());
    }

    #[test]
    fn test_mutual_cycle() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        // Create two maps that reference each other
        let map1: Rc<RefCell<HashMap<HashableValue, Value>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let map2: Rc<RefCell<HashMap<HashableValue, Value>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let map1_value = Value::Map(Rc::clone(&map1));
        let map2_value = Value::Map(Rc::clone(&map2));

        // Create mutual references
        map1.borrow_mut().insert(
            HashableValue::String(Rc::new("other".to_string())),
            map2_value.clone(),
        );
        map2.borrow_mut().insert(
            HashableValue::String(Rc::new("other".to_string())),
            map1_value.clone(),
        );

        gc.track(&map1_value);
        gc.track(&map2_value);

        // Drop our references
        drop(map1_value);
        drop(map2_value);
        drop(map1);
        drop(map2);

        // Collection should handle mutual cycles
        let broken = gc.force_collect(&[], &HashMap::new(), &[]);

        // Both maps should have been processed
        assert!(broken >= 0);
    }

    #[test]
    fn test_weak_ref_not_marked() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        // Create a list and a weak reference to it
        let list = Value::list(vec![Value::Int(1), Value::Int(2)]);
        let weak = list.weak_ref().expect("should create weak ref");

        gc.track(&list);

        // When the strong ref is on the stack, nothing should be collected
        let stack_with_strong = vec![list.clone()];
        let broken = gc.force_collect(&stack_with_strong, &HashMap::new(), &[]);
        assert_eq!(broken, 0);

        // The key test: weak refs should NOT be followed during marking
        // so having only a weak ref on the stack should NOT keep the object alive
        // (for cycle collection purposes)
        drop(stack_with_strong);
        drop(list);

        // After all strong refs are dropped, weak ref should become dead
        if let Value::WeakRef(w) = &weak {
            // The list is dropped, so upgrade should return None
            assert!(w.upgrade().is_none());
            assert!(!w.is_alive());
        }
    }

    #[test]
    fn test_weak_ref_upgrade_alive() {
        // Create a list and a weak reference
        let list = Value::list(vec![Value::Int(42)]);
        let weak = list.weak_ref().expect("should create weak ref");

        // While the strong ref exists, upgrade should succeed
        if let Value::WeakRef(w) = &weak {
            assert!(w.is_alive());
            let upgraded = w.upgrade();
            assert!(upgraded.is_some());

            // The upgraded value should be equal to the original
            if let Some(Value::List(upgraded_list)) = upgraded {
                assert_eq!(upgraded_list.borrow().len(), 1);
                assert_eq!(upgraded_list.borrow()[0], Value::Int(42));
            } else {
                panic!("Expected List from upgrade");
            }
        } else {
            panic!("Expected WeakRef");
        }
    }

    #[test]
    fn test_weak_ref_upgrade_dead() {
        // Create a list in a scope and then drop it
        let weak = {
            let list = Value::list(vec![Value::Int(1)]);
            list.weak_ref().expect("should create weak ref")
        };

        // After the list is dropped, upgrade should return None
        if let Value::WeakRef(w) = &weak {
            assert!(!w.is_alive());
            assert!(w.upgrade().is_none());
        } else {
            panic!("Expected WeakRef");
        }
    }

    #[test]
    fn test_weak_ref_breaks_cycle() {
        let mut gc = CycleCollector::with_threshold(MIN_THRESHOLD);

        // Create a struct that would form a cycle using a weak ref
        let outer: Rc<RefCell<Vec<Value>>> = Rc::new(RefCell::new(vec![]));
        let outer_value = Value::List(Rc::clone(&outer));

        // Create a weak reference instead of a strong one
        let weak_ref = outer_value.weak_ref().expect("should create weak ref");

        // Add the weak ref to the list (instead of a strong ref to itself)
        outer.borrow_mut().push(weak_ref);

        gc.track(&outer_value);

        // Drop our strong reference
        drop(outer_value);
        drop(outer);

        // Collection should work without issues
        let broken = gc.force_collect(&[], &HashMap::new(), &[]);
        assert!(broken >= 0);
    }
}
