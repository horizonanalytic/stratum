//! Bytecode compiler - transforms AST into bytecode

use std::rc::Rc;

use crate::ast::{
    BinOp, Block, CallArg, CatchClause, CompoundOp, ElseBranch, ExecutionMode, ExecutionModeOverride,
    Expr, ExprKind, FieldInit, Function, Ident, Item, ItemKind, Literal, MatchArm, Module, Param,
    Pattern, PatternKind, Stmt, StmtKind, StringPart, TopLevelItem, TopLevelLet, UnaryOp,
};
use crate::lexer::Span;

use super::chunk::Chunk;
use super::error::{CompileError, CompileErrorKind};
use super::opcode::OpCode;
use super::value::{Function as BytecodeFunction, Value};

/// A local variable in scope
#[derive(Debug, Clone)]
struct Local {
    /// Variable name
    name: String,

    /// Scope depth (0 = global)
    depth: u32,

    /// Whether the variable has been initialized
    initialized: bool,

    /// Whether the variable is captured by a closure
    is_captured: bool,
}

/// An upvalue (captured variable from outer scope)
#[derive(Debug, Clone, Copy)]
struct Upvalue {
    /// Index of the variable in the enclosing scope
    index: u8,

    /// Whether it's a local in the immediately enclosing function
    is_local: bool,
}

/// Loop information for break/continue
#[derive(Debug, Clone)]
struct LoopInfo {
    /// Bytecode offset of loop start (for continue)
    start: usize,

    /// Scope depth when entering the loop
    scope_depth: u32,

    /// Offsets of break jumps to patch
    break_jumps: Vec<usize>,
}

/// Function type being compiled
#[derive(Debug, Clone, Copy, PartialEq)]
enum FunctionType {
    /// Top-level script
    Script,

    /// Regular function
    Function,

    /// Method
    Method,

    /// Initializer (constructor)
    Initializer,
}

/// Compiler state for a single function
struct CompilerState {
    /// The function being compiled
    function: BytecodeFunction,

    /// Function type
    function_type: FunctionType,

    /// Local variables
    locals: Vec<Local>,

    /// Upvalues
    upvalues: Vec<Upvalue>,

    /// Current scope depth
    scope_depth: u32,

    /// Active loops (for break/continue)
    loops: Vec<LoopInfo>,

    /// Enclosing compiler state (for nested functions)
    enclosing: Option<Box<CompilerState>>,

    /// Whether this function is async (reserved for future async compilation)
    #[allow(dead_code)]
    is_async: bool,
}

impl CompilerState {
    fn new(function_type: FunctionType, name: String, is_async: bool) -> Self {
        let mut state = Self {
            function: BytecodeFunction::new(name, 0),
            function_type,
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            loops: Vec::new(),
            enclosing: None,
            is_async,
        };

        // Reserve slot 0 for 'this' in methods or empty slot in functions
        let first_local = if function_type == FunctionType::Method
            || function_type == FunctionType::Initializer
        {
            Local {
                name: "this".to_string(),
                depth: 0,
                initialized: true,
                is_captured: false,
            }
        } else {
            Local {
                name: String::new(),
                depth: 0,
                initialized: true,
                is_captured: false,
            }
        };
        state.locals.push(first_local);

        state
    }

    fn chunk(&self) -> &Chunk {
        &self.function.chunk
    }

    fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.function.chunk
    }
}

/// Bytecode compiler
pub struct Compiler {
    /// Current compiler state
    current: CompilerState,

    /// Collected errors
    errors: Vec<CompileError>,

    /// Source file name
    source_name: Option<String>,

    /// Module-level execution mode (from #![compile] or #![interpret] directives)
    module_mode: Option<ExecutionMode>,

    /// CLI override for execution mode (overrides all directives)
    mode_override: Option<ExecutionModeOverride>,
}

impl Compiler {
    /// Create a new compiler
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: CompilerState::new(FunctionType::Script, "<script>".to_string(), false),
            errors: Vec::new(),
            source_name: None,
            module_mode: None,
            mode_override: None,
        }
    }

    /// Create a new compiler with a source name
    #[must_use]
    pub fn with_source(source_name: impl Into<String>) -> Self {
        let name = source_name.into();
        let mut compiler = Self::new();
        compiler.source_name = Some(name.clone());
        compiler.current.function.chunk.source_name = Some(name);
        compiler
    }

    /// Set the execution mode override for this compilation
    ///
    /// When set, this overrides all function and module-level directives.
    #[must_use]
    pub fn with_mode_override(mut self, mode_override: Option<ExecutionModeOverride>) -> Self {
        self.mode_override = mode_override;
        self
    }

    /// Resolve the execution mode for a function, considering overrides and defaults
    fn resolve_function_mode(&self, func: &Function) -> ExecutionMode {
        // CLI override takes precedence over everything
        if let Some(override_mode) = self.mode_override {
            return match override_mode {
                ExecutionModeOverride::InterpretAll => ExecutionMode::Interpret,
                ExecutionModeOverride::CompileAll => ExecutionMode::Compile,
            };
        }

        // Otherwise, use the function's resolution with module default
        func.resolve_execution_mode(self.module_mode, ExecutionMode::Interpret)
    }

    /// Compile a module to bytecode
    pub fn compile_module(mut self, module: &Module) -> Result<Rc<BytecodeFunction>, Vec<CompileError>> {
        // Capture module-level execution mode from inner attributes (e.g., #![compile])
        self.module_mode = module.execution_mode();

        // First pass: compile all function definitions (hoisted)
        // This ensures functions are available before they're called
        for tl_item in &module.top_level {
            if let TopLevelItem::Item(item) = tl_item {
                if matches!(item.kind, ItemKind::Function(_)) {
                    self.compile_item(item);
                }
            }
        }

        // Second pass: compile top-level lets, statements, and non-function items in order
        for tl_item in &module.top_level {
            self.compile_top_level_item(tl_item);
        }

        // Emit implicit return
        self.emit_return(module.span);

        if self.errors.is_empty() {
            Ok(Rc::new(self.current.function))
        } else {
            Err(self.errors)
        }
    }

    /// Compile a top-level item
    fn compile_top_level_item(&mut self, tl_item: &TopLevelItem) {
        match tl_item {
            TopLevelItem::Item(item) => {
                // Functions are compiled in the first pass (hoisted), skip them here
                if !matches!(item.kind, ItemKind::Function(_)) {
                    self.compile_item(item);
                }
            }
            TopLevelItem::Let(let_decl) => self.compile_top_level_let(let_decl),
            TopLevelItem::Statement(stmt) => {
                // The statement() method handles popping for expression statements
                self.statement(stmt);
            }
        }
    }

    /// Compile a top-level let declaration
    fn compile_top_level_let(&mut self, let_decl: &TopLevelLet) {
        let line = self.line_from_span(let_decl.span);

        // Compile the value expression
        self.expression(&let_decl.value);

        // Handle the pattern binding
        // For now, we only support simple identifier patterns at the top level
        match &let_decl.pattern.kind {
            PatternKind::Ident(ident) => {
                // Declare and define as a global variable
                self.declare_variable(ident);
                self.define_variable(ident, line);
            }
            PatternKind::Wildcard => {
                // Just pop the value - it's not bound to anything
                self.emit_op(OpCode::Pop, line);
            }
            _ => {
                // For complex patterns, we need destructuring support
                // For now, emit an error
                self.error(CompileErrorKind::UnsupportedPattern, let_decl.pattern.span);
            }
        }
    }

    /// Compile a single expression (for REPL)
    pub fn compile_expression(mut self, expr: &Expr) -> Result<Rc<BytecodeFunction>, Vec<CompileError>> {
        self.expression(expr);
        // The expression result is already on the stack, just emit Return
        let line = self.line_from_span(expr.span);
        self.emit_op(OpCode::Return, line);

        if self.errors.is_empty() {
            Ok(Rc::new(self.current.function))
        } else {
            Err(self.errors)
        }
    }

    /// Compile a test function (used by the test runner)
    /// This compiles the function and wraps it in a call
    pub fn compile_test_function(
        mut self,
        func: &Function,
    ) -> Result<Rc<BytecodeFunction>, Vec<CompileError>> {
        let line = self.line_from_span(func.span);

        // Compile the function definition (registers it as a global)
        self.compile_function_def(func);

        // Emit a call to the function
        if let Some(name_constant) = self.identifier_constant(&func.name.name, func.span) {
            self.emit_op_u16(OpCode::LoadGlobal, name_constant, line);

            // Call with 0 arguments
            self.emit_op(OpCode::Call, line);
            self.emit_byte(0, line);
        }

        // Return the result
        self.emit_op(OpCode::Return, line);

        if self.errors.is_empty() {
            Ok(Rc::new(self.current.function))
        } else {
            Err(self.errors)
        }
    }

    /// Compile REPL input (expression, statement(s), or function definition)
    ///
    /// For expressions: compiles and returns the result
    /// For statements: compiles, executes, and returns null
    /// For functions: compiles and registers globally, returns null
    pub fn compile_repl_input(
        mut self,
        input: &crate::parser::ReplInput,
    ) -> Result<Rc<BytecodeFunction>, Vec<CompileError>> {
        use crate::parser::ReplInput;

        match input {
            ReplInput::Expression(expr) => {
                // Compile expression and return its value
                self.expression(expr);
                let line = self.line_from_span(expr.span);
                self.emit_op(OpCode::Return, line);
            }
            ReplInput::Statement(stmt) => {
                // Compile statement(s) and return null
                self.statement(stmt);
                let line = self.line_from_span(stmt.span);
                self.emit_op(OpCode::Null, line);
                self.emit_op(OpCode::Return, line);
            }
            ReplInput::Statements(stmts) => {
                // Compile multiple statements and return the last expression value (or null)
                let mut last_span = Span::new(0, 0);
                for stmt in stmts {
                    self.statement(stmt);
                    last_span = stmt.span;
                }
                // If the last statement was an expression, we need to retrieve its value
                // For now, just return null (the value was popped by statement())
                self.emit_op(OpCode::Null, self.line_from_span(last_span));
                self.emit_op(OpCode::Return, self.line_from_span(last_span));
            }
            ReplInput::Function(func) => {
                // Compile function definition and return null
                let line = self.line_from_span(func.span);
                self.compile_function_def(func);
                self.emit_op(OpCode::Null, line);
                self.emit_op(OpCode::Return, line);
            }
        }

        if self.errors.is_empty() {
            Ok(Rc::new(self.current.function))
        } else {
            Err(self.errors)
        }
    }

    // ===== Item Compilation =====

    fn compile_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function(func) => self.compile_function_def(func),
            ItemKind::Struct(_def) => {
                // Structs are handled at runtime through type info
                // The compiler just needs to know the field names/order
                // This will be more fully implemented when we add the VM
            }
            ItemKind::Enum(_def) => {
                // Similarly, enums are mostly handled at runtime
            }
            ItemKind::Interface(_def) => {
                // Interfaces are checked at compile time by the type checker
                // No bytecode generation needed
            }
            ItemKind::Impl(_def) => {
                // Impl blocks attach methods to types
                // This will be handled when we add method dispatch
            }
            ItemKind::Import(_import) => {
                // Imports are resolved by the module system
                // Will be implemented with the module loader
            }
        }
    }

    fn compile_function_def(&mut self, func: &Function) {
        let name = func.name.name.clone();
        let line = self.line_from_span(func.span);

        // Declare the function name in current scope
        self.declare_variable(&func.name);
        if self.current.scope_depth > 0 {
            self.mark_initialized();
        }

        // Compile the function body
        self.function(func, FunctionType::Function);

        // Define the global (if at top level)
        self.define_variable(&func.name, line);

        // Store function name in constants for reference
        let _ = self.current.chunk_mut().add_constant(Value::string(name));
    }

    fn function(&mut self, func: &Function, function_type: FunctionType) {
        let name = func.name.name.clone();
        let _line = self.line_from_span(func.span);

        // Start a new compiler state for the function
        let enclosing = std::mem::replace(
            &mut self.current,
            CompilerState::new(function_type, name, func.is_async),
        );
        self.current.enclosing = Some(Box::new(enclosing));
        self.begin_scope();

        // Compile parameters
        for param in &func.params {
            if self.current.function.arity == 255 {
                self.error(CompileErrorKind::TooManyParameters, param.span);
                break;
            }
            self.current.function.arity += 1;
            self.declare_variable(&param.name);
            self.mark_initialized();
        }

        // Compile body statements
        for stmt in &func.body.stmts {
            self.statement(stmt);
        }

        // Compile trailing expression if present (this is the return value)
        let line = self.line_from_span(func.span);
        if let Some(expr) = &func.body.expr {
            self.expression(expr);
            self.emit_op(OpCode::Return, line);
        } else {
            // No trailing expression - emit null and return
            self.emit_return(func.span);
        }

        // End function scope
        self.end_scope(line);

        // Get the completed function - need to take enclosing first to avoid borrow issue
        let enclosing = self.current.enclosing.take().unwrap();
        let function_state = std::mem::replace(&mut self.current, *enclosing);

        // Emit closure instruction
        let upvalue_count = function_state.upvalues.len();
        let mut completed_function = function_state.function;
        completed_function.upvalue_count = upvalue_count as u16;

        // Set execution mode based on function attributes and module mode
        completed_function.execution_mode = self.resolve_function_mode(func);

        let func_value = Value::Function(Rc::new(completed_function));
        if let Some(const_idx) = self.current.chunk_mut().add_constant(func_value) {
            self.emit_op_u16(OpCode::Closure, const_idx, line);

            // Emit upvalue descriptors
            for upvalue in &function_state.upvalues {
                self.emit_byte(if upvalue.is_local { 1 } else { 0 }, line);
                self.emit_byte(upvalue.index, line);
            }
        } else {
            self.error(CompileErrorKind::TooManyConstants, func.span);
        }
    }

    // ===== Statement Compilation =====

    fn statement(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, ty: _, value } => {
                self.let_statement(pattern, value, stmt.span);
            }
            StmtKind::Expr(expr) => {
                self.expression(expr);
                self.emit_op(OpCode::Pop, self.line_from_span(stmt.span));
            }
            StmtKind::Assign { target, value } => {
                self.assignment(target, value, stmt.span);
                self.emit_op(OpCode::Pop, self.line_from_span(stmt.span));
            }
            StmtKind::CompoundAssign { target, op, value } => {
                self.compound_assignment(target, *op, value, stmt.span);
                self.emit_op(OpCode::Pop, self.line_from_span(stmt.span));
            }
            StmtKind::Return(expr) => {
                self.return_statement(expr.as_ref(), stmt.span);
            }
            StmtKind::For { pattern, iter, body } => {
                self.for_loop(pattern, iter, body, stmt.span);
            }
            StmtKind::While { cond, body } => {
                self.while_loop(cond, body, stmt.span);
            }
            StmtKind::Loop { body } => {
                self.infinite_loop(body, stmt.span);
            }
            StmtKind::Break => {
                self.break_statement(stmt.span);
            }
            StmtKind::Continue => {
                self.continue_statement(stmt.span);
            }
            StmtKind::TryCatch {
                try_block,
                catches,
                finally,
            } => {
                self.try_catch(try_block, catches, finally.as_ref(), stmt.span);
            }
            StmtKind::Throw(expr) => {
                self.expression(expr);
                self.emit_op(OpCode::Throw, self.line_from_span(stmt.span));
            }
        }
    }

    fn let_statement(&mut self, pattern: &Pattern, value: &Expr, span: Span) {
        // For now, only handle simple identifier patterns
        match &pattern.kind {
            PatternKind::Ident(name) => {
                self.declare_variable(name);
                self.expression(value);
                self.define_variable(name, self.line_from_span(span));
            }
            _ => {
                // Pattern destructuring will be implemented later
                self.error(
                    CompileErrorKind::Unsupported("pattern destructuring in let".to_string()),
                    pattern.span,
                );
            }
        }
    }

    fn assignment(&mut self, target: &Expr, value: &Expr, span: Span) {
        let line = self.line_from_span(span);

        match &target.kind {
            ExprKind::Ident(name) => {
                self.expression(value);
                self.set_variable(&name.name, line, span);
            }
            ExprKind::Field { expr, field } => {
                self.expression(expr);
                self.expression(value);
                if let Some(idx) = self.identifier_constant(&field.name, span) {
                    self.emit_op_u16(OpCode::SetField, idx, line);
                }
            }
            ExprKind::Index { expr, index } => {
                self.expression(expr);
                self.expression(index);
                self.expression(value);
                self.emit_op(OpCode::SetIndex, line);
            }
            _ => {
                self.error(CompileErrorKind::InvalidAssignmentTarget, target.span);
            }
        }
    }

    fn compound_assignment(&mut self, target: &Expr, op: CompoundOp, value: &Expr, span: Span) {
        let line = self.line_from_span(span);

        match &target.kind {
            ExprKind::Ident(name) => {
                // Load current value
                self.get_variable(&name.name, line, span);
                // Compute new value
                self.expression(value);
                // Apply operation
                match op {
                    CompoundOp::Add => self.emit_op(OpCode::Add, line),
                    CompoundOp::Sub => self.emit_op(OpCode::Sub, line),
                    CompoundOp::Mul => self.emit_op(OpCode::Mul, line),
                    CompoundOp::Div => self.emit_op(OpCode::Div, line),
                    CompoundOp::Mod => self.emit_op(OpCode::Mod, line),
                }
                // Store result
                self.set_variable(&name.name, line, span);
            }
            _ => {
                // For field and index assignment, we need to be careful
                // to not evaluate the target twice. For now, just error.
                self.error(
                    CompileErrorKind::Unsupported(
                        "compound assignment on fields/indices".to_string(),
                    ),
                    target.span,
                );
            }
        }
    }

    fn return_statement(&mut self, value: Option<&Expr>, span: Span) {
        let line = self.line_from_span(span);

        if self.current.function_type == FunctionType::Script {
            self.error(CompileErrorKind::ReturnOutsideFunction, span);
            return;
        }

        if let Some(expr) = value {
            if self.current.function_type == FunctionType::Initializer {
                // Initializers can't return a value
                self.error(
                    CompileErrorKind::Unsupported("return value in initializer".to_string()),
                    span,
                );
            }
            self.expression(expr);
        } else {
            self.emit_op(OpCode::Null, line);
        }

        self.emit_op(OpCode::Return, line);
    }

    fn for_loop(&mut self, pattern: &Pattern, iter: &Expr, body: &Block, span: Span) {
        let line = self.line_from_span(span);

        // Begin new scope for loop variable
        self.begin_scope();

        // Evaluate iterator expression and convert to iterator
        self.expression(iter);
        self.emit_op(OpCode::GetIter, line);

        // Store iterator in a hidden local
        let iter_slot = self.current.locals.len();
        self.current.locals.push(Local {
            name: String::new(), // Anonymous
            depth: self.current.scope_depth,
            initialized: true,
            is_captured: false,
        });

        // Loop start
        let loop_start = self.current.chunk().current_offset();
        self.current.loops.push(LoopInfo {
            start: loop_start,
            scope_depth: self.current.scope_depth,
            break_jumps: Vec::new(),
        });

        // Get next item or jump to end
        // LoadLocal pushes a copy of the iterator onto the stack
        // IterNext peeks that copy and pushes the next value (or jumps if exhausted)
        self.emit_op_u16(OpCode::LoadLocal, iter_slot as u16, line);
        let exit_jump = self.emit_jump(OpCode::IterNext, line);

        // After IterNext (when not jumping), stack has: [..., iterator_copy, next_value]
        // Use PopBelow to remove the iterator copy while keeping next_value on top
        // This ensures next_value is at the correct slot for the loop variable
        self.emit_op_u8(OpCode::PopBelow, 1, line);

        // Bind loop variable (now at the correct stack slot)
        match &pattern.kind {
            PatternKind::Ident(name) => {
                self.declare_variable(name);
                self.mark_initialized();
            }
            _ => {
                self.error(
                    CompileErrorKind::Unsupported("pattern destructuring in for".to_string()),
                    pattern.span,
                );
            }
        }

        // Compile body
        self.block(body);

        // Pop loop variable (but keep iterator in its slot)
        self.emit_op(OpCode::Pop, line);

        // Loop back
        self.emit_loop(loop_start, line);

        // Patch exit jump - when IterNext jumps here, stack has: [..., iterator_copy]
        // (IterNext pushed nothing because iterator was exhausted)
        self.patch_jump(exit_jump);

        // Pop the iterator copy that LoadLocal pushed
        self.emit_op(OpCode::Pop, line);

        // Patch break jumps
        let loop_info = self.current.loops.pop().unwrap();
        for jump in loop_info.break_jumps {
            self.patch_jump(jump);
        }

        self.end_scope(line);
    }

    fn while_loop(&mut self, cond: &Expr, body: &Block, span: Span) {
        let line = self.line_from_span(span);

        let loop_start = self.current.chunk().current_offset();
        self.current.loops.push(LoopInfo {
            start: loop_start,
            scope_depth: self.current.scope_depth,
            break_jumps: Vec::new(),
        });

        // Condition
        self.expression(cond);
        let exit_jump = self.emit_jump(OpCode::JumpIfFalse, line);
        // Note: JumpIfFalse already pops the condition, no need for explicit Pop

        // Body
        self.block(body);

        // Loop back
        self.emit_loop(loop_start, line);

        // Exit
        self.patch_jump(exit_jump);
        // Note: JumpIfFalse already popped the condition when jumping here

        // Patch break jumps
        let loop_info = self.current.loops.pop().unwrap();
        for jump in loop_info.break_jumps {
            self.patch_jump(jump);
        }
    }

    fn infinite_loop(&mut self, body: &Block, span: Span) {
        let line = self.line_from_span(span);

        let loop_start = self.current.chunk().current_offset();
        self.current.loops.push(LoopInfo {
            start: loop_start,
            scope_depth: self.current.scope_depth,
            break_jumps: Vec::new(),
        });

        // Body
        self.block(body);

        // Loop back
        self.emit_loop(loop_start, line);

        // Patch break jumps
        let loop_info = self.current.loops.pop().unwrap();
        for jump in loop_info.break_jumps {
            self.patch_jump(jump);
        }
    }

    fn break_statement(&mut self, span: Span) {
        let line = self.line_from_span(span);

        if self.current.loops.is_empty() {
            self.error(CompileErrorKind::BreakOutsideLoop, span);
            return;
        }

        // Close any locals in inner scopes
        let loop_depth = self.current.loops.last().unwrap().scope_depth;
        self.close_upvalues_to_depth(loop_depth, line);

        // Jump to after loop (will be patched)
        let jump = self.emit_jump(OpCode::Jump, line);
        self.current.loops.last_mut().unwrap().break_jumps.push(jump);
    }

    fn continue_statement(&mut self, span: Span) {
        let line = self.line_from_span(span);

        if self.current.loops.is_empty() {
            self.error(CompileErrorKind::ContinueOutsideLoop, span);
            return;
        }

        // Close any locals in inner scopes
        let loop_depth = self.current.loops.last().unwrap().scope_depth;
        self.close_upvalues_to_depth(loop_depth, line);

        // Jump to loop start
        let loop_start = self.current.loops.last().unwrap().start;
        self.emit_loop(loop_start, line);
    }

    fn try_catch(
        &mut self,
        try_block: &Block,
        catches: &[CatchClause],
        finally: Option<&Block>,
        span: Span,
    ) {
        let line = self.line_from_span(span);

        // Emit PushHandler with placeholders for catch and finally offsets
        self.emit_op(OpCode::PushHandler, line);
        let catch_offset_pos = self.current.chunk().current_offset();
        self.emit_byte(0, line); // catch offset placeholder byte 1
        self.emit_byte(0, line); // catch offset placeholder byte 2
        let finally_offset_pos = self.current.chunk().current_offset();
        self.emit_byte(0, line); // finally offset placeholder byte 1
        self.emit_byte(0, line); // finally offset placeholder byte 2

        // Compile try block
        self.block(try_block);

        // Pop handler on normal exit
        self.emit_op(OpCode::PopHandler, line);

        // Jump over catch blocks
        let end_jump = self.emit_jump(OpCode::Jump, line);

        // Patch catch offset to here (offset is from AFTER reading both offsets, i.e., finally_offset_pos + 2)
        let catch_target = self.current.chunk().current_offset();
        let catch_offset = (catch_target as isize - (finally_offset_pos as isize + 2)) as i16;
        self.current.chunk_mut().patch_i16(catch_offset_pos, catch_offset);

        // Compile catch clauses
        for catch in catches {
            self.begin_scope();

            // If there's a binding, declare it
            if let Some(binding) = &catch.binding {
                self.declare_variable(binding);
                self.mark_initialized();
            } else {
                // Pop the exception if not bound
                self.emit_op(OpCode::Pop, line);
            }

            self.block(&catch.body);
            self.end_scope(line);
        }

        // Patch end jump
        self.patch_jump(end_jump);

        // Compile finally block if present
        if let Some(finally_block) = finally {
            self.block(finally_block);
        }
    }

    // ===== Expression Compilation =====

    fn expression(&mut self, expr: &Expr) {
        let line = self.line_from_span(expr.span);

        match &expr.kind {
            ExprKind::Literal(lit) => self.literal(lit, line, expr.span),

            ExprKind::Ident(name) => {
                self.get_variable(&name.name, line, expr.span);
            }

            ExprKind::Binary { left, op, right } => {
                self.binary(left, *op, right, line, expr.span);
            }

            ExprKind::Unary { op, expr: inner } => {
                self.expression(inner);
                match op {
                    UnaryOp::Neg => self.emit_op(OpCode::Neg, line),
                    UnaryOp::Not => self.emit_op(OpCode::Not, line),
                }
            }

            ExprKind::Paren(inner) => {
                self.expression(inner);
            }

            ExprKind::Call {
                callee,
                args,
                trailing_closure,
            } => {
                self.call(callee, args, trailing_closure.as_deref(), line, expr.span);
            }

            ExprKind::Index {
                expr: target,
                index,
            } => {
                self.expression(target);
                self.expression(index);
                self.emit_op(OpCode::GetIndex, line);
            }

            ExprKind::Field {
                expr: target,
                field,
            } => {
                self.expression(target);
                if let Some(idx) = self.identifier_constant(&field.name, expr.span) {
                    self.emit_op_u16(OpCode::GetField, idx, line);
                }
            }

            ExprKind::NullSafeField {
                expr: target,
                field,
            } => {
                self.expression(target);
                if let Some(idx) = self.identifier_constant(&field.name, expr.span) {
                    self.emit_op_u16(OpCode::NullSafeGetField, idx, line);
                }
            }

            ExprKind::NullSafeIndex {
                expr: target,
                index,
            } => {
                self.expression(target);
                self.expression(index);
                self.emit_op(OpCode::NullSafeGetIndex, line);
            }

            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.if_expression(cond, then_branch, else_branch.as_ref(), line);
            }

            ExprKind::Match { expr: target, arms } => {
                self.match_expression(target, arms, line, expr.span);
            }

            ExprKind::Lambda {
                params,
                return_type: _,
                body,
            } => {
                self.lambda(params, body, line, expr.span);
            }

            ExprKind::Block(block) => {
                self.block_expression(block, line);
            }

            ExprKind::List(elements) => {
                for elem in elements {
                    self.expression(elem);
                }
                self.emit_op_u16(OpCode::NewList, elements.len() as u16, line);
            }

            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.expression(key);
                    self.expression(value);
                }
                self.emit_op_u16(OpCode::NewMap, entries.len() as u16, line);
            }

            ExprKind::StringInterp { parts } => {
                self.string_interpolation(parts, line);
            }

            ExprKind::Await(inner) => {
                self.expression(inner);
                self.emit_op(OpCode::Await, line);
            }

            ExprKind::Try(inner) => {
                // Try expression wraps result in Result type
                // For now, just evaluate the expression
                self.expression(inner);
            }

            ExprKind::StructInit { name, fields } => {
                self.struct_init(name, fields, line, expr.span);
            }

            ExprKind::EnumVariant {
                enum_name,
                variant,
                data,
            } => {
                self.enum_variant(enum_name.as_ref(), variant, data.as_deref(), line, expr.span);
            }

            ExprKind::Placeholder => {
                // Placeholder (_) is only valid inside pipeline expressions.
                // If we reach here, it means _ was used outside of a |> context.
                self.error(
                    CompileErrorKind::InvalidPlaceholder,
                    expr.span,
                );
            }

            ExprKind::ColumnShorthand(ident) => {
                // Column shorthand (.column_name) should have been transformed into a lambda
                // by call_expr when it appears as a function argument. If we reach here,
                // it means .column was used in an unsupported context.
                self.error(
                    CompileErrorKind::InvalidColumnShorthand(ident.name.clone()),
                    expr.span,
                );
            }

            ExprKind::StateBinding(inner) => {
                // State binding (&state.field) creates a reference for reactive GUI updates.
                // The inner expression should be a field access path like "state.field".
                // For now, we compile it as a special StateBinding value that wraps the path.
                self.compile_state_binding(inner, line, expr.span);
            }
        }
    }

    /// Compile a state binding expression (&state.field)
    fn compile_state_binding(&mut self, expr: &Expr, line: u32, span: Span) {
        // Extract the field path from the expression
        let path = self.extract_field_path(expr);

        // Emit the state binding opcode with the path as a string constant
        if let Some(idx) = self.current.chunk_mut().add_constant(Value::string(path)) {
            self.emit_op_u16(OpCode::StateBinding, idx, line);
        } else {
            self.error(CompileErrorKind::TooManyConstants, span);
        }
    }

    /// Extract a dotted field path from an expression (e.g., "state.user.name")
    fn extract_field_path(&self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Ident(ident) => ident.name.clone(),
            ExprKind::Field { expr: base, field } => {
                let base_path = self.extract_field_path(base);
                format!("{}.{}", base_path, field.name)
            }
            _ => {
                // For other expressions, fall back to a placeholder
                "<expr>".to_string()
            }
        }
    }

    fn literal(&mut self, lit: &Literal, line: u32, span: Span) {
        match lit {
            Literal::Int(n) => {
                if let Some(idx) = self.current.chunk_mut().add_constant(Value::Int(*n)) {
                    self.emit_op_u16(OpCode::Const, idx, line);
                } else {
                    self.error(CompileErrorKind::TooManyConstants, span);
                }
            }
            Literal::Float(n) => {
                if let Some(idx) = self.current.chunk_mut().add_constant(Value::Float(*n)) {
                    self.emit_op_u16(OpCode::Const, idx, line);
                } else {
                    self.error(CompileErrorKind::TooManyConstants, span);
                }
            }
            Literal::String(s) => {
                if let Some(idx) = self.current.chunk_mut().add_constant(Value::string(s.clone())) {
                    self.emit_op_u16(OpCode::Const, idx, line);
                } else {
                    self.error(CompileErrorKind::TooManyConstants, span);
                }
            }
            Literal::Bool(true) => self.emit_op(OpCode::True, line),
            Literal::Bool(false) => self.emit_op(OpCode::False, line),
            Literal::Null => self.emit_op(OpCode::Null, line),
        }
    }

    fn binary(&mut self, left: &Expr, op: BinOp, right: &Expr, line: u32, span: Span) {
        match op {
            // Short-circuit operators
            BinOp::And => {
                self.expression(left);
                let end_jump = self.emit_jump(OpCode::JumpIfFalse, line);
                self.emit_op(OpCode::Pop, line);
                self.expression(right);
                self.patch_jump(end_jump);
            }

            BinOp::Or => {
                self.expression(left);
                let end_jump = self.emit_jump(OpCode::JumpIfTrue, line);
                self.emit_op(OpCode::Pop, line);
                self.expression(right);
                self.patch_jump(end_jump);
            }

            BinOp::NullCoalesce => {
                self.expression(left);
                let end_jump = self.emit_jump(OpCode::JumpIfNotNull, line);
                self.emit_op(OpCode::Pop, line);
                self.expression(right);
                self.patch_jump(end_jump);
            }

            BinOp::Pipe => {
                // Pipeline operator: a |> f or a |> f(b, c) or a |> f(_, b)
                // Desugaring:
                // - a |> f           -> f(a)
                // - a |> f(b)        -> f(a, b)    (no placeholder = prepend)
                // - a |> f(_, b)     -> f(a, b)    (placeholder marks position)
                // - a |> f(b, _)     -> f(b, a)
                // - a |> f(_, _, _)  -> f(a, a, a) (all placeholders get same value)

                match &right.kind {
                    ExprKind::Call { callee, args, .. } => {
                        // Check if any argument is a placeholder
                        let has_placeholder = args
                            .iter()
                            .any(|arg| matches!(arg.value().kind, ExprKind::Placeholder));

                        // Check if callee is a function that takes column names (select, group_by)
                        let is_column_name_func = matches!(&callee.kind, ExprKind::Ident(ident) if ident.name == "select" || ident.name == "group_by");

                        if has_placeholder {
                            // Compile function first
                            self.expression(callee);
                            // For each argument: compile LHS if placeholder, else the arg
                            for arg in args {
                                let arg_expr = arg.value();
                                if matches!(arg_expr.kind, ExprKind::Placeholder) {
                                    self.expression(left);
                                } else if is_column_name_func {
                                    // For select/group_by, emit column names as strings for simple shorthands
                                    self.compile_select_arg(arg_expr, line);
                                } else if Self::contains_column_shorthand(arg_expr) {
                                    self.compile_column_shorthand_lambda(arg_expr, line);
                                } else {
                                    self.expression(arg_expr);
                                }
                            }
                            self.emit_op_u8(OpCode::Call, args.len() as u8, line);
                        } else {
                            // No placeholder - prepend left as first argument
                            self.expression(callee);
                            self.expression(left); // Piped value becomes first arg
                            for arg in args {
                                let arg_expr = arg.value();
                                if is_column_name_func {
                                    // For select/group_by, emit column names as strings for simple shorthands
                                    self.compile_select_arg(arg_expr, line);
                                } else if Self::contains_column_shorthand(arg_expr) {
                                    self.compile_column_shorthand_lambda(arg_expr, line);
                                } else {
                                    self.expression(arg_expr);
                                }
                            }
                            self.emit_op_u8(OpCode::Call, (args.len() + 1) as u8, line);
                        }
                    }
                    _ => {
                        // Bare function reference: a |> f -> f(a)
                        self.expression(right); // Function
                        self.expression(left); // Argument
                        self.emit_op_u8(OpCode::Call, 1, line);
                    }
                }
            }

            BinOp::Range => {
                self.expression(left);
                self.expression(right);
                self.emit_op(OpCode::NewRange, line);
            }

            BinOp::RangeInclusive => {
                self.expression(left);
                self.expression(right);
                self.emit_op(OpCode::NewRangeInclusive, line);
            }

            // Regular binary operators
            _ => {
                self.expression(left);
                self.expression(right);
                match op {
                    BinOp::Add => self.emit_op(OpCode::Add, line),
                    BinOp::Sub => self.emit_op(OpCode::Sub, line),
                    BinOp::Mul => self.emit_op(OpCode::Mul, line),
                    BinOp::Div => self.emit_op(OpCode::Div, line),
                    BinOp::Mod => self.emit_op(OpCode::Mod, line),
                    BinOp::Eq => self.emit_op(OpCode::Eq, line),
                    BinOp::Ne => self.emit_op(OpCode::Ne, line),
                    BinOp::Lt => self.emit_op(OpCode::Lt, line),
                    BinOp::Le => self.emit_op(OpCode::Le, line),
                    BinOp::Gt => self.emit_op(OpCode::Gt, line),
                    BinOp::Ge => self.emit_op(OpCode::Ge, line),
                    _ => {
                        self.error(CompileErrorKind::Internal(format!("unhandled binary op: {op:?}")), span);
                    }
                }
            }
        }
    }

    fn call(
        &mut self,
        callee: &Expr,
        args: &[CallArg],
        trailing_closure: Option<&Expr>,
        line: u32,
        span: Span,
    ) {
        // Calculate total argument count (args + optional trailing closure)
        let total_args = args.len() + if trailing_closure.is_some() { 1 } else { 0 };

        if total_args > 255 {
            self.error(CompileErrorKind::TooManyArguments, span);
            return;
        }

        // Check for method call optimization
        if let ExprKind::Field { expr, field } = &callee.kind {
            // This is a method call: obj.method(args)
            // Check if it's a select/group_by method for special handling
            let is_column_name_method = field.name == "select" || field.name == "group_by";

            self.expression(expr);
            for arg in args {
                let arg_expr = arg.value();
                if is_column_name_method {
                    // For select/group_by, emit column names as strings for simple shorthands
                    self.compile_select_arg(arg_expr, line);
                } else if Self::contains_column_shorthand(arg_expr) {
                    // Transform column shorthand arguments into lambdas
                    self.compile_column_shorthand_lambda(arg_expr, line);
                } else {
                    self.expression(arg_expr);
                }
            }
            // Compile trailing closure if present
            if let Some(closure) = trailing_closure {
                self.expression(closure);
            }
            if let Some(idx) = self.identifier_constant(&field.name, span) {
                self.emit_op(OpCode::Invoke, line);
                self.emit_byte((idx & 0xFF) as u8, line);
                self.emit_byte((idx >> 8) as u8, line);
                self.emit_byte(total_args as u8, line);
            }
            return;
        }

        // Check if callee is a function that takes column names (select, group_by)
        let is_column_name_func = matches!(&callee.kind, ExprKind::Ident(ident) if ident.name == "select" || ident.name == "group_by");

        // Regular function call
        self.expression(callee);
        for arg in args {
            let arg_expr = arg.value();
            if is_column_name_func {
                // For select/group_by, emit column names as strings for simple shorthands
                self.compile_select_arg(arg_expr, line);
            } else if Self::contains_column_shorthand(arg_expr) {
                // Transform column shorthand arguments into lambdas
                self.compile_column_shorthand_lambda(arg_expr, line);
            } else {
                self.expression(arg_expr);
            }
        }
        // Compile trailing closure if present
        if let Some(closure) = trailing_closure {
            self.expression(closure);
        }
        self.emit_op_u8(OpCode::Call, total_args as u8, line);
    }

    fn if_expression(
        &mut self,
        cond: &Expr,
        then_branch: &Block,
        else_branch: Option<&ElseBranch>,
        line: u32,
    ) {
        // Condition
        self.expression(cond);
        let else_jump = self.emit_jump(OpCode::JumpIfFalse, line);
        // Note: JumpIfFalse already pops the condition

        // Then branch
        self.block_expression(then_branch, line);

        let end_jump = self.emit_jump(OpCode::Jump, line);

        // Else branch
        self.patch_jump(else_jump);
        // Note: JumpIfFalse already popped the condition when jumping here

        match else_branch {
            Some(ElseBranch::Block(block)) => {
                self.block_expression(block, line);
            }
            Some(ElseBranch::ElseIf(else_if)) => {
                self.expression(else_if);
            }
            None => {
                self.emit_op(OpCode::Null, line);
            }
        }

        self.patch_jump(end_jump);
    }

    fn match_expression(&mut self, target: &Expr, arms: &[MatchArm], line: u32, span: Span) {
        // Evaluate the match target
        self.expression(target);

        let mut end_jumps = Vec::new();

        for arm in arms {
            // Duplicate the value for comparison
            self.emit_op(OpCode::Dup, line);

            // Compile pattern matching
            // For now, only support simple patterns
            match &arm.pattern.kind {
                PatternKind::Wildcard => {
                    // Always matches - just pop the duplicate
                    self.emit_op(OpCode::Pop, line);
                }
                PatternKind::Literal(lit) => {
                    // Compare with literal
                    self.literal(lit, line, arm.pattern.span);
                    self.emit_op(OpCode::Eq, line);
                    let next_arm = self.emit_jump(OpCode::JumpIfFalse, line);
                    // Note: JumpIfFalse already popped the comparison result
                    self.emit_op(OpCode::Pop, line); // Pop target duplicate

                    // Compile arm body
                    self.expression(&arm.body);
                    end_jumps.push(self.emit_jump(OpCode::Jump, line));

                    self.patch_jump(next_arm);
                    // Note: JumpIfFalse already popped the comparison result when jumping here
                    continue;
                }
                PatternKind::Ident(name) => {
                    // Binding - bind the value
                    self.begin_scope();
                    self.declare_variable(name);
                    self.mark_initialized();

                    // Guard condition if present
                    if let Some(guard) = &arm.guard {
                        self.expression(guard);
                        let next_arm = self.emit_jump(OpCode::JumpIfFalse, line);
                        // Note: JumpIfFalse already popped the guard result

                        // Compile arm body
                        self.expression(&arm.body);
                        self.end_scope(line);
                        self.emit_op(OpCode::Pop, line); // Pop original target
                        end_jumps.push(self.emit_jump(OpCode::Jump, line));

                        self.patch_jump(next_arm);
                        // Note: JumpIfFalse already popped the guard result when jumping here
                        self.end_scope(line);
                        continue;
                    }

                    // No guard - just execute
                    self.expression(&arm.body);
                    self.end_scope(line);
                    self.emit_op(OpCode::Pop, line); // Pop original target
                    end_jumps.push(self.emit_jump(OpCode::Jump, line));
                    continue;
                }
                _ => {
                    self.error(
                        CompileErrorKind::Unsupported("complex match patterns".to_string()),
                        arm.pattern.span,
                    );
                    continue;
                }
            }

            // Compile arm body
            self.expression(&arm.body);
            self.emit_op(OpCode::Pop, line); // Pop target (for wildcard)
            end_jumps.push(self.emit_jump(OpCode::Jump, line));
        }

        // If no arm matched, push null
        self.emit_op(OpCode::Pop, line); // Pop target
        self.emit_op(OpCode::Null, line);

        // Patch all end jumps
        for jump in end_jumps {
            self.patch_jump(jump);
        }

        // Note: proper match exhaustiveness should be checked by type checker
        let _ = span; // Suppress unused warning
    }

    fn lambda(&mut self, params: &[Param], body: &Expr, line: u32, span: Span) {
        // Create synthetic function
        let name = format!("<lambda@{}>", line);

        // Start a new compiler state (lambdas are synchronous)
        let enclosing = std::mem::replace(
            &mut self.current,
            CompilerState::new(FunctionType::Function, name, false),
        );
        self.current.enclosing = Some(Box::new(enclosing));
        self.begin_scope();

        // Compile parameters
        for param in params {
            if self.current.function.arity == 255 {
                self.error(CompileErrorKind::TooManyParameters, param.span);
                break;
            }
            self.current.function.arity += 1;
            self.declare_variable(&param.name);
            self.mark_initialized();
        }

        // Compile body expression
        self.expression(body);
        self.emit_op(OpCode::Return, line);

        // End function scope
        self.end_scope(line);

        // Get the completed function - need to take enclosing first to avoid borrow issue
        let enclosing = self.current.enclosing.take().unwrap();
        let function = std::mem::replace(&mut self.current, *enclosing);

        // Emit closure instruction
        let upvalue_count = function.upvalues.len();
        let mut completed_function = function.function;
        completed_function.upvalue_count = upvalue_count as u16;

        let func_value = Value::Function(Rc::new(completed_function));
        if let Some(const_idx) = self.current.chunk_mut().add_constant(func_value) {
            self.emit_op_u16(OpCode::Closure, const_idx, line);

            // Emit upvalue descriptors
            for upvalue in &function.upvalues {
                self.emit_byte(if upvalue.is_local { 1 } else { 0 }, line);
                self.emit_byte(upvalue.index, line);
            }
        } else {
            self.error(CompileErrorKind::TooManyConstants, span);
        }
    }

    fn block(&mut self, block: &Block) {
        self.begin_scope();
        for stmt in &block.stmts {
            self.statement(stmt);
        }
        // Also compile the trailing expression if present (for side effects, discard result)
        if let Some(expr) = &block.expr {
            let line = self.line_from_span(expr.span);
            self.expression(expr);
            self.emit_op(OpCode::Pop, line); // Discard the result
        }
        let line = self.line_from_span(block.span);
        self.end_scope(line);
    }

    fn block_expression(&mut self, block: &Block, line: u32) {
        self.begin_scope();
        for stmt in &block.stmts {
            self.statement(stmt);
        }

        // Count locals at current scope depth (these need cleanup after the expression)
        let locals_to_pop = self
            .current
            .locals
            .iter()
            .filter(|l| l.depth == self.current.scope_depth)
            .count();

        // Final expression is the block's value
        if let Some(expr) = &block.expr {
            self.expression(expr);
        } else {
            self.emit_op(OpCode::Null, line);
        }

        // Pop locals while preserving the result on top of stack
        if locals_to_pop > 0 {
            // TODO: Handle captured locals (CloseUpvalue) - for now just pop
            self.emit_op_u8(OpCode::PopBelow, locals_to_pop as u8, line);
        }

        // Clean up compiler state (decrement scope, remove locals from tracking)
        self.current.scope_depth -= 1;
        self.current
            .locals
            .retain(|l| l.depth <= self.current.scope_depth);
    }

    fn string_interpolation(&mut self, parts: &[StringPart], line: u32) {
        if parts.is_empty() {
            // Empty string
            if let Some(idx) = self.current.chunk_mut().add_constant(Value::string("")) {
                self.emit_op_u16(OpCode::Const, idx, line);
            }
            return;
        }

        let mut count = 0u16;
        for part in parts {
            match part {
                StringPart::Literal(s) => {
                    if let Some(idx) = self.current.chunk_mut().add_constant(Value::string(s.clone())) {
                        self.emit_op_u16(OpCode::Const, idx, line);
                        count += 1;
                    }
                }
                StringPart::Expr(expr) => {
                    self.expression(expr);
                    count += 1;
                }
            }
        }

        self.emit_op_u16(OpCode::StringConcat, count, line);
    }

    fn struct_init(&mut self, name: &Ident, fields: &[FieldInit], line: u32, span: Span) {
        // Push (field_name, field_value) pairs for each field
        // Stack will be: name1, value1, name2, value2, ...
        for field in fields {
            // Push field name as constant
            if let Some(name_idx) = self.identifier_constant(&field.name.name, field.span) {
                self.emit_op_u16(OpCode::Const, name_idx, line);
            }

            // Push field value
            if let Some(value) = &field.value {
                self.expression(value);
            } else {
                // Shorthand: { x } means { x: x }
                self.get_variable(&field.name.name, line, field.span);
            }
        }

        // Create struct with type name and field count
        if let Some(idx) = self.identifier_constant(&name.name, span) {
            let field_count = fields.len() as u16;
            self.emit_op_u16_u16(OpCode::NewStruct, idx, field_count, line);
        }
    }

    fn enum_variant(
        &mut self,
        _enum_name: Option<&Ident>,
        variant: &Ident,
        data: Option<&Expr>,
        line: u32,
        span: Span,
    ) {
        // Push data if present
        if let Some(d) = data {
            self.expression(d);
        } else {
            self.emit_op(OpCode::Null, line);
        }

        // Create variant
        if let Some(idx) = self.identifier_constant(&variant.name, span) {
            self.emit_op_u16(OpCode::NewEnumVariant, idx, line);
        }
    }

    // ===== Variable Management =====

    fn declare_variable(&mut self, name: &Ident) {
        if self.current.scope_depth == 0 {
            // Global - nothing to declare
            return;
        }

        // Check for duplicate in same scope
        for local in self.current.locals.iter().rev() {
            if local.depth < self.current.scope_depth {
                break;
            }
            if local.name == name.name {
                self.error(
                    CompileErrorKind::DuplicateVariable(name.name.clone()),
                    name.span,
                );
                return;
            }
        }

        // Add local
        self.current.locals.push(Local {
            name: name.name.clone(),
            depth: self.current.scope_depth,
            initialized: false,
            is_captured: false,
        });
    }

    fn define_variable(&mut self, name: &Ident, line: u32) {
        if self.current.scope_depth > 0 {
            // Local - already on stack, just mark initialized
            self.mark_initialized();
            return;
        }

        // Global
        if let Some(idx) = self.identifier_constant(&name.name, name.span) {
            self.emit_op_u16(OpCode::DefineGlobal, idx, line);
        }
    }

    fn mark_initialized(&mut self) {
        if self.current.scope_depth == 0 {
            return;
        }
        if let Some(local) = self.current.locals.last_mut() {
            local.initialized = true;
        }
    }

    fn get_variable(&mut self, name: &str, line: u32, span: Span) {
        // Try local first
        if let Some(slot) = self.resolve_local(name) {
            self.emit_op_u16(OpCode::LoadLocal, slot, line);
            return;
        }

        // Try upvalue
        if let Some(upvalue) = self.resolve_upvalue(name) {
            self.emit_op_u8(OpCode::LoadUpvalue, upvalue, line);
            return;
        }

        // Must be global
        if let Some(idx) = self.identifier_constant(name, span) {
            self.emit_op_u16(OpCode::LoadGlobal, idx, line);
        }
    }

    fn set_variable(&mut self, name: &str, line: u32, span: Span) {
        // Try local first
        if let Some(slot) = self.resolve_local(name) {
            self.emit_op_u16(OpCode::StoreLocal, slot, line);
            return;
        }

        // Try upvalue
        if let Some(upvalue) = self.resolve_upvalue(name) {
            self.emit_op_u8(OpCode::StoreUpvalue, upvalue, line);
            return;
        }

        // Must be global
        if let Some(idx) = self.identifier_constant(name, span) {
            self.emit_op_u16(OpCode::StoreGlobal, idx, line);
        }
    }

    fn resolve_local(&self, name: &str) -> Option<u16> {
        for (i, local) in self.current.locals.iter().enumerate().rev() {
            if local.name == name {
                if !local.initialized {
                    // Can't read variable in its own initializer
                    // (This error will be handled elsewhere)
                }
                return Some(i as u16);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        let enclosing = self.current.enclosing.as_mut()?;

        // Try to find in enclosing function's locals
        for (i, local) in enclosing.locals.iter().enumerate().rev() {
            if local.name == name {
                // Mark as captured
                let idx = i;
                // We need to mark the local as captured, but we have a mutable borrow
                // We'll handle this by returning early and doing the modification
                return Some(self.add_upvalue(idx as u8, true));
            }
        }

        // Try enclosing function's upvalues (recursive)
        // This is tricky with our current borrow situation
        // For now, we'll simplify and not support deep closure capturing

        None
    }

    fn add_upvalue(&mut self, index: u8, is_local: bool) -> u8 {
        // Check if we already have this upvalue
        for (i, upvalue) in self.current.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i as u8;
            }
        }

        if self.current.upvalues.len() >= 256 {
            // Error will be handled when we return
            return 0;
        }

        self.current.upvalues.push(Upvalue { index, is_local });
        (self.current.upvalues.len() - 1) as u8
    }

    // ===== Scope Management =====

    fn begin_scope(&mut self) {
        self.current.scope_depth += 1;
    }

    fn end_scope(&mut self, line: u32) {
        self.current.scope_depth -= 1;

        // Pop locals from the ended scope
        while !self.current.locals.is_empty()
            && self.current.locals.last().unwrap().depth > self.current.scope_depth
        {
            let local = self.current.locals.pop().unwrap();
            if local.is_captured {
                self.emit_op(OpCode::CloseUpvalue, line);
            } else {
                self.emit_op(OpCode::Pop, line);
            }
        }
    }

    fn close_upvalues_to_depth(&mut self, depth: u32, line: u32) {
        // Collect what we need to emit first to avoid borrow issues
        let ops: Vec<OpCode> = self
            .current
            .locals
            .iter()
            .rev()
            .take_while(|local| local.depth > depth)
            .map(|local| {
                if local.is_captured {
                    OpCode::CloseUpvalue
                } else {
                    OpCode::Pop
                }
            })
            .collect();

        for op in ops {
            self.emit_op(op, line);
        }
    }

    // ===== Bytecode Emission Helpers =====

    fn emit_op(&mut self, op: OpCode, line: u32) {
        self.current.chunk_mut().write_op(op, line);
    }

    fn emit_op_u8(&mut self, op: OpCode, operand: u8, line: u32) {
        self.current.chunk_mut().write_op_u8(op, operand, line);
    }

    fn emit_op_u16(&mut self, op: OpCode, operand: u16, line: u32) {
        self.current.chunk_mut().write_op_u16(op, operand, line);
    }

    fn emit_op_u16_u16(&mut self, op: OpCode, operand1: u16, operand2: u16, line: u32) {
        self.current.chunk_mut().write_op_u16(op, operand1, line);
        self.current.chunk_mut().write_u16(operand2, line);
    }

    fn emit_byte(&mut self, byte: u8, line: u32) {
        self.current.chunk_mut().write_byte(byte, line);
    }

    fn emit_jump(&mut self, op: OpCode, line: u32) -> usize {
        self.current.chunk_mut().emit_jump(op, line)
    }

    fn patch_jump(&mut self, offset: usize) {
        self.current.chunk_mut().patch_jump(offset);
    }

    fn emit_loop(&mut self, loop_start: usize, line: u32) {
        self.current.chunk_mut().emit_loop(loop_start, line);
    }

    fn emit_return(&mut self, span: Span) {
        let line = self.line_from_span(span);

        if self.current.function_type == FunctionType::Initializer {
            // Initializers return 'this'
            self.emit_op_u16(OpCode::LoadLocal, 0, line);
        } else {
            self.emit_op(OpCode::Null, line);
        }
        self.emit_op(OpCode::Return, line);
    }

    fn identifier_constant(&mut self, name: &str, span: Span) -> Option<u16> {
        match self.current.chunk_mut().add_constant(Value::string(name)) {
            Some(idx) => Some(idx),
            None => {
                self.error(CompileErrorKind::TooManyConstants, span);
                None
            }
        }
    }

    // ===== Error Handling =====

    fn error(&mut self, kind: CompileErrorKind, span: Span) {
        self.errors.push(CompileError::new(kind, span));
    }

    fn line_from_span(&self, span: Span) -> u32 {
        // For now, use start position as line number
        // A proper implementation would map spans to line numbers
        span.start
    }

    /// Check if an expression contains any ColumnShorthand nodes
    fn contains_column_shorthand(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::ColumnShorthand(_) => true,
            ExprKind::Binary { left, right, .. } => {
                Self::contains_column_shorthand(left) || Self::contains_column_shorthand(right)
            }
            ExprKind::Unary { expr: e, .. } => Self::contains_column_shorthand(e),
            ExprKind::Paren(e) => Self::contains_column_shorthand(e),
            ExprKind::Call { callee, args, trailing_closure } => {
                Self::contains_column_shorthand(callee)
                    || args.iter().any(|arg| Self::contains_column_shorthand(arg.value()))
                    || trailing_closure.as_ref().map_or(false, |tc| Self::contains_column_shorthand(tc))
            }
            ExprKind::Index { expr: e, index } => {
                Self::contains_column_shorthand(e) || Self::contains_column_shorthand(index)
            }
            ExprKind::Field { expr: e, .. } => Self::contains_column_shorthand(e),
            ExprKind::NullSafeField { expr: e, .. } => Self::contains_column_shorthand(e),
            ExprKind::NullSafeIndex { expr: e, index } => {
                Self::contains_column_shorthand(e) || Self::contains_column_shorthand(index)
            }
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                Self::contains_column_shorthand(cond)
                    || then_branch.stmts.iter().any(|s| {
                        if let crate::ast::Stmt {
                            kind: crate::ast::StmtKind::Expr(e),
                            ..
                        } = s
                        {
                            Self::contains_column_shorthand(e)
                        } else {
                            false
                        }
                    })
                    || then_branch
                        .expr
                        .as_ref()
                        .map_or(false, |e| Self::contains_column_shorthand(e))
                    || else_branch.as_ref().map_or(false, |eb| match eb {
                        ElseBranch::Block(b) => {
                            b.expr
                                .as_ref()
                                .map_or(false, |e| Self::contains_column_shorthand(e))
                        }
                        ElseBranch::ElseIf(e) => Self::contains_column_shorthand(e),
                    })
            }
            ExprKind::List(items) => items.iter().any(Self::contains_column_shorthand),
            ExprKind::Map(pairs) => pairs
                .iter()
                .any(|(k, v)| Self::contains_column_shorthand(k) || Self::contains_column_shorthand(v)),
            ExprKind::Lambda { body, .. } => Self::contains_column_shorthand(body),
            ExprKind::StringInterp { parts } => parts.iter().any(|p| match p {
                StringPart::Expr(e) => Self::contains_column_shorthand(e),
                StringPart::Literal(_) => false,
            }),
            ExprKind::Await(e) | ExprKind::Try(e) => Self::contains_column_shorthand(e),
            ExprKind::StateBinding(e) => Self::contains_column_shorthand(e),
            // Terminals that don't contain column shorthand
            ExprKind::Literal(_)
            | ExprKind::Ident(_)
            | ExprKind::Placeholder
            | ExprKind::StructInit { .. }
            | ExprKind::EnumVariant { .. }
            | ExprKind::Match { .. }
            | ExprKind::Block(_) => false,
        }
    }

    /// Compile an expression containing ColumnShorthand by wrapping it in a lambda.
    /// The lambda has a single parameter `$row` and all ColumnShorthand nodes
    /// become field accesses on `$row`.
    fn compile_column_shorthand_lambda(&mut self, expr: &Expr, line: u32) {
        // Create synthetic function like regular lambda compilation
        let name = format!("<column_shorthand_lambda@{}>", line);
        let row_param_name = "$row";

        // Start a new compiler state (lambdas are synchronous)
        let enclosing = std::mem::replace(
            &mut self.current,
            CompilerState::new(FunctionType::Function, name, false),
        );
        self.current.enclosing = Some(Box::new(enclosing));
        self.begin_scope();

        // Declare the $row parameter
        self.current.function.arity = 1;
        // Create a synthetic Ident for the $row parameter
        let row_ident = crate::ast::Ident::new(row_param_name, expr.span);
        self.declare_variable(&row_ident);
        self.mark_initialized();

        // Compile the body expression, transforming column shorthands
        self.compile_with_column_shorthand_transform(expr, row_param_name, line);
        self.emit_op(OpCode::Return, line);

        // End function scope
        self.end_scope(line);

        // Get the completed function
        let enclosing = self.current.enclosing.take().unwrap();
        let function = std::mem::replace(&mut self.current, *enclosing);

        // Emit closure instruction
        let upvalue_count = function.upvalues.len();
        let mut completed_function = function.function;
        completed_function.upvalue_count = upvalue_count as u16;

        let func_value = Value::Function(Rc::new(completed_function));
        if let Some(const_idx) = self.current.chunk_mut().add_constant(func_value) {
            self.emit_op_u16(OpCode::Closure, const_idx, line);

            // Emit upvalue descriptors
            for upvalue in &function.upvalues {
                self.emit_byte(if upvalue.is_local { 1 } else { 0 }, line);
                self.emit_byte(upvalue.index, line);
            }
        } else {
            self.error(CompileErrorKind::TooManyConstants, expr.span);
        }
    }

    /// Compile an argument for the select function.
    /// For simple ColumnShorthand, emit the column name as a string.
    /// For other expressions, compile them normally.
    fn compile_select_arg(&mut self, arg: &Expr, line: u32) {
        match &arg.kind {
            ExprKind::ColumnShorthand(ident) => {
                // For select, emit the column name as a string constant
                let name = &ident.name;
                let value = Value::string(name.clone());
                if let Some(const_idx) = self.current.chunk_mut().add_constant(value) {
                    self.emit_op_u16(OpCode::Const, const_idx, line);
                } else {
                    self.error(CompileErrorKind::TooManyConstants, arg.span);
                }
            }
            // For non-column-shorthand expressions, compile normally
            _ => {
                self.expression(arg);
            }
        }
    }

    /// Compile an expression, transforming ColumnShorthand into field access on the given local
    fn compile_with_column_shorthand_transform(&mut self, expr: &Expr, row_var: &str, line: u32) {
        match &expr.kind {
            ExprKind::ColumnShorthand(ident) => {
                // Transform .column_name into $row.column_name
                self.get_variable(row_var, line, expr.span);
                if let Some(idx) = self.identifier_constant(&ident.name, expr.span) {
                    self.emit_op_u16(OpCode::GetField, idx, line);
                }
            }
            ExprKind::Binary { left, op, right } => {
                // Handle short-circuit operators specially
                match op {
                    BinOp::And => {
                        self.compile_with_column_shorthand_transform(left, row_var, line);
                        let end_jump = self.emit_jump(OpCode::JumpIfFalse, line);
                        self.emit_op(OpCode::Pop, line);
                        self.compile_with_column_shorthand_transform(right, row_var, line);
                        self.patch_jump(end_jump);
                    }
                    BinOp::Or => {
                        self.compile_with_column_shorthand_transform(left, row_var, line);
                        let end_jump = self.emit_jump(OpCode::JumpIfTrue, line);
                        self.emit_op(OpCode::Pop, line);
                        self.compile_with_column_shorthand_transform(right, row_var, line);
                        self.patch_jump(end_jump);
                    }
                    BinOp::NullCoalesce => {
                        self.compile_with_column_shorthand_transform(left, row_var, line);
                        let end_jump = self.emit_jump(OpCode::JumpIfNotNull, line);
                        self.emit_op(OpCode::Pop, line);
                        self.compile_with_column_shorthand_transform(right, row_var, line);
                        self.patch_jump(end_jump);
                    }
                    _ => {
                        self.compile_with_column_shorthand_transform(left, row_var, line);
                        self.compile_with_column_shorthand_transform(right, row_var, line);
                        match op {
                            BinOp::Add => self.emit_op(OpCode::Add, line),
                            BinOp::Sub => self.emit_op(OpCode::Sub, line),
                            BinOp::Mul => self.emit_op(OpCode::Mul, line),
                            BinOp::Div => self.emit_op(OpCode::Div, line),
                            BinOp::Mod => self.emit_op(OpCode::Mod, line),
                            BinOp::Eq => self.emit_op(OpCode::Eq, line),
                            BinOp::Ne => self.emit_op(OpCode::Ne, line),
                            BinOp::Lt => self.emit_op(OpCode::Lt, line),
                            BinOp::Le => self.emit_op(OpCode::Le, line),
                            BinOp::Gt => self.emit_op(OpCode::Gt, line),
                            BinOp::Ge => self.emit_op(OpCode::Ge, line),
                            BinOp::Range => self.emit_op(OpCode::NewRange, line),
                            BinOp::RangeInclusive => self.emit_op(OpCode::NewRangeInclusive, line),
                            BinOp::Pipe | BinOp::And | BinOp::Or | BinOp::NullCoalesce => {
                                // Already handled above
                            }
                        }
                    }
                }
            }
            ExprKind::Unary { op, expr: e } => {
                self.compile_with_column_shorthand_transform(e, row_var, line);
                match op {
                    UnaryOp::Neg => self.emit_op(OpCode::Neg, line),
                    UnaryOp::Not => self.emit_op(OpCode::Not, line),
                }
            }
            ExprKind::Paren(e) => {
                self.compile_with_column_shorthand_transform(e, row_var, line);
            }
            ExprKind::Field { expr: e, field } => {
                self.compile_with_column_shorthand_transform(e, row_var, line);
                if let Some(idx) = self.identifier_constant(&field.name, expr.span) {
                    self.emit_op_u16(OpCode::GetField, idx, line);
                }
            }
            ExprKind::Call { callee, args, trailing_closure } => {
                // For nested calls, recursively transform
                let total_args = args.len() + if trailing_closure.is_some() { 1 } else { 0 };
                if let ExprKind::Field {
                    expr: obj,
                    field,
                } = &callee.kind
                {
                    // Method call
                    self.compile_with_column_shorthand_transform(obj, row_var, line);
                    for arg in args {
                        self.compile_with_column_shorthand_transform(arg.value(), row_var, line);
                    }
                    if let Some(tc) = trailing_closure {
                        self.compile_with_column_shorthand_transform(tc, row_var, line);
                    }
                    if let Some(idx) = self.identifier_constant(&field.name, expr.span) {
                        self.emit_op(OpCode::Invoke, line);
                        self.emit_byte((idx & 0xFF) as u8, line);
                        self.emit_byte((idx >> 8) as u8, line);
                        self.emit_byte(total_args as u8, line);
                    }
                } else {
                    // Regular call
                    self.compile_with_column_shorthand_transform(callee, row_var, line);
                    for arg in args {
                        self.compile_with_column_shorthand_transform(arg.value(), row_var, line);
                    }
                    if let Some(tc) = trailing_closure {
                        self.compile_with_column_shorthand_transform(tc, row_var, line);
                    }
                    self.emit_op_u8(OpCode::Call, total_args as u8, line);
                }
            }
            // For expressions without column shorthand, just compile normally
            _ => self.expression(expr),
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn compile_module(source: &str) -> Result<Rc<BytecodeFunction>, Vec<CompileError>> {
        let module = Parser::parse_module(source).expect("Parse error");

        let compiler = Compiler::new();
        compiler.compile_module(&module)
    }

    fn compile_expr(source: &str) -> Result<Rc<BytecodeFunction>, Vec<CompileError>> {
        let expr = Parser::parse_expression(source).expect("Parse error");

        let compiler = Compiler::new();
        compiler.compile_expression(&expr)
    }

    #[test]
    fn compile_literal() {
        let result = compile_expr("42");
        assert!(result.is_ok());
        let func = result.unwrap();
        assert!(!func.chunk.is_empty());
    }

    #[test]
    fn compile_arithmetic() {
        let result = compile_expr("1 + 2 * 3");
        assert!(result.is_ok());
    }

    #[test]
    fn compile_function() {
        let result = compile_module("fx add(a, b) { a + b }");
        assert!(result.is_ok());
    }

    #[test]
    fn compile_if_expression() {
        let result = compile_expr("if true { 1 } else { 2 }");
        assert!(result.is_ok());
    }

    #[test]
    fn compile_function_with_let() {
        let result = compile_module("fx test() { let x = 42\n x }");
        assert!(result.is_ok());
    }

    #[test]
    fn compile_function_with_while() {
        let result = compile_module("fx test() { while false { } }");
        assert!(result.is_ok());
    }

    // ===== Execution Mode Propagation Tests =====

    /// Helper to get a function's execution mode from compiled module
    fn get_function_mode(script: &BytecodeFunction, name: &str) -> Option<ExecutionMode> {
        for constant in script.chunk.constants() {
            if let Value::Function(func) = constant {
                if func.name == name {
                    return Some(func.execution_mode);
                }
            }
        }
        None
    }

    #[test]
    fn compile_function_default_mode() {
        // Functions without directives should default to Interpret
        let result = compile_module("fx add(a, b) { a + b }");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(get_function_mode(&script, "add"), Some(ExecutionMode::Interpret));
    }

    #[test]
    fn compile_function_with_compile_directive() {
        // #[compile] should set mode to Compile
        let result = compile_module("#[compile]\nfx fast() { 42 }");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(get_function_mode(&script, "fast"), Some(ExecutionMode::Compile));
    }

    #[test]
    fn compile_function_with_interpret_directive() {
        // #[interpret] should set mode to Interpret
        let result = compile_module("#[interpret]\nfx slow() { 42 }");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(get_function_mode(&script, "slow"), Some(ExecutionMode::Interpret));
    }

    #[test]
    fn compile_function_with_compile_hot_directive() {
        // #[compile(hot)] should set mode to CompileHot
        let result = compile_module("#[compile(hot)]\nfx hot() { 42 }");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(get_function_mode(&script, "hot"), Some(ExecutionMode::CompileHot));
    }

    #[test]
    fn compile_module_level_directive() {
        // Module-level #![compile] should make functions default to Compile
        let result = compile_module("#![compile]\nfx one() { 1 }\nfx two() { 2 }");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(get_function_mode(&script, "one"), Some(ExecutionMode::Compile));
        assert_eq!(get_function_mode(&script, "two"), Some(ExecutionMode::Compile));
    }

    #[test]
    fn compile_function_overrides_module() {
        // Function directive should override module directive
        let result = compile_module("#![compile]\n#[interpret]\nfx slow() { 1 }");
        assert!(result.is_ok());
        let script = result.unwrap();
        // Function overrides module
        assert_eq!(get_function_mode(&script, "slow"), Some(ExecutionMode::Interpret));
    }

    #[test]
    fn compile_with_interpret_all_override() {
        // CLI --interpret-all should override all directives
        let module = Parser::parse_module("#[compile]\nfx fast() { 42 }").expect("Parse error");
        let compiler = Compiler::new().with_mode_override(Some(ExecutionModeOverride::InterpretAll));
        let result = compiler.compile_module(&module);
        assert!(result.is_ok());
        let script = result.unwrap();
        // Should be Interpret despite #[compile] directive
        assert_eq!(get_function_mode(&script, "fast"), Some(ExecutionMode::Interpret));
    }

    #[test]
    fn compile_with_compile_all_override() {
        // CLI --compile-all should override all directives
        let module = Parser::parse_module("#[interpret]\nfx slow() { 42 }").expect("Parse error");
        let compiler = Compiler::new().with_mode_override(Some(ExecutionModeOverride::CompileAll));
        let result = compiler.compile_module(&module);
        assert!(result.is_ok());
        let script = result.unwrap();
        // Should be Compile despite #[interpret] directive
        assert_eq!(get_function_mode(&script, "slow"), Some(ExecutionMode::Compile));
    }
}
