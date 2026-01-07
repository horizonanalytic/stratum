//! Counter example demonstrating the Stratum GUI proof-of-concept
//!
//! Run with: cargo run --package stratum-gui --example counter

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use stratum_core::bytecode::{StructInstance, Value};
use stratum_gui::GuiRuntime;

fn main() {
    // Create initial state as a Stratum struct value
    let mut fields = HashMap::new();
    fields.insert("count".to_string(), Value::Int(0));

    let mut instance = StructInstance::new("CounterState".to_string());
    instance.fields = fields;
    let state = Value::Struct(Rc::new(RefCell::new(instance)));

    // Create and run the GUI
    let runtime = GuiRuntime::new(state)
        .with_window("Stratum Counter", (400, 300))
        .with_spacing(20.0);

    if let Err(e) = runtime.run() {
        eprintln!("GUI error: {e}");
        std::process::exit(1);
    }
}
