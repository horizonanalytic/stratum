//! Language bindings for the Stratum GUI framework
//!
//! This module provides the bridge between Stratum's VM and the GUI framework.
//! It registers the "Gui" namespace with the VM at runtime, allowing Stratum
//! code to create and manipulate GUI elements.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use stratum_core::bytecode::Value;
use stratum_core::vm::{RuntimeResult, VM};

use crate::element::GuiElement;
use crate::natives::gui_native_functions;
use crate::runtime::GuiRuntime;
use crate::theme::{StratumPalette, ThemePreset};

/// Pending theme change request
#[derive(Clone)]
pub enum PendingTheme {
    /// Set theme from a preset
    Preset(ThemePreset),
    /// Set a custom theme with name and palette
    Custom { name: String, palette: StratumPalette },
}

/// A pending field update request from a callback
#[derive(Clone)]
pub struct PendingFieldUpdate {
    /// The field name to update
    pub field: String,
    /// The new value
    pub value: Value,
}

// Thread-local storage for quit requests, themes, callbacks, and field updates
thread_local! {
    static QUIT_REQUESTED: Cell<bool> = const { Cell::new(false) };
    static PENDING_THEME: RefCell<Option<PendingTheme>> = const { RefCell::new(None) };
    /// Pending callbacks registered during view function execution
    /// These are drained when the runtime starts and registered with the callback registry
    static PENDING_CALLBACKS: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
    /// Pending field updates from callbacks
    /// These are processed after callback execution completes
    static PENDING_FIELD_UPDATES: RefCell<Vec<PendingFieldUpdate>> = const { RefCell::new(Vec::new()) };
}

/// Request application quit (called from Gui.quit())
pub fn request_quit() {
    QUIT_REQUESTED.with(|flag| flag.set(true));
}

/// Check if quit was requested and clear the flag
pub fn take_quit_request() -> bool {
    QUIT_REQUESTED.with(|flag| {
        let was_requested = flag.get();
        flag.set(false);
        was_requested
    })
}

/// Request a theme preset change (called from Gui.set_theme())
pub fn request_theme_preset(preset: ThemePreset) {
    PENDING_THEME.with(|theme| {
        *theme.borrow_mut() = Some(PendingTheme::Preset(preset));
    });
}

/// Request a custom theme change (called from Gui.custom_theme())
pub fn request_custom_theme(name: String, palette: StratumPalette) {
    PENDING_THEME.with(|theme| {
        *theme.borrow_mut() = Some(PendingTheme::Custom { name, palette });
    });
}

/// Take the pending theme request and clear it
pub fn take_pending_theme() -> Option<PendingTheme> {
    PENDING_THEME.with(|theme| theme.borrow_mut().take())
}

/// Register a callback closure and return its ID
///
/// This is called from Gui.register_callback() in Stratum code.
/// The callback is stored in thread-local storage and drained when
/// the runtime starts.
///
/// Note: IDs start from 1 to match CallbackRegistry's ID scheme.
pub fn register_pending_callback(callback: Value) -> i64 {
    PENDING_CALLBACKS.with(|callbacks| {
        let mut cbs = callbacks.borrow_mut();
        // IDs start from 1 to match CallbackRegistry::next_id which starts at 1
        let id = (cbs.len() + 1) as i64;
        cbs.push(callback);
        id
    })
}

/// Take all pending callbacks and clear the list
///
/// Called by GuiRuntime when starting to register all pending callbacks.
pub fn take_pending_callbacks() -> Vec<Value> {
    PENDING_CALLBACKS.with(|callbacks| {
        std::mem::take(&mut *callbacks.borrow_mut())
    })
}

/// Clear pending callbacks without returning them
///
/// Called to reset state between runs.
pub fn clear_pending_callbacks() {
    PENDING_CALLBACKS.with(|callbacks| {
        callbacks.borrow_mut().clear();
    });
}

/// Request a field update (called from Gui.update_field())
///
/// This queues a field update to be applied after the current callback completes.
pub fn request_field_update(field: String, value: Value) {
    PENDING_FIELD_UPDATES.with(|updates| {
        updates.borrow_mut().push(PendingFieldUpdate { field, value });
    });
}

/// Take all pending field updates and clear the list
///
/// Called by the runtime after callback execution to apply state changes.
pub fn take_pending_field_updates() -> Vec<PendingFieldUpdate> {
    PENDING_FIELD_UPDATES.with(|updates| {
        std::mem::take(&mut *updates.borrow_mut())
    })
}

/// Register the GUI namespace with the VM
///
/// This function should be called during application initialization to make
/// GUI functionality available to Stratum code.
///
/// # Example
/// ```ignore
/// let mut vm = VM::new();
/// stratum_gui::register_gui(&mut vm);
/// // Now Stratum code can use Gui.vstack(), Gui.button(), etc.
/// // Method chaining also works: element.bold().color(255, 0, 0)
/// ```
pub fn register_gui(vm: &mut VM) {
    // Register the main Gui namespace handler
    vm.register_namespace("Gui", gui_method);

    // Register special methods that need VM access
    vm.register_vm_method("Gui", "run", gui_run_method);
    vm.register_vm_method("Gui", "app", gui_app_method);
    vm.register_vm_method("Gui", "quit", gui_quit_method);
    vm.register_vm_method("Gui", "register_callback", gui_register_callback_method);
    vm.register_vm_method("Gui", "update_field", gui_update_field_method);

    // Register method handler for GuiElement values to enable method chaining
    vm.register_value_method_handler("GuiElement", gui_element_method);
}

/// Handle method calls on GuiElement values for fluent method chaining
///
/// This enables syntax like: `Gui.text("Hello").bold().color(255, 0, 0).width(100)`
///
/// The receiver (GuiElement) is prepended to the args and passed to the
/// appropriate native function.
pub fn gui_element_method(receiver: &Value, method: &str, args: &[Value]) -> Result<Value, String> {
    // Get the native function registry
    let natives = gui_native_functions();

    // Map fluent method names to native function names
    // The native functions expect the element as the first argument
    let native_name = match method {
        // Text styling - short fluent names
        "bold" => "gui_set_text_bold",
        "text_color" | "color" => "gui_set_text_color",
        "text_size" | "font_size" | "size" => "gui_set_text_size",

        // Common element properties - short fluent names
        "disabled" => "gui_set_disabled",
        "placeholder" => "gui_set_placeholder",
        "secure" => "gui_set_secure",
        "value" => "gui_set_value",
        "checked" => "gui_set_checked",
        "progress" => "gui_set_progress",
        "opacity" => "gui_set_opacity",

        // Layout properties - short fluent names
        "spacing" => "gui_set_spacing",
        "padding" => "gui_set_padding",
        "width" => "gui_set_width",
        "height" => "gui_set_height",
        "alignment" => "gui_set_alignment",

        // Styling - short fluent names
        "background" => "gui_set_background",
        "foreground" => "gui_set_foreground",
        "border_color" => "gui_set_border_color",
        "border_width" => "gui_set_border_width",
        "corner_radius" => "gui_set_corner_radius",

        // Image properties
        "content_fit" => "gui_set_content_fit",
        "image_path" => "gui_set_image_path",

        // Checkbox/Toggle/Radio properties
        "checkbox_label" => "gui_set_checkbox_label",
        "toggle_on" => "gui_set_toggle_on",
        "toggle_label" => "gui_set_toggle_label",
        "radio_value" => "gui_set_radio_value",
        "radio_selected" => "gui_set_radio_selected",
        "radio_label" => "gui_set_radio_label",

        // Dropdown properties
        "dropdown_options" | "options" => "gui_set_dropdown_options",
        "dropdown_selected" | "selected" => "gui_set_dropdown_selected",
        "dropdown_placeholder" => "gui_set_dropdown_placeholder",

        // Slider properties
        "slider_value" => "gui_set_slider_value",
        "slider_range" | "range" => "gui_set_slider_range",
        "slider_step" | "step" => "gui_set_slider_step",

        // Container operations
        "add_child" | "child" => "gui_add_child",

        // DataTable configuration
        "table_columns" | "columns" => "gui_set_table_columns",
        "page_size" => "gui_set_page_size",
        "current_page" => "gui_set_current_page",
        "sortable" => "gui_set_sortable",
        "sort_by" => "gui_set_sort_by",
        "selectable" => "gui_set_selectable",
        "selected_rows" => "gui_set_selected_rows",
        "column_width" => "gui_set_column_width",

        // Chart configuration
        "chart_title" | "title" => "gui_set_chart_title",
        "chart_size" => "gui_set_chart_size",
        "chart_data" => "gui_set_chart_data",
        "chart_data_arrays" => "gui_set_chart_data_arrays",
        "add_series" => "gui_add_chart_series",
        "chart_labels" | "labels" => "gui_set_chart_labels",
        "show_legend" | "legend" => "gui_set_show_legend",
        "show_grid" | "grid" => "gui_set_show_grid",
        "bar_color" => "gui_set_bar_color",
        "inner_radius" => "gui_set_inner_radius",

        // OLAP Cube widget configuration
        "cube" => "gui_set_cube",
        "row_dimensions" => "gui_set_row_dimensions",
        "measures" => "gui_set_measures",
        "cube_chart_type" | "chart_type" => "gui_set_cube_chart_type",
        "x_dimension" => "gui_set_x_dimension",
        "y_measure" => "gui_set_y_measure",
        "series_dimension" => "gui_set_series_dimension",
        "filter_dimension" => "gui_set_filter_dimension",
        "hierarchy" => "gui_set_hierarchy",
        "current_level" => "gui_set_current_level",

        // Interactive events
        "on_press" | "on_click" => "gui_on_press",
        "on_mouse_release" => "gui_on_mouse_release",
        "on_double_click" => "gui_on_double_click",
        "on_right_press" => "gui_on_right_press",
        "on_right_release" => "gui_on_right_release",
        "on_hover_enter" => "gui_on_hover_enter",
        "on_hover_exit" => "gui_on_hover_exit",
        "on_mouse_move" => "gui_on_mouse_move",
        "on_mouse_scroll" => "gui_on_mouse_scroll",
        "cursor" => "gui_set_cursor",

        // Form element events
        "on_change" => "gui_on_change",
        "on_submit" => "gui_on_submit",
        "on_toggle" => "gui_on_toggle",
        "on_select" => "gui_on_select",

        // DataTable events
        "on_sort" => "gui_on_sort",
        "on_page_change" => "gui_on_page_change",
        "on_selection_change" => "gui_on_selection_change",
        "on_row_click" => "gui_on_row_click",
        "on_cell_click" => "gui_on_cell_click",

        // OLAP events
        "on_drill" => "gui_on_drill",
        "on_roll_up" => "gui_on_roll_up",
        "on_level_change" => "gui_on_level_change",

        // Also support the full set_* names for compatibility
        "set_text_bold" => "gui_set_text_bold",
        "set_text_color" => "gui_set_text_color",
        "set_text_size" => "gui_set_text_size",
        "set_disabled" => "gui_set_disabled",
        "set_placeholder" => "gui_set_placeholder",
        "set_secure" => "gui_set_secure",
        "set_value" => "gui_set_value",
        "set_checked" => "gui_set_checked",
        "set_progress" => "gui_set_progress",
        "set_opacity" => "gui_set_opacity",
        "set_spacing" => "gui_set_spacing",
        "set_padding" => "gui_set_padding",
        "set_width" => "gui_set_width",
        "set_height" => "gui_set_height",
        "set_alignment" => "gui_set_alignment",
        "set_background" => "gui_set_background",
        "set_foreground" => "gui_set_foreground",
        "set_border_color" => "gui_set_border_color",
        "set_border_width" => "gui_set_border_width",
        "set_corner_radius" => "gui_set_corner_radius",
        "set_content_fit" => "gui_set_content_fit",
        "set_image_path" => "gui_set_image_path",
        "set_checkbox_label" => "gui_set_checkbox_label",
        "set_toggle_on" => "gui_set_toggle_on",
        "set_toggle_label" => "gui_set_toggle_label",
        "set_radio_value" => "gui_set_radio_value",
        "set_radio_selected" => "gui_set_radio_selected",
        "set_radio_label" => "gui_set_radio_label",
        "set_dropdown_options" => "gui_set_dropdown_options",
        "set_dropdown_selected" => "gui_set_dropdown_selected",
        "set_dropdown_placeholder" => "gui_set_dropdown_placeholder",
        "set_slider_value" => "gui_set_slider_value",
        "set_slider_range" => "gui_set_slider_range",
        "set_slider_step" => "gui_set_slider_step",
        "set_table_columns" => "gui_set_table_columns",
        "set_page_size" => "gui_set_page_size",
        "set_current_page" => "gui_set_current_page",
        "set_sortable" => "gui_set_sortable",
        "set_sort_by" => "gui_set_sort_by",
        "set_selectable" => "gui_set_selectable",
        "set_selected_rows" => "gui_set_selected_rows",
        "set_column_width" => "gui_set_column_width",
        "set_chart_title" => "gui_set_chart_title",
        "set_chart_size" => "gui_set_chart_size",
        "set_chart_data" => "gui_set_chart_data",
        "set_chart_data_arrays" => "gui_set_chart_data_arrays",
        "set_chart_labels" => "gui_set_chart_labels",
        "set_show_legend" => "gui_set_show_legend",
        "set_show_grid" => "gui_set_show_grid",
        "set_bar_color" => "gui_set_bar_color",
        "set_inner_radius" => "gui_set_inner_radius",
        "set_cube" => "gui_set_cube",
        "set_row_dimensions" => "gui_set_row_dimensions",
        "set_measures" => "gui_set_measures",
        "set_cube_chart_type" => "gui_set_cube_chart_type",
        "set_x_dimension" => "gui_set_x_dimension",
        "set_y_measure" => "gui_set_y_measure",
        "set_series_dimension" => "gui_set_series_dimension",
        "set_filter_dimension" => "gui_set_filter_dimension",
        "set_hierarchy" => "gui_set_hierarchy",
        "set_current_level" => "gui_set_current_level",
        "set_cursor" => "gui_set_cursor",
        "add_chart_series" => "gui_add_chart_series",
        "bind_field" => "gui_bind_field",

        _ => return Err(format!("GuiElement has no method '{method}'")),
    };

    // Build args with receiver prepended
    let mut full_args = Vec::with_capacity(args.len() + 1);
    full_args.push(receiver.clone());
    full_args.extend_from_slice(args);

    // Find and call the native function
    for (name, func) in &natives {
        if *name == native_name {
            return (func.function)(&full_args);
        }
    }

    Err(format!("GUI native function '{}' not found", native_name))
}

/// Dispatch GUI namespace methods
///
/// Maps clean Stratum method names (e.g., "vstack") to gui natives (e.g., "gui_vstack")
pub fn gui_method(method: &str, args: &[Value]) -> Result<Value, String> {
    // Get the native function registry
    let natives = gui_native_functions();

    // Map the clean method name to the gui_* prefixed name
    let native_name = match method {
        // Layout functions
        "vstack" => "gui_vstack",
        "hstack" => "gui_hstack",
        "zstack" => "gui_zstack",
        "grid" => "gui_grid",
        "scroll_view" => "gui_scroll_view",
        "spacer" => "gui_spacer",
        "container" => "gui_container",

        // Widget functions
        "text" => "gui_text",
        "button" => "gui_button",
        "text_field" => "gui_text_field",
        "checkbox" => "gui_checkbox",
        "radio_button" => "gui_radio_button",
        "dropdown" => "gui_dropdown",
        "slider" => "gui_slider",
        "toggle" => "gui_toggle",
        "progress_bar" => "gui_progress_bar",
        "image" => "gui_image",

        // DataTable functions
        "data_table" => "gui_data_table",

        // Chart functions
        "bar_chart" => "gui_bar_chart",
        "line_chart" => "gui_line_chart",
        "pie_chart" => "gui_pie_chart",

        // OLAP Cube widget functions
        "cube_table" => "gui_cube_table",
        "cube_chart" => "gui_cube_chart",
        "dimension_filter" => "gui_dimension_filter",
        "hierarchy_navigator" => "gui_hierarchy_navigator",
        "measure_selector" => "gui_measure_selector",

        // Conditional and list rendering
        "if" => "gui_if",
        "for_each" => "gui_for_each",

        // Computed property
        "computed" => "gui_computed",

        // Interactive element
        "interactive" => "gui_interactive",

        // Theme presets and management
        "theme_presets" => "gui_theme_presets",
        "set_theme" => "gui_set_theme",
        "custom_theme" => "gui_custom_theme",

        // Element modification functions
        "set_text_bold" => "gui_set_text_bold",
        "set_text_color" => "gui_set_text_color",
        "set_text_size" => "gui_set_text_size",
        "set_disabled" => "gui_set_disabled",
        "set_placeholder" => "gui_set_placeholder",
        "set_secure" => "gui_set_secure",
        "set_value" => "gui_set_value",
        "bind_field" => "gui_bind_field",
        "set_checked" => "gui_set_checked",
        "set_checkbox_label" => "gui_set_checkbox_label",
        "set_radio_value" => "gui_set_radio_value",
        "set_radio_selected" => "gui_set_radio_selected",
        "set_radio_label" => "gui_set_radio_label",
        "set_dropdown_options" => "gui_set_dropdown_options",
        "set_dropdown_selected" => "gui_set_dropdown_selected",
        "set_dropdown_placeholder" => "gui_set_dropdown_placeholder",
        "set_slider_value" => "gui_set_slider_value",
        "set_slider_range" => "gui_set_slider_range",
        "set_slider_step" => "gui_set_slider_step",
        "set_toggle_on" => "gui_set_toggle_on",
        "set_toggle_label" => "gui_set_toggle_label",
        "set_progress" => "gui_set_progress",
        "set_image_path" => "gui_set_image_path",
        "set_content_fit" => "gui_set_content_fit",
        "set_opacity" => "gui_set_opacity",
        "add_child" => "gui_add_child",
        "set_spacing" => "gui_set_spacing",
        "set_padding" => "gui_set_padding",
        "set_width" => "gui_set_width",
        "set_height" => "gui_set_height",
        "set_alignment" => "gui_set_alignment",
        "set_background" => "gui_set_background",
        "set_foreground" => "gui_set_foreground",
        "set_border_color" => "gui_set_border_color",
        "set_border_width" => "gui_set_border_width",
        "set_corner_radius" => "gui_set_corner_radius",

        // DataTable configuration
        "set_table_columns" => "gui_set_table_columns",
        "set_page_size" => "gui_set_page_size",
        "set_current_page" => "gui_set_current_page",
        "set_sortable" => "gui_set_sortable",
        "set_sort_by" => "gui_set_sort_by",
        "set_selectable" => "gui_set_selectable",
        "set_selected_rows" => "gui_set_selected_rows",
        "set_column_width" => "gui_set_column_width",
        "on_sort" => "gui_on_sort",
        "on_page_change" => "gui_on_page_change",
        "on_selection_change" => "gui_on_selection_change",
        "on_row_click" => "gui_on_row_click",
        "on_cell_click" => "gui_on_cell_click",

        // Chart configuration
        "set_chart_title" => "gui_set_chart_title",
        "set_chart_size" => "gui_set_chart_size",
        "set_chart_data" => "gui_set_chart_data",
        "set_chart_data_arrays" => "gui_set_chart_data_arrays",
        "add_chart_series" => "gui_add_chart_series",
        "set_chart_labels" => "gui_set_chart_labels",
        "set_show_legend" => "gui_set_show_legend",
        "set_show_grid" => "gui_set_show_grid",
        "set_bar_color" => "gui_set_bar_color",
        "set_inner_radius" => "gui_set_inner_radius",

        // OLAP Cube widget configuration
        "set_cube" => "gui_set_cube",
        "set_row_dimensions" => "gui_set_row_dimensions",
        "set_measures" => "gui_set_measures",
        "set_cube_chart_type" => "gui_set_cube_chart_type",
        "set_x_dimension" => "gui_set_x_dimension",
        "set_y_measure" => "gui_set_y_measure",
        "set_series_dimension" => "gui_set_series_dimension",
        "set_filter_dimension" => "gui_set_filter_dimension",
        "set_hierarchy" => "gui_set_hierarchy",
        "set_current_level" => "gui_set_current_level",
        "on_drill" => "gui_on_drill",
        "on_roll_up" => "gui_on_roll_up",
        "on_level_change" => "gui_on_level_change",

        // Interactive events
        "on_press" => "gui_on_press",
        "on_mouse_release" => "gui_on_mouse_release",
        "on_double_click" => "gui_on_double_click",
        "on_right_press" => "gui_on_right_press",
        "on_right_release" => "gui_on_right_release",
        "on_hover_enter" => "gui_on_hover_enter",
        "on_hover_exit" => "gui_on_hover_exit",
        "on_mouse_move" => "gui_on_mouse_move",
        "on_mouse_scroll" => "gui_on_mouse_scroll",
        "set_cursor" => "gui_set_cursor",

        // Form element events
        "on_change" => "gui_on_change",
        "on_submit" => "gui_on_submit",
        "on_toggle" => "gui_on_toggle",
        "on_select" => "gui_on_select",

        _ => return Err(format!("Gui has no method '{method}'")),
    };

    // Find and call the native function
    for (name, func) in &natives {
        if *name == native_name {
            return (func.function)(args);
        }
    }

    Err(format!("GUI native function '{}' not found", native_name))
}

/// Handle Gui.run() which runs a GUI element in a window
///
/// Signature: Gui.run(element, title?, width?, height?)
/// - element: A GuiElement to render
/// - title: Optional window title (default: "Stratum App")
/// - width: Optional window width (default: 800)
/// - height: Optional window height (default: 600)
pub fn gui_run_method(vm: &mut VM, _method: &str, args: &[Value]) -> RuntimeResult<Value> {
    use stratum_core::vm::RuntimeErrorKind;

    if args.is_empty() {
        return Err(vm.runtime_error(RuntimeErrorKind::ArityMismatch {
            expected: 1,
            got: 0,
        }));
    }

    // Extract the GuiElement from the first argument
    let element = match &args[0] {
        Value::GuiElement(e) => {
            // Downcast to GuiElement
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                elem.clone()
            } else {
                return Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "GuiElement",
                    got: "unknown GUI type",
                    operation: "Gui.run",
                }));
            }
        }
        other => {
            return Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                expected: "GuiElement",
                got: other.type_name(),
                operation: "Gui.run",
            }));
        }
    };

    // Extract optional window title
    let title = if let Some(Value::String(s)) = args.get(1) {
        s.as_str().to_string()
    } else {
        "Stratum App".to_string()
    };

    // Extract optional window size
    let width = if let Some(Value::Int(w)) = args.get(2) {
        *w as u32
    } else {
        800
    };
    let height = if let Some(Value::Int(h)) = args.get(3) {
        *h as u32
    } else {
        600
    };

    // Create an initial state (empty for now, can be extended later)
    let initial_state = Value::Null;

    // Create and run the GUI runtime
    let runtime = GuiRuntime::new(initial_state)
        .with_window(&title, (width, height))
        .with_root(element);

    // Run the GUI - this blocks until the window is closed
    runtime.run().map_err(|e| {
        vm.runtime_error(RuntimeErrorKind::UserError(format!("GUI error: {}", e)))
    })?;

    Ok(Value::Null)
}

/// Handle Gui.app() which creates a reactive GUI application with state
///
/// Signature: Gui.app(title, initial_state, view_fn)
/// - title: Window title string
/// - initial_state: Initial state value (typically a struct)
/// - view_fn: A closure that takes state and returns a GuiElement
///
/// The view_fn is re-invoked whenever state changes (via callbacks),
/// automatically re-rendering the UI. Blocks until the window is closed.
pub fn gui_app_method(vm: &mut VM, _method: &str, args: &[Value]) -> RuntimeResult<Value> {
    use stratum_core::vm::RuntimeErrorKind;

    if args.len() < 3 {
        return Err(vm.runtime_error(RuntimeErrorKind::ArityMismatch {
            expected: 3,
            got: args.len() as u8,
        }));
    }

    // Extract title
    let title = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        other => {
            return Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                expected: "String",
                got: other.type_name(),
                operation: "Gui.app title",
            }));
        }
    };

    // Extract initial state (any value, typically a struct)
    let initial_state = args[1].clone();

    // Extract view function (must be a closure)
    let view_fn = match &args[2] {
        Value::Closure(_) => args[2].clone(),
        other => {
            return Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                expected: "Closure",
                got: other.type_name(),
                operation: "Gui.app view_fn",
            }));
        }
    };

    // Extract optional window size from args 3 and 4
    let width = if let Some(Value::Int(w)) = args.get(3) {
        *w as u32
    } else {
        800
    };
    let height = if let Some(Value::Int(h)) = args.get(4) {
        *h as u32
    } else {
        600
    };

    // Invoke view_fn with initial state to get the initial element tree
    let initial_element = vm.invoke_callback(&view_fn, vec![initial_state.clone()]).map_err(|e| {
        vm.runtime_error(RuntimeErrorKind::UserError(format!(
            "Failed to invoke view function: {}",
            e
        )))
    })?;

    // Extract GuiElement from result
    let element = match &initial_element {
        Value::GuiElement(e) => {
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                elem.clone()
            } else {
                return Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                    expected: "GuiElement",
                    got: "unknown GUI type",
                    operation: "Gui.app view_fn result",
                }));
            }
        }
        other => {
            return Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                expected: "GuiElement",
                got: other.type_name(),
                operation: "Gui.app view_fn result",
            }));
        }
    };

    // Create a new VM for callback execution
    // This is needed because we only have &mut VM, but the runtime needs an owned VM
    // The new VM has the same bindings registered
    let mut callback_vm = VM::new();
    register_gui(&mut callback_vm);

    // Create the GUI runtime with state, view function, and VM for callbacks
    let runtime = GuiRuntime::new(initial_state)
        .with_window(&title, (width, height))
        .with_root(element)
        .with_view_fn(Arc::new(view_fn))
        .with_vm(callback_vm);

    // Run the GUI - this blocks until the window is closed
    runtime.run().map_err(|e| {
        vm.runtime_error(RuntimeErrorKind::UserError(format!("GUI error: {}", e)))
    })?;

    Ok(Value::Null)
}

/// Handle Gui.quit() which requests application shutdown
///
/// Signature: Gui.quit()
///
/// This sets a flag that causes the application to exit gracefully
/// after the current callback completes.
pub fn gui_quit_method(_vm: &mut VM, _method: &str, _args: &[Value]) -> RuntimeResult<Value> {
    // Set the quit flag - will be checked after callback execution
    request_quit();
    Ok(Value::Null)
}

/// Handle Gui.register_callback() which registers a closure for later invocation
///
/// Signature: Gui.register_callback(closure) -> Int
///
/// Returns a callback ID that can be passed to button handlers etc.
/// The callback will be invoked when the associated UI event occurs.
pub fn gui_register_callback_method(vm: &mut VM, _method: &str, args: &[Value]) -> RuntimeResult<Value> {
    use stratum_core::vm::RuntimeErrorKind;

    if args.is_empty() {
        return Err(vm.runtime_error(RuntimeErrorKind::ArityMismatch {
            expected: 1,
            got: 0,
        }));
    }

    // The argument should be a closure
    match &args[0] {
        Value::Closure(_) => {
            let id = register_pending_callback(args[0].clone());
            Ok(Value::Int(id))
        }
        other => {
            Err(vm.runtime_error(RuntimeErrorKind::TypeError {
                expected: "Closure",
                got: other.type_name(),
                operation: "Gui.register_callback",
            }))
        }
    }
}

/// Handle Gui.update_field() which updates a state field from within a callback
///
/// Signature: Gui.update_field(field_name, value)
///
/// This allows callbacks to modify application state. The update is queued
/// and applied after the callback completes, triggering a re-render.
///
/// # Example
/// ```stratum
/// let dec_id = Gui.register_callback(|s: CounterState| {
///     Gui.update_field("count", s.count - 1);
/// });
/// ```
pub fn gui_update_field_method(_vm: &mut VM, _method: &str, args: &[Value]) -> RuntimeResult<Value> {
    use stratum_core::vm::RuntimeErrorKind;

    if args.len() < 2 {
        return Err(_vm.runtime_error(RuntimeErrorKind::ArityMismatch {
            expected: 2,
            got: args.len() as u8,
        }));
    }

    // Extract field name
    let field_name = match &args[0] {
        Value::String(s) => s.as_str().to_string(),
        other => {
            return Err(_vm.runtime_error(RuntimeErrorKind::TypeError {
                expected: "String",
                got: other.type_name(),
                operation: "Gui.update_field field_name",
            }));
        }
    };

    // Queue the field update
    request_field_update(field_name, args[1].clone());

    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_core::VM;

    #[test]
    fn test_gui_method_unknown() {
        let result = gui_method("unknown_method", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Gui has no method"));
    }

    #[test]
    fn test_gui_method_vstack() {
        // Test that vstack creates an element
        let result = gui_method("vstack", &[]);
        assert!(result.is_ok());
        if let Value::GuiElement(elem) = result.unwrap() {
            assert_eq!(elem.kind_name(), "VStack");
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_method_text() {
        let result = gui_method("text", &[Value::string("Hello")]);
        assert!(result.is_ok());
        if let Value::GuiElement(elem) = result.unwrap() {
            assert_eq!(elem.kind_name(), "Text");
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_quit_flag_initially_false() {
        // Ensure the quit flag starts as false
        let was_requested = take_quit_request();
        assert!(!was_requested);
    }

    #[test]
    fn test_request_quit_sets_flag() {
        // Request quit and verify flag is set
        request_quit();
        let was_requested = take_quit_request();
        assert!(was_requested);

        // Flag should be cleared after take
        let was_requested_again = take_quit_request();
        assert!(!was_requested_again);
    }

    #[test]
    fn test_gui_quit_method_sets_flag() {
        // Clear any existing quit flag
        let _ = take_quit_request();

        // Call the quit method
        let mut vm = VM::new();
        let result = gui_quit_method(&mut vm, "quit", &[]);

        // Should succeed and return Null
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);

        // Flag should be set
        let was_requested = take_quit_request();
        assert!(was_requested);
    }

    #[test]
    fn test_theme_preset_request() {
        // Clear any pending theme
        let _ = take_pending_theme();

        // Request theme preset
        request_theme_preset(ThemePreset::Dracula);

        // Verify theme was set
        let pending = take_pending_theme();
        assert!(pending.is_some());
        if let Some(PendingTheme::Preset(preset)) = pending {
            assert_eq!(preset.name(), "dracula");
        } else {
            panic!("Expected preset theme");
        }

        // Should be cleared after take
        assert!(take_pending_theme().is_none());
    }

    #[test]
    fn test_custom_theme_request() {
        use crate::theme::Color;

        // Clear any pending theme
        let _ = take_pending_theme();

        // Create custom palette
        let palette = StratumPalette::new(
            Color::rgb(30, 30, 30),      // background
            Color::rgb(200, 200, 200),   // text
            Color::rgb(100, 150, 255),   // primary
            Color::rgb(100, 200, 100),   // success
            Color::rgb(255, 200, 100),   // warning
            Color::rgb(255, 100, 100),   // danger
        );

        // Request custom theme
        request_custom_theme("MyTheme".to_string(), palette);

        // Verify theme was set
        let pending = take_pending_theme();
        assert!(pending.is_some());
        if let Some(PendingTheme::Custom { name, palette }) = pending {
            assert_eq!(name, "MyTheme");
            assert_eq!(palette.background.r, 30);
            assert_eq!(palette.text.r, 200);
        } else {
            panic!("Expected custom theme");
        }
    }

    #[test]
    fn test_field_update_request() {
        // Clear any pending field updates
        let _ = take_pending_field_updates();

        // Request a field update
        request_field_update("count".to_string(), Value::Int(42));

        // Verify update was queued
        let updates = take_pending_field_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].field, "count");
        assert_eq!(updates[0].value, Value::Int(42));

        // Should be cleared after take
        assert!(take_pending_field_updates().is_empty());
    }

    #[test]
    fn test_gui_update_field_method() {
        // Clear any pending field updates
        let _ = take_pending_field_updates();

        // Call the update_field method
        let mut vm = VM::new();
        let result = gui_update_field_method(
            &mut vm,
            "update_field",
            &[Value::string("score"), Value::Int(100)],
        );

        // Should succeed and return Null
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);

        // Update should be queued
        let updates = take_pending_field_updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].field, "score");
        assert_eq!(updates[0].value, Value::Int(100));
    }

    #[test]
    fn test_gui_update_field_method_missing_args() {
        let mut vm = VM::new();

        // Should fail with only one argument
        let result = gui_update_field_method(&mut vm, "update_field", &[Value::string("count")]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gui_method_set_theme_binding() {
        // Verify the binding exists
        let result = gui_method("set_theme", &[Value::string("light")]);
        assert!(result.is_ok());

        // Clear the pending theme
        let _ = take_pending_theme();
    }

    #[test]
    fn test_gui_method_theme_presets_binding() {
        // Verify the binding exists
        let result = gui_method("theme_presets", &[]);
        assert!(result.is_ok());
        if let Value::List(list) = result.unwrap() {
            let borrowed = list.borrow();
            assert!(borrowed.len() >= 20);
        }
    }

    #[test]
    fn test_gui_method_on_change_binding() {
        // Create a text field element first
        let field = gui_method("text_field", &[]).unwrap();
        // Apply on_change callback
        let result = gui_method("on_change", &[field, Value::Int(42)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_method_on_submit_binding() {
        // Create a text field element first
        let field = gui_method("text_field", &[]).unwrap();
        // Apply on_submit callback
        let result = gui_method("on_submit", &[field, Value::Int(55)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_method_on_toggle_binding() {
        // Create a checkbox element first
        let checkbox = gui_method("checkbox", &[Value::string("Test")]).unwrap();
        // Apply on_toggle callback
        let result = gui_method("on_toggle", &[checkbox, Value::Int(77)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_method_on_select_binding() {
        // Create a dropdown element first
        let options = Value::list(vec![Value::string("A"), Value::string("B")]);
        let dropdown = gui_method("dropdown", &[options]).unwrap();
        // Apply on_select callback
        let result = gui_method("on_select", &[dropdown, Value::Int(99)]);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Method Chaining Tests
    // ========================================================================

    #[test]
    fn test_gui_element_method_bold() {
        // Create a text element
        let text = gui_method("text", &[Value::string("Hello")]).unwrap();
        // Call bold() method directly on the element
        let result = gui_element_method(&text, "bold", &[]);
        assert!(result.is_ok());
        if let Value::GuiElement(elem) = result.unwrap() {
            assert_eq!(elem.kind_name(), "Text");
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_element_method_width() {
        // Create a text element
        let text = gui_method("text", &[Value::string("Test")]).unwrap();
        // Call width() method
        let result = gui_element_method(&text, "width", &[Value::Int(200)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_height() {
        // Create a container element
        let container = gui_method("vstack", &[]).unwrap();
        // Call height() method
        let result = gui_element_method(&container, "height", &[Value::Int(100)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_spacing() {
        // Create a vstack element
        let vstack = gui_method("vstack", &[]).unwrap();
        // Call spacing() method
        let result = gui_element_method(&vstack, "spacing", &[Value::Int(16)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_padding() {
        // Create a container element
        let container = gui_method("container", &[]).unwrap();
        // Call padding() method
        let result = gui_element_method(&container, "padding", &[Value::Int(20)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_background() {
        // Create a container element
        let container = gui_method("container", &[]).unwrap();
        // Call background() method with RGBA values
        let result = gui_element_method(&container, "background", &[
            Value::Int(255), Value::Int(0), Value::Int(0), Value::Int(255)
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_chaining_simulation() {
        // Simulate method chaining: text.bold().width(100)
        let text = gui_method("text", &[Value::string("Chained")]).unwrap();

        // First call: bold()
        let bolded = gui_element_method(&text, "bold", &[]).unwrap();

        // Second call: width(100)
        let sized = gui_element_method(&bolded, "width", &[Value::Int(100)]).unwrap();

        // Verify final result is still a GuiElement
        if let Value::GuiElement(elem) = sized {
            assert_eq!(elem.kind_name(), "Text");
        } else {
            panic!("Expected GuiElement after chaining");
        }
    }

    #[test]
    fn test_gui_element_method_unknown() {
        let text = gui_method("text", &[Value::string("Test")]).unwrap();
        let result = gui_element_method(&text, "unknown_method", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("GuiElement has no method"));
    }

    #[test]
    fn test_gui_element_method_chart_title() {
        // Create a bar chart
        let data = Value::list(vec![
            Value::list(vec![Value::string("A"), Value::Int(10)]),
            Value::list(vec![Value::string("B"), Value::Int(20)]),
        ]);
        let chart = gui_method("bar_chart", &[data]).unwrap();
        // Call title() method (alias for chart_title)
        let result = gui_element_method(&chart, "title", &[Value::string("My Chart")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_on_click() {
        // Create an interactive element (not a button - buttons have built-in click handling)
        let interactive = gui_method("interactive", &[]).unwrap();
        // Call on_click() method (alias for on_press)
        let result = gui_element_method(&interactive, "on_click", &[Value::Int(123)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_element_method_set_prefix_compatibility() {
        // Test that set_* names also work for compatibility
        let text = gui_method("text", &[Value::string("Test")]).unwrap();
        let result = gui_element_method(&text, "set_text_bold", &[]);
        assert!(result.is_ok());
    }
}
