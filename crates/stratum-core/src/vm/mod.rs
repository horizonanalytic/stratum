//! Virtual Machine for the Stratum programming language
//!
//! This module provides a stack-based bytecode interpreter that executes
//! compiled Stratum code.

mod error;
mod natives;

pub use error::{RuntimeError, RuntimeErrorKind, RuntimeResult, StackFrame};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::bytecode::{
    Chunk, Closure, EnumVariantInstance, Function, HashableValue, NativeFunction, OpCode, Range,
    StructInstance, Upvalue, Value,
};

/// Maximum call stack depth
const MAX_FRAMES: usize = 256;

/// Maximum value stack size
const MAX_STACK: usize = 65536;

/// A call frame on the call stack
#[derive(Clone)]
struct CallFrame {
    /// The closure being executed
    closure: Rc<Closure>,

    /// Instruction pointer (index into chunk code)
    ip: usize,

    /// Base of this frame's locals on the value stack
    /// (slot 0 is the function itself for methods, or first local)
    stack_base: usize,
}

impl CallFrame {
    fn new(closure: Rc<Closure>, stack_base: usize) -> Self {
        Self {
            closure,
            ip: 0,
            stack_base,
        }
    }

    #[inline]
    fn chunk(&self) -> &Chunk {
        &self.closure.function.chunk
    }
}

/// Exception handler on the handler stack
#[derive(Clone)]
struct ExceptionHandler {
    /// Frame index where the handler was registered
    frame_index: usize,

    /// Stack depth when handler was pushed
    stack_depth: usize,

    /// IP to jump to for catch block
    catch_ip: usize,

    /// IP to jump to for finally block (0 if none)
    finally_ip: usize,
}

/// The Stratum Virtual Machine
pub struct VM {
    /// Value stack
    stack: Vec<Value>,

    /// Call stack
    frames: Vec<CallFrame>,

    /// Global variables
    globals: HashMap<String, Value>,

    /// Open upvalues (variables that are captured but still on stack)
    open_upvalues: Vec<Rc<RefCell<Upvalue>>>,

    /// Exception handler stack
    handlers: Vec<ExceptionHandler>,

    /// Current exception being propagated (if any)
    current_exception: Option<Value>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    /// Create a new VM instance
    #[must_use]
    pub fn new() -> Self {
        let mut vm = Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(16),
            globals: HashMap::new(),
            open_upvalues: Vec::new(),
            handlers: Vec::new(),
            current_exception: None,
        };

        // Register built-in functions
        vm.register_natives();

        vm
    }

    /// Register native/built-in functions
    fn register_natives(&mut self) {
        // Print function
        self.define_native("print", -1, |args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{arg}");
            }
            Ok(Value::Null)
        });

        // Println function
        self.define_native("println", -1, |args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{arg}");
            }
            println!();
            Ok(Value::Null)
        });

        // Type inspection
        self.define_native("type_of", 1, |args| {
            Ok(Value::string(args[0].type_name()))
        });

        // Assertions
        self.define_native("assert", 1, |args| {
            if args[0].is_truthy() {
                Ok(Value::Null)
            } else {
                Err("assertion failed".to_string())
            }
        });

        self.define_native("assert_eq", 2, |args| {
            if args[0] == args[1] {
                Ok(Value::Null)
            } else {
                Err(format!(
                    "assertion failed: {:?} != {:?}",
                    args[0], args[1]
                ))
            }
        });

        // Length function (for strings, lists, maps)
        self.define_native("len", 1, |args| match &args[0] {
            Value::String(s) => Ok(Value::Int(s.len() as i64)),
            Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
            Value::Map(m) => Ok(Value::Int(m.borrow().len() as i64)),
            other => Err(format!("{} has no length", other.type_name())),
        });

        // String conversion
        self.define_native("str", 1, |args| {
            Ok(Value::string(format!("{}", args[0])))
        });

        // Int conversion
        self.define_native("int", 1, |args| match &args[0] {
            Value::Int(i) => Ok(Value::Int(*i)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::String(s) => s
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|_| format!("cannot convert '{}' to int", s)),
            Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
            other => Err(format!("cannot convert {} to int", other.type_name())),
        });

        // Float conversion
        self.define_native("float", 1, |args| match &args[0] {
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::String(s) => s
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| format!("cannot convert '{}' to float", s)),
            other => Err(format!("cannot convert {} to float", other.type_name())),
        });

        // Range function
        self.define_native("range", 2, |args| {
            let start = match &args[0] {
                Value::Int(i) => *i,
                other => return Err(format!("range start must be Int, got {}", other.type_name())),
            };
            let end = match &args[1] {
                Value::Int(i) => *i,
                other => return Err(format!("range end must be Int, got {}", other.type_name())),
            };
            Ok(Value::Range(Rc::new(Range::exclusive(start, end))))
        });

        // Register native namespace modules
        self.globals.insert("File".to_string(), Value::NativeNamespace("File"));
        self.globals.insert("Dir".to_string(), Value::NativeNamespace("Dir"));
        self.globals.insert("Path".to_string(), Value::NativeNamespace("Path"));
        self.globals.insert("Env".to_string(), Value::NativeNamespace("Env"));
        self.globals.insert("Args".to_string(), Value::NativeNamespace("Args"));
        self.globals.insert("Shell".to_string(), Value::NativeNamespace("Shell"));
        self.globals.insert("Http".to_string(), Value::NativeNamespace("Http"));

        // Data encoding modules
        self.globals.insert("Json".to_string(), Value::NativeNamespace("Json"));
        self.globals.insert("Toml".to_string(), Value::NativeNamespace("Toml"));
        self.globals.insert("Yaml".to_string(), Value::NativeNamespace("Yaml"));
        self.globals.insert("Base64".to_string(), Value::NativeNamespace("Base64"));
        self.globals.insert("Url".to_string(), Value::NativeNamespace("Url"));

        // DateTime and Time modules
        self.globals.insert("DateTime".to_string(), Value::NativeNamespace("DateTime"));
        self.globals.insert("Duration".to_string(), Value::NativeNamespace("Duration"));
        self.globals.insert("Time".to_string(), Value::NativeNamespace("Time"));

        // Regex module
        self.globals.insert("Regex".to_string(), Value::NativeNamespace("Regex"));

        // Hashing, UUID, and Random modules
        self.globals.insert("Hash".to_string(), Value::NativeNamespace("Hash"));
        self.globals.insert("Uuid".to_string(), Value::NativeNamespace("Uuid"));
        self.globals.insert("Random".to_string(), Value::NativeNamespace("Random"));

        // User Input module
        self.globals.insert("Input".to_string(), Value::NativeNamespace("Input"));

        // Logging module
        self.globals.insert("Log".to_string(), Value::NativeNamespace("Log"));

        // System info module
        self.globals.insert("System".to_string(), Value::NativeNamespace("System"));

        // Database module
        self.globals.insert("Db".to_string(), Value::NativeNamespace("Db"));
    }

    /// Define a native function
    fn define_native(
        &mut self,
        name: &'static str,
        arity: i8,
        function: fn(&[Value]) -> Result<Value, String>,
    ) {
        let native = NativeFunction::new(name, arity, function);
        self.globals
            .insert(name.to_string(), Value::NativeFunction(native));
    }

    /// Execute a compiled function
    pub fn run(&mut self, function: Rc<Function>) -> RuntimeResult<Value> {
        // Clear any leftover state from previous runs
        // (globals are preserved for REPL-style usage)
        self.stack.clear();
        self.frames.clear();
        self.open_upvalues.clear();
        self.handlers.clear();
        self.current_exception = None;

        // Wrap the function in a closure
        let closure = Rc::new(Closure::new(function));

        // Push the closure onto the stack (slot 0 of the frame)
        self.stack.push(Value::Closure(closure.clone()));

        // Create the initial frame
        self.frames.push(CallFrame::new(closure, 0));

        // Run the main execution loop
        self.execute()
    }

    /// Main execution loop
    fn execute(&mut self) -> RuntimeResult<Value> {
        loop {
            // Check for exception propagation
            if let Some(exception) = self.current_exception.take() {
                if !self.handle_exception(exception.clone())? {
                    // No handler found, propagate error
                    return Err(self.runtime_error(RuntimeErrorKind::UncaughtException(exception)));
                }
                continue;
            }

            // Get current instruction
            let frame = self.current_frame();
            let chunk = frame.chunk();

            if frame.ip >= chunk.len() {
                // End of bytecode reached
                let result = self.stack.pop().unwrap_or(Value::Null);
                return Ok(result);
            }

            let instruction = chunk
                .read_byte(frame.ip)
                .ok_or_else(|| self.runtime_error(RuntimeErrorKind::Internal("unexpected end of bytecode".to_string())))?;
            let opcode = OpCode::try_from(instruction)
                .map_err(|op| self.runtime_error(RuntimeErrorKind::InvalidOpcode(op)))?;

            // Advance IP past the opcode
            self.current_frame_mut().ip += 1;

            // Handle Return specially in main loop (not in execute_opcode)
            if opcode == OpCode::Return {
                let result = self.pop()?;

                // Close any upvalues in this frame
                let frame = &self.frames[self.frames.len() - 1];
                self.close_upvalues(frame.stack_base);

                // Pop the frame
                let frame = self.frames.pop().unwrap();

                // If this was the last frame, we're done
                if self.frames.is_empty() {
                    return Ok(result);
                }

                // Pop locals and the function itself
                self.stack.truncate(frame.stack_base);

                // Push the return value
                self.push(result)?;
                continue;
            }

            // Execute all other opcodes
            self.execute_opcode(opcode)?;
        }
    }

    // ===== Stack operations =====

    #[inline]
    fn push(&mut self, value: Value) -> RuntimeResult<()> {
        if self.stack.len() >= MAX_STACK {
            return Err(self.runtime_error(RuntimeErrorKind::StackOverflow));
        }
        self.stack.push(value);
        Ok(())
    }

    #[inline]
    fn pop(&mut self) -> RuntimeResult<Value> {
        self.stack
            .pop()
            .ok_or_else(|| self.runtime_error(RuntimeErrorKind::StackUnderflow))
    }

    #[inline]
    fn peek(&self, distance: usize) -> RuntimeResult<&Value> {
        self.stack
            .get(self.stack.len().saturating_sub(1 + distance))
            .ok_or_else(|| self.runtime_error(RuntimeErrorKind::StackUnderflow))
    }

    // ===== Frame operations =====

    #[inline]
    fn current_frame(&self) -> &CallFrame {
        &self.frames[self.frames.len() - 1]
    }

    #[inline]
    fn current_frame_mut(&mut self) -> &mut CallFrame {
        let len = self.frames.len();
        &mut self.frames[len - 1]
    }

    // ===== Bytecode reading =====

    fn read_u8(&mut self) -> u8 {
        let frame = self.current_frame();
        let byte = frame.chunk().read_byte(frame.ip).unwrap_or(0);
        self.current_frame_mut().ip += 1;
        byte
    }

    fn read_u16(&mut self) -> u16 {
        let frame = self.current_frame();
        let value = frame.chunk().read_u16(frame.ip).unwrap_or(0);
        self.current_frame_mut().ip += 2;
        value
    }

    fn read_i16(&mut self) -> i16 {
        let frame = self.current_frame();
        let value = frame.chunk().read_i16(frame.ip).unwrap_or(0);
        self.current_frame_mut().ip += 2;
        value
    }

    fn jump(&mut self, offset: i16) {
        let frame = self.current_frame_mut();
        frame.ip = (frame.ip as isize + offset as isize) as usize;
    }

    // ===== Constant pool access =====

    fn get_constant(&self, index: usize) -> &Value {
        self.current_frame()
            .chunk()
            .get_constant(index as u16)
            .expect("invalid constant index")
    }

    fn get_constant_string(&self, index: usize) -> RuntimeResult<String> {
        match self.get_constant(index) {
            Value::String(s) => Ok((**s).clone()),
            _ => Err(self.runtime_error(RuntimeErrorKind::Internal(
                "expected string constant".to_string(),
            ))),
        }
    }

    fn get_constant_function(&self, index: usize) -> RuntimeResult<Rc<Function>> {
        match self.get_constant(index) {
            Value::Function(f) => Ok(f.clone()),
            _ => Err(self.runtime_error(RuntimeErrorKind::Internal(
                "expected function constant".to_string(),
            ))),
        }
    }

    // ===== Local variables =====

    fn get_local(&self, slot: usize) -> &Value {
        let base = self.current_frame().stack_base;
        &self.stack[base + slot]
    }

    fn set_local(&mut self, slot: usize, value: Value) {
        let base = self.current_frame().stack_base;
        self.stack[base + slot] = value;
    }

    // ===== Upvalues =====

    fn capture_upvalue(&mut self, local_slot: usize) -> Rc<RefCell<Upvalue>> {
        let base = self.current_frame().stack_base;
        let stack_slot = base + local_slot;

        // Check if we already have an upvalue for this slot
        for upvalue in &self.open_upvalues {
            if let Upvalue::Open(slot) = *upvalue.borrow() {
                if slot == stack_slot {
                    return upvalue.clone();
                }
            }
        }

        // Create a new open upvalue
        let upvalue = Rc::new(RefCell::new(Upvalue::Open(stack_slot)));
        self.open_upvalues.push(upvalue.clone());
        upvalue
    }

    fn get_upvalue(&self, index: usize) -> RuntimeResult<Value> {
        let upvalue = &self.current_frame().closure.upvalues[index];
        let value = match &*upvalue.borrow() {
            Upvalue::Open(slot) => self.stack[*slot].clone(),
            Upvalue::Closed(value) => value.clone(),
        };
        Ok(value)
    }

    fn set_upvalue(&mut self, index: usize, value: Value) -> RuntimeResult<()> {
        let upvalue = self.current_frame().closure.upvalues[index].clone();
        match &mut *upvalue.borrow_mut() {
            Upvalue::Open(slot) => {
                self.stack[*slot] = value;
            }
            Upvalue::Closed(v) => {
                *v = value;
            }
        }
        Ok(())
    }

    fn close_upvalues(&mut self, from_slot: usize) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let should_close = {
                let upvalue = self.open_upvalues[i].borrow();
                if let Upvalue::Open(slot) = *upvalue {
                    slot >= from_slot
                } else {
                    false
                }
            };

            if should_close {
                let upvalue = self.open_upvalues.remove(i);
                let mut upvalue_mut = upvalue.borrow_mut();
                if let Upvalue::Open(slot) = *upvalue_mut {
                    *upvalue_mut = Upvalue::Closed(self.stack[slot].clone());
                }
            } else {
                i += 1;
            }
        }
    }

    // ===== Binary operations =====

    fn binary_op<F>(&mut self, f: F) -> RuntimeResult<()>
    where
        F: FnOnce(Value, Value) -> Result<Value, RuntimeErrorKind>,
    {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = f(left, right).map_err(|kind| self.runtime_error(kind))?;
        self.push(result)
    }

    fn numeric_binary_op<I, F>(
        &mut self,
        op_name: &'static str,
        int_op: I,
        float_op: F,
    ) -> RuntimeResult<()>
    where
        I: FnOnce(i64, i64) -> i64,
        F: FnOnce(f64, f64) -> f64,
    {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = match (&left, &right) {
            (Value::Int(x), Value::Int(y)) => Value::Int(int_op(*x, *y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(float_op(*x, *y)),
            (Value::Int(x), Value::Float(y)) => Value::Float(float_op(*x as f64, *y)),
            (Value::Float(x), Value::Int(y)) => Value::Float(float_op(*x, *y as f64)),
            _ => {
                return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "numeric",
                    got: left.type_name(),
                    operation: op_name,
                }));
            }
        };
        self.push(result)
    }

    fn comparison_op<I, F>(
        &mut self,
        op_name: &'static str,
        int_op: I,
        float_op: F,
    ) -> RuntimeResult<()>
    where
        I: FnOnce(i64, i64) -> bool,
        F: FnOnce(f64, f64) -> bool,
    {
        let right = self.pop()?;
        let left = self.pop()?;
        let result = match (&left, &right) {
            (Value::Int(x), Value::Int(y)) => int_op(*x, *y),
            (Value::Float(x), Value::Float(y)) => float_op(*x, *y),
            (Value::Int(x), Value::Float(y)) => float_op(*x as f64, *y),
            (Value::Float(x), Value::Int(y)) => float_op(*x, *y as f64),
            (Value::String(x), Value::String(y)) => match op_name {
                "<" => x < y,
                "<=" => x <= y,
                ">" => x > y,
                ">=" => x >= y,
                _ => {
                    return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "comparable",
                        got: left.type_name(),
                        operation: op_name,
                    }));
                }
            },
            _ => {
                return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "comparable",
                    got: left.type_name(),
                    operation: op_name,
                }));
            }
        };
        self.push(Value::Bool(result))
    }

    // ===== Function calls =====

    fn call_value(&mut self, arg_count: u8) -> RuntimeResult<()> {
        let callee = self.peek(arg_count as usize)?.clone();

        match callee {
            Value::Closure(closure) => self.call_closure(closure, arg_count),
            Value::NativeFunction(native) => self.call_native(native, arg_count),
            Value::BoundMethod(method) => {
                // Replace the method on the stack with the receiver
                let slot = self.stack.len() - 1 - arg_count as usize;
                self.stack[slot] = method.receiver.clone();
                self.call_closure(method.method.clone(), arg_count)
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::NotCallable(callee.type_name()))),
        }
    }

    fn call_closure(&mut self, closure: Rc<Closure>, arg_count: u8) -> RuntimeResult<()> {
        if arg_count != closure.function.arity {
            return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                expected: closure.function.arity,
                got: arg_count,
            }));
        }

        if self.frames.len() >= MAX_FRAMES {
            return Err(self.runtime_error(RuntimeErrorKind::StackOverflow));
        }

        // Stack layout: [..., closure, arg0, arg1, ...]
        // stack_base points to closure (slot 0 of the frame)
        let stack_base = self.stack.len() - arg_count as usize - 1;
        self.frames.push(CallFrame::new(closure, stack_base));

        Ok(())
    }

    fn call_native(&mut self, native: NativeFunction, arg_count: u8) -> RuntimeResult<()> {
        // Check arity
        if native.arity >= 0 && arg_count != native.arity as u8 {
            return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                expected: native.arity as u8,
                got: arg_count,
            }));
        }

        // Collect arguments
        let args: Vec<Value> = (0..arg_count)
            .map(|_| self.pop())
            .collect::<RuntimeResult<Vec<_>>>()?
            .into_iter()
            .rev()
            .collect();

        // Pop the function itself
        self.pop()?;

        // Call the native function
        let result = (native.function)(&args)
            .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?;

        self.push(result)
    }

    /// Call a closure with arguments and execute until it returns, collecting the result.
    /// This is used for higher-order functions like map, filter, reduce.
    fn call_closure_sync(&mut self, closure: Rc<Closure>, args: Vec<Value>) -> RuntimeResult<Value> {
        let arity = closure.function.arity;
        if args.len() as u8 != arity {
            return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                expected: arity,
                got: args.len() as u8,
            }));
        }

        // Remember current frame count to know when we've returned
        let starting_frame_count = self.frames.len();

        // Push closure and args onto stack
        self.push(Value::Closure(closure.clone()))?;
        for arg in args {
            self.push(arg)?;
        }

        // Set up the call frame
        let stack_base = self.stack.len() - arity as usize - 1;
        self.frames.push(CallFrame::new(closure, stack_base));

        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000;

        // Execute until we return to the original frame depth
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(self.runtime_error(RuntimeErrorKind::Internal(
                    format!("call_closure_sync exceeded {} iterations - likely infinite loop", MAX_ITERATIONS),
                )));
            }

            // Check for exception propagation
            if let Some(exception) = self.current_exception.take() {
                if !self.handle_exception(exception.clone())? {
                    return Err(self.runtime_error(RuntimeErrorKind::UncaughtException(exception)));
                }
                // If we unwound past our starting point, the exception escaped
                if self.frames.len() < starting_frame_count {
                    return Err(self.runtime_error(RuntimeErrorKind::UncaughtException(
                        Value::string("exception escaped closure"),
                    )));
                }
                continue;
            }

            let frame = self.current_frame();
            let chunk = frame.chunk();

            if frame.ip >= chunk.len() {
                // Unexpected end of bytecode
                return Err(self.runtime_error(RuntimeErrorKind::Internal(
                    format!("unexpected end of bytecode in closure: ip={}, len={}", frame.ip, chunk.len()),
                )));
            }

            let instruction = chunk
                .read_byte(frame.ip)
                .ok_or_else(|| {
                    self.runtime_error(RuntimeErrorKind::Internal(
                        "unexpected end of bytecode".to_string(),
                    ))
                })?;
            let opcode = OpCode::try_from(instruction)
                .map_err(|op| self.runtime_error(RuntimeErrorKind::InvalidOpcode(op)))?;

            self.current_frame_mut().ip += 1;

            // Handle Return specially to detect when closure is done
            if opcode == OpCode::Return {
                let result = self.pop()?;

                // Close upvalues
                let frame = &self.frames[self.frames.len() - 1];
                self.close_upvalues(frame.stack_base);

                // Pop the frame
                let frame = self.frames.pop().unwrap();

                // If we're back to starting frame count (or less), the closure is done
                if self.frames.len() <= starting_frame_count {
                    // Clean up stack
                    self.stack.truncate(frame.stack_base);
                    return Ok(result);
                }

                // Otherwise, push result and continue (nested call within closure)
                self.stack.truncate(frame.stack_base);
                self.push(result)?;
                continue;
            }

            // Execute other opcodes normally
            self.execute_opcode(opcode)?;
        }
    }

    /// Execute a single opcode (extracted from the main loop for reuse)
    fn execute_opcode(&mut self, opcode: OpCode) -> RuntimeResult<()> {
        match opcode {
            OpCode::Const => {
                let index = self.read_u16() as usize;
                let value = self.get_constant(index).clone();
                self.push(value)?;
            }

            OpCode::Null => self.push(Value::Null)?,
            OpCode::True => self.push(Value::Bool(true))?,
            OpCode::False => self.push(Value::Bool(false))?,

            OpCode::Pop => {
                self.pop()?;
            }

            OpCode::Dup => {
                let value = self.peek(0)?.clone();
                self.push(value)?;
            }

            OpCode::PopBelow => {
                let count = self.read_u8() as usize;
                if count > 0 {
                    let result = self.pop()?;
                    for _ in 0..count {
                        self.pop()?;
                    }
                    self.push(result)?;
                }
            }

            OpCode::LoadLocal => {
                let slot = self.read_u16() as usize;
                let value = self.get_local(slot).clone();
                self.push(value)?;
            }

            OpCode::StoreLocal => {
                let slot = self.read_u16() as usize;
                let value = self.peek(0)?.clone();
                self.set_local(slot, value);
            }

            OpCode::LoadGlobal => {
                let name_index = self.read_u16() as usize;
                let name = self.get_constant_string(name_index)?;
                let value = self
                    .globals
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UndefinedVariable(name)))?;
                self.push(value)?;
            }

            OpCode::StoreGlobal => {
                let name_index = self.read_u16() as usize;
                let name = self.get_constant_string(name_index)?;
                if !self.globals.contains_key(&name) {
                    return Err(self.runtime_error(RuntimeErrorKind::UndefinedVariable(name)));
                }
                let value = self.peek(0)?.clone();
                self.globals.insert(name, value);
            }

            OpCode::DefineGlobal => {
                let name_index = self.read_u16() as usize;
                let name = self.get_constant_string(name_index)?;
                let value = self.pop()?;
                self.globals.insert(name, value);
            }

            OpCode::LoadUpvalue => {
                let index = self.read_u8() as usize;
                let value = self.get_upvalue(index)?;
                self.push(value)?;
            }

            OpCode::StoreUpvalue => {
                let index = self.read_u8() as usize;
                let value = self.peek(0)?.clone();
                self.set_upvalue(index, value)?;
            }

            OpCode::CloseUpvalue => {
                let slot = self.stack.len() - 1;
                self.close_upvalues(slot);
                self.pop()?;
            }

            // Arithmetic operations
            OpCode::Add => self.binary_op(|a, b| match (a, b) {
                (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
                (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
                (Value::Int(x), Value::Float(y)) => Ok(Value::Float(x as f64 + y)),
                (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + y as f64)),
                (Value::String(x), Value::String(y)) => {
                    Ok(Value::string(format!("{}{}", *x, *y)))
                }
                (Value::String(x), other) => Ok(Value::string(format!("{}{}", *x, other))),
                (other, Value::String(y)) => Ok(Value::string(format!("{}{}", other, *y))),
                (l, _) => Err(RuntimeErrorKind::TypeError {
                    expected: "numeric or string",
                    got: l.type_name(),
                    operation: "+",
                }),
            })?,

            OpCode::Sub => self.numeric_binary_op("-", |x, y| x - y, |x, y| x - y)?,
            OpCode::Mul => self.numeric_binary_op("*", |x, y| x * y, |x, y| x * y)?,

            OpCode::Div => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    (Value::Int(_), Value::Int(0)) | (Value::Float(_), Value::Int(0)) => {
                        return Err(self.runtime_error(RuntimeErrorKind::DivisionByZero));
                    }
                    (Value::Int(_), Value::Float(y)) if *y == 0.0 => {
                        return Err(self.runtime_error(RuntimeErrorKind::DivisionByZero));
                    }
                    (Value::Float(_), Value::Float(y)) if *y == 0.0 => {
                        return Err(self.runtime_error(RuntimeErrorKind::DivisionByZero));
                    }
                    (Value::Int(x), Value::Int(y)) => Value::Int(x / y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x / y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 / y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x / *y as f64),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "numeric",
                            got: left.type_name(),
                            operation: "/",
                        }));
                    }
                };
                self.push(result)?;
            }

            OpCode::Mod => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    (Value::Int(_), Value::Int(0)) => {
                        return Err(self.runtime_error(RuntimeErrorKind::DivisionByZero));
                    }
                    (Value::Int(x), Value::Int(y)) => Value::Int(x % y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x % y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 % y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x % *y as f64),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "numeric",
                            got: left.type_name(),
                            operation: "%",
                        }));
                    }
                };
                self.push(result)?;
            }

            OpCode::Neg => {
                let value = self.pop()?;
                let result = match value {
                    Value::Int(x) => Value::Int(-x),
                    Value::Float(x) => Value::Float(-x),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "numeric",
                            got: value.type_name(),
                            operation: "unary -",
                        }));
                    }
                };
                self.push(result)?;
            }

            // Comparison operations
            OpCode::Eq => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left == right))?;
            }

            OpCode::Ne => {
                let right = self.pop()?;
                let left = self.pop()?;
                self.push(Value::Bool(left != right))?;
            }

            OpCode::Lt => self.comparison_op("<", |x, y| x < y, |x, y| x < y)?,
            OpCode::Le => self.comparison_op("<=", |x, y| x <= y, |x, y| x <= y)?,
            OpCode::Gt => self.comparison_op(">", |x, y| x > y, |x, y| x > y)?,
            OpCode::Ge => self.comparison_op(">=", |x, y| x >= y, |x, y| x >= y)?,

            OpCode::Not => {
                let value = self.pop()?;
                self.push(Value::Bool(!value.is_truthy()))?;
            }

            // Control flow
            OpCode::Jump => {
                let offset = self.read_i16();
                self.jump(offset);
            }

            OpCode::JumpIfFalse => {
                let offset = self.read_i16();
                let condition = self.pop()?;
                if !condition.is_truthy() {
                    self.jump(offset);
                }
            }

            OpCode::JumpIfTrue => {
                let offset = self.read_i16();
                let condition = self.pop()?;
                if condition.is_truthy() {
                    self.jump(offset);
                }
            }

            OpCode::JumpIfNull => {
                let offset = self.read_i16();
                let value = self.peek(0)?;
                if value.is_null() {
                    self.jump(offset);
                }
            }

            OpCode::JumpIfNotNull => {
                let offset = self.read_i16();
                let value = self.peek(0)?;
                if !value.is_null() {
                    self.jump(offset);
                }
            }

            OpCode::PopJumpIfNull => {
                let offset = self.read_i16();
                let value = self.peek(0)?;
                if value.is_null() {
                    self.jump(offset);
                } else {
                    self.pop()?;
                }
            }

            OpCode::Loop => {
                let offset = self.read_i16();
                self.jump(offset);
            }

            // Function calls
            OpCode::Call => {
                let arg_count = self.read_u8();
                self.call_value(arg_count)?;
            }

            OpCode::Return => {
                // Return is handled specially in execute() and call_closure_sync
                // If we get here from execute_opcode, it's an internal error
                return Err(self.runtime_error(RuntimeErrorKind::Internal(
                    "Return should be handled by caller".to_string(),
                )));
            }

            OpCode::Closure => {
                let func_index = self.read_u16() as usize;
                let function = self.get_constant_function(func_index)?;
                let upvalue_count = function.upvalue_count as usize;

                let mut closure = Closure::new(function);

                // Read upvalue descriptors
                for _ in 0..upvalue_count {
                    let is_local = self.read_u8() != 0;
                    let index = self.read_u8() as usize;

                    let upvalue = if is_local {
                        self.capture_upvalue(index)
                    } else {
                        self.current_frame().closure.upvalues[index].clone()
                    };
                    closure.upvalues.push(upvalue);
                }

                self.push(Value::Closure(Rc::new(closure)))?;
            }

            // Object operations
            OpCode::GetField => {
                let field_index = self.read_u16() as usize;
                let field_name = self.get_constant_string(field_index)?;
                let object = self.pop()?;
                let value = self.get_field(&object, &field_name)?;
                self.push(value)?;
            }

            OpCode::SetField => {
                let field_index = self.read_u16() as usize;
                let field_name = self.get_constant_string(field_index)?;
                let value = self.pop()?;
                let object = self.pop()?;
                self.set_field(object, &field_name, value.clone())?;
                self.push(value)?;
            }

            OpCode::GetProperty => {
                let prop_index = self.read_u16() as usize;
                let prop_name = self.get_constant_string(prop_index)?;
                let object = self.pop()?;
                let value = self.get_property(&object, &prop_name)?;
                self.push(value)?;
            }

            OpCode::GetIndex => {
                let index = self.pop()?;
                let collection = self.pop()?;
                let value = self.get_index(&collection, &index)?;
                self.push(value)?;
            }

            OpCode::SetIndex => {
                let value = self.pop()?;
                let index = self.pop()?;
                let collection = self.pop()?;
                self.set_index(collection, index, value.clone())?;
                self.push(value)?;
            }

            // Collection literals
            OpCode::NewList => {
                let count = self.read_u16() as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(self.pop()?);
                }
                items.reverse();
                self.push(Value::list(items))?;
            }

            OpCode::NewMap => {
                let count = self.read_u16() as usize;
                let mut map = HashMap::new();
                for _ in 0..count {
                    let value = self.pop()?;
                    let key = self.pop()?;
                    let hashable = HashableValue::try_from(key).map_err(|_| {
                        self.runtime_error(RuntimeErrorKind::UnhashableType(value.type_name()))
                    })?;
                    map.insert(hashable, value);
                }
                self.push(Value::Map(Rc::new(RefCell::new(map))))?;
            }

            OpCode::NewStruct => {
                let type_index = self.read_u16() as usize;
                let type_name = self.get_constant_string(type_index)?;
                let instance = StructInstance::new(type_name);
                self.push(Value::Struct(Rc::new(RefCell::new(instance))))?;
            }

            // Iteration
            OpCode::GetIter => {
                let iterable = self.pop()?;
                let iterator = self.make_iterator(iterable)?;
                self.push(iterator)?;
            }

            OpCode::IterNext => {
                let offset = self.read_i16();
                let iter_value = self.peek(0)?.clone();
                match &iter_value {
                    Value::Iterator(iter) => {
                        if let Some(value) = iter.borrow_mut().next() {
                            self.push(value)?;
                        } else {
                            self.jump(offset);
                        }
                    }
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::NotIterable(
                            iter_value.type_name(),
                        )));
                    }
                }
            }

            // Exception handling
            OpCode::Throw => {
                let exception = self.pop()?;
                self.current_exception = Some(exception);
            }

            OpCode::PushHandler => {
                let catch_offset = self.read_i16();
                let finally_offset = self.read_i16();
                let frame = self.frames.len() - 1;
                let ip = self.current_frame().ip;
                self.handlers.push(ExceptionHandler {
                    frame_index: frame,
                    stack_depth: self.stack.len(),
                    catch_ip: (ip as isize + catch_offset as isize) as usize,
                    finally_ip: if finally_offset != 0 {
                        (ip as isize + finally_offset as isize) as usize
                    } else {
                        0
                    },
                });
            }

            OpCode::PopHandler => {
                self.handlers.pop();
            }

            // String operations
            OpCode::StringConcat => {
                let count = self.read_u16() as usize;
                let mut parts = Vec::with_capacity(count);
                for _ in 0..count {
                    parts.push(format!("{}", self.pop()?));
                }
                parts.reverse();
                self.push(Value::string(parts.join("")))?;
            }

            // Range operations
            OpCode::NewRange => {
                let end = self.pop()?;
                let start = self.pop()?;
                let range = self.make_range(start, end, false)?;
                self.push(range)?;
            }

            OpCode::NewRangeInclusive => {
                let end = self.pop()?;
                let start = self.pop()?;
                let range = self.make_range(start, end, true)?;
                self.push(range)?;
            }

            // Type operations
            OpCode::IsNull => {
                let value = self.pop()?;
                self.push(Value::Bool(value.is_null()))?;
            }

            OpCode::IsInstance => {
                let type_index = self.read_u16() as usize;
                let type_name = self.get_constant_string(type_index)?;
                let value = self.pop()?;
                let is_instance = self.check_type(&value, &type_name);
                self.push(Value::Bool(is_instance))?;
            }

            // Method invocation
            OpCode::Invoke => {
                let method_index = self.read_u16() as usize;
                let arg_count = self.read_u8();
                let method_name = self.get_constant_string(method_index)?;
                self.invoke(method_name, arg_count)?;
            }

            // Enum operations
            OpCode::NewEnumVariant => {
                let info_index = self.read_u16() as usize;
                let info = self.get_constant_string(info_index)?;
                // Format: "EnumName.VariantName"
                let parts: Vec<&str> = info.split('.').collect();
                let (enum_name, variant_name) = if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    (String::new(), info)
                };
                // Check if variant has data (top of stack)
                let data = if self.peek(0).map(|v| !v.is_null()).unwrap_or(false) {
                    Some(self.pop()?)
                } else {
                    let _ = self.pop(); // Pop the null
                    None
                };
                let variant = EnumVariantInstance::new(enum_name, variant_name, data);
                self.push(Value::EnumVariant(Rc::new(variant)))?;
            }

            OpCode::MatchVariant => {
                let variant_index = self.read_u16() as usize;
                let expected = self.get_constant_string(variant_index)?;
                let value = self.peek(0)?.clone();
                match &value {
                    Value::EnumVariant(variant) => {
                        if variant.variant_name == expected {
                            self.pop()?; // Remove the enum from stack
                            if let Some(data) = &variant.data {
                                self.push(data.clone())?;
                            } else {
                                self.push(Value::Null)?;
                            }
                            self.push(Value::Bool(true))?;
                        } else {
                            self.push(Value::Bool(false))?;
                        }
                    }
                    _ => {
                        self.push(Value::Bool(false))?;
                    }
                }
            }

            // Null-safe operations
            OpCode::NullSafeGetField => {
                let field_index = self.read_u16() as usize;
                let object = self.pop()?;
                if object.is_null() {
                    self.push(Value::Null)?;
                } else {
                    let field_name = self.get_constant_string(field_index)?;
                    let value = self.get_field(&object, &field_name)?;
                    self.push(value)?;
                }
            }

            OpCode::NullSafeGetIndex => {
                let index = self.pop()?;
                let collection = self.pop()?;
                if collection.is_null() {
                    self.push(Value::Null)?;
                } else {
                    let value = self.get_index(&collection, &index)?;
                    self.push(value)?;
                }
            }

            OpCode::Await => {
                // For now, just return an error - async is not yet implemented
                return Err(self.runtime_error(RuntimeErrorKind::AwaitOutsideAsync));
            }

            OpCode::Breakpoint => {
                // No-op for now
            }
        }
        Ok(())
    }

    fn invoke(&mut self, method_name: String, arg_count: u8) -> RuntimeResult<()> {
        let receiver = self.peek(arg_count as usize)?.clone();

        match &receiver {
            Value::Struct(instance) => {
                // Check if there's a method with this name
                if let Some(method) = instance.borrow().fields.get(&method_name) {
                    if let Value::Closure(closure) = method {
                        // Replace receiver with bound method call
                        return self.call_closure(closure.clone(), arg_count);
                    }
                }
                // Try built-in struct methods
                self.invoke_builtin_method(&receiver, &method_name, arg_count)
            }
            Value::String(_) | Value::List(_) | Value::Map(_) | Value::NativeNamespace(_) | Value::DbConnection(_) => {
                self.invoke_builtin_method(&receiver, &method_name, arg_count)
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                expected: "object with methods",
                got: receiver.type_name(),
                operation: "method call",
            })),
        }
    }

    fn invoke_builtin_method(
        &mut self,
        receiver: &Value,
        method_name: &str,
        arg_count: u8,
    ) -> RuntimeResult<()> {
        // Collect arguments
        let args: Vec<Value> = (0..arg_count)
            .map(|_| self.pop())
            .collect::<RuntimeResult<Vec<_>>>()?
            .into_iter()
            .rev()
            .collect();

        // Pop the receiver
        self.pop()?;

        let result = match receiver {
            Value::String(s) => self.string_method(s, method_name, &args)?,
            Value::List(l) => self.list_method(l, method_name, &args)?,
            Value::Map(m) => self.map_method(m, method_name, &args)?,
            Value::NativeNamespace(ns) => {
                natives::dispatch_namespace_method(ns, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::DbConnection(conn) => {
                natives::db_connection_method(conn, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            _ => {
                return Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                    type_name: receiver.type_name().to_string(),
                    field: method_name.to_string(),
                }));
            }
        };

        self.push(result)
    }

    fn string_method(
        &self,
        s: &Rc<String>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            "length" | "len" => Ok(Value::Int(s.len() as i64)),
            "is_empty" => Ok(Value::Bool(s.is_empty())),
            "contains" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(needle) => Ok(Value::Bool(s.contains(needle.as_str()))),
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "contains",
                    })),
                }
            }
            "starts_with" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(prefix) => Ok(Value::Bool(s.starts_with(prefix.as_str()))),
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "starts_with",
                    })),
                }
            }
            "ends_with" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(suffix) => Ok(Value::Bool(s.ends_with(suffix.as_str()))),
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "ends_with",
                    })),
                }
            }
            "to_upper" | "to_uppercase" => Ok(Value::string(s.to_uppercase())),
            "to_lower" | "to_lowercase" => Ok(Value::string(s.to_lowercase())),
            "trim" => Ok(Value::string(s.trim())),
            "split" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(sep) => {
                        let parts: Vec<Value> =
                            s.split(sep.as_str()).map(Value::string).collect();
                        Ok(Value::list(parts))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "split",
                    })),
                }
            }
            "replace" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }
                match (&args[0], &args[1]) {
                    (Value::String(from), Value::String(to)) => {
                        Ok(Value::string(s.replace(from.as_str(), to.as_str())))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "replace",
                    })),
                }
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "String".to_string(),
                field: method.to_string(),
            })),
        }
    }

    fn list_method(
        &mut self,
        list: &Rc<RefCell<Vec<Value>>>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            "length" | "len" => Ok(Value::Int(list.borrow().len() as i64)),
            "is_empty" => Ok(Value::Bool(list.borrow().is_empty())),
            "push" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                list.borrow_mut().push(args[0].clone());
                Ok(Value::Null)
            }
            "pop" => {
                list.borrow_mut()
                    .pop()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::IndexOutOfBounds {
                        index: 0,
                        length: 0,
                    }))
            }
            "first" => {
                list.borrow()
                    .first()
                    .cloned()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::IndexOutOfBounds {
                        index: 0,
                        length: 0,
                    }))
            }
            "last" => {
                list.borrow()
                    .last()
                    .cloned()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::IndexOutOfBounds {
                        index: 0,
                        length: 0,
                    }))
            }
            "reverse" => {
                list.borrow_mut().reverse();
                Ok(Value::Null)
            }
            "contains" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                Ok(Value::Bool(list.borrow().contains(&args[0])))
            }
            "join" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(sep) => {
                        let parts: Vec<String> =
                            list.borrow().iter().map(|v| format!("{v}")).collect();
                        Ok(Value::string(parts.join(sep.as_str())))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "join",
                    })),
                }
            }
            // Higher-order functions
            "map" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                let closure = match &args[0] {
                    Value::Closure(c) => c.clone(),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Function",
                            got: args[0].type_name(),
                            operation: "map",
                        }));
                    }
                };
                let input = list.borrow().clone();
                let mut results = Vec::with_capacity(input.len());
                for item in input {
                    let result = self.call_closure_sync(closure.clone(), vec![item])?;
                    results.push(result);
                }
                Ok(Value::list(results))
            }
            "filter" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                let closure = match &args[0] {
                    Value::Closure(c) => c.clone(),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Function",
                            got: args[0].type_name(),
                            operation: "filter",
                        }));
                    }
                };
                let input = list.borrow().clone();
                let mut results = Vec::new();
                for item in input {
                    let result = self.call_closure_sync(closure.clone(), vec![item.clone()])?;
                    if result.is_truthy() {
                        results.push(item);
                    }
                }
                Ok(Value::list(results))
            }
            "reduce" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }
                let closure = match &args[0] {
                    Value::Closure(c) => c.clone(),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Function",
                            got: args[0].type_name(),
                            operation: "reduce",
                        }));
                    }
                };
                let input = list.borrow().clone();
                if input.is_empty() {
                    if args.len() == 2 {
                        return Ok(args[1].clone());
                    } else {
                        return Err(self.runtime_error(RuntimeErrorKind::UserError(
                            "reduce on empty list with no initial value".to_string(),
                        )));
                    }
                }
                let (initial, start_idx) = if args.len() == 2 {
                    (args[1].clone(), 0)
                } else {
                    (input[0].clone(), 1)
                };
                let mut accumulator = initial;
                for item in input.into_iter().skip(start_idx) {
                    accumulator =
                        self.call_closure_sync(closure.clone(), vec![accumulator, item])?;
                }
                Ok(accumulator)
            }
            "find" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                let closure = match &args[0] {
                    Value::Closure(c) => c.clone(),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Function",
                            got: args[0].type_name(),
                            operation: "find",
                        }));
                    }
                };
                let input = list.borrow().clone();
                for item in input {
                    let result = self.call_closure_sync(closure.clone(), vec![item.clone()])?;
                    if result.is_truthy() {
                        return Ok(item);
                    }
                }
                Ok(Value::Null)
            }
            "sort" => {
                // Sort with optional comparison closure
                let mut items = list.borrow().clone();
                if args.is_empty() {
                    // Default sort - compare values directly
                    items.sort_by(|a, b| {
                        match (a, b) {
                            (Value::Int(x), Value::Int(y)) => x.cmp(y),
                            (Value::Float(x), Value::Float(y)) => {
                                x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            (Value::String(x), Value::String(y)) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    Ok(Value::list(items))
                } else if args.len() == 1 {
                    // Sort with comparison closure
                    let closure = match &args[0] {
                        Value::Closure(c) => c.clone(),
                        _ => {
                            return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "Function",
                                got: args[0].type_name(),
                                operation: "sort",
                            }));
                        }
                    };
                    // We need to sort using the closure, but closures can fail
                    // Use a simple approach: collect comparisons first
                    let mut indices: Vec<usize> = (0..items.len()).collect();
                    let mut error: Option<RuntimeError> = None;
                    indices.sort_by(|&i, &j| {
                        if error.is_some() {
                            return std::cmp::Ordering::Equal;
                        }
                        let a = items[i].clone();
                        let b = items[j].clone();
                        match self.call_closure_sync(closure.clone(), vec![a, b]) {
                            Ok(Value::Int(n)) => {
                                if n < 0 {
                                    std::cmp::Ordering::Less
                                } else if n > 0 {
                                    std::cmp::Ordering::Greater
                                } else {
                                    std::cmp::Ordering::Equal
                                }
                            }
                            Ok(_) => std::cmp::Ordering::Equal,
                            Err(e) => {
                                error = Some(e);
                                std::cmp::Ordering::Equal
                            }
                        }
                    });
                    if let Some(e) = error {
                        return Err(e);
                    }
                    let sorted: Vec<Value> = indices.into_iter().map(|i| items[i].clone()).collect();
                    Ok(Value::list(sorted))
                } else {
                    Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }))
                }
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "List".to_string(),
                field: method.to_string(),
            })),
        }
    }

    fn map_method(
        &mut self,
        map: &Rc<RefCell<HashMap<HashableValue, Value>>>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            "length" | "len" => Ok(Value::Int(map.borrow().len() as i64)),
            "is_empty" => Ok(Value::Bool(map.borrow().is_empty())),
            "contains_key" | "has" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                let key = HashableValue::try_from(args[0].clone()).map_err(|_| {
                    self.runtime_error(RuntimeErrorKind::UnhashableType(args[0].type_name()))
                })?;
                Ok(Value::Bool(map.borrow().contains_key(&key)))
            }
            "get" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                let key = HashableValue::try_from(args[0].clone()).map_err(|_| {
                    self.runtime_error(RuntimeErrorKind::UnhashableType(args[0].type_name()))
                })?;
                let default = if args.len() == 2 {
                    args[1].clone()
                } else {
                    Value::Null
                };
                Ok(map.borrow().get(&key).cloned().unwrap_or(default))
            }
            "set" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }
                let key = HashableValue::try_from(args[0].clone()).map_err(|_| {
                    self.runtime_error(RuntimeErrorKind::UnhashableType(args[0].type_name()))
                })?;
                map.borrow_mut().insert(key, args[1].clone());
                // Return the map for method chaining
                Ok(Value::Map(map.clone()))
            }
            "remove" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                let key = HashableValue::try_from(args[0].clone()).map_err(|_| {
                    self.runtime_error(RuntimeErrorKind::UnhashableType(args[0].type_name()))
                })?;
                Ok(map.borrow_mut().remove(&key).unwrap_or(Value::Null))
            }
            "keys" => {
                let keys: Vec<Value> = map
                    .borrow()
                    .keys()
                    .map(|k| Value::from(k.clone()))
                    .collect();
                Ok(Value::list(keys))
            }
            "values" => {
                let values: Vec<Value> = map.borrow().values().cloned().collect();
                Ok(Value::list(values))
            }
            "entries" => {
                let entries: Vec<Value> = map
                    .borrow()
                    .iter()
                    .map(|(k, v)| {
                        // Each entry is a list [key, value]
                        Value::list(vec![Value::from(k.clone()), v.clone()])
                    })
                    .collect();
                Ok(Value::list(entries))
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "Map".to_string(),
                field: method.to_string(),
            })),
        }
    }

    // ===== Field access =====

    fn get_field(&self, object: &Value, field: &str) -> RuntimeResult<Value> {
        match object {
            Value::Struct(instance) => instance
                .borrow()
                .fields
                .get(field)
                .cloned()
                .ok_or_else(|| {
                    self.runtime_error(RuntimeErrorKind::UndefinedField {
                        type_name: instance.borrow().type_name.clone(),
                        field: field.to_string(),
                    })
                }),
            Value::Map(map) => {
                let key = HashableValue::String(Rc::new(field.to_string()));
                Ok(map.borrow().get(&key).cloned().unwrap_or(Value::Null))
            }
            Value::Null => Err(self.runtime_error(RuntimeErrorKind::NullReference)),
            _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                expected: "struct or map",
                got: object.type_name(),
                operation: "field access",
            })),
        }
    }

    fn set_field(&mut self, object: Value, field: &str, value: Value) -> RuntimeResult<()> {
        match object {
            Value::Struct(instance) => {
                instance.borrow_mut().fields.insert(field.to_string(), value);
                Ok(())
            }
            Value::Map(map) => {
                let key = HashableValue::String(Rc::new(field.to_string()));
                map.borrow_mut().insert(key, value);
                Ok(())
            }
            Value::Null => Err(self.runtime_error(RuntimeErrorKind::NullReference)),
            _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                expected: "struct or map",
                got: object.type_name(),
                operation: "field assignment",
            })),
        }
    }

    fn get_property(&self, object: &Value, property: &str) -> RuntimeResult<Value> {
        // First try as a field
        if let Ok(value) = self.get_field(object, property) {
            return Ok(value);
        }

        // Then try as a built-in property
        match object {
            Value::String(s) => match property {
                "length" | "len" => Ok(Value::Int(s.len() as i64)),
                _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                    type_name: "String".to_string(),
                    field: property.to_string(),
                })),
            },
            Value::List(l) => match property {
                "length" | "len" => Ok(Value::Int(l.borrow().len() as i64)),
                _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                    type_name: "List".to_string(),
                    field: property.to_string(),
                })),
            },
            Value::Map(m) => match property {
                "length" | "len" => Ok(Value::Int(m.borrow().len() as i64)),
                _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                    type_name: "Map".to_string(),
                    field: property.to_string(),
                })),
            },
            Value::Range(r) => match property {
                "start" => Ok(Value::Int(r.start)),
                "end" => Ok(Value::Int(r.end)),
                "inclusive" => Ok(Value::Bool(r.inclusive)),
                _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                    type_name: "Range".to_string(),
                    field: property.to_string(),
                })),
            },
            Value::EnumVariant(e) => match property {
                "name" | "variant_name" => Ok(Value::string(&e.variant_name)),
                "data" => Ok(e.data.clone().unwrap_or(Value::Null)),
                _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                    type_name: "EnumVariant".to_string(),
                    field: property.to_string(),
                })),
            },
            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: object.type_name().to_string(),
                field: property.to_string(),
            })),
        }
    }

    // ===== Index access =====

    fn get_index(&self, collection: &Value, index: &Value) -> RuntimeResult<Value> {
        match (collection, index) {
            (Value::List(list), Value::Int(i)) => {
                let list = list.borrow();
                let idx = self.normalize_index(*i, list.len())?;
                Ok(list[idx].clone())
            }
            (Value::String(s), Value::Int(i)) => {
                let idx = self.normalize_index(*i, s.len())?;
                s.chars()
                    .nth(idx)
                    .map(|c| Value::string(c.to_string()))
                    .ok_or_else(|| {
                        self.runtime_error(RuntimeErrorKind::IndexOutOfBounds {
                            index: *i,
                            length: s.len(),
                        })
                    })
            }
            (Value::Map(map), key) => {
                let hashable = HashableValue::try_from(key.clone()).map_err(|_| {
                    self.runtime_error(RuntimeErrorKind::UnhashableType(key.type_name()))
                })?;
                Ok(map.borrow().get(&hashable).cloned().unwrap_or(Value::Null))
            }
            (Value::Null, _) => Err(self.runtime_error(RuntimeErrorKind::NullReference)),
            (_, Value::Int(_)) => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                expected: "List or String",
                got: collection.type_name(),
                operation: "index",
            })),
            (_, _) => Err(self.runtime_error(RuntimeErrorKind::InvalidIndexType {
                got: index.type_name(),
            })),
        }
    }

    fn set_index(&mut self, collection: Value, index: Value, value: Value) -> RuntimeResult<()> {
        match (collection, index) {
            (Value::List(list), Value::Int(i)) => {
                let len = list.borrow().len();
                let idx = self.normalize_index(i, len)?;
                list.borrow_mut()[idx] = value;
                Ok(())
            }
            (Value::Map(map), key) => {
                let hashable = HashableValue::try_from(key.clone()).map_err(|_| {
                    self.runtime_error(RuntimeErrorKind::UnhashableType(key.type_name()))
                })?;
                map.borrow_mut().insert(hashable, value);
                Ok(())
            }
            (Value::Null, _) => Err(self.runtime_error(RuntimeErrorKind::NullReference)),
            (collection, _) => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                expected: "List or Map",
                got: collection.type_name(),
                operation: "index assignment",
            })),
        }
    }

    fn normalize_index(&self, index: i64, length: usize) -> RuntimeResult<usize> {
        let len = length as i64;
        let idx = if index < 0 { len + index } else { index };
        if idx < 0 || idx >= len {
            Err(self.runtime_error(RuntimeErrorKind::IndexOutOfBounds {
                index,
                length,
            }))
        } else {
            Ok(idx as usize)
        }
    }

    // ===== Iteration =====

    fn make_iterator(&self, iterable: Value) -> RuntimeResult<Value> {
        match iterable {
            Value::Range(range) => {
                let iter: Box<dyn Iterator<Item = Value>> = if range.inclusive {
                    Box::new((range.start..=range.end).map(Value::Int))
                } else {
                    Box::new((range.start..range.end).map(Value::Int))
                };
                Ok(Value::Iterator(Rc::new(RefCell::new(iter))))
            }
            Value::List(list) => {
                let items = list.borrow().clone();
                let iter: Box<dyn Iterator<Item = Value>> = Box::new(items.into_iter());
                Ok(Value::Iterator(Rc::new(RefCell::new(iter))))
            }
            Value::String(s) => {
                let chars: Vec<Value> = s.chars().map(|c| Value::string(c.to_string())).collect();
                let iter: Box<dyn Iterator<Item = Value>> = Box::new(chars.into_iter());
                Ok(Value::Iterator(Rc::new(RefCell::new(iter))))
            }
            Value::Map(map) => {
                let keys: Vec<Value> = map
                    .borrow()
                    .keys()
                    .map(|k| Value::from(k.clone()))
                    .collect();
                let iter: Box<dyn Iterator<Item = Value>> = Box::new(keys.into_iter());
                Ok(Value::Iterator(Rc::new(RefCell::new(iter))))
            }
            Value::Iterator(iter) => Ok(Value::Iterator(iter)),
            _ => Err(self.runtime_error(RuntimeErrorKind::NotIterable(iterable.type_name()))),
        }
    }

    fn make_range(&self, start: Value, end: Value, inclusive: bool) -> RuntimeResult<Value> {
        let start = match start {
            Value::Int(i) => i,
            _ => {
                return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "Int",
                    got: start.type_name(),
                    operation: "range start",
                }));
            }
        };
        let end = match end {
            Value::Int(i) => i,
            _ => {
                return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "Int",
                    got: end.type_name(),
                    operation: "range end",
                }));
            }
        };
        let range = if inclusive {
            Range::inclusive(start, end)
        } else {
            Range::exclusive(start, end)
        };
        Ok(Value::Range(Rc::new(range)))
    }

    // ===== Type checking =====

    fn check_type(&self, value: &Value, type_name: &str) -> bool {
        match (value, type_name) {
            (Value::Null, "Null") => true,
            (Value::Bool(_), "Bool") => true,
            (Value::Int(_), "Int") => true,
            (Value::Float(_), "Float") => true,
            (Value::String(_), "String") => true,
            (Value::List(_), "List") => true,
            (Value::Map(_), "Map") => true,
            (Value::Function(_) | Value::Closure(_) | Value::NativeFunction(_), "Function") => {
                true
            }
            (Value::Struct(s), name) => s.borrow().type_name == name,
            (Value::EnumVariant(e), name) => e.enum_name == name,
            (Value::Range(_), "Range") => true,
            (Value::Iterator(_), "Iterator") => true,
            _ => false,
        }
    }

    // ===== Exception handling =====

    fn handle_exception(&mut self, exception: Value) -> RuntimeResult<bool> {
        while let Some(handler) = self.handlers.pop() {
            // Check if the handler is in the current call stack
            if handler.frame_index >= self.frames.len() {
                continue;
            }

            // Unwind the call stack to the handler's frame
            while self.frames.len() > handler.frame_index + 1 {
                let frame = self.frames.pop().unwrap();
                self.close_upvalues(frame.stack_base);
            }

            // Reset the stack to the handler's depth
            self.stack.truncate(handler.stack_depth);

            // Push the exception value
            self.push(exception)?;

            // Jump to the catch handler
            self.current_frame_mut().ip = handler.catch_ip;

            return Ok(true);
        }

        Ok(false)
    }

    // ===== Error handling =====

    fn runtime_error(&self, kind: RuntimeErrorKind) -> RuntimeError {
        let mut error = RuntimeError::new(kind);

        // Build stack trace
        for frame in self.frames.iter().rev() {
            let line = frame.chunk().get_line(frame.ip.saturating_sub(1));
            let source = frame.chunk().source_name.clone();
            let function_name = if frame.closure.function.name.is_empty() {
                "<script>".to_string()
            } else {
                frame.closure.function.name.clone()
            };
            let stack_frame = if let Some(src) = source {
                StackFrame::with_source(function_name, line, src)
            } else {
                StackFrame::new(function_name, line)
            };
            error.stack_trace.push(stack_frame);
        }

        error
    }

    /// Get a reference to the global variables
    pub fn globals(&self) -> &HashMap<String, Value> {
        &self.globals
    }

    /// Get a mutable reference to the global variables
    pub fn globals_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.globals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Chunk;

    fn make_function(chunk: Chunk) -> Rc<Function> {
        Rc::new(Function {
            name: String::new(),
            arity: 0,
            upvalue_count: 0,
            chunk,
        })
    }

    #[test]
    fn test_push_constants() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(42), 1);
        chunk.emit_constant(Value::Float(3.14), 1);
        chunk.emit_constant(Value::string("hello"), 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::string("hello"));
    }

    #[test]
    fn test_arithmetic() {
        // 10 + 20 = 30
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(10), 1);
        chunk.emit_constant(Value::Int(20), 1);
        chunk.write_op(OpCode::Add, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Int(30));
    }

    #[test]
    fn test_subtraction() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(50), 1);
        chunk.emit_constant(Value::Int(20), 1);
        chunk.write_op(OpCode::Sub, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Int(30));
    }

    #[test]
    fn test_division_by_zero() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(10), 1);
        chunk.emit_constant(Value::Int(0), 1);
        chunk.write_op(OpCode::Div, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk));
        assert!(result.is_err());
    }

    #[test]
    fn test_comparison() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(5), 1);
        chunk.emit_constant(Value::Int(10), 1);
        chunk.write_op(OpCode::Lt, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_string_concat() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::string("Hello, "), 1);
        chunk.emit_constant(Value::string("World!"), 1);
        chunk.write_op(OpCode::Add, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::string("Hello, World!"));
    }

    #[test]
    fn test_global_variables() {
        let mut chunk = Chunk::new();

        // Define global 'x' = 42
        chunk.emit_constant(Value::Int(42), 1);
        let name_idx = chunk.add_constant(Value::string("x")).unwrap();
        chunk.write_op(OpCode::DefineGlobal, 1);
        chunk.write_u16(name_idx, 1);

        // Load global 'x'
        chunk.write_op(OpCode::LoadGlobal, 1);
        chunk.write_u16(name_idx, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_list_creation() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(1), 1);
        chunk.emit_constant(Value::Int(2), 1);
        chunk.emit_constant(Value::Int(3), 1);
        chunk.write_op(OpCode::NewList, 1);
        chunk.write_u16(3, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        match result {
            Value::List(l) => {
                let list = l.borrow();
                assert_eq!(list.len(), 3);
                assert_eq!(list[0], Value::Int(1));
                assert_eq!(list[1], Value::Int(2));
                assert_eq!(list[2], Value::Int(3));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_negation() {
        let mut chunk = Chunk::new();
        chunk.emit_constant(Value::Int(42), 1);
        chunk.write_op(OpCode::Neg, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Int(-42));
    }

    #[test]
    fn test_not() {
        let mut chunk = Chunk::new();
        chunk.write_op(OpCode::True, 1);
        chunk.write_op(OpCode::Not, 1);
        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn test_jump_if_false() {
        // if false { 42 } else { 100 }
        let mut chunk = Chunk::new();

        // Push false
        chunk.write_op(OpCode::False, 1);

        // JumpIfFalse to else branch
        let jump_offset = chunk.emit_jump(OpCode::JumpIfFalse, 1);

        // Then branch: push 42
        chunk.emit_constant(Value::Int(42), 1);
        let else_jump = chunk.emit_jump(OpCode::Jump, 1);

        // Patch the JumpIfFalse
        chunk.patch_jump(jump_offset);

        // Else branch: push 100
        chunk.emit_constant(Value::Int(100), 1);

        // Patch the Jump
        chunk.patch_jump(else_jump);

        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Int(100));
    }

    #[test]
    fn test_loop() {
        // sum = 0; for i in 0..3 { sum = sum + i }; sum
        // This is a simplified version that just loops 3 times adding to a counter
        let mut chunk = Chunk::new();

        // Push initial counter value 0
        chunk.emit_constant(Value::Int(0), 1);

        // Loop start
        let loop_start = chunk.len();

        // Duplicate counter
        chunk.write_op(OpCode::Dup, 1);

        // Push 3
        chunk.emit_constant(Value::Int(3), 1);

        // Compare: counter < 3
        chunk.write_op(OpCode::Lt, 1);

        // Jump if false (exit loop)
        let exit_jump = chunk.emit_jump(OpCode::JumpIfFalse, 1);

        // Increment counter by 1
        chunk.emit_constant(Value::Int(1), 1);
        chunk.write_op(OpCode::Add, 1);

        // Loop back
        chunk.emit_loop(loop_start, 1);

        // Patch exit jump
        chunk.patch_jump(exit_jump);

        chunk.write_op(OpCode::Return, 1);

        let mut vm = VM::new();
        let result = vm.run(make_function(chunk)).unwrap();
        assert_eq!(result, Value::Int(3));
    }
}
