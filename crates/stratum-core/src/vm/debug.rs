//! Debug support for the Stratum virtual machine
//!
//! This module provides debugging infrastructure including breakpoints,
//! stepping, and state inspection.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::bytecode::Value;

/// Represents a debug location in source code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DebugLocation {
    /// Source file path (if known)
    pub file: Option<PathBuf>,
    /// Line number (1-indexed)
    pub line: u32,
}

impl DebugLocation {
    /// Create a new debug location
    pub fn new(file: Option<PathBuf>, line: u32) -> Self {
        Self { file, line }
    }

    /// Create a location with just a line number
    pub fn line(line: u32) -> Self {
        Self { file: None, line }
    }
}

/// A breakpoint in the debugger
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique breakpoint ID
    pub id: u32,
    /// Location of the breakpoint
    pub location: DebugLocation,
    /// Whether the breakpoint is enabled
    pub enabled: bool,
    /// Optional condition expression (not yet implemented)
    pub condition: Option<String>,
}

impl Breakpoint {
    /// Create a new breakpoint
    pub fn new(id: u32, location: DebugLocation) -> Self {
        Self {
            id,
            location,
            enabled: true,
            condition: None,
        }
    }
}

/// Debug action to take after a breakpoint or step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    /// Continue execution normally
    Continue,
    /// Step to the next line in the same function
    StepOver,
    /// Step into function calls
    StepInto,
    /// Step out of the current function
    StepOut,
    /// Stop execution completely
    Stop,
}

/// The result of a debug step
#[derive(Debug, Clone)]
pub enum DebugStepResult {
    /// Execution completed normally with a value
    Completed(Value),
    /// Execution paused at a breakpoint or step
    Paused(DebugState),
    /// Execution was stopped
    Stopped,
    /// An error occurred
    Error(String),
}

/// Current debug state when paused
#[derive(Debug, Clone)]
pub struct DebugState {
    /// Current source location
    pub location: DebugLocation,
    /// Current function name
    pub function_name: String,
    /// Call stack frames
    pub call_stack: Vec<DebugStackFrame>,
    /// Local variables in current scope
    pub locals: Vec<DebugVariable>,
    /// Why execution is paused
    pub pause_reason: PauseReason,
}

/// Reason for pausing execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseReason {
    /// Hit a breakpoint
    Breakpoint(u32),
    /// Completed a step
    Step,
    /// Entry point (start of debug session)
    Entry,
}

/// A stack frame in the debug call stack
#[derive(Debug, Clone)]
pub struct DebugStackFrame {
    /// Function name
    pub function_name: String,
    /// Source file (if known)
    pub file: Option<String>,
    /// Current line number
    pub line: u32,
    /// Index in call stack (0 = top)
    pub index: usize,
}

/// A variable in the debug view
#[derive(Debug, Clone)]
pub struct DebugVariable {
    /// Variable name
    pub name: String,
    /// Variable value (as string representation)
    pub value: String,
    /// Type name
    pub type_name: String,
}

impl DebugVariable {
    /// Create a debug variable from a Value
    pub fn from_value(name: String, value: &Value) -> Self {
        Self {
            name,
            value: format!("{}", value),
            type_name: value.type_name().to_string(),
        }
    }
}

/// Debug context that tracks debug state for a VM
#[derive(Debug, Default)]
pub struct DebugContext {
    /// Active breakpoints indexed by file and line
    breakpoints: HashMap<Option<PathBuf>, HashSet<u32>>,
    /// All breakpoints by ID
    breakpoints_by_id: HashMap<u32, Breakpoint>,
    /// Next breakpoint ID
    next_breakpoint_id: u32,
    /// Whether debug mode is active
    pub debug_mode: bool,
    /// Current stepping mode
    step_mode: Option<StepMode>,
    /// Frame depth when step was initiated (for step over/out)
    step_frame_depth: usize,
    /// Line when step was initiated (to detect line changes)
    step_line: u32,
}

/// Internal stepping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepMode {
    /// Step to next line (step into)
    Into,
    /// Step to next line in same or shallower frame (step over)
    Over,
    /// Step until frame depth decreases (step out)
    Out,
}

impl DebugContext {
    /// Create a new debug context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a breakpoint
    pub fn add_breakpoint(&mut self, file: Option<PathBuf>, line: u32) -> u32 {
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id += 1;

        let location = DebugLocation::new(file.clone(), line);
        let breakpoint = Breakpoint::new(id, location);

        self.breakpoints_by_id.insert(id, breakpoint);
        self.breakpoints
            .entry(file)
            .or_default()
            .insert(line);

        id
    }

    /// Remove a breakpoint by ID
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(bp) = self.breakpoints_by_id.remove(&id) {
            if let Some(lines) = self.breakpoints.get_mut(&bp.location.file) {
                lines.remove(&bp.location.line);
            }
            true
        } else {
            false
        }
    }

    /// Check if there's a breakpoint at the given location
    pub fn has_breakpoint(&self, file: Option<&PathBuf>, line: u32) -> bool {
        self.breakpoints
            .get(&file.cloned())
            .map(|lines| lines.contains(&line))
            .unwrap_or(false)
    }

    /// Clear all breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
        self.breakpoints_by_id.clear();
    }

    /// Get all breakpoint lines for a file
    pub fn get_breakpoint_lines(&self, file: Option<&PathBuf>) -> Vec<u32> {
        self.breakpoints
            .get(&file.cloned())
            .map(|lines| lines.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Start step into mode
    pub fn start_step_into(&mut self, current_frame_depth: usize, current_line: u32) {
        self.step_mode = Some(StepMode::Into);
        self.step_frame_depth = current_frame_depth;
        self.step_line = current_line;
    }

    /// Start step over mode
    pub fn start_step_over(&mut self, current_frame_depth: usize, current_line: u32) {
        self.step_mode = Some(StepMode::Over);
        self.step_frame_depth = current_frame_depth;
        self.step_line = current_line;
    }

    /// Start step out mode
    pub fn start_step_out(&mut self, current_frame_depth: usize, current_line: u32) {
        self.step_mode = Some(StepMode::Out);
        self.step_frame_depth = current_frame_depth;
        self.step_line = current_line;
    }

    /// Clear stepping mode
    pub fn clear_step(&mut self) {
        self.step_mode = None;
    }

    /// Check if we should break due to stepping
    pub fn should_break_for_step(&self, current_frame_depth: usize, current_line: u32) -> bool {
        match self.step_mode {
            None => false,
            Some(StepMode::Into) => {
                // Break on any line change
                current_line != self.step_line
            }
            Some(StepMode::Over) => {
                // Break when we're at the same or shallower depth and line changed
                current_frame_depth <= self.step_frame_depth && current_line != self.step_line
            }
            Some(StepMode::Out) => {
                // Break when frame depth decreased
                current_frame_depth < self.step_frame_depth
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_breakpoint() {
        let mut ctx = DebugContext::new();

        let id1 = ctx.add_breakpoint(None, 10);
        let id2 = ctx.add_breakpoint(None, 20);

        assert!(ctx.has_breakpoint(None, 10));
        assert!(ctx.has_breakpoint(None, 20));
        assert!(!ctx.has_breakpoint(None, 15));

        assert!(ctx.remove_breakpoint(id1));
        assert!(!ctx.has_breakpoint(None, 10));
        assert!(ctx.has_breakpoint(None, 20));

        assert!(!ctx.remove_breakpoint(id1)); // Already removed
        assert!(ctx.remove_breakpoint(id2));
    }

    #[test]
    fn test_breakpoint_with_file() {
        let mut ctx = DebugContext::new();
        let file = PathBuf::from("test.strat");

        ctx.add_breakpoint(Some(file.clone()), 5);

        assert!(ctx.has_breakpoint(Some(&file), 5));
        assert!(!ctx.has_breakpoint(None, 5));
        assert!(!ctx.has_breakpoint(Some(&PathBuf::from("other.strat")), 5));
    }

    #[test]
    fn test_step_into() {
        let mut ctx = DebugContext::new();
        ctx.start_step_into(1, 10);

        // Same line - don't break
        assert!(!ctx.should_break_for_step(1, 10));
        assert!(!ctx.should_break_for_step(2, 10)); // Deeper but same line

        // Different line - break
        assert!(ctx.should_break_for_step(1, 11));
        assert!(ctx.should_break_for_step(2, 11)); // Deeper and different line
    }

    #[test]
    fn test_step_over() {
        let mut ctx = DebugContext::new();
        ctx.start_step_over(2, 10);

        // Deeper frame - don't break even if line changed
        assert!(!ctx.should_break_for_step(3, 15));

        // Same depth, same line - don't break
        assert!(!ctx.should_break_for_step(2, 10));

        // Same depth, different line - break
        assert!(ctx.should_break_for_step(2, 11));

        // Shallower depth, different line - break
        assert!(ctx.should_break_for_step(1, 15));
    }

    #[test]
    fn test_step_out() {
        let mut ctx = DebugContext::new();
        ctx.start_step_out(3, 10);

        // Same or deeper depth - don't break
        assert!(!ctx.should_break_for_step(3, 15));
        assert!(!ctx.should_break_for_step(4, 20));

        // Shallower depth - break
        assert!(ctx.should_break_for_step(2, 15));
        assert!(ctx.should_break_for_step(1, 20));
    }
}
