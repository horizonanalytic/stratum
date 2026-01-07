//! Virtual Machine for the Stratum programming language
//!
//! This module provides a stack-based bytecode interpreter that executes
//! compiled Stratum code.

mod debug;
mod error;
mod executor;
mod natives;
mod output;

pub use debug::{
    Breakpoint, DebugAction, DebugContext, DebugLocation, DebugStackFrame, DebugState,
    DebugStepResult, DebugVariable, PauseReason,
};
pub use error::{RuntimeError, RuntimeErrorKind, RuntimeResult, StackFrame};
pub use executor::{AsyncExecutor, CoroutineResult};
pub use output::{with_output_capture, OutputCapture};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::ExecutionMode;
use crate::gc::CycleCollector;
use crate::bytecode::{
    Chunk, Closure, CoroutineState, EnumVariantInstance, Function, FutureStatus,
    HashableValue, NativeFunction, OpCode, Range, SavedCallFrame, SavedExceptionHandler,
    StructInstance, Upvalue, Value,
};
use crate::data::{AggSpec, DataFrame, GroupedDataFrame, Series};
use crate::jit::{call_jit_function, CompiledFunction, JitCompiler, JitContext};

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

/// Default threshold for hot path detection (number of calls before JIT compilation)
const DEFAULT_HOT_THRESHOLD: usize = 1000;

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

    /// Suspended coroutine (set when awaiting a pending future)
    suspended_coroutine: Option<Value>,

    /// JIT compiler (lazily initialized when first needed)
    jit_compiler: Option<JitCompiler>,

    /// JIT context for caching compiled functions
    jit_context: JitContext,

    /// Whether JIT compilation is enabled
    jit_enabled: bool,

    /// Call counts per function for hot path detection (keyed by function pointer)
    call_counts: HashMap<*const Function, usize>,

    /// Threshold for triggering JIT compilation of hot functions
    hot_threshold: usize,

    /// Debug context for breakpoints and stepping
    debug_context: DebugContext,

    /// Current source file being executed (for debug location tracking)
    current_source: Option<std::path::PathBuf>,

    /// Cycle collector for detecting and breaking reference cycles
    gc: CycleCollector,
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
            suspended_coroutine: None,
            jit_compiler: None,
            jit_context: JitContext::new(),
            jit_enabled: true, // JIT enabled by default
            call_counts: HashMap::new(),
            hot_threshold: DEFAULT_HOT_THRESHOLD,
            debug_context: DebugContext::new(),
            current_source: None,
            gc: CycleCollector::new(),
        };

        // Register built-in functions
        vm.register_natives();

        vm
    }

    /// Create a new VM instance with JIT disabled
    #[must_use]
    pub fn new_without_jit() -> Self {
        let mut vm = Self::new();
        vm.jit_enabled = false;
        vm
    }

    /// Enable or disable JIT compilation
    pub fn set_jit_enabled(&mut self, enabled: bool) {
        self.jit_enabled = enabled;
    }

    /// Check if JIT compilation is enabled
    #[must_use]
    pub fn is_jit_enabled(&self) -> bool {
        self.jit_enabled
    }

    /// Set the hot path detection threshold
    ///
    /// Functions marked with `#[compile(hot)]` will be JIT-compiled after
    /// being called this many times.
    pub fn set_hot_threshold(&mut self, threshold: usize) {
        self.hot_threshold = threshold;
    }

    /// Get the current hot path detection threshold
    #[must_use]
    pub fn get_hot_threshold(&self) -> usize {
        self.hot_threshold
    }

    /// Get or create the JIT compiler (lazy initialization)
    fn get_jit_compiler(&mut self) -> &mut JitCompiler {
        if self.jit_compiler.is_none() {
            self.jit_compiler = Some(JitCompiler::new());
        }
        self.jit_compiler.as_mut().unwrap()
    }

    /// Compile a function with JIT and cache it
    fn jit_compile_function(&mut self, function: &Function) -> Result<CompiledFunction, String> {
        let name = function.name.clone();
        let arity = function.arity;

        // Check if already compiled
        if let Some(compiled) = self.jit_context.get(&name) {
            return Ok(compiled.clone());
        }

        // Compile the function
        let compiler = self.get_jit_compiler();
        match compiler.compile_function(function) {
            Ok(ptr) => {
                let compiled = CompiledFunction {
                    ptr,
                    arity,
                    name: name.clone(),
                };
                self.jit_context.register(name, ptr, arity);
                Ok(compiled)
            }
            Err(e) => Err(format!("JIT compilation failed: {}", e)),
        }
    }

    /// Register native/built-in functions
    fn register_natives(&mut self) {
        // Print function (without newline)
        self.define_native("print", -1, |args| {
            let mut output = String::new();
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    output.push(' ');
                }
                output.push_str(&format!("{arg}"));
            }
            // Try to capture output, fall back to stdout
            if !output::capture_print(&output) {
                print!("{output}");
            }
            Ok(Value::Null)
        });

        // Println function (with newline)
        self.define_native("println", -1, |args| {
            let mut output = String::new();
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    output.push(' ');
                }
                output.push_str(&format!("{arg}"));
            }
            // Try to capture output, fall back to stdout
            if !output::capture_output(&output) {
                println!("{output}");
            }
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

        // DataFrame operations for pipeline usage
        self.define_native("select", -1, |args| {
            if args.is_empty() {
                return Err("select requires a DataFrame as the first argument".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "select expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            // Collect column names from remaining arguments
            let col_names: Result<Vec<&str>, String> = args[1..]
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.as_str()),
                    other => Err(format!(
                        "select column names must be strings, got {}",
                        other.type_name()
                    )),
                })
                .collect();

            let col_names = col_names?;
            let result = df.select(&col_names).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // group_by function for pipeline usage: df |> group_by(.col1, .col2)
        self.define_native("group_by", -1, |args| {
            if args.is_empty() {
                return Err("group_by requires a DataFrame as the first argument".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df.clone(),
                other => {
                    return Err(format!(
                        "group_by expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            // Collect column names from remaining arguments
            let col_names: Result<Vec<String>, String> = args[1..]
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok((**s).clone()),
                    other => Err(format!(
                        "group_by column names must be strings, got {}",
                        other.type_name()
                    )),
                })
                .collect();

            let col_names = col_names?;
            if col_names.is_empty() {
                return Err("group_by requires at least one column name".to_string());
            }

            let grouped = GroupedDataFrame::new(df, col_names).map_err(|e| e.to_string())?;
            Ok(Value::GroupedDataFrame(std::sync::Arc::new(grouped)))
        });

        // Aggregation functions for pipeline usage: grouped |> sum("col", "output")
        self.define_native("sum", -1, |args| {
            native_grouped_agg(args, "sum", |gdf, col, out| gdf.sum(col, out))
        });

        self.define_native("mean", -1, |args| {
            native_grouped_agg(args, "mean", |gdf, col, out| gdf.mean(col, out))
        });

        self.define_native("avg", -1, |args| {
            native_grouped_agg(args, "avg", |gdf, col, out| gdf.mean(col, out))
        });

        self.define_native("min", -1, |args| {
            native_grouped_agg(args, "min", |gdf, col, out| gdf.min(col, out))
        });

        self.define_native("max", -1, |args| {
            native_grouped_agg(args, "max", |gdf, col, out| gdf.max(col, out))
        });

        self.define_native("count", -1, |args| {
            if args.is_empty() {
                return Err("count requires a GroupedDataFrame as the first argument".to_string());
            }
            let gdf = match &args[0] {
                Value::GroupedDataFrame(gdf) => gdf,
                other => return Err(format!("count expects GroupedDataFrame, got {}", other.type_name())),
            };
            let output = if args.len() > 1 {
                match &args[1] {
                    Value::String(s) => Some(s.as_str()),
                    other => return Err(format!("count output name must be string, got {}", other.type_name())),
                }
            } else {
                None
            };
            let result = gdf.count(output).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        self.define_native("first", -1, |args| {
            native_grouped_agg(args, "first", |gdf, col, out| gdf.first(col, out))
        });

        self.define_native("last", -1, |args| {
            native_grouped_agg(args, "last", |gdf, col, out| gdf.last(col, out))
        });

        // agg function for multiple aggregations: grouped |> agg(Agg.sum(...), Agg.count(...))
        self.define_native("agg", -1, |args| {
            if args.is_empty() {
                return Err("agg requires a GroupedDataFrame as the first argument".to_string());
            }
            let gdf = match &args[0] {
                Value::GroupedDataFrame(gdf) => gdf,
                other => return Err(format!("agg expects GroupedDataFrame, got {}", other.type_name())),
            };
            let specs: Result<Vec<AggSpec>, String> = args[1..]
                .iter()
                .map(|v| match v {
                    Value::AggSpec(spec) => Ok((**spec).clone()),
                    other => Err(format!("agg arguments must be AggSpec, got {}", other.type_name())),
                })
                .collect();
            let specs = specs?;
            if specs.is_empty() {
                return Err("agg requires at least one aggregation spec".to_string());
            }
            let result = gdf.aggregate(&specs).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // join(dataframe, other_dataframe, join_spec) -> DataFrame
        // Used in pipelines: df1 |> join(df2, Join.on("id"))
        self.define_native("join", 3, |args| {
            if args.len() != 3 {
                return Err("join requires 3 arguments: DataFrame, DataFrame, JoinSpec".to_string());
            }

            let left_df = match &args[0] {
                Value::DataFrame(df) => df.clone(),
                other => {
                    return Err(format!(
                        "join expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let right_df = match &args[1] {
                Value::DataFrame(df) => df.clone(),
                other => {
                    return Err(format!(
                        "join expects DataFrame as second argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let spec = match &args[2] {
                Value::JoinSpec(spec) => spec.clone(),
                other => {
                    return Err(format!(
                        "join expects JoinSpec as third argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let result = left_df.join(&right_df, &spec).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // sort_by(dataframe, col1, col2, ...) -> DataFrame
        // Used in pipelines: df |> sort_by("age", "-score") where - prefix means descending
        self.define_native("sort_by", -1, |args| {
            if args.is_empty() {
                return Err("sort_by requires a DataFrame as the first argument".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "sort_by expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            // Collect (column_name, descending) pairs from remaining arguments
            let mut sort_cols: Vec<(&str, bool)> = Vec::new();

            for arg in &args[1..] {
                match arg {
                    Value::String(s) => {
                        if let Some(col) = s.strip_prefix('-') {
                            sort_cols.push((col, true)); // descending
                        } else {
                            sort_cols.push((s.as_str(), false)); // ascending
                        }
                    }
                    other => {
                        return Err(format!(
                            "sort_by column names must be strings, got {}",
                            other.type_name()
                        ));
                    }
                }
            }

            if sort_cols.is_empty() {
                return Err("sort_by requires at least one column name".to_string());
            }

            let result = df.sort_by(&sort_cols).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // take(dataframe, n) -> DataFrame - alias for limit
        // Used in pipelines: df |> take(10)
        self.define_native("take", 2, |args| {
            if args.len() != 2 {
                return Err("take requires 2 arguments: DataFrame, n".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "take expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let n = match &args[1] {
                Value::Int(n) => *n as usize,
                other => {
                    return Err(format!(
                        "take expects Int as second argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let result = df.take_rows(n).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // limit(dataframe, n) -> DataFrame - alias for take
        self.define_native("limit", 2, |args| {
            if args.len() != 2 {
                return Err("limit requires 2 arguments: DataFrame, n".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "limit expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let n = match &args[1] {
                Value::Int(n) => *n as usize,
                other => {
                    return Err(format!(
                        "limit expects Int as second argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let result = df.take_rows(n).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // distinct(dataframe, col1?, col2?, ...) -> DataFrame
        // Used in pipelines: df |> distinct() or df |> distinct("name", "age")
        self.define_native("distinct", -1, |args| {
            if args.is_empty() {
                return Err("distinct requires a DataFrame as the first argument".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "distinct expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            if args.len() == 1 {
                // Distinct on all columns
                let result = df.distinct().map_err(|e| e.to_string())?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            } else {
                // Distinct on specified columns
                let col_names: Result<Vec<&str>, String> = args[1..]
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(s.as_str()),
                        other => Err(format!(
                            "distinct column names must be strings, got {}",
                            other.type_name()
                        )),
                    })
                    .collect();

                let col_names = col_names?;
                let result = df.distinct_by(&col_names).map_err(|e| e.to_string())?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
        });

        // unique is an alias for distinct
        self.define_native("unique", -1, |args| {
            if args.is_empty() {
                return Err("unique requires a DataFrame as the first argument".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "unique expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            if args.len() == 1 {
                let result = df.distinct().map_err(|e| e.to_string())?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            } else {
                let col_names: Result<Vec<&str>, String> = args[1..]
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(s.as_str()),
                        other => Err(format!(
                            "unique column names must be strings, got {}",
                            other.type_name()
                        )),
                    })
                    .collect();

                let col_names = col_names?;
                let result = df.distinct_by(&col_names).map_err(|e| e.to_string())?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
        });

        // drop_columns(dataframe, col1, col2, ...) -> DataFrame - remove columns
        // Used in pipelines: df |> drop_columns("col1", "col2")
        self.define_native("drop_columns", -1, |args| {
            if args.is_empty() {
                return Err("drop_columns requires a DataFrame as the first argument".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "drop_columns expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let col_names: Result<Vec<&str>, String> = args[1..]
                .iter()
                .map(|v| match v {
                    Value::String(s) => Ok(s.as_str()),
                    other => Err(format!(
                        "drop_columns column names must be strings, got {}",
                        other.type_name()
                    )),
                })
                .collect();

            let col_names = col_names?;
            if col_names.is_empty() {
                return Err("drop_columns requires at least one column name".to_string());
            }

            let result = DataFrame::drop(df, &col_names).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // rename(dataframe, old_name, new_name) -> DataFrame
        // Used in pipelines: df |> rename("old", "new")
        self.define_native("rename", 3, |args| {
            if args.len() != 3 {
                return Err("rename requires 3 arguments: DataFrame, old_name, new_name".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df,
                other => {
                    return Err(format!(
                        "rename expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let old_name = match &args[1] {
                Value::String(s) => s.as_str(),
                other => {
                    return Err(format!(
                        "rename expects String as second argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let new_name = match &args[2] {
                Value::String(s) => s.as_str(),
                other => {
                    return Err(format!(
                        "rename expects String as third argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let result = df.rename_column(old_name, new_name).map_err(|e| e.to_string())?;
            Ok(Value::DataFrame(std::sync::Arc::new(result)))
        });

        // Cube pipeline functions
        // dimension(cube_builder, name) -> CubeBuilder
        // Used in pipelines: Cube.from(df) |> dimension("region")
        self.define_native("dimension", -1, |args| {
            use std::sync::{Arc, Mutex};

            if args.is_empty() {
                return Err("dimension requires a CubeBuilder as the first argument".to_string());
            }

            let builder_arc = match &args[0] {
                Value::CubeBuilder(b) => b.clone(),
                other => {
                    return Err(format!(
                        "dimension expects CubeBuilder as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            // Take the builder out, apply the operation, and put it back
            let mut guard = builder_arc
                .lock()
                .map_err(|_| "CubeBuilder lock poisoned")?;
            let builder = guard
                .take()
                .ok_or("CubeBuilder has already been consumed (built)")?;

            // Add all dimension names
            let mut result_builder = builder;
            for arg in &args[1..] {
                let name = match arg {
                    Value::String(s) => s.as_str(),
                    other => {
                        return Err(format!(
                            "dimension names must be strings, got {}",
                            other.type_name()
                        ))
                    }
                };
                result_builder = result_builder.dimension(name).map_err(|e| e.to_string())?;
            }

            // Return a new CubeBuilder with the result
            Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(result_builder)))))
        });

        // measure(cube_builder, name, agg_func) -> CubeBuilder
        // Used in pipelines: Cube.from(df) |> measure("revenue", sum)
        self.define_native("measure", -1, |args| {
            use crate::data::CubeAggFunc;
            use std::sync::{Arc, Mutex};

            if args.len() < 3 {
                return Err(
                    "measure requires at least 3 arguments: CubeBuilder, column_name, agg_function"
                        .to_string(),
                );
            }

            let builder_arc = match &args[0] {
                Value::CubeBuilder(b) => b.clone(),
                other => {
                    return Err(format!(
                        "measure expects CubeBuilder as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let name = match &args[1] {
                Value::String(s) => s.as_str(),
                other => {
                    return Err(format!(
                        "measure column name must be a string, got {}",
                        other.type_name()
                    ))
                }
            };

            // Parse aggregation function - can be a native function like sum, count, etc.
            let agg_func = match &args[2] {
                Value::NativeFunction(f) => match f.name {
                    "sum" => CubeAggFunc::Sum,
                    "avg" | "mean" => CubeAggFunc::Avg,
                    "min" => CubeAggFunc::Min,
                    "max" => CubeAggFunc::Max,
                    "count" => CubeAggFunc::Count,
                    "first" => CubeAggFunc::First,
                    "last" => CubeAggFunc::Last,
                    other => {
                        return Err(format!(
                            "unsupported aggregation function for measure: {}",
                            other
                        ))
                    }
                },
                Value::String(s) => match s.to_lowercase().as_str() {
                    "sum" => CubeAggFunc::Sum,
                    "avg" | "mean" | "average" => CubeAggFunc::Avg,
                    "min" => CubeAggFunc::Min,
                    "max" => CubeAggFunc::Max,
                    "count" => CubeAggFunc::Count,
                    "count_distinct" => CubeAggFunc::CountDistinct,
                    "median" => CubeAggFunc::Median,
                    "stddev" | "std" => CubeAggFunc::StdDev,
                    "variance" | "var" => CubeAggFunc::Variance,
                    "first" => CubeAggFunc::First,
                    "last" => CubeAggFunc::Last,
                    other => {
                        return Err(format!(
                            "unsupported aggregation function for measure: {}",
                            other
                        ))
                    }
                },
                other => {
                    return Err(format!(
                        "measure aggregation must be a function or string, got {}",
                        other.type_name()
                    ))
                }
            };

            let mut guard = builder_arc
                .lock()
                .map_err(|_| "CubeBuilder lock poisoned")?;
            let builder = guard
                .take()
                .ok_or("CubeBuilder has already been consumed (built)")?;

            let result_builder = builder.measure(name, agg_func).map_err(|e| e.to_string())?;

            Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(result_builder)))))
        });

        // hierarchy(cube_builder, name, levels) -> CubeBuilder
        // Used in pipelines: Cube.from(df) |> hierarchy("time", ["year", "quarter", "month"])
        self.define_native("hierarchy", 3, |args| {
            use std::sync::{Arc, Mutex};

            let builder_arc = match &args[0] {
                Value::CubeBuilder(b) => b.clone(),
                other => {
                    return Err(format!(
                        "hierarchy expects CubeBuilder as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let name = match &args[1] {
                Value::String(s) => s.as_str(),
                other => {
                    return Err(format!(
                        "hierarchy name must be a string, got {}",
                        other.type_name()
                    ))
                }
            };

            let levels: Vec<&str> = match &args[2] {
                Value::List(list) => {
                    let borrowed = list.borrow();
                    let mut result = Vec::new();
                    for item in borrowed.iter() {
                        match item {
                            Value::String(s) => result.push(s.as_str()),
                            other => {
                                return Err(format!(
                                    "hierarchy level must be a string, got {}",
                                    other.type_name()
                                ))
                            }
                        }
                    }
                    // We need to convert to owned strings since we're borrowing
                    drop(borrowed);
                    let levels_owned: Vec<String> = match &args[2] {
                        Value::List(list) => list
                            .borrow()
                            .iter()
                            .filter_map(|v| match v {
                                Value::String(s) => Some((**s).clone()),
                                _ => None,
                            })
                            .collect(),
                        _ => unreachable!(),
                    };

                    let mut guard = builder_arc
                        .lock()
                        .map_err(|_| "CubeBuilder lock poisoned")?;
                    let builder = guard
                        .take()
                        .ok_or("CubeBuilder has already been consumed (built)")?;

                    let levels_refs: Vec<&str> = levels_owned.iter().map(|s| s.as_str()).collect();
                    let result_builder = builder
                        .hierarchy(name, &levels_refs)
                        .map_err(|e| e.to_string())?;

                    return Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(result_builder)))));
                }
                other => {
                    return Err(format!(
                        "hierarchy levels must be a list of strings, got {}",
                        other.type_name()
                    ))
                }
            };

            // This branch won't be reached due to the early return above
            #[allow(unreachable_code)]
            Ok(Value::Null)
        });

        // build(cube_builder) -> Cube
        // Used in pipelines: Cube.from(df) |> dimension("region") |> build()
        self.define_native("build", 1, |args| {
            let builder_arc = match &args[0] {
                Value::CubeBuilder(b) => b.clone(),
                other => {
                    return Err(format!(
                        "build expects CubeBuilder as argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let mut guard = builder_arc
                .lock()
                .map_err(|_| "CubeBuilder lock poisoned")?;
            let builder = guard
                .take()
                .ok_or("CubeBuilder has already been consumed (built)")?;

            let cube = builder.build().map_err(|e| e.to_string())?;
            Ok(Value::Cube(std::sync::Arc::new(cube)))
        });

        // OLAP Operations
        // slice(cube_or_query, dimension, value) -> CubeQuery
        // Used in pipelines: cube |> slice("region", "West")
        self.define_native("slice", 3, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            // Get dimension and value first
            let dimension = match &args[1] {
                Value::String(s) => (**s).clone(),
                other => {
                    return Err(format!(
                        "slice dimension must be a string, got {}",
                        other.type_name()
                    ))
                }
            };

            let value = match &args[2] {
                Value::String(s) => (**s).clone(),
                Value::Int(i) => i.to_string(),
                Value::Float(f) => f.to_string(),
                other => {
                    return Err(format!(
                        "slice value must be a string or number, got {}",
                        other.type_name()
                    ))
                }
            };

            // Handle both Cube and CubeQuery as input
            match &args[0] {
                Value::Cube(cube) => {
                    let query = CubeQuery::new(cube).slice(dimension, value);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    let new_query = query.slice(dimension, value);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                other => Err(format!(
                    "slice expects Cube or CubeQuery as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // dice(cube_or_query, filters_map) -> CubeQuery
        // Used in pipelines: cube |> dice({ region: "West", year: 2024 })
        self.define_native("dice", 2, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            // Parse filters from map
            let filters: Vec<(String, String)> = match &args[1] {
                Value::Map(map) => {
                    let borrowed = map.borrow();
                    let mut result = Vec::new();
                    for (key, val) in borrowed.iter() {
                        let dim = match key {
                            crate::bytecode::HashableValue::String(s) => (**s).clone(),
                            _ => return Err("dice filter keys must be strings".to_string()),
                        };
                        let value = match val {
                            Value::String(s) => (**s).clone(),
                            Value::Int(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                            other => {
                                return Err(format!(
                                    "dice filter value must be string or number, got {}",
                                    other.type_name()
                                ))
                            }
                        };
                        result.push((dim, value));
                    }
                    result
                }
                other => {
                    return Err(format!(
                        "dice expects a map of filters, got {}",
                        other.type_name()
                    ))
                }
            };

            // Handle both Cube and CubeQuery as input
            match &args[0] {
                Value::Cube(cube) => {
                    let mut query = CubeQuery::new(cube);
                    for (dim, val) in filters {
                        query = query.slice(dim, val);
                    }
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let mut query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    for (dim, val) in filters {
                        query = query.slice(dim, val);
                    }
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "dice expects Cube or CubeQuery as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // drill_down(cube_or_query, hierarchy_name) -> CubeQuery
        // drill_down(cube_or_query, hierarchy_name, levels) -> CubeQuery
        // Used in pipelines: cube |> drill_down("time") or cube |> drill_down("time", 2)
        self.define_native("drill_down", -1, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            if args.len() < 2 {
                return Err("drill_down requires at least 2 arguments: cube/query and hierarchy name".to_string());
            }

            let hierarchy = match &args[1] {
                Value::String(s) => (**s).clone(),
                other => {
                    return Err(format!(
                        "drill_down hierarchy name must be a string, got {}",
                        other.type_name()
                    ))
                }
            };

            // Optional: number of levels to drill down
            let levels_to_drill: usize = if args.len() > 2 {
                match &args[2] {
                    Value::Int(i) => {
                        if *i < 1 {
                            return Err("drill_down levels must be positive".to_string());
                        }
                        *i as usize
                    }
                    other => {
                        return Err(format!(
                            "drill_down levels must be an integer, got {}",
                            other.type_name()
                        ))
                    }
                }
            } else {
                1 // Default: drill down one level
            };

            // Handle both Cube and CubeQuery as input
            match &args[0] {
                Value::Cube(cube) => {
                    // Get hierarchy levels from the cube
                    let hierarchies = cube.hierarchies_with_levels();
                    let levels = hierarchies
                        .iter()
                        .find(|(name, _)| name == &hierarchy)
                        .map(|(_, levels)| levels.clone())
                        .ok_or_else(|| format!("hierarchy '{}' not found in cube", hierarchy))?;

                    // Take the appropriate number of levels for drill-down
                    let target_levels: Vec<String> = levels
                        .into_iter()
                        .take(levels_to_drill + 1)
                        .collect();

                    let query = CubeQuery::new(cube).drill_down(hierarchy, target_levels);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;

                    // For CubeQuery, we pass the hierarchy name and empty levels
                    // The actual level resolution happens during query execution
                    let new_query = query.drill_down(hierarchy, vec![]);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                other => Err(format!(
                    "drill_down expects Cube or CubeQuery as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // roll_up(cube_or_query, hierarchy_name) -> CubeQuery
        // roll_up(cube_or_query, hierarchy_name, levels) -> CubeQuery
        // Used in pipelines: cube |> roll_up("time") or cube |> roll_up("time", 2)
        self.define_native("roll_up", -1, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            if args.len() < 2 {
                return Err("roll_up requires at least 2 arguments: cube/query and hierarchy/dimension name".to_string());
            }

            let dimension = match &args[1] {
                Value::String(s) => (**s).clone(),
                other => {
                    return Err(format!(
                        "roll_up dimension name must be a string, got {}",
                        other.type_name()
                    ))
                }
            };

            // Optional: number of levels to roll up
            let levels_to_roll: usize = if args.len() > 2 {
                match &args[2] {
                    Value::Int(i) => {
                        if *i < 1 {
                            return Err("roll_up levels must be positive".to_string());
                        }
                        *i as usize
                    }
                    other => {
                        return Err(format!(
                            "roll_up levels must be an integer, got {}",
                            other.type_name()
                        ))
                    }
                }
            } else {
                1 // Default: roll up one level
            };

            // Handle both Cube and CubeQuery as input
            match &args[0] {
                Value::Cube(cube) => {
                    // For roll_up, we remove dimensions from grouping
                    // If it's a hierarchy, we might remove multiple levels
                    let dims_to_remove: Vec<String> = (0..levels_to_roll)
                        .map(|_| dimension.clone())
                        .collect();

                    let query = CubeQuery::new(cube).roll_up(dims_to_remove);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;

                    let dims_to_remove: Vec<String> = (0..levels_to_roll)
                        .map(|_| dimension.clone())
                        .collect();

                    let new_query = query.roll_up(dims_to_remove);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                other => Err(format!(
                    "roll_up expects Cube or CubeQuery as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // to_dataframe(cube_query) -> DataFrame
        // Used in pipelines: cube |> slice("region", "West") |> to_dataframe()
        self.define_native("to_dataframe", 1, |args| {
            use std::sync::Arc;

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .as_ref()
                        .ok_or("CubeQuery has already been consumed")?;

                    let df = query.to_dataframe().map_err(|e| e.to_string())?;
                    Ok(Value::DataFrame(Arc::new(df)))
                }
                Value::Cube(cube) => {
                    // If given a Cube directly, create a simple query and execute it
                    use crate::data::CubeQuery;
                    let query = CubeQuery::new(cube);
                    let df = query.to_dataframe().map_err(|e| e.to_string())?;
                    Ok(Value::DataFrame(Arc::new(df)))
                }
                other => Err(format!(
                    "to_dataframe expects CubeQuery or Cube, got {}",
                    other.type_name()
                )),
            }
        });

        // ========== Query Interface for CubeQuery ==========

        // query(cube) -> CubeQuery
        // Creates a new CubeQuery builder from a Cube for SQL-style queries
        // Used in pipelines: cube |> query() |> select("region", "SUM(revenue)") |> execute()
        self.define_native("query", 1, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            match &args[0] {
                Value::Cube(cube) => {
                    let query = CubeQuery::new(cube);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "query expects Cube as argument, got {}",
                    other.type_name()
                )),
            }
        });

        // cube_select(query, ...columns) -> CubeQuery
        // Sets the SELECT expressions for a cube query
        // Used in pipelines: query |> cube_select("region", "SUM(revenue) as total")
        self.define_native("cube_select", -1, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            if args.is_empty() {
                return Err("cube_select requires at least 1 argument: query".to_string());
            }

            // Extract select expressions from remaining args
            let exprs: Vec<String> = args[1..]
                .iter()
                .map(|arg| match arg {
                    Value::String(s) => Ok((**s).clone()),
                    other => Err(format!(
                        "cube_select columns must be strings, got {}",
                        other.type_name()
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    let new_query = query.select(exprs);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                Value::Cube(cube) => {
                    // Start a new query with select
                    let query = CubeQuery::new(cube).select(exprs);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "cube_select expects CubeQuery or Cube as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // where_(query, condition) -> CubeQuery
        // Sets the WHERE filter condition for a cube query (SQL-style expression)
        // Used in pipelines: query |> where_("year >= 2020 AND region = 'West'")
        self.define_native("where_", 2, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            let condition = match &args[1] {
                Value::String(s) => (**s).clone(),
                other => {
                    return Err(format!(
                        "where_ condition must be a string, got {}",
                        other.type_name()
                    ))
                }
            };

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    let new_query = query.where_clause(condition);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                Value::Cube(cube) => {
                    // Start a new query with where
                    let query = CubeQuery::new(cube).where_clause(condition);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "where_ expects CubeQuery or Cube as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // cube_group_by(query, ...columns) -> CubeQuery
        // Sets the GROUP BY columns for a cube query
        // Used in pipelines: query |> cube_group_by("region", "product")
        self.define_native("cube_group_by", -1, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            if args.is_empty() {
                return Err("cube_group_by requires at least 1 argument: query".to_string());
            }

            // Extract group by columns from remaining args
            let cols: Vec<String> = args[1..]
                .iter()
                .map(|arg| match arg {
                    Value::String(s) => Ok((**s).clone()),
                    other => Err(format!(
                        "cube_group_by columns must be strings, got {}",
                        other.type_name()
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    let new_query = query.group_by(cols);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                Value::Cube(cube) => {
                    // Start a new query with group_by
                    let query = CubeQuery::new(cube).group_by(cols);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "cube_group_by expects CubeQuery or Cube as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // cube_order_by(query, ...columns) -> CubeQuery
        // Sets the ORDER BY columns for a cube query
        // Used in pipelines: query |> cube_order_by("-SUM(revenue)", "region")
        self.define_native("cube_order_by", -1, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            if args.is_empty() {
                return Err("cube_order_by requires at least 1 argument: query".to_string());
            }

            // Extract order by columns from remaining args
            // Support "-column" for DESC ordering
            let cols: Vec<String> = args[1..]
                .iter()
                .map(|arg| match arg {
                    Value::String(s) => {
                        let col = (**s).clone();
                        // Convert "-column" to "column DESC"
                        if col.starts_with('-') {
                            Ok(format!("{} DESC", &col[1..]))
                        } else {
                            Ok(col)
                        }
                    }
                    other => Err(format!(
                        "cube_order_by columns must be strings, got {}",
                        other.type_name()
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    let new_query = query.order_by(cols);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                Value::Cube(cube) => {
                    // Start a new query with order_by
                    let query = CubeQuery::new(cube).order_by(cols);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "cube_order_by expects CubeQuery or Cube as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // cube_limit(query, count) -> CubeQuery
        // Sets the LIMIT for a cube query
        // Used in pipelines: query |> cube_limit(10)
        self.define_native("cube_limit", 2, |args| {
            use crate::data::CubeQuery;
            use std::sync::{Arc, Mutex};

            let count = match &args[1] {
                Value::Int(n) => {
                    if *n < 0 {
                        return Err("cube_limit count must be non-negative".to_string());
                    }
                    *n as usize
                }
                other => {
                    return Err(format!(
                        "cube_limit count must be an integer, got {}",
                        other.type_name()
                    ))
                }
            };

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let mut guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .take()
                        .ok_or("CubeQuery has already been consumed")?;
                    let new_query = query.limit(count);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(new_query)))))
                }
                Value::Cube(cube) => {
                    // Start a new query with limit
                    let query = CubeQuery::new(cube).limit(count);
                    Ok(Value::CubeQuery(Arc::new(Mutex::new(Some(query)))))
                }
                other => Err(format!(
                    "cube_limit expects CubeQuery or Cube as first argument, got {}",
                    other.type_name()
                )),
            }
        });

        // execute(query) -> DataFrame
        // Executes a CubeQuery and returns a DataFrame (alias for to_dataframe)
        // Used in pipelines: query |> select("region") |> execute()
        self.define_native("execute", 1, |args| {
            use std::sync::Arc;

            match &args[0] {
                Value::CubeQuery(query_arc) => {
                    let guard = query_arc
                        .lock()
                        .map_err(|_| "CubeQuery lock poisoned")?;
                    let query = guard
                        .as_ref()
                        .ok_or("CubeQuery has already been consumed")?;

                    let df = query.to_dataframe().map_err(|e| e.to_string())?;
                    Ok(Value::DataFrame(Arc::new(df)))
                }
                Value::Cube(cube) => {
                    // If given a Cube directly, create a simple query and execute it
                    use crate::data::CubeQuery;
                    let query = CubeQuery::new(cube);
                    let df = query.to_dataframe().map_err(|e| e.to_string())?;
                    Ok(Value::DataFrame(Arc::new(df)))
                }
                other => Err(format!(
                    "execute expects CubeQuery or Cube, got {}",
                    other.type_name()
                )),
            }
        });

        // to_cube(df) or to_cube(df, name) -> CubeBuilder
        // Creates a CubeBuilder from a DataFrame for pipeline usage
        // Used in pipelines: df |> to_cube("sales") |> dimension("region") |> build()
        self.define_native("to_cube", -1, |args| {
            use crate::data::CubeBuilder;
            use std::sync::{Arc, Mutex};

            if args.is_empty() {
                return Err("to_cube requires at least 1 argument: dataframe".to_string());
            }

            let df = match &args[0] {
                Value::DataFrame(df) => df.clone(),
                other => {
                    return Err(format!(
                        "to_cube expects DataFrame as first argument, got {}",
                        other.type_name()
                    ))
                }
            };

            let builder = if args.len() == 1 {
                // to_cube(df) - no name
                CubeBuilder::from_dataframe(&df).map_err(|e| e.to_string())?
            } else {
                // to_cube(df, "name") - with name
                match &args[1] {
                    Value::String(name) => {
                        CubeBuilder::from_dataframe_with_name(name.as_str(), &df)
                            .map_err(|e| e.to_string())?
                    }
                    other => {
                        return Err(format!(
                            "to_cube name must be a string, got {}",
                            other.type_name()
                        ))
                    }
                }
            };

            Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(builder)))))
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

        // Compression modules
        self.globals.insert("Gzip".to_string(), Value::NativeNamespace("Gzip"));
        self.globals.insert("Zip".to_string(), Value::NativeNamespace("Zip"));

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

        // Math module (constants and functions)
        self.globals.insert("Math".to_string(), Value::NativeNamespace("Math"));

        // User Input module
        self.globals.insert("Input".to_string(), Value::NativeNamespace("Input"));

        // Logging module
        self.globals.insert("Log".to_string(), Value::NativeNamespace("Log"));

        // System info module
        self.globals.insert("System".to_string(), Value::NativeNamespace("System"));

        // Database module
        self.globals.insert("Db".to_string(), Value::NativeNamespace("Db"));

        // Network modules (TCP/UDP/WebSocket)
        self.globals.insert("Tcp".to_string(), Value::NativeNamespace("Tcp"));
        self.globals.insert("Udp".to_string(), Value::NativeNamespace("Udp"));
        self.globals.insert("WebSocket".to_string(), Value::NativeNamespace("WebSocket"));

        // Data operations module (DataFrame, Series)
        self.globals.insert("Data".to_string(), Value::NativeNamespace("Data"));

        // Aggregation builder module (for group_by + aggregate)
        self.globals.insert("Agg".to_string(), Value::NativeNamespace("Agg"));

        // Join builder module (for DataFrame joins)
        self.globals.insert("Join".to_string(), Value::NativeNamespace("Join"));

        // Cube module (OLAP cube for multi-dimensional analysis)
        self.globals.insert("Cube".to_string(), Value::NativeNamespace("Cube"));
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
        self.suspended_coroutine = None;

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

            // Check if execution was suspended (e.g., by await)
            if let Some(coroutine) = self.suspended_coroutine.take() {
                return Ok(coroutine);
            }
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

    /// Close ALL open upvalues (for coroutine suspension)
    fn close_all_upvalues(&mut self) {
        self.close_upvalues(0);
    }

    // ===== Coroutine suspension/resumption =====

    /// Suspend the current execution, creating a coroutine that can be resumed later.
    /// This is called when awaiting a pending future.
    fn suspend(&mut self, awaited_future: Value) -> Value {
        // Close all upvalues so the coroutine has self-contained state
        self.close_all_upvalues();

        // Convert frames to saved frames
        let saved_frames: Vec<SavedCallFrame> = self
            .frames
            .iter()
            .map(|f| SavedCallFrame {
                closure: f.closure.clone(),
                ip: f.ip,
                stack_base: f.stack_base,
            })
            .collect();

        // Convert handlers to saved handlers
        let saved_handlers: Vec<SavedExceptionHandler> = self
            .handlers
            .iter()
            .map(|h| SavedExceptionHandler {
                frame_index: h.frame_index,
                stack_depth: h.stack_depth,
                catch_ip: h.catch_ip,
                finally_ip: h.finally_ip,
            })
            .collect();

        // Create the coroutine state
        let coro = CoroutineState::suspended(
            saved_frames,
            self.stack.clone(),
            saved_handlers,
            awaited_future,
        );

        // Clear VM state
        self.frames.clear();
        self.stack.clear();
        self.handlers.clear();

        Value::Coroutine(Rc::new(RefCell::new(coro)))
    }

    /// Resume a suspended coroutine with a value (the result of the awaited future).
    /// Returns Ok(()) if resumption was successful and execution should continue.
    pub fn resume_coroutine(&mut self, coro: &CoroutineState, resume_value: Value) -> RuntimeResult<()> {
        // Restore frames
        self.frames = coro
            .frames
            .iter()
            .map(|f| CallFrame {
                closure: f.closure.clone(),
                ip: f.ip,
                stack_base: f.stack_base,
            })
            .collect();

        // Restore stack
        self.stack = coro.stack.clone();

        // Restore handlers
        self.handlers = coro
            .handlers
            .iter()
            .map(|h| ExceptionHandler {
                frame_index: h.frame_index,
                stack_depth: h.stack_depth,
                catch_ip: h.catch_ip,
                finally_ip: h.finally_ip,
            })
            .collect();

        // Push the resume value (result of the await)
        self.push(resume_value)?;

        Ok(())
    }

    /// Continue execution after resuming a coroutine.
    /// Returns the result of execution (either a final value or a new coroutine if suspended again).
    pub fn continue_execution(&mut self) -> RuntimeResult<Value> {
        self.execute()
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

    /// Comparison operation with Series support
    fn series_comparison_op<SeriesOp, ScalarOp, FlippedOp, I, F>(
        &mut self,
        op_name: &'static str,
        series_op: SeriesOp,
        scalar_op: ScalarOp,
        flipped_scalar_op: FlippedOp,
        int_op: I,
        float_op: F,
    ) -> RuntimeResult<()>
    where
        SeriesOp: FnOnce(&Series, &Series) -> crate::data::DataResult<Series>,
        ScalarOp: FnOnce(&Series, &Value) -> crate::data::DataResult<Series>,
        FlippedOp: FnOnce(&Series, &Value) -> crate::data::DataResult<Series>,
        I: FnOnce(i64, i64) -> bool,
        F: FnOnce(f64, f64) -> bool,
    {
        let right = self.pop()?;
        let left = self.pop()?;
        match (&left, &right) {
            // Series-Series comparison
            (Value::Series(s1), Value::Series(s2)) => {
                let result = series_op(s1, s2)
                    .map(|s| Value::Series(std::sync::Arc::new(s)))
                    .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                self.push(result)
            }
            // Series-Scalar comparison
            (Value::Series(s), scalar) => {
                let result = scalar_op(s, scalar)
                    .map(|s| Value::Series(std::sync::Arc::new(s)))
                    .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                self.push(result)
            }
            // Scalar-Series comparison (flip the operation)
            (scalar, Value::Series(s)) => {
                // For scalar < series, we need series > scalar (flip comparison)
                let result = flipped_scalar_op(s, scalar)
                    .map(|s| Value::Series(std::sync::Arc::new(s)))
                    .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                self.push(result)
            }
            // Scalar operations
            (Value::Int(x), Value::Int(y)) => self.push(Value::Bool(int_op(*x, *y))),
            (Value::Float(x), Value::Float(y)) => self.push(Value::Bool(float_op(*x, *y))),
            (Value::Int(x), Value::Float(y)) => self.push(Value::Bool(float_op(*x as f64, *y))),
            (Value::Float(x), Value::Int(y)) => self.push(Value::Bool(float_op(*x, *y as f64))),
            (Value::String(x), Value::String(y)) => {
                let result = match op_name {
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
                };
                self.push(Value::Bool(result))
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                expected: "comparable",
                got: left.type_name(),
                operation: op_name,
            })),
        }
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

        // Check if we can use JIT (requires JIT enabled and no upvalues)
        let can_jit = self.jit_enabled && closure.upvalues.is_empty();

        if can_jit {
            // Determine if we should use JIT based on execution mode
            let should_jit = match closure.function.execution_mode {
                ExecutionMode::Compile => true,
                ExecutionMode::CompileHot => {
                    // Check if already JIT-compiled
                    if self.jit_context.is_compiled(&closure.function.name) {
                        true
                    } else {
                        // Increment call count and check if threshold reached
                        let fn_ptr = Rc::as_ptr(&closure.function);
                        let count = self.call_counts.entry(fn_ptr).or_insert(0);
                        *count += 1;
                        *count >= self.hot_threshold
                    }
                }
                ExecutionMode::Interpret => false,
            };

            if should_jit {
                // Try to JIT compile and execute
                match self.call_closure_jit(&closure, arg_count) {
                    Ok(result) => {
                        // Pop the closure and arguments from stack
                        let pop_count = arg_count as usize + 1;
                        for _ in 0..pop_count {
                            self.pop()?;
                        }
                        // Push the result
                        return self.push(result);
                    }
                    Err(_) => {
                        // JIT compilation failed, fall back to interpreter
                        // This is expected for unsupported opcodes
                    }
                }
            }
        }

        // Stack layout: [..., closure, arg0, arg1, ...]
        // stack_base points to closure (slot 0 of the frame)
        let stack_base = self.stack.len() - arg_count as usize - 1;
        self.frames.push(CallFrame::new(closure, stack_base));

        Ok(())
    }

    /// Call a closure using JIT compilation
    fn call_closure_jit(&mut self, closure: &Rc<Closure>, arg_count: u8) -> Result<Value, String> {
        // Compile the function
        let compiled = self.jit_compile_function(&closure.function)?;

        // Collect arguments from the stack (they're after the closure)
        let stack_len = self.stack.len();
        let args: Vec<Value> = self.stack[stack_len - arg_count as usize..].to_vec();

        // Call the JIT-compiled function
        let result = call_jit_function(&compiled, &args);

        Ok(result)
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
                // Series operations
                (Value::Series(s1), Value::Series(s2)) => {
                    s1.add(&s2)
                        .map(|s| Value::Series(std::sync::Arc::new(s)))
                        .map_err(|e| RuntimeErrorKind::DataError(e.to_string()))
                }
                (Value::Series(s), scalar @ (Value::Int(_) | Value::Float(_))) => {
                    s.add_scalar(&scalar)
                        .map(|s| Value::Series(std::sync::Arc::new(s)))
                        .map_err(|e| RuntimeErrorKind::DataError(e.to_string()))
                }
                (scalar @ (Value::Int(_) | Value::Float(_)), Value::Series(s)) => {
                    s.add_scalar(&scalar)
                        .map(|s| Value::Series(std::sync::Arc::new(s)))
                        .map_err(|e| RuntimeErrorKind::DataError(e.to_string()))
                }
                // Scalar operations
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

            OpCode::Sub => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    // Series operations
                    (Value::Series(s1), Value::Series(s2)) => {
                        s1.sub(s2)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    (Value::Series(s), scalar @ (Value::Int(_) | Value::Float(_))) => {
                        s.sub_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    // Scalar - Series: need to negate and add
                    (scalar @ (Value::Int(_) | Value::Float(_)), Value::Series(s)) => {
                        let neg = s.neg()
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        neg.add_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    // Scalar operations
                    (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 - y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x - *y as f64),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "numeric",
                            got: left.type_name(),
                            operation: "-",
                        }));
                    }
                };
                self.push(result)?;
            }

            OpCode::Mul => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    // Series operations
                    (Value::Series(s1), Value::Series(s2)) => {
                        s1.mul(s2)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    (Value::Series(s), scalar @ (Value::Int(_) | Value::Float(_))) => {
                        s.mul_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    (scalar @ (Value::Int(_) | Value::Float(_)), Value::Series(s)) => {
                        s.mul_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    // Scalar operations
                    (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(*x as f64 * y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x * *y as f64),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "numeric",
                            got: left.type_name(),
                            operation: "*",
                        }));
                    }
                };
                self.push(result)?;
            }

            OpCode::Div => {
                let right = self.pop()?;
                let left = self.pop()?;
                let result = match (&left, &right) {
                    // Series operations
                    (Value::Series(s1), Value::Series(s2)) => {
                        s1.div(s2)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    (Value::Series(s), scalar @ (Value::Int(_) | Value::Float(_))) => {
                        s.div_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
                    // Note: scalar / Series is not supported (would need element-wise reciprocal)
                    // Scalar operations with zero checks
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
                    Value::Series(s) => {
                        s.neg()
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?
                    }
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
                match (&left, &right) {
                    // Series operations
                    (Value::Series(s1), Value::Series(s2)) => {
                        let result = s1.eq(s2)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    (Value::Series(s), scalar) => {
                        let result = s.eq_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    (scalar, Value::Series(s)) => {
                        let result = s.eq_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    _ => self.push(Value::Bool(left == right))?,
                }
            }

            OpCode::Ne => {
                let right = self.pop()?;
                let left = self.pop()?;
                match (&left, &right) {
                    // Series operations
                    (Value::Series(s1), Value::Series(s2)) => {
                        let result = s1.neq(s2)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    (Value::Series(s), scalar) => {
                        let result = s.neq_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    (scalar, Value::Series(s)) => {
                        let result = s.neq_scalar(scalar)
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    _ => self.push(Value::Bool(left != right))?,
                }
            }

            OpCode::Lt => self.series_comparison_op("<", Series::lt, Series::lt_scalar, Series::gt_scalar, |x, y| x < y, |x, y| x < y)?,
            OpCode::Le => self.series_comparison_op("<=", Series::le, Series::le_scalar, Series::ge_scalar, |x, y| x <= y, |x, y| x <= y)?,
            OpCode::Gt => self.series_comparison_op(">", Series::gt, Series::gt_scalar, Series::lt_scalar, |x, y| x > y, |x, y| x > y)?,
            OpCode::Ge => self.series_comparison_op(">=", Series::ge, Series::ge_scalar, Series::le_scalar, |x, y| x >= y, |x, y| x >= y)?,

            OpCode::Not => {
                let value = self.pop()?;
                match value {
                    Value::Series(s) => {
                        let result = s.not()
                            .map(|s| Value::Series(std::sync::Arc::new(s)))
                            .map_err(|e| self.runtime_error(RuntimeErrorKind::DataError(e.to_string())))?;
                        self.push(result)?;
                    }
                    _ => self.push(Value::Bool(!value.is_truthy()))?,
                }
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
                let field_count = self.read_u16() as usize;
                let type_name = self.get_constant_string(type_index)?;

                // Create struct and populate fields
                // Stack has (name, value) pairs, we pop in reverse order
                let mut instance = StructInstance::new(type_name);
                for _ in 0..field_count {
                    let value = self.pop()?;
                    let name = self.pop()?;
                    let field_name = match name {
                        Value::String(s) => (*s).clone(),
                        _ => {
                            return Err(self.runtime_error(RuntimeErrorKind::InvalidOperation(
                                format!(
                                    "expected string for field name, got {}",
                                    name.type_name()
                                ),
                            )));
                        }
                    };
                    instance.fields.insert(field_name, value);
                }
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
                // Exception will be handled at the top of the main execution loop
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
                let future = self.pop()?;
                match &future {
                    Value::Future(fut) => {
                        let fut_ref = fut.borrow();
                        match &fut_ref.status {
                            FutureStatus::Ready => {
                                // Future is ready, push its result and continue
                                let result = fut_ref.result.clone().unwrap_or(Value::Null);
                                drop(fut_ref); // Release borrow
                                self.push(result)?;
                            }
                            FutureStatus::Pending => {
                                // Future is pending - suspend execution
                                drop(fut_ref); // Release borrow before suspend
                                let coroutine = self.suspend(future);
                                self.suspended_coroutine = Some(coroutine);
                                // The execute loop will check this and return
                            }
                            FutureStatus::Failed(err) => {
                                // Future failed - throw an exception
                                let err_msg = err.clone();
                                drop(fut_ref);
                                return Err(self.runtime_error(RuntimeErrorKind::AsyncError(err_msg)));
                            }
                        }
                    }
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Future",
                            got: future.type_name(),
                            operation: "await",
                        }));
                    }
                }
            }

            OpCode::Breakpoint => {
                // No-op for now
            }

            OpCode::StateBinding => {
                // Create a StateBinding value that represents a reactive binding to a field path
                let path_index = self.read_u16() as usize;
                let path = self.get_constant_string(path_index)?;
                // Push a StateBinding value - for now represented as a tagged String
                // The GUI runtime will interpret this as a binding path
                self.push(Value::StateBinding(path))?;
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
            Value::String(_) | Value::List(_) | Value::Map(_) | Value::NativeNamespace(_) | Value::DbConnection(_) | Value::DataFrame(_) | Value::Series(_) | Value::GroupedDataFrame(_) | Value::SqlContext(_) | Value::Cube(_) | Value::CubeBuilder(_) | Value::CubeQuery(_) => {
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
            Value::TcpStream(stream) => {
                natives::tcp_stream_method(stream, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::TcpListener(listener) => {
                natives::tcp_listener_method(listener, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::UdpSocket(socket) => {
                natives::udp_socket_method(socket, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::WebSocket(ws) => {
                natives::websocket_method(ws, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::WebSocketServer(server) => {
                natives::websocket_server_method(server, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::WebSocketServerConn(conn) => {
                natives::websocket_server_conn_method(conn, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::DataFrame(df) => self.dataframe_method(df, method_name, &args)?,
            Value::Series(s) => self.series_method(s, method_name, &args)?,
            Value::GroupedDataFrame(gdf) => self.grouped_dataframe_method(gdf, method_name, &args)?,
            Value::SqlContext(ctx) => {
                natives::sql_context_method(ctx, method_name, &args)
                    .map_err(|msg| self.runtime_error(RuntimeErrorKind::UserError(msg)))?
            }
            Value::Cube(cube) => self.cube_method(cube, method_name, &args)?,
            Value::CubeBuilder(builder) => self.cubebuilder_method(builder, method_name, &args)?,
            Value::CubeQuery(query) => self.cubequery_method(query, method_name, &args)?,
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

    fn dataframe_method(
        &mut self,
        df: &std::sync::Arc<DataFrame>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            // Basic info
            "columns" => {
                let cols: Vec<Value> = df.columns().into_iter().map(Value::string).collect();
                Ok(Value::list(cols))
            }
            "rows" | "num_rows" | "len" => Ok(Value::Int(df.num_rows() as i64)),
            "num_columns" => Ok(Value::Int(df.num_columns() as i64)),
            "is_empty" => Ok(Value::Bool(df.is_empty())),
            "schema" => {
                // Return schema as a map of column name -> type string
                let mut schema_map = HashMap::new();
                for field in df.schema().fields() {
                    let key = HashableValue::String(Rc::new(field.name().clone()));
                    let type_str = Value::string(format!("{:?}", field.data_type()));
                    schema_map.insert(key, type_str);
                }
                Ok(Value::Map(Rc::new(RefCell::new(schema_map))))
            }

            // Row operations
            "head" => {
                let n = if args.is_empty() {
                    5
                } else {
                    match &args[0] {
                        Value::Int(n) => *n as usize,
                        _ => return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Int",
                            got: args[0].type_name(),
                            operation: "head",
                        })),
                    }
                };
                let result = df.head(n).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "tail" => {
                let n = if args.is_empty() {
                    5
                } else {
                    match &args[0] {
                        Value::Int(n) => *n as usize,
                        _ => return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "Int",
                            got: args[0].type_name(),
                            operation: "tail",
                        })),
                    }
                };
                let result = df.tail(n).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "sample" => {
                let n = match &args[0] {
                    Value::Int(n) => *n as usize,
                    _ => return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "Int",
                        got: args[0].type_name(),
                        operation: "sample",
                    })),
                };
                let result = df.sample(n).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }

            // Column operations
            "column" | "col" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(name) => {
                        let series = df.column(name).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Series(std::sync::Arc::new(series)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "column",
                    })),
                }
            }
            "select" => {
                let col_names: Result<Vec<&str>, _> = args
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(s.as_str()),
                        _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "String",
                            got: v.type_name(),
                            operation: "select",
                        })),
                    })
                    .collect();
                let col_names = col_names?;
                let result = df.select(&col_names).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "drop" | "drop_columns" => {
                let col_names: Result<Vec<&str>, _> = args
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(s.as_str()),
                        _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "String",
                            got: v.type_name(),
                            operation: "drop",
                        })),
                    })
                    .collect();
                let col_names = col_names?;
                let result = DataFrame::drop(df, &col_names).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "rename" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }
                match (&args[0], &args[1]) {
                    (Value::String(old_name), Value::String(new_name)) => {
                        let result = df.rename_column(old_name, new_name).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::DataFrame(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "rename",
                    })),
                }
            }

            // Display
            "to_string" | "print" => {
                let max_rows = if args.is_empty() {
                    20
                } else {
                    match &args[0] {
                        Value::Int(n) => *n as usize,
                        _ => 20,
                    }
                };
                Ok(Value::string(df.to_pretty_string(max_rows)))
            }

            // Filtering
            "filter" => self.dataframe_filter(df, args),

            // Grouping
            "group_by" => {
                // Collect column names from string arguments
                let col_names: Result<Vec<String>, _> = args
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok((**s).clone()),
                        _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "String",
                            got: v.type_name(),
                            operation: "group_by",
                        })),
                    })
                    .collect();
                let col_names = col_names?;

                if col_names.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "group_by requires at least one column name".to_string()
                    )));
                }

                let grouped = GroupedDataFrame::new(df.clone(), col_names).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::GroupedDataFrame(std::sync::Arc::new(grouped)))
            }

            // Join operations
            "join" => {
                // df.join(other_df, join_spec)
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }

                let right_df = match &args[0] {
                    Value::DataFrame(df) => df.clone(),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "DataFrame",
                            got: args[0].type_name(),
                            operation: "join",
                        }));
                    }
                };

                let spec = match &args[1] {
                    Value::JoinSpec(s) => s.clone(),
                    _ => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "JoinSpec",
                            got: args[1].type_name(),
                            operation: "join",
                        }));
                    }
                };

                let result = df.join(&right_df, &spec).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }

            // Sorting
            "sort_by" => {
                // Collect (column_name, descending) pairs
                // Args can be: strings (ascending) or tuples of (string, bool)
                // For now, we support simple form: sort_by("col1", "col2") - all ascending
                // And with descending flag via string prefixed with "-": sort_by("-col1", "col2")
                let mut sort_cols: Vec<(&str, bool)> = Vec::new();

                for arg in args {
                    match arg {
                        Value::String(s) => {
                            if let Some(col) = s.strip_prefix('-') {
                                sort_cols.push((col, true)); // descending
                            } else {
                                sort_cols.push((s.as_str(), false)); // ascending
                            }
                        }
                        _ => {
                            return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "String",
                                got: arg.type_name(),
                                operation: "sort_by",
                            }));
                        }
                    }
                }

                if sort_cols.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "sort_by requires at least one column name".to_string(),
                    )));
                }

                let result = df.sort_by(&sort_cols).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }

            // Take/Limit (alias for head)
            "take" | "limit" => {
                let n = if args.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }));
                } else {
                    match &args[0] {
                        Value::Int(n) => *n as usize,
                        _ => {
                            return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "Int",
                                got: args[0].type_name(),
                                operation: "take",
                            }));
                        }
                    }
                };
                let result = df.take_rows(n).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }

            // Distinct/Unique
            "distinct" | "unique" => {
                if args.is_empty() {
                    // Distinct on all columns
                    let result = df.distinct().map_err(|e| {
                        self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                    })?;
                    Ok(Value::DataFrame(std::sync::Arc::new(result)))
                } else {
                    // Distinct on specified columns
                    let col_names: Result<Vec<&str>, _> = args
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => Ok(s.as_str()),
                            _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "String",
                                got: v.type_name(),
                                operation: "distinct",
                            })),
                        })
                        .collect();
                    let col_names = col_names?;
                    let result = df.distinct_by(&col_names).map_err(|e| {
                        self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                    })?;
                    Ok(Value::DataFrame(std::sync::Arc::new(result)))
                }
            }

            // File I/O - write methods
            "to_parquet" | "write_parquet" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(path) => {
                        crate::data::write_parquet(df, path.as_str()).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Null)
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "to_parquet",
                    })),
                }
            }

            "to_csv" | "write_csv" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(path) => {
                        crate::data::write_csv(df, path.as_str()).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Null)
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "to_csv",
                    })),
                }
            }

            "to_json" | "write_json" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(path) => {
                        crate::data::write_json(df, path.as_str()).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Null)
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "to_json",
                    })),
                }
            }

            // Cube conversion - create a CubeBuilder from this DataFrame
            // Usage: df.to_cube() or df.to_cube("name")
            "to_cube" => {
                use crate::data::CubeBuilder;
                use std::sync::{Arc, Mutex};

                let builder = if args.is_empty() {
                    // df.to_cube() - no name
                    CubeBuilder::from_dataframe(df).map_err(|e| {
                        self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                    })?
                } else {
                    // df.to_cube("name") - with name
                    match &args[0] {
                        Value::String(name) => {
                            CubeBuilder::from_dataframe_with_name(name.as_str(), df).map_err(|e| {
                                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                            })?
                        }
                        _ => {
                            return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "String",
                                got: args[0].type_name(),
                                operation: "to_cube",
                            }))
                        }
                    }
                };

                Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(builder)))))
            }

            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "DataFrame".to_string(),
                field: method.to_string(),
            })),
        }
    }

    fn dataframe_filter(
        &mut self,
        df: &std::sync::Arc<DataFrame>,
        args: &[Value],
    ) -> RuntimeResult<Value> {
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

        // Iterate over rows and collect indices where predicate returns true
        let mut matching_indices = Vec::new();
        for (idx, row_result) in df.iter_rows().enumerate() {
            let row = row_result.map_err(|e| {
                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
            })?;
            let result = self.call_closure_sync(closure.clone(), vec![row])?;
            if result.is_truthy() {
                matching_indices.push(idx);
            }
        }

        // Create filtered DataFrame
        let filtered_df = df.filter_by_indices(&matching_indices).map_err(|e| {
            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
        })?;

        Ok(Value::DataFrame(std::sync::Arc::new(filtered_df)))
    }

    fn series_method(
        &self,
        series: &std::sync::Arc<Series>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            // Basic info
            "name" => Ok(Value::string(series.name())),
            "len" | "length" => Ok(Value::Int(series.len() as i64)),
            "is_empty" => Ok(Value::Bool(series.is_empty())),
            "dtype" | "data_type" => Ok(Value::string(format!("{:?}", series.data_type()))),
            "null_count" => Ok(Value::Int(series.null_count() as i64)),
            "count" => Ok(Value::Int(series.count() as i64)),

            // Element access
            "get" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::Int(idx) => {
                        series.get(*idx as usize).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "Int",
                        got: args[0].type_name(),
                        operation: "get",
                    })),
                }
            }
            "is_null" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::Int(idx) => Ok(Value::Bool(series.is_null(*idx as usize))),
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "Int",
                        got: args[0].type_name(),
                        operation: "is_null",
                    })),
                }
            }

            // Aggregations
            "sum" => series.sum().map_err(|e| {
                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
            }),
            "mean" | "avg" => series.mean().map_err(|e| {
                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
            }),
            "min" => series.min().map_err(|e| {
                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
            }),
            "max" => series.max().map_err(|e| {
                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
            }),

            // Conversion
            "to_list" | "to_values" => {
                let values = series.to_values().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::list(values))
            }

            // Rename
            "rename" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(name) => {
                        let new_series = Series::new(name.as_str(), series.array().clone());
                        Ok(Value::Series(std::sync::Arc::new(new_series)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "rename",
                    })),
                }
            }

            // String operations
            "is_string" => Ok(Value::Bool(series.is_string())),

            "str_len" => {
                let result = series.str_len().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::Series(std::sync::Arc::new(result)))
            }

            "str_contains" | "contains" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(pattern) => {
                        let result = series.str_contains(pattern.as_str()).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Series(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "str_contains",
                    })),
                }
            }

            "str_starts_with" | "starts_with" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(prefix) => {
                        let result = series.str_starts_with(prefix.as_str()).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Series(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "str_starts_with",
                    })),
                }
            }

            "str_ends_with" | "ends_with" => {
                if args.len() != 1 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::String(suffix) => {
                        let result = series.str_ends_with(suffix.as_str()).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Series(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "str_ends_with",
                    })),
                }
            }

            "str_to_lowercase" | "to_lowercase" | "lower" => {
                let result = series.str_to_lowercase().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::Series(std::sync::Arc::new(result)))
            }

            "str_to_uppercase" | "to_uppercase" | "upper" => {
                let result = series.str_to_uppercase().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::Series(std::sync::Arc::new(result)))
            }

            "str_trim" | "trim" => {
                let result = series.str_trim().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::Series(std::sync::Arc::new(result)))
            }

            "str_trim_start" | "trim_start" | "ltrim" => {
                let result = series.str_trim_start().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::Series(std::sync::Arc::new(result)))
            }

            "str_trim_end" | "trim_end" | "rtrim" => {
                let result = series.str_trim_end().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::Series(std::sync::Arc::new(result)))
            }

            "str_substring" | "substring" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 1,
                        got: args.len() as u8,
                    }));
                }
                match &args[0] {
                    Value::Int(start) => {
                        let len = if args.len() == 2 {
                            match &args[1] {
                                Value::Int(l) => Some(*l as u64),
                                _ => {
                                    return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                        expected: "Int",
                                        got: args[1].type_name(),
                                        operation: "str_substring",
                                    }))
                                }
                            }
                        } else {
                            None
                        };
                        let result = series.str_substring(*start, len).map_err(|e| {
                            self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                        })?;
                        Ok(Value::Series(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "Int",
                        got: args[0].type_name(),
                        operation: "str_substring",
                    })),
                }
            }

            "str_replace" | "replace" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }
                match (&args[0], &args[1]) {
                    (Value::String(pattern), Value::String(replacement)) => {
                        let result = series
                            .str_replace(pattern.as_str(), replacement.as_str())
                            .map_err(|e| {
                                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                            })?;
                        Ok(Value::Series(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[0].type_name(),
                        operation: "str_replace",
                    })),
                }
            }

            "str_split_get" | "split_get" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::ArityMismatch {
                        expected: 2,
                        got: args.len() as u8,
                    }));
                }
                match (&args[0], &args[1]) {
                    (Value::String(delimiter), Value::Int(index)) => {
                        let result = series
                            .str_split_get(delimiter.as_str(), *index as usize)
                            .map_err(|e| {
                                self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                            })?;
                        Ok(Value::Series(std::sync::Arc::new(result)))
                    }
                    _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String, Int",
                        got: args[0].type_name(),
                        operation: "str_split_get",
                    })),
                }
            }

            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "Series".to_string(),
                field: method.to_string(),
            })),
        }
    }

    fn grouped_dataframe_method(
        &self,
        gdf: &std::sync::Arc<GroupedDataFrame>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            // Info methods
            "num_groups" => Ok(Value::Int(gdf.num_groups() as i64)),
            "group_columns" => {
                let cols: Vec<Value> = gdf
                    .group_columns()
                    .iter()
                    .map(|s| Value::string(s.clone()))
                    .collect();
                Ok(Value::list(cols))
            }

            // Simple aggregation methods (return DataFrame)
            "sum" => {
                let (column, output) = self.parse_simple_agg_args(args, "sum")?;
                let result = gdf.sum(&column, output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "mean" | "avg" => {
                let (column, output) = self.parse_simple_agg_args(args, "mean")?;
                let result = gdf.mean(&column, output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "min" => {
                let (column, output) = self.parse_simple_agg_args(args, "min")?;
                let result = gdf.min(&column, output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "max" => {
                let (column, output) = self.parse_simple_agg_args(args, "max")?;
                let result = gdf.max(&column, output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "count" => {
                let output = if args.is_empty() {
                    None
                } else {
                    match &args[0] {
                        Value::String(s) => Some((**s).clone()),
                        _ => {
                            return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "String",
                                got: args[0].type_name(),
                                operation: "count",
                            }))
                        }
                    }
                };
                let result = gdf.count(output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "first" => {
                let (column, output) = self.parse_simple_agg_args(args, "first")?;
                let result = gdf.first(&column, output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }
            "last" => {
                let (column, output) = self.parse_simple_agg_args(args, "last")?;
                let result = gdf.last(&column, output.as_deref()).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }

            // Builder pattern aggregation: agg(Agg.sum(...), Agg.count(...), ...)
            "agg" | "aggregate" => {
                if args.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "agg requires at least one aggregation spec".to_string()
                    )));
                }

                // Collect AggSpec values from arguments
                let specs: Result<Vec<AggSpec>, _> = args
                    .iter()
                    .map(|v| match v {
                        Value::AggSpec(spec) => Ok((**spec).clone()),
                        _ => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "AggSpec",
                            got: v.type_name(),
                            operation: "agg",
                        })),
                    })
                    .collect();
                let specs = specs?;

                let result = gdf.aggregate(&specs).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;
                Ok(Value::DataFrame(std::sync::Arc::new(result)))
            }

            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "GroupedDataFrame".to_string(),
                field: method.to_string(),
            })),
        }
    }

    /// Parse arguments for simple aggregation methods like sum, mean, etc.
    fn parse_simple_agg_args(&self, args: &[Value], method: &'static str) -> RuntimeResult<(String, Option<String>)> {
        if args.is_empty() || args.len() > 2 {
            return Err(self.runtime_error(RuntimeErrorKind::UserError(format!(
                "{method} expects 1 or 2 arguments (column, optional output_name)"
            ))));
        }

        let column = match &args[0] {
            Value::String(s) => (**s).clone(),
            _ => {
                return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "String",
                    got: args[0].type_name(),
                    operation: method,
                }))
            }
        };

        let output = if args.len() == 2 {
            match &args[1] {
                Value::String(s) => Some((**s).clone()),
                _ => {
                    return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: args[1].type_name(),
                        operation: method,
                    }))
                }
            }
        } else {
            None
        };

        Ok((column, output))
    }

    // ===== Cube methods =====

    fn cube_method(
        &self,
        cube: &std::sync::Arc<crate::data::Cube>,
        method: &str,
        _args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            // Metadata methods
            "name" => Ok(cube.name().map(Value::string).unwrap_or(Value::Null)),
            "row_count" | "rows" => Ok(Value::Int(cube.row_count() as i64)),
            "batch_count" => Ok(Value::Int(cube.batch_count() as i64)),
            "dimensions" => {
                let dims: Vec<Value> = cube.dimension_names().into_iter().map(Value::string).collect();
                Ok(Value::list(dims))
            }
            "measures" => {
                let measures: Vec<Value> = cube.measure_names().into_iter().map(Value::string).collect();
                Ok(Value::list(measures))
            }
            "hierarchies" => {
                // Return a Map of hierarchy_name -> [level1, level2, ...]
                use crate::bytecode::HashableValue;
                use std::cell::RefCell;
                use std::collections::HashMap;
                use std::rc::Rc;

                let hierarchies = cube.hierarchies_with_levels();
                let mut map = HashMap::new();
                for (name, levels) in hierarchies {
                    let key = HashableValue::String(Rc::new(name));
                    let levels_list: Vec<Value> = levels.into_iter().map(Value::string).collect();
                    map.insert(key, Value::list(levels_list));
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            }
            "dimension_values" => {
                // dimension_values(dim_name) -> List of unique values
                if _args.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "dimension_values requires a dimension name argument".to_string()
                    )));
                }
                let dim_name = match &_args[0] {
                    Value::String(s) => (**s).clone(),
                    other => return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: other.type_name(),
                        operation: "dimension_values",
                    })),
                };

                // Check if dimension exists
                if !cube.has_dimension(&dim_name) {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        format!("dimension '{}' not found in cube", dim_name)
                    )));
                }

                // Get unique values using a query
                let values = cube.dimension_values(&dim_name).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;

                Ok(Value::list(values))
            }
            "current_level" => {
                // current_level(hierarchy_name) -> String (the current level in the hierarchy)
                if _args.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "current_level requires a hierarchy name argument".to_string()
                    )));
                }
                let hierarchy_name = match &_args[0] {
                    Value::String(s) => (**s).clone(),
                    other => return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: other.type_name(),
                        operation: "current_level",
                    })),
                };

                // Get the current level (for Cube, this is the first level of the hierarchy)
                match cube.current_level(&hierarchy_name) {
                    Some(level) => Ok(Value::string(level)),
                    None => Err(self.runtime_error(RuntimeErrorKind::UserError(
                        format!("hierarchy '{}' not found in cube", hierarchy_name)
                    ))),
                }
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "Cube".to_string(),
                field: method.to_string(),
            })),
        }
    }

    fn cubebuilder_method(
        &self,
        builder: &std::sync::Arc<std::sync::Mutex<Option<crate::data::CubeBuilder>>>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        use crate::data::CubeAggFunc;
        use std::sync::{Arc, Mutex};

        match method {
            // dimension(name, ...) - add one or more dimensions
            "dimension" => {
                let mut guard = builder
                    .lock()
                    .map_err(|_| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder lock poisoned".to_string())))?;
                let inner_builder = guard
                    .take()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder has already been consumed (built)".to_string())))?;

                let mut result_builder = inner_builder;
                for arg in args {
                    let name = match arg {
                        Value::String(s) => s.as_str(),
                        other => {
                            return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                expected: "String",
                                got: other.type_name(),
                                operation: "dimension",
                            }))
                        }
                    };
                    result_builder = result_builder.dimension(name).map_err(|e| {
                        self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                    })?;
                }

                Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(result_builder)))))
            }

            // measure(name, agg_func) - add a measure with aggregation
            "measure" => {
                if args.len() < 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "measure requires 2 arguments: column_name and aggregation_function".to_string()
                    )));
                }

                let name = match &args[0] {
                    Value::String(s) => s.as_str(),
                    other => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "String",
                            got: other.type_name(),
                            operation: "measure",
                        }))
                    }
                };

                let agg_func = match &args[1] {
                    Value::NativeFunction(f) => match f.name {
                        "sum" => CubeAggFunc::Sum,
                        "avg" | "mean" => CubeAggFunc::Avg,
                        "min" => CubeAggFunc::Min,
                        "max" => CubeAggFunc::Max,
                        "count" => CubeAggFunc::Count,
                        "first" => CubeAggFunc::First,
                        "last" => CubeAggFunc::Last,
                        other => {
                            return Err(self.runtime_error(RuntimeErrorKind::UserError(format!(
                                "unsupported aggregation function for measure: {other}"
                            ))))
                        }
                    },
                    Value::String(s) => match s.to_lowercase().as_str() {
                        "sum" => CubeAggFunc::Sum,
                        "avg" | "mean" | "average" => CubeAggFunc::Avg,
                        "min" => CubeAggFunc::Min,
                        "max" => CubeAggFunc::Max,
                        "count" => CubeAggFunc::Count,
                        "count_distinct" => CubeAggFunc::CountDistinct,
                        "median" => CubeAggFunc::Median,
                        "stddev" | "std" => CubeAggFunc::StdDev,
                        "variance" | "var" => CubeAggFunc::Variance,
                        "first" => CubeAggFunc::First,
                        "last" => CubeAggFunc::Last,
                        other => {
                            return Err(self.runtime_error(RuntimeErrorKind::UserError(format!(
                                "unsupported aggregation function for measure: {other}"
                            ))))
                        }
                    },
                    other => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "function or String",
                            got: other.type_name(),
                            operation: "measure",
                        }))
                    }
                };

                let mut guard = builder
                    .lock()
                    .map_err(|_| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder lock poisoned".to_string())))?;
                let inner_builder = guard
                    .take()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder has already been consumed (built)".to_string())))?;

                let result_builder = inner_builder.measure(name, agg_func).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;

                Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(result_builder)))))
            }

            // hierarchy(name, levels) - add a hierarchy
            "hierarchy" => {
                if args.len() != 2 {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "hierarchy requires 2 arguments: name and list of levels".to_string()
                    )));
                }

                let name = match &args[0] {
                    Value::String(s) => (**s).clone(),
                    other => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "String",
                            got: other.type_name(),
                            operation: "hierarchy",
                        }))
                    }
                };

                let levels: Vec<String> = match &args[1] {
                    Value::List(list) => {
                        list.borrow()
                            .iter()
                            .map(|v| match v {
                                Value::String(s) => Ok((**s).clone()),
                                other => Err(self.runtime_error(RuntimeErrorKind::TypeError {
                                    expected: "String",
                                    got: other.type_name(),
                                    operation: "hierarchy level",
                                })),
                            })
                            .collect::<RuntimeResult<Vec<_>>>()?
                    }
                    other => {
                        return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                            expected: "List",
                            got: other.type_name(),
                            operation: "hierarchy",
                        }))
                    }
                };

                let mut guard = builder
                    .lock()
                    .map_err(|_| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder lock poisoned".to_string())))?;
                let inner_builder = guard
                    .take()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder has already been consumed (built)".to_string())))?;

                let levels_refs: Vec<&str> = levels.iter().map(|s| s.as_str()).collect();
                let result_builder = inner_builder.hierarchy(&name, &levels_refs).map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;

                Ok(Value::CubeBuilder(Arc::new(Mutex::new(Some(result_builder)))))
            }

            // build() - finalize the cube
            "build" => {
                let mut guard = builder
                    .lock()
                    .map_err(|_| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder lock poisoned".to_string())))?;
                let inner_builder = guard
                    .take()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UserError("CubeBuilder has already been consumed (built)".to_string())))?;

                let cube = inner_builder.build().map_err(|e| {
                    self.runtime_error(RuntimeErrorKind::UserError(e.to_string()))
                })?;

                Ok(Value::Cube(std::sync::Arc::new(cube)))
            }

            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "CubeBuilder".to_string(),
                field: method.to_string(),
            })),
        }
    }

    fn cubequery_method(
        &self,
        query: &std::sync::Arc<std::sync::Mutex<Option<crate::data::CubeQuery>>>,
        method: &str,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match method {
            "current_level" => {
                // current_level(hierarchy_name) -> String (the current level in the hierarchy)
                if args.is_empty() {
                    return Err(self.runtime_error(RuntimeErrorKind::UserError(
                        "current_level requires a hierarchy name argument".to_string()
                    )));
                }
                let hierarchy_name = match &args[0] {
                    Value::String(s) => (**s).clone(),
                    other => return Err(self.runtime_error(RuntimeErrorKind::TypeError {
                        expected: "String",
                        got: other.type_name(),
                        operation: "current_level",
                    })),
                };

                // Get the query without consuming it
                let guard = query
                    .lock()
                    .map_err(|_| self.runtime_error(RuntimeErrorKind::UserError("CubeQuery lock poisoned".to_string())))?;
                let q = guard
                    .as_ref()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UserError("CubeQuery has already been consumed".to_string())))?;

                // Get the current level from the query
                match q.current_level(&hierarchy_name) {
                    Some(level) => Ok(Value::string(level)),
                    None => Err(self.runtime_error(RuntimeErrorKind::UserError(
                        format!("hierarchy '{}' not found in cube", hierarchy_name)
                    ))),
                }
            }
            "cube_name" => {
                let guard = query
                    .lock()
                    .map_err(|_| self.runtime_error(RuntimeErrorKind::UserError("CubeQuery lock poisoned".to_string())))?;
                let q = guard
                    .as_ref()
                    .ok_or_else(|| self.runtime_error(RuntimeErrorKind::UserError("CubeQuery has already been consumed".to_string())))?;

                Ok(q.cube_name().map(Value::string).unwrap_or(Value::Null))
            }
            _ => Err(self.runtime_error(RuntimeErrorKind::UndefinedField {
                type_name: "CubeQuery".to_string(),
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

    /// Invoke a Stratum closure with the given arguments and return the result.
    ///
    /// This is the public API for GUI callback execution. It allows external code
    /// (like the GUI framework) to invoke Stratum closures in response to events.
    ///
    /// # Arguments
    /// * `closure` - The closure to invoke (must be `Value::Closure`)
    /// * `args` - Arguments to pass to the closure
    ///
    /// # Returns
    /// The return value of the closure, or an error if invocation fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The value is not a closure
    /// - Argument count doesn't match arity
    /// - The closure throws an exception
    pub fn invoke_callback(&mut self, closure: &Value, args: Vec<Value>) -> RuntimeResult<Value> {
        match closure {
            Value::Closure(c) => self.call_closure_sync(c.clone(), args),
            Value::NativeFunction(nf) => {
                let result = (nf.function)(&args).map_err(|msg| {
                    self.runtime_error(RuntimeErrorKind::Internal(msg))
                })?;
                Ok(result)
            }
            other => Err(self.runtime_error(RuntimeErrorKind::NotCallable(
                other.type_name(),
            ))),
        }
    }

    /// Get a reference to the global variables
    pub fn globals(&self) -> &HashMap<String, Value> {
        &self.globals
    }

    /// Get a mutable reference to the global variables
    pub fn globals_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.globals
    }

    // ===== Debug API =====

    /// Enable or disable debug mode
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_context.debug_mode = enabled;
    }

    /// Check if debug mode is enabled
    pub fn is_debug_mode(&self) -> bool {
        self.debug_context.debug_mode
    }

    /// Set the current source file for debug location tracking
    pub fn set_source_file(&mut self, path: Option<std::path::PathBuf>) {
        self.current_source = path;
    }

    /// Add a breakpoint at the given line
    pub fn add_breakpoint(&mut self, file: Option<std::path::PathBuf>, line: u32) -> u32 {
        self.debug_context.add_breakpoint(file, line)
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        self.debug_context.remove_breakpoint(id)
    }

    /// Clear all breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.debug_context.clear_breakpoints();
    }

    /// Get all breakpoint lines for a file
    pub fn get_breakpoint_lines(&self, file: Option<&std::path::PathBuf>) -> Vec<u32> {
        self.debug_context.get_breakpoint_lines(file)
    }

    /// Get the current debug state (call stack, locals, location)
    pub fn get_debug_state(&self, pause_reason: PauseReason) -> DebugState {
        let (location, function_name) = if !self.frames.is_empty() {
            let frame = &self.frames[self.frames.len() - 1];
            let line = frame.chunk().get_line(frame.ip.saturating_sub(1));
            let func_name = frame.closure.function.name.clone();
            let file = self.current_source.clone();
            (DebugLocation::new(file, line), func_name)
        } else {
            (DebugLocation::line(0), "<script>".to_string())
        };

        let call_stack = self.get_call_stack();
        let locals = self.get_local_variables();

        DebugState {
            location,
            function_name,
            call_stack,
            locals,
            pause_reason,
        }
    }

    /// Get the current call stack
    pub fn get_call_stack(&self) -> Vec<DebugStackFrame> {
        self.frames
            .iter()
            .rev()
            .enumerate()
            .map(|(idx, frame)| {
                let line = frame.chunk().get_line(frame.ip.saturating_sub(1));
                let source = frame.closure.function.chunk.source_name.clone();
                DebugStackFrame {
                    function_name: frame.closure.function.name.clone(),
                    file: source,
                    line,
                    index: idx,
                }
            })
            .collect()
    }

    /// Get local variables in the current frame
    pub fn get_local_variables(&self) -> Vec<DebugVariable> {
        if self.frames.is_empty() {
            return Vec::new();
        }

        let frame = &self.frames[self.frames.len() - 1];
        let func = &frame.closure.function;
        let mut locals = Vec::new();

        // Calculate the number of locals from the stack layout
        // Locals are stored from stack_base up to the current stack position
        let stack_end = if self.frames.len() > 1 {
            // If we have multiple frames, locals end at the next frame's base
            self.stack.len()
        } else {
            self.stack.len()
        };

        let local_count = stack_end.saturating_sub(frame.stack_base);
        for i in 0..local_count {
            let slot = frame.stack_base + i;
            if slot < self.stack.len() {
                let value = &self.stack[slot];
                let name = if i < func.arity as usize {
                    format!("arg{}", i)
                } else {
                    format!("local{}", i - func.arity as usize)
                };
                locals.push(DebugVariable::from_value(name, value));
            }
        }

        locals
    }

    /// Get the current line number
    pub fn get_current_line(&self) -> u32 {
        if self.frames.is_empty() {
            return 0;
        }
        let frame = &self.frames[self.frames.len() - 1];
        frame.chunk().get_line(frame.ip.saturating_sub(1))
    }

    /// Get the current frame depth
    fn get_frame_depth(&self) -> usize {
        self.frames.len()
    }

    /// Prepare for step into
    pub fn step_into(&mut self) {
        let depth = self.get_frame_depth();
        let line = self.get_current_line();
        self.debug_context.start_step_into(depth, line);
    }

    /// Prepare for step over
    pub fn step_over(&mut self) {
        let depth = self.get_frame_depth();
        let line = self.get_current_line();
        self.debug_context.start_step_over(depth, line);
    }

    /// Prepare for step out
    pub fn step_out(&mut self) {
        let depth = self.get_frame_depth();
        let line = self.get_current_line();
        self.debug_context.start_step_out(depth, line);
    }

    /// Run in debug mode, stopping at breakpoints and steps
    pub fn run_debug(&mut self, function: Rc<Function>) -> DebugStepResult {
        // Set up for debug execution
        self.debug_context.debug_mode = true;

        // Clear any leftover state from previous runs
        self.stack.clear();
        self.frames.clear();
        self.open_upvalues.clear();
        self.handlers.clear();
        self.current_exception = None;
        self.suspended_coroutine = None;

        // Wrap the function in a closure
        let closure = Rc::new(Closure::new(function));

        // Push the closure onto the stack
        self.stack.push(Value::Closure(closure.clone()));

        // Create the initial frame
        self.frames.push(CallFrame::new(closure, 0));

        // Run the debug execution loop
        self.execute_debug()
    }

    /// Continue debug execution from current position
    pub fn continue_debug(&mut self) -> DebugStepResult {
        self.debug_context.clear_step();
        self.execute_debug()
    }

    /// Execute with debug support (checking breakpoints and steps)
    fn execute_debug(&mut self) -> DebugStepResult {
        loop {
            // Check for exception propagation
            if let Some(exception) = self.current_exception.take() {
                if let Ok(handled) = self.handle_exception(exception.clone()) {
                    if !handled {
                        return DebugStepResult::Error(format!("Uncaught exception: {}", exception));
                    }
                    continue;
                } else {
                    return DebugStepResult::Error(format!("Uncaught exception: {}", exception));
                }
            }

            // Get current instruction
            if self.frames.is_empty() {
                return DebugStepResult::Completed(Value::Null);
            }

            let frame = self.current_frame();
            let chunk = frame.chunk();

            if frame.ip >= chunk.len() {
                let result = self.stack.pop().unwrap_or(Value::Null);
                return DebugStepResult::Completed(result);
            }

            // Check for breakpoints and stepping before executing
            let current_line = chunk.get_line(frame.ip);
            let frame_depth = self.frames.len();

            // Check breakpoint
            if self.debug_context.has_breakpoint(self.current_source.as_ref(), current_line) {
                // Find breakpoint ID
                let bp_id = 0; // Simplified - would need to look up actual ID
                self.debug_context.clear_step();
                return DebugStepResult::Paused(self.get_debug_state(PauseReason::Breakpoint(bp_id)));
            }

            // Check stepping
            if self.debug_context.should_break_for_step(frame_depth, current_line) {
                self.debug_context.clear_step();
                return DebugStepResult::Paused(self.get_debug_state(PauseReason::Step));
            }

            // Execute instruction
            let instruction = match chunk.read_byte(frame.ip) {
                Some(b) => b,
                None => return DebugStepResult::Error("Unexpected end of bytecode".to_string()),
            };

            let opcode = match OpCode::try_from(instruction) {
                Ok(op) => op,
                Err(op) => return DebugStepResult::Error(format!("Invalid opcode: {}", op)),
            };

            // Advance IP
            self.current_frame_mut().ip += 1;

            // Handle Return specially
            if opcode == OpCode::Return {
                let result = match self.pop() {
                    Ok(v) => v,
                    Err(e) => return DebugStepResult::Error(format!("{}", e)),
                };

                let frame = &self.frames[self.frames.len() - 1];
                self.close_upvalues(frame.stack_base);

                let frame = self.frames.pop().unwrap();

                if self.frames.is_empty() {
                    return DebugStepResult::Completed(result);
                }

                self.stack.truncate(frame.stack_base);
                if let Err(e) = self.push(result) {
                    return DebugStepResult::Error(format!("{}", e));
                }
                continue;
            }

            // Execute all other opcodes
            if let Err(e) = self.execute_opcode(opcode) {
                return DebugStepResult::Error(format!("{}", e));
            }

            // Check if execution was suspended
            if let Some(coroutine) = self.suspended_coroutine.take() {
                return DebugStepResult::Completed(coroutine);
            }
        }
    }

    // ===== Garbage Collection API =====

    /// Track a value for cycle collection
    ///
    /// Call this when creating container values (List, Map, Struct) that might
    /// participate in reference cycles.
    pub fn gc_track(&mut self, value: &Value) {
        self.gc.track(value);
    }

    /// Run cycle collection if the allocation threshold has been reached
    ///
    /// Returns the number of cycles broken, or 0 if collection was not triggered.
    pub fn gc_collect_if_needed(&mut self) -> usize {
        if self.gc.should_collect() {
            self.gc.collect(&self.stack, &self.globals, &self.open_upvalues)
        } else {
            0
        }
    }

    /// Force a cycle collection regardless of threshold
    ///
    /// Returns the number of cycles broken.
    pub fn gc_collect(&mut self) -> usize {
        self.gc.force_collect(&self.stack, &self.globals, &self.open_upvalues)
    }

    /// Get garbage collection statistics
    #[must_use]
    pub fn gc_stats(&self) -> crate::gc::GcStats {
        self.gc.stats()
    }

    /// Set the garbage collection threshold
    ///
    /// Collection will be triggered when this many container allocations occur.
    pub fn gc_set_threshold(&mut self, threshold: usize) {
        self.gc.set_threshold(threshold);
    }

    /// Get the current garbage collection threshold
    #[must_use]
    pub fn gc_threshold(&self) -> usize {
        self.gc.threshold()
    }

    /// Enable or disable automatic garbage collection
    pub fn gc_set_auto(&mut self, enabled: bool) {
        self.gc.set_auto_collect(enabled);
    }

    /// Check if automatic garbage collection is enabled
    #[must_use]
    pub fn gc_is_auto_enabled(&self) -> bool {
        self.gc.is_auto_collect_enabled()
    }
}

/// Helper function for native grouped aggregation functions
fn native_grouped_agg<F>(args: &[Value], name: &str, agg_fn: F) -> Result<Value, String>
where
    F: FnOnce(&GroupedDataFrame, &str, Option<&str>) -> crate::data::DataResult<DataFrame>,
{
    if args.is_empty() {
        return Err(format!("{name} requires a GroupedDataFrame as the first argument"));
    }
    let gdf = match &args[0] {
        Value::GroupedDataFrame(gdf) => gdf,
        other => return Err(format!("{name} expects GroupedDataFrame, got {}", other.type_name())),
    };
    if args.len() < 2 {
        return Err(format!("{name} requires at least a column name argument"));
    }
    let column = match &args[1] {
        Value::String(s) => s.as_str(),
        other => return Err(format!("{name} column name must be string, got {}", other.type_name())),
    };
    let output = if args.len() > 2 {
        match &args[2] {
            Value::String(s) => Some(s.as_str()),
            other => return Err(format!("{name} output name must be string, got {}", other.type_name())),
        }
    } else {
        None
    };
    let result = agg_fn(gdf, column, output).map_err(|e| e.to_string())?;
    Ok(Value::DataFrame(std::sync::Arc::new(result)))
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
            execution_mode: crate::ast::ExecutionMode::default(),
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
