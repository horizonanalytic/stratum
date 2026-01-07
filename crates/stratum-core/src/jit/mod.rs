//! JIT Compilation module for Stratum
//!
//! This module provides Just-In-Time compilation of Stratum bytecode to native
//! machine code using the Cranelift code generator.
//!
//! # Architecture
//!
//! The JIT compiler translates Stratum bytecode into Cranelift IR, which is then
//! compiled to native machine code. The compilation happens at the function level:
//!
//! ```text
//! Bytecode Chunk → CraneliftCompiler → Cranelift IR → Native Code → Function Pointer
//! ```
//!
//! # Value Representation
//!
//! Stratum values are represented as tagged pointers in native code:
//! - Primitive types (Int, Float, Bool) are stored inline
//! - Reference types (String, List, Map, etc.) are represented as pointers with RC
//!
//! # Calling Convention
//!
//! JIT-compiled functions use the system calling convention with:
//! - Arguments passed as `Value` pointers
//! - Return value as `Value`
//! - Runtime functions called for complex operations

mod compiler;
mod runtime;
pub mod types;

pub use compiler::JitCompiler;
pub use runtime::{
    call_jit_function, CompiledFunction, JitContext, JitRuntime, PackedValue,
    packed_to_value, value_to_packed,
};
pub use types::ValueLayout;

use thiserror::Error;

/// Errors that can occur during JIT compilation
#[derive(Debug, Error)]
pub enum JitError {
    /// Cranelift compilation error
    #[error("Cranelift compilation error: {0}")]
    Cranelift(String),

    /// Unsupported bytecode instruction
    #[error("Unsupported bytecode instruction: {0}")]
    UnsupportedInstruction(String),

    /// Type mismatch during compilation
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    /// Function not found
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Memory allocation error
    #[error("Memory allocation error: {0}")]
    MemoryError(String),

    /// Internal compiler error
    #[error("Internal JIT compiler error: {0}")]
    Internal(String),
}

/// Result type for JIT operations
pub type JitResult<T> = Result<T, JitError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jit_module_loads() {
        // Basic test that the module compiles
        let _ = JitCompiler::new();
    }
}
