//! AOT Runtime support functions
//!
//! This module provides runtime functions that AOT-compiled code links against.
//! These are minimal implementations needed for standalone executables.

use std::cell::RefCell;
use std::rc::Rc;

use crate::bytecode::Value;
use crate::jit::types::ValueTag;

/// Packed value representation for AOT FFI
///
/// This matches the layout expected by compiled code:
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

    /// Create a bool value
    #[must_use]
    pub const fn bool(value: bool) -> Self {
        Self {
            tag_padded: ValueTag::Bool as u64,
            data: value as u64,
        }
    }
}

// =============================================================================
// Runtime Helper Functions for AOT binaries
// =============================================================================
//
// These functions are linked into AOT executables.

/// Add two integer values
#[no_mangle]
pub extern "C" fn stratum_aot_add_int(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

/// Subtract two integer values
#[no_mangle]
pub extern "C" fn stratum_aot_sub_int(a: i64, b: i64) -> i64 {
    a.wrapping_sub(b)
}

/// Multiply two integer values
#[no_mangle]
pub extern "C" fn stratum_aot_mul_int(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

/// Divide two integer values
#[no_mangle]
pub extern "C" fn stratum_aot_div_int(a: i64, b: i64) -> i64 {
    if b == 0 {
        stratum_aot_panic(b"Division by zero\0".as_ptr(), 16);
    }
    a / b
}

/// Add two float values
#[no_mangle]
pub extern "C" fn stratum_aot_add_float(a: f64, b: f64) -> f64 {
    a + b
}

/// Print an integer to stdout
#[no_mangle]
pub extern "C" fn stratum_aot_print_int(value: i64) {
    println!("{}", value);
}

/// Print a float to stdout
#[no_mangle]
pub extern "C" fn stratum_aot_print_float(value: f64) {
    println!("{}", value);
}

/// Print a boolean to stdout
#[no_mangle]
pub extern "C" fn stratum_aot_print_bool(value: bool) {
    println!("{}", value);
}

/// Print a string to stdout
///
/// # Safety
/// ptr must be a valid pointer to a String
#[no_mangle]
pub unsafe extern "C" fn stratum_aot_print_string(ptr: *const String) {
    if ptr.is_null() {
        println!("null");
    } else {
        println!("{}", &*ptr);
    }
}

/// Panic with a message
#[no_mangle]
pub extern "C" fn stratum_aot_panic(msg_ptr: *const u8, msg_len: usize) -> ! {
    let msg = if msg_ptr.is_null() || msg_len == 0 {
        "unknown error".to_string()
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(msg_ptr, msg_len);
            String::from_utf8_lossy(slice).to_string()
        }
    };
    panic!("Stratum runtime error: {}", msg);
}

/// Increment reference count
///
/// # Safety
/// ptr must be a valid pointer to an Rc's inner data
#[no_mangle]
pub unsafe extern "C" fn stratum_aot_rc_inc(ptr: *const ()) {
    if !ptr.is_null() {
        Rc::<()>::increment_strong_count(ptr);
    }
}

/// Decrement reference count
///
/// # Safety
/// ptr must be a valid pointer to an Rc's inner data
#[no_mangle]
pub unsafe extern "C" fn stratum_aot_rc_dec(ptr: *const (), tag: u8) {
    if ptr.is_null() {
        return;
    }

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
            Rc::<()>::decrement_strong_count(ptr);
        }
    }
}

/// Allocate a new string
#[no_mangle]
pub extern "C" fn stratum_aot_alloc_string(ptr: *const u8, len: usize) -> *const String {
    let s = if ptr.is_null() || len == 0 {
        String::new()
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(ptr, len);
            String::from_utf8_lossy(slice).to_string()
        }
    };
    Rc::into_raw(Rc::new(s))
}

/// Concatenate two strings
///
/// # Safety
/// Both pointers must be valid pointers to String data
#[no_mangle]
pub unsafe extern "C" fn stratum_aot_concat_strings(
    a: *const String,
    b: *const String,
) -> *const String {
    let a_str = if a.is_null() { "" } else { &*a };
    let b_str = if b.is_null() { "" } else { &*b };
    let result = Rc::new(format!("{}{}", a_str, b_str));
    Rc::into_raw(result)
}

/// Allocate a new empty list
#[no_mangle]
pub extern "C" fn stratum_aot_alloc_list() -> *const RefCell<Vec<Value>> {
    Rc::into_raw(Rc::new(RefCell::new(Vec::new())))
}

/// Get list length
///
/// # Safety
/// ptr must be a valid pointer to a list
#[no_mangle]
pub unsafe extern "C" fn stratum_aot_list_len(ptr: *const RefCell<Vec<Value>>) -> i64 {
    if ptr.is_null() {
        0
    } else {
        (*ptr).borrow().len() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_value() {
        let null = PackedValue::null();
        assert_eq!(null.tag(), ValueTag::Null as u8);

        let int = PackedValue::int(42);
        assert_eq!(int.tag(), ValueTag::Int as u8);
        assert_eq!(int.data as i64, 42);

        let b = PackedValue::bool(true);
        assert_eq!(b.tag(), ValueTag::Bool as u8);
        assert_eq!(b.data, 1);
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(stratum_aot_add_int(10, 32), 42);
        assert_eq!(stratum_aot_sub_int(50, 8), 42);
        assert_eq!(stratum_aot_mul_int(6, 7), 42);
        assert_eq!(stratum_aot_div_int(84, 2), 42);
    }
}
