//! JIT Runtime support functions
//!
//! This module provides runtime functions that JIT-compiled code calls into
//! for operations that are too complex to inline, such as:
//! - Reference counting (increment/decrement)
//! - Value allocation
//! - Type checking
//! - Error handling

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::bytecode::Value;

use super::types::ValueTag;

/// Runtime support for JIT-compiled code
///
/// The runtime maintains function pointers that can be called from native code
/// to perform operations that require interaction with the Rust runtime.
pub struct JitRuntime {
    /// Cache of compiled function pointers
    compiled_functions: HashMap<String, *const u8>,
}

impl JitRuntime {
    /// Create a new JIT runtime
    #[must_use]
    pub fn new() -> Self {
        Self {
            compiled_functions: HashMap::new(),
        }
    }

    /// Register a compiled function
    pub fn register_function(&mut self, name: String, ptr: *const u8) {
        self.compiled_functions.insert(name, ptr);
    }

    /// Get a compiled function pointer by name
    #[must_use]
    pub fn get_function(&self, name: &str) -> Option<*const u8> {
        self.compiled_functions.get(name).copied()
    }
}

impl Default for JitRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Runtime Helper Functions (called from JIT code)
// =============================================================================
//
// These functions are called from JIT-compiled code via function pointers.
// They use C calling convention for compatibility with Cranelift.

/// Packed value representation for FFI
///
/// This matches the layout expected by JIT code:
/// - First 8 bytes: tag (u8) + padding (7 bytes)
/// - Second 8 bytes: data (u64)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PackedValue {
    /// Tag + padding packed as u64
    pub tag_padded: u64,
    /// Data portion
    pub data: u64,
}

impl PackedValue {
    /// Get the tag from the packed representation
    #[must_use]
    pub const fn tag(&self) -> u8 {
        (self.tag_padded & 0xFF) as u8
    }

    /// Create a null value
    #[must_use]
    pub const fn null() -> Self {
        Self {
            tag_padded: ValueTag::Null as u64,
            data: 0,
        }
    }

    /// Create an integer value
    #[must_use]
    pub const fn int(value: i64) -> Self {
        Self {
            tag_padded: ValueTag::Int as u64,
            data: value as u64,
        }
    }

    /// Create a float value
    #[must_use]
    pub fn float(value: f64) -> Self {
        Self {
            tag_padded: ValueTag::Float as u64,
            data: value.to_bits(),
        }
    }

    /// Create a bool value
    #[must_use]
    pub const fn bool(value: bool) -> Self {
        Self {
            tag_padded: ValueTag::Bool as u64,
            data: value as u64,
        }
    }

    /// Convert to an integer (panics if not an Int)
    #[must_use]
    pub fn as_int(&self) -> i64 {
        debug_assert_eq!(self.tag(), ValueTag::Int as u8);
        self.data as i64
    }

    /// Convert to a float (panics if not a Float)
    #[must_use]
    pub fn as_float(&self) -> f64 {
        debug_assert_eq!(self.tag(), ValueTag::Float as u8);
        f64::from_bits(self.data)
    }

    /// Convert to a bool (panics if not a Bool)
    #[must_use]
    pub fn as_bool(&self) -> bool {
        debug_assert_eq!(self.tag(), ValueTag::Bool as u8);
        self.data != 0
    }
}

/// Convert a Stratum Value to PackedValue for JIT
///
/// # Safety
/// For reference types, this stores a raw pointer. The caller must ensure
/// the Value remains alive for the duration of JIT execution.
pub fn value_to_packed(value: &Value) -> PackedValue {
    match value {
        Value::Null => PackedValue::null(),
        Value::Bool(b) => PackedValue::bool(*b),
        Value::Int(i) => PackedValue::int(*i),
        Value::Float(f) => PackedValue::float(*f),
        Value::String(s) => PackedValue {
            tag_padded: ValueTag::String as u64,
            data: Rc::as_ptr(s) as u64,
        },
        Value::List(l) => PackedValue {
            tag_padded: ValueTag::List as u64,
            data: Rc::as_ptr(l) as u64,
        },
        Value::Map(m) => PackedValue {
            tag_padded: ValueTag::Map as u64,
            data: Rc::as_ptr(m) as u64,
        },
        Value::Closure(c) => PackedValue {
            tag_padded: ValueTag::Closure as u64,
            data: Rc::as_ptr(c) as u64,
        },
        Value::Function(f) => PackedValue {
            tag_padded: ValueTag::Function as u64,
            data: Rc::as_ptr(f) as u64,
        },
        // For other types, we store a boxed pointer
        _ => {
            let boxed = Box::new(value.clone());
            PackedValue {
                tag_padded: 0xFF, // Special tag for boxed values
                data: Box::into_raw(boxed) as u64,
            }
        }
    }
}

/// Convert a PackedValue back to a Stratum Value
///
/// # Safety
/// This function reconstructs Values from raw pointers. The pointers must
/// be valid and the reference counts must be correct.
pub unsafe fn packed_to_value(packed: PackedValue) -> Value {
    match packed.tag() {
        t if t == ValueTag::Null as u8 => Value::Null,
        t if t == ValueTag::Bool as u8 => Value::Bool(packed.data != 0),
        t if t == ValueTag::Int as u8 => Value::Int(packed.data as i64),
        t if t == ValueTag::Float as u8 => Value::Float(f64::from_bits(packed.data)),
        t if t == ValueTag::String as u8 => {
            let ptr = packed.data as *const String;
            // Increment ref count and return a clone
            Rc::increment_strong_count(ptr);
            Value::String(Rc::from_raw(ptr))
        }
        t if t == ValueTag::List as u8 => {
            let ptr = packed.data as *const RefCell<Vec<Value>>;
            Rc::increment_strong_count(ptr);
            Value::List(Rc::from_raw(ptr))
        }
        t if t == ValueTag::Map as u8 => {
            use crate::bytecode::HashableValue;
            let ptr = packed.data as *const RefCell<HashMap<HashableValue, Value>>;
            Rc::increment_strong_count(ptr);
            Value::Map(Rc::from_raw(ptr))
        }
        0xFF => {
            // Boxed fallback value
            let ptr = packed.data as *mut Value;
            *Box::from_raw(ptr)
        }
        _ => panic!("Unknown value tag: {}", packed.tag()),
    }
}

// =============================================================================
// Extern "C" functions callable from JIT code
// =============================================================================

/// Increment reference count for a reference-type value
///
/// # Safety
/// ptr must be a valid pointer to an Rc's inner data
#[no_mangle]
pub unsafe extern "C" fn stratum_rc_inc(ptr: *const ()) {
    if !ptr.is_null() {
        // Generic increment - we don't know the concrete type but
        // Rc's layout is consistent
        Rc::<()>::increment_strong_count(ptr);
    }
}

/// Decrement reference count for a reference-type value
///
/// # Safety
/// ptr must be a valid pointer to an Rc's inner data, and the
/// reference count must be at least 1.
#[no_mangle]
pub unsafe extern "C" fn stratum_rc_dec(ptr: *const (), tag: u8) {
    if ptr.is_null() {
        return;
    }

    // We need to know the type to properly drop
    // This is a simplification - in a full implementation we'd need
    // proper type-aware reference counting
    match tag {
        t if t == ValueTag::String as u8 => {
            let ptr = ptr as *const String;
            Rc::decrement_strong_count(ptr);
        }
        t if t == ValueTag::List as u8 => {
            let ptr = ptr as *const RefCell<Vec<Value>>;
            Rc::decrement_strong_count(ptr);
        }
        _ => {
            // For other types, we use a generic decrement
            // This may leak if the count goes to 0 and we can't drop properly
            Rc::<()>::decrement_strong_count(ptr);
        }
    }
}

/// Add two integer values
#[no_mangle]
pub extern "C" fn stratum_add_int(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

/// Add two float values
#[no_mangle]
pub extern "C" fn stratum_add_float(a: f64, b: f64) -> f64 {
    a + b
}

/// Concatenate two strings, returning a new Rc<String>
///
/// # Safety
/// Both pointers must be valid pointers to String data
#[no_mangle]
pub unsafe extern "C" fn stratum_concat_strings(a: *const String, b: *const String) -> *const String {
    let a_str = &*a;
    let b_str = &*b;
    let result = Rc::new(format!("{}{}", a_str, b_str));
    Rc::into_raw(result)
}

/// Create a new empty list
#[no_mangle]
pub extern "C" fn stratum_new_list() -> *const RefCell<Vec<Value>> {
    let list = Rc::new(RefCell::new(Vec::new()));
    Rc::into_raw(list)
}

/// Get the length of a list
///
/// # Safety
/// ptr must be a valid pointer to a list's RefCell
#[no_mangle]
pub unsafe extern "C" fn stratum_list_len(ptr: *const RefCell<Vec<Value>>) -> i64 {
    let list = &*ptr;
    list.borrow().len() as i64
}

/// Check if a value is truthy
#[no_mangle]
pub extern "C" fn stratum_is_truthy(tag: u8, data: u64) -> bool {
    match tag {
        t if t == ValueTag::Null as u8 => false,
        t if t == ValueTag::Bool as u8 => data != 0,
        _ => true, // All other values are truthy
    }
}

/// Print a value (for debugging)
#[no_mangle]
pub extern "C" fn stratum_print_int(value: i64) {
    println!("{}", value);
}

/// Print a float value
#[no_mangle]
pub extern "C" fn stratum_print_float(value: f64) {
    println!("{}", value);
}

/// Print a bool value
#[no_mangle]
pub extern "C" fn stratum_print_bool(value: bool) {
    println!("{}", value);
}

/// Throw a runtime error
#[no_mangle]
pub extern "C" fn stratum_runtime_error(msg_ptr: *const u8, msg_len: usize) -> ! {
    let msg = if msg_ptr.is_null() {
        "unknown error".to_string()
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(msg_ptr, msg_len);
            String::from_utf8_lossy(slice).to_string()
        }
    };
    panic!("Stratum runtime error: {}", msg);
}

/// Call a function by looking up its pointer in the JIT context
/// This is used for JIT-to-JIT calls when we know the function is compiled
///
/// Arguments:
/// - func_ptr: Pointer to the compiled function
/// - arg_count: Number of arguments
/// - args: Pointer to array of PackedValue arguments
///
/// Returns: PackedValue (as two u64s via pointer)
#[no_mangle]
pub unsafe extern "C" fn stratum_call_jit_direct(
    func_ptr: *const u8,
    arity: u8,
    args_ptr: *const PackedValue,
    result_tag: *mut u64,
    result_data: *mut u64,
) {
    let args = std::slice::from_raw_parts(args_ptr, arity as usize);
    let args_vec: Vec<Value> = args.iter().map(|p| packed_to_value(*p)).collect();

    let func = CompiledFunction {
        ptr: func_ptr,
        arity,
        name: String::new(),
    };

    let result = call_jit_function(&func, &args_vec);
    let packed = value_to_packed(&result);
    *result_tag = packed.tag_padded;
    *result_data = packed.data;
}

// =============================================================================
// Mixed-Mode Call Support
// =============================================================================
//
// These types and functions support calling between JIT and interpreted code.

/// FFI-safe return type for JIT functions returning packed values
///
/// Tuples are not FFI-safe, so we use this repr(C) struct instead.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReturnPair {
    /// Tag + padding
    pub tag: u64,
    /// Data
    pub data: u64,
}

/// Function pointer type for JIT-compiled functions
/// Takes (arg_count, args_ptr) and returns a PackedValue
#[allow(dead_code)]
pub type JitFunctionPtr = extern "C" fn(u8, *const PackedValue) -> PackedValue;

/// Compiled function entry for the JIT cache
#[derive(Clone)]
pub struct CompiledFunction {
    /// The function pointer
    pub ptr: *const u8,
    /// Number of parameters
    pub arity: u8,
    /// Function name (for debugging)
    pub name: String,
}

// SAFETY: Function pointers are immutable and can be shared across threads
unsafe impl Send for CompiledFunction {}
unsafe impl Sync for CompiledFunction {}

/// Context for JIT execution, holding compiled function cache
pub struct JitContext {
    /// Cache of compiled functions by name
    functions: HashMap<String, CompiledFunction>,
}

impl JitContext {
    /// Create a new JIT context
    #[must_use]
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Register a compiled function
    pub fn register(&mut self, name: String, ptr: *const u8, arity: u8) {
        self.functions.insert(name.clone(), CompiledFunction {
            ptr,
            arity,
            name,
        });
    }

    /// Get a compiled function by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CompiledFunction> {
        self.functions.get(name)
    }

    /// Check if a function is compiled
    #[must_use]
    pub fn is_compiled(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }
}

impl Default for JitContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Call a JIT-compiled function with packed value arguments
///
/// This function is safe to call because it validates the arity and the function
/// pointer is guaranteed to be valid when obtained from `JitCompiler::compile_function`.
pub fn call_jit_function(
    func: &CompiledFunction,
    args: &[Value],
) -> Value {
    assert_eq!(args.len(), func.arity as usize, "Argument count mismatch");
    // SAFETY: The function pointer is valid because it comes from JitCompiler
    // and the argument count matches the arity.
    unsafe { call_jit_function_unsafe(func, args) }
}

/// Internal unsafe implementation of JIT function call
///
/// # Safety
/// The function pointer must be valid and the arguments must match the arity.
unsafe fn call_jit_function_unsafe(
    func: &CompiledFunction,
    args: &[Value],
) -> Value {
    // Convert arguments to packed values
    let packed_args: Vec<PackedValue> = args.iter().map(value_to_packed).collect();

    // Create the function pointer type
    // JIT functions take pairs of i64 (tag, data) for each argument
    // and return a pair of i64 (tag, data)

    match func.arity {
        0 => {
            type Fn0 = extern "C" fn() -> ReturnPair;
            let f: Fn0 = std::mem::transmute(func.ptr);
            let ret = f();
            packed_to_value(PackedValue { tag_padded: ret.tag, data: ret.data })
        }
        1 => {
            type Fn1 = extern "C" fn(u64, u64) -> ReturnPair;
            let f: Fn1 = std::mem::transmute(func.ptr);
            let ret = f(packed_args[0].tag_padded, packed_args[0].data);
            packed_to_value(PackedValue { tag_padded: ret.tag, data: ret.data })
        }
        2 => {
            type Fn2 = extern "C" fn(u64, u64, u64, u64) -> ReturnPair;
            let f: Fn2 = std::mem::transmute(func.ptr);
            let ret = f(
                packed_args[0].tag_padded, packed_args[0].data,
                packed_args[1].tag_padded, packed_args[1].data,
            );
            packed_to_value(PackedValue { tag_padded: ret.tag, data: ret.data })
        }
        3 => {
            type Fn3 = extern "C" fn(u64, u64, u64, u64, u64, u64) -> ReturnPair;
            let f: Fn3 = std::mem::transmute(func.ptr);
            let ret = f(
                packed_args[0].tag_padded, packed_args[0].data,
                packed_args[1].tag_padded, packed_args[1].data,
                packed_args[2].tag_padded, packed_args[2].data,
            );
            packed_to_value(PackedValue { tag_padded: ret.tag, data: ret.data })
        }
        4 => {
            type Fn4 = extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> ReturnPair;
            let f: Fn4 = std::mem::transmute(func.ptr);
            let ret = f(
                packed_args[0].tag_padded, packed_args[0].data,
                packed_args[1].tag_padded, packed_args[1].data,
                packed_args[2].tag_padded, packed_args[2].data,
                packed_args[3].tag_padded, packed_args[3].data,
            );
            packed_to_value(PackedValue { tag_padded: ret.tag, data: ret.data })
        }
        _ => {
            // For functions with more arguments, we'd need a more general approach
            // For now, panic with a clear message
            panic!("JIT functions with more than 4 arguments not yet supported (got {})", func.arity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_value_int() {
        let packed = PackedValue::int(42);
        assert_eq!(packed.tag(), ValueTag::Int as u8);
        assert_eq!(packed.as_int(), 42);
    }

    #[test]
    fn packed_value_float() {
        let packed = PackedValue::float(3.14);
        assert_eq!(packed.tag(), ValueTag::Float as u8);
        assert!((packed.as_float() - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn packed_value_bool() {
        let packed = PackedValue::bool(true);
        assert_eq!(packed.tag(), ValueTag::Bool as u8);
        assert!(packed.as_bool());

        let packed_false = PackedValue::bool(false);
        assert!(!packed_false.as_bool());
    }

    #[test]
    fn value_roundtrip_primitives() {
        let values = vec![
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Int(42),
            Value::Int(-100),
            Value::Float(3.14159),
        ];

        for value in values {
            let packed = value_to_packed(&value);
            let restored = unsafe { packed_to_value(packed) };
            assert_eq!(value, restored);
        }
    }

    #[test]
    fn value_roundtrip_string() {
        let value = Value::string("hello world");
        let packed = value_to_packed(&value);
        let restored = unsafe { packed_to_value(packed) };
        assert_eq!(value, restored);
    }

    #[test]
    fn runtime_helpers() {
        assert_eq!(stratum_add_int(10, 20), 30);
        assert!((stratum_add_float(1.5, 2.5) - 4.0).abs() < f64::EPSILON);
        assert!(stratum_is_truthy(ValueTag::Int as u8, 0));
        assert!(!stratum_is_truthy(ValueTag::Null as u8, 0));
    }
}
