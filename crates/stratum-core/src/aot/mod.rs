//! AOT (Ahead-of-Time) Compilation module for Stratum
//!
//! This module provides ahead-of-time compilation of Stratum bytecode to native
//! executables using the Cranelift code generator.
//!
//! # Architecture
//!
//! The AOT compiler translates Stratum bytecode into Cranelift IR, which is then
//! compiled to an object file and linked into a standalone executable:
//!
//! ```text
//! Bytecode Chunk → AotCompiler → Cranelift IR → Object File → Linker → Executable
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let compiler = AotCompiler::new()?;
//! compiler.add_function(&function)?;
//! compiler.build("output.stratum")?;
//! ```

mod compiler;
mod linker;
mod runtime;

pub use compiler::{AotCompiler, AotResult};
pub use linker::{Linker, LinkerConfig};

use thiserror::Error;

/// Errors that can occur during AOT compilation
#[derive(Debug, Error)]
pub enum AotError {
    /// Cranelift compilation error
    #[error("Cranelift compilation error: {0}")]
    Cranelift(String),

    /// Unsupported bytecode instruction
    #[error("Unsupported bytecode instruction: {0}")]
    UnsupportedInstruction(String),

    /// Linking error
    #[error("Linking error: {0}")]
    LinkError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// No main function found
    #[error("No main function found in module")]
    NoMainFunction,

    /// Build error
    #[error("Build error: {0}")]
    BuildError(String),
}
