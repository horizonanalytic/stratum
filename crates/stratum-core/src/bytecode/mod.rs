//! Bytecode module for the Stratum virtual machine
//!
//! This module provides:
//! - `OpCode`: The bytecode instruction set
//! - `Value`: Runtime value representation
//! - `Chunk`: A sequence of bytecode instructions
//! - `Compiler`: AST to bytecode compilation
//! - Disassembler utilities for debugging

mod chunk;
mod compiler;
mod debug;
mod error;
mod opcode;
mod value;

pub use chunk::Chunk;
pub use compiler::Compiler;
pub use debug::{disassemble_chunk, disassemble_instruction, trace_instruction};
pub use error::{CompileError, CompileErrorKind, CompileResult};
pub use opcode::OpCode;
pub use value::{
    BoundMethod, Closure, CoroutineState, CoroutineStatus, DbConnection, DbConnectionKind,
    EnumVariantInstance, ExpectationState, Function, FutureState, FutureStatus, GuiValue,
    HashableValue, ImageWrapper, NativeFunction, Range, SavedCallFrame, SavedExceptionHandler,
    StructInstance, TcpListenerWrapper, TcpStreamWrapper, UdpSocketWrapper, Upvalue, Value,
    WeakRefValue, WebSocketServerConnWrapper, WebSocketServerWrapper, WebSocketWrapper,
    XmlDocumentWrapper,
};
