//! Native functions for GUI element creation
//!
//! This module provides native functions that can be registered with the Stratum VM
//! to allow creating GUI elements from Stratum code.

use std::sync::Arc;

use stratum_core::bytecode::{NativeFunction, Value};

use crate::callback::CallbackId;
use crate::charts::{BarChartConfig, DataPoint, DataSeries, LineChartConfig, PieChartConfig};
use crate::element::{GuiElement, GuiElementKind, ImageContentFit};
use crate::layout::{HAlign, ScrollDirection, Size, VAlign};

/// Result type for native GUI functions
pub type NativeResult = Result<Value, String>;

/// Create all GUI native functions for registration with the VM
#[must_use]
pub fn gui_native_functions() -> Vec<(&'static str, NativeFunction)> {
    vec![
        // Layout functions
        (
            "gui_vstack",
            NativeFunction::new("gui_vstack", -1, gui_vstack),
        ),
        (
            "gui_hstack",
            NativeFunction::new("gui_hstack", -1, gui_hstack),
        ),
        (
            "gui_zstack",
            NativeFunction::new("gui_zstack", -1, gui_zstack),
        ),
        ("gui_grid", NativeFunction::new("gui_grid", -1, gui_grid)),
        (
            "gui_scroll_view",
            NativeFunction::new("gui_scroll_view", -1, gui_scroll_view),
        ),
        (
            "gui_spacer",
            NativeFunction::new("gui_spacer", -1, gui_spacer),
        ),
        (
            "gui_container",
            NativeFunction::new("gui_container", -1, gui_container),
        ),
        // Widget functions
        ("gui_text", NativeFunction::new("gui_text", -1, gui_text)),
        (
            "gui_button",
            NativeFunction::new("gui_button", -1, gui_button),
        ),
        (
            "gui_text_field",
            NativeFunction::new("gui_text_field", -1, gui_text_field),
        ),
        (
            "gui_checkbox",
            NativeFunction::new("gui_checkbox", -1, gui_checkbox),
        ),
        (
            "gui_radio_button",
            NativeFunction::new("gui_radio_button", -1, gui_radio_button),
        ),
        (
            "gui_dropdown",
            NativeFunction::new("gui_dropdown", -1, gui_dropdown),
        ),
        // Text styling functions
        (
            "gui_set_text_bold",
            NativeFunction::new("gui_set_text_bold", 1, gui_set_text_bold),
        ),
        (
            "gui_set_text_color",
            NativeFunction::new("gui_set_text_color", -1, gui_set_text_color),
        ),
        (
            "gui_set_text_size",
            NativeFunction::new("gui_set_text_size", 2, gui_set_text_size),
        ),
        // Button styling functions
        (
            "gui_set_disabled",
            NativeFunction::new("gui_set_disabled", 2, gui_set_disabled),
        ),
        // TextField styling functions
        (
            "gui_set_placeholder",
            NativeFunction::new("gui_set_placeholder", 2, gui_set_placeholder),
        ),
        (
            "gui_set_secure",
            NativeFunction::new("gui_set_secure", 2, gui_set_secure),
        ),
        (
            "gui_set_value",
            NativeFunction::new("gui_set_value", 2, gui_set_value),
        ),
        (
            "gui_bind_field",
            NativeFunction::new("gui_bind_field", 2, gui_bind_field),
        ),
        // Checkbox styling functions
        (
            "gui_set_checked",
            NativeFunction::new("gui_set_checked", 2, gui_set_checked),
        ),
        (
            "gui_set_checkbox_label",
            NativeFunction::new("gui_set_checkbox_label", 2, gui_set_checkbox_label),
        ),
        // RadioButton styling functions
        (
            "gui_set_radio_value",
            NativeFunction::new("gui_set_radio_value", 2, gui_set_radio_value),
        ),
        (
            "gui_set_radio_selected",
            NativeFunction::new("gui_set_radio_selected", 2, gui_set_radio_selected),
        ),
        (
            "gui_set_radio_label",
            NativeFunction::new("gui_set_radio_label", 2, gui_set_radio_label),
        ),
        // Dropdown styling functions
        (
            "gui_set_dropdown_options",
            NativeFunction::new("gui_set_dropdown_options", 2, gui_set_dropdown_options),
        ),
        (
            "gui_set_dropdown_selected",
            NativeFunction::new("gui_set_dropdown_selected", 2, gui_set_dropdown_selected),
        ),
        (
            "gui_set_dropdown_placeholder",
            NativeFunction::new(
                "gui_set_dropdown_placeholder",
                2,
                gui_set_dropdown_placeholder,
            ),
        ),
        // Slider functions
        (
            "gui_slider",
            NativeFunction::new("gui_slider", -1, gui_slider),
        ),
        (
            "gui_set_slider_value",
            NativeFunction::new("gui_set_slider_value", 2, gui_set_slider_value),
        ),
        (
            "gui_set_slider_range",
            NativeFunction::new("gui_set_slider_range", 3, gui_set_slider_range),
        ),
        (
            "gui_set_slider_step",
            NativeFunction::new("gui_set_slider_step", 2, gui_set_slider_step),
        ),
        // Toggle functions
        (
            "gui_toggle",
            NativeFunction::new("gui_toggle", -1, gui_toggle),
        ),
        (
            "gui_set_toggle_on",
            NativeFunction::new("gui_set_toggle_on", 2, gui_set_toggle_on),
        ),
        (
            "gui_set_toggle_label",
            NativeFunction::new("gui_set_toggle_label", 2, gui_set_toggle_label),
        ),
        // ProgressBar functions
        (
            "gui_progress_bar",
            NativeFunction::new("gui_progress_bar", -1, gui_progress_bar),
        ),
        (
            "gui_set_progress",
            NativeFunction::new("gui_set_progress", 2, gui_set_progress),
        ),
        // Image functions
        ("gui_image", NativeFunction::new("gui_image", -1, gui_image)),
        (
            "gui_set_image_path",
            NativeFunction::new("gui_set_image_path", 2, gui_set_image_path),
        ),
        (
            "gui_set_content_fit",
            NativeFunction::new("gui_set_content_fit", 2, gui_set_content_fit),
        ),
        (
            "gui_set_opacity",
            NativeFunction::new("gui_set_opacity", 2, gui_set_opacity),
        ),
        // Utility functions
        (
            "gui_add_child",
            NativeFunction::new("gui_add_child", 2, gui_add_child),
        ),
        (
            "gui_set_spacing",
            NativeFunction::new("gui_set_spacing", 2, gui_set_spacing),
        ),
        (
            "gui_set_padding",
            NativeFunction::new("gui_set_padding", 2, gui_set_padding),
        ),
        (
            "gui_set_width",
            NativeFunction::new("gui_set_width", 2, gui_set_width),
        ),
        (
            "gui_set_height",
            NativeFunction::new("gui_set_height", 2, gui_set_height),
        ),
        (
            "gui_set_alignment",
            NativeFunction::new("gui_set_alignment", 3, gui_set_alignment),
        ),
        // Conditional and list rendering
        ("gui_if", NativeFunction::new("gui_if", -1, gui_if)),
        (
            "gui_for_each",
            NativeFunction::new("gui_for_each", -1, gui_for_each),
        ),
        // Computed property registration
        (
            "gui_computed",
            NativeFunction::new("gui_computed", 3, gui_computed),
        ),
        // DataTable functions
        (
            "gui_data_table",
            NativeFunction::new("gui_data_table", -1, gui_data_table),
        ),
        (
            "gui_set_table_columns",
            NativeFunction::new("gui_set_table_columns", 2, gui_set_table_columns),
        ),
        (
            "gui_set_page_size",
            NativeFunction::new("gui_set_page_size", 2, gui_set_page_size),
        ),
        (
            "gui_set_current_page",
            NativeFunction::new("gui_set_current_page", 2, gui_set_current_page),
        ),
        (
            "gui_set_sortable",
            NativeFunction::new("gui_set_sortable", 2, gui_set_sortable),
        ),
        (
            "gui_set_sort_by",
            NativeFunction::new("gui_set_sort_by", 3, gui_set_sort_by),
        ),
        (
            "gui_set_selectable",
            NativeFunction::new("gui_set_selectable", 2, gui_set_selectable),
        ),
        (
            "gui_set_selected_rows",
            NativeFunction::new("gui_set_selected_rows", 2, gui_set_selected_rows),
        ),
        (
            "gui_set_column_width",
            NativeFunction::new("gui_set_column_width", 3, gui_set_column_width),
        ),
        (
            "gui_on_sort",
            NativeFunction::new("gui_on_sort", 2, gui_on_sort),
        ),
        (
            "gui_on_page_change",
            NativeFunction::new("gui_on_page_change", 2, gui_on_page_change),
        ),
        (
            "gui_on_selection_change",
            NativeFunction::new("gui_on_selection_change", 2, gui_on_selection_change),
        ),
        (
            "gui_on_row_click",
            NativeFunction::new("gui_on_row_click", 2, gui_on_row_click),
        ),
        (
            "gui_on_cell_click",
            NativeFunction::new("gui_on_cell_click", 2, gui_on_cell_click),
        ),
        // Chart functions
        (
            "gui_bar_chart",
            NativeFunction::new("gui_bar_chart", -1, gui_bar_chart),
        ),
        (
            "gui_line_chart",
            NativeFunction::new("gui_line_chart", -1, gui_line_chart),
        ),
        (
            "gui_pie_chart",
            NativeFunction::new("gui_pie_chart", -1, gui_pie_chart),
        ),
        (
            "gui_set_chart_title",
            NativeFunction::new("gui_set_chart_title", 2, gui_set_chart_title),
        ),
        (
            "gui_set_chart_size",
            NativeFunction::new("gui_set_chart_size", 3, gui_set_chart_size),
        ),
        (
            "gui_set_chart_data",
            NativeFunction::new("gui_set_chart_data", 2, gui_set_chart_data),
        ),
        (
            "gui_set_chart_data_arrays",
            NativeFunction::new("gui_set_chart_data_arrays", 3, gui_set_chart_data_arrays),
        ),
        (
            "gui_add_chart_series",
            NativeFunction::new("gui_add_chart_series", 3, gui_add_chart_series),
        ),
        (
            "gui_set_chart_labels",
            NativeFunction::new("gui_set_chart_labels", 2, gui_set_chart_labels),
        ),
        (
            "gui_set_show_legend",
            NativeFunction::new("gui_set_show_legend", 2, gui_set_show_legend),
        ),
        (
            "gui_set_show_grid",
            NativeFunction::new("gui_set_show_grid", 2, gui_set_show_grid),
        ),
        (
            "gui_set_bar_color",
            NativeFunction::new("gui_set_bar_color", 4, gui_set_bar_color),
        ),
        (
            "gui_set_inner_radius",
            NativeFunction::new("gui_set_inner_radius", 2, gui_set_inner_radius),
        ),
        // OLAP Cube widget functions
        (
            "gui_cube_table",
            NativeFunction::new("gui_cube_table", -1, gui_cube_table),
        ),
        (
            "gui_cube_chart",
            NativeFunction::new("gui_cube_chart", -1, gui_cube_chart),
        ),
        (
            "gui_dimension_filter",
            NativeFunction::new("gui_dimension_filter", -1, gui_dimension_filter),
        ),
        (
            "gui_hierarchy_navigator",
            NativeFunction::new("gui_hierarchy_navigator", -1, gui_hierarchy_navigator),
        ),
        (
            "gui_measure_selector",
            NativeFunction::new("gui_measure_selector", -1, gui_measure_selector),
        ),
        // OLAP Cube widget configuration functions
        (
            "gui_set_cube",
            NativeFunction::new("gui_set_cube", 2, gui_set_cube),
        ),
        (
            "gui_set_row_dimensions",
            NativeFunction::new("gui_set_row_dimensions", 2, gui_set_row_dimensions),
        ),
        (
            "gui_set_measures",
            NativeFunction::new("gui_set_measures", 2, gui_set_measures),
        ),
        (
            "gui_set_cube_chart_type",
            NativeFunction::new("gui_set_cube_chart_type", 2, gui_set_cube_chart_type),
        ),
        (
            "gui_set_x_dimension",
            NativeFunction::new("gui_set_x_dimension", 2, gui_set_x_dimension),
        ),
        (
            "gui_set_y_measure",
            NativeFunction::new("gui_set_y_measure", 2, gui_set_y_measure),
        ),
        (
            "gui_set_series_dimension",
            NativeFunction::new("gui_set_series_dimension", 2, gui_set_series_dimension),
        ),
        (
            "gui_set_filter_dimension",
            NativeFunction::new("gui_set_filter_dimension", 2, gui_set_filter_dimension),
        ),
        (
            "gui_set_hierarchy",
            NativeFunction::new("gui_set_hierarchy", 2, gui_set_hierarchy),
        ),
        (
            "gui_set_current_level",
            NativeFunction::new("gui_set_current_level", 2, gui_set_current_level),
        ),
        (
            "gui_on_drill",
            NativeFunction::new("gui_on_drill", 2, gui_on_drill),
        ),
        (
            "gui_on_roll_up",
            NativeFunction::new("gui_on_roll_up", 2, gui_on_roll_up),
        ),
        (
            "gui_on_level_change",
            NativeFunction::new("gui_on_level_change", 2, gui_on_level_change),
        ),
        // Widget styling functions
        (
            "gui_set_background",
            NativeFunction::new("gui_set_background", -1, gui_set_background),
        ),
        (
            "gui_set_foreground",
            NativeFunction::new("gui_set_foreground", -1, gui_set_foreground),
        ),
        (
            "gui_set_border_color",
            NativeFunction::new("gui_set_border_color", -1, gui_set_border_color),
        ),
        (
            "gui_set_border_width",
            NativeFunction::new("gui_set_border_width", 2, gui_set_border_width),
        ),
        (
            "gui_set_corner_radius",
            NativeFunction::new("gui_set_corner_radius", 2, gui_set_corner_radius),
        ),
        // Theme functions
        (
            "gui_theme_presets",
            NativeFunction::new("gui_theme_presets", 0, gui_theme_presets),
        ),
        (
            "gui_set_theme",
            NativeFunction::new("gui_set_theme", 1, gui_set_theme),
        ),
        (
            "gui_custom_theme",
            NativeFunction::new("gui_custom_theme", 2, gui_custom_theme),
        ),
        // Interactive element functions
        (
            "gui_interactive",
            NativeFunction::new("gui_interactive", -1, gui_interactive),
        ),
        (
            "gui_on_press",
            NativeFunction::new("gui_on_press", 2, gui_on_press),
        ),
        (
            "gui_on_mouse_release",
            NativeFunction::new("gui_on_mouse_release", 2, gui_on_mouse_release),
        ),
        (
            "gui_on_double_click",
            NativeFunction::new("gui_on_double_click", 2, gui_on_double_click),
        ),
        (
            "gui_on_right_press",
            NativeFunction::new("gui_on_right_press", 2, gui_on_right_press),
        ),
        (
            "gui_on_right_release",
            NativeFunction::new("gui_on_right_release", 2, gui_on_right_release),
        ),
        (
            "gui_on_hover_enter",
            NativeFunction::new("gui_on_hover_enter", 2, gui_on_hover_enter),
        ),
        (
            "gui_on_hover_exit",
            NativeFunction::new("gui_on_hover_exit", 2, gui_on_hover_exit),
        ),
        (
            "gui_on_mouse_move",
            NativeFunction::new("gui_on_mouse_move", 2, gui_on_mouse_move),
        ),
        (
            "gui_on_mouse_scroll",
            NativeFunction::new("gui_on_mouse_scroll", 2, gui_on_mouse_scroll),
        ),
        (
            "gui_set_cursor",
            NativeFunction::new("gui_set_cursor", 2, gui_set_cursor),
        ),
        // Form element event handlers
        (
            "gui_on_change",
            NativeFunction::new("gui_on_change", 2, gui_on_change),
        ),
        (
            "gui_on_submit",
            NativeFunction::new("gui_on_submit", 2, gui_on_submit),
        ),
        (
            "gui_on_toggle",
            NativeFunction::new("gui_on_toggle", 2, gui_on_toggle),
        ),
        (
            "gui_on_select",
            NativeFunction::new("gui_on_select", 2, gui_on_select),
        ),
    ]
}

// Helper to clone GuiElement from Value
fn clone_gui_element(value: &Value) -> Result<GuiElement, String> {
    match value {
        Value::GuiElement(e) => {
            // Use as_any to downcast to GuiElement
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                Ok(elem.clone())
            } else {
                Err("failed to downcast to GuiElement".to_string())
            }
        }
        _ => Err(format!("expected GuiElement, got {}", value.type_name())),
    }
}

// Helper to extract optional float argument
fn get_opt_float(args: &[Value], index: usize) -> Option<f64> {
    args.get(index).and_then(|v| match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    })
}

// Helper to extract required float argument
fn get_float(args: &[Value], index: usize, name: &str) -> Result<f64, String> {
    match args.get(index) {
        Some(Value::Float(f)) => Ok(*f),
        Some(Value::Int(i)) => Ok(*i as f64),
        Some(v) => Err(format!("{} must be a number, got {}", name, v.type_name())),
        None => Err(format!("missing required argument: {}", name)),
    }
}

// Helper to extract required int argument
fn get_int(args: &[Value], index: usize, name: &str) -> Result<i64, String> {
    match args.get(index) {
        Some(Value::Int(i)) => Ok(*i),
        Some(v) => Err(format!(
            "{} must be an integer, got {}",
            name,
            v.type_name()
        )),
        None => Err(format!("missing required argument: {}", name)),
    }
}

// Helper to extract a CallbackId from a Value
fn get_callback_id(value: &Value) -> Result<CallbackId, String> {
    match value {
        Value::Int(i) => {
            if *i < 0 {
                Err("callback_id must be a non-negative integer".to_string())
            } else {
                Ok(CallbackId::new(*i as u64))
            }
        }
        _ => Err(format!(
            "callback_id must be an integer, got {}",
            value.type_name()
        )),
    }
}

// Helper to extract required string argument
fn get_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.to_string()),
        Some(v) => Err(format!("{} must be a string, got {}", name, v.type_name())),
        None => Err(format!("missing required argument: {}", name)),
    }
}

// Helper to extract StateBinding path if present
fn get_state_binding_path(value: &Value) -> Option<String> {
    match value {
        Value::StateBinding(path) => Some(path.clone()),
        _ => None,
    }
}

// Helper to collect child elements from a list
fn collect_children(value: &Value) -> Result<Vec<GuiElement>, String> {
    match value {
        Value::List(list) => {
            let list = list.borrow();
            let mut children = Vec::with_capacity(list.len());
            for item in list.iter() {
                children.push(clone_gui_element(item)?);
            }
            Ok(children)
        }
        _ => Err(format!(
            "children must be a list, got {}",
            value.type_name()
        )),
    }
}

/// Create a VStack element
/// gui_vstack() or gui_vstack(spacing) or gui_vstack(spacing, children)
fn gui_vstack(args: &[Value]) -> NativeResult {
    let spacing = get_opt_float(args, 0).unwrap_or(0.0) as f32;

    let mut element = GuiElement::vstack_with_spacing(spacing).build();

    // If second arg is a list, use it as children
    if let Some(children_val) = args.get(1) {
        let children = collect_children(children_val)?;
        element.children = children.into_iter().map(Arc::new).collect();
    }

    Ok(element.into_value())
}

/// Create an HStack element
/// gui_hstack() or gui_hstack(spacing) or gui_hstack(spacing, children)
fn gui_hstack(args: &[Value]) -> NativeResult {
    let spacing = get_opt_float(args, 0).unwrap_or(0.0) as f32;

    let mut element = GuiElement::hstack_with_spacing(spacing).build();

    // If second arg is a list, use it as children
    if let Some(children_val) = args.get(1) {
        let children = collect_children(children_val)?;
        element.children = children.into_iter().map(Arc::new).collect();
    }

    Ok(element.into_value())
}

/// Create a ZStack element
/// gui_zstack() or gui_zstack(children)
fn gui_zstack(args: &[Value]) -> NativeResult {
    let mut element = GuiElement::zstack().build();

    // If first arg is a list, use it as children
    if let Some(children_val) = args.first() {
        let children = collect_children(children_val)?;
        element.children = children.into_iter().map(Arc::new).collect();
    }

    Ok(element.into_value())
}

/// Create a Grid element
/// gui_grid(columns) or gui_grid(columns, children) or gui_grid(columns, spacing, children)
fn gui_grid(args: &[Value]) -> NativeResult {
    let columns = get_int(args, 0, "columns")? as usize;

    let mut element = GuiElement::grid(columns).build();

    // Check for optional spacing and children
    match args.len() {
        2 => {
            // Second arg is children
            let children = collect_children(&args[1])?;
            element.children = children.into_iter().map(Arc::new).collect();
        }
        3.. => {
            // Second arg is spacing, third is children
            if let GuiElementKind::Grid(ref mut config) = element.kind {
                config.spacing = get_float(args, 1, "spacing")? as f32;
            }
            let children = collect_children(&args[2])?;
            element.children = children.into_iter().map(Arc::new).collect();
        }
        _ => {}
    }

    Ok(element.into_value())
}

/// Create a ScrollView element
/// gui_scroll_view() or gui_scroll_view(direction) or gui_scroll_view(direction, child)
/// direction: "vertical", "horizontal", or "both"
fn gui_scroll_view(args: &[Value]) -> NativeResult {
    let direction = if let Some(Value::String(s)) = args.first() {
        match s.as_str() {
            "vertical" => ScrollDirection::Vertical,
            "horizontal" => ScrollDirection::Horizontal,
            "both" => ScrollDirection::Both,
            _ => return Err(format!("invalid scroll direction: {}", s)),
        }
    } else {
        ScrollDirection::Vertical
    };

    let mut element = GuiElement::scroll_view()
        .scroll_direction(direction)
        .build();

    // If second arg exists, use it as the single child
    if let Some(child_val) = args.get(1) {
        let child = clone_gui_element(child_val)?;
        element.children = vec![Arc::new(child)];
    }

    Ok(element.into_value())
}

/// Create a Spacer element
/// gui_spacer() or gui_spacer(width, height) or gui_spacer("horizontal") or gui_spacer("vertical")
fn gui_spacer(args: &[Value]) -> NativeResult {
    let element = match args.first() {
        Some(Value::String(s)) => match s.as_str() {
            "horizontal" => GuiElement::horizontal_spacer().build(),
            "vertical" => GuiElement::vertical_spacer().build(),
            _ => return Err(format!("invalid spacer type: {}", s)),
        },
        Some(Value::Float(_)) | Some(Value::Int(_)) => {
            let width = get_float(args, 0, "width")? as f32;
            let height = get_opt_float(args, 1).unwrap_or(width as f64) as f32;
            let mut elem = GuiElement::spacer().build();
            if let GuiElementKind::Spacer(ref mut config) = elem.kind {
                config.width = Some(Size::Fixed(width));
                config.height = Some(Size::Fixed(height));
            }
            elem
        }
        _ => GuiElement::spacer().build(),
    };

    Ok(element.into_value())
}

/// Create a Container element
/// gui_container() or gui_container(child) or gui_container(child, center)
fn gui_container(args: &[Value]) -> NativeResult {
    let mut builder = GuiElement::container();

    // Check for center flag
    if let Some(Value::Bool(true)) = args.get(1) {
        builder = builder.center();
    }

    let mut element = builder.build();

    // If first arg is a GuiElement, use it as child
    if let Some(child_val) = args.first() {
        if matches!(child_val, Value::GuiElement(_)) {
            let child = clone_gui_element(child_val)?;
            element.children = vec![Arc::new(child)];
        }
    }

    Ok(element.into_value())
}

/// Create a Text element
/// gui_text(content) or gui_text(content, size)
fn gui_text(args: &[Value]) -> NativeResult {
    let content = get_string(args, 0, "content")?;

    let mut builder = GuiElement::text(&content);

    if let Some(size) = get_opt_float(args, 1) {
        builder = builder.text_size(size as f32);
    }

    Ok(builder.build().into_value())
}

/// Create a Button element
/// gui_button(label) or gui_button(label, callback_id) or gui_button(label, callback_id, disabled)
fn gui_button(args: &[Value]) -> NativeResult {
    let label = get_string(args, 0, "label")?;

    let mut builder = GuiElement::button(&label);

    // If second arg is an int, use it as callback ID
    if let Some(Value::Int(id)) = args.get(1) {
        builder = builder.on_click(CallbackId::new(*id as u64));
    }

    // If third arg is a bool, use it as disabled state
    if let Some(Value::Bool(disabled)) = args.get(2) {
        builder = builder.disabled(*disabled);
    }

    Ok(builder.build().into_value())
}

/// Create a TextField element
/// gui_text_field() or gui_text_field(value) or gui_text_field(value, placeholder)
/// gui_text_field(&state.field) - with state binding for two-way binding
fn gui_text_field(args: &[Value]) -> NativeResult {
    let mut builder = GuiElement::text_field();

    // First arg can be initial value (String) or state binding (StateBinding)
    if let Some(arg) = args.first() {
        if let Some(path) = get_state_binding_path(arg) {
            // State binding: enable two-way binding to this field path
            builder = builder.bind_field(&path);
        } else if let Value::String(s) = arg {
            builder = builder.value(s.as_str());
        }
    }

    // Second arg is placeholder
    if let Some(Value::String(s)) = args.get(1) {
        builder = builder.placeholder(s.as_str());
    }

    // Third arg is secure mode (bool)
    if let Some(Value::Bool(secure)) = args.get(2) {
        builder = builder.secure(*secure);
    }

    Ok(builder.build().into_value())
}

/// Create a Checkbox element
/// gui_checkbox(label) or gui_checkbox(label, checked) or gui_checkbox(label, checked, callback_id)
/// gui_checkbox(label, &state.field) - with state binding for two-way binding
fn gui_checkbox(args: &[Value]) -> NativeResult {
    let label = get_string(args, 0, "label")?;

    let mut builder = GuiElement::checkbox(&label);

    // Second arg can be checked state (bool) or state binding (StateBinding)
    if let Some(arg) = args.get(1) {
        if let Some(path) = get_state_binding_path(arg) {
            // State binding: enable two-way binding to this field path
            builder = builder.bind_field(&path);
        } else if let Value::Bool(checked) = arg {
            builder = builder.checked(*checked);
        }
    }

    // Third arg is callback ID for on_toggle
    if let Some(Value::Int(id)) = args.get(2) {
        builder = builder.on_toggle(CallbackId::new(*id as u64));
    }

    Ok(builder.build().into_value())
}

/// Set checked state on a Checkbox element
/// gui_set_checked(element, checked) -> new_element
fn gui_set_checked(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_checked requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let checked = match &args[1] {
        Value::Bool(b) => *b,
        v => return Err(format!("checked must be a boolean, got {}", v.type_name())),
    };

    if let GuiElementKind::Checkbox(ref mut config) = element.kind {
        config.checked = checked;
    } else {
        return Err("gui_set_checked can only be applied to Checkbox elements".to_string());
    }

    Ok(element.into_value())
}

/// Set label on a Checkbox element
/// gui_set_checkbox_label(element, label) -> new_element
fn gui_set_checkbox_label(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_checkbox_label requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let label = get_string(args, 1, "label")?;

    if let GuiElementKind::Checkbox(ref mut config) = element.kind {
        config.label = label;
    } else {
        return Err("gui_set_checkbox_label can only be applied to Checkbox elements".to_string());
    }

    Ok(element.into_value())
}

/// Create a RadioButton element
/// gui_radio_button(label, value) or gui_radio_button(label, value, selected_value)
/// or gui_radio_button(label, value, selected_value, callback_id)
/// gui_radio_button(label, value, &state.field) - with state binding for two-way binding
fn gui_radio_button(args: &[Value]) -> NativeResult {
    let label = get_string(args, 0, "label")?;
    let value = get_string(args, 1, "value")?;

    let mut builder = GuiElement::radio_button(&label, &value);

    // Third arg can be selected_value (string) or state binding (StateBinding)
    if let Some(arg) = args.get(2) {
        if let Some(path) = get_state_binding_path(arg) {
            // State binding: enable two-way binding to this field path
            builder = builder.bind_field(&path);
        } else if let Value::String(s) = arg {
            builder = builder.selected_value(s.as_str());
        }
    }

    // Fourth arg is callback ID for on_select
    if let Some(Value::Int(id)) = args.get(3) {
        builder = builder.on_select(CallbackId::new(*id as u64));
    }

    Ok(builder.build().into_value())
}

/// Set the value this radio button represents
/// gui_set_radio_value(element, value) -> new_element
fn gui_set_radio_value(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_radio_value requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let value = get_string(args, 1, "value")?;

    if let GuiElementKind::RadioButton(ref mut config) = element.kind {
        config.value = value;
    } else {
        return Err("gui_set_radio_value can only be applied to RadioButton elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the currently selected value for comparison
/// gui_set_radio_selected(element, selected_value) -> new_element
fn gui_set_radio_selected(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_radio_selected requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let selected = get_string(args, 1, "selected_value")?;

    if let GuiElementKind::RadioButton(ref mut config) = element.kind {
        config.selected_value = Some(selected);
    } else {
        return Err(
            "gui_set_radio_selected can only be applied to RadioButton elements".to_string(),
        );
    }

    Ok(element.into_value())
}

/// Set label on a RadioButton element
/// gui_set_radio_label(element, label) -> new_element
fn gui_set_radio_label(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_radio_label requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let label = get_string(args, 1, "label")?;

    if let GuiElementKind::RadioButton(ref mut config) = element.kind {
        config.label = label;
    } else {
        return Err("gui_set_radio_label can only be applied to RadioButton elements".to_string());
    }

    Ok(element.into_value())
}

/// Create a Dropdown element
/// gui_dropdown(options_list) or gui_dropdown(options_list, selected)
/// or gui_dropdown(options_list, selected, placeholder) or gui_dropdown(options_list, selected, placeholder, callback_id)
/// gui_dropdown(options_list, &state.field) - with state binding for two-way binding
fn gui_dropdown(args: &[Value]) -> NativeResult {
    // First arg must be a list of strings
    let options = match args.first() {
        Some(Value::List(list)) => {
            let list = list.borrow();
            let mut opts = Vec::with_capacity(list.len());
            for item in list.iter() {
                match item {
                    Value::String(s) => opts.push(s.to_string()),
                    v => {
                        return Err(format!(
                            "dropdown options must be strings, got {}",
                            v.type_name()
                        ))
                    }
                }
            }
            opts
        }
        Some(v) => {
            return Err(format!(
                "first argument must be a list of options, got {}",
                v.type_name()
            ))
        }
        None => return Err("gui_dropdown requires at least 1 argument (options list)".to_string()),
    };

    let mut builder = GuiElement::dropdown(options);

    // Second arg can be selected value (string) or state binding (StateBinding)
    if let Some(arg) = args.get(1) {
        if let Some(path) = get_state_binding_path(arg) {
            // State binding: enable two-way binding to this field path
            builder = builder.bind_field(&path);
        } else if let Value::String(s) = arg {
            builder = builder.selected(s.as_str());
        }
    }

    // Third arg is placeholder (string)
    if let Some(Value::String(s)) = args.get(2) {
        builder = builder.dropdown_placeholder(s.as_str());
    }

    // Fourth arg is callback ID for on_select
    if let Some(Value::Int(id)) = args.get(3) {
        builder = builder.on_select(CallbackId::new(*id as u64));
    }

    Ok(builder.build().into_value())
}

/// Set options on a Dropdown element
/// gui_set_dropdown_options(element, options_list) -> new_element
fn gui_set_dropdown_options(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_dropdown_options requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    let options = match &args[1] {
        Value::List(list) => {
            let list = list.borrow();
            let mut opts = Vec::with_capacity(list.len());
            for item in list.iter() {
                match item {
                    Value::String(s) => opts.push(s.to_string()),
                    v => {
                        return Err(format!(
                            "dropdown options must be strings, got {}",
                            v.type_name()
                        ))
                    }
                }
            }
            opts
        }
        v => return Err(format!("options must be a list, got {}", v.type_name())),
    };

    if let GuiElementKind::Dropdown(ref mut config) = element.kind {
        config.options = options;
    } else {
        return Err(
            "gui_set_dropdown_options can only be applied to Dropdown elements".to_string(),
        );
    }

    Ok(element.into_value())
}

/// Set selected value on a Dropdown element
/// gui_set_dropdown_selected(element, selected) -> new_element
fn gui_set_dropdown_selected(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_dropdown_selected requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    let selected = match &args[1] {
        Value::String(s) => Some(s.to_string()),
        Value::Null => None,
        v => {
            return Err(format!(
                "selected must be a string or null, got {}",
                v.type_name()
            ))
        }
    };

    if let GuiElementKind::Dropdown(ref mut config) = element.kind {
        config.selected = selected;
    } else {
        return Err(
            "gui_set_dropdown_selected can only be applied to Dropdown elements".to_string(),
        );
    }

    Ok(element.into_value())
}

/// Set placeholder on a Dropdown element
/// gui_set_dropdown_placeholder(element, placeholder) -> new_element
fn gui_set_dropdown_placeholder(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_dropdown_placeholder requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let placeholder = get_string(args, 1, "placeholder")?;

    if let GuiElementKind::Dropdown(ref mut config) = element.kind {
        config.placeholder = Some(placeholder);
    } else {
        return Err(
            "gui_set_dropdown_placeholder can only be applied to Dropdown elements".to_string(),
        );
    }

    Ok(element.into_value())
}

/// Set placeholder text on a TextField element
/// gui_set_placeholder(element, placeholder) -> new_element
fn gui_set_placeholder(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_placeholder requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let placeholder = get_string(args, 1, "placeholder")?;

    if let GuiElementKind::TextField(ref mut config) = element.kind {
        config.placeholder = placeholder;
    } else {
        return Err("gui_set_placeholder can only be applied to TextField elements".to_string());
    }

    Ok(element.into_value())
}

/// Set secure mode on a TextField element (for passwords)
/// gui_set_secure(element, secure) -> new_element
fn gui_set_secure(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_secure requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let secure = match &args[1] {
        Value::Bool(b) => *b,
        v => return Err(format!("secure must be a boolean, got {}", v.type_name())),
    };

    if let GuiElementKind::TextField(ref mut config) = element.kind {
        config.secure = secure;
    } else {
        return Err("gui_set_secure can only be applied to TextField elements".to_string());
    }

    Ok(element.into_value())
}

/// Set value on a TextField element
/// gui_set_value(element, value) -> new_element
fn gui_set_value(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_value requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let value = get_string(args, 1, "value")?;

    if let GuiElementKind::TextField(ref mut config) = element.kind {
        config.value = value;
    } else {
        return Err("gui_set_value can only be applied to TextField elements".to_string());
    }

    Ok(element.into_value())
}

/// Bind a TextField, Checkbox, RadioButton, or Dropdown to a state field path for automatic updates
/// gui_bind_field(element, field_path) -> new_element
fn gui_bind_field(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_bind_field requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let field_path = get_string(args, 1, "field_path")?;

    match &mut element.kind {
        GuiElementKind::TextField(config) => {
            config.field_path = Some(field_path);
        }
        GuiElementKind::Checkbox(config) => {
            config.field_path = Some(field_path);
        }
        GuiElementKind::RadioButton(config) => {
            config.field_path = Some(field_path);
        }
        GuiElementKind::Dropdown(config) => {
            config.field_path = Some(field_path);
        }
        GuiElementKind::Slider(config) => {
            config.field_path = Some(field_path);
        }
        GuiElementKind::Toggle(config) => {
            config.field_path = Some(field_path);
        }
        _ => {
            return Err(
                "gui_bind_field can only be applied to TextField, Checkbox, RadioButton, Dropdown, Slider, or Toggle elements".to_string(),
            );
        }
    }

    Ok(element.into_value())
}

/// Add a child to a parent element
/// gui_add_child(parent, child) -> new_parent
fn gui_add_child(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_add_child requires 2 arguments".to_string());
    }

    let mut parent = clone_gui_element(&args[0])?;
    let child = clone_gui_element(&args[1])?;

    parent.children.push(Arc::new(child));

    Ok(parent.into_value())
}

/// Set spacing on a layout element
/// gui_set_spacing(element, spacing) -> new_element
fn gui_set_spacing(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_spacing requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let spacing = get_float(args, 1, "spacing")? as f32;

    match &mut element.kind {
        GuiElementKind::VStack(c) => c.spacing = spacing,
        GuiElementKind::HStack(c) => c.spacing = spacing,
        GuiElementKind::Grid(c) => c.spacing = spacing,
        _ => return Err("spacing can only be set on VStack, HStack, or Grid".to_string()),
    }

    Ok(element.into_value())
}

/// Set padding on an element
/// gui_set_padding(element, padding) -> new_element
fn gui_set_padding(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_padding requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let padding = get_float(args, 1, "padding")? as f32;

    element.style.padding = Some(padding);

    Ok(element.into_value())
}

/// Set width on an element
/// gui_set_width(element, width) -> new_element
/// width can be: "fill", "shrink", or a number for fixed width
fn gui_set_width(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_width requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    let size = match &args[1] {
        Value::String(s) => match s.as_str() {
            "fill" => Size::Fill,
            "shrink" => Size::Shrink,
            _ => return Err(format!("invalid width: {}", s)),
        },
        Value::Float(f) => Size::Fixed(*f as f32),
        Value::Int(i) => Size::Fixed(*i as f32),
        v => {
            return Err(format!(
                "width must be a string or number, got {}",
                v.type_name()
            ))
        }
    };

    element.style.width = Some(size);

    Ok(element.into_value())
}

/// Set height on an element
/// gui_set_height(element, height) -> new_element
/// height can be: "fill", "shrink", or a number for fixed height
fn gui_set_height(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_height requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    let size = match &args[1] {
        Value::String(s) => match s.as_str() {
            "fill" => Size::Fill,
            "shrink" => Size::Shrink,
            _ => return Err(format!("invalid height: {}", s)),
        },
        Value::Float(f) => Size::Fixed(*f as f32),
        Value::Int(i) => Size::Fixed(*i as f32),
        v => {
            return Err(format!(
                "height must be a string or number, got {}",
                v.type_name()
            ))
        }
    };

    element.style.height = Some(size);

    Ok(element.into_value())
}

/// Parse a string value into HAlign
fn parse_h_align(value: &Value) -> Result<HAlign, String> {
    match value {
        Value::String(s) => match s.to_lowercase().as_str() {
            "start" | "left" => Ok(HAlign::Start),
            "center" => Ok(HAlign::Center),
            "end" | "right" => Ok(HAlign::End),
            other => Err(format!(
                "invalid horizontal alignment '{}', expected: start, center, end, left, or right",
                other
            )),
        },
        v => Err(format!("alignment must be a string, got {}", v.type_name())),
    }
}

/// Parse a string value into VAlign
fn parse_v_align(value: &Value) -> Result<VAlign, String> {
    match value {
        Value::String(s) => match s.to_lowercase().as_str() {
            "top" | "start" => Ok(VAlign::Top),
            "center" => Ok(VAlign::Center),
            "bottom" | "end" => Ok(VAlign::Bottom),
            other => Err(format!(
                "invalid vertical alignment '{}', expected: top, center, bottom, start, or end",
                other
            )),
        },
        v => Err(format!("alignment must be a string, got {}", v.type_name())),
    }
}

/// Set alignment on layout elements
/// gui_set_alignment(element, h_align, v_align) -> new_element
///
/// Supports: VStack (h_align only), HStack (v_align only), Grid (both), Container (both)
fn gui_set_alignment(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(
            "gui_set_alignment requires 3 arguments: element, h_align, v_align".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let h_align = parse_h_align(&args[1])?;
    let v_align = parse_v_align(&args[2])?;

    match &mut element.kind {
        GuiElementKind::VStack(c) => {
            // VStack aligns children horizontally
            c.align = h_align;
        }
        GuiElementKind::HStack(c) => {
            // HStack aligns children vertically
            c.align = v_align;
        }
        GuiElementKind::Grid(c) => {
            c.cell_align_x = h_align;
            c.cell_align_y = v_align;
        }
        GuiElementKind::Container(c) => {
            c.align_x = h_align;
            c.align_y = v_align;
        }
        _ => {
            return Err(
                "alignment can only be set on VStack, HStack, Grid, or Container".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set bold on a text element
/// gui_set_text_bold(element) -> new_element
fn gui_set_text_bold(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("gui_set_text_bold requires 1 argument".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    if let GuiElementKind::Text(ref mut config) = element.kind {
        config.bold = true;
    } else {
        return Err("gui_set_text_bold can only be applied to Text elements".to_string());
    }

    Ok(element.into_value())
}

/// Set color on a text element
/// gui_set_text_color(element, r, g, b) or gui_set_text_color(element, r, g, b, a)
fn gui_set_text_color(args: &[Value]) -> NativeResult {
    if args.len() < 4 {
        return Err(
            "gui_set_text_color requires at least 4 arguments (element, r, g, b)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let r = get_int(args, 1, "r")? as u8;
    let g = get_int(args, 2, "g")? as u8;
    let b = get_int(args, 3, "b")? as u8;
    let a = args
        .get(4)
        .map(|v| match v {
            Value::Int(i) => Ok(*i as u8),
            _ => Err("alpha must be an integer".to_string()),
        })
        .transpose()?
        .unwrap_or(255);

    if let GuiElementKind::Text(ref mut config) = element.kind {
        config.color = Some((r, g, b, a));
    } else {
        return Err("gui_set_text_color can only be applied to Text elements".to_string());
    }

    Ok(element.into_value())
}

/// Set font size on a text element
/// gui_set_text_size(element, size) -> new_element
fn gui_set_text_size(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_text_size requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let size = get_float(args, 1, "size")? as f32;

    if let GuiElementKind::Text(ref mut config) = element.kind {
        config.size = Some(size);
    } else {
        return Err("gui_set_text_size can only be applied to Text elements".to_string());
    }

    Ok(element.into_value())
}

/// Set disabled state on a button element
/// gui_set_disabled(element, disabled) -> new_element
fn gui_set_disabled(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_disabled requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let disabled = match &args[1] {
        Value::Bool(b) => *b,
        v => return Err(format!("disabled must be a boolean, got {}", v.type_name())),
    };

    if let GuiElementKind::Button(ref mut config) = element.kind {
        config.disabled = disabled;
    } else {
        return Err("gui_set_disabled can only be applied to Button elements".to_string());
    }

    Ok(element.into_value())
}

// ==================== Slider Native Functions ====================

/// Create a Slider element
/// gui_slider(min, max) or gui_slider(min, max, value) or gui_slider(min, max, value, step)
/// gui_slider(min, max, &state.field) - with state binding for two-way binding
fn gui_slider(args: &[Value]) -> NativeResult {
    let min = get_float(args, 0, "min")?;
    let max = get_float(args, 1, "max")?;

    let mut builder = GuiElement::slider(min, max);

    // Third arg can be initial value (number) or state binding (StateBinding)
    if let Some(arg) = args.get(2) {
        if let Some(path) = get_state_binding_path(arg) {
            // State binding: enable two-way binding to this field path
            builder = builder.bind_field(&path);
        } else if let Some(value) = get_opt_float(args, 2) {
            builder = builder.slider_value(value);
        }
    }

    // Fourth arg is step
    if let Some(step) = get_opt_float(args, 3) {
        builder = builder.slider_step(step);
    }

    Ok(builder.build().into_value())
}

/// Set slider value
/// gui_set_slider_value(element, value) -> new_element
fn gui_set_slider_value(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_slider_value requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let value = get_float(args, 1, "value")?;

    if let GuiElementKind::Slider(ref mut config) = element.kind {
        config.value = value.clamp(config.min, config.max);
    } else {
        return Err("gui_set_slider_value can only be applied to Slider elements".to_string());
    }

    Ok(element.into_value())
}

/// Set slider range
/// gui_set_slider_range(element, min, max) -> new_element
fn gui_set_slider_range(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err("gui_set_slider_range requires 3 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let min = get_float(args, 1, "min")?;
    let max = get_float(args, 2, "max")?;

    if let GuiElementKind::Slider(ref mut config) = element.kind {
        config.min = min;
        config.max = max;
        config.value = config.value.clamp(min, max);
    } else {
        return Err("gui_set_slider_range can only be applied to Slider elements".to_string());
    }

    Ok(element.into_value())
}

/// Set slider step
/// gui_set_slider_step(element, step) -> new_element
fn gui_set_slider_step(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_slider_step requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let step = get_float(args, 1, "step")?;

    if let GuiElementKind::Slider(ref mut config) = element.kind {
        config.step = step;
    } else {
        return Err("gui_set_slider_step can only be applied to Slider elements".to_string());
    }

    Ok(element.into_value())
}

// ==================== Toggle Native Functions ====================

/// Create a Toggle element
/// gui_toggle(label) or gui_toggle(label, is_on) or gui_toggle(label, is_on, callback_id)
/// gui_toggle(label, &state.field) - with state binding for two-way binding
fn gui_toggle(args: &[Value]) -> NativeResult {
    let label = get_string(args, 0, "label")?;

    let mut builder = GuiElement::toggle(&label);

    // Second arg can be is_on state (bool) or state binding (StateBinding)
    if let Some(arg) = args.get(1) {
        if let Some(path) = get_state_binding_path(arg) {
            // State binding: enable two-way binding to this field path
            builder = builder.bind_field(&path);
        } else if let Value::Bool(is_on) = arg {
            builder = builder.is_on(*is_on);
        }
    }

    // Third arg is callback ID for on_toggle
    if let Some(Value::Int(id)) = args.get(2) {
        builder = builder.on_toggle(CallbackId::new(*id as u64));
    }

    Ok(builder.build().into_value())
}

/// Set toggle on state
/// gui_set_toggle_on(element, is_on) -> new_element
fn gui_set_toggle_on(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_toggle_on requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let is_on = match &args[1] {
        Value::Bool(b) => *b,
        v => return Err(format!("is_on must be a boolean, got {}", v.type_name())),
    };

    if let GuiElementKind::Toggle(ref mut config) = element.kind {
        config.is_on = is_on;
    } else {
        return Err("gui_set_toggle_on can only be applied to Toggle elements".to_string());
    }

    Ok(element.into_value())
}

/// Set toggle label
/// gui_set_toggle_label(element, label) -> new_element
fn gui_set_toggle_label(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_toggle_label requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let label = get_string(args, 1, "label")?;

    if let GuiElementKind::Toggle(ref mut config) = element.kind {
        config.label = label;
    } else {
        return Err("gui_set_toggle_label can only be applied to Toggle elements".to_string());
    }

    Ok(element.into_value())
}

// ==================== ProgressBar Native Functions ====================

/// Create a ProgressBar element
/// gui_progress_bar(value) - value should be 0.0 to 1.0
fn gui_progress_bar(args: &[Value]) -> NativeResult {
    let value = get_opt_float(args, 0).unwrap_or(0.0) as f32;

    let element = GuiElement::progress_bar(value.clamp(0.0, 1.0)).build();

    Ok(element.into_value())
}

/// Set progress value
/// gui_set_progress(element, value) -> new_element
fn gui_set_progress(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_progress requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let value = get_float(args, 1, "value")? as f32;

    if let GuiElementKind::ProgressBar(ref mut config) = element.kind {
        config.value = value.clamp(0.0, 1.0);
    } else {
        return Err("gui_set_progress can only be applied to ProgressBar elements".to_string());
    }

    Ok(element.into_value())
}

// ==================== Image Native Functions ====================

/// Create an Image element
/// gui_image(path) or gui_image(path, content_fit)
/// content_fit can be: "contain", "cover", "fill", "none", "scale-down"
fn gui_image(args: &[Value]) -> NativeResult {
    let path = get_string(args, 0, "path")?;

    let mut builder = GuiElement::image(&path);

    // Second arg is content fit
    if let Some(Value::String(s)) = args.get(1) {
        let fit = match s.as_str() {
            "contain" => ImageContentFit::Contain,
            "cover" => ImageContentFit::Cover,
            "fill" => ImageContentFit::Fill,
            "none" => ImageContentFit::None,
            "scale-down" => ImageContentFit::ScaleDown,
            _ => return Err(format!("invalid content_fit: {}", s)),
        };
        builder = builder.content_fit(fit);
    }

    Ok(builder.build().into_value())
}

/// Set image path
/// gui_set_image_path(element, path) -> new_element
fn gui_set_image_path(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_image_path requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let path = get_string(args, 1, "path")?;

    if let GuiElementKind::Image(ref mut config) = element.kind {
        config.path = Some(path);
    } else {
        return Err("gui_set_image_path can only be applied to Image elements".to_string());
    }

    Ok(element.into_value())
}

/// Set image content fit
/// gui_set_content_fit(element, fit) -> new_element
/// fit can be: "contain", "cover", "fill", "none", "scale-down"
fn gui_set_content_fit(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_content_fit requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let fit_str = get_string(args, 1, "fit")?;

    let fit = match fit_str.as_str() {
        "contain" => ImageContentFit::Contain,
        "cover" => ImageContentFit::Cover,
        "fill" => ImageContentFit::Fill,
        "none" => ImageContentFit::None,
        "scale-down" => ImageContentFit::ScaleDown,
        _ => return Err(format!("invalid content_fit: {}", fit_str)),
    };

    if let GuiElementKind::Image(ref mut config) = element.kind {
        config.content_fit = fit;
    } else {
        return Err("gui_set_content_fit can only be applied to Image elements".to_string());
    }

    Ok(element.into_value())
}

/// Set image opacity
/// gui_set_opacity(element, opacity) -> new_element
fn gui_set_opacity(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_opacity requires 2 arguments".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let opacity = get_float(args, 1, "opacity")? as f32;

    if let GuiElementKind::Image(ref mut config) = element.kind {
        config.opacity = opacity.clamp(0.0, 1.0);
    } else {
        return Err("gui_set_opacity can only be applied to Image elements".to_string());
    }

    Ok(element.into_value())
}

// ==================== Conditional Rendering ====================

/// Create a conditional element
/// gui_if(condition_field, true_element) -> conditional element
/// gui_if(condition_field, true_element, false_element) -> conditional element
fn gui_if(args: &[Value]) -> NativeResult {
    if args.len() < 2 {
        return Err(
            "gui_if requires at least 2 arguments (condition_field, true_element)".to_string(),
        );
    }

    let condition_field = get_string(args, 0, "condition_field")?;
    let true_element = clone_gui_element(&args[1])?;
    let false_element = if args.len() >= 3 {
        Some(clone_gui_element(&args[2])?)
    } else {
        None
    };

    let element = GuiElement::conditional(&condition_field)
        .true_element(true_element)
        .false_element(false_element.unwrap_or_else(|| GuiElement::spacer().build()))
        .build();

    Ok(element.into_value())
}

// ==================== List Rendering ====================

/// Create a for-each element for list rendering
/// gui_for_each(list_field) -> for_each element
///
/// Note: The template callback must be registered separately via the runtime's
/// callback registry. Use the template_id builder method to associate it.
///
/// For full list rendering functionality, use the runtime's expand_for_each method
/// which handles closure execution and element generation.
fn gui_for_each(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("gui_for_each requires at least 1 argument (list_field)".to_string());
    }

    let list_field = get_string(args, 0, "list_field")?;

    let builder = GuiElement::for_each(&list_field);

    Ok(builder.build().into_value())
}

// ==================== Computed Properties ====================

/// Register a computed property (note: this is a placeholder - actual registration
/// happens via ReactiveState, this native just returns a descriptor)
/// gui_computed(name, dependencies, compute_fn) -> computed property descriptor
fn gui_computed(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(
            "gui_computed requires 3 arguments (name, dependencies, compute_fn)".to_string(),
        );
    }

    let name = get_string(args, 0, "name")?;

    // Get dependencies as a list of strings (validated but stored via args[1])
    let deps_value = &args[1];
    let _dependencies = match deps_value {
        Value::List(list) => {
            let list = list.borrow();
            let mut deps = Vec::new();
            for item in list.iter() {
                if let Value::String(s) = item {
                    deps.push(s.to_string());
                } else {
                    return Err(format!(
                        "dependency must be a string, got {}",
                        item.type_name()
                    ));
                }
            }
            deps
        }
        _ => {
            return Err(format!(
                "dependencies must be a list, got {}",
                deps_value.type_name()
            ))
        }
    };

    let compute_fn = args[2].clone();
    if !matches!(compute_fn, Value::Closure(_)) {
        return Err(format!(
            "compute_fn must be a closure, got {}",
            compute_fn.type_name()
        ));
    }

    // Return a struct describing the computed property
    // The actual registration happens at the runtime level
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use stratum_core::bytecode::StructInstance;

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), Value::String(Rc::new(name)));
    fields.insert("dependencies".to_string(), args[1].clone());
    fields.insert("compute_fn".to_string(), compute_fn);

    let mut instance = StructInstance::new("ComputedPropertyDescriptor".to_string());
    instance.fields = fields;

    Ok(Value::Struct(Rc::new(RefCell::new(instance))))
}

// ==================== DataTable Functions ====================

/// Create a data table element for displaying DataFrame data
/// gui_data_table(dataframe) -> data_table element
/// gui_data_table(dataframe, page_size) -> data_table element with pagination
fn gui_data_table(args: &[Value]) -> NativeResult {
    if args.is_empty() {
        return Err("gui_data_table requires at least 1 argument (dataframe)".to_string());
    }

    // Extract DataFrame from Value
    let df = match &args[0] {
        Value::DataFrame(df) => Arc::clone(df),
        _ => {
            return Err(format!(
                "gui_data_table first argument must be a DataFrame, got {}",
                args[0].type_name()
            ))
        }
    };

    let mut builder = GuiElement::data_table().dataframe(df);

    // Optional page_size
    if let Some(page_size) = args.get(1) {
        if let Value::Int(size) = page_size {
            if *size > 0 {
                builder = builder.page_size(Some(*size as usize));
            }
        }
    }

    Ok(builder.build().into_value())
}

/// Set the columns to display in a data table
/// gui_set_table_columns(element, columns) -> new_element
fn gui_set_table_columns(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_table_columns requires 2 arguments (element, columns)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    // Extract column names from list
    let columns = match &args[1] {
        Value::List(list) => {
            let list = list.borrow();
            let mut cols = Vec::new();
            for item in list.iter() {
                if let Value::String(s) = item {
                    cols.push(s.to_string());
                } else {
                    return Err(format!(
                        "column name must be a string, got {}",
                        item.type_name()
                    ));
                }
            }
            cols
        }
        _ => {
            return Err(format!(
                "columns must be a list, got {}",
                args[1].type_name()
            ))
        }
    };

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.columns = Some(columns);
    } else {
        return Err("gui_set_table_columns can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the page size for pagination
/// gui_set_page_size(element, size) -> new_element
fn gui_set_page_size(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_page_size requires 2 arguments (element, size)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let size = get_int(args, 1, "size")?;

    match &mut element.kind {
        GuiElementKind::DataTable(config) => {
            config.page_size = if size > 0 { Some(size as usize) } else { None };
        }
        GuiElementKind::CubeTable(config) => {
            config.page_size = if size > 0 { Some(size as usize) } else { None };
        }
        _ => {
            return Err(
                "gui_set_page_size can only be applied to DataTable or CubeTable elements"
                    .to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set the current page (0-indexed)
/// gui_set_current_page(element, page) -> new_element
fn gui_set_current_page(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_current_page requires 2 arguments (element, page)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let page = get_int(args, 1, "page")?;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.current_page = page.max(0) as usize;
    } else {
        return Err("gui_set_current_page can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Enable or disable sorting
/// gui_set_sortable(element, sortable) -> new_element
fn gui_set_sortable(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_sortable requires 2 arguments (element, sortable)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let sortable = match &args[1] {
        Value::Bool(b) => *b,
        _ => {
            return Err(format!(
                "sortable must be a boolean, got {}",
                args[1].type_name()
            ))
        }
    };

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.sortable = sortable;
    } else {
        return Err("gui_set_sortable can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the sort column and direction
/// gui_set_sort_by(element, column, ascending) -> new_element
fn gui_set_sort_by(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(
            "gui_set_sort_by requires 3 arguments (element, column, ascending)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let column = get_string(args, 1, "column")?;
    let ascending = match &args[2] {
        Value::Bool(b) => *b,
        _ => {
            return Err(format!(
                "ascending must be a boolean, got {}",
                args[2].type_name()
            ))
        }
    };

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.sort_column = Some(column);
        config.sort_ascending = ascending;
    } else {
        return Err("gui_set_sort_by can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Enable or disable row selection
/// gui_set_selectable(element, selectable) -> new_element
fn gui_set_selectable(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_selectable requires 2 arguments (element, selectable)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let selectable = match &args[1] {
        Value::Bool(b) => *b,
        _ => {
            return Err(format!(
                "selectable must be a boolean, got {}",
                args[1].type_name()
            ))
        }
    };

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.selectable = selectable;
    } else {
        return Err("gui_set_selectable can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the selected rows
/// gui_set_selected_rows(element, rows) -> new_element
fn gui_set_selected_rows(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_selected_rows requires 2 arguments (element, rows)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    // Extract row indices from list
    let rows = match &args[1] {
        Value::List(list) => {
            let list = list.borrow();
            let mut indices = Vec::new();
            for item in list.iter() {
                if let Value::Int(i) = item {
                    if *i >= 0 {
                        indices.push(*i as usize);
                    }
                } else {
                    return Err(format!(
                        "row index must be an integer, got {}",
                        item.type_name()
                    ));
                }
            }
            indices
        }
        _ => return Err(format!("rows must be a list, got {}", args[1].type_name())),
    };

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.selected_rows = rows;
    } else {
        return Err("gui_set_selected_rows can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set a column width
/// gui_set_column_width(element, column, width) -> new_element
fn gui_set_column_width(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(
            "gui_set_column_width requires 3 arguments (element, column, width)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let column = get_string(args, 1, "column")?;
    let width = get_float(args, 2, "width")? as f32;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        // Remove existing width for this column if any
        config.column_widths.retain(|(c, _)| c != &column);
        config.column_widths.push((column, width));
    } else {
        return Err("gui_set_column_width can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the on_sort callback
/// gui_on_sort(element, callback_id) -> new_element
fn gui_on_sort(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_sort requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_int(args, 1, "callback_id")?;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.on_sort = Some(CallbackId::new(callback_id as u64));
    } else {
        return Err("gui_on_sort can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the on_page_change callback
/// gui_on_page_change(element, callback_id) -> new_element
fn gui_on_page_change(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_page_change requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_int(args, 1, "callback_id")?;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.on_page_change = Some(CallbackId::new(callback_id as u64));
    } else {
        return Err("gui_on_page_change can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the on_selection_change callback
/// gui_on_selection_change(element, callback_id) -> new_element
fn gui_on_selection_change(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(
            "gui_on_selection_change requires 2 arguments (element, callback_id)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_int(args, 1, "callback_id")?;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.on_selection_change = Some(CallbackId::new(callback_id as u64));
    } else {
        return Err(
            "gui_on_selection_change can only be applied to DataTable elements".to_string(),
        );
    }

    Ok(element.into_value())
}

/// Set the on_row_click callback
/// gui_on_row_click(element, callback_id) -> new_element
fn gui_on_row_click(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_row_click requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_int(args, 1, "callback_id")?;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.on_row_click = Some(CallbackId::new(callback_id as u64));
    } else {
        return Err("gui_on_row_click can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

/// Set the on_cell_click callback
/// gui_on_cell_click(element, callback_id) -> new_element
fn gui_on_cell_click(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_cell_click requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_int(args, 1, "callback_id")?;

    if let GuiElementKind::DataTable(ref mut config) = element.kind {
        config.on_cell_click = Some(CallbackId::new(callback_id as u64));
    } else {
        return Err("gui_on_cell_click can only be applied to DataTable elements".to_string());
    }

    Ok(element.into_value())
}

// ========== Chart Native Functions ==========

/// Create a BarChart element
/// gui_bar_chart() or gui_bar_chart(data) where data is a list of [label, value] pairs
fn gui_bar_chart(args: &[Value]) -> NativeResult {
    let mut config = BarChartConfig::default();

    // Parse data if provided as first argument
    if let Some(data_val) = args.first() {
        config.data = parse_chart_data(data_val)?;
    }

    let element = GuiElement::bar_chart_with_data(config.data).build();
    Ok(element.into_value())
}

/// Create a LineChart element
/// gui_line_chart() or gui_line_chart(labels, series_name, values)
fn gui_line_chart(args: &[Value]) -> NativeResult {
    let mut config = LineChartConfig::default();

    // Parse labels if provided
    if let Some(labels_val) = args.first() {
        if let Value::List(list) = labels_val {
            let list = list.borrow();
            for item in list.iter() {
                if let Value::String(s) = item {
                    config.labels.push(s.to_string());
                }
            }
        }
    }

    let element = GuiElement::line_chart_with_data(config.labels, config.series).build();
    Ok(element.into_value())
}

/// Create a PieChart element
/// gui_pie_chart() or gui_pie_chart(data) where data is a list of [label, value] pairs
fn gui_pie_chart(args: &[Value]) -> NativeResult {
    let mut config = PieChartConfig::default();

    // Parse data if provided as first argument
    if let Some(data_val) = args.first() {
        config.data = parse_chart_data(data_val)?;
    }

    let element = GuiElement::pie_chart_with_data(config.data).build();
    Ok(element.into_value())
}

/// Helper to parse chart data from Value (list of [label, value] pairs)
fn parse_chart_data(value: &Value) -> Result<Vec<DataPoint>, String> {
    let mut data = Vec::new();

    match value {
        Value::List(list) => {
            let list = list.borrow();
            for item in list.iter() {
                // [label, value] pairs
                if let Value::List(pair) = item {
                    let pair = pair.borrow();
                    if pair.len() >= 2 {
                        let label = match &pair[0] {
                            Value::String(s) => s.to_string(),
                            v => v.to_string(),
                        };
                        let point_value = match &pair[1] {
                            Value::Float(f) => *f,
                            Value::Int(i) => *i as f64,
                            _ => continue,
                        };
                        data.push(DataPoint::new(label, point_value));
                    }
                }
            }
        }
        _ => return Err("chart data must be a list of [label, value] pairs".to_string()),
    }

    Ok(data)
}

/// Set the chart title
/// gui_set_chart_title(element, title) -> new_element
fn gui_set_chart_title(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_chart_title requires 2 arguments (element, title)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let title = get_string(args, 1, "title")?;

    match &mut element.kind {
        GuiElementKind::BarChart(c) => c.title = Some(title),
        GuiElementKind::LineChart(c) => c.title = Some(title),
        GuiElementKind::PieChart(c) => c.title = Some(title),
        GuiElementKind::CubeChart(c) => c.title = Some(title),
        _ => return Err("gui_set_chart_title can only be applied to chart elements".to_string()),
    }

    Ok(element.into_value())
}

/// Set the chart size
/// gui_set_chart_size(element, width, height) -> new_element
fn gui_set_chart_size(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err("gui_set_chart_size requires 3 arguments (element, width, height)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let width = get_float(args, 1, "width")? as f32;
    let height = get_float(args, 2, "height")? as f32;

    match &mut element.kind {
        GuiElementKind::BarChart(c) => {
            c.width = width;
            c.height = height;
        }
        GuiElementKind::LineChart(c) => {
            c.width = width;
            c.height = height;
        }
        GuiElementKind::PieChart(c) => {
            c.width = width;
            c.height = height;
        }
        GuiElementKind::CubeChart(c) => {
            c.width = width;
            c.height = height;
        }
        _ => return Err("gui_set_chart_size can only be applied to chart elements".to_string()),
    }

    Ok(element.into_value())
}

/// Set chart data (for BarChart, PieChart)
/// gui_set_chart_data(element, data) -> new_element
fn gui_set_chart_data(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_chart_data requires 2 arguments (element, data)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let data = parse_chart_data(&args[1])?;

    match &mut element.kind {
        GuiElementKind::BarChart(c) => c.data = data,
        GuiElementKind::PieChart(c) => c.data = data,
        _ => {
            return Err(
                "gui_set_chart_data can only be applied to BarChart or PieChart".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set chart data from separate label and value arrays (type-safe alternative)
/// gui_set_chart_data_arrays(element, labels, values) -> new_element
fn gui_set_chart_data_arrays(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(
            "gui_set_chart_data_arrays requires 3 arguments (element, labels, values)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;

    // Extract labels
    let labels = match &args[1] {
        Value::List(list) => list
            .borrow()
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok((**s).clone()),
                _ => Ok(v.to_string()),
            })
            .collect::<Result<Vec<String>, String>>()?,
        _ => return Err("labels must be a list of strings".to_string()),
    };

    // Extract values
    let values = match &args[2] {
        Value::List(list) => list
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Float(f) => Ok(*f),
                Value::Int(i) => Ok(*i as f64),
                _ => Err(format!("values must be numeric, got {}", v.type_name())),
            })
            .collect::<Result<Vec<f64>, String>>()?,
        _ => return Err("values must be a list of numbers".to_string()),
    };

    if labels.len() != values.len() {
        return Err(format!(
            "labels and values must have the same length: {} vs {}",
            labels.len(),
            values.len()
        ));
    }

    // Build DataPoints
    let data: Vec<DataPoint> = labels
        .into_iter()
        .zip(values)
        .map(|(label, value)| DataPoint { label, value })
        .collect();

    match &mut element.kind {
        GuiElementKind::BarChart(c) => c.data = data,
        GuiElementKind::PieChart(c) => c.data = data,
        _ => {
            return Err(
                "gui_set_chart_data_arrays can only be applied to BarChart or PieChart".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Add a data series to a LineChart
/// gui_add_chart_series(element, name, values) -> new_element
fn gui_add_chart_series(args: &[Value]) -> NativeResult {
    if args.len() != 3 {
        return Err(
            "gui_add_chart_series requires 3 arguments (element, name, values)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let name = get_string(args, 1, "name")?;

    let values = match &args[2] {
        Value::List(list) => {
            let list = list.borrow();
            list.iter()
                .filter_map(|v| match v {
                    Value::Float(f) => Some(*f),
                    Value::Int(i) => Some(*i as f64),
                    _ => None,
                })
                .collect()
        }
        _ => return Err("values must be a list of numbers".to_string()),
    };

    if let GuiElementKind::LineChart(c) = &mut element.kind {
        c.series.push(DataSeries::new(name, values));
    } else {
        return Err("gui_add_chart_series can only be applied to LineChart".to_string());
    }

    Ok(element.into_value())
}

/// Set x-axis labels for a LineChart
/// gui_set_chart_labels(element, labels) -> new_element
fn gui_set_chart_labels(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_chart_labels requires 2 arguments (element, labels)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;

    let labels = match &args[1] {
        Value::List(list) => {
            let list = list.borrow();
            list.iter()
                .map(|v| match v {
                    Value::String(s) => s.to_string(),
                    v => v.to_string(),
                })
                .collect()
        }
        _ => return Err("labels must be a list of strings".to_string()),
    };

    if let GuiElementKind::LineChart(c) = &mut element.kind {
        c.labels = labels;
    } else {
        return Err("gui_set_chart_labels can only be applied to LineChart".to_string());
    }

    Ok(element.into_value())
}

/// Show or hide the chart legend
/// gui_set_show_legend(element, show) -> new_element
fn gui_set_show_legend(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_show_legend requires 2 arguments (element, show)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let show = match &args[1] {
        Value::Bool(b) => *b,
        _ => return Err("show must be a boolean".to_string()),
    };

    match &mut element.kind {
        GuiElementKind::BarChart(c) => c.show_legend = show,
        GuiElementKind::LineChart(c) => c.show_legend = show,
        GuiElementKind::PieChart(c) => c.show_legend = show,
        GuiElementKind::CubeChart(c) => c.show_legend = show,
        _ => return Err("gui_set_show_legend can only be applied to chart elements".to_string()),
    }

    Ok(element.into_value())
}

/// Show or hide the chart grid
/// gui_set_show_grid(element, show) -> new_element
fn gui_set_show_grid(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_show_grid requires 2 arguments (element, show)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let show = match &args[1] {
        Value::Bool(b) => *b,
        _ => return Err("show must be a boolean".to_string()),
    };

    match &mut element.kind {
        GuiElementKind::BarChart(c) => c.show_grid = show,
        GuiElementKind::LineChart(c) => c.show_grid = show,
        GuiElementKind::CubeChart(c) => c.show_grid = show,
        _ => {
            return Err(
                "gui_set_show_grid can only be applied to BarChart, LineChart, or CubeChart"
                    .to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set bar color for BarChart
/// gui_set_bar_color(element, r, g, b) -> new_element
fn gui_set_bar_color(args: &[Value]) -> NativeResult {
    if args.len() != 4 {
        return Err("gui_set_bar_color requires 4 arguments (element, r, g, b)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let r = get_int(args, 1, "r")? as u8;
    let g = get_int(args, 2, "g")? as u8;
    let b = get_int(args, 3, "b")? as u8;

    if let GuiElementKind::BarChart(c) = &mut element.kind {
        c.bar_color = Some((r, g, b));
    } else {
        return Err("gui_set_bar_color can only be applied to BarChart".to_string());
    }

    Ok(element.into_value())
}

/// Set inner radius ratio for PieChart (donut chart)
/// gui_set_inner_radius(element, ratio) -> new_element
fn gui_set_inner_radius(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_inner_radius requires 2 arguments (element, ratio)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let ratio = get_float(args, 1, "ratio")? as f32;

    if let GuiElementKind::PieChart(c) = &mut element.kind {
        c.inner_radius_ratio = ratio.clamp(0.0, 0.9);
    } else {
        return Err("gui_set_inner_radius can only be applied to PieChart".to_string());
    }

    Ok(element.into_value())
}

// =============================================================================
// OLAP Cube Widget Native Functions
// =============================================================================

/// Create a CubeTable element
fn gui_cube_table(args: &[Value]) -> NativeResult {
    use crate::element::{CubeTableConfig, ElementStyle, GuiElement, GuiElementKind};

    let cube = if !args.is_empty() {
        match &args[0] {
            Value::Cube(c) => Some(c.clone()),
            _ => None,
        }
    } else {
        None
    };

    let element = GuiElement {
        kind: GuiElementKind::CubeTable(CubeTableConfig {
            cube,
            ..Default::default()
        }),
        children: Vec::new(),
        style: ElementStyle::new(),
    };

    Ok(element.into_value())
}

/// Create a CubeChart element
fn gui_cube_chart(args: &[Value]) -> NativeResult {
    use crate::element::{
        CubeChartConfig, CubeChartType, ElementStyle, GuiElement, GuiElementKind,
    };

    let cube = if !args.is_empty() {
        match &args[0] {
            Value::Cube(c) => Some(c.clone()),
            _ => None,
        }
    } else {
        None
    };

    let chart_type = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => CubeChartType::from_str(s),
            _ => CubeChartType::Bar,
        }
    } else {
        CubeChartType::Bar
    };

    let element = GuiElement {
        kind: GuiElementKind::CubeChart(CubeChartConfig {
            cube,
            chart_type,
            ..Default::default()
        }),
        children: Vec::new(),
        style: ElementStyle::new(),
    };

    Ok(element.into_value())
}

/// Create a DimensionFilter element
fn gui_dimension_filter(args: &[Value]) -> NativeResult {
    use crate::element::{DimensionFilterConfig, ElementStyle, GuiElement, GuiElementKind};

    let cube = if !args.is_empty() {
        match &args[0] {
            Value::Cube(c) => Some(c.clone()),
            _ => None,
        }
    } else {
        None
    };

    let dimension = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.to_string(),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let element = GuiElement {
        kind: GuiElementKind::DimensionFilter(DimensionFilterConfig {
            cube,
            dimension,
            ..Default::default()
        }),
        children: Vec::new(),
        style: ElementStyle::new(),
    };

    Ok(element.into_value())
}

/// Create a HierarchyNavigator element
fn gui_hierarchy_navigator(args: &[Value]) -> NativeResult {
    use crate::element::{ElementStyle, GuiElement, GuiElementKind, HierarchyNavigatorConfig};

    let cube = if !args.is_empty() {
        match &args[0] {
            Value::Cube(c) => Some(c.clone()),
            _ => None,
        }
    } else {
        None
    };

    let hierarchy = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.to_string(),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let element = GuiElement {
        kind: GuiElementKind::HierarchyNavigator(HierarchyNavigatorConfig {
            cube,
            hierarchy,
            ..Default::default()
        }),
        children: Vec::new(),
        style: ElementStyle::new(),
    };

    Ok(element.into_value())
}

/// Create a MeasureSelector element
fn gui_measure_selector(args: &[Value]) -> NativeResult {
    use crate::element::{ElementStyle, GuiElement, GuiElementKind, MeasureSelectorConfig};

    let cube = if !args.is_empty() {
        match &args[0] {
            Value::Cube(c) => Some(c.clone()),
            _ => None,
        }
    } else {
        None
    };

    let element = GuiElement {
        kind: GuiElementKind::MeasureSelector(MeasureSelectorConfig {
            cube,
            ..Default::default()
        }),
        children: Vec::new(),
        style: ElementStyle::new(),
    };

    Ok(element.into_value())
}

/// Set the cube for an OLAP widget
fn gui_set_cube(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_cube requires 2 arguments (element, cube)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let cube = match &args[1] {
        Value::Cube(c) => Some(c.clone()),
        _ => return Err("second argument must be a Cube".to_string()),
    };

    match &mut element.kind {
        GuiElementKind::CubeTable(c) => c.cube = cube,
        GuiElementKind::CubeChart(c) => c.cube = cube,
        GuiElementKind::DimensionFilter(c) => c.cube = cube,
        GuiElementKind::HierarchyNavigator(c) => c.cube = cube,
        GuiElementKind::MeasureSelector(c) => c.cube = cube,
        _ => return Err("gui_set_cube can only be applied to OLAP widgets".to_string()),
    }

    Ok(element.into_value())
}

/// Set row dimensions for CubeTable
fn gui_set_row_dimensions(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(
            "gui_set_row_dimensions requires 2 arguments (element, dimensions)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let dims = match &args[1] {
        Value::List(list) => list
            .borrow()
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        _ => return Err("second argument must be a list of strings".to_string()),
    };

    if let GuiElementKind::CubeTable(c) = &mut element.kind {
        c.row_dimensions = dims;
    } else {
        return Err("gui_set_row_dimensions can only be applied to CubeTable".to_string());
    }

    Ok(element.into_value())
}

/// Set measures for CubeTable or MeasureSelector
fn gui_set_measures(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_measures requires 2 arguments (element, measures)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let measures = match &args[1] {
        Value::List(list) => list
            .borrow()
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        _ => return Err("second argument must be a list of strings".to_string()),
    };

    match &mut element.kind {
        GuiElementKind::CubeTable(c) => c.measures = measures,
        GuiElementKind::MeasureSelector(c) => c.selected_measures = measures,
        _ => {
            return Err(
                "gui_set_measures can only be applied to CubeTable or MeasureSelector".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set chart type for CubeChart
fn gui_set_cube_chart_type(args: &[Value]) -> NativeResult {
    use crate::element::CubeChartType;

    if args.len() != 2 {
        return Err("gui_set_cube_chart_type requires 2 arguments (element, type)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let chart_type = match &args[1] {
        Value::String(s) => CubeChartType::from_str(s),
        _ => return Err("second argument must be a string (bar, line, pie)".to_string()),
    };

    if let GuiElementKind::CubeChart(c) = &mut element.kind {
        c.chart_type = chart_type;
    } else {
        return Err("gui_set_cube_chart_type can only be applied to CubeChart".to_string());
    }

    Ok(element.into_value())
}

/// Set X dimension for CubeChart
fn gui_set_x_dimension(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_x_dimension requires 2 arguments (element, dimension)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let dim = match &args[1] {
        Value::String(s) => s.to_string(),
        _ => return Err("second argument must be a string".to_string()),
    };

    if let GuiElementKind::CubeChart(c) = &mut element.kind {
        c.x_dimension = Some(dim);
    } else {
        return Err("gui_set_x_dimension can only be applied to CubeChart".to_string());
    }

    Ok(element.into_value())
}

/// Set Y measure for CubeChart
fn gui_set_y_measure(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_y_measure requires 2 arguments (element, measure)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let measure = match &args[1] {
        Value::String(s) => s.to_string(),
        _ => return Err("second argument must be a string".to_string()),
    };

    if let GuiElementKind::CubeChart(c) = &mut element.kind {
        c.y_measure = Some(measure);
    } else {
        return Err("gui_set_y_measure can only be applied to CubeChart".to_string());
    }

    Ok(element.into_value())
}

/// Set series dimension for CubeChart
fn gui_set_series_dimension(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(
            "gui_set_series_dimension requires 2 arguments (element, dimension)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let dim = match &args[1] {
        Value::String(s) => s.to_string(),
        _ => return Err("second argument must be a string".to_string()),
    };

    if let GuiElementKind::CubeChart(c) = &mut element.kind {
        c.series_dimension = Some(dim);
    } else {
        return Err("gui_set_series_dimension can only be applied to CubeChart".to_string());
    }

    Ok(element.into_value())
}

/// Set dimension for DimensionFilter
fn gui_set_filter_dimension(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err(
            "gui_set_filter_dimension requires 2 arguments (element, dimension)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let dim = match &args[1] {
        Value::String(s) => s.to_string(),
        _ => return Err("second argument must be a string".to_string()),
    };

    if let GuiElementKind::DimensionFilter(c) = &mut element.kind {
        c.dimension = dim;
    } else {
        return Err("gui_set_filter_dimension can only be applied to DimensionFilter".to_string());
    }

    Ok(element.into_value())
}

/// Set hierarchy for HierarchyNavigator
fn gui_set_hierarchy(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_hierarchy requires 2 arguments (element, hierarchy)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let hier = match &args[1] {
        Value::String(s) => s.to_string(),
        _ => return Err("second argument must be a string".to_string()),
    };

    if let GuiElementKind::HierarchyNavigator(c) = &mut element.kind {
        c.hierarchy = hier;
    } else {
        return Err("gui_set_hierarchy can only be applied to HierarchyNavigator".to_string());
    }

    Ok(element.into_value())
}

/// Set current level for HierarchyNavigator
fn gui_set_current_level(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_current_level requires 2 arguments (element, level)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let level = match &args[1] {
        Value::String(s) => s.to_string(),
        _ => return Err("second argument must be a string".to_string()),
    };

    if let GuiElementKind::HierarchyNavigator(c) = &mut element.kind {
        c.current_level = Some(level);
    } else {
        return Err("gui_set_current_level can only be applied to HierarchyNavigator".to_string());
    }

    Ok(element.into_value())
}

/// Set drill callback for CubeTable or HierarchyNavigator
fn gui_on_drill(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_drill requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = match &args[1] {
        Value::Int(id) => CallbackId::new(*id as u64),
        _ => return Err("second argument must be a callback ID".to_string()),
    };

    match &mut element.kind {
        GuiElementKind::CubeTable(c) => c.on_drill = Some(callback_id),
        GuiElementKind::HierarchyNavigator(c) => c.on_drill_down = Some(callback_id),
        _ => {
            return Err(
                "gui_on_drill can only be applied to CubeTable or HierarchyNavigator".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set roll-up callback for CubeTable or HierarchyNavigator
fn gui_on_roll_up(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_roll_up requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = match &args[1] {
        Value::Int(id) => CallbackId::new(*id as u64),
        _ => return Err("second argument must be a callback ID".to_string()),
    };

    match &mut element.kind {
        GuiElementKind::CubeTable(c) => c.on_roll_up = Some(callback_id),
        GuiElementKind::HierarchyNavigator(c) => c.on_roll_up = Some(callback_id),
        _ => {
            return Err(
                "gui_on_roll_up can only be applied to CubeTable or HierarchyNavigator".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set level change callback for HierarchyNavigator
fn gui_on_level_change(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_level_change requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = match &args[1] {
        Value::Int(id) => CallbackId::new(*id as u64),
        _ => return Err("second argument must be a callback ID".to_string()),
    };

    if let GuiElementKind::HierarchyNavigator(c) = &mut element.kind {
        c.on_level_change = Some(callback_id);
    } else {
        return Err("gui_on_level_change can only be applied to HierarchyNavigator".to_string());
    }

    Ok(element.into_value())
}

// =============================================================================
// Widget Styling Functions
// =============================================================================

/// Set background color for any widget
/// gui_set_background(element, r, g, b) or gui_set_background(element, r, g, b, a)
fn gui_set_background(args: &[Value]) -> NativeResult {
    use crate::theme::Color;

    if args.len() < 4 {
        return Err(
            "gui_set_background requires at least 4 arguments (element, r, g, b)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let r = get_int(args, 1, "r")? as u8;
    let g = get_int(args, 2, "g")? as u8;
    let b = get_int(args, 3, "b")? as u8;
    let a = if args.len() > 4 {
        get_int(args, 4, "a")? as u8
    } else {
        255
    };

    element.style.widget_style.background = Some(Color::rgba(r, g, b, a));
    Ok(element.into_value())
}

/// Set foreground/text color for any widget
/// gui_set_foreground(element, r, g, b) or gui_set_foreground(element, r, g, b, a)
fn gui_set_foreground(args: &[Value]) -> NativeResult {
    use crate::theme::Color;

    if args.len() < 4 {
        return Err(
            "gui_set_foreground requires at least 4 arguments (element, r, g, b)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let r = get_int(args, 1, "r")? as u8;
    let g = get_int(args, 2, "g")? as u8;
    let b = get_int(args, 3, "b")? as u8;
    let a = if args.len() > 4 {
        get_int(args, 4, "a")? as u8
    } else {
        255
    };

    element.style.widget_style.foreground = Some(Color::rgba(r, g, b, a));
    Ok(element.into_value())
}

/// Set border color for any widget
/// gui_set_border_color(element, r, g, b) or gui_set_border_color(element, r, g, b, a)
fn gui_set_border_color(args: &[Value]) -> NativeResult {
    use crate::theme::Color;

    if args.len() < 4 {
        return Err(
            "gui_set_border_color requires at least 4 arguments (element, r, g, b)".to_string(),
        );
    }

    let mut element = clone_gui_element(&args[0])?;
    let r = get_int(args, 1, "r")? as u8;
    let g = get_int(args, 2, "g")? as u8;
    let b = get_int(args, 3, "b")? as u8;
    let a = if args.len() > 4 {
        get_int(args, 4, "a")? as u8
    } else {
        255
    };

    element.style.widget_style.border_color = Some(Color::rgba(r, g, b, a));
    Ok(element.into_value())
}

/// Set border width for any widget
/// gui_set_border_width(element, width) -> new_element
fn gui_set_border_width(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_border_width requires 2 arguments (element, width)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let width = get_float(args, 1, "width")? as f32;

    element.style.widget_style.border_width = Some(width);
    Ok(element.into_value())
}

/// Set corner radius for rounded corners
/// gui_set_corner_radius(element, radius) -> new_element
fn gui_set_corner_radius(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_set_corner_radius requires 2 arguments (element, radius)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let radius = get_float(args, 1, "radius")? as f32;

    element.style.widget_style.corner_radius = Some(radius);
    Ok(element.into_value())
}

/// Get list of all available theme preset names
/// gui_theme_presets() -> [string]
fn gui_theme_presets(_args: &[Value]) -> NativeResult {
    use crate::theme::ThemePreset;
    use std::cell::RefCell;
    use std::rc::Rc;

    let names: Vec<Value> = ThemePreset::all_names()
        .iter()
        .map(|s| Value::String(Rc::new((*s).to_string())))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(names))))
}

/// Set the application theme by preset name
/// gui_set_theme(preset_name) -> null
fn gui_set_theme(args: &[Value]) -> NativeResult {
    use crate::bindings::request_theme_preset;
    use crate::theme::ThemePreset;

    let preset_name = get_string(args, 0, "preset_name")?;

    let preset = ThemePreset::from_name(&preset_name).ok_or_else(|| {
        format!(
            "Unknown theme preset: '{}'. Use Gui.theme_presets() to see available themes.",
            preset_name
        )
    })?;

    request_theme_preset(preset);
    Ok(Value::Null)
}

/// Create and set a custom theme with a name and palette
/// gui_custom_theme(name, palette) -> null
/// palette can be:
///   - A struct with fields: background, text, primary, success, warning, danger
///   - Each color field can be: (r, g, b), (r, g, b, a), or "#RRGGBB" hex string
fn gui_custom_theme(args: &[Value]) -> NativeResult {
    use crate::bindings::request_custom_theme;
    use crate::theme::StratumPalette;

    if args.len() < 2 {
        return Err("gui_custom_theme requires 2 arguments (name, palette)".to_string());
    }

    let name = get_string(args, 0, "name")?;
    let palette_value = &args[1];

    // Extract palette from struct
    let palette = match palette_value {
        Value::Struct(struct_ref) => {
            let instance = struct_ref.borrow();

            let background = extract_color_from_field(&instance.fields, "background")?;
            let text = extract_color_from_field(&instance.fields, "text")?;
            let primary = extract_color_from_field(&instance.fields, "primary")?;
            let success = extract_color_from_field(&instance.fields, "success")?;
            let warning = extract_color_from_field(&instance.fields, "warning")?;
            let danger = extract_color_from_field(&instance.fields, "danger")?;

            StratumPalette::new(background, text, primary, success, warning, danger)
        }
        _ => {
            return Err(format!(
                "palette must be a struct with color fields, got {}",
                palette_value.type_name()
            ))
        }
    };

    request_custom_theme(name, palette);
    Ok(Value::Null)
}

/// Helper to extract a Color from a struct field
fn extract_color_from_field(
    fields: &std::collections::HashMap<String, Value>,
    field_name: &str,
) -> Result<crate::theme::Color, String> {
    let value = fields
        .get(field_name)
        .ok_or_else(|| format!("palette missing required field: '{}'", field_name))?;

    extract_color_value(value, field_name)
}

/// Helper to extract a Color from a Value
fn extract_color_value(value: &Value, context: &str) -> Result<crate::theme::Color, String> {
    use crate::theme::Color;

    match value {
        // Hex string: "#RRGGBB" or "#RRGGBBAA"
        Value::String(s) => {
            Color::from_hex(s).ok_or_else(|| format!("{}: invalid hex color '{}'", context, s))
        }
        // List/tuple: [r, g, b] or [r, g, b, a]
        Value::List(list) => {
            let list = list.borrow();
            match list.len() {
                3 => {
                    let r = get_u8_from_value(&list[0], &format!("{}.r", context))?;
                    let g = get_u8_from_value(&list[1], &format!("{}.g", context))?;
                    let b = get_u8_from_value(&list[2], &format!("{}.b", context))?;
                    Ok(Color::rgb(r, g, b))
                }
                4 => {
                    let r = get_u8_from_value(&list[0], &format!("{}.r", context))?;
                    let g = get_u8_from_value(&list[1], &format!("{}.g", context))?;
                    let b = get_u8_from_value(&list[2], &format!("{}.b", context))?;
                    let a = get_u8_from_value(&list[3], &format!("{}.a", context))?;
                    Ok(Color::rgba(r, g, b, a))
                }
                n => Err(format!(
                    "{}: color list must have 3 or 4 elements, got {}",
                    context, n
                )),
            }
        }
        // Struct with r, g, b, a fields
        Value::Struct(struct_ref) => {
            let instance = struct_ref.borrow();
            let r = instance
                .fields
                .get("r")
                .map(|v| get_u8_from_value(v, &format!("{}.r", context)))
                .transpose()?
                .unwrap_or(0);
            let g = instance
                .fields
                .get("g")
                .map(|v| get_u8_from_value(v, &format!("{}.g", context)))
                .transpose()?
                .unwrap_or(0);
            let b = instance
                .fields
                .get("b")
                .map(|v| get_u8_from_value(v, &format!("{}.b", context)))
                .transpose()?
                .unwrap_or(0);
            let a = instance
                .fields
                .get("a")
                .map(|v| get_u8_from_value(v, &format!("{}.a", context)))
                .transpose()?
                .unwrap_or(255);
            Ok(Color::rgba(r, g, b, a))
        }
        _ => Err(format!(
            "{}: color must be hex string, [r,g,b], [r,g,b,a], or color struct, got {}",
            context,
            value.type_name()
        )),
    }
}

/// Helper to extract u8 from a Value (for color components)
fn get_u8_from_value(value: &Value, context: &str) -> Result<u8, String> {
    match value {
        Value::Int(i) => {
            if *i < 0 || *i > 255 {
                Err(format!(
                    "{}: color component must be 0-255, got {}",
                    context, i
                ))
            } else {
                Ok(*i as u8)
            }
        }
        Value::Float(f) => {
            // Allow floats in 0.0-1.0 range (convert to 0-255)
            if *f >= 0.0 && *f <= 1.0 {
                Ok((f * 255.0) as u8)
            } else if *f >= 0.0 && *f <= 255.0 {
                Ok(*f as u8)
            } else {
                Err(format!(
                    "{}: color component must be 0-255 or 0.0-1.0, got {}",
                    context, f
                ))
            }
        }
        _ => Err(format!(
            "{}: color component must be a number, got {}",
            context,
            value.type_name()
        )),
    }
}

// ============================================================================
// Interactive Element Functions
// ============================================================================

/// Create an interactive wrapper element
/// gui_interactive(child?) -> element
fn gui_interactive(args: &[Value]) -> NativeResult {
    let builder = GuiElement::interactive();

    // If a child element is provided, add it
    let builder = if !args.is_empty() {
        let child = clone_gui_element(&args[0])?;
        builder.child(child)
    } else {
        builder
    };

    Ok(builder.build().into_value())
}

/// Set on_press callback for Interactive element
/// gui_on_press(element, callback_id) -> new_element
fn gui_on_press(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_press requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_press = Some(callback_id);
    } else {
        return Err("gui_on_press can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_release callback for Interactive element
/// gui_on_mouse_release(element, callback_id) -> new_element
fn gui_on_mouse_release(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_mouse_release requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_release = Some(callback_id);
    } else {
        return Err("gui_on_mouse_release can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_double_click callback for Interactive element
/// gui_on_double_click(element, callback_id) -> new_element
fn gui_on_double_click(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_double_click requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_double_click = Some(callback_id);
    } else {
        return Err("gui_on_double_click can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_right_press callback for Interactive element
/// gui_on_right_press(element, callback_id) -> new_element
fn gui_on_right_press(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_right_press requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_right_press = Some(callback_id);
    } else {
        return Err("gui_on_right_press can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_right_release callback for Interactive element
/// gui_on_right_release(element, callback_id) -> new_element
fn gui_on_right_release(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_right_release requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_right_release = Some(callback_id);
    } else {
        return Err("gui_on_right_release can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_enter (hover enter) callback for Interactive element
/// gui_on_hover_enter(element, callback_id) -> new_element
fn gui_on_hover_enter(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_hover_enter requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_enter = Some(callback_id);
    } else {
        return Err("gui_on_hover_enter can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_exit (hover exit) callback for Interactive element
/// gui_on_hover_exit(element, callback_id) -> new_element
fn gui_on_hover_exit(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_hover_exit requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_exit = Some(callback_id);
    } else {
        return Err("gui_on_hover_exit can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_move callback for Interactive element
/// gui_on_mouse_move(element, callback_id) -> new_element
fn gui_on_mouse_move(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_mouse_move requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_move = Some(callback_id);
    } else {
        return Err("gui_on_mouse_move can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_scroll callback for Interactive element
/// gui_on_mouse_scroll(element, callback_id) -> new_element
fn gui_on_mouse_scroll(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_mouse_scroll requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.on_scroll = Some(callback_id);
    } else {
        return Err("gui_on_mouse_scroll can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

/// Set cursor style for Interactive element
/// gui_set_cursor(element, cursor_name) -> new_element
fn gui_set_cursor(args: &[Value]) -> NativeResult {
    use crate::element::CursorStyle;

    if args.len() != 2 {
        return Err("gui_set_cursor requires 2 arguments (element, cursor_name)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let cursor_name = get_string(args, 1, "cursor_name")?;

    if let GuiElementKind::Interactive(ref mut config) = element.kind {
        config.cursor_style = Some(CursorStyle::from_str(&cursor_name));
    } else {
        return Err("gui_set_cursor can only be applied to Interactive elements".to_string());
    }

    Ok(element.into_value())
}

// =============================================================================
// Widget Event Handlers (on_change, on_submit, on_toggle, on_select)
// =============================================================================

/// Set on_change callback for form elements (TextField, Slider)
/// gui_on_change(element, callback_id) -> new_element
fn gui_on_change(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_change requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    match &mut element.kind {
        GuiElementKind::TextField(config) => config.on_change = Some(callback_id),
        GuiElementKind::Slider(config) => config.on_change = Some(callback_id),
        GuiElementKind::MeasureSelector(config) => config.on_change = Some(callback_id),
        _ => return Err(
            "gui_on_change can only be applied to TextField, Slider, or MeasureSelector elements"
                .to_string(),
        ),
    }

    Ok(element.into_value())
}

/// Set on_submit callback for TextField elements (triggered on Enter key)
/// gui_on_submit(element, callback_id) -> new_element
fn gui_on_submit(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_submit requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    if let GuiElementKind::TextField(config) = &mut element.kind {
        config.on_submit = Some(callback_id);
    } else {
        return Err("gui_on_submit can only be applied to TextField elements".to_string());
    }

    Ok(element.into_value())
}

/// Set on_toggle callback for toggle-based elements (Checkbox, Toggle)
/// gui_on_toggle(element, callback_id) -> new_element
fn gui_on_toggle(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_toggle requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    match &mut element.kind {
        GuiElementKind::Checkbox(config) => config.on_toggle = Some(callback_id),
        GuiElementKind::Toggle(config) => config.on_toggle = Some(callback_id),
        _ => {
            return Err(
                "gui_on_toggle can only be applied to Checkbox or Toggle elements".to_string(),
            )
        }
    }

    Ok(element.into_value())
}

/// Set on_select callback for selection-based elements (RadioButton, Dropdown, DimensionFilter)
/// gui_on_select(element, callback_id) -> new_element
fn gui_on_select(args: &[Value]) -> NativeResult {
    if args.len() != 2 {
        return Err("gui_on_select requires 2 arguments (element, callback_id)".to_string());
    }

    let mut element = clone_gui_element(&args[0])?;
    let callback_id = get_callback_id(&args[1])?;

    match &mut element.kind {
        GuiElementKind::RadioButton(config) => config.on_select = Some(callback_id),
        GuiElementKind::Dropdown(config) => config.on_select = Some(callback_id),
        GuiElementKind::DimensionFilter(config) => config.on_select = Some(callback_id),
        _ => return Err("gui_on_select can only be applied to RadioButton, Dropdown, or DimensionFilter elements".to_string()),
    }

    Ok(element.into_value())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_vstack_empty() {
        let result = gui_vstack(&[]);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(matches!(value, Value::GuiElement(_)));
    }

    #[test]
    fn test_gui_vstack_with_spacing() {
        let result = gui_vstack(&[Value::Float(16.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_text() {
        let result = gui_text(&[Value::string("Hello")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_text_with_size() {
        let result = gui_text(&[Value::string("Hello"), Value::Float(24.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_button() {
        let result = gui_button(&[Value::string("Click me")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_spacer_horizontal() {
        let result = gui_spacer(&[Value::string("horizontal")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_spacer_vertical() {
        let result = gui_spacer(&[Value::string("vertical")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_spacer_fixed() {
        let result = gui_spacer(&[Value::Float(10.0), Value::Float(20.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_grid() {
        let result = gui_grid(&[Value::Int(3)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_padding() {
        let elem = gui_vstack(&[]).unwrap();
        let result = gui_set_padding(&[elem, Value::Float(16.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_width() {
        let elem = gui_vstack(&[]).unwrap();
        let result = gui_set_width(&[elem, Value::string("fill")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_add_child() {
        let parent = gui_vstack(&[]).unwrap();
        let child = gui_text(&[Value::string("Child")]).unwrap();
        let result = gui_add_child(&[parent, child]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_text_bold() {
        let elem = gui_text(&[Value::string("Bold text")]).unwrap();
        let result = gui_set_text_bold(&[elem]);
        assert!(result.is_ok());

        // Verify bold was set
        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Text(config) = &gui_elem.kind {
                    assert!(config.bold);
                } else {
                    panic!("Expected Text element");
                }
            }
        }
    }

    // ==================== Slider Tests ====================

    #[test]
    fn test_gui_slider() {
        let result = gui_slider(&[Value::Float(0.0), Value::Float(100.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_slider_with_value() {
        let result = gui_slider(&[Value::Float(0.0), Value::Float(100.0), Value::Float(50.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_slider_with_step() {
        let result = gui_slider(&[
            Value::Float(0.0),
            Value::Float(100.0),
            Value::Float(50.0),
            Value::Float(5.0),
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_slider_value() {
        let elem = gui_slider(&[Value::Float(0.0), Value::Float(100.0)]).unwrap();
        let result = gui_set_slider_value(&[elem, Value::Float(25.0)]);
        assert!(result.is_ok());
    }

    // ==================== Toggle Tests ====================

    #[test]
    fn test_gui_toggle() {
        let result = gui_toggle(&[Value::string("Enable feature")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_toggle_with_state() {
        let result = gui_toggle(&[Value::string("Enable feature"), Value::Bool(true)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_toggle_on() {
        let elem = gui_toggle(&[Value::string("Feature")]).unwrap();
        let result = gui_set_toggle_on(&[elem, Value::Bool(true)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_toggle_label() {
        let elem = gui_toggle(&[Value::string("Feature")]).unwrap();
        let result = gui_set_toggle_label(&[elem, Value::string("New Label")]);
        assert!(result.is_ok());
    }

    // ==================== ProgressBar Tests ====================

    #[test]
    fn test_gui_progress_bar() {
        let result = gui_progress_bar(&[Value::Float(0.5)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_progress_bar_default() {
        let result = gui_progress_bar(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_progress() {
        let elem = gui_progress_bar(&[Value::Float(0.0)]).unwrap();
        let result = gui_set_progress(&[elem, Value::Float(0.75)]);
        assert!(result.is_ok());
    }

    // ==================== Image Tests ====================

    #[test]
    fn test_gui_image() {
        let result = gui_image(&[Value::string("test.png")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_image_with_content_fit() {
        let result = gui_image(&[Value::string("test.png"), Value::string("cover")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_image_path() {
        let elem = gui_image(&[Value::string("test.png")]).unwrap();
        let result = gui_set_image_path(&[elem, Value::string("new.png")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_content_fit() {
        let elem = gui_image(&[Value::string("test.png")]).unwrap();
        let result = gui_set_content_fit(&[elem, Value::string("contain")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_opacity() {
        let elem = gui_image(&[Value::string("test.png")]).unwrap();
        let result = gui_set_opacity(&[elem, Value::Float(0.5)]);
        assert!(result.is_ok());
    }

    // ==================== Existing Tests ====================

    #[test]
    fn test_gui_set_text_color() {
        let elem = gui_text(&[Value::string("Colored text")]).unwrap();
        let result = gui_set_text_color(&[elem, Value::Int(255), Value::Int(0), Value::Int(0)]);
        assert!(result.is_ok());

        // Verify color was set
        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Text(config) = &gui_elem.kind {
                    assert_eq!(config.color, Some((255, 0, 0, 255)));
                } else {
                    panic!("Expected Text element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_text_color_with_alpha() {
        let elem = gui_text(&[Value::string("Colored text")]).unwrap();
        let result = gui_set_text_color(&[
            elem,
            Value::Int(0),
            Value::Int(128),
            Value::Int(255),
            Value::Int(128),
        ]);
        assert!(result.is_ok());

        // Verify color with alpha was set
        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Text(config) = &gui_elem.kind {
                    assert_eq!(config.color, Some((0, 128, 255, 128)));
                } else {
                    panic!("Expected Text element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_text_size() {
        let elem = gui_text(&[Value::string("Sized text")]).unwrap();
        let result = gui_set_text_size(&[elem, Value::Float(32.0)]);
        assert!(result.is_ok());

        // Verify size was set
        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Text(config) = &gui_elem.kind {
                    assert_eq!(config.size, Some(32.0));
                } else {
                    panic!("Expected Text element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_disabled() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_disabled(&[elem, Value::Bool(true)]);
        assert!(result.is_ok());

        // Verify disabled was set
        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Button(config) = &gui_elem.kind {
                    assert!(config.disabled);
                } else {
                    panic!("Expected Button element");
                }
            }
        }
    }

    #[test]
    fn test_gui_button_with_disabled() {
        let result = gui_button(&[Value::string("Disabled"), Value::Int(1), Value::Bool(true)]);
        assert!(result.is_ok());

        // Verify disabled was set
        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Button(config) = &gui_elem.kind {
                    assert!(config.disabled);
                    assert!(config.on_click.is_some());
                } else {
                    panic!("Expected Button element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_text_bold_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_text_bold(&[elem]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Text elements"));
    }

    #[test]
    fn test_gui_set_disabled_wrong_type() {
        let elem = gui_text(&[Value::string("Text")]).unwrap();
        let result = gui_set_disabled(&[elem, Value::Bool(true)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Button elements"));
    }

    #[test]
    fn test_gui_text_field_empty() {
        let result = gui_text_field(&[]);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(matches!(value, Value::GuiElement(_)));
    }

    #[test]
    fn test_gui_text_field_with_value() {
        let result = gui_text_field(&[Value::string("hello")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.value, "hello");
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_text_field_with_placeholder() {
        let result = gui_text_field(&[Value::string(""), Value::string("Enter name")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.placeholder, "Enter name");
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_text_field_secure() {
        let result = gui_text_field(&[
            Value::string(""),
            Value::string("Password"),
            Value::Bool(true),
        ]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert!(config.secure);
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_placeholder() {
        let elem = gui_text_field(&[]).unwrap();
        let result = gui_set_placeholder(&[elem, Value::string("Search...")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.placeholder, "Search...");
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_secure() {
        let elem = gui_text_field(&[]).unwrap();
        let result = gui_set_secure(&[elem, Value::Bool(true)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert!(config.secure);
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_value() {
        let elem = gui_text_field(&[]).unwrap();
        let result = gui_set_value(&[elem, Value::string("new value")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.value, "new value");
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_bind_field() {
        let elem = gui_text_field(&[]).unwrap();
        let result = gui_bind_field(&[elem, Value::string("state.username")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.username".to_string()));
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_placeholder_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_placeholder(&[elem, Value::string("placeholder")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TextField elements"));
    }

    #[test]
    fn test_gui_set_secure_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_secure(&[elem, Value::Bool(true)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TextField elements"));
    }

    #[test]
    fn test_gui_checkbox() {
        let result = gui_checkbox(&[Value::string("Accept")]);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(matches!(value, Value::GuiElement(_)));
    }

    #[test]
    fn test_gui_checkbox_with_checked() {
        let result = gui_checkbox(&[Value::string("Remember me"), Value::Bool(true)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert_eq!(config.label, "Remember me");
                    assert!(config.checked);
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    #[test]
    fn test_gui_checkbox_with_callback() {
        let result = gui_checkbox(&[Value::string("Toggle"), Value::Bool(false), Value::Int(42)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert!(!config.checked);
                    assert!(config.on_toggle.is_some());
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_checked() {
        let elem = gui_checkbox(&[Value::string("Test")]).unwrap();
        let result = gui_set_checked(&[elem, Value::Bool(true)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert!(config.checked);
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_checked_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_checked(&[elem, Value::Bool(true)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Checkbox elements"));
    }

    #[test]
    fn test_gui_set_checkbox_label() {
        let elem = gui_checkbox(&[Value::string("Old label")]).unwrap();
        let result = gui_set_checkbox_label(&[elem, Value::string("New label")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert_eq!(config.label, "New label");
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_checkbox_label_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_checkbox_label(&[elem, Value::string("label")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Checkbox elements"));
    }

    #[test]
    fn test_gui_bind_field_checkbox() {
        let elem = gui_checkbox(&[Value::string("Remember")]).unwrap();
        let result = gui_bind_field(&[elem, Value::string("state.remember")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.remember".to_string()));
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    // RadioButton tests

    #[test]
    fn test_gui_radio_button() {
        let result = gui_radio_button(&[Value::string("Option A"), Value::string("a")]);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(matches!(value, Value::GuiElement(_)));
    }

    #[test]
    fn test_gui_radio_button_with_selected() {
        let result = gui_radio_button(&[
            Value::string("Option A"),
            Value::string("a"),
            Value::string("a"),
        ]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.label, "Option A");
                    assert_eq!(config.value, "a");
                    assert_eq!(config.selected_value, Some("a".to_string()));
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_radio_button_with_callback() {
        let result = gui_radio_button(&[
            Value::string("Option B"),
            Value::string("b"),
            Value::string("a"),
            Value::Int(42),
        ]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.value, "b");
                    assert!(config.on_select.is_some());
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_radio_value() {
        let elem = gui_radio_button(&[Value::string("Label"), Value::string("old")]).unwrap();
        let result = gui_set_radio_value(&[elem, Value::string("new")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.value, "new");
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_radio_value_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_radio_value(&[elem, Value::string("value")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("RadioButton elements"));
    }

    #[test]
    fn test_gui_set_radio_selected() {
        let elem = gui_radio_button(&[Value::string("Label"), Value::string("a")]).unwrap();
        let result = gui_set_radio_selected(&[elem, Value::string("b")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.selected_value, Some("b".to_string()));
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_radio_selected_wrong_type() {
        let elem = gui_checkbox(&[Value::string("Checkbox")]).unwrap();
        let result = gui_set_radio_selected(&[elem, Value::string("value")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("RadioButton elements"));
    }

    #[test]
    fn test_gui_set_radio_label() {
        let elem = gui_radio_button(&[Value::string("Old Label"), Value::string("a")]).unwrap();
        let result = gui_set_radio_label(&[elem, Value::string("New Label")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.label, "New Label");
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_radio_label_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let result = gui_set_radio_label(&[elem, Value::string("label")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("RadioButton elements"));
    }

    #[test]
    fn test_gui_bind_field_radio_button() {
        let elem = gui_radio_button(&[Value::string("Option"), Value::string("opt")]).unwrap();
        let result = gui_bind_field(&[elem, Value::string("state.selection")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.selection".to_string()));
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    // Dropdown tests

    #[test]
    fn test_gui_dropdown() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("Red"),
            Value::string("Green"),
            Value::string("Blue"),
        ])));
        let result = gui_dropdown(&[options]);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(matches!(value, Value::GuiElement(_)));
    }

    #[test]
    fn test_gui_dropdown_with_selected() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
            Value::string("B"),
        ])));
        let result = gui_dropdown(&[options, Value::string("B")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.selected, Some("B".to_string()));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_dropdown_with_placeholder() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("X"),
        ])));
        let result = gui_dropdown(&[options, Value::Null, Value::string("Choose...")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.placeholder, Some("Choose...".to_string()));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_dropdown_with_callback() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
        ])));
        let result = gui_dropdown(&[options, Value::Null, Value::Null, Value::Int(99)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert!(config.on_select.is_some());
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_dropdown_options() {
        let initial_options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("Old"),
        ])));
        let elem = gui_dropdown(&[initial_options]).unwrap();

        let new_options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("New1"),
            Value::string("New2"),
        ])));
        let result = gui_set_dropdown_options(&[elem, new_options]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.options, vec!["New1".to_string(), "New2".to_string()]);
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_dropdown_options_wrong_type() {
        let elem = gui_button(&[Value::string("Button")]).unwrap();
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
        ])));
        let result = gui_set_dropdown_options(&[elem, options]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Dropdown elements"));
    }

    #[test]
    fn test_gui_set_dropdown_selected() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
            Value::string("B"),
        ])));
        let elem = gui_dropdown(&[options]).unwrap();
        let result = gui_set_dropdown_selected(&[elem, Value::string("B")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.selected, Some("B".to_string()));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_dropdown_selected_null() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
        ])));
        let elem = gui_dropdown(&[options, Value::string("A")]).unwrap();
        let result = gui_set_dropdown_selected(&[elem, Value::Null]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert!(config.selected.is_none());
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_dropdown_selected_wrong_type() {
        let elem = gui_checkbox(&[Value::string("Check")]).unwrap();
        let result = gui_set_dropdown_selected(&[elem, Value::string("value")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Dropdown elements"));
    }

    #[test]
    fn test_gui_set_dropdown_placeholder() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
        ])));
        let elem = gui_dropdown(&[options]).unwrap();
        let result = gui_set_dropdown_placeholder(&[elem, Value::string("Select one...")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.placeholder, Some("Select one...".to_string()));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_dropdown_placeholder_wrong_type() {
        let elem = gui_text(&[Value::string("Text")]).unwrap();
        let result = gui_set_dropdown_placeholder(&[elem, Value::string("placeholder")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Dropdown elements"));
    }

    #[test]
    fn test_gui_bind_field_dropdown() {
        let options = Value::List(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::string("A"),
        ])));
        let elem = gui_dropdown(&[options]).unwrap();
        let result = gui_bind_field(&[elem, Value::string("state.color")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.color".to_string()));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    // ==================== Conditional Rendering Tests ====================

    #[test]
    fn test_gui_if_basic() {
        let true_elem = gui_text(&[Value::string("True branch")]).unwrap();
        let result = gui_if(&[Value::string("show_details"), true_elem]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                assert!(matches!(gui_elem.kind, GuiElementKind::Conditional(_)));
            } else {
                panic!("Failed to downcast to GuiElement");
            }
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_if_with_else() {
        let true_elem = gui_text(&[Value::string("True")]).unwrap();
        let false_elem = gui_text(&[Value::string("False")]).unwrap();
        let result = gui_if(&[Value::string("condition"), true_elem, false_elem]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_if_missing_args() {
        let result = gui_if(&[Value::string("condition")]);
        assert!(result.is_err());
    }

    // ==================== ForEach Tests ====================

    #[test]
    fn test_gui_for_each_basic() {
        let result = gui_for_each(&[Value::string("items")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::ForEach(config) = &gui_elem.kind {
                    assert_eq!(config.list_field, "items");
                } else {
                    panic!("Expected ForEach element");
                }
            } else {
                panic!("Failed to downcast to GuiElement");
            }
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_for_each_missing_args() {
        let result = gui_for_each(&[]);
        assert!(result.is_err());
    }

    // ==================== DataTable Tests ====================

    fn create_test_dataframe() -> Value {
        use stratum_core::data::{DataFrame, Series};
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
        let ages = Series::from_ints("age", vec![30, 25, 35]);
        let df = DataFrame::from_series(vec![names, ages]).unwrap();
        Value::DataFrame(Arc::new(df))
    }

    #[test]
    fn test_gui_data_table_basic() {
        let df = create_test_dataframe();
        let result = gui_data_table(&[df]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                assert!(matches!(gui_elem.kind, GuiElementKind::DataTable(_)));
            } else {
                panic!("Failed to downcast to GuiElement");
            }
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_data_table_with_page_size() {
        let df = create_test_dataframe();
        let result = gui_data_table(&[df, Value::Int(10)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_data_table_missing_args() {
        let result = gui_data_table(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gui_data_table_wrong_type() {
        let result = gui_data_table(&[Value::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gui_set_table_columns() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let columns = Value::list(vec![Value::string("name")]);
        let result = gui_set_table_columns(&[elem, columns]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_page_size() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_set_page_size(&[elem, Value::Int(25)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_current_page() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_set_current_page(&[elem, Value::Int(2)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_sortable() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_set_sortable(&[elem, Value::Bool(true)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_sort_by() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_set_sort_by(&[elem, Value::string("name"), Value::Bool(true)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_selectable() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_set_selectable(&[elem, Value::Bool(true)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_selected_rows() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let rows = Value::list(vec![Value::Int(0), Value::Int(2)]);
        let result = gui_set_selected_rows(&[elem, rows]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_column_width() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_set_column_width(&[elem, Value::string("name"), Value::Float(150.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_sort() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_on_sort(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_page_change() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_on_page_change(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_selection_change() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_on_selection_change(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_row_click() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_on_row_click(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_cell_click() {
        let df = create_test_dataframe();
        let elem = gui_data_table(&[df]).unwrap();
        let result = gui_on_cell_click(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_data_table_wrong_element() {
        let elem = gui_text(&[Value::string("Not a table")]).unwrap();
        let result = gui_set_sortable(&[elem, Value::Bool(true)]);
        assert!(result.is_err());
    }

    // Widget Styling Tests

    #[test]
    fn test_gui_set_background() {
        let elem = gui_button(&[Value::string("Test")]).unwrap();
        let result = gui_set_background(&[elem, Value::Int(255), Value::Int(128), Value::Int(64)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_background_with_alpha() {
        let elem = gui_button(&[Value::string("Test")]).unwrap();
        let result = gui_set_background(&[
            elem,
            Value::Int(255),
            Value::Int(128),
            Value::Int(64),
            Value::Int(200),
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_foreground() {
        let elem = gui_text(&[Value::string("Test")]).unwrap();
        let result = gui_set_foreground(&[elem, Value::Int(0), Value::Int(0), Value::Int(0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_border_color() {
        let elem = gui_button(&[Value::string("Test")]).unwrap();
        let result =
            gui_set_border_color(&[elem, Value::Int(100), Value::Int(100), Value::Int(100)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_border_width() {
        let elem = gui_button(&[Value::string("Test")]).unwrap();
        let result = gui_set_border_width(&[elem, Value::Float(2.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_corner_radius() {
        let elem = gui_button(&[Value::string("Test")]).unwrap();
        let result = gui_set_corner_radius(&[elem, Value::Float(8.0)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_theme_presets() {
        let result = gui_theme_presets(&[]);
        assert!(result.is_ok());
        if let Value::List(list) = result.unwrap() {
            let borrowed = list.borrow();
            assert!(!borrowed.is_empty());
            assert!(borrowed.len() >= 20); // At least 20 preset themes
        } else {
            panic!("Expected list of theme names");
        }
    }

    #[test]
    fn test_gui_set_theme_valid_preset() {
        // Clear any pending theme
        let _ = crate::bindings::take_pending_theme();

        let result = gui_set_theme(&[Value::string("dark")]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Null);

        // Verify theme was requested
        let pending = crate::bindings::take_pending_theme();
        assert!(pending.is_some());
        if let Some(crate::bindings::PendingTheme::Preset(preset)) = pending {
            assert_eq!(preset.name(), "dark");
        } else {
            panic!("Expected preset theme");
        }
    }

    #[test]
    fn test_gui_set_theme_invalid_preset() {
        let result = gui_set_theme(&[Value::string("nonexistent_theme")]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown theme preset"));
    }

    #[test]
    fn test_gui_set_theme_case_insensitive() {
        // Clear any pending theme
        let _ = crate::bindings::take_pending_theme();

        // Should work with various casings
        let result = gui_set_theme(&[Value::string("DRACULA")]);
        assert!(result.is_ok());

        let pending = crate::bindings::take_pending_theme();
        assert!(pending.is_some());
        if let Some(crate::bindings::PendingTheme::Preset(preset)) = pending {
            assert_eq!(preset.name(), "dracula");
        }
    }

    #[test]
    fn test_gui_custom_theme_with_struct() {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;
        use stratum_core::bytecode::StructInstance;

        // Clear any pending theme
        let _ = crate::bindings::take_pending_theme();

        // Create a palette struct with color values as lists
        let mut palette_fields = HashMap::new();
        palette_fields.insert(
            "background".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(40),
                Value::Int(44),
                Value::Int(52),
            ]))),
        );
        palette_fields.insert(
            "text".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(255),
                Value::Int(255),
                Value::Int(255),
            ]))),
        );
        palette_fields.insert(
            "primary".to_string(),
            Value::String(Rc::new("#61AFEF".to_string())),
        );
        palette_fields.insert(
            "success".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(152),
                Value::Int(195),
                Value::Int(121),
            ]))),
        );
        palette_fields.insert(
            "warning".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(229),
                Value::Int(192),
                Value::Int(123),
            ]))),
        );
        palette_fields.insert(
            "danger".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(224),
                Value::Int(108),
                Value::Int(117),
            ]))),
        );

        let palette_instance = StructInstance {
            type_name: "Palette".to_string(),
            fields: palette_fields,
        };
        let palette_value = Value::Struct(Rc::new(RefCell::new(palette_instance)));

        let result = gui_custom_theme(&[Value::string("OneDark"), palette_value]);
        assert!(result.is_ok());

        let pending = crate::bindings::take_pending_theme();
        assert!(pending.is_some());
        if let Some(crate::bindings::PendingTheme::Custom { name, palette }) = pending {
            assert_eq!(name, "OneDark");
            assert_eq!(palette.background.r, 40);
            assert_eq!(palette.text.r, 255);
            assert_eq!(palette.primary.r, 97); // #61 = 97
        } else {
            panic!("Expected custom theme");
        }
    }

    #[test]
    fn test_gui_custom_theme_missing_field() {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;
        use stratum_core::bytecode::StructInstance;

        // Create incomplete palette struct (missing 'danger' field)
        let mut palette_fields = HashMap::new();
        palette_fields.insert(
            "background".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(40),
                Value::Int(44),
                Value::Int(52),
            ]))),
        );
        palette_fields.insert(
            "text".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(255),
                Value::Int(255),
                Value::Int(255),
            ]))),
        );
        palette_fields.insert(
            "primary".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(97),
                Value::Int(175),
                Value::Int(239),
            ]))),
        );
        palette_fields.insert(
            "success".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(152),
                Value::Int(195),
                Value::Int(121),
            ]))),
        );
        palette_fields.insert(
            "warning".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::Int(229),
                Value::Int(192),
                Value::Int(123),
            ]))),
        );
        // 'danger' field intentionally missing

        let palette_instance = StructInstance {
            type_name: "Palette".to_string(),
            fields: palette_fields,
        };
        let palette_value = Value::Struct(Rc::new(RefCell::new(palette_instance)));

        let result = gui_custom_theme(&[Value::string("Incomplete"), palette_value]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("missing required field"));
        assert!(err.contains("danger"));
    }

    #[test]
    fn test_gui_set_background_missing_args() {
        let elem = gui_button(&[Value::string("Test")]).unwrap();
        let result = gui_set_background(&[elem, Value::Int(255)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gui_set_border_width_missing_args() {
        let result = gui_set_border_width(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_gui_set_corner_radius_missing_args() {
        let result = gui_set_corner_radius(&[]);
        assert!(result.is_err());
    }

    // =========================================================================
    // OLAP Cube Widget Tests
    // =========================================================================

    fn create_test_cube() -> Value {
        use stratum_core::data::{Cube, CubeAggFunc, DataFrame, Series};
        // Create a simple test DataFrame for the cube
        let regions = Series::from_strings("region", vec!["North", "South", "North", "South"]);
        let products = Series::from_strings("product", vec!["A", "A", "B", "B"]);
        let revenue = Series::from_floats("revenue", vec![100.0, 150.0, 200.0, 250.0]);
        let units = Series::from_ints("units", vec![10, 15, 20, 25]);

        let df = DataFrame::from_series(vec![regions, products, revenue, units]).unwrap();

        // Build a cube from the DataFrame
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .dimension("product")
            .unwrap()
            .measure("revenue", CubeAggFunc::Sum)
            .unwrap()
            .measure("units", CubeAggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        Value::Cube(Arc::new(cube))
    }

    #[test]
    fn test_gui_cube_table_basic() {
        let result = gui_cube_table(&[]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                assert!(matches!(gui_elem.kind, GuiElementKind::CubeTable(_)));
            } else {
                panic!("Failed to downcast to GuiElement");
            }
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_cube_table_with_cube() {
        let cube = create_test_cube();
        let result = gui_cube_table(&[cube]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::CubeTable(config) = &gui_elem.kind {
                    assert!(config.cube.is_some());
                } else {
                    panic!("Expected CubeTable element");
                }
            }
        }
    }

    #[test]
    fn test_gui_cube_chart_basic() {
        let result = gui_cube_chart(&[]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                assert!(matches!(gui_elem.kind, GuiElementKind::CubeChart(_)));
            } else {
                panic!("Failed to downcast to GuiElement");
            }
        } else {
            panic!("Expected GuiElement");
        }
    }

    #[test]
    fn test_gui_cube_chart_with_type() {
        let cube = create_test_cube();
        let result = gui_cube_chart(&[cube, Value::string("line")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::CubeChart(config) = &gui_elem.kind {
                    assert!(matches!(
                        config.chart_type,
                        crate::element::CubeChartType::Line
                    ));
                } else {
                    panic!("Expected CubeChart element");
                }
            }
        }
    }

    #[test]
    fn test_gui_dimension_filter_basic() {
        let cube = create_test_cube();
        let result = gui_dimension_filter(&[cube, Value::string("region")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::DimensionFilter(config) = &gui_elem.kind {
                    assert_eq!(config.dimension, "region");
                    assert!(config.cube.is_some());
                } else {
                    panic!("Expected DimensionFilter element");
                }
            }
        }
    }

    #[test]
    fn test_gui_hierarchy_navigator_basic() {
        let cube = create_test_cube();
        let result = gui_hierarchy_navigator(&[cube, Value::string("time")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::HierarchyNavigator(config) = &gui_elem.kind {
                    assert_eq!(config.hierarchy, "time");
                } else {
                    panic!("Expected HierarchyNavigator element");
                }
            }
        }
    }

    #[test]
    fn test_gui_measure_selector_basic() {
        let cube = create_test_cube();
        let result = gui_measure_selector(&[cube]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::MeasureSelector(config) = &gui_elem.kind {
                    assert!(config.cube.is_some());
                } else {
                    panic!("Expected MeasureSelector element");
                }
            }
        }
    }

    #[test]
    fn test_gui_set_cube() {
        let elem = gui_cube_table(&[]).unwrap();
        let cube = create_test_cube();
        let result = gui_set_cube(&[elem, cube]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_row_dimensions() {
        let cube = create_test_cube();
        let elem = gui_cube_table(&[cube]).unwrap();
        let dims = Value::list(vec![Value::string("region"), Value::string("product")]);
        let result = gui_set_row_dimensions(&[elem, dims]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_measures_cube_table() {
        let cube = create_test_cube();
        let elem = gui_cube_table(&[cube]).unwrap();
        let measures = Value::list(vec![Value::string("revenue"), Value::string("units")]);
        let result = gui_set_measures(&[elem, measures]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_cube_chart_type() {
        let cube = create_test_cube();
        let elem = gui_cube_chart(&[cube]).unwrap();
        let result = gui_set_cube_chart_type(&[elem, Value::string("pie")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_x_dimension() {
        let cube = create_test_cube();
        let elem = gui_cube_chart(&[cube]).unwrap();
        let result = gui_set_x_dimension(&[elem, Value::string("region")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_y_measure() {
        let cube = create_test_cube();
        let elem = gui_cube_chart(&[cube]).unwrap();
        let result = gui_set_y_measure(&[elem, Value::string("revenue")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_series_dimension() {
        let cube = create_test_cube();
        let elem = gui_cube_chart(&[cube]).unwrap();
        let result = gui_set_series_dimension(&[elem, Value::string("product")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_filter_dimension() {
        let cube = create_test_cube();
        let elem = gui_dimension_filter(&[cube, Value::string("region")]).unwrap();
        let result = gui_set_filter_dimension(&[elem, Value::string("product")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_hierarchy() {
        let cube = create_test_cube();
        let elem = gui_hierarchy_navigator(&[cube, Value::string("time")]).unwrap();
        let result = gui_set_hierarchy(&[elem, Value::string("location")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_set_current_level() {
        let cube = create_test_cube();
        let elem = gui_hierarchy_navigator(&[cube, Value::string("time")]).unwrap();
        let result = gui_set_current_level(&[elem, Value::string("quarter")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_drill() {
        let cube = create_test_cube();
        let elem = gui_cube_table(&[cube]).unwrap();
        let result = gui_on_drill(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_roll_up() {
        let cube = create_test_cube();
        let elem = gui_cube_table(&[cube]).unwrap();
        let result = gui_on_roll_up(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gui_on_level_change() {
        let cube = create_test_cube();
        let elem = gui_hierarchy_navigator(&[cube, Value::string("time")]).unwrap();
        let result = gui_on_level_change(&[elem, Value::Int(1)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cube_widget_wrong_element_type() {
        // Test that cube-specific functions fail on non-cube elements
        let text_elem = gui_text(&[Value::string("Not a cube widget")]).unwrap();

        let result = gui_set_row_dimensions(&[text_elem.clone(), Value::list(vec![])]);
        assert!(result.is_err());

        let result = gui_set_x_dimension(&[text_elem.clone(), Value::string("dim")]);
        assert!(result.is_err());

        let result = gui_set_filter_dimension(&[text_elem.clone(), Value::string("dim")]);
        assert!(result.is_err());

        let result = gui_set_hierarchy(&[text_elem, Value::string("hier")]);
        assert!(result.is_err());
    }

    // ==================== Alignment Tests ====================

    #[test]
    fn test_gui_set_alignment_vstack() {
        let vstack = gui_vstack(&[]).unwrap();
        let result = gui_set_alignment(&[vstack, Value::string("left"), Value::string("top")]);
        assert!(result.is_ok());
        if let Value::GuiElement(e) = result.unwrap() {
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::VStack(c) = &elem.kind {
                    assert_eq!(c.align, HAlign::Start);
                }
            }
        }
    }

    #[test]
    fn test_gui_set_alignment_hstack() {
        let hstack = gui_hstack(&[]).unwrap();
        let result = gui_set_alignment(&[hstack, Value::string("center"), Value::string("bottom")]);
        assert!(result.is_ok());
        if let Value::GuiElement(e) = result.unwrap() {
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::HStack(c) = &elem.kind {
                    assert_eq!(c.align, VAlign::Bottom);
                }
            }
        }
    }

    #[test]
    fn test_gui_set_alignment_grid() {
        let grid = gui_grid(&[Value::Int(3)]).unwrap();
        let result = gui_set_alignment(&[grid, Value::string("end"), Value::string("center")]);
        assert!(result.is_ok());
        if let Value::GuiElement(e) = result.unwrap() {
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Grid(c) = &elem.kind {
                    assert_eq!(c.cell_align_x, HAlign::End);
                    assert_eq!(c.cell_align_y, VAlign::Center);
                }
            }
        }
    }

    #[test]
    fn test_gui_set_alignment_container() {
        let container = gui_container(&[]).unwrap();
        let result = gui_set_alignment(&[container, Value::string("right"), Value::string("top")]);
        assert!(result.is_ok());
        if let Value::GuiElement(e) = result.unwrap() {
            if let Some(elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Container(c) = &elem.kind {
                    assert_eq!(c.align_x, HAlign::End);
                    assert_eq!(c.align_y, VAlign::Top);
                }
            }
        }
    }

    #[test]
    fn test_gui_set_alignment_invalid_element() {
        let text = gui_text(&[Value::string("Hello")]).unwrap();
        let result = gui_set_alignment(&[text, Value::string("center"), Value::string("center")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("can only be set on"));
    }

    #[test]
    fn test_gui_set_alignment_invalid_h_align() {
        let vstack = gui_vstack(&[]).unwrap();
        let result =
            gui_set_alignment(&[vstack, Value::string("invalid"), Value::string("center")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid horizontal alignment"));
    }

    #[test]
    fn test_gui_set_alignment_invalid_v_align() {
        let vstack = gui_vstack(&[]).unwrap();
        let result =
            gui_set_alignment(&[vstack, Value::string("center"), Value::string("invalid")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid vertical alignment"));
    }

    // ==================== Form Event Handler Tests ====================

    #[test]
    fn test_gui_on_change_text_field() {
        let elem = gui_text_field(&[]).unwrap();
        let result = gui_on_change(&[elem, Value::Int(42)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.on_change, Some(CallbackId::new(42)));
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_change_slider() {
        let elem = gui_slider(&[Value::Float(0.0), Value::Float(100.0)]).unwrap();
        let result = gui_on_change(&[elem, Value::Int(99)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Slider(config) = &gui_elem.kind {
                    assert_eq!(config.on_change, Some(CallbackId::new(99)));
                } else {
                    panic!("Expected Slider element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_change_wrong_element() {
        let elem = gui_button(&[Value::string("Click")]).unwrap();
        let result = gui_on_change(&[elem, Value::Int(1)]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("can only be applied to TextField"));
    }

    #[test]
    fn test_gui_on_submit_text_field() {
        let elem = gui_text_field(&[]).unwrap();
        let result = gui_on_submit(&[elem, Value::Int(55)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.on_submit, Some(CallbackId::new(55)));
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_submit_wrong_element() {
        let elem = gui_button(&[Value::string("Click")]).unwrap();
        let result = gui_on_submit(&[elem, Value::Int(1)]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("can only be applied to TextField"));
    }

    #[test]
    fn test_gui_on_toggle_checkbox() {
        let elem = gui_checkbox(&[Value::string("Check me")]).unwrap();
        let result = gui_on_toggle(&[elem, Value::Int(77)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert_eq!(config.on_toggle, Some(CallbackId::new(77)));
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_toggle_toggle() {
        let elem = gui_toggle(&[Value::string("Enable")]).unwrap();
        let result = gui_on_toggle(&[elem, Value::Int(88)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Toggle(config) = &gui_elem.kind {
                    assert_eq!(config.on_toggle, Some(CallbackId::new(88)));
                } else {
                    panic!("Expected Toggle element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_toggle_wrong_element() {
        let elem = gui_text(&[Value::string("Text")]).unwrap();
        let result = gui_on_toggle(&[elem, Value::Int(1)]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("can only be applied to Checkbox or Toggle"));
    }

    #[test]
    fn test_gui_on_select_dropdown() {
        let options = Value::list(vec![Value::string("A"), Value::string("B")]);
        let elem = gui_dropdown(&[options]).unwrap();
        let result = gui_on_select(&[elem, Value::Int(33)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.on_select, Some(CallbackId::new(33)));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_select_radio_button() {
        let elem = gui_radio_button(&[Value::string("Option"), Value::string("a")]).unwrap();
        let result = gui_on_select(&[elem, Value::Int(44)]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.on_select, Some(CallbackId::new(44)));
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_on_select_wrong_element() {
        let elem = gui_text(&[Value::string("Text")]).unwrap();
        let result = gui_on_select(&[elem, Value::Int(1)]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("can only be applied to RadioButton, Dropdown"));
    }

    // ==================== State Binding Tests ====================

    #[test]
    fn test_gui_text_field_with_state_binding() {
        // Test that TextField accepts a StateBinding and sets field_path
        let binding = Value::StateBinding("state.name".to_string());
        let result = gui_text_field(&[binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.name".to_string()));
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_text_field_with_string_value() {
        // Test that TextField still accepts a string value
        let result = gui_text_field(&[Value::string("initial value")]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(config.value, "initial value");
                    assert_eq!(config.field_path, None);
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }

    #[test]
    fn test_gui_checkbox_with_state_binding() {
        // Test that Checkbox accepts a StateBinding and sets field_path
        let binding = Value::StateBinding("state.agreed".to_string());
        let result = gui_checkbox(&[Value::string("I agree"), binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Checkbox(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.agreed".to_string()));
                } else {
                    panic!("Expected Checkbox element");
                }
            }
        }
    }

    #[test]
    fn test_gui_slider_with_state_binding() {
        // Test that Slider accepts a StateBinding and sets field_path
        let binding = Value::StateBinding("state.volume".to_string());
        let result = gui_slider(&[Value::Float(0.0), Value::Float(100.0), binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Slider(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.volume".to_string()));
                } else {
                    panic!("Expected Slider element");
                }
            }
        }
    }

    #[test]
    fn test_gui_dropdown_with_state_binding() {
        // Test that Dropdown accepts a StateBinding and sets field_path
        let options = Value::list(vec![
            Value::string("Red"),
            Value::string("Green"),
            Value::string("Blue"),
        ]);
        let binding = Value::StateBinding("state.color".to_string());
        let result = gui_dropdown(&[options, binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Dropdown(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.color".to_string()));
                } else {
                    panic!("Expected Dropdown element");
                }
            }
        }
    }

    #[test]
    fn test_gui_radio_button_with_state_binding() {
        // Test that RadioButton accepts a StateBinding and sets field_path
        let binding = Value::StateBinding("state.size".to_string());
        let result = gui_radio_button(&[Value::string("Small"), Value::string("small"), binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::RadioButton(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.size".to_string()));
                } else {
                    panic!("Expected RadioButton element");
                }
            }
        }
    }

    #[test]
    fn test_gui_toggle_with_state_binding() {
        // Test that Toggle accepts a StateBinding and sets field_path
        let binding = Value::StateBinding("state.enabled".to_string());
        let result = gui_toggle(&[Value::string("Enable"), binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::Toggle(config) = &gui_elem.kind {
                    assert_eq!(config.field_path, Some("state.enabled".to_string()));
                } else {
                    panic!("Expected Toggle element");
                }
            }
        }
    }

    #[test]
    fn test_nested_state_binding_path() {
        // Test that nested paths like "state.user.profile.name" are preserved
        let binding = Value::StateBinding("state.user.profile.name".to_string());
        let result = gui_text_field(&[binding]);
        assert!(result.is_ok());

        let value = result.unwrap();
        if let Value::GuiElement(e) = value {
            if let Some(gui_elem) = e.as_any().downcast_ref::<GuiElement>() {
                if let GuiElementKind::TextField(config) = &gui_elem.kind {
                    assert_eq!(
                        config.field_path,
                        Some("state.user.profile.name".to_string())
                    );
                } else {
                    panic!("Expected TextField element");
                }
            }
        }
    }
}
