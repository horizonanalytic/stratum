//! Data Explorer Panel
//!
//! A dedicated panel for exploring variables, DataFrames, and Cubes.
//! Similar to MATLAB's Workspace Explorer or Python debugger variable views.
//!
//! Implements Phase 6.11 of the Workshop IDE.

use iced::widget::{
    button, column, container, row, rule, scrollable, text, text_input, Column, Space,
};
use iced::{Element, Font, Length, Theme};
use std::collections::HashMap;
use stratum_core::bytecode::Value;

/// Messages for the Data Explorer panel
#[derive(Debug, Clone)]
pub enum DataExplorerMessage {
    /// Filter text changed
    FilterChanged(String),
    /// Refresh variables from source
    Refresh,
    /// Select a variable for detailed inspection
    SelectVariable(String),
    /// Toggle expansion of a tree node
    ToggleExpand(String),
    /// Double-click to print value in REPL
    PrintVariable(String),
    /// Context menu action: copy value
    CopyValue(String),
    /// Context menu action: view full value
    ViewFull(String),
    /// Show context menu at position
    ShowContextMenu(String, f32, f32),
    /// Close context menu
    CloseContextMenu,
}

/// A variable entry for display
#[derive(Debug, Clone)]
pub struct VariableEntry {
    /// Variable name
    pub name: String,
    /// Type name
    pub type_name: String,
    /// Summary value (short preview)
    pub summary: String,
    /// Full value (for detailed view) - kept for future full value inspection API
    #[allow(dead_code)]
    pub value: Value,
    /// Depth in tree (0 = top level)
    pub depth: usize,
    /// Path to this variable (for nested values)
    pub path: String,
    /// Whether this is expandable (struct, list, map)
    pub expandable: bool,
    /// Whether this is currently expanded
    pub expanded: bool,
}

/// DataFrame column information
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    /// Null count - part of column statistics API for future use
    #[allow(dead_code)]
    pub null_count: usize,
}

/// DataFrame inspection data
#[derive(Debug, Clone)]
pub struct DataFrameInfo {
    pub num_rows: usize,
    pub num_columns: usize,
    pub columns: Vec<ColumnInfo>,
    pub preview_rows: Vec<Vec<String>>,
}

/// Cube inspection data
#[derive(Debug, Clone)]
pub struct CubeInfo {
    pub name: String,
    pub dimensions: Vec<String>,
    pub measures: Vec<String>,
}

/// Context menu state - fields used for positioning context menu (feature not yet fully implemented)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ContextMenuState {
    pub variable_path: String,
    pub x: f32,
    pub y: f32,
}

/// Data Explorer panel for inspecting variables
#[derive(Debug)]
pub struct DataExplorerPanel {
    /// All variable entries (flattened tree)
    variables: Vec<VariableEntry>,
    /// Currently selected variable path
    selected: Option<String>,
    /// Expanded paths
    expanded_paths: std::collections::HashSet<String>,
    /// Filter text
    filter: String,
    /// Context menu state
    context_menu: Option<ContextMenuState>,
    /// DataFrame info for selected variable (if it's a DataFrame)
    dataframe_info: Option<DataFrameInfo>,
    /// Cube info for selected variable (if it's a Cube)
    cube_info: Option<CubeInfo>,
}

impl Default for DataExplorerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl DataExplorerPanel {
    /// Create a new Data Explorer panel
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            selected: None,
            expanded_paths: std::collections::HashSet::new(),
            filter: String::new(),
            context_menu: None,
            dataframe_info: None,
            cube_info: None,
        }
    }

    /// Update variables from a globals HashMap (from REPL VM)
    pub fn update_from_globals(&mut self, globals: &HashMap<String, Value>) {
        self.variables.clear();
        self.dataframe_info = None;
        self.cube_info = None;

        // Filter out internal variables (starting with __)
        let mut names: Vec<_> = globals
            .keys()
            .filter(|name| !name.starts_with("__"))
            .filter(|name| !is_builtin_name(name))
            .cloned()
            .collect();
        names.sort();

        for name in names {
            if let Some(value) = globals.get(&name) {
                self.add_variable_entry(&name, &name, value, 0);
            }
        }

        // Update selected variable info if still valid
        if let Some(ref _path) = self.selected {
            self.update_selected_info(globals);
        }
    }

    /// Add a variable entry and its children if expanded
    fn add_variable_entry(&mut self, name: &str, path: &str, value: &Value, depth: usize) {
        let type_name = value.type_name().to_string();
        let summary = self.value_summary(value);
        let expandable = self.is_expandable(value);
        let expanded = self.expanded_paths.contains(path);

        self.variables.push(VariableEntry {
            name: name.to_string(),
            type_name,
            summary,
            value: value.clone(),
            depth,
            path: path.to_string(),
            expandable,
            expanded,
        });

        // If expanded, add children
        if expanded {
            self.add_children(path, value, depth + 1);
        }
    }

    /// Add children of an expandable value
    fn add_children(&mut self, parent_path: &str, value: &Value, depth: usize) {
        match value {
            Value::List(list) => {
                let items = list.borrow();
                for (i, item) in items.iter().enumerate().take(100) {
                    let name = format!("[{i}]");
                    let path = format!("{parent_path}[{i}]");
                    self.add_variable_entry(&name, &path, item, depth);
                }
                if items.len() > 100 {
                    self.variables.push(VariableEntry {
                        name: format!("... ({} more)", items.len() - 100),
                        type_name: String::new(),
                        summary: String::new(),
                        value: Value::Null,
                        depth,
                        path: format!("{parent_path}[...]"),
                        expandable: false,
                        expanded: false,
                    });
                }
            }
            Value::Map(map) => {
                let entries = map.borrow();
                let mut keys: Vec<_> = entries.keys().collect();
                keys.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
                for (_i, key) in keys.iter().enumerate().take(100) {
                    if let Some(val) = entries.get(*key) {
                        let key_str = format!("{key:?}");
                        let path = format!("{parent_path}.{key_str}");
                        self.add_variable_entry(&key_str, &path, val, depth);
                    }
                }
                if entries.len() > 100 {
                    self.variables.push(VariableEntry {
                        name: format!("... ({} more)", entries.len() - 100),
                        type_name: String::new(),
                        summary: String::new(),
                        value: Value::Null,
                        depth,
                        path: format!("{parent_path}..."),
                        expandable: false,
                        expanded: false,
                    });
                }
            }
            Value::Struct(instance) => {
                let fields = instance.borrow();
                let mut field_names: Vec<_> = fields.fields.keys().cloned().collect();
                field_names.sort();
                for field_name in field_names {
                    if let Some(field_value) = fields.fields.get(&field_name) {
                        let path = format!("{parent_path}.{field_name}");
                        self.add_variable_entry(&field_name, &path, field_value, depth);
                    }
                }
            }
            _ => {}
        }
    }

    /// Check if a value is expandable
    fn is_expandable(&self, value: &Value) -> bool {
        matches!(value, Value::List(_) | Value::Map(_) | Value::Struct(_))
    }

    /// Get a short summary of a value
    fn value_summary(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f:.6}"),
            Value::String(s) => {
                if s.len() > 50 {
                    format!("\"{}...\"", &s[..47])
                } else {
                    format!("\"{s}\"")
                }
            }
            Value::List(list) => {
                let len = list.borrow().len();
                format!("[{len} items]")
            }
            Value::Map(map) => {
                let len = map.borrow().len();
                format!("{{{len} entries}}")
            }
            Value::Struct(instance) => {
                let fields = instance.borrow();
                format!("{} {{{} fields}}", fields.type_name, fields.fields.len())
            }
            Value::DataFrame(df) => {
                format!("DataFrame [{} x {}]", df.num_columns(), df.num_rows())
            }
            Value::Series(series) => {
                format!("Series '{}' [{}]", series.name(), series.len())
            }
            Value::Cube(cube) => {
                let name = cube.name().unwrap_or("unnamed");
                format!(
                    "Cube '{}' [{}d x {}m]",
                    name,
                    cube.dimension_names().len(),
                    cube.measure_names().len()
                )
            }
            Value::Function(_) => "<function>".to_string(),
            Value::Closure(_) => "<closure>".to_string(),
            Value::NativeFunction(_) => "<native fn>".to_string(),
            Value::Range(r) => format!("{}..{}", r.start, r.end),
            _ => format!("<{}>", value.type_name()),
        }
    }

    /// Update detailed info for selected variable
    fn update_selected_info(&mut self, globals: &HashMap<String, Value>) {
        self.dataframe_info = None;
        self.cube_info = None;

        let Some(ref selected_path) = self.selected else {
            return;
        };

        // Find the value at the selected path
        let value = self.find_value_at_path(selected_path, globals);
        let Some(value) = value else {
            return;
        };

        match &value {
            Value::DataFrame(df) => {
                let schema = df.schema();
                let columns: Vec<ColumnInfo> = schema
                    .fields()
                    .iter()
                    .map(|field| ColumnInfo {
                        name: field.name().clone(),
                        data_type: format!("{:?}", field.data_type()),
                        null_count: 0, // Would need to compute from data
                    })
                    .collect();

                // Get preview rows (first 10)
                let mut preview_rows = Vec::new();
                if let Ok(head_df) = df.head(10) {
                    for row_idx in 0..head_df.num_rows() {
                        let mut row_values = Vec::new();
                        for col_idx in 0..head_df.num_columns() {
                            if let Ok(series) = head_df.column_by_index(col_idx) {
                                let val_str = series
                                    .get(row_idx)
                                    .map(|v| format!("{v}"))
                                    .unwrap_or_else(|_| "null".to_string());
                                row_values.push(val_str);
                            }
                        }
                        preview_rows.push(row_values);
                    }
                }

                self.dataframe_info = Some(DataFrameInfo {
                    num_rows: df.num_rows(),
                    num_columns: df.num_columns(),
                    columns,
                    preview_rows,
                });
            }
            Value::Cube(cube) => {
                self.cube_info = Some(CubeInfo {
                    name: cube.name().unwrap_or("unnamed").to_string(),
                    dimensions: cube.dimension_names(),
                    measures: cube.measure_names(),
                });
            }
            _ => {}
        }
    }

    /// Find a value at a given path in globals
    fn find_value_at_path(&self, path: &str, globals: &HashMap<String, Value>) -> Option<Value> {
        // Simple case: top-level variable
        if !path.contains('.') && !path.contains('[') {
            return globals.get(path).cloned();
        }

        // Parse path and navigate
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let root_name = parts[0].split('[').next()?;
        let mut current = globals.get(root_name)?.clone();

        // Navigate through path
        for part in &parts[1..] {
            current = self.navigate_path_part(&current, part)?;
        }

        Some(current)
    }

    /// Navigate one part of a path
    fn navigate_path_part(&self, value: &Value, part: &str) -> Option<Value> {
        // Handle array index
        if part.starts_with('[') && part.ends_with(']') {
            let idx_str = &part[1..part.len() - 1];
            let idx: usize = idx_str.parse().ok()?;
            if let Value::List(list) = value {
                return list.borrow().get(idx).cloned();
            }
        }

        // Handle struct field
        if let Value::Struct(instance) = value {
            return instance.borrow().fields.get(part).cloned();
        }

        None
    }

    /// Handle a message
    pub fn update(&mut self, message: DataExplorerMessage) -> Option<DataExplorerAction> {
        match message {
            DataExplorerMessage::FilterChanged(filter) => {
                self.filter = filter;
                None
            }
            DataExplorerMessage::Refresh => Some(DataExplorerAction::RequestRefresh),
            DataExplorerMessage::SelectVariable(path) => {
                self.selected = Some(path);
                Some(DataExplorerAction::RequestRefresh)
            }
            DataExplorerMessage::ToggleExpand(path) => {
                if self.expanded_paths.contains(&path) {
                    self.expanded_paths.remove(&path);
                } else {
                    self.expanded_paths.insert(path);
                }
                Some(DataExplorerAction::RequestRefresh)
            }
            DataExplorerMessage::PrintVariable(path) => {
                Some(DataExplorerAction::PrintInRepl(path))
            }
            DataExplorerMessage::CopyValue(path) => Some(DataExplorerAction::CopyToClipboard(path)),
            DataExplorerMessage::ViewFull(path) => {
                self.selected = Some(path);
                self.context_menu = None;
                Some(DataExplorerAction::RequestRefresh)
            }
            DataExplorerMessage::ShowContextMenu(path, x, y) => {
                self.context_menu = Some(ContextMenuState {
                    variable_path: path,
                    x,
                    y,
                });
                None
            }
            DataExplorerMessage::CloseContextMenu => {
                self.context_menu = None;
                None
            }
        }
    }

    /// Render the panel
    pub fn view(&self) -> Element<'_, DataExplorerMessage> {
        let header = self.render_header();
        let variable_list = self.render_variable_list();
        let detail_view = self.render_detail_view();

        let content = column![header, rule::horizontal(1), variable_list, rule::horizontal(1), detail_view]
            .spacing(4)
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.base.color.into()),
                    ..Default::default()
                }
            })
            .into()
    }

    /// Render the header with filter and refresh button
    fn render_header(&self) -> Element<'_, DataExplorerMessage> {
        let title = text("Data Explorer").size(14);

        let filter_input = text_input("Filter...", &self.filter)
            .on_input(DataExplorerMessage::FilterChanged)
            .size(11)
            .padding(4)
            .width(Length::Fill);

        let refresh_btn = button(text("Refresh").size(11))
            .on_press(DataExplorerMessage::Refresh)
            .padding([4, 8])
            .style(button::secondary);

        column![
            title,
            row![filter_input, refresh_btn].spacing(4).align_y(iced::Alignment::Center)
        ]
        .spacing(4)
        .into()
    }

    /// Render the variable list
    fn render_variable_list(&self) -> Element<'_, DataExplorerMessage> {
        if self.variables.is_empty() {
            return container(
                text("No variables defined.\nUse the REPL to create variables.")
                    .size(11)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            )
            .padding(8)
            .width(Length::Fill)
            .height(Length::FillPortion(2))
            .into();
        }

        let filter_lower = self.filter.to_lowercase();
        let items: Vec<Element<'_, DataExplorerMessage>> = self
            .variables
            .iter()
            .filter(|v| {
                if self.filter.is_empty() {
                    true
                } else {
                    v.name.to_lowercase().contains(&filter_lower)
                        || v.type_name.to_lowercase().contains(&filter_lower)
                }
            })
            .map(|v| self.render_variable_row(v))
            .collect();

        scrollable(Column::with_children(items).spacing(1))
            .height(Length::FillPortion(2))
            .width(Length::Fill)
            .into()
    }

    /// Render a single variable row
    fn render_variable_row<'a>(&'a self, var: &'a VariableEntry) -> Element<'a, DataExplorerMessage> {
        let indent = Space::new().width(Length::Fixed((var.depth * 16) as f32));

        let expand_icon: Element<'a, DataExplorerMessage> = if var.expandable {
            let icon = if var.expanded { "v" } else { ">" };
            button(text(icon).size(10).font(Font::MONOSPACE))
                .on_press(DataExplorerMessage::ToggleExpand(var.path.clone()))
                .padding([2, 4])
                .style(button::text)
                .into()
        } else {
            button(text(" ").size(10))
                .padding([2, 4])
                .style(button::text)
                .into()
        };

        let is_selected = self.selected.as_ref() == Some(&var.path);
        let name_color = if is_selected {
            iced::Color::from_rgb(0.4, 0.8, 1.0)
        } else {
            iced::Color::from_rgb(0.7, 0.9, 1.0)
        };
        let type_color = iced::Color::from_rgb(0.6, 0.7, 0.6);
        let value_color = iced::Color::from_rgb(0.9, 0.8, 0.6);

        let name_text = text(&var.name).size(11).font(Font::MONOSPACE).color(name_color);
        let type_text = text(format!(": {}", var.type_name))
            .size(10)
            .font(Font::MONOSPACE)
            .color(type_color);
        let value_text = text(format!(" = {}", var.summary))
            .size(10)
            .font(Font::MONOSPACE)
            .color(value_color);

        let row_content: Element<'a, DataExplorerMessage> =
            row![indent, expand_icon, name_text, type_text, value_text]
                .spacing(2)
                .align_y(iced::Alignment::Center)
                .into();

        // Wrap in button for selection
        button(row_content)
            .on_press(DataExplorerMessage::SelectVariable(var.path.clone()))
            .padding([2, 4])
            .width(Length::Fill)
            .style(if is_selected {
                button::primary
            } else {
                button::text
            })
            .into()
    }

    /// Render the detail view for selected variable
    fn render_detail_view(&self) -> Element<'_, DataExplorerMessage> {
        // DataFrame inspector
        if let Some(ref df_info) = self.dataframe_info {
            return self.render_dataframe_info(df_info);
        }

        // Cube inspector
        if let Some(ref cube_info) = self.cube_info {
            return self.render_cube_info(cube_info);
        }

        // Default: show selected variable summary or instructions
        if let Some(ref path) = self.selected {
            if let Some(var) = self.variables.iter().find(|v| &v.path == path) {
                let header = text(format!("Selected: {}", var.name))
                    .size(12)
                    .font(Font::MONOSPACE);
                let type_info = text(format!("Type: {}", var.type_name)).size(11);
                let value_info = text(format!("Value: {}", var.summary)).size(11);

                return container(column![header, type_info, value_info].spacing(4))
                    .padding(8)
                    .width(Length::Fill)
                    .height(Length::FillPortion(1))
                    .into();
            }
        }

        container(
            text("Select a variable to view details")
                .size(11)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        )
        .padding(8)
        .width(Length::Fill)
        .height(Length::FillPortion(1))
        .center_x(Length::Fill)
        .into()
    }

    /// Render DataFrame information
    fn render_dataframe_info(&self, info: &DataFrameInfo) -> Element<'_, DataExplorerMessage> {
        let header = text(format!(
            "DataFrame: {} columns x {} rows",
            info.num_columns, info.num_rows
        ))
        .size(12);

        // Column list
        let column_header = text("Columns:").size(11);
        let columns: Vec<Element<'_, DataExplorerMessage>> = info
            .columns
            .iter()
            .map(|col| {
                text(format!("  {} ({})", col.name, col.data_type))
                    .size(10)
                    .font(Font::MONOSPACE)
                    .into()
            })
            .collect();

        let mut content = Column::new()
            .push(header)
            .push(Space::new().height(4))
            .push(column_header);

        for col in columns {
            content = content.push(col);
        }

        // Preview section
        if !info.preview_rows.is_empty() {
            content = content.push(Space::new().height(8));
            content = content.push(text("Preview (first 10 rows):").size(11));

            // Column headers
            let col_names: String = info
                .columns
                .iter()
                .map(|c| format!("{:>12}", truncate_str(&c.name, 12)))
                .collect::<Vec<_>>()
                .join(" | ");
            content = content.push(
                text(col_names)
                    .size(10)
                    .font(Font::MONOSPACE)
                    .color(iced::Color::from_rgb(0.7, 0.7, 0.7)),
            );

            // Separator
            let sep_len = info.columns.len() * 15;
            content = content.push(
                text("-".repeat(sep_len))
                    .size(10)
                    .font(Font::MONOSPACE)
                    .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
            );

            // Data rows
            for row in &info.preview_rows {
                let row_str: String = row
                    .iter()
                    .map(|v| format!("{:>12}", truncate_str(v, 12)))
                    .collect::<Vec<_>>()
                    .join(" | ");
                content = content.push(text(row_str).size(10).font(Font::MONOSPACE));
            }
        }

        scrollable(content.spacing(2).padding(8))
            .height(Length::FillPortion(1))
            .width(Length::Fill)
            .into()
    }

    /// Render Cube information
    fn render_cube_info(&self, info: &CubeInfo) -> Element<'_, DataExplorerMessage> {
        let header = text(format!(
            "Cube: '{}' ({} dimensions, {} measures)",
            info.name,
            info.dimensions.len(),
            info.measures.len()
        ))
        .size(12);

        let mut content = Column::new().push(header).push(Space::new().height(8));

        // Dimensions
        content = content.push(text("Dimensions:").size(11));
        for dim in &info.dimensions {
            content = content.push(
                text(format!("  {dim}"))
                    .size(10)
                    .font(Font::MONOSPACE)
                    .color(iced::Color::from_rgb(0.7, 0.9, 0.7)),
            );
        }

        content = content.push(Space::new().height(8));

        // Measures
        content = content.push(text("Measures:").size(11));
        for measure in &info.measures {
            content = content.push(
                text(format!("  {measure}"))
                    .size(10)
                    .font(Font::MONOSPACE)
                    .color(iced::Color::from_rgb(0.9, 0.8, 0.7)),
            );
        }

        scrollable(content.spacing(2).padding(8))
            .height(Length::FillPortion(1))
            .width(Length::Fill)
            .into()
    }
}

/// Actions returned from update that Workshop needs to handle
#[derive(Debug, Clone)]
pub enum DataExplorerAction {
    /// Request refresh of variable data
    RequestRefresh,
    /// Print a variable value in the REPL
    PrintInRepl(String),
    /// Copy a value to clipboard
    CopyToClipboard(String),
}

/// Check if a name is a built-in function/namespace
fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "println"
            | "len"
            | "type_of"
            | "range"
            | "assert"
            | "select"
            | "group_by"
            | "sort"
            | "distinct"
            | "rename"
            | "filter"
            | "where"
            | "join"
            | "union"
            | "take"
            | "skip"
            | "File"
            | "Dir"
            | "Path"
            | "Env"
            | "Args"
            | "Shell"
            | "Agg"
            | "Join"
            | "Cube"
            | "read_csv"
            | "read_json"
            | "read_parquet"
            | "to_csv"
            | "to_json"
            | "to_parquet"
            | "sql"
            | "sql_ctx"
            | "register"
            | "execute"
            | "http_get"
            | "http_post"
            | "spawn"
            | "sleep"
            | "Some"
            | "None"
            | "Ok"
            | "Err"
    )
}

/// Truncate a string to a max length
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = DataExplorerPanel::new();
        assert!(panel.variables.is_empty());
        assert!(panel.selected.is_none());
        assert!(panel.filter.is_empty());
    }

    #[test]
    fn test_filter_change() {
        let mut panel = DataExplorerPanel::new();
        panel.update(DataExplorerMessage::FilterChanged("test".to_string()));
        assert_eq!(panel.filter, "test");
    }

    #[test]
    fn test_toggle_expand() {
        let mut panel = DataExplorerPanel::new();
        let path = "my_var".to_string();

        // Initially not expanded
        assert!(!panel.expanded_paths.contains(&path));

        // Toggle on
        panel.update(DataExplorerMessage::ToggleExpand(path.clone()));
        assert!(panel.expanded_paths.contains(&path));

        // Toggle off
        panel.update(DataExplorerMessage::ToggleExpand(path.clone()));
        assert!(!panel.expanded_paths.contains(&path));
    }

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin_name("print"));
        assert!(is_builtin_name("File"));
        assert!(!is_builtin_name("my_variable"));
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }
}
