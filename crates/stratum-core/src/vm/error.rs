//! Runtime errors for the Stratum virtual machine

use std::fmt;

use crate::bytecode::Value;

/// A runtime error that occurred during VM execution
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// The kind of error
    pub kind: RuntimeErrorKind,

    /// Stack trace at the point of error
    pub stack_trace: Vec<StackFrame>,
}

impl RuntimeError {
    /// Create a new runtime error
    pub fn new(kind: RuntimeErrorKind) -> Self {
        Self {
            kind,
            stack_trace: Vec::new(),
        }
    }

    /// Add a stack frame to the trace
    pub fn with_frame(mut self, frame: StackFrame) -> Self {
        self.stack_trace.push(frame);
        self
    }

    /// Add stack trace frames
    pub fn with_trace(mut self, trace: Vec<StackFrame>) -> Self {
        self.stack_trace = trace;
        self
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RuntimeError: {}", self.kind)?;
        if !self.stack_trace.is_empty() {
            writeln!(f, "Stack trace:")?;
            for frame in &self.stack_trace {
                writeln!(f, "  at {} (line {})", frame.function_name, frame.line)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {}

/// A stack frame in a stack trace
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// The function name
    pub function_name: String,

    /// The source line number
    pub line: u32,

    /// The source file name (if available)
    pub source: Option<String>,
}

impl StackFrame {
    /// Create a new stack frame
    pub fn new(function_name: String, line: u32) -> Self {
        Self {
            function_name,
            line,
            source: None,
        }
    }

    /// Create a stack frame with source info
    pub fn with_source(function_name: String, line: u32, source: String) -> Self {
        Self {
            function_name,
            line,
            source: Some(source),
        }
    }
}

/// The kind of runtime error
#[derive(Debug, Clone)]
pub enum RuntimeErrorKind {
    /// Type mismatch in an operation
    TypeError {
        expected: &'static str,
        got: &'static str,
        operation: &'static str,
    },

    /// Division by zero
    DivisionByZero,

    /// Undefined variable
    UndefinedVariable(String),

    /// Undefined field on struct/object
    UndefinedField {
        type_name: String,
        field: String,
    },

    /// Index out of bounds
    IndexOutOfBounds {
        index: i64,
        length: usize,
    },

    /// Invalid index type
    InvalidIndexType {
        got: &'static str,
    },

    /// Value is not callable
    NotCallable(&'static str),

    /// Wrong number of arguments
    ArityMismatch {
        expected: u8,
        got: u8,
    },

    /// Value is not iterable
    NotIterable(&'static str),

    /// Stack underflow (internal error)
    StackUnderflow,

    /// Stack overflow
    StackOverflow,

    /// Invalid opcode
    InvalidOpcode(u8),

    /// Uncaught exception
    UncaughtException(Value),

    /// User-thrown error with message
    UserError(String),

    /// Assertion failed
    AssertionFailed(Option<String>),

    /// Invalid operation
    InvalidOperation(String),

    /// Key not found in map
    KeyNotFound(String),

    /// Cannot use value as map key
    UnhashableType(&'static str),

    /// Null pointer dereference
    NullReference,

    /// Break/continue outside loop (should be caught at compile time)
    BreakOutsideLoop,

    /// Return outside function (should be caught at compile time)
    ReturnOutsideFunction,

    /// Await outside async function
    AwaitOutsideAsync,

    /// Internal VM error
    Internal(String),
}

impl fmt::Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeErrorKind::TypeError {
                expected,
                got,
                operation,
            } => {
                write!(
                    f,
                    "type error: {operation} expected {expected}, got {got}"
                )
            }
            RuntimeErrorKind::DivisionByZero => write!(f, "division by zero"),
            RuntimeErrorKind::UndefinedVariable(name) => {
                write!(f, "undefined variable '{name}'")
            }
            RuntimeErrorKind::UndefinedField { type_name, field } => {
                write!(f, "undefined field '{field}' on type {type_name}")
            }
            RuntimeErrorKind::IndexOutOfBounds { index, length } => {
                write!(f, "index {index} out of bounds for length {length}")
            }
            RuntimeErrorKind::InvalidIndexType { got } => {
                write!(f, "cannot index with type {got}")
            }
            RuntimeErrorKind::NotCallable(type_name) => {
                write!(f, "{type_name} is not callable")
            }
            RuntimeErrorKind::ArityMismatch { expected, got } => {
                write!(f, "expected {expected} arguments, got {got}")
            }
            RuntimeErrorKind::NotIterable(type_name) => {
                write!(f, "{type_name} is not iterable")
            }
            RuntimeErrorKind::StackUnderflow => write!(f, "stack underflow"),
            RuntimeErrorKind::StackOverflow => write!(f, "stack overflow"),
            RuntimeErrorKind::InvalidOpcode(op) => write!(f, "invalid opcode: {op}"),
            RuntimeErrorKind::UncaughtException(value) => {
                write!(f, "uncaught exception: {value}")
            }
            RuntimeErrorKind::UserError(msg) => write!(f, "{msg}"),
            RuntimeErrorKind::AssertionFailed(msg) => {
                if let Some(m) = msg {
                    write!(f, "assertion failed: {m}")
                } else {
                    write!(f, "assertion failed")
                }
            }
            RuntimeErrorKind::InvalidOperation(msg) => write!(f, "{msg}"),
            RuntimeErrorKind::KeyNotFound(key) => write!(f, "key not found: {key}"),
            RuntimeErrorKind::UnhashableType(type_name) => {
                write!(f, "{type_name} cannot be used as a map key")
            }
            RuntimeErrorKind::NullReference => {
                write!(f, "cannot access property of null")
            }
            RuntimeErrorKind::BreakOutsideLoop => write!(f, "break outside of loop"),
            RuntimeErrorKind::ReturnOutsideFunction => {
                write!(f, "return outside of function")
            }
            RuntimeErrorKind::AwaitOutsideAsync => {
                write!(f, "await outside of async function")
            }
            RuntimeErrorKind::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

/// Result type for VM operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;
