//! Type mapping between Stratum and Cranelift
//!
//! This module defines how Stratum runtime values are represented in native code
//! and provides the type mappings for Cranelift IR generation.

use cranelift_codegen::ir::{types, Type as ClifType};

/// Layout information for Stratum values in native code
///
/// Stratum uses a tagged union representation for values. The layout is:
///
/// ```text
/// struct Value {
///     tag: u8,        // Discriminant (0=Null, 1=Bool, 2=Int, 3=Float, 4+=ref types)
///     _pad: [u8; 7],  // Padding for alignment
///     data: u64,      // Either inline value or pointer to heap data
/// }
/// ```
///
/// Total size: 16 bytes, aligned to 8 bytes
#[derive(Debug, Clone, Copy)]
pub struct ValueLayout;

impl ValueLayout {
    /// Size of a Value in bytes
    pub const SIZE: u32 = 16;

    /// Alignment of a Value
    pub const ALIGN: u32 = 8;

    /// Offset of the tag field
    pub const TAG_OFFSET: u32 = 0;

    /// Offset of the data field
    pub const DATA_OFFSET: u32 = 8;
}

/// Value type tags matching the Stratum Value enum discriminants
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Null = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    String = 4,
    List = 5,
    Map = 6,
    Function = 7,
    Closure = 8,
    NativeFunction = 9,
    Struct = 10,
    EnumVariant = 11,
    Range = 12,
    Iterator = 13,
    BoundMethod = 14,
    NativeNamespace = 15,
    Regex = 16,
    DbConnection = 17,
    Future = 18,
    Coroutine = 19,
}

impl ValueTag {
    /// Returns true if this tag represents a reference-counted type
    #[must_use]
    pub const fn is_ref_counted(self) -> bool {
        matches!(
            self,
            ValueTag::String
                | ValueTag::List
                | ValueTag::Map
                | ValueTag::Function
                | ValueTag::Closure
                | ValueTag::Struct
                | ValueTag::EnumVariant
                | ValueTag::Range
                | ValueTag::Iterator
                | ValueTag::BoundMethod
                | ValueTag::Regex
                | ValueTag::DbConnection
                | ValueTag::Future
                | ValueTag::Coroutine
        )
    }

    /// Returns true if this tag represents a primitive type (stored inline)
    #[must_use]
    pub const fn is_primitive(self) -> bool {
        matches!(self, ValueTag::Null | ValueTag::Bool | ValueTag::Int | ValueTag::Float)
    }
}

/// Cranelift type mappings for Stratum types
#[derive(Debug, Clone, Copy)]
pub struct CraneliftTypes;

impl CraneliftTypes {
    /// The pointer type for the target platform
    pub const POINTER: ClifType = types::I64;

    /// Type for integer values
    pub const INT: ClifType = types::I64;

    /// Type for floating-point values
    pub const FLOAT: ClifType = types::F64;

    /// Type for boolean values (uses i8 for Cranelift compatibility)
    pub const BOOL: ClifType = types::I8;

    /// Type for value tags
    pub const TAG: ClifType = types::I8;

    /// Type for the inline data portion of a Value
    pub const DATA: ClifType = types::I64;

    /// Returns the Cranelift type for a function parameter/return
    /// All values are passed as 16-byte structs, but for simplicity
    /// we pass them as two I64 values (tag+padding, data)
    pub const VALUE_FIRST: ClifType = types::I64;
    pub const VALUE_SECOND: ClifType = types::I64;
}

/// Memory flags for Cranelift memory operations
pub mod mem_flags {
    use cranelift_codegen::ir::MemFlags;

    /// Flags for loading/storing values with known alignment
    pub fn aligned() -> MemFlags {
        MemFlags::trusted()
    }

    /// Flags for potentially unaligned access
    pub fn unaligned() -> MemFlags {
        MemFlags::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_layout_size() {
        // Value should be 16 bytes to match the Rust enum layout
        assert_eq!(ValueLayout::SIZE, 16);
    }

    #[test]
    fn value_tag_discriminants() {
        assert_eq!(ValueTag::Null as u8, 0);
        assert_eq!(ValueTag::Bool as u8, 1);
        assert_eq!(ValueTag::Int as u8, 2);
        assert_eq!(ValueTag::Float as u8, 3);
    }

    #[test]
    fn tag_classification() {
        assert!(ValueTag::Null.is_primitive());
        assert!(ValueTag::Bool.is_primitive());
        assert!(ValueTag::Int.is_primitive());
        assert!(ValueTag::Float.is_primitive());

        assert!(ValueTag::String.is_ref_counted());
        assert!(ValueTag::List.is_ref_counted());
        assert!(ValueTag::Closure.is_ref_counted());

        assert!(!ValueTag::NativeNamespace.is_ref_counted());
    }
}
