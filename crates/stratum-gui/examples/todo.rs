//! Todo app example demonstrating Stratum GUI state management
//!
//! This example shows:
//! - State binding for reactive updates
//! - VStack/HStack layouts
//! - Checkbox widgets with two-way binding
//! - TextField for input
//! - Button widgets
//! - Conditional rendering
//!
//! Run with: cargo run --package stratum-gui --example todo

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use stratum_core::bytecode::{StructInstance, Value};
use stratum_gui::element::GuiElement;
use stratum_gui::theme::ThemePreset;
use stratum_gui::GuiRuntime;

fn main() {
    // Create initial state with todo items
    // Each todo has a label and a completion status
    let mut fields = HashMap::new();

    // Input field for new todos
    fields.insert(
        "new_todo_text".to_string(),
        Value::String(Rc::new(String::new())),
    );

    // Pre-populated todo items (since dynamic list manipulation requires VM callbacks)
    // In a full implementation, these would be stored as a Value::List
    fields.insert(
        "todo_0_label".to_string(),
        Value::String(Rc::new("Learn Stratum basics".to_string())),
    );
    fields.insert("todo_0_completed".to_string(), Value::Bool(true));

    fields.insert(
        "todo_1_label".to_string(),
        Value::String(Rc::new("Build a GUI app".to_string())),
    );
    fields.insert("todo_1_completed".to_string(), Value::Bool(false));

    fields.insert(
        "todo_2_label".to_string(),
        Value::String(Rc::new("Add data operations".to_string())),
    );
    fields.insert("todo_2_completed".to_string(), Value::Bool(false));

    fields.insert(
        "todo_3_label".to_string(),
        Value::String(Rc::new("Create dashboard".to_string())),
    );
    fields.insert("todo_3_completed".to_string(), Value::Bool(false));

    fields.insert(
        "todo_4_label".to_string(),
        Value::String(Rc::new("Deploy to production".to_string())),
    );
    fields.insert("todo_4_completed".to_string(), Value::Bool(false));

    // Track item count for display
    fields.insert("total_items".to_string(), Value::Int(5));

    let mut instance = StructInstance::new("TodoState".to_string());
    instance.fields = fields;
    let state = Value::Struct(Rc::new(RefCell::new(instance)));

    // Build the todo app UI
    let root = build_todo_ui();

    // Create and run the GUI
    let runtime = GuiRuntime::new(state)
        .with_window("Stratum Todo App", (500, 600))
        .with_padding(20.0)
        .with_theme_preset(ThemePreset::Light)
        .with_root(root);

    if let Err(e) = runtime.run() {
        eprintln!("GUI error: {e}");
        std::process::exit(1);
    }
}

/// Build the todo app UI using GuiElement builder pattern
fn build_todo_ui() -> GuiElement {
    // Title
    let title = GuiElement::text("Todo List").text_size(32.0).bold().build();

    let subtitle = GuiElement::text("Track your tasks with Stratum")
        .text_size(14.0)
        .build();

    // Input section for new todos
    let input_field = GuiElement::text_field()
        .placeholder("What needs to be done?")
        .bind_field("new_todo_text")
        .build();

    let add_button = GuiElement::button("Add Todo").build();

    let input_row = GuiElement::hstack()
        .spacing(10.0)
        .child(input_field)
        .child(add_button)
        .build();

    // Separator
    let separator = GuiElement::text("─────────────────────────────────").build();

    // Todo items section
    let items_header = GuiElement::text("Your Tasks:")
        .text_size(18.0)
        .bold()
        .build();

    // Build todo item rows
    let todo_0 = build_todo_item("todo_0_label", "todo_0_completed", "Learn Stratum basics");
    let todo_1 = build_todo_item("todo_1_label", "todo_1_completed", "Build a GUI app");
    let todo_2 = build_todo_item("todo_2_label", "todo_2_completed", "Add data operations");
    let todo_3 = build_todo_item("todo_3_label", "todo_3_completed", "Create dashboard");
    let todo_4 = build_todo_item("todo_4_label", "todo_4_completed", "Deploy to production");

    // Items container
    let items_list = GuiElement::vstack()
        .spacing(8.0)
        .child(todo_0)
        .child(todo_1)
        .child(todo_2)
        .child(todo_3)
        .child(todo_4)
        .build();

    // Footer with item count
    let footer = GuiElement::text("Click checkboxes to mark tasks complete")
        .text_size(12.0)
        .build();

    // Main layout
    GuiElement::vstack()
        .spacing(16.0)
        .child(title)
        .child(subtitle)
        .child(input_row)
        .child(separator)
        .child(items_header)
        .child(items_list)
        .child(footer)
        .build()
}

/// Build a single todo item row with checkbox and label
fn build_todo_item(_label_field: &str, completed_field: &str, label_text: &str) -> GuiElement {
    // Checkbox with state binding for completion status
    let checkbox = GuiElement::checkbox(label_text)
        .bind_field(completed_field)
        .build();

    // Wrap in an HStack for consistent layout
    GuiElement::hstack().spacing(10.0).child(checkbox).build()
}
