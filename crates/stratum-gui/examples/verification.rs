//! Verification example for GUI framework
//!
//! This example exercises the key features needed for the verification checklist:
//! - Window opens with title and content
//! - VStack, HStack, Grid layouts work
//! - Core widgets render correctly (Text, Button, TextField, Checkbox, etc.)
//!
//! Run with: cargo run --package stratum-gui --example verification

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use stratum_core::bytecode::{StructInstance, Value};
use stratum_gui::element::GuiElement;
use stratum_gui::GuiRuntime;

fn main() {
    // Create initial state
    let mut fields = HashMap::new();
    fields.insert("count".to_string(), Value::Int(0));
    fields.insert("text_value".to_string(), Value::String(Rc::new(String::new())));
    fields.insert("checked".to_string(), Value::Bool(false));

    let mut instance = StructInstance::new("VerificationState".to_string());
    instance.fields = fields;
    let state = Value::Struct(Rc::new(RefCell::new(instance)));

    // Build the GUI element tree to verify layouts and widgets
    let root = build_verification_ui();

    // Create and run the GUI
    let runtime = GuiRuntime::new(state)
        .with_window("Stratum GUI - Verification", (800, 600))
        .with_padding(20.0)
        .with_root(root);

    if let Err(e) = runtime.run() {
        eprintln!("GUI error: {e}");
        std::process::exit(1);
    }
}

/// Build the verification UI using GuiElement
fn build_verification_ui() -> GuiElement {
    // Title section
    let title = GuiElement::text("GUI Framework Verification")
        .text_size(28.0)
        .bold()
        .build();

    let subtitle = GuiElement::text("Testing layouts and widgets")
        .text_size(16.0)
        .build();

    // Layout verification section - HStack
    let hstack_label = GuiElement::text("HStack Layout:").build();
    let hstack_demo = GuiElement::hstack()
        .spacing(10.0)
        .child(GuiElement::text("Item 1").build())
        .child(GuiElement::text("Item 2").build())
        .child(GuiElement::text("Item 3").build())
        .build();

    // Layout verification section - Grid
    let grid_label = GuiElement::text("Grid Layout (3 columns):").build();
    let grid_demo = GuiElement::grid(3)
        .spacing(8.0)
        .child(GuiElement::text("Cell 1").build())
        .child(GuiElement::text("Cell 2").build())
        .child(GuiElement::text("Cell 3").build())
        .child(GuiElement::text("Cell 4").build())
        .child(GuiElement::text("Cell 5").build())
        .child(GuiElement::text("Cell 6").build())
        .build();

    // Widget verification section - Button
    let button_label = GuiElement::text("Button Widget:").build();
    let button_demo = GuiElement::button("Click Me").build();

    // Widget verification section - TextField
    let textfield_label = GuiElement::text("TextField Widget:").build();
    let textfield_demo = GuiElement::text_field()
        .placeholder("Enter text here...")
        .build();

    // Widget verification section - Checkbox
    let checkbox_label = GuiElement::text("Checkbox Widget:").build();
    let checkbox_demo = GuiElement::checkbox("I agree to the terms").build();

    // Widget verification section - Radio buttons
    let radio_label = GuiElement::text("Radio Buttons:").build();
    let radio_demo = GuiElement::hstack()
        .spacing(16.0)
        .child(GuiElement::radio_button("Option A", "a").build())
        .child(GuiElement::radio_button("Option B", "b").build())
        .child(GuiElement::radio_button("Option C", "c").build())
        .build();

    // Widget verification section - Dropdown
    let dropdown_label = GuiElement::text("Dropdown Widget:").build();
    let dropdown_demo = GuiElement::dropdown(vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()])
        .dropdown_placeholder("Select a color...")
        .build();

    // Widget verification section - Slider
    let slider_label = GuiElement::text("Slider Widget:").build();
    let slider_demo = GuiElement::slider(0.0, 100.0)
        .slider_value(50.0)
        .build();

    // Widget verification section - Toggle
    let toggle_label = GuiElement::text("Toggle Widget:").build();
    let toggle_demo = GuiElement::toggle("Enable feature")
        .build();

    // Widget verification section - ProgressBar
    let progress_label = GuiElement::text("ProgressBar Widget:").build();
    let progress_demo = GuiElement::progress_bar(0.7)
        .build();

    // Build the main VStack layout
    GuiElement::vstack()
        .spacing(16.0)
        .child(title)
        .child(subtitle)
        // Layouts section
        .child(GuiElement::text("--- Layouts ---").bold().build())
        .child(hstack_label)
        .child(hstack_demo)
        .child(grid_label)
        .child(grid_demo)
        // Widgets section
        .child(GuiElement::text("--- Widgets ---").bold().build())
        .child(button_label)
        .child(button_demo)
        .child(textfield_label)
        .child(textfield_demo)
        .child(checkbox_label)
        .child(checkbox_demo)
        .child(radio_label)
        .child(radio_demo)
        .child(dropdown_label)
        .child(dropdown_demo)
        .child(slider_label)
        .child(slider_demo)
        .child(toggle_label)
        .child(toggle_demo)
        .child(progress_label)
        .child(progress_demo)
        .build()
}
