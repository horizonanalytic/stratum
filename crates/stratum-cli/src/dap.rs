//! Debug Adapter Protocol (DAP) implementation for Stratum
//!
//! This module implements the Debug Adapter Protocol, allowing VS Code and other
//! DAP-compatible editors to debug Stratum programs.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{anyhow, Result};
use dap::events::{OutputEventBody, StoppedEventBody};
use dap::responses::{
    ContinueResponse, ScopesResponse, SetBreakpointsResponse, StackTraceResponse, ThreadsResponse,
    VariablesResponse,
};
use dap::types::{
    Breakpoint, Capabilities, OutputEventCategory, Scope, Source, StackFrame,
    StoppedEventReason, Thread, Variable,
};
use dap::{events::Event, requests::Command, responses::ResponseBody, server::Server};
use stratum_core::{
    DebugStackFrame, DebugState, DebugStepResult, DebugVariable, PauseReason, VM,
};

/// Thread ID for the main thread (Stratum is single-threaded)
const MAIN_THREAD_ID: i64 = 1;

/// The Stratum Debug Adapter
pub struct StratumDebugAdapter {
    /// The VM instance for debugging
    vm: Option<VM>,
    /// Current source file being debugged
    source_file: Option<PathBuf>,
    /// Compiled function (if any)
    compiled_function: Option<Rc<stratum_core::bytecode::Function>>,
    /// Whether the debug session has started
    session_started: bool,
    /// Next variable reference ID
    next_var_ref: AtomicI64,
    /// Variable references (scope_id -> variables)
    variable_scopes: HashMap<i64, Vec<DebugVariable>>,
    /// Current debug state (when paused)
    current_state: Option<DebugState>,
    /// Breakpoint ID to DAP breakpoint mapping
    breakpoint_map: HashMap<u32, Breakpoint>,
    /// Whether to stop on entry
    stop_on_entry: bool,
}

impl Default for StratumDebugAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl StratumDebugAdapter {
    /// Create a new debug adapter
    pub fn new() -> Self {
        Self {
            vm: None,
            source_file: None,
            compiled_function: None,
            session_started: false,
            next_var_ref: AtomicI64::new(1),
            variable_scopes: HashMap::new(),
            current_state: None,
            breakpoint_map: HashMap::new(),
            stop_on_entry: false,
        }
    }

    /// Compile a source file for debugging
    fn compile_source(&mut self, source_path: &PathBuf) -> Result<()> {
        let source = std::fs::read_to_string(source_path)
            .map_err(|e| anyhow!("Failed to read source file: {}", e))?;

        // Parse as module
        let module = stratum_core::Parser::parse_module(&source).map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("{}", e)).collect();
            anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
        })?;

        // Type check
        let mut type_checker = stratum_core::TypeChecker::new();
        let type_result = type_checker.check_module(&module);
        if !type_result.errors.is_empty() {
            let error_msgs: Vec<String> = type_result
                .errors
                .iter()
                .map(|e| format!("{}", e))
                .collect();
            return Err(anyhow!("Type errors:\n{}", error_msgs.join("\n")));
        }

        // Compile
        let function = stratum_core::Compiler::with_source(source_path.display().to_string())
            .compile_module(&module)
            .map_err(|errors| {
                let error_msgs: Vec<String> = errors.iter().map(|e| format!("{}", e)).collect();
                anyhow!("Compile errors:\n{}", error_msgs.join("\n"))
            })?;

        self.compiled_function = Some(function);
        self.source_file = Some(source_path.clone());

        Ok(())
    }

    /// Start or continue execution
    fn run_execution(&mut self) -> Option<DebugStepResult> {
        let vm = self.vm.as_mut()?;

        let result = if self.session_started {
            vm.continue_debug()
        } else {
            self.session_started = true;
            let function = self.compiled_function.clone()?;
            vm.run_debug(function)
        };

        Some(result)
    }

    /// Convert DebugStackFrame to DAP StackFrame
    fn to_dap_stack_frame(&self, frame: &DebugStackFrame) -> StackFrame {
        let source = frame.file.as_ref().map(|f| Source {
            name: Some(
                PathBuf::from(f)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            path: Some(f.clone()),
            source_reference: None,
            presentation_hint: None,
            origin: None,
            sources: None,
            adapter_data: None,
            checksums: None,
        });

        StackFrame {
            id: frame.index as i64,
            name: frame.function_name.clone(),
            source,
            line: frame.line as i64,
            column: 1,
            end_line: None,
            end_column: None,
            can_restart: Some(false),
            instruction_pointer_reference: None,
            module_id: None,
            presentation_hint: None,
        }
    }

    /// Allocate a new variable reference
    fn alloc_var_ref(&self) -> i64 {
        self.next_var_ref.fetch_add(1, Ordering::SeqCst)
    }

    /// Get capabilities
    fn get_capabilities() -> Capabilities {
        Capabilities {
            supports_configuration_done_request: Some(true),
            supports_function_breakpoints: Some(false),
            supports_conditional_breakpoints: Some(false),
            supports_hit_conditional_breakpoints: Some(false),
            supports_evaluate_for_hovers: Some(false),
            exception_breakpoint_filters: None,
            supports_step_back: Some(false),
            supports_set_variable: Some(false),
            supports_restart_frame: Some(false),
            supports_goto_targets_request: Some(false),
            supports_step_in_targets_request: Some(false),
            supports_completions_request: Some(false),
            completion_trigger_characters: None,
            supports_modules_request: Some(false),
            additional_module_columns: None,
            supported_checksum_algorithms: None,
            supports_restart_request: Some(false),
            supports_exception_options: Some(false),
            supports_value_formatting_options: Some(false),
            supports_exception_info_request: Some(false),
            support_terminate_debuggee: Some(true),
            support_suspend_debuggee: Some(false),
            supports_delayed_stack_trace_loading: Some(false),
            supports_loaded_sources_request: Some(false),
            supports_log_points: Some(false),
            supports_terminate_threads_request: Some(false),
            supports_set_expression: Some(false),
            supports_terminate_request: Some(true),
            supports_data_breakpoints: Some(false),
            supports_read_memory_request: Some(false),
            supports_write_memory_request: Some(false),
            supports_disassemble_request: Some(false),
            supports_cancel_request: Some(false),
            supports_breakpoint_locations_request: Some(false),
            supports_clipboard_context: Some(false),
            supports_stepping_granularity: Some(false),
            supports_instruction_breakpoints: Some(false),
            supports_exception_filter_options: Some(false),
            supports_single_thread_execution_requests: Some(false),
            ..Default::default()
        }
    }

    /// Create a stopped event
    fn create_stopped_event(reason: StoppedEventReason, description: Option<String>) -> Event {
        Event::Stopped(StoppedEventBody {
            reason,
            description,
            thread_id: Some(MAIN_THREAD_ID),
            preserve_focus_hint: Some(false),
            text: None,
            all_threads_stopped: Some(true),
            hit_breakpoint_ids: None,
        })
    }

    /// Create an output event
    fn create_output_event(category: OutputEventCategory, output: String) -> Event {
        Event::Output(OutputEventBody {
            category: Some(category),
            output,
            group: None,
            variables_reference: None,
            source: None,
            line: None,
            column: None,
            data: None,
        })
    }
}

/// Run the Debug Adapter Protocol server on stdio
pub fn run_dap_server() -> Result<()> {
    let input = BufReader::new(std::io::stdin());
    let output = BufWriter::new(std::io::stdout());
    let mut server = Server::new(input, output);
    let mut adapter = StratumDebugAdapter::new();
    let mut initialized_sent = false;

    loop {
        let req = match server.poll_request()? {
            Some(req) => req,
            None => break, // EOF
        };

        match req.command {
            Command::Initialize(ref _args) => {
                let capabilities = StratumDebugAdapter::get_capabilities();
                let rsp = req.success(ResponseBody::Initialize(capabilities));
                server.respond(rsp)?;
            }

            Command::Launch(ref args) => {
                // Extract program path from launch arguments
                let program = args
                    .additional_data
                    .as_ref()
                    .and_then(|v| v.get("program"))
                    .and_then(|v| v.as_str());

                if let Some(program) = program {
                    let source_path = PathBuf::from(program);
                    if !source_path.exists() {
                        let rsp = req.error(&format!("Source file not found: {}", source_path.display()));
                        server.respond(rsp)?;
                        continue;
                    }

                    // Check for stopOnEntry
                    adapter.stop_on_entry = args
                        .additional_data
                        .as_ref()
                        .and_then(|v| v.get("stopOnEntry"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // Compile the source
                    if let Err(e) = adapter.compile_source(&source_path) {
                        let rsp = req.error(&format!("Compilation error: {}", e));
                        server.respond(rsp)?;
                        continue;
                    }

                    // Create and configure the VM
                    let mut vm = VM::new();
                    vm.set_debug_mode(true);
                    vm.set_source_file(Some(source_path.clone()));
                    adapter.vm = Some(vm);

                    // Send initialized event
                    if !initialized_sent {
                        server.send_event(Event::Initialized)?;
                        initialized_sent = true;
                    }

                    let rsp = req.success(ResponseBody::Launch);
                    server.respond(rsp)?;
                } else {
                    let rsp = req.error("Missing 'program' in launch configuration");
                    server.respond(rsp)?;
                }
            }

            Command::SetBreakpoints(ref args) => {
                if let Some(vm) = adapter.vm.as_mut() {
                    // Clear existing breakpoints for this source
                    vm.clear_breakpoints();
                    adapter.breakpoint_map.clear();

                    let source_path = args.source.path.as_ref().map(PathBuf::from);

                    let mut breakpoints = Vec::new();

                    if let Some(source_breakpoints) = &args.breakpoints {
                        for bp in source_breakpoints {
                            let line = bp.line as u32;
                            let bp_id = vm.add_breakpoint(source_path.clone(), line);

                            let dap_bp = Breakpoint {
                                id: Some(bp_id as i64),
                                verified: true,
                                message: None,
                                source: Some(args.source.clone()),
                                line: Some(bp.line),
                                column: None,
                                end_line: None,
                                end_column: None,
                                instruction_reference: None,
                                offset: None,
                            };

                            adapter.breakpoint_map.insert(bp_id, dap_bp.clone());
                            breakpoints.push(dap_bp);
                        }
                    }

                    let rsp = req.success(ResponseBody::SetBreakpoints(SetBreakpointsResponse {
                        breakpoints,
                    }));
                    server.respond(rsp)?;
                } else {
                    let rsp = req.error("VM not initialized");
                    server.respond(rsp)?;
                }
            }

            Command::ConfigurationDone => {
                let rsp = req.success(ResponseBody::ConfigurationDone);
                server.respond(rsp)?;

                // Start execution
                if adapter.stop_on_entry {
                    // Stop on entry
                    server.send_event(StratumDebugAdapter::create_stopped_event(
                        StoppedEventReason::Entry,
                        Some("Stopped on entry".to_string()),
                    ))?;
                } else {
                    // Run until breakpoint or completion
                    if let Some(result) = adapter.run_execution() {
                        match result {
                            DebugStepResult::Paused(state) => {
                                adapter.current_state = Some(state.clone());
                                let reason = match state.pause_reason {
                                    PauseReason::Breakpoint(_) => StoppedEventReason::Breakpoint,
                                    PauseReason::Step => StoppedEventReason::Step,
                                    PauseReason::Entry => StoppedEventReason::Entry,
                                };
                                server.send_event(StratumDebugAdapter::create_stopped_event(
                                    reason, None,
                                ))?;
                            }
                            DebugStepResult::Completed(_) => {
                                server.send_event(Event::Terminated(None))?;
                            }
                            DebugStepResult::Stopped => {
                                server.send_event(Event::Terminated(None))?;
                            }
                            DebugStepResult::Error(msg) => {
                                server.send_event(StratumDebugAdapter::create_output_event(
                                    OutputEventCategory::Stderr,
                                    format!("Error: {}\n", msg),
                                ))?;
                                server.send_event(Event::Terminated(None))?;
                            }
                        }
                    }
                }
            }

            Command::Threads => {
                let threads = vec![Thread {
                    id: MAIN_THREAD_ID,
                    name: "main".to_string(),
                }];

                let rsp = req.success(ResponseBody::Threads(ThreadsResponse { threads }));
                server.respond(rsp)?;
            }

            Command::StackTrace(ref _args) => {
                let frames = if let Some(state) = &adapter.current_state {
                    state
                        .call_stack
                        .iter()
                        .map(|f| adapter.to_dap_stack_frame(f))
                        .collect()
                } else if let Some(vm) = &adapter.vm {
                    vm.get_call_stack()
                        .iter()
                        .map(|f| adapter.to_dap_stack_frame(f))
                        .collect()
                } else {
                    Vec::new()
                };

                let total = frames.len() as i64;

                let rsp = req.success(ResponseBody::StackTrace(StackTraceResponse {
                    stack_frames: frames,
                    total_frames: Some(total),
                }));
                server.respond(rsp)?;
            }

            Command::Scopes(ref _args) => {
                // Clear old variable scopes
                adapter.variable_scopes.clear();

                // Get locals for the requested frame
                let locals = if let Some(state) = &adapter.current_state {
                    state.locals.clone()
                } else if let Some(vm) = &adapter.vm {
                    vm.get_local_variables()
                } else {
                    Vec::new()
                };

                // Allocate a variable reference for locals
                let locals_ref = adapter.alloc_var_ref();
                adapter.variable_scopes.insert(locals_ref, locals);

                let scopes = vec![Scope {
                    name: "Locals".to_string(),
                    presentation_hint: Some(dap::types::ScopePresentationhint::Locals),
                    variables_reference: locals_ref,
                    named_variables: None,
                    indexed_variables: None,
                    expensive: false,
                    source: None,
                    line: None,
                    column: None,
                    end_line: None,
                    end_column: None,
                }];

                let rsp = req.success(ResponseBody::Scopes(ScopesResponse { scopes }));
                server.respond(rsp)?;
            }

            Command::Variables(ref args) => {
                let var_ref = args.variables_reference;

                let variables = if let Some(debug_vars) = adapter.variable_scopes.get(&var_ref) {
                    debug_vars
                        .iter()
                        .map(|v| Variable {
                            name: v.name.clone(),
                            value: v.value.clone(),
                            type_field: Some(v.type_name.clone()),
                            presentation_hint: None,
                            evaluate_name: None,
                            variables_reference: 0,
                            named_variables: None,
                            indexed_variables: None,
                            memory_reference: None,
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                let rsp = req.success(ResponseBody::Variables(VariablesResponse { variables }));
                server.respond(rsp)?;
            }

            Command::Continue(ref _args) => {
                let rsp = req.success(ResponseBody::Continue(ContinueResponse {
                    all_threads_continued: Some(true),
                }));
                server.respond(rsp)?;

                if let Some(result) = adapter.run_execution() {
                    handle_execution_result(&mut adapter, &mut server, result)?;
                }
            }

            Command::Next(ref _args) => {
                if let Some(vm) = adapter.vm.as_mut() {
                    vm.step_over();
                }

                let rsp = req.success(ResponseBody::Next);
                server.respond(rsp)?;

                if let Some(result) = adapter.run_execution() {
                    handle_execution_result(&mut adapter, &mut server, result)?;
                }
            }

            Command::StepIn(ref _args) => {
                if let Some(vm) = adapter.vm.as_mut() {
                    vm.step_into();
                }

                let rsp = req.success(ResponseBody::StepIn);
                server.respond(rsp)?;

                if let Some(result) = adapter.run_execution() {
                    handle_execution_result(&mut adapter, &mut server, result)?;
                }
            }

            Command::StepOut(ref _args) => {
                if let Some(vm) = adapter.vm.as_mut() {
                    vm.step_out();
                }

                let rsp = req.success(ResponseBody::StepOut);
                server.respond(rsp)?;

                if let Some(result) = adapter.run_execution() {
                    handle_execution_result(&mut adapter, &mut server, result)?;
                }
            }

            Command::Pause(ref _args) => {
                let rsp = req.success(ResponseBody::Pause);
                server.respond(rsp)?;
            }

            Command::Disconnect(ref _args) => {
                adapter.vm = None;
                adapter.compiled_function = None;
                adapter.current_state = None;
                adapter.session_started = false;

                let rsp = req.success(ResponseBody::Disconnect);
                server.respond(rsp)?;
                break;
            }

            Command::Terminate(ref _args) => {
                adapter.vm = None;
                server.send_event(Event::Terminated(None))?;
                let rsp = req.success(ResponseBody::Terminate);
                server.respond(rsp)?;
            }

            _ => {
                // We don't need the command details for unsupported commands
                let rsp = req.error("Unsupported command");
                server.respond(rsp)?;
            }
        }
    }

    Ok(())
}

fn handle_execution_result<R: std::io::Read, W: std::io::Write>(
    adapter: &mut StratumDebugAdapter,
    server: &mut Server<R, W>,
    result: DebugStepResult,
) -> Result<()> {
    match result {
        DebugStepResult::Paused(state) => {
            adapter.current_state = Some(state.clone());
            let reason = match state.pause_reason {
                PauseReason::Breakpoint(_) => StoppedEventReason::Breakpoint,
                PauseReason::Step => StoppedEventReason::Step,
                PauseReason::Entry => StoppedEventReason::Entry,
            };
            server.send_event(StratumDebugAdapter::create_stopped_event(reason, None))?;
        }
        DebugStepResult::Completed(value) => {
            if !matches!(value, stratum_core::bytecode::Value::Null) {
                server.send_event(StratumDebugAdapter::create_output_event(
                    OutputEventCategory::Stdout,
                    format!("{}\n", value),
                ))?;
            }
            server.send_event(Event::Terminated(None))?;
        }
        DebugStepResult::Stopped => {
            server.send_event(Event::Terminated(None))?;
        }
        DebugStepResult::Error(msg) => {
            server.send_event(StratumDebugAdapter::create_output_event(
                OutputEventCategory::Stderr,
                format!("Error: {}\n", msg),
            ))?;
            server.send_event(Event::Terminated(None))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = StratumDebugAdapter::new();
        assert!(!adapter.session_started);
        assert!(adapter.vm.is_none());
    }

    #[test]
    fn test_var_ref_allocation() {
        let adapter = StratumDebugAdapter::new();
        let ref1 = adapter.alloc_var_ref();
        let ref2 = adapter.alloc_var_ref();
        assert_eq!(ref1, 1);
        assert_eq!(ref2, 2);
    }

    #[test]
    fn test_capabilities() {
        let caps = StratumDebugAdapter::get_capabilities();
        assert!(caps.supports_configuration_done_request.unwrap_or(false));
        assert!(caps.support_terminate_debuggee.unwrap_or(false));
    }
}
