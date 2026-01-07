//! JIT Compiler implementation
//!
//! This module provides the main `JitCompiler` that translates Stratum bytecode
//! to native machine code using Cranelift.

use std::collections::HashMap;
use std::mem;
use std::rc::Rc;
use std::sync::Arc;

use cranelift_codegen::ir::{condcodes::IntCC, AbiParam, InstBuilder, MemFlags, Signature, UserFuncName};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

use crate::bytecode::{Chunk, Function, OpCode, Value};

use super::runtime;
use super::types::{CraneliftTypes, ValueTag};
use super::{JitError, JitResult};

/// The JIT compiler for Stratum bytecode
///
/// This struct manages the Cranelift JIT compilation context and provides
/// methods to compile Stratum functions to native code.
pub struct JitCompiler {
    /// The Cranelift JIT module
    module: JITModule,

    /// Target ISA for code generation
    #[allow(dead_code)]
    isa: Arc<dyn TargetIsa>,

    /// Compilation context (reused between compilations)
    ctx: Context,

    /// Function builder context (reused between compilations)
    builder_ctx: FunctionBuilderContext,

    /// Cache of runtime helper function IDs
    #[allow(dead_code)]
    runtime_funcs: HashMap<&'static str, FuncId>,

    /// Cache of compiled Stratum function IDs
    compiled_functions: HashMap<String, FuncId>,
}

impl JitCompiler {
    /// Create a new JIT compiler
    ///
    /// # Panics
    /// Panics if the native target cannot be determined or if Cranelift
    /// cannot be configured for the host platform.
    #[must_use]
    pub fn new() -> Self {
        // Configure Cranelift for the host machine
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("Host machine is not supported: {}", msg);
        });

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder.clone()))
            .unwrap();

        // Create the JIT module
        let mut jit_builder = JITBuilder::with_isa(isa.clone(), cranelift_module::default_libcall_names());

        // Register runtime symbols
        Self::register_runtime_symbols(&mut jit_builder);

        let module = JITModule::new(jit_builder);

        Self {
            module,
            isa,
            ctx: Context::new(),
            builder_ctx: FunctionBuilderContext::new(),
            runtime_funcs: HashMap::new(),
            compiled_functions: HashMap::new(),
        }
    }

    /// Register runtime helper functions as symbols available to JIT code
    fn register_runtime_symbols(builder: &mut JITBuilder) {
        // Reference counting
        builder.symbol("stratum_rc_inc", runtime::stratum_rc_inc as *const u8);
        builder.symbol("stratum_rc_dec", runtime::stratum_rc_dec as *const u8);

        // Arithmetic helpers
        builder.symbol("stratum_add_int", runtime::stratum_add_int as *const u8);
        builder.symbol("stratum_add_float", runtime::stratum_add_float as *const u8);

        // String operations
        builder.symbol("stratum_concat_strings", runtime::stratum_concat_strings as *const u8);

        // List operations
        builder.symbol("stratum_new_list", runtime::stratum_new_list as *const u8);
        builder.symbol("stratum_list_len", runtime::stratum_list_len as *const u8);

        // Utilities
        builder.symbol("stratum_is_truthy", runtime::stratum_is_truthy as *const u8);
        builder.symbol("stratum_print_int", runtime::stratum_print_int as *const u8);
        builder.symbol("stratum_print_float", runtime::stratum_print_float as *const u8);
        builder.symbol("stratum_print_bool", runtime::stratum_print_bool as *const u8);
        builder.symbol("stratum_runtime_error", runtime::stratum_runtime_error as *const u8);

        // Function call support
        builder.symbol("stratum_call_jit_direct", runtime::stratum_call_jit_direct as *const u8);
    }

    /// Get or declare a runtime function
    #[allow(dead_code)]
    fn get_runtime_func(&mut self, name: &'static str, sig: Signature) -> JitResult<FuncId> {
        if let Some(&id) = self.runtime_funcs.get(name) {
            return Ok(id);
        }

        let id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        self.runtime_funcs.insert(name, id);
        Ok(id)
    }

    /// Create a signature for functions that take a packed value and return one
    #[allow(dead_code)]
    fn value_to_value_sig(&self) -> Signature {
        let mut sig = self.module.make_signature();
        // Input: tag+pad (i64) + data (i64)
        sig.params.push(AbiParam::new(CraneliftTypes::VALUE_FIRST));
        sig.params.push(AbiParam::new(CraneliftTypes::VALUE_SECOND));
        // Output: tag+pad (i64) + data (i64)
        sig.returns.push(AbiParam::new(CraneliftTypes::VALUE_FIRST));
        sig.returns.push(AbiParam::new(CraneliftTypes::VALUE_SECOND));
        sig
    }

    /// Compile a Stratum function to native code
    ///
    /// Returns a function pointer that can be called with `PackedValue` arguments.
    pub fn compile_function(&mut self, function: &Function) -> JitResult<*const u8> {
        // Check if already compiled
        if let Some(&func_id) = self.compiled_functions.get(&function.name) {
            return Ok(self.module.get_finalized_function(func_id));
        }

        // Create the Cranelift function signature
        let mut sig = self.module.make_signature();

        // Each parameter is a packed value (two i64s)
        for _ in 0..function.arity {
            sig.params.push(AbiParam::new(CraneliftTypes::VALUE_FIRST));
            sig.params.push(AbiParam::new(CraneliftTypes::VALUE_SECOND));
        }

        // Return type is also a packed value
        sig.returns.push(AbiParam::new(CraneliftTypes::VALUE_FIRST));
        sig.returns.push(AbiParam::new(CraneliftTypes::VALUE_SECOND));

        // Declare the function
        let name = format!("stratum_{}", function.name);
        let func_id = self
            .module
            .declare_function(&name, Linkage::Local, &sig)
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        // Build the function body
        self.ctx.func.signature = sig;
        self.ctx.func.name = UserFuncName::user(0, func_id.as_u32());

        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_ctx);
            let mut compiler = FunctionCompiler::new(&mut builder, &function.chunk, function.arity);
            compiler.compile()?;
            builder.finalize();
        }

        // Compile to machine code
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        self.module.clear_context(&mut self.ctx);

        // Finalize and get the function pointer
        self.module
            .finalize_definitions()
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        let ptr = self.module.get_finalized_function(func_id);
        self.compiled_functions.insert(function.name.clone(), func_id);

        Ok(ptr)
    }

    /// Compile a simple expression that takes no arguments and returns an i64
    ///
    /// This is a simplified interface for testing basic JIT functionality.
    pub fn compile_simple_int_function(
        &mut self,
        name: &str,
        body: impl FnOnce(&mut FunctionBuilder<'_>, Variable) -> JitResult<()>,
    ) -> JitResult<fn() -> i64> {
        let mut sig = self.module.make_signature();
        sig.returns.push(AbiParam::new(CraneliftTypes::INT));

        let func_id = self
            .module
            .declare_function(name, Linkage::Local, &sig)
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        self.ctx.func.signature = sig;
        self.ctx.func.name = UserFuncName::user(0, func_id.as_u32());

        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_ctx);

            // Create entry block
            let entry = builder.create_block();
            builder.switch_to_block(entry);
            builder.seal_block(entry);

            // Create result variable
            let result_var = Variable::from_u32(0);
            builder.declare_var(result_var, CraneliftTypes::INT);

            // Execute the body
            body(&mut builder, result_var)?;

            // Return the result
            let result = builder.use_var(result_var);
            builder.ins().return_(&[result]);

            builder.finalize();
        }

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        self.module.clear_context(&mut self.ctx);

        self.module
            .finalize_definitions()
            .map_err(|e| JitError::Cranelift(e.to_string()))?;

        let ptr = self.module.get_finalized_function(func_id);

        // Safety: The function pointer is valid and has the correct signature
        Ok(unsafe { mem::transmute::<*const u8, fn() -> i64>(ptr) })
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal state for compiling a single function
struct FunctionCompiler<'a, 'b> {
    builder: &'a mut FunctionBuilder<'b>,
    chunk: &'a Chunk,
    arity: u8,

    /// Variables for each stack slot (simulating the VM stack)
    /// Each value is stored as two variables: tag+pad and data
    stack_vars: Vec<(Variable, Variable)>,

    /// Current logical stack depth
    stack_depth: usize,

    /// Next variable index to allocate
    next_var: usize,

    /// Variables for local variables (parameters + locals)
    /// Each local is two variables
    locals: Vec<(Variable, Variable)>,

    /// Block for each bytecode offset (for jump targets)
    blocks: HashMap<usize, cranelift_codegen::ir::Block>,

    /// Current instruction pointer
    ip: usize,

    /// Whether the current block has a terminator (jump/return)
    block_terminated: bool,
}

impl<'a, 'b> FunctionCompiler<'a, 'b> {
    fn new(builder: &'a mut FunctionBuilder<'b>, chunk: &'a Chunk, arity: u8) -> Self {
        Self {
            builder,
            chunk,
            arity,
            stack_vars: Vec::new(),
            stack_depth: 0,
            next_var: 0,
            locals: Vec::new(),
            blocks: HashMap::new(),
            ip: 0,
            block_terminated: false,
        }
    }

    /// Read a u8 from the chunk, panicking if out of bounds
    fn read_u8(&self, offset: usize) -> u8 {
        self.chunk.read_byte(offset).expect("bytecode read out of bounds")
    }

    /// Read a u16 from the chunk, panicking if out of bounds
    fn read_u16(&self, offset: usize) -> u16 {
        self.chunk.read_u16(offset).expect("bytecode read out of bounds")
    }

    /// Read an i16 from the chunk, panicking if out of bounds
    fn read_i16(&self, offset: usize) -> i16 {
        self.chunk.read_i16(offset).expect("bytecode read out of bounds")
    }

    /// Allocate a pair of variables for a value (tag+pad, data)
    fn alloc_value_vars(&mut self) -> (Variable, Variable) {
        let tag_var = Variable::from_u32(self.next_var as u32);
        self.next_var += 1;
        let data_var = Variable::from_u32(self.next_var as u32);
        self.next_var += 1;

        self.builder.declare_var(tag_var, CraneliftTypes::VALUE_FIRST);
        self.builder.declare_var(data_var, CraneliftTypes::VALUE_SECOND);

        (tag_var, data_var)
    }

    /// Push a value onto the virtual stack
    fn push(&mut self, tag: cranelift_codegen::ir::Value, data: cranelift_codegen::ir::Value) {
        if self.stack_depth >= self.stack_vars.len() {
            let vars = self.alloc_value_vars();
            self.stack_vars.push(vars);
        }
        let (tag_var, data_var) = self.stack_vars[self.stack_depth];
        self.builder.def_var(tag_var, tag);
        self.builder.def_var(data_var, data);
        self.stack_depth += 1;
    }

    /// Pop a value from the virtual stack
    fn pop(&mut self) -> (cranelift_codegen::ir::Value, cranelift_codegen::ir::Value) {
        assert!(self.stack_depth > 0, "Stack underflow");
        self.stack_depth -= 1;
        let (tag_var, data_var) = self.stack_vars[self.stack_depth];
        (self.builder.use_var(tag_var), self.builder.use_var(data_var))
    }

    /// Peek at the top of the stack without popping
    fn peek(&mut self) -> (cranelift_codegen::ir::Value, cranelift_codegen::ir::Value) {
        assert!(self.stack_depth > 0, "Stack underflow");
        let (tag_var, data_var) = self.stack_vars[self.stack_depth - 1];
        (self.builder.use_var(tag_var), self.builder.use_var(data_var))
    }

    /// Get or create a block for a given bytecode offset
    fn get_block(&mut self, offset: usize) -> cranelift_codegen::ir::Block {
        if let Some(&block) = self.blocks.get(&offset) {
            return block;
        }
        let block = self.builder.create_block();
        self.blocks.insert(offset, block);
        block
    }

    /// Compile the function bytecode
    fn compile(&mut self) -> JitResult<()> {
        // Create entry block with parameters
        let entry = self.builder.create_block();

        // Add parameters to entry block
        for _ in 0..self.arity {
            self.builder.append_block_param(entry, CraneliftTypes::VALUE_FIRST);
            self.builder.append_block_param(entry, CraneliftTypes::VALUE_SECOND);
        }

        self.builder.switch_to_block(entry);

        // Set up locals from parameters
        let params: Vec<_> = self.builder.block_params(entry).to_vec();
        for i in 0..self.arity as usize {
            let vars = self.alloc_value_vars();
            self.builder.def_var(vars.0, params[i * 2]);
            self.builder.def_var(vars.1, params[i * 2 + 1]);
            self.locals.push(vars);
        }

        // Scan bytecode to find jump targets and create blocks
        self.scan_for_blocks()?;

        // Compile each instruction
        let code = self.chunk.code().to_vec();
        while self.ip < code.len() {
            // Check if we need to start a new block here
            if self.blocks.contains_key(&self.ip) && self.ip > 0 {
                let block = self.blocks[&self.ip];
                // Jump to the block if we're falling through
                if !self.block_terminated {
                    self.builder.ins().jump(block, &[]);
                }
                self.builder.switch_to_block(block);
                self.block_terminated = false;
            }

            let op = OpCode::try_from(code[self.ip])
                .map_err(|b| JitError::UnsupportedInstruction(format!("unknown opcode {}", b)))?;

            self.compile_instruction(op)?;
        }

        // Seal all blocks
        for &block in self.blocks.values() {
            self.builder.seal_block(block);
        }
        self.builder.seal_block(entry);

        // If no explicit return, return null
        if !self.block_terminated {
            let null_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Null as i64);
            let null_data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, 0);
            self.builder.ins().return_(&[null_tag, null_data]);
        }

        Ok(())
    }

    /// Scan bytecode to find all jump targets
    fn scan_for_blocks(&mut self) -> JitResult<()> {
        let code = self.chunk.code();
        let mut ip = 0;

        while ip < code.len() {
            let op = OpCode::try_from(code[ip])
                .map_err(|b| JitError::UnsupportedInstruction(format!("unknown opcode {}", b)))?;

            match op {
                OpCode::Jump
                | OpCode::JumpIfFalse
                | OpCode::JumpIfTrue
                | OpCode::JumpIfNull
                | OpCode::JumpIfNotNull
                | OpCode::Loop => {
                    // Read the offset
                    let offset = self.read_i16(ip + 1);
                    let target = ((ip as isize + 3) + offset as isize) as usize;
                    self.get_block(target);
                }
                OpCode::IterNext => {
                    let offset = self.read_i16(ip + 1);
                    let target = ((ip as isize + 3) + offset as isize) as usize;
                    self.get_block(target);
                }
                _ => {}
            }

            ip += op.size();
        }

        Ok(())
    }

    /// Compile a single bytecode instruction
    fn compile_instruction(&mut self, op: OpCode) -> JitResult<()> {
        let start_ip = self.ip;
        self.ip += 1; // Advance past opcode

        match op {
            OpCode::Const => {
                let index = self.read_u16(start_ip + 1);
                self.ip += 2;

                let constant = &self.chunk.constants()[index as usize];
                self.compile_constant(constant)?;
            }

            OpCode::Null => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Null as i64);
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, 0);
                self.push(tag, data);
            }

            OpCode::True => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Bool as i64);
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, 1);
                self.push(tag, data);
            }

            OpCode::False => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Bool as i64);
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, 0);
                self.push(tag, data);
            }

            OpCode::Pop => {
                let _ = self.pop();
            }

            OpCode::Dup => {
                let (tag, data) = self.peek();
                self.push(tag, data);
            }

            OpCode::LoadLocal => {
                let slot = self.read_u16(start_ip + 1) as usize;
                self.ip += 2;

                // Ensure we have enough locals
                while self.locals.len() <= slot {
                    let vars = self.alloc_value_vars();
                    self.locals.push(vars);
                }

                let (tag_var, data_var) = self.locals[slot];
                let tag = self.builder.use_var(tag_var);
                let data = self.builder.use_var(data_var);
                self.push(tag, data);
            }

            OpCode::StoreLocal => {
                let slot = self.read_u16(start_ip + 1) as usize;
                self.ip += 2;

                // Ensure we have enough locals
                while self.locals.len() <= slot {
                    let vars = self.alloc_value_vars();
                    self.locals.push(vars);
                }

                let (tag, data) = self.peek();
                let (tag_var, data_var) = self.locals[slot];
                self.builder.def_var(tag_var, tag);
                self.builder.def_var(data_var, data);
            }

            OpCode::Add => self.compile_binary_op(BinaryOp::Add)?,
            OpCode::Sub => self.compile_binary_op(BinaryOp::Sub)?,
            OpCode::Mul => self.compile_binary_op(BinaryOp::Mul)?,
            OpCode::Div => self.compile_binary_op(BinaryOp::Div)?,
            OpCode::Mod => self.compile_binary_op(BinaryOp::Mod)?,

            OpCode::Neg => self.compile_unary_neg()?,

            OpCode::Eq => self.compile_comparison(IntCC::Equal)?,
            OpCode::Ne => self.compile_comparison(IntCC::NotEqual)?,
            OpCode::Lt => self.compile_comparison(IntCC::SignedLessThan)?,
            OpCode::Le => self.compile_comparison(IntCC::SignedLessThanOrEqual)?,
            OpCode::Gt => self.compile_comparison(IntCC::SignedGreaterThan)?,
            OpCode::Ge => self.compile_comparison(IntCC::SignedGreaterThanOrEqual)?,

            OpCode::Not => self.compile_unary_not()?,

            OpCode::Jump => {
                let offset = self.read_i16(start_ip + 1);
                self.ip += 2;
                let target = ((start_ip as isize + 3) + offset as isize) as usize;
                let block = self.get_block(target);
                self.builder.ins().jump(block, &[]);
                self.block_terminated = true;
            }

            OpCode::JumpIfFalse => {
                let offset = self.read_i16(start_ip + 1);
                self.ip += 2;
                let target = ((start_ip as isize + 3) + offset as isize) as usize;

                let (tag, data) = self.pop();
                let target_block = self.get_block(target);
                let fallthrough = self.builder.create_block();

                // Check if value is falsy (null or false)
                let is_null = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Null as i64);
                let is_bool = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Bool as i64);
                let is_false = self.builder.ins().icmp_imm(IntCC::Equal, data, 0);
                let bool_and_false = self.builder.ins().band(is_bool, is_false);
                let is_falsy = self.builder.ins().bor(is_null, bool_and_false);

                self.builder.ins().brif(is_falsy, target_block, &[], fallthrough, &[]);
                self.builder.switch_to_block(fallthrough);
                self.builder.seal_block(fallthrough);
            }

            OpCode::JumpIfTrue => {
                let offset = self.read_i16(start_ip + 1);
                self.ip += 2;
                let target = ((start_ip as isize + 3) + offset as isize) as usize;

                let (tag, data) = self.pop();
                let target_block = self.get_block(target);
                let fallthrough = self.builder.create_block();

                // Check if value is truthy (not null and not false)
                let is_null = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Null as i64);
                let is_bool = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Bool as i64);
                let is_false = self.builder.ins().icmp_imm(IntCC::Equal, data, 0);
                let bool_and_false = self.builder.ins().band(is_bool, is_false);
                let is_falsy = self.builder.ins().bor(is_null, bool_and_false);
                let is_truthy = self.builder.ins().bnot(is_falsy);
                let one = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, 1);
                let is_truthy_bool = self.builder.ins().band(is_truthy, one);

                self.builder.ins().brif(is_truthy_bool, target_block, &[], fallthrough, &[]);
                self.builder.switch_to_block(fallthrough);
                self.builder.seal_block(fallthrough);
            }

            OpCode::Loop => {
                let offset = self.read_i16(start_ip + 1);
                self.ip += 2;
                let target = ((start_ip as isize + 3) + offset as isize) as usize;
                let block = self.get_block(target);
                self.builder.ins().jump(block, &[]);
                self.block_terminated = true;
            }

            OpCode::Return => {
                let (tag, data) = if self.stack_depth > 0 {
                    self.pop()
                } else {
                    let null_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Null as i64);
                    let null_data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, 0);
                    (null_tag, null_data)
                };
                self.builder.ins().return_(&[tag, data]);
                self.block_terminated = true;
            }

            OpCode::PopBelow => {
                // Pop N values below the top
                let count = self.read_u8(start_ip + 1) as usize;
                self.ip += 1;

                if count > 0 && self.stack_depth > 1 {
                    // Save top of stack
                    let top = self.pop();
                    // Pop count-1 more values
                    for _ in 0..(count.min(self.stack_depth)) {
                        let _ = self.pop();
                    }
                    // Push top back
                    self.push(top.0, top.1);
                }
            }

            OpCode::IsNull => {
                let (tag, _data) = self.pop();
                let is_null = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Null as i64);
                let result = self.builder.ins().uextend(CraneliftTypes::VALUE_SECOND, is_null);
                let bool_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Bool as i64);
                self.push(bool_tag, result);
            }

            OpCode::JumpIfNull => {
                let offset = self.read_i16(start_ip + 1);
                self.ip += 2;
                let target = ((start_ip as isize + 3) + offset as isize) as usize;

                let (tag, _data) = self.peek();
                let target_block = self.get_block(target);
                let fallthrough = self.builder.create_block();

                let is_null = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Null as i64);
                self.builder.ins().brif(is_null, target_block, &[], fallthrough, &[]);
                self.builder.switch_to_block(fallthrough);
                self.builder.seal_block(fallthrough);
            }

            OpCode::JumpIfNotNull => {
                let offset = self.read_i16(start_ip + 1);
                self.ip += 2;
                let target = ((start_ip as isize + 3) + offset as isize) as usize;

                let (tag, _data) = self.peek();
                let target_block = self.get_block(target);
                let fallthrough = self.builder.create_block();

                let is_null = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Null as i64);
                let is_not_null = self.builder.ins().bnot(is_null);
                let one = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, 1);
                let is_not_null_bool = self.builder.ins().band(is_not_null, one);

                self.builder.ins().brif(is_not_null_bool, target_block, &[], fallthrough, &[]);
                self.builder.switch_to_block(fallthrough);
                self.builder.seal_block(fallthrough);
            }

            // Instructions not yet implemented - emit runtime call or error
            // These cause fallback to interpreter which is the correct behavior
            _ => {
                return Err(JitError::UnsupportedInstruction(format!("{:?}", op)));
            }
        }

        Ok(())
    }

    /// Compile a constant value
    fn compile_constant(&mut self, value: &Value) -> JitResult<()> {
        match value {
            Value::Null => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Null as i64);
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, 0);
                self.push(tag, data);
            }
            Value::Bool(b) => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Bool as i64);
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, if *b { 1 } else { 0 });
                self.push(tag, data);
            }
            Value::Int(i) => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Int as i64);
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, *i);
                self.push(tag, data);
            }
            Value::Float(f) => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Float as i64);
                let bits = f.to_bits() as i64;
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, bits);
                self.push(tag, data);
            }
            Value::String(s) => {
                let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::String as i64);
                // Store the Rc pointer directly
                let ptr = Rc::as_ptr(s) as i64;
                let data = self.builder.ins().iconst(CraneliftTypes::VALUE_SECOND, ptr);
                self.push(tag, data);
            }
            _ => {
                // For complex types, we'd need to call into the runtime
                return Err(JitError::UnsupportedInstruction(format!(
                    "constant type {:?} not yet supported",
                    value.type_name()
                )));
            }
        }
        Ok(())
    }

    /// Compile a binary arithmetic operation
    fn compile_binary_op(&mut self, op: BinaryOp) -> JitResult<()> {
        let (_right_tag, right_data) = self.pop();
        let (left_tag, left_data) = self.pop();

        // For now, assume both are integers or both are floats
        // In a complete implementation, we'd check tags and dispatch appropriately

        // Create blocks for int and float paths
        let int_block = self.builder.create_block();
        let float_block = self.builder.create_block();
        let merge_block = self.builder.create_block();

        // Add block params for the merge block (result tag and data)
        self.builder.append_block_param(merge_block, CraneliftTypes::VALUE_FIRST);
        self.builder.append_block_param(merge_block, CraneliftTypes::VALUE_SECOND);

        // Check if left is an integer
        let is_int = self.builder.ins().icmp_imm(IntCC::Equal, left_tag, ValueTag::Int as i64);
        self.builder.ins().brif(is_int, int_block, &[], float_block, &[]);

        // Integer path
        self.builder.switch_to_block(int_block);
        let int_result = match op {
            BinaryOp::Add => self.builder.ins().iadd(left_data, right_data),
            BinaryOp::Sub => self.builder.ins().isub(left_data, right_data),
            BinaryOp::Mul => self.builder.ins().imul(left_data, right_data),
            BinaryOp::Div => self.builder.ins().sdiv(left_data, right_data),
            BinaryOp::Mod => self.builder.ins().srem(left_data, right_data),
        };
        let int_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Int as i64);
        self.builder.ins().jump(merge_block, &[int_tag, int_result]);
        self.builder.seal_block(int_block);

        // Float path
        self.builder.switch_to_block(float_block);
        let left_float = self.builder.ins().bitcast(CraneliftTypes::FLOAT, MemFlags::new(), left_data);
        let right_float = self.builder.ins().bitcast(CraneliftTypes::FLOAT, MemFlags::new(), right_data);
        let float_result = match op {
            BinaryOp::Add => self.builder.ins().fadd(left_float, right_float),
            BinaryOp::Sub => self.builder.ins().fsub(left_float, right_float),
            BinaryOp::Mul => self.builder.ins().fmul(left_float, right_float),
            BinaryOp::Div => self.builder.ins().fdiv(left_float, right_float),
            BinaryOp::Mod => {
                // Cranelift doesn't have fmod, would need to call into runtime
                return Err(JitError::UnsupportedInstruction("float modulo".to_string()));
            }
        };
        let float_data = self.builder.ins().bitcast(CraneliftTypes::VALUE_SECOND, MemFlags::new(), float_result);
        let float_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Float as i64);
        self.builder.ins().jump(merge_block, &[float_tag, float_data]);
        self.builder.seal_block(float_block);

        // Merge block
        self.builder.switch_to_block(merge_block);
        let result_tag = self.builder.block_params(merge_block)[0];
        let result_data = self.builder.block_params(merge_block)[1];
        self.push(result_tag, result_data);
        self.builder.seal_block(merge_block);

        Ok(())
    }

    /// Compile unary negation
    fn compile_unary_neg(&mut self) -> JitResult<()> {
        let (tag, data) = self.pop();

        // Create blocks for int and float paths
        let int_block = self.builder.create_block();
        let float_block = self.builder.create_block();
        let merge_block = self.builder.create_block();

        self.builder.append_block_param(merge_block, CraneliftTypes::VALUE_FIRST);
        self.builder.append_block_param(merge_block, CraneliftTypes::VALUE_SECOND);

        let is_int = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Int as i64);
        self.builder.ins().brif(is_int, int_block, &[], float_block, &[]);

        // Integer path
        self.builder.switch_to_block(int_block);
        let neg_int = self.builder.ins().ineg(data);
        let int_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Int as i64);
        self.builder.ins().jump(merge_block, &[int_tag, neg_int]);
        self.builder.seal_block(int_block);

        // Float path
        self.builder.switch_to_block(float_block);
        let float_val = self.builder.ins().bitcast(CraneliftTypes::FLOAT, MemFlags::new(), data);
        let neg_float = self.builder.ins().fneg(float_val);
        let neg_data = self.builder.ins().bitcast(CraneliftTypes::VALUE_SECOND, MemFlags::new(), neg_float);
        let float_tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Float as i64);
        self.builder.ins().jump(merge_block, &[float_tag, neg_data]);
        self.builder.seal_block(float_block);

        // Merge
        self.builder.switch_to_block(merge_block);
        let result_tag = self.builder.block_params(merge_block)[0];
        let result_data = self.builder.block_params(merge_block)[1];
        self.push(result_tag, result_data);
        self.builder.seal_block(merge_block);

        Ok(())
    }

    /// Compile a comparison operation
    fn compile_comparison(&mut self, cc: IntCC) -> JitResult<()> {
        let (_right_tag, right_data) = self.pop();
        let (_left_tag, left_data) = self.pop();

        // For integers, use integer comparison
        // For floats, we'd need float comparison
        // For now, assume integers

        let result = self.builder.ins().icmp(cc, left_data, right_data);
        let result_data = self.builder.ins().uextend(CraneliftTypes::VALUE_SECOND, result);
        let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Bool as i64);

        self.push(tag, result_data);
        Ok(())
    }

    /// Compile logical NOT
    fn compile_unary_not(&mut self) -> JitResult<()> {
        let (tag, data) = self.pop();

        // Value is falsy if null or (bool and false)
        let is_null = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Null as i64);
        let is_bool = self.builder.ins().icmp_imm(IntCC::Equal, tag, ValueTag::Bool as i64);
        let is_false = self.builder.ins().icmp_imm(IntCC::Equal, data, 0);
        let bool_and_false = self.builder.ins().band(is_bool, is_false);
        let is_falsy = self.builder.ins().bor(is_null, bool_and_false);

        // NOT falsy = truthy
        let result = self.builder.ins().uextend(CraneliftTypes::VALUE_SECOND, is_falsy);
        let tag = self.builder.ins().iconst(CraneliftTypes::VALUE_FIRST, ValueTag::Bool as i64);

        self.push(tag, result);
        Ok(())
    }
}

/// Binary operations
#[derive(Debug, Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jit_compiler_creates() {
        let compiler = JitCompiler::new();
        assert!(compiler.compiled_functions.is_empty());
    }

    #[test]
    fn jit_compile_simple_int() {
        let mut compiler = JitCompiler::new();

        let func = compiler.compile_simple_int_function("add_numbers", |builder, result| {
            // Compute 10 + 32
            let a = builder.ins().iconst(CraneliftTypes::INT, 10);
            let b = builder.ins().iconst(CraneliftTypes::INT, 32);
            let sum = builder.ins().iadd(a, b);
            builder.def_var(result, sum);
            Ok(())
        }).unwrap();

        assert_eq!(func(), 42);
    }

    #[test]
    fn jit_compile_arithmetic() {
        let mut compiler = JitCompiler::new();

        let func = compiler.compile_simple_int_function("complex_math", |builder, result| {
            // Compute (5 * 8) + (100 / 4) - 15 = 40 + 25 - 15 = 50
            let a = builder.ins().iconst(CraneliftTypes::INT, 5);
            let b = builder.ins().iconst(CraneliftTypes::INT, 8);
            let mul = builder.ins().imul(a, b);

            let c = builder.ins().iconst(CraneliftTypes::INT, 100);
            let d = builder.ins().iconst(CraneliftTypes::INT, 4);
            let div = builder.ins().sdiv(c, d);

            let add = builder.ins().iadd(mul, div);

            let e = builder.ins().iconst(CraneliftTypes::INT, 15);
            let sub = builder.ins().isub(add, e);

            builder.def_var(result, sub);
            Ok(())
        }).unwrap();

        assert_eq!(func(), 50);
    }

    #[test]
    fn jit_compile_conditionals() {
        let mut compiler = JitCompiler::new();

        let func = compiler.compile_simple_int_function("conditional", |builder, result| {
            // if 10 > 5 { 100 } else { 200 }
            let a = builder.ins().iconst(CraneliftTypes::INT, 10);
            let b = builder.ins().iconst(CraneliftTypes::INT, 5);
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, a, b);

            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            builder.append_block_param(merge_block, CraneliftTypes::INT);

            builder.ins().brif(cmp, then_block, &[], else_block, &[]);

            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            let then_val = builder.ins().iconst(CraneliftTypes::INT, 100);
            builder.ins().jump(merge_block, &[then_val]);

            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            let else_val = builder.ins().iconst(CraneliftTypes::INT, 200);
            builder.ins().jump(merge_block, &[else_val]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let phi_val = builder.block_params(merge_block)[0];
            builder.def_var(result, phi_val);

            Ok(())
        }).unwrap();

        assert_eq!(func(), 100);
    }
}
