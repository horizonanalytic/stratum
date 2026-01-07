//! GUI element types for Stratum integration
//!
//! This module defines the `GuiElement` type which represents a GUI widget
//! that can be stored as a Stratum `Value` and rendered to iced widgets.

use std::fmt;
use std::sync::Arc;

use iced::widget::{button, canvas, checkbox, column, container, mouse_area, pick_list, progress_bar, radio, row, scrollable, slider, text, text_input, toggler, Image};
use iced::{font, Color, ContentFit, Element, Fill, Font, Length, Point};

use crate::charts::{BarChartConfig, BarChartProgram, LineChartConfig, LineChartProgram, PieChartConfig, PieChartProgram, DataPoint, DataSeries};

use stratum_core::bytecode::{GuiValue, Value};
use stratum_core::data::DataFrame;

use crate::callback::{CallbackExecutor, CallbackId};
use crate::layout::{Container, Grid, HAlign, HStack, ScrollDirection, ScrollView, Size, Spacer, VAlign, VStack, ZStack};
use crate::runtime::Message;
use crate::state::ReactiveState;
use crate::theme::{Color as StratumColor, WidgetStyle};

/// A GUI element that can be composed into a widget tree.
///
/// `GuiElement` is the primary type used to represent GUI components in Stratum.
/// It can be stored as a Value and later rendered to iced widgets.
#[derive(Clone)]
pub struct GuiElement {
    /// The kind of element (layout, widget, etc.)
    pub kind: GuiElementKind,
    /// Child elements (for containers/layouts)
    pub children: Vec<Arc<GuiElement>>,
    /// Common styling properties
    pub style: ElementStyle,
}

impl fmt::Debug for GuiElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuiElement")
            .field("kind", &self.kind)
            .field("children", &self.children.len())
            .finish()
    }
}

/// Common styling properties applicable to all elements
#[derive(Debug, Clone, Default)]
pub struct ElementStyle {
    /// Padding around the element
    pub padding: Option<f32>,
    /// Width
    pub width: Option<Size>,
    /// Height
    pub height: Option<Size>,
    /// Whether element is visible
    pub visible: bool,
    /// Widget-specific styling (background, foreground, border, etc.)
    pub widget_style: WidgetStyle,
}

impl ElementStyle {
    /// Create new default style
    #[must_use]
    pub fn new() -> Self {
        Self {
            visible: true,
            ..Default::default()
        }
    }

    /// Set background color
    #[must_use]
    pub fn with_background(mut self, color: StratumColor) -> Self {
        self.widget_style.background = Some(color);
        self
    }

    /// Set foreground/text color
    #[must_use]
    pub fn with_foreground(mut self, color: StratumColor) -> Self {
        self.widget_style.foreground = Some(color);
        self
    }

    /// Set border color
    #[must_use]
    pub fn with_border_color(mut self, color: StratumColor) -> Self {
        self.widget_style.border_color = Some(color);
        self
    }

    /// Set border width
    #[must_use]
    pub fn with_border_width(mut self, width: f32) -> Self {
        self.widget_style.border_width = Some(width);
        self
    }

    /// Set corner radius
    #[must_use]
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.widget_style.corner_radius = Some(radius);
        self
    }
}

/// The specific kind of GUI element
#[derive(Debug, Clone)]
pub enum GuiElementKind {
    /// Vertical stack layout
    VStack(VStackConfig),
    /// Horizontal stack layout
    HStack(HStackConfig),
    /// Overlay stack layout
    ZStack(ZStackConfig),
    /// Grid layout
    Grid(GridConfig),
    /// Scrollable container
    ScrollView(ScrollViewConfig),
    /// Empty space
    Spacer(SpacerConfig),
    /// Generic container
    Container(ContainerConfig),
    /// Text display
    Text(TextConfig),
    /// Clickable button
    Button(ButtonConfig),
    /// Text input field
    TextField(TextFieldConfig),
    /// Checkbox with label
    Checkbox(CheckboxConfig),
    /// Radio button for single selection from a group
    RadioButton(RadioButtonConfig),
    /// Dropdown for selecting one option from a list
    Dropdown(DropdownConfig),
    /// Slider for selecting a numeric value from a range
    Slider(SliderConfig),
    /// Toggle switch for binary on/off choices
    Toggle(ToggleConfig),
    /// Progress bar for visualizing completion
    ProgressBar(ProgressBarConfig),
    /// Image display
    Image(ImageConfig),
    /// Conditional rendering (if/else)
    Conditional(ConditionalConfig),
    /// List rendering (for each item in a list)
    ForEach(ForEachConfig),
    /// Data table for displaying DataFrames
    DataTable(DataTableConfig),
    /// Bar chart for categorical data visualization
    BarChart(BarChartConfig),
    /// Line chart for trend visualization
    LineChart(LineChartConfig),
    /// Pie chart for proportion visualization
    PieChart(PieChartConfig),
    /// OLAP Cube table with drill-down support
    CubeTable(CubeTableConfig),
    /// OLAP Cube chart with drill-down support
    CubeChart(CubeChartConfig),
    /// Dimension filter dropdown for cubes
    DimensionFilter(DimensionFilterConfig),
    /// Hierarchy level navigator for cubes
    HierarchyNavigator(HierarchyNavigatorConfig),
    /// Measure selector for cubes
    MeasureSelector(MeasureSelectorConfig),
    /// Interactive wrapper for mouse/hover events
    Interactive(InteractiveConfig),
}

/// VStack configuration
#[derive(Debug, Clone, Default)]
pub struct VStackConfig {
    /// Spacing between children
    pub spacing: f32,
    /// Horizontal alignment
    pub align: HAlign,
}

/// HStack configuration
#[derive(Debug, Clone, Default)]
pub struct HStackConfig {
    /// Spacing between children
    pub spacing: f32,
    /// Vertical alignment
    pub align: VAlign,
}

/// ZStack configuration
#[derive(Debug, Clone, Default)]
pub struct ZStackConfig {
    // ZStack has minimal config - just layers elements
}

/// Grid configuration
#[derive(Debug, Clone)]
pub struct GridConfig {
    /// Number of columns
    pub columns: usize,
    /// Spacing between cells
    pub spacing: f32,
    /// Horizontal alignment within cells
    pub cell_align_x: HAlign,
    /// Vertical alignment within cells
    pub cell_align_y: VAlign,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: 1,
            spacing: 0.0,
            cell_align_x: HAlign::Center,
            cell_align_y: VAlign::Center,
        }
    }
}

/// ScrollView configuration
#[derive(Debug, Clone, Default)]
pub struct ScrollViewConfig {
    /// Scroll direction
    pub direction: ScrollDirection,
}

/// Spacer configuration
#[derive(Debug, Clone)]
pub struct SpacerConfig {
    /// Width (None = fill)
    pub width: Option<Size>,
    /// Height (None = fill)
    pub height: Option<Size>,
}

impl Default for SpacerConfig {
    fn default() -> Self {
        Self {
            width: Some(Size::Fill),
            height: Some(Size::Fill),
        }
    }
}

/// Container configuration
#[derive(Debug, Clone, Default)]
pub struct ContainerConfig {
    /// Horizontal alignment
    pub align_x: HAlign,
    /// Vertical alignment
    pub align_y: VAlign,
    /// Center horizontally
    pub center_x: bool,
    /// Center vertically
    pub center_y: bool,
    /// Max width
    pub max_width: Option<f32>,
    /// Max height
    pub max_height: Option<f32>,
}

/// Text configuration
#[derive(Debug, Clone)]
pub struct TextConfig {
    /// The text content
    pub content: String,
    /// Font size
    pub size: Option<f32>,
    /// Bold text
    pub bold: bool,
    /// Text color (r, g, b, a)
    pub color: Option<(u8, u8, u8, u8)>,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            content: String::new(),
            size: None,
            bold: false,
            color: None,
        }
    }
}

/// Button configuration
#[derive(Debug, Clone)]
pub struct ButtonConfig {
    /// Button label
    pub label: String,
    /// Callback ID to invoke on click
    pub on_click: Option<CallbackId>,
    /// Whether button is disabled
    pub disabled: bool,
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            on_click: None,
            disabled: false,
        }
    }
}

/// TextField configuration
#[derive(Debug, Clone)]
pub struct TextFieldConfig {
    /// Current text value
    pub value: String,
    /// Placeholder text shown when empty
    pub placeholder: String,
    /// Whether to hide text (password mode)
    pub secure: bool,
    /// State field path to bind for automatic updates
    pub field_path: Option<String>,
    /// Callback ID to invoke on text change
    pub on_change: Option<CallbackId>,
    /// Callback ID to invoke on submit (Enter key)
    pub on_submit: Option<CallbackId>,
}

impl Default for TextFieldConfig {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            secure: false,
            field_path: None,
            on_change: None,
            on_submit: None,
        }
    }
}

/// Checkbox configuration
#[derive(Debug, Clone)]
pub struct CheckboxConfig {
    /// Label text displayed next to the checkbox
    pub label: String,
    /// Whether the checkbox is checked
    pub checked: bool,
    /// State field path to bind for automatic updates
    pub field_path: Option<String>,
    /// Callback ID to invoke when the checkbox is toggled
    pub on_toggle: Option<CallbackId>,
}

impl Default for CheckboxConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            checked: false,
            field_path: None,
            on_toggle: None,
        }
    }
}

/// RadioButton configuration
///
/// Radio buttons allow selecting one option from a group. The `value` represents
/// what this specific radio button stands for, while `selected_value` represents
/// the currently selected value in the group. When `value == selected_value`,
/// the radio button appears selected.
#[derive(Debug, Clone)]
pub struct RadioButtonConfig {
    /// Label text displayed next to the radio button
    pub label: String,
    /// The value this radio button represents
    pub value: String,
    /// The currently selected value in the group (for comparison)
    pub selected_value: Option<String>,
    /// State field path to bind for automatic updates
    pub field_path: Option<String>,
    /// Callback ID to invoke when this radio button is selected
    pub on_select: Option<CallbackId>,
}

impl Default for RadioButtonConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            value: String::new(),
            selected_value: None,
            field_path: None,
            on_select: None,
        }
    }
}

/// Dropdown configuration
///
/// Dropdowns (pick lists) display a list of options and allow selecting one.
/// The `options` field contains all available choices, and `selected` tracks
/// the currently selected option.
#[derive(Debug, Clone)]
pub struct DropdownConfig {
    /// Available options to choose from
    pub options: Vec<String>,
    /// Currently selected option (None if nothing selected)
    pub selected: Option<String>,
    /// Placeholder text shown when nothing is selected
    pub placeholder: Option<String>,
    /// State field path to bind for automatic updates
    pub field_path: Option<String>,
    /// Callback ID to invoke when selection changes
    pub on_select: Option<CallbackId>,
}

impl Default for DropdownConfig {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            placeholder: None,
            field_path: None,
            on_select: None,
        }
    }
}

/// Slider configuration
///
/// Sliders allow selecting a numeric value within a range by dragging a handle.
/// Supports both integer and floating-point values.
#[derive(Debug, Clone)]
pub struct SliderConfig {
    /// Current value
    pub value: f64,
    /// Minimum value (inclusive)
    pub min: f64,
    /// Maximum value (inclusive)
    pub max: f64,
    /// Step size (0.0 means continuous)
    pub step: f64,
    /// State field path to bind for automatic updates
    pub field_path: Option<String>,
    /// Callback ID to invoke when the value changes
    pub on_change: Option<CallbackId>,
    /// Callback ID to invoke when the slider is released
    pub on_release: Option<CallbackId>,
}

impl Default for SliderConfig {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            field_path: None,
            on_change: None,
            on_release: None,
        }
    }
}

/// Toggle configuration
///
/// Toggles (switches) provide a binary on/off choice with a sliding animation.
/// Similar to checkbox but with a different visual appearance.
#[derive(Debug, Clone)]
pub struct ToggleConfig {
    /// Label text displayed next to the toggle
    pub label: String,
    /// Whether the toggle is on
    pub is_on: bool,
    /// State field path to bind for automatic updates
    pub field_path: Option<String>,
    /// Callback ID to invoke when the toggle is switched
    pub on_toggle: Option<CallbackId>,
}

impl Default for ToggleConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            is_on: false,
            field_path: None,
            on_toggle: None,
        }
    }
}

/// ProgressBar configuration
///
/// Progress bars visualize the completion of an operation.
/// Value should be between 0.0 (0%) and 1.0 (100%).
#[derive(Debug, Clone)]
pub struct ProgressBarConfig {
    /// Current progress value (0.0 to 1.0)
    pub value: f32,
}

impl Default for ProgressBarConfig {
    fn default() -> Self {
        Self { value: 0.0 }
    }
}

/// Image configuration
///
/// Images display raster graphics from a file path or bytes.
#[derive(Debug, Clone)]
pub struct ImageConfig {
    /// Path to the image file
    pub path: Option<String>,
    /// Image content fit mode
    pub content_fit: ImageContentFit,
    /// Optional fixed width
    pub image_width: Option<f32>,
    /// Optional fixed height
    pub image_height: Option<f32>,
    /// Opacity (0.0 to 1.0)
    pub opacity: f32,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            path: None,
            content_fit: ImageContentFit::Contain,
            image_width: None,
            image_height: None,
            opacity: 1.0,
        }
    }
}

/// How an image should be sized within its container
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageContentFit {
    /// Scale the image to fit within the bounds while maintaining aspect ratio
    #[default]
    Contain,
    /// Scale the image to fill the bounds, cropping if necessary
    Cover,
    /// Fill the bounds exactly, distorting the image if necessary
    Fill,
    /// Don't scale the image
    None,
    /// Scale down only if the image is larger than the bounds
    ScaleDown,
}

impl ImageContentFit {
    /// Convert to iced's ContentFit
    #[must_use]
    pub fn to_iced(self) -> ContentFit {
        match self {
            Self::Contain => ContentFit::Contain,
            Self::Cover => ContentFit::Cover,
            Self::Fill => ContentFit::Fill,
            Self::None => ContentFit::None,
            Self::ScaleDown => ContentFit::ScaleDown,
        }
    }
}

/// Conditional rendering configuration
///
/// Renders different elements based on a boolean condition from state.
#[derive(Debug, Clone)]
pub struct ConditionalConfig {
    /// Path to the boolean field in state (e.g., "show_details")
    pub condition_field: String,
    /// Element to render when condition is true
    pub true_element: Option<Arc<GuiElement>>,
    /// Element to render when condition is false (else branch)
    pub false_element: Option<Arc<GuiElement>>,
}

impl Default for ConditionalConfig {
    fn default() -> Self {
        Self {
            condition_field: String::new(),
            true_element: None,
            false_element: None,
        }
    }
}

/// List rendering configuration
///
/// Renders an element for each item in a list, using a template callback.
/// The template callback is registered in the CallbackRegistry and called
/// for each list item during list expansion (before rendering).
#[derive(Debug, Clone, Default)]
pub struct ForEachConfig {
    /// Path to the list field in state (e.g., "items")
    pub list_field: String,
    /// Template callback ID: called with (item, index) -> GuiElement
    /// Registered in CallbackRegistry
    pub template_id: Option<CallbackId>,
    /// Optional key function callback ID: (item) -> key for efficient updates
    pub key_fn_id: Option<CallbackId>,
}

/// Data table configuration
///
/// Displays tabular data from a DataFrame with features like sorting,
/// pagination, row selection, and customizable column widths.
#[derive(Clone)]
pub struct DataTableConfig {
    /// The DataFrame to display
    pub dataframe: Option<Arc<DataFrame>>,
    /// Columns to display (None = all columns)
    pub columns: Option<Vec<String>>,
    /// Number of rows per page (None = show all)
    pub page_size: Option<usize>,
    /// Current page number (0-indexed)
    pub current_page: usize,
    /// Whether columns are sortable
    pub sortable: bool,
    /// Currently sorted column (None = no sort)
    pub sort_column: Option<String>,
    /// Sort direction (true = ascending, false = descending)
    pub sort_ascending: bool,
    /// Whether rows can be selected
    pub selectable: bool,
    /// Currently selected row indices
    pub selected_rows: Vec<usize>,
    /// Custom column widths (column name -> width in pixels)
    pub column_widths: Vec<(String, f32)>,
    /// Callback when a row is clicked (receives row index)
    pub on_row_click: Option<CallbackId>,
    /// Callback when a cell is clicked (receives row index, column name)
    pub on_cell_click: Option<CallbackId>,
    /// Callback when sort changes (receives column name, ascending)
    pub on_sort: Option<CallbackId>,
    /// Callback when page changes (receives new page number)
    pub on_page_change: Option<CallbackId>,
    /// Callback when selection changes (receives selected row indices)
    pub on_selection_change: Option<CallbackId>,
    /// Custom cell renderers (column name -> callback that takes cell value and returns element)
    pub cell_renderers: Vec<(String, CallbackId)>,
}

impl Default for DataTableConfig {
    fn default() -> Self {
        Self {
            dataframe: None,
            columns: None,
            page_size: Some(50),
            current_page: 0,
            sortable: true,
            sort_column: None,
            sort_ascending: true,
            selectable: false,
            selected_rows: Vec::new(),
            column_widths: Vec::new(),
            on_row_click: None,
            on_cell_click: None,
            on_sort: None,
            on_page_change: None,
            on_selection_change: None,
            cell_renderers: Vec::new(),
        }
    }
}

impl fmt::Debug for DataTableConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataTableConfig")
            .field("columns", &self.columns)
            .field("page_size", &self.page_size)
            .field("current_page", &self.current_page)
            .field("sortable", &self.sortable)
            .field("sort_column", &self.sort_column)
            .field("selectable", &self.selectable)
            .field("selected_rows", &self.selected_rows.len())
            .finish()
    }
}

/// Sort direction for data tables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    /// Sort in ascending order (A-Z, 0-9)
    #[default]
    Ascending,
    /// Sort in descending order (Z-A, 9-0)
    Descending,
}

// =============================================================================
// OLAP Cube Widget Configurations
// =============================================================================

/// CubeTable configuration
///
/// OLAP-aware data table that displays Cube data with drill-down/roll-up support.
/// Unlike DataTable, CubeTable understands dimensions, measures, and hierarchies.
#[derive(Clone)]
pub struct CubeTableConfig {
    /// The Cube to display (stored as Arc for thread-safety)
    pub cube: Option<Arc<stratum_core::data::Cube>>,
    /// Row dimensions (dimensions to show as row headers)
    pub row_dimensions: Vec<String>,
    /// Column dimensions (dimensions to pivot as column headers)
    pub column_dimensions: Vec<String>,
    /// Measures to display
    pub measures: Vec<String>,
    /// Number of rows per page (None = show all)
    pub page_size: Option<usize>,
    /// Current page number (0-indexed)
    pub current_page: usize,
    /// Whether to show drill-down controls
    pub show_drill_controls: bool,
    /// Callback when a dimension value is clicked for drill-down
    pub on_drill: Option<CallbackId>,
    /// Callback when roll-up is requested
    pub on_roll_up: Option<CallbackId>,
    /// Callback when a cell is clicked
    pub on_cell_click: Option<CallbackId>,
    /// Callback when page changes
    pub on_page_change: Option<CallbackId>,
}

impl Default for CubeTableConfig {
    fn default() -> Self {
        Self {
            cube: None,
            row_dimensions: Vec::new(),
            column_dimensions: Vec::new(),
            measures: Vec::new(),
            page_size: Some(50),
            current_page: 0,
            show_drill_controls: true,
            on_drill: None,
            on_roll_up: None,
            on_cell_click: None,
            on_page_change: None,
        }
    }
}

impl fmt::Debug for CubeTableConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CubeTableConfig")
            .field("row_dimensions", &self.row_dimensions)
            .field("column_dimensions", &self.column_dimensions)
            .field("measures", &self.measures)
            .field("page_size", &self.page_size)
            .field("current_page", &self.current_page)
            .field("show_drill_controls", &self.show_drill_controls)
            .finish()
    }
}

/// CubeChart configuration
///
/// OLAP-aware chart that visualizes Cube data with drill-down support.
/// Auto-updates when slice/drill operations are performed on the cube.
#[derive(Clone)]
pub struct CubeChartConfig {
    /// The Cube to visualize
    pub cube: Option<Arc<stratum_core::data::Cube>>,
    /// Chart type ("bar", "line", "pie")
    pub chart_type: CubeChartType,
    /// X-axis dimension
    pub x_dimension: Option<String>,
    /// Y-axis measure
    pub y_measure: Option<String>,
    /// Series dimension (for grouped charts)
    pub series_dimension: Option<String>,
    /// Chart title
    pub title: Option<String>,
    /// Chart width
    pub width: f32,
    /// Chart height
    pub height: f32,
    /// Whether to show legend
    pub show_legend: bool,
    /// Whether to show grid lines
    pub show_grid: bool,
    /// Callback when a chart element is clicked (for drill-down)
    pub on_click: Option<CallbackId>,
}

impl Default for CubeChartConfig {
    fn default() -> Self {
        Self {
            cube: None,
            chart_type: CubeChartType::Bar,
            x_dimension: None,
            y_measure: None,
            series_dimension: None,
            title: None,
            width: 400.0,
            height: 300.0,
            show_legend: true,
            show_grid: true,
            on_click: None,
        }
    }
}

impl fmt::Debug for CubeChartConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CubeChartConfig")
            .field("chart_type", &self.chart_type)
            .field("x_dimension", &self.x_dimension)
            .field("y_measure", &self.y_measure)
            .field("series_dimension", &self.series_dimension)
            .field("title", &self.title)
            .finish()
    }
}

/// Chart type for CubeChart
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CubeChartType {
    /// Bar chart (default)
    #[default]
    Bar,
    /// Line chart
    Line,
    /// Pie chart
    Pie,
}

impl CubeChartType {
    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "line" => Self::Line,
            "pie" => Self::Pie,
            _ => Self::Bar,
        }
    }
}

/// DimensionFilter configuration
///
/// Dropdown filter for a cube dimension. Populates options from dimension_values()
/// and triggers slice operations when a value is selected.
#[derive(Clone)]
pub struct DimensionFilterConfig {
    /// The Cube to filter
    pub cube: Option<Arc<stratum_core::data::Cube>>,
    /// The dimension to filter on
    pub dimension: String,
    /// Label to display
    pub label: Option<String>,
    /// Currently selected value (None = "All")
    pub selected_value: Option<String>,
    /// Whether to show "All" option
    pub show_all_option: bool,
    /// Placeholder text when nothing is selected
    pub placeholder: Option<String>,
    /// State field path for binding
    pub field_path: Option<String>,
    /// Callback when selection changes
    pub on_select: Option<CallbackId>,
}

impl Default for DimensionFilterConfig {
    fn default() -> Self {
        Self {
            cube: None,
            dimension: String::new(),
            label: None,
            selected_value: None,
            show_all_option: true,
            placeholder: Some("Select...".to_string()),
            field_path: None,
            on_select: None,
        }
    }
}

impl fmt::Debug for DimensionFilterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DimensionFilterConfig")
            .field("dimension", &self.dimension)
            .field("label", &self.label)
            .field("selected_value", &self.selected_value)
            .field("show_all_option", &self.show_all_option)
            .finish()
    }
}

/// HierarchyNavigator configuration
///
/// Breadcrumb-style navigator for hierarchy levels. Shows current level
/// and allows drilling down or rolling up within a hierarchy.
#[derive(Clone)]
pub struct HierarchyNavigatorConfig {
    /// The Cube containing the hierarchy
    pub cube: Option<Arc<stratum_core::data::Cube>>,
    /// The hierarchy to navigate
    pub hierarchy: String,
    /// Current level in the hierarchy
    pub current_level: Option<String>,
    /// Label to display
    pub label: Option<String>,
    /// Callback when drill-down is requested
    pub on_drill_down: Option<CallbackId>,
    /// Callback when roll-up is requested
    pub on_roll_up: Option<CallbackId>,
    /// Callback when a specific level is clicked
    pub on_level_change: Option<CallbackId>,
}

impl Default for HierarchyNavigatorConfig {
    fn default() -> Self {
        Self {
            cube: None,
            hierarchy: String::new(),
            current_level: None,
            label: None,
            on_drill_down: None,
            on_roll_up: None,
            on_level_change: None,
        }
    }
}

impl fmt::Debug for HierarchyNavigatorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HierarchyNavigatorConfig")
            .field("hierarchy", &self.hierarchy)
            .field("current_level", &self.current_level)
            .field("label", &self.label)
            .finish()
    }
}

/// MeasureSelector configuration
///
/// Multi-select widget for toggling which measures are visible.
/// Useful for dashboards where users want to focus on specific metrics.
#[derive(Clone)]
pub struct MeasureSelectorConfig {
    /// The Cube containing the measures
    pub cube: Option<Arc<stratum_core::data::Cube>>,
    /// Currently selected measures
    pub selected_measures: Vec<String>,
    /// Label to display
    pub label: Option<String>,
    /// State field path for binding selected measures
    pub field_path: Option<String>,
    /// Callback when selection changes
    pub on_change: Option<CallbackId>,
}

impl Default for MeasureSelectorConfig {
    fn default() -> Self {
        Self {
            cube: None,
            selected_measures: Vec::new(),
            label: None,
            field_path: None,
            on_change: None,
        }
    }
}

impl fmt::Debug for MeasureSelectorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MeasureSelectorConfig")
            .field("selected_measures", &self.selected_measures)
            .field("label", &self.label)
            .finish()
    }
}

/// Row selection mode for data tables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// No row selection
    #[default]
    None,
    /// Single row selection
    Single,
    /// Multiple row selection
    Multiple,
}

/// Interactive wrapper configuration
///
/// Wraps any element and adds mouse event handlers (click, hover, etc.)
/// using iced's MouseArea widget.
#[derive(Debug, Clone, Default)]
pub struct InteractiveConfig {
    /// Callback for left mouse button press
    pub on_press: Option<CallbackId>,
    /// Callback for left mouse button release
    pub on_release: Option<CallbackId>,
    /// Callback for double-click
    pub on_double_click: Option<CallbackId>,
    /// Callback for right mouse button press
    pub on_right_press: Option<CallbackId>,
    /// Callback for right mouse button release
    pub on_right_release: Option<CallbackId>,
    /// Callback for middle mouse button press
    pub on_middle_press: Option<CallbackId>,
    /// Callback for middle mouse button release
    pub on_middle_release: Option<CallbackId>,
    /// Callback when mouse enters the element area
    pub on_enter: Option<CallbackId>,
    /// Callback when mouse exits the element area
    pub on_exit: Option<CallbackId>,
    /// Callback when mouse moves within the element area
    pub on_move: Option<CallbackId>,
    /// Callback when scroll wheel is used
    pub on_scroll: Option<CallbackId>,
    /// Cursor style when hovering (e.g., "pointer", "grab")
    pub cursor_style: Option<CursorStyle>,
}

/// Cursor/interaction style for hover state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    /// Default cursor
    #[default]
    Default,
    /// Pointer cursor (hand)
    Pointer,
    /// Text selection cursor
    Text,
    /// Crosshair cursor
    Crosshair,
    /// Move cursor
    Move,
    /// Grab cursor (open hand)
    Grab,
    /// Grabbing cursor (closed hand)
    Grabbing,
    /// Not allowed cursor
    NotAllowed,
    /// Resize horizontally
    ResizeHorizontal,
    /// Resize vertically
    ResizeVertical,
}

impl CursorStyle {
    /// Convert to iced mouse::Interaction
    #[must_use]
    pub fn to_iced(self) -> iced::mouse::Interaction {
        use iced::mouse::Interaction;
        match self {
            Self::Default => Interaction::Idle,
            Self::Pointer => Interaction::Pointer,
            Self::Text => Interaction::Text,
            Self::Crosshair => Interaction::Crosshair,
            Self::Move => Interaction::Move,
            Self::Grab => Interaction::Grab,
            Self::Grabbing => Interaction::Grabbing,
            Self::NotAllowed => Interaction::NotAllowed,
            Self::ResizeHorizontal => Interaction::ResizingHorizontally,
            Self::ResizeVertical => Interaction::ResizingVertically,
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pointer" | "hand" => Self::Pointer,
            "text" => Self::Text,
            "crosshair" => Self::Crosshair,
            "move" => Self::Move,
            "grab" => Self::Grab,
            "grabbing" => Self::Grabbing,
            "not-allowed" | "notallowed" | "forbidden" => Self::NotAllowed,
            "resize-horizontal" | "ew-resize" => Self::ResizeHorizontal,
            "resize-vertical" | "ns-resize" => Self::ResizeVertical,
            _ => Self::Default,
        }
    }
}

impl GuiElement {
    /// Create a new VStack element
    #[must_use]
    pub fn vstack() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::VStack(VStackConfig::default()))
    }

    /// Create a VStack with spacing
    #[must_use]
    pub fn vstack_with_spacing(spacing: f32) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::VStack(VStackConfig {
            spacing,
            ..Default::default()
        }))
    }

    /// Create a new HStack element
    #[must_use]
    pub fn hstack() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::HStack(HStackConfig::default()))
    }

    /// Create an HStack with spacing
    #[must_use]
    pub fn hstack_with_spacing(spacing: f32) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::HStack(HStackConfig {
            spacing,
            ..Default::default()
        }))
    }

    /// Create a new ZStack element
    #[must_use]
    pub fn zstack() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::ZStack(ZStackConfig::default()))
    }

    /// Create a new Grid element
    #[must_use]
    pub fn grid(columns: usize) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Grid(GridConfig {
            columns: columns.max(1),
            ..Default::default()
        }))
    }

    /// Create a new ScrollView element
    #[must_use]
    pub fn scroll_view() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::ScrollView(ScrollViewConfig::default()))
    }

    /// Create a new Spacer element
    #[must_use]
    pub fn spacer() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Spacer(SpacerConfig::default()))
    }

    /// Create a horizontal spacer
    #[must_use]
    pub fn horizontal_spacer() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Spacer(SpacerConfig {
            width: Some(Size::Fill),
            height: Some(Size::Fixed(0.0)),
        }))
    }

    /// Create a vertical spacer
    #[must_use]
    pub fn vertical_spacer() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Spacer(SpacerConfig {
            width: Some(Size::Fixed(0.0)),
            height: Some(Size::Fill),
        }))
    }

    /// Create a new Container element
    #[must_use]
    pub fn container() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Container(ContainerConfig::default()))
    }

    /// Create a new Text element
    #[must_use]
    pub fn text(content: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Text(TextConfig {
            content: content.into(),
            ..Default::default()
        }))
    }

    /// Create a new Button element
    #[must_use]
    pub fn button(label: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Button(ButtonConfig {
            label: label.into(),
            ..Default::default()
        }))
    }

    /// Create a new TextField element
    #[must_use]
    pub fn text_field() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::TextField(TextFieldConfig::default()))
    }

    /// Create a new TextField element with initial value
    #[must_use]
    pub fn text_field_with_value(value: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::TextField(TextFieldConfig {
            value: value.into(),
            ..Default::default()
        }))
    }

    /// Create a new Checkbox element
    #[must_use]
    pub fn checkbox(label: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Checkbox(CheckboxConfig {
            label: label.into(),
            ..Default::default()
        }))
    }

    /// Create a new Checkbox element with initial checked state
    #[must_use]
    pub fn checkbox_with_state(label: impl Into<String>, checked: bool) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Checkbox(CheckboxConfig {
            label: label.into(),
            checked,
            ..Default::default()
        }))
    }

    /// Create a new RadioButton element
    ///
    /// The `value` parameter is what this radio button represents in its group.
    #[must_use]
    pub fn radio_button(label: impl Into<String>, value: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::RadioButton(RadioButtonConfig {
            label: label.into(),
            value: value.into(),
            ..Default::default()
        }))
    }

    /// Create a new RadioButton element with the currently selected value
    ///
    /// The radio button appears selected when `value == selected_value`.
    #[must_use]
    pub fn radio_button_with_selection(
        label: impl Into<String>,
        value: impl Into<String>,
        selected_value: Option<String>,
    ) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::RadioButton(RadioButtonConfig {
            label: label.into(),
            value: value.into(),
            selected_value,
            ..Default::default()
        }))
    }

    /// Create a new Dropdown element with options
    #[must_use]
    pub fn dropdown(options: Vec<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Dropdown(DropdownConfig {
            options,
            ..Default::default()
        }))
    }

    /// Create a new Dropdown element with options and initial selection
    #[must_use]
    pub fn dropdown_with_selection(options: Vec<String>, selected: Option<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Dropdown(DropdownConfig {
            options,
            selected,
            ..Default::default()
        }))
    }

    /// Create a new Slider element with a range
    #[must_use]
    pub fn slider(min: f64, max: f64) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Slider(SliderConfig {
            min,
            max,
            ..Default::default()
        }))
    }

    /// Create a new Slider element with a range and initial value
    #[must_use]
    pub fn slider_with_value(min: f64, max: f64, value: f64) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Slider(SliderConfig {
            value: value.clamp(min, max),
            min,
            max,
            ..Default::default()
        }))
    }

    /// Create a new Toggle element
    #[must_use]
    pub fn toggle(label: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Toggle(ToggleConfig {
            label: label.into(),
            ..Default::default()
        }))
    }

    /// Create a new Toggle element with initial state
    #[must_use]
    pub fn toggle_with_state(label: impl Into<String>, is_on: bool) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Toggle(ToggleConfig {
            label: label.into(),
            is_on,
            ..Default::default()
        }))
    }

    /// Create a new ProgressBar element
    #[must_use]
    pub fn progress_bar(value: f32) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::ProgressBar(ProgressBarConfig {
            value: value.clamp(0.0, 1.0),
        }))
    }

    /// Create a new Image element from a file path
    #[must_use]
    pub fn image(path: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Image(ImageConfig {
            path: Some(path.into()),
            ..Default::default()
        }))
    }

    /// Create a conditional element (if/else rendering)
    ///
    /// The condition_field should be a path to a boolean field in state.
    #[must_use]
    pub fn conditional(condition_field: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Conditional(ConditionalConfig {
            condition_field: condition_field.into(),
            ..Default::default()
        }))
    }

    /// Create a for-each element for list rendering
    ///
    /// The list_field should be a path to a list field in state.
    /// The template_id should be a registered callback that takes (item, index) and returns a GuiElement.
    #[must_use]
    pub fn for_each(list_field: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::ForEach(ForEachConfig {
            list_field: list_field.into(),
            ..Default::default()
        }))
    }

    /// Create a for-each element with a template callback ID
    #[must_use]
    pub fn for_each_with_template(list_field: impl Into<String>, template_id: CallbackId) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::ForEach(ForEachConfig {
            list_field: list_field.into(),
            template_id: Some(template_id),
            key_fn_id: None,
        }))
    }

    /// Create a data table element
    ///
    /// The data table displays data from a DataFrame in a tabular format
    /// with features like sorting, pagination, and row selection.
    #[must_use]
    pub fn data_table() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::DataTable(DataTableConfig::default()))
    }

    /// Create a data table with a DataFrame
    #[must_use]
    pub fn data_table_with_data(dataframe: Arc<DataFrame>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::DataTable(DataTableConfig {
            dataframe: Some(dataframe),
            ..Default::default()
        }))
    }

    // ========== Chart Builders ==========

    /// Create a new bar chart element
    #[must_use]
    pub fn bar_chart() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::BarChart(BarChartConfig::default()))
    }

    /// Create a bar chart with data
    #[must_use]
    pub fn bar_chart_with_data(data: Vec<DataPoint>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::BarChart(BarChartConfig {
            data,
            ..Default::default()
        }))
    }

    /// Create a new line chart element
    #[must_use]
    pub fn line_chart() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::LineChart(LineChartConfig::default()))
    }

    /// Create a line chart with data
    #[must_use]
    pub fn line_chart_with_data(labels: Vec<String>, series: Vec<DataSeries>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::LineChart(LineChartConfig {
            labels,
            series,
            ..Default::default()
        }))
    }

    /// Create a new pie chart element
    #[must_use]
    pub fn pie_chart() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::PieChart(PieChartConfig::default()))
    }

    /// Create a pie chart with data
    #[must_use]
    pub fn pie_chart_with_data(data: Vec<DataPoint>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::PieChart(PieChartConfig {
            data,
            ..Default::default()
        }))
    }

    // =========================================================================
    // OLAP Cube Widget Builders
    // =========================================================================

    /// Create a new CubeTable element
    #[must_use]
    pub fn cube_table() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::CubeTable(CubeTableConfig::default()))
    }

    /// Create a CubeTable element with a cube
    #[must_use]
    pub fn cube_table_with_cube(cube: Arc<stratum_core::data::Cube>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::CubeTable(CubeTableConfig {
            cube: Some(cube),
            ..Default::default()
        }))
    }

    /// Create a new CubeChart element
    #[must_use]
    pub fn cube_chart() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::CubeChart(CubeChartConfig::default()))
    }

    /// Create a CubeChart element with a cube
    #[must_use]
    pub fn cube_chart_with_cube(cube: Arc<stratum_core::data::Cube>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::CubeChart(CubeChartConfig {
            cube: Some(cube),
            ..Default::default()
        }))
    }

    /// Create a new DimensionFilter element
    #[must_use]
    pub fn dimension_filter(dimension: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::DimensionFilter(DimensionFilterConfig {
            dimension: dimension.into(),
            ..Default::default()
        }))
    }

    /// Create a DimensionFilter element with a cube
    #[must_use]
    pub fn dimension_filter_with_cube(
        cube: Arc<stratum_core::data::Cube>,
        dimension: impl Into<String>,
    ) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::DimensionFilter(DimensionFilterConfig {
            cube: Some(cube),
            dimension: dimension.into(),
            ..Default::default()
        }))
    }

    /// Create a new HierarchyNavigator element
    #[must_use]
    pub fn hierarchy_navigator(hierarchy: impl Into<String>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::HierarchyNavigator(HierarchyNavigatorConfig {
            hierarchy: hierarchy.into(),
            ..Default::default()
        }))
    }

    /// Create a HierarchyNavigator element with a cube
    #[must_use]
    pub fn hierarchy_navigator_with_cube(
        cube: Arc<stratum_core::data::Cube>,
        hierarchy: impl Into<String>,
    ) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::HierarchyNavigator(HierarchyNavigatorConfig {
            cube: Some(cube),
            hierarchy: hierarchy.into(),
            ..Default::default()
        }))
    }

    /// Create a new MeasureSelector element
    #[must_use]
    pub fn measure_selector() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::MeasureSelector(MeasureSelectorConfig::default()))
    }

    /// Create a MeasureSelector element with a cube
    #[must_use]
    pub fn measure_selector_with_cube(cube: Arc<stratum_core::data::Cube>) -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::MeasureSelector(MeasureSelectorConfig {
            cube: Some(cube),
            ..Default::default()
        }))
    }

    // =========================================================================
    // Interactive Wrapper Builder
    // =========================================================================

    /// Create an interactive wrapper element
    ///
    /// Wraps any element to add mouse event handlers (click, hover, etc.)
    /// ```ignore
    /// GuiElement::interactive()
    ///     .on_press(click_callback)
    ///     .on_hover_enter(enter_callback)
    ///     .cursor("pointer")
    ///     .child(button_element)
    ///     .build()
    /// ```
    #[must_use]
    pub fn interactive() -> GuiElementBuilder {
        GuiElementBuilder::new(GuiElementKind::Interactive(InteractiveConfig::default()))
    }

    /// Render this element to an iced Element
    pub fn render(&self) -> Element<'_, Message> {
        if !self.style.visible {
            return iced::widget::Space::new().into();
        }

        match &self.kind {
            GuiElementKind::VStack(config) => {
                let children: Vec<Element<'_, Message>> =
                    self.children.iter().map(|c| c.render()).collect();

                let mut vstack = VStack::new()
                    .spacing(config.spacing)
                    .align(config.align);

                if let Some(padding) = self.style.padding {
                    vstack = vstack.padding(padding);
                }
                if let Some(width) = self.style.width {
                    vstack = vstack.width(width);
                }
                if let Some(height) = self.style.height {
                    vstack = vstack.height(height);
                }

                vstack.render(children)
            }

            GuiElementKind::HStack(config) => {
                let children: Vec<Element<'_, Message>> =
                    self.children.iter().map(|c| c.render()).collect();

                let mut hstack = HStack::new()
                    .spacing(config.spacing)
                    .align(config.align);

                if let Some(padding) = self.style.padding {
                    hstack = hstack.padding(padding);
                }
                if let Some(width) = self.style.width {
                    hstack = hstack.width(width);
                }
                if let Some(height) = self.style.height {
                    hstack = hstack.height(height);
                }

                hstack.render(children)
            }

            GuiElementKind::ZStack(_config) => {
                let children: Vec<Element<'_, Message>> =
                    self.children.iter().map(|c| c.render()).collect();

                let mut zstack = ZStack::new();

                if let Some(padding) = self.style.padding {
                    zstack = zstack.padding(padding);
                }
                if let Some(width) = self.style.width {
                    zstack = zstack.width(width);
                }
                if let Some(height) = self.style.height {
                    zstack = zstack.height(height);
                }

                zstack.render(children)
            }

            GuiElementKind::Grid(config) => {
                let children: Vec<Element<'_, Message>> =
                    self.children.iter().map(|c| c.render()).collect();

                let mut grid = Grid::new(config.columns)
                    .spacing(config.spacing)
                    .cell_align_x(config.cell_align_x)
                    .cell_align_y(config.cell_align_y);

                if let Some(padding) = self.style.padding {
                    grid = grid.padding(padding);
                }
                if let Some(width) = self.style.width {
                    grid = grid.width(width);
                }
                if let Some(height) = self.style.height {
                    grid = grid.height(height);
                }

                grid.render(children)
            }

            GuiElementKind::ScrollView(config) => {
                // ScrollView expects a single child, take the first or use empty space
                let content = if let Some(child) = self.children.first() {
                    child.render()
                } else {
                    iced::widget::Space::new().into()
                };

                let mut scroll = ScrollView::new().direction(config.direction);

                if let Some(width) = self.style.width {
                    scroll = scroll.width(width);
                }
                if let Some(height) = self.style.height {
                    scroll = scroll.height(height);
                }

                scroll.render(content)
            }

            GuiElementKind::Spacer(config) => {
                let mut spacer = Spacer::new();

                if let Some(w) = config.width {
                    spacer = spacer.width(w);
                }
                if let Some(h) = config.height {
                    spacer = spacer.height(h);
                }

                spacer.render()
            }

            GuiElementKind::Container(config) => {
                // Container expects a single child
                let content = if let Some(child) = self.children.first() {
                    child.render()
                } else {
                    iced::widget::Space::new().into()
                };

                let mut c = Container::new()
                    .align_x(config.align_x)
                    .align_y(config.align_y);

                if config.center_x {
                    c = c.center_x();
                }
                if config.center_y {
                    c = c.center_y();
                }
                if let Some(max_w) = config.max_width {
                    c = c.max_width(max_w);
                }
                if let Some(max_h) = config.max_height {
                    c = c.max_height(max_h);
                }
                if let Some(padding) = self.style.padding {
                    c = c.padding(padding);
                }
                if let Some(width) = self.style.width {
                    c = c.width(width);
                }
                if let Some(height) = self.style.height {
                    c = c.height(height);
                }

                c.render(content)
            }

            GuiElementKind::Text(config) => {
                let mut t = text(&config.content);

                if let Some(size) = config.size {
                    t = t.size(size);
                }

                // Apply bold font weight
                if config.bold {
                    t = t.font(Font {
                        weight: font::Weight::Bold,
                        ..Font::default()
                    });
                }

                // Apply color
                if let Some((r, g, b, a)) = config.color {
                    let color = Color::from_rgba8(r, g, b, f32::from(a) / 255.0);
                    t = t.color(color);
                }

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(t).padding(padding).into()
                } else {
                    t.into()
                }
            }

            GuiElementKind::Button(config) => {
                let label = text(&config.label);
                let mut b = button(label);

                if let Some(padding) = self.style.padding {
                    b = b.padding(padding);
                }

                if !config.disabled {
                    if let Some(callback_id) = config.on_click {
                        b = b.on_press(Message::InvokeCallback(callback_id));
                    }
                }

                b.into()
            }

            GuiElementKind::TextField(config) => {
                let mut input = text_input(&config.placeholder, &config.value);

                // Apply secure mode for password fields
                if config.secure {
                    input = input.secure(true);
                }

                // Handle text input changes
                // Priority: field_path binding > on_change callback
                if let Some(ref field) = config.field_path {
                    let field = field.clone();
                    input = input.on_input(move |text| Message::SetStringField {
                        field: field.clone(),
                        value: text,
                    });
                } else if let Some(callback_id) = config.on_change {
                    input = input.on_input(move |text| Message::TextFieldChanged {
                        callback_id,
                        value: text,
                    });
                }

                // Handle submit (Enter key)
                if let Some(callback_id) = config.on_submit {
                    input = input.on_submit(Message::InvokeCallback(callback_id));
                }

                // Apply width from style
                if let Some(width) = self.style.width {
                    input = input.width(width.to_iced());
                }

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(input).padding(padding).into()
                } else {
                    input.into()
                }
            }

            GuiElementKind::Checkbox(config) => {
                let label = config.label.clone();

                // Create checkbox with on_toggle handler
                // Priority: field_path binding > on_toggle callback
                let cb = if let Some(ref field) = config.field_path {
                    let field = field.clone();
                    checkbox(config.checked).label(label).on_toggle(move |checked| {
                        Message::SetBoolField {
                            field: field.clone(),
                            value: checked,
                        }
                    })
                } else if let Some(callback_id) = config.on_toggle {
                    checkbox(config.checked).label(label).on_toggle(move |checked| {
                        Message::CheckboxToggled {
                            callback_id,
                            checked,
                        }
                    })
                } else {
                    // No binding or callback - checkbox is read-only
                    checkbox(config.checked).label(label)
                };

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(cb).padding(padding).into()
                } else {
                    cb.into()
                }
            }

            GuiElementKind::RadioButton(config) => {
                let label = config.label.clone();
                let value_str = config.value.clone();

                // Determine if this radio button is selected
                // We use () as the iced value type (since String doesn't impl Copy)
                // The actual string value is captured in the closure
                let is_selected = config.selected_value.as_ref() == Some(&config.value);
                let selected: Option<()> = if is_selected { Some(()) } else { None };

                // Create radio button with selection handler
                // iced radio takes: label, value, selected option, on_click handler
                // Priority: field_path binding > on_select callback > no-op
                let rb = if let Some(ref field) = config.field_path {
                    let field = field.clone();
                    let value_for_message = value_str.clone();
                    radio(label, (), selected, move |()| {
                        Message::SetStringField {
                            field: field.clone(),
                            value: value_for_message.clone(),
                        }
                    })
                } else if let Some(callback_id) = config.on_select {
                    let value_for_message = value_str.clone();
                    radio(label, (), selected, move |()| {
                        Message::RadioButtonSelected {
                            callback_id,
                            value: value_for_message.clone(),
                        }
                    })
                } else {
                    // No binding or callback - radio emits NoOp
                    radio(label, (), selected, |()| Message::NoOp)
                };

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(rb).padding(padding).into()
                } else {
                    rb.into()
                }
            }

            GuiElementKind::Dropdown(config) => {
                let options = config.options.clone();
                let selected = config.selected.clone();
                let placeholder = config.placeholder.clone().unwrap_or_default();

                // Create pick_list with selection handler
                // Priority: field_path binding > on_select callback
                let pl = if let Some(ref field) = config.field_path {
                    let field = field.clone();
                    pick_list(options, selected, move |value: String| {
                        Message::SetStringField {
                            field: field.clone(),
                            value,
                        }
                    })
                    .placeholder(placeholder)
                } else if let Some(callback_id) = config.on_select {
                    pick_list(options, selected, move |value: String| {
                        Message::DropdownSelected {
                            callback_id,
                            value,
                        }
                    })
                    .placeholder(placeholder)
                } else {
                    // No binding or callback - pick_list still needs a handler
                    pick_list(options, selected, |_: String| Message::NoOp).placeholder(placeholder)
                };

                // Apply width from style
                let pl = if let Some(width) = self.style.width {
                    pl.width(width.to_iced())
                } else {
                    pl
                };

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(pl).padding(padding).into()
                } else {
                    pl.into()
                }
            }

            GuiElementKind::Slider(config) => {
                let value = config.value;
                let range = config.min..=config.max;

                // Create slider with change handler
                // Priority: field_path binding > on_change callback > no-op
                let sl = if let Some(ref field) = config.field_path {
                    let field = field.clone();
                    slider(range, value, move |v| Message::SetFloatField {
                        field: field.clone(),
                        value: v,
                    })
                } else if let Some(callback_id) = config.on_change {
                    slider(range, value, move |v| Message::SliderChanged {
                        callback_id,
                        value: v,
                    })
                } else {
                    // No binding or callback - slider emits NoOp
                    slider(range, value, |_| Message::NoOp)
                };

                // Apply step if non-zero
                let sl = if config.step > 0.0 {
                    sl.step(config.step)
                } else {
                    sl
                };

                // Apply on_release if present
                let sl = if let Some(callback_id) = config.on_release {
                    sl.on_release(Message::InvokeCallback(callback_id))
                } else {
                    sl
                };

                // Apply width from style
                let sl = if let Some(width) = self.style.width {
                    sl.width(width.to_iced())
                } else {
                    sl
                };

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(sl).padding(padding).into()
                } else {
                    sl.into()
                }
            }

            GuiElementKind::Toggle(config) => {
                let label = config.label.clone();

                // Create toggler with toggle handler
                // Priority: field_path binding > on_toggle callback > no-op
                let tg = if let Some(ref field) = config.field_path {
                    let field = field.clone();
                    toggler(config.is_on)
                        .label(label)
                        .on_toggle(move |is_on| Message::SetBoolField {
                            field: field.clone(),
                            value: is_on,
                        })
                } else if let Some(callback_id) = config.on_toggle {
                    toggler(config.is_on)
                        .label(label)
                        .on_toggle(move |is_on| Message::ToggleSwitched {
                            callback_id,
                            is_on,
                        })
                } else {
                    // No binding or callback - toggler is read-only (no on_toggle)
                    toggler(config.is_on).label(label)
                };

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(tg).padding(padding).into()
                } else {
                    tg.into()
                }
            }

            GuiElementKind::ProgressBar(config) => {
                // Progress bar takes a range and current value
                // We normalize to 0.0..=1.0 for iced
                let pb = progress_bar(0.0..=1.0, config.value);

                // Apply width from style (iced 0.14 uses .length() for width)
                let pb = if let Some(width) = self.style.width {
                    pb.length(width.to_iced())
                } else {
                    pb
                };

                // Apply height from style (iced 0.14 uses .girth() for height)
                let pb = if let Some(height) = self.style.height {
                    pb.girth(height.to_iced())
                } else {
                    pb
                };

                // Wrap in container if padding is needed
                if let Some(padding) = self.style.padding {
                    container(pb).padding(padding).into()
                } else {
                    pb.into()
                }
            }

            GuiElementKind::Image(config) => {
                if let Some(ref path) = config.path {
                    let mut img = Image::new(path.as_str());

                    // Apply content fit
                    img = img.content_fit(config.content_fit.to_iced());

                    // Apply opacity if not fully opaque
                    if config.opacity < 1.0 {
                        img = img.opacity(config.opacity);
                    }

                    // Apply dimensions from config
                    if let Some(w) = config.image_width {
                        img = img.width(w);
                    }
                    if let Some(h) = config.image_height {
                        img = img.height(h);
                    }

                    // Apply dimensions from style (overrides config)
                    if let Some(width) = self.style.width {
                        img = img.width(width.to_iced());
                    }
                    if let Some(height) = self.style.height {
                        img = img.height(height.to_iced());
                    }

                    // Wrap in container if padding is needed
                    if let Some(padding) = self.style.padding {
                        container(img).padding(padding).into()
                    } else {
                        img.into()
                    }
                } else {
                    // No path - render empty space
                    iced::widget::Space::new().into()
                }
            }

            // Conditional and ForEach require state - render empty space without it
            // Use render_with_state() for proper rendering
            GuiElementKind::Conditional(_) | GuiElementKind::ForEach(_) => {
                iced::widget::Space::new().into()
            }

            // DataTable renders a table from DataFrame data
            GuiElementKind::DataTable(config) => {
                self.render_data_table(config)
            }

            GuiElementKind::BarChart(config) => {
                self.render_bar_chart(config)
            }

            GuiElementKind::LineChart(config) => {
                self.render_line_chart(config)
            }

            GuiElementKind::PieChart(config) => {
                self.render_pie_chart(config)
            }

            // OLAP Cube widgets
            GuiElementKind::CubeTable(config) => {
                self.render_cube_table(config)
            }

            GuiElementKind::CubeChart(config) => {
                self.render_cube_chart(config)
            }

            GuiElementKind::DimensionFilter(config) => {
                self.render_dimension_filter(config)
            }

            GuiElementKind::HierarchyNavigator(config) => {
                self.render_hierarchy_navigator(config)
            }

            GuiElementKind::MeasureSelector(config) => {
                self.render_measure_selector(config)
            }

            GuiElementKind::Interactive(config) => {
                self.render_interactive(config)
            }
        }
    }

    /// Render an Interactive element with mouse event handlers
    fn render_interactive(&self, config: &InteractiveConfig) -> Element<'_, Message> {
        // Interactive expects a single child, take the first or use empty space
        let content = if let Some(child) = self.children.first() {
            child.render()
        } else {
            iced::widget::Space::new().into()
        };

        // Create mouse_area wrapper
        let mut area = mouse_area(content);

        // Add event handlers based on config
        if let Some(callback_id) = config.on_press {
            area = area.on_press(Message::MousePress {
                callback_id,
                x: 0.0, // Position will be set by on_move events
                y: 0.0,
            });
        }

        if let Some(callback_id) = config.on_release {
            area = area.on_release(Message::MouseRelease {
                callback_id,
                x: 0.0,
                y: 0.0,
            });
        }

        if let Some(callback_id) = config.on_double_click {
            area = area.on_double_click(Message::MouseDoubleClick {
                callback_id,
                x: 0.0,
                y: 0.0,
            });
        }

        if let Some(callback_id) = config.on_right_press {
            area = area.on_right_press(Message::MouseRightPress {
                callback_id,
                x: 0.0,
                y: 0.0,
            });
        }

        if let Some(callback_id) = config.on_right_release {
            area = area.on_right_release(Message::MouseRightRelease {
                callback_id,
                x: 0.0,
                y: 0.0,
            });
        }

        if let Some(callback_id) = config.on_middle_press {
            area = area.on_middle_press(Message::MouseMiddlePress { callback_id });
        }

        if let Some(callback_id) = config.on_middle_release {
            area = area.on_middle_release(Message::MouseMiddleRelease { callback_id });
        }

        if let Some(callback_id) = config.on_enter {
            area = area.on_enter(Message::MouseEnter { callback_id });
        }

        if let Some(callback_id) = config.on_exit {
            area = area.on_exit(Message::MouseExit { callback_id });
        }

        if let Some(callback_id) = config.on_move {
            area = area.on_move(move |point: Point| Message::MouseMove {
                callback_id,
                x: point.x,
                y: point.y,
            });
        }

        if let Some(callback_id) = config.on_scroll {
            area = area.on_scroll(move |delta| {
                let (delta_x, delta_y) = match delta {
                    iced::mouse::ScrollDelta::Lines { x, y } => (x, y),
                    iced::mouse::ScrollDelta::Pixels { x, y } => (x, y),
                };
                Message::MouseScroll {
                    callback_id,
                    delta_x,
                    delta_y,
                }
            });
        }

        // Set cursor interaction style if specified
        if let Some(cursor_style) = config.cursor_style {
            area = area.interaction(cursor_style.to_iced());
        }

        area.into()
    }

    /// Render a DataTable element using a grid-based layout
    fn render_data_table(&self, config: &DataTableConfig) -> Element<'_, Message> {
        let Some(ref df) = config.dataframe else {
            // No data - show empty placeholder
            return container(text("No data"))
                .padding(20)
                .into();
        };

        // Determine which columns to display
        let columns_to_show: Vec<String> = if let Some(ref cols) = config.columns {
            cols.clone()
        } else {
            df.columns()
        };

        if columns_to_show.is_empty() {
            return container(text("No columns"))
                .padding(20)
                .into();
        }

        // Calculate effective column count (with selection checkbox column)
        let has_selection = config.selectable && config.on_selection_change.is_some();
        let num_columns = if has_selection { columns_to_show.len() + 1 } else { columns_to_show.len() };

        // Calculate pagination
        let total_rows = df.num_rows();
        let page_size = config.page_size.unwrap_or(total_rows);
        let start_row = config.current_page * page_size;
        let end_row = (start_row + page_size).min(total_rows);
        let total_pages = if page_size > 0 { (total_rows + page_size - 1) / page_size } else { 1 };

        // Build header row
        let mut header_cells: Vec<Element<'_, Message>> = Vec::new();

        // Selection column header (empty or "select all")
        if has_selection {
            header_cells.push(
                container(text("☑"))
                    .padding(8)
                    .into()
            );
        }

        // Column headers - clickable if sortable
        for col_name in &columns_to_show {
            let is_sorted = config.sort_column.as_ref() == Some(col_name);
            let sort_indicator = if is_sorted {
                if config.sort_ascending { " ▲" } else { " ▼" }
            } else if config.sortable {
                " ○" // Indicate sortable
            } else {
                ""
            };
            let header_text = format!("{col_name}{sort_indicator}");

            let header_label = text(header_text).font(Font {
                weight: font::Weight::Bold,
                ..Font::default()
            });

            // Get column width if specified
            let col_width = config.column_widths
                .iter()
                .find(|(c, _)| c == col_name)
                .map(|(_, w)| *w);

            // Make header clickable for sorting if sortable and callback is set
            let header_elem: Element<'_, Message> = if config.sortable {
                if let Some(sort_callback) = config.on_sort {
                    let mut header_btn = button(header_label)
                        .on_press(Message::DataTableSort {
                            callback_id: sort_callback,
                            column: col_name.clone(),
                        })
                        .padding([8, 12]);
                    if let Some(w) = col_width {
                        header_btn = header_btn.width(w);
                    }
                    header_btn.into()
                } else {
                    let mut header_container = container(header_label).padding(8);
                    if let Some(w) = col_width {
                        header_container = header_container.width(w);
                    }
                    header_container.into()
                }
            } else {
                let mut header_container = container(header_label).padding(8);
                if let Some(w) = col_width {
                    header_container = header_container.width(w);
                }
                header_container.into()
            };

            header_cells.push(header_elem);
        }

        // Build data rows
        let mut all_cells: Vec<Element<'_, Message>> = header_cells;

        for row_idx in start_row..end_row {
            // Selection checkbox
            if has_selection {
                let is_selected = config.selected_rows.contains(&row_idx);
                if let Some(selection_callback) = config.on_selection_change {
                    // Create a checkbox that toggles this row's selection
                    let mut new_selection = config.selected_rows.clone();
                    if is_selected {
                        new_selection.retain(|&r| r != row_idx);
                    } else {
                        new_selection.push(row_idx);
                    }
                    all_cells.push(
                        container(
                            checkbox(is_selected)
                                .on_toggle(move |_| Message::DataTableRowSelect {
                                    callback_id: selection_callback,
                                    rows: new_selection.clone(),
                                })
                        )
                        .padding(4)
                        .into()
                    );
                } else {
                    all_cells.push(
                        container(checkbox(is_selected))
                            .padding(4)
                            .into()
                    );
                }
            }

            // Data cells
            for col_name in &columns_to_show {
                let value = df.column(col_name)
                    .ok()
                    .and_then(|series| series.get(row_idx).ok())
                    .map(|v| format!("{v}"))
                    .unwrap_or_default();

                // Check for custom cell renderer
                let cell_content: Element<'_, Message> = if let Some((_renderer_col, _renderer_id)) =
                    config.cell_renderers.iter().find(|(c, _)| c == col_name)
                {
                    // Custom cell renderers are callbacks that return GuiElements
                    // For now, render the value as text - full renderer support would need
                    // executor access in render(), which requires architectural changes
                    text(value.clone()).into()
                } else {
                    text(value.clone()).into()
                };

                // Get column width if specified
                let col_width = config.column_widths
                    .iter()
                    .find(|(c, _)| c == col_name)
                    .map(|(_, w)| *w);

                // Build the cell with optional width and click handling
                let cell_elem: Element<'_, Message> = if let Some(cell_callback) = config.on_cell_click {
                    // Cell is clickable
                    let col_name_owned = col_name.clone();
                    let mut cell_btn = button(cell_content)
                        .on_press(Message::DataTableCellClick {
                            callback_id: cell_callback,
                            row: row_idx,
                            column: col_name_owned,
                        })
                        .padding(4);
                    if let Some(w) = col_width {
                        cell_btn = cell_btn.width(w);
                    }
                    cell_btn.into()
                } else if let Some(row_callback) = config.on_row_click {
                    // Row click - make cell clickable too
                    let mut cell_btn = button(cell_content)
                        .on_press(Message::DataTableRowClick {
                            callback_id: row_callback,
                            row: row_idx,
                        })
                        .padding(4);
                    if let Some(w) = col_width {
                        cell_btn = cell_btn.width(w);
                    }
                    cell_btn.into()
                } else {
                    // Non-clickable cell
                    let mut cell_container = container(cell_content).padding(4);
                    if let Some(w) = col_width {
                        cell_container = cell_container.width(w);
                    }
                    cell_container.into()
                };

                all_cells.push(cell_elem);
            }
        }

        // Create grid layout for the table
        let mut grid = Grid::new(num_columns).spacing(1.0);

        if let Some(padding) = self.style.padding {
            grid = grid.padding(padding);
        }
        if let Some(width) = self.style.width {
            grid = grid.width(width);
        }
        if let Some(height) = self.style.height {
            grid = grid.height(height);
        }

        let grid_element = grid.render(all_cells);

        // Build pagination controls
        let pagination = if total_pages > 1 {
            let page_info = text(format!("Page {} of {}", config.current_page + 1, total_pages));
            let row_info = text(format!("Rows {}-{} of {}", start_row + 1, end_row, total_rows));

            // Pagination buttons
            let has_page_callback = config.on_page_change.is_some();
            let current_page = config.current_page;

            let prev_button: Element<'_, Message> = if current_page > 0 && has_page_callback {
                button(text("◀ Prev"))
                    .on_press(Message::DataTablePageChange {
                        callback_id: config.on_page_change.unwrap(),
                        page: current_page - 1,
                    })
                    .padding([4, 8])
                    .into()
            } else {
                button(text("◀ Prev"))
                    .padding([4, 8])
                    .into()
            };

            let next_button: Element<'_, Message> = if current_page + 1 < total_pages && has_page_callback {
                button(text("Next ▶"))
                    .on_press(Message::DataTablePageChange {
                        callback_id: config.on_page_change.unwrap(),
                        page: current_page + 1,
                    })
                    .padding([4, 8])
                    .into()
            } else {
                button(text("Next ▶"))
                    .padding([4, 8])
                    .into()
            };

            Some(
                row![
                    row_info,
                    iced::widget::Space::new().width(Fill),
                    prev_button,
                    page_info,
                    next_button,
                ]
                .spacing(10)
                .padding(8)
                .align_y(iced::Alignment::Center)
            )
        } else {
            None
        };

        // Combine grid and pagination
        if let Some(pagination_row) = pagination {
            column![
                scrollable(grid_element).height(Fill),
                pagination_row,
            ]
            .spacing(4)
            .into()
        } else {
            scrollable(grid_element)
                .height(Fill)
                .into()
        }
    }

    /// Render this element to an iced Element with state access
    ///
    /// This method is required for conditional and list rendering, which need
    /// to read state values to determine what to render.
    pub fn render_with_state(
        &self,
        state: &ReactiveState,
        executor: Option<&CallbackExecutor>,
    ) -> Element<'_, Message> {
        if !self.style.visible {
            return iced::widget::Space::new().into();
        }

        match &self.kind {
            // Conditional rendering
            GuiElementKind::Conditional(config) => {
                // Get the condition value from state
                let condition = state.get_path(&config.condition_field)
                    .map(|v| matches!(v, Value::Bool(true)))
                    .unwrap_or(false);

                if condition {
                    if let Some(ref element) = config.true_element {
                        element.render_with_state(state, executor)
                    } else {
                        iced::widget::Space::new().into()
                    }
                } else if let Some(ref element) = config.false_element {
                    element.render_with_state(state, executor)
                } else {
                    iced::widget::Space::new().into()
                }
            }

            // ForEach rendering - renders any pre-expanded children
            // Note: Dynamic list expansion should be done by the runtime before rendering.
            // The template closure is stored for runtime use, not for rendering.
            GuiElementKind::ForEach(_config) => {
                if self.children.is_empty() {
                    iced::widget::Space::new().into()
                } else {
                    // Render pre-expanded children
                    let children: Vec<Element<'_, Message>> = self.children
                        .iter()
                        .map(|c| c.render_with_state(state, executor))
                        .collect();

                    let vstack = VStack::new();
                    vstack.render(children)
                }
            }

            // For all other element types, delegate to the regular render
            // but use render_with_state for children
            _ => self.render_children_with_state(state, executor),
        }
    }

    /// Render children with state access
    fn render_children_with_state(
        &self,
        state: &ReactiveState,
        executor: Option<&CallbackExecutor>,
    ) -> Element<'_, Message> {
        // This is a simplified version that handles children with state
        // For now, we'll use a macro-like approach to avoid duplicating all the rendering code
        match &self.kind {
            GuiElementKind::VStack(config) => {
                let children: Vec<Element<'_, Message>> = self.children
                    .iter()
                    .map(|c| c.render_with_state(state, executor))
                    .collect();

                let mut vstack = VStack::new()
                    .spacing(config.spacing)
                    .align(config.align);

                if let Some(padding) = self.style.padding {
                    vstack = vstack.padding(padding);
                }
                if let Some(width) = self.style.width {
                    vstack = vstack.width(width);
                }
                if let Some(height) = self.style.height {
                    vstack = vstack.height(height);
                }

                vstack.render(children)
            }

            GuiElementKind::HStack(config) => {
                let children: Vec<Element<'_, Message>> = self.children
                    .iter()
                    .map(|c| c.render_with_state(state, executor))
                    .collect();

                let mut hstack = HStack::new()
                    .spacing(config.spacing)
                    .align(config.align);

                if let Some(padding) = self.style.padding {
                    hstack = hstack.padding(padding);
                }
                if let Some(width) = self.style.width {
                    hstack = hstack.width(width);
                }
                if let Some(height) = self.style.height {
                    hstack = hstack.height(height);
                }

                hstack.render(children)
            }

            GuiElementKind::ZStack(_config) => {
                let children: Vec<Element<'_, Message>> = self.children
                    .iter()
                    .map(|c| c.render_with_state(state, executor))
                    .collect();

                let mut zstack = ZStack::new();

                if let Some(padding) = self.style.padding {
                    zstack = zstack.padding(padding);
                }
                if let Some(width) = self.style.width {
                    zstack = zstack.width(width);
                }
                if let Some(height) = self.style.height {
                    zstack = zstack.height(height);
                }

                zstack.render(children)
            }

            GuiElementKind::Grid(config) => {
                let children: Vec<Element<'_, Message>> = self.children
                    .iter()
                    .map(|c| c.render_with_state(state, executor))
                    .collect();

                let mut grid = Grid::new(config.columns)
                    .spacing(config.spacing)
                    .cell_align_x(config.cell_align_x)
                    .cell_align_y(config.cell_align_y);

                if let Some(padding) = self.style.padding {
                    grid = grid.padding(padding);
                }
                if let Some(width) = self.style.width {
                    grid = grid.width(width);
                }
                if let Some(height) = self.style.height {
                    grid = grid.height(height);
                }

                grid.render(children)
            }

            GuiElementKind::ScrollView(config) => {
                let content = if let Some(child) = self.children.first() {
                    child.render_with_state(state, executor)
                } else {
                    iced::widget::Space::new().into()
                };

                let mut scroll = ScrollView::new().direction(config.direction);

                if let Some(width) = self.style.width {
                    scroll = scroll.width(width);
                }
                if let Some(height) = self.style.height {
                    scroll = scroll.height(height);
                }

                scroll.render(content)
            }

            GuiElementKind::Container(config) => {
                let content = if let Some(child) = self.children.first() {
                    child.render_with_state(state, executor)
                } else {
                    iced::widget::Space::new().into()
                };

                let mut c = Container::new()
                    .align_x(config.align_x)
                    .align_y(config.align_y);

                if config.center_x {
                    c = c.center_x();
                }
                if config.center_y {
                    c = c.center_y();
                }
                if let Some(max_w) = config.max_width {
                    c = c.max_width(max_w);
                }
                if let Some(max_h) = config.max_height {
                    c = c.max_height(max_h);
                }
                if let Some(padding) = self.style.padding {
                    c = c.padding(padding);
                }
                if let Some(width) = self.style.width {
                    c = c.width(width);
                }
                if let Some(height) = self.style.height {
                    c = c.height(height);
                }

                c.render(content)
            }

            // For non-container elements, use the regular render
            _ => self.render(),
        }
    }

    /// Render a BarChart element using iced's canvas widget
    fn render_bar_chart(&self, config: &BarChartConfig) -> Element<'_, Message> {
        let program = BarChartProgram {
            config: config.clone(),
        };

        let width = self.style.width
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config.width));
        let height = self.style.height
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config.height));

        let chart = canvas(program)
            .width(width)
            .height(height);

        if let Some(padding) = self.style.padding {
            container(chart).padding(padding).into()
        } else {
            chart.into()
        }
    }

    /// Render a LineChart element using iced's canvas widget
    fn render_line_chart(&self, config: &LineChartConfig) -> Element<'_, Message> {
        let program = LineChartProgram {
            config: config.clone(),
        };

        let width = self.style.width
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config.width));
        let height = self.style.height
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config.height));

        let chart = canvas(program)
            .width(width)
            .height(height);

        if let Some(padding) = self.style.padding {
            container(chart).padding(padding).into()
        } else {
            chart.into()
        }
    }

    /// Render a PieChart element using iced's canvas widget
    fn render_pie_chart(&self, config: &PieChartConfig) -> Element<'_, Message> {
        let program = PieChartProgram {
            config: config.clone(),
        };

        let width = self.style.width
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config.width));
        let height = self.style.height
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config.height));

        let chart = canvas(program)
            .width(width)
            .height(height);

        if let Some(padding) = self.style.padding {
            container(chart).padding(padding).into()
        } else {
            chart.into()
        }
    }

    // =========================================================================
    // OLAP Cube Widget Rendering
    // =========================================================================

    /// Render a CubeTable element - OLAP-aware data table with drill-down
    fn render_cube_table(&self, config: &CubeTableConfig) -> Element<'_, Message> {
        let Some(ref cube) = config.cube else {
            return container(text("No cube data"))
                .padding(20)
                .into();
        };

        // Clone config values for use in the function
        let on_drill = config.on_drill;
        let on_roll_up = config.on_roll_up;
        let show_drill_controls = config.show_drill_controls;
        let current_page = config.current_page;
        let page_size = config.page_size;
        let on_page_change = config.on_page_change;

        // Get dimensions and measures to display (owned)
        let row_dims: Vec<String> = if config.row_dimensions.is_empty() {
            cube.dimension_names()
        } else {
            config.row_dimensions.clone()
        };

        let measures: Vec<String> = if config.measures.is_empty() {
            cube.measure_names()
        } else {
            config.measures.clone()
        };

        if row_dims.is_empty() && measures.is_empty() {
            return container(text("No dimensions or measures"))
                .padding(20)
                .into();
        }

        // Calculate column count: row dimensions + measures
        let num_columns = row_dims.len() + measures.len();
        let row_dims_len = row_dims.len();
        let measures_len = measures.len();

        // Build header row
        let mut header_cells: Vec<Element<'_, Message>> = Vec::new();

        // Dimension headers with drill-down indicator - use indexed access
        for idx in 0..row_dims_len {
            let dim_name = row_dims[idx].clone();
            let header_text = if show_drill_controls {
                format!("{} ▼", dim_name)
            } else {
                dim_name.clone()
            };

            let header_label = text(header_text).font(Font {
                weight: font::Weight::Bold,
                ..Font::default()
            });

            let header_elem: Element<'_, Message> = if let Some(drill_callback) = on_drill {
                button(header_label)
                    .on_press(Message::CubeDrillDown {
                        callback_id: drill_callback,
                        dimension: dim_name,
                        value: None,
                    })
                    .padding([8, 12])
                    .into()
            } else {
                container(header_label).padding(8).into()
            };

            header_cells.push(header_elem);
        }

        // Measure headers - use indexed access
        for idx in 0..measures_len {
            let measure_name = measures[idx].clone();
            let header_label = text(measure_name).font(Font {
                weight: font::Weight::Bold,
                ..Font::default()
            });
            header_cells.push(container(header_label).padding(8).into());
        }

        // Build data rows - for now, show cube statistics as placeholder
        // Full implementation would query cube data via to_dataframe()
        let mut all_cells: Vec<Element<'_, Message>> = header_cells;

        // Add roll-up control row if enabled
        if show_drill_controls && on_roll_up.is_some() {
            let first_dim = if !row_dims.is_empty() { row_dims[0].clone() } else { String::new() };
            let rollup_btn: Element<'_, Message> = if let Some(rollup_callback) = on_roll_up {
                button(text("▲ Roll Up"))
                    .on_press(Message::CubeRollUp {
                        callback_id: rollup_callback,
                        dimension: first_dim,
                    })
                    .padding([4, 8])
                    .into()
            } else {
                iced::widget::Space::new().into()
            };

            // Span rollup button across all columns
            all_cells.push(rollup_btn);
            for _ in 1..num_columns {
                all_cells.push(iced::widget::Space::new().into());
            }
        }

        // Show cube info row
        let info_text = format!(
            "{} rows, {} dimensions, {} measures",
            cube.row_count(),
            cube.dimension_names().len(),
            cube.measure_names().len()
        );
        all_cells.push(container(text(info_text)).padding(8).into());
        for _ in 1..num_columns {
            all_cells.push(iced::widget::Space::new().into());
        }

        // Create grid layout
        let mut grid = Grid::new(num_columns).spacing(1.0);

        if let Some(padding) = self.style.padding {
            grid = grid.padding(padding);
        }
        if let Some(width) = self.style.width {
            grid = grid.width(width);
        }
        if let Some(height) = self.style.height {
            grid = grid.height(height);
        }

        let grid_element = grid.render(all_cells);

        // Build pagination controls if needed
        if page_size.is_some() && on_page_change.is_some() {
            let page_info = text(format!("Page {}", current_page + 1));
            let pagination_row = row![page_info].spacing(10).padding(8);

            column![
                scrollable(grid_element).height(Fill),
                pagination_row,
            ]
            .spacing(4)
            .into()
        } else {
            scrollable(grid_element)
                .height(Fill)
                .into()
        }
    }

    /// Render a CubeChart element - OLAP-aware chart with drill-down
    fn render_cube_chart(&self, config: &CubeChartConfig) -> Element<'_, Message> {
        let Some(ref cube) = config.cube else {
            return container(text("No cube data"))
                .padding(20)
                .into();
        };

        // Clone all config values upfront
        let title_opt = config.title.clone();
        let chart_type = config.chart_type;
        let series_dimension_opt = config.series_dimension.clone();
        let config_width = config.width;
        let config_height = config.height;

        // Get the dimension and measure names (owned)
        let x_dim = config.x_dimension.clone()
            .or_else(|| cube.dimension_names().first().cloned())
            .unwrap_or_else(|| "dimension".to_string());
        let y_measure = config.y_measure.clone()
            .or_else(|| cube.measure_names().first().cloned())
            .unwrap_or_else(|| "measure".to_string());

        // Get cube stats (owned values)
        let row_count = cube.row_count();
        let dim_count = cube.dimension_names().len();

        // For now, render a placeholder that shows cube info
        // Full implementation would query cube and generate chart data
        let mut content_col = column![].spacing(8);

        // Title
        if let Some(title) = title_opt {
            content_col = content_col.push(
                text(title).font(Font {
                    weight: font::Weight::Bold,
                    ..Font::default()
                }).size(18)
            );
        }

        // Chart type indicator
        let chart_type_str = match chart_type {
            CubeChartType::Bar => "Bar Chart",
            CubeChartType::Line => "Line Chart",
            CubeChartType::Pie => "Pie Chart",
        };

        content_col = content_col.push(
            text(format!("{} - {} by {}", chart_type_str, y_measure, x_dim))
        );

        // Cube info
        content_col = content_col.push(
            text(format!(
                "Data: {} rows across {} dimensions",
                row_count,
                dim_count
            )).size(12)
        );

        // Series info if present
        if let Some(series_dim) = series_dimension_opt {
            content_col = content_col.push(
                text(format!("Grouped by: {}", series_dim)).size(12)
            );
        }

        let width = self.style.width
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config_width));
        let height = self.style.height
            .map(|s| s.to_iced())
            .unwrap_or(Length::Fixed(config_height));

        let chart_container = container(content_col)
            .width(width)
            .height(height)
            .padding(16);

        if let Some(padding) = self.style.padding {
            container(chart_container).padding(padding).into()
        } else {
            chart_container.into()
        }
    }

    /// Render a DimensionFilter element - dropdown for cube dimension filtering
    fn render_dimension_filter(&self, config: &DimensionFilterConfig) -> Element<'_, Message> {
        // Clone all config values upfront
        let label_opt = config.label.clone();
        let show_all_option = config.show_all_option;
        let dimension = config.dimension.clone();
        let selected = config.selected_value.clone();
        let placeholder = config.placeholder.clone().unwrap_or_else(|| "Select...".to_string());
        let on_select = config.on_select;
        let field_path_opt = config.field_path.clone();

        // Get dimension values from cube (owned)
        let mut options: Vec<String> = Vec::new();

        if show_all_option {
            options.push("All".to_string());
        }

        if let Some(ref cube) = config.cube {
            if let Ok(values) = cube.dimension_values(&dimension) {
                for val in values {
                    options.push(format!("{}", val));
                }
            }
        }

        let mut content_col = column![].spacing(4);

        // Label
        if let Some(label) = label_opt {
            content_col = content_col.push(
                text(label).font(Font {
                    weight: font::Weight::Bold,
                    ..Font::default()
                })
            );
        }

        // Create pick_list with selection handler
        let pl = if let Some(callback_id) = on_select {
            let dim_clone = dimension.clone();
            pick_list(options, selected, move |value: String| {
                let actual_value = if value == "All" { None } else { Some(value) };
                Message::CubeDimensionSelect {
                    callback_id,
                    dimension: dim_clone.clone(),
                    value: actual_value,
                }
            })
            .placeholder(placeholder)
        } else if let Some(field) = field_path_opt {
            pick_list(options, selected, move |value: String| {
                Message::SetStringField {
                    field: field.clone(),
                    value,
                }
            })
            .placeholder(placeholder)
        } else {
            pick_list(options, selected, |_: String| Message::NoOp)
                .placeholder(placeholder)
        };

        content_col = content_col.push(pl);

        let result = container(content_col);

        if let Some(padding) = self.style.padding {
            result.padding(padding).into()
        } else {
            result.into()
        }
    }

    /// Render a HierarchyNavigator element - breadcrumb for hierarchy navigation
    fn render_hierarchy_navigator(&self, config: &HierarchyNavigatorConfig) -> Element<'_, Message> {
        // Get hierarchy levels from cube and clone them for ownership
        let levels: Vec<String> = if let Some(ref cube) = config.cube {
            cube.hierarchies_with_levels()
                .iter()
                .find(|(name, _)| name == &config.hierarchy)
                .map(|(_, levels)| levels.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Get owned copies of config values for use in closures
        let hierarchy_name = config.hierarchy.clone();
        let current_level_opt = config.current_level.clone();
        let on_drill_down = config.on_drill_down;
        let on_roll_up = config.on_roll_up;
        let on_level_change = config.on_level_change;
        let label_opt = config.label.clone();

        let mut content_row = row![].spacing(4).align_y(iced::Alignment::Center);

        // Label
        if let Some(label) = label_opt {
            content_row = content_row.push(
                text(format!("{}: ", label)).font(Font {
                    weight: font::Weight::Bold,
                    ..Font::default()
                })
            );
        }

        if levels.is_empty() {
            content_row = content_row.push(text("No hierarchy levels"));
        } else {
            let current_level = current_level_opt
                .unwrap_or_else(|| levels.first().cloned().unwrap_or_default());

            let current_idx = levels.iter().position(|l| l == &current_level).unwrap_or(0);
            let levels_len = levels.len();

            // Roll-up button (if not at top level)
            if current_idx > 0 {
                if let Some(rollup_callback) = on_roll_up {
                    let hierarchy_clone = hierarchy_name.clone();
                    content_row = content_row.push(
                        button(text("◀"))
                            .on_press(Message::CubeRollUp {
                                callback_id: rollup_callback,
                                dimension: hierarchy_clone,
                            })
                            .padding([4, 8])
                    );
                }
            }

            // Level breadcrumbs - use indexed access with owned strings
            for idx in 0..levels_len {
                let level = levels[idx].clone();
                let is_current = idx == current_idx;
                let level_display = level.clone();

                let level_text = if is_current {
                    text(level_display).font(Font {
                        weight: font::Weight::Bold,
                        ..Font::default()
                    })
                } else {
                    text(level_display)
                };

                let level_elem: Element<'_, Message> = if let Some(level_callback) = on_level_change {
                    let hierarchy_clone = hierarchy_name.clone();
                    button(level_text)
                        .on_press(Message::CubeHierarchyLevelChange {
                            callback_id: level_callback,
                            hierarchy: hierarchy_clone,
                            level,
                        })
                        .padding([4, 8])
                        .into()
                } else {
                    container(level_text).padding([4, 8]).into()
                };

                content_row = content_row.push(level_elem);

                // Separator between levels
                if idx < levels_len - 1 {
                    content_row = content_row.push(text(" > "));
                }
            }

            // Drill-down button (if not at bottom level)
            if current_idx < levels_len - 1 {
                if let Some(drill_callback) = on_drill_down {
                    content_row = content_row.push(
                        button(text("▶"))
                            .on_press(Message::CubeDrillDown {
                                callback_id: drill_callback,
                                dimension: hierarchy_name.clone(),
                                value: None,
                            })
                            .padding([4, 8])
                    );
                }
            }
        }

        let result = container(content_row);

        if let Some(padding) = self.style.padding {
            result.padding(padding).into()
        } else {
            result.into()
        }
    }

    /// Render a MeasureSelector element - multi-select for visible measures
    fn render_measure_selector(&self, config: &MeasureSelectorConfig) -> Element<'_, Message> {
        // Clone config values upfront to avoid lifetime issues
        let label_opt = config.label.clone();
        let on_change = config.on_change;
        let selected_measures = config.selected_measures.clone();

        // Get all measures from cube
        let all_measures: Vec<String> = if let Some(ref cube) = config.cube {
            cube.measure_names()
        } else {
            Vec::new()
        };

        let mut content_col = column![].spacing(4);

        // Label
        if let Some(label) = label_opt {
            content_col = content_col.push(
                text(label).font(Font {
                    weight: font::Weight::Bold,
                    ..Font::default()
                })
            );
        }

        if all_measures.is_empty() {
            content_col = content_col.push(text("No measures available"));
        } else {
            // Create a checkbox for each measure - use indexed access with owned strings
            let measures_len = all_measures.len();
            for idx in 0..measures_len {
                let measure = all_measures[idx].clone();
                let measure_label = measure.clone();
                let is_selected = selected_measures.contains(&measure);

                let cb = if let Some(callback_id) = on_change {
                    let measure_name = measure.clone();
                    let base_selection = selected_measures.clone();

                    checkbox(is_selected)
                        .label(measure_label)
                        .on_toggle(move |checked| {
                            let mut updated = base_selection.clone();
                            if checked {
                                if !updated.contains(&measure_name) {
                                    updated.push(measure_name.clone());
                                }
                            } else {
                                let name = measure_name.clone();
                                updated.retain(|m| m != &name);
                            }
                            Message::CubeMeasureSelect {
                                callback_id,
                                measures: updated,
                            }
                        })
                } else {
                    checkbox(is_selected).label(measure_label)
                };

                content_col = content_col.push(cb);
            }
        }

        let result = container(content_col);

        if let Some(padding) = self.style.padding {
            result.padding(padding).into()
        } else {
            result.into()
        }
    }

    /// Convert this GuiElement into a Value for use in Stratum code
    #[must_use]
    pub fn into_value(self) -> Value {
        Value::GuiElement(Arc::new(self))
    }
}

impl GuiValue for GuiElement {
    fn kind_name(&self) -> &'static str {
        match &self.kind {
            GuiElementKind::VStack(_) => "VStack",
            GuiElementKind::HStack(_) => "HStack",
            GuiElementKind::ZStack(_) => "ZStack",
            GuiElementKind::Grid(_) => "Grid",
            GuiElementKind::ScrollView(_) => "ScrollView",
            GuiElementKind::Spacer(_) => "Spacer",
            GuiElementKind::Container(_) => "Container",
            GuiElementKind::Text(_) => "Text",
            GuiElementKind::Button(_) => "Button",
            GuiElementKind::TextField(_) => "TextField",
            GuiElementKind::Checkbox(_) => "Checkbox",
            GuiElementKind::RadioButton(_) => "RadioButton",
            GuiElementKind::Dropdown(_) => "Dropdown",
            GuiElementKind::Slider(_) => "Slider",
            GuiElementKind::Toggle(_) => "Toggle",
            GuiElementKind::ProgressBar(_) => "ProgressBar",
            GuiElementKind::Image(_) => "Image",
            GuiElementKind::Conditional(_) => "Conditional",
            GuiElementKind::ForEach(_) => "ForEach",
            GuiElementKind::DataTable(_) => "DataTable",
            GuiElementKind::BarChart(_) => "BarChart",
            GuiElementKind::LineChart(_) => "LineChart",
            GuiElementKind::PieChart(_) => "PieChart",
            GuiElementKind::CubeTable(_) => "CubeTable",
            GuiElementKind::CubeChart(_) => "CubeChart",
            GuiElementKind::DimensionFilter(_) => "DimensionFilter",
            GuiElementKind::HierarchyNavigator(_) => "HierarchyNavigator",
            GuiElementKind::MeasureSelector(_) => "MeasureSelector",
            GuiElementKind::Interactive(_) => "Interactive",
        }
    }

    fn clone_boxed(&self) -> Arc<dyn GuiValue> {
        Arc::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Builder for constructing GUI elements
#[derive(Clone)]
pub struct GuiElementBuilder {
    kind: GuiElementKind,
    children: Vec<Arc<GuiElement>>,
    style: ElementStyle,
}

impl GuiElementBuilder {
    /// Create a new builder with the given element kind
    fn new(kind: GuiElementKind) -> Self {
        Self {
            kind,
            children: Vec::new(),
            style: ElementStyle::new(),
        }
    }

    /// Add a child element
    #[must_use]
    pub fn child(mut self, child: GuiElement) -> Self {
        self.children.push(Arc::new(child));
        self
    }

    /// Add multiple child elements
    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = GuiElement>) -> Self {
        for child in children {
            self.children.push(Arc::new(child));
        }
        self
    }

    /// Set padding
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.style.padding = Some(padding);
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.style.width = Some(width);
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.style.height = Some(height);
        self
    }

    /// Set visibility
    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.style.visible = visible;
        self
    }

    /// Set spacing (for VStack, HStack, Grid)
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        match &mut self.kind {
            GuiElementKind::VStack(c) => c.spacing = spacing,
            GuiElementKind::HStack(c) => c.spacing = spacing,
            GuiElementKind::Grid(c) => c.spacing = spacing,
            _ => {}
        }
        self
    }

    /// Set horizontal alignment (for VStack, Container)
    #[must_use]
    pub fn align_x(mut self, align: HAlign) -> Self {
        match &mut self.kind {
            GuiElementKind::VStack(c) => c.align = align,
            GuiElementKind::Container(c) => c.align_x = align,
            GuiElementKind::Grid(c) => c.cell_align_x = align,
            _ => {}
        }
        self
    }

    /// Set vertical alignment (for HStack, Container)
    #[must_use]
    pub fn align_y(mut self, align: VAlign) -> Self {
        match &mut self.kind {
            GuiElementKind::HStack(c) => c.align = align,
            GuiElementKind::Container(c) => c.align_y = align,
            GuiElementKind::Grid(c) => c.cell_align_y = align,
            _ => {}
        }
        self
    }

    /// Set text size (for Text elements)
    #[must_use]
    pub fn text_size(mut self, size: f32) -> Self {
        if let GuiElementKind::Text(c) = &mut self.kind {
            c.size = Some(size);
        }
        self
    }

    /// Set bold (for Text elements)
    #[must_use]
    pub fn bold(mut self) -> Self {
        if let GuiElementKind::Text(c) = &mut self.kind {
            c.bold = true;
        }
        self
    }

    /// Set color (for Text elements) - RGBA values 0-255
    #[must_use]
    pub fn color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        if let GuiElementKind::Text(c) = &mut self.kind {
            c.color = Some((r, g, b, a));
        }
        self
    }

    /// Set color with RGB (alpha defaults to 255) (for Text elements)
    #[must_use]
    pub fn color_rgb(mut self, r: u8, g: u8, b: u8) -> Self {
        if let GuiElementKind::Text(c) = &mut self.kind {
            c.color = Some((r, g, b, 255));
        }
        self
    }

    /// Set on_click callback (for Button elements)
    #[must_use]
    pub fn on_click(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Button(c) = &mut self.kind {
            c.on_click = Some(callback_id);
        }
        self
    }

    /// Set disabled state (for Button elements)
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        if let GuiElementKind::Button(c) = &mut self.kind {
            c.disabled = disabled;
        }
        self
    }

    /// Set value (for TextField elements)
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        if let GuiElementKind::TextField(c) = &mut self.kind {
            c.value = value.into();
        }
        self
    }

    /// Set placeholder text (for TextField elements)
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if let GuiElementKind::TextField(c) = &mut self.kind {
            c.placeholder = placeholder.into();
        }
        self
    }

    /// Set secure mode (for TextField elements) - hides text for passwords
    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        if let GuiElementKind::TextField(c) = &mut self.kind {
            c.secure = secure;
        }
        self
    }

    /// Bind to a state field path (for TextField, Checkbox, RadioButton, Dropdown, Slider, and Toggle elements)
    /// The field will automatically update when the user interacts
    #[must_use]
    pub fn bind_field(mut self, field_path: impl Into<String>) -> Self {
        let path = field_path.into();
        match &mut self.kind {
            GuiElementKind::TextField(c) => c.field_path = Some(path),
            GuiElementKind::Checkbox(c) => c.field_path = Some(path),
            GuiElementKind::RadioButton(c) => c.field_path = Some(path),
            GuiElementKind::Dropdown(c) => c.field_path = Some(path),
            GuiElementKind::Slider(c) => c.field_path = Some(path),
            GuiElementKind::Toggle(c) => c.field_path = Some(path),
            _ => {}
        }
        self
    }

    /// Set on_change callback (for TextField and Slider elements)
    #[must_use]
    pub fn on_change(mut self, callback_id: CallbackId) -> Self {
        match &mut self.kind {
            GuiElementKind::TextField(c) => c.on_change = Some(callback_id),
            GuiElementKind::Slider(c) => c.on_change = Some(callback_id),
            _ => {}
        }
        self
    }

    /// Set on_submit callback (for TextField elements)
    #[must_use]
    pub fn on_submit(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::TextField(c) = &mut self.kind {
            c.on_submit = Some(callback_id);
        }
        self
    }

    /// Set checked state (for Checkbox elements)
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        if let GuiElementKind::Checkbox(c) = &mut self.kind {
            c.checked = checked;
        }
        self
    }

    /// Set on_toggle callback (for Checkbox and Toggle elements)
    #[must_use]
    pub fn on_toggle(mut self, callback_id: CallbackId) -> Self {
        match &mut self.kind {
            GuiElementKind::Checkbox(c) => c.on_toggle = Some(callback_id),
            GuiElementKind::Toggle(c) => c.on_toggle = Some(callback_id),
            _ => {}
        }
        self
    }

    /// Set the value this radio button represents (for RadioButton elements)
    #[must_use]
    pub fn radio_value(mut self, value: impl Into<String>) -> Self {
        if let GuiElementKind::RadioButton(c) = &mut self.kind {
            c.value = value.into();
        }
        self
    }

    /// Set the currently selected value for comparison (for RadioButton elements)
    #[must_use]
    pub fn selected_value(mut self, selected: impl Into<String>) -> Self {
        if let GuiElementKind::RadioButton(c) = &mut self.kind {
            c.selected_value = Some(selected.into());
        }
        self
    }

    /// Set on_select callback (for RadioButton and Dropdown elements)
    #[must_use]
    pub fn on_select(mut self, callback_id: CallbackId) -> Self {
        match &mut self.kind {
            GuiElementKind::RadioButton(c) => c.on_select = Some(callback_id),
            GuiElementKind::Dropdown(c) => c.on_select = Some(callback_id),
            GuiElementKind::DimensionFilter(c) => c.on_select = Some(callback_id),
            _ => {}
        }
        self
    }

    /// Set dropdown options (for Dropdown elements)
    #[must_use]
    pub fn options(mut self, options: Vec<String>) -> Self {
        if let GuiElementKind::Dropdown(c) = &mut self.kind {
            c.options = options;
        }
        self
    }

    /// Set the currently selected option (for Dropdown elements)
    #[must_use]
    pub fn selected(mut self, selected: impl Into<String>) -> Self {
        if let GuiElementKind::Dropdown(c) = &mut self.kind {
            c.selected = Some(selected.into());
        }
        self
    }

    /// Set dropdown placeholder text (for Dropdown elements)
    #[must_use]
    pub fn dropdown_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if let GuiElementKind::Dropdown(c) = &mut self.kind {
            c.placeholder = Some(placeholder.into());
        }
        self
    }

    /// Center content (for Container)
    #[must_use]
    pub fn center(mut self) -> Self {
        if let GuiElementKind::Container(c) = &mut self.kind {
            c.center_x = true;
            c.center_y = true;
        }
        self
    }

    /// Set max width (for Container)
    #[must_use]
    pub fn max_width(mut self, max_width: f32) -> Self {
        if let GuiElementKind::Container(c) = &mut self.kind {
            c.max_width = Some(max_width);
        }
        self
    }

    /// Set max height (for Container)
    #[must_use]
    pub fn max_height(mut self, max_height: f32) -> Self {
        if let GuiElementKind::Container(c) = &mut self.kind {
            c.max_height = Some(max_height);
        }
        self
    }

    /// Set scroll direction (for ScrollView)
    #[must_use]
    pub fn scroll_direction(mut self, direction: ScrollDirection) -> Self {
        if let GuiElementKind::ScrollView(c) = &mut self.kind {
            c.direction = direction;
        }
        self
    }

    // ==================== Slider builder methods ====================

    /// Set slider value (for Slider elements)
    #[must_use]
    pub fn slider_value(mut self, value: f64) -> Self {
        if let GuiElementKind::Slider(c) = &mut self.kind {
            c.value = value.clamp(c.min, c.max);
        }
        self
    }

    /// Set slider range (for Slider elements)
    #[must_use]
    pub fn slider_range(mut self, min: f64, max: f64) -> Self {
        if let GuiElementKind::Slider(c) = &mut self.kind {
            c.min = min;
            c.max = max;
            // Clamp value to new range
            c.value = c.value.clamp(min, max);
        }
        self
    }

    /// Set slider step size (for Slider elements)
    #[must_use]
    pub fn slider_step(mut self, step: f64) -> Self {
        if let GuiElementKind::Slider(c) = &mut self.kind {
            c.step = step;
        }
        self
    }

    /// Set on_release callback (for Slider elements)
    #[must_use]
    pub fn on_release(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Slider(c) = &mut self.kind {
            c.on_release = Some(callback_id);
        }
        self
    }

    // ==================== Toggle builder methods ====================

    /// Set toggle state (for Toggle elements)
    #[must_use]
    pub fn is_on(mut self, is_on: bool) -> Self {
        if let GuiElementKind::Toggle(c) = &mut self.kind {
            c.is_on = is_on;
        }
        self
    }

    /// Set toggle label (for Toggle elements)
    #[must_use]
    pub fn toggle_label(mut self, label: impl Into<String>) -> Self {
        if let GuiElementKind::Toggle(c) = &mut self.kind {
            c.label = label.into();
        }
        self
    }

    // ==================== ProgressBar builder methods ====================

    /// Set progress value (for ProgressBar elements, 0.0 to 1.0)
    #[must_use]
    pub fn progress(mut self, value: f32) -> Self {
        if let GuiElementKind::ProgressBar(c) = &mut self.kind {
            c.value = value.clamp(0.0, 1.0);
        }
        self
    }

    // ==================== Image builder methods ====================

    /// Set image path (for Image elements)
    #[must_use]
    pub fn image_path(mut self, path: impl Into<String>) -> Self {
        if let GuiElementKind::Image(c) = &mut self.kind {
            c.path = Some(path.into());
        }
        self
    }

    /// Set image content fit mode (for Image elements)
    #[must_use]
    pub fn content_fit(mut self, fit: ImageContentFit) -> Self {
        if let GuiElementKind::Image(c) = &mut self.kind {
            c.content_fit = fit;
        }
        self
    }

    /// Set image opacity (for Image elements, 0.0 to 1.0)
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        if let GuiElementKind::Image(c) = &mut self.kind {
            c.opacity = opacity.clamp(0.0, 1.0);
        }
        self
    }

    /// Set image dimensions (for Image elements)
    #[must_use]
    pub fn image_dimensions(mut self, width: f32, height: f32) -> Self {
        if let GuiElementKind::Image(c) = &mut self.kind {
            c.image_width = Some(width);
            c.image_height = Some(height);
        }
        self
    }

    // ==================== Conditional builder methods ====================

    /// Set the element to render when condition is true (for Conditional elements)
    #[must_use]
    pub fn true_element(mut self, element: GuiElement) -> Self {
        if let GuiElementKind::Conditional(c) = &mut self.kind {
            c.true_element = Some(Arc::new(element));
        }
        self
    }

    /// Set the element to render when condition is false (for Conditional elements)
    #[must_use]
    pub fn false_element(mut self, element: GuiElement) -> Self {
        if let GuiElementKind::Conditional(c) = &mut self.kind {
            c.false_element = Some(Arc::new(element));
        }
        self
    }

    // ==================== ForEach builder methods ====================

    /// Set the template callback ID for ForEach elements
    #[must_use]
    pub fn template_id(mut self, id: CallbackId) -> Self {
        if let GuiElementKind::ForEach(c) = &mut self.kind {
            c.template_id = Some(id);
        }
        self
    }

    /// Set the key function callback ID for efficient list updates (for ForEach elements)
    #[must_use]
    pub fn key_fn_id(mut self, id: CallbackId) -> Self {
        if let GuiElementKind::ForEach(c) = &mut self.kind {
            c.key_fn_id = Some(id);
        }
        self
    }

    // ==================== DataTable builder methods ====================

    /// Set the DataFrame for the data table (for DataTable elements)
    #[must_use]
    pub fn dataframe(mut self, df: Arc<DataFrame>) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.dataframe = Some(df);
        }
        self
    }

    /// Set which columns to display (for DataTable elements)
    /// If not set, all columns will be displayed
    #[must_use]
    pub fn table_columns(mut self, columns: Vec<String>) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.columns = Some(columns);
        }
        self
    }

    /// Set the page size for pagination (for DataTable elements)
    /// Set to None to show all rows
    #[must_use]
    pub fn page_size(mut self, size: Option<usize>) -> Self {
        match &mut self.kind {
            GuiElementKind::DataTable(c) => c.page_size = size,
            GuiElementKind::CubeTable(c) => c.page_size = size,
            _ => {}
        }
        self
    }

    /// Set the current page (for DataTable elements, 0-indexed)
    #[must_use]
    pub fn current_page(mut self, page: usize) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.current_page = page;
        }
        self
    }

    /// Enable or disable column sorting (for DataTable elements)
    #[must_use]
    pub fn sortable(mut self, sortable: bool) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.sortable = sortable;
        }
        self
    }

    /// Set the column to sort by (for DataTable elements)
    #[must_use]
    pub fn sort_by(mut self, column: impl Into<String>, ascending: bool) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.sort_column = Some(column.into());
            c.sort_ascending = ascending;
        }
        self
    }

    /// Enable or disable row selection (for DataTable elements)
    #[must_use]
    pub fn selectable(mut self, selectable: bool) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.selectable = selectable;
        }
        self
    }

    /// Set the currently selected rows (for DataTable elements)
    #[must_use]
    pub fn selected_rows(mut self, rows: Vec<usize>) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.selected_rows = rows;
        }
        self
    }

    /// Set a custom width for a specific column (for DataTable elements)
    #[must_use]
    pub fn column_width(mut self, column: impl Into<String>, width: f32) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.column_widths.push((column.into(), width));
        }
        self
    }

    /// Set callback for row clicks (for DataTable elements)
    #[must_use]
    pub fn on_row_click(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.on_row_click = Some(callback_id);
        }
        self
    }

    /// Set callback for cell clicks (for DataTable elements)
    #[must_use]
    pub fn on_cell_click(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.on_cell_click = Some(callback_id);
        }
        self
    }

    /// Set callback for sort changes (for DataTable elements)
    #[must_use]
    pub fn on_sort(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.on_sort = Some(callback_id);
        }
        self
    }

    /// Set callback for page changes (for DataTable elements)
    #[must_use]
    pub fn on_page_change(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.on_page_change = Some(callback_id);
        }
        self
    }

    /// Set callback for selection changes (for DataTable elements)
    #[must_use]
    pub fn on_selection_change(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.on_selection_change = Some(callback_id);
        }
        self
    }

    /// Set a custom cell renderer for a specific column (for DataTable elements)
    #[must_use]
    pub fn cell_renderer(mut self, column: impl Into<String>, callback_id: CallbackId) -> Self {
        if let GuiElementKind::DataTable(c) = &mut self.kind {
            c.cell_renderers.push((column.into(), callback_id));
        }
        self
    }

    // ========== Chart Builder Methods ==========

    /// Set the chart title (for BarChart, LineChart, PieChart)
    #[must_use]
    pub fn chart_title(mut self, title: impl Into<String>) -> Self {
        match &mut self.kind {
            GuiElementKind::BarChart(c) => c.title = Some(title.into()),
            GuiElementKind::LineChart(c) => c.title = Some(title.into()),
            GuiElementKind::PieChart(c) => c.title = Some(title.into()),
            _ => {}
        }
        self
    }

    /// Set chart data points (for BarChart, PieChart)
    #[must_use]
    pub fn chart_data(mut self, data: Vec<DataPoint>) -> Self {
        match &mut self.kind {
            GuiElementKind::BarChart(c) => c.data = data,
            GuiElementKind::PieChart(c) => c.data = data,
            _ => {}
        }
        self
    }

    /// Set chart size (for BarChart, LineChart, PieChart)
    #[must_use]
    pub fn chart_size(mut self, width: f32, height: f32) -> Self {
        match &mut self.kind {
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
            _ => {}
        }
        self
    }

    /// Show or hide legend (for BarChart, LineChart, PieChart)
    #[must_use]
    pub fn show_legend(mut self, show: bool) -> Self {
        match &mut self.kind {
            GuiElementKind::BarChart(c) => c.show_legend = show,
            GuiElementKind::LineChart(c) => c.show_legend = show,
            GuiElementKind::PieChart(c) => c.show_legend = show,
            _ => {}
        }
        self
    }

    /// Show or hide grid lines (for BarChart, LineChart)
    #[must_use]
    pub fn show_grid(mut self, show: bool) -> Self {
        match &mut self.kind {
            GuiElementKind::BarChart(c) => c.show_grid = show,
            GuiElementKind::LineChart(c) => c.show_grid = show,
            _ => {}
        }
        self
    }

    /// Show or hide value labels on bars (for BarChart)
    #[must_use]
    pub fn show_values(mut self, show: bool) -> Self {
        if let GuiElementKind::BarChart(c) = &mut self.kind {
            c.show_values = show;
        }
        self
    }

    /// Set bar color (for BarChart)
    #[must_use]
    pub fn bar_color(mut self, r: u8, g: u8, b: u8) -> Self {
        if let GuiElementKind::BarChart(c) = &mut self.kind {
            c.bar_color = Some((r, g, b));
        }
        self
    }

    /// Set x-axis label (for BarChart, LineChart)
    #[must_use]
    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        match &mut self.kind {
            GuiElementKind::BarChart(c) => c.x_label = Some(label.into()),
            GuiElementKind::LineChart(c) => c.x_label = Some(label.into()),
            _ => {}
        }
        self
    }

    /// Set y-axis label (for BarChart, LineChart)
    #[must_use]
    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        match &mut self.kind {
            GuiElementKind::BarChart(c) => c.y_label = Some(label.into()),
            GuiElementKind::LineChart(c) => c.y_label = Some(label.into()),
            _ => {}
        }
        self
    }

    /// Set x-axis labels (for LineChart)
    #[must_use]
    pub fn line_labels(mut self, labels: Vec<String>) -> Self {
        if let GuiElementKind::LineChart(c) = &mut self.kind {
            c.labels = labels;
        }
        self
    }

    /// Add a data series (for LineChart)
    #[must_use]
    pub fn add_series(mut self, series: DataSeries) -> Self {
        if let GuiElementKind::LineChart(c) = &mut self.kind {
            c.series.push(series);
        }
        self
    }

    /// Set all data series (for LineChart)
    #[must_use]
    pub fn line_series(mut self, series: Vec<DataSeries>) -> Self {
        if let GuiElementKind::LineChart(c) = &mut self.kind {
            c.series = series;
        }
        self
    }

    /// Show or hide data points on lines (for LineChart)
    #[must_use]
    pub fn show_points(mut self, show: bool) -> Self {
        if let GuiElementKind::LineChart(c) = &mut self.kind {
            c.show_points = show;
        }
        self
    }

    /// Enable area fill under lines (for LineChart)
    #[must_use]
    pub fn fill_area(mut self, fill: bool) -> Self {
        if let GuiElementKind::LineChart(c) = &mut self.kind {
            c.fill_area = fill;
        }
        self
    }

    /// Set series colors (for LineChart)
    #[must_use]
    pub fn series_colors(mut self, colors: Vec<(u8, u8, u8)>) -> Self {
        if let GuiElementKind::LineChart(c) = &mut self.kind {
            c.series_colors = colors;
        }
        self
    }

    /// Show or hide percentage labels (for PieChart)
    #[must_use]
    pub fn show_percentages(mut self, show: bool) -> Self {
        if let GuiElementKind::PieChart(c) = &mut self.kind {
            c.show_percentages = show;
        }
        self
    }

    /// Set slice colors (for PieChart)
    #[must_use]
    pub fn slice_colors(mut self, colors: Vec<(u8, u8, u8)>) -> Self {
        if let GuiElementKind::PieChart(c) = &mut self.kind {
            c.slice_colors = colors;
        }
        self
    }

    /// Set inner radius ratio for donut chart (for PieChart, 0.0 = regular pie)
    #[must_use]
    pub fn inner_radius(mut self, ratio: f32) -> Self {
        if let GuiElementKind::PieChart(c) = &mut self.kind {
            c.inner_radius_ratio = ratio.clamp(0.0, 0.9);
        }
        self
    }

    // =========================================================================
    // OLAP Cube Widget Builder Methods
    // =========================================================================

    /// Set the cube for OLAP widgets
    #[must_use]
    pub fn cube(mut self, cube: Arc<stratum_core::data::Cube>) -> Self {
        match &mut self.kind {
            GuiElementKind::CubeTable(c) => c.cube = Some(cube),
            GuiElementKind::CubeChart(c) => c.cube = Some(cube),
            GuiElementKind::DimensionFilter(c) => c.cube = Some(cube),
            GuiElementKind::HierarchyNavigator(c) => c.cube = Some(cube),
            GuiElementKind::MeasureSelector(c) => c.cube = Some(cube),
            _ => {}
        }
        self
    }

    /// Set row dimensions for CubeTable
    #[must_use]
    pub fn row_dimensions(mut self, dims: Vec<String>) -> Self {
        if let GuiElementKind::CubeTable(c) = &mut self.kind {
            c.row_dimensions = dims;
        }
        self
    }

    /// Set column dimensions for CubeTable (pivot)
    #[must_use]
    pub fn column_dimensions(mut self, dims: Vec<String>) -> Self {
        if let GuiElementKind::CubeTable(c) = &mut self.kind {
            c.column_dimensions = dims;
        }
        self
    }

    /// Set measures for CubeTable or MeasureSelector
    #[must_use]
    pub fn measures(mut self, measures: Vec<String>) -> Self {
        match &mut self.kind {
            GuiElementKind::CubeTable(c) => c.measures = measures,
            GuiElementKind::MeasureSelector(c) => c.selected_measures = measures,
            _ => {}
        }
        self
    }

    /// Set whether to show drill controls for CubeTable
    #[must_use]
    pub fn show_drill_controls(mut self, show: bool) -> Self {
        if let GuiElementKind::CubeTable(c) = &mut self.kind {
            c.show_drill_controls = show;
        }
        self
    }

    /// Set drill callback for CubeTable or HierarchyNavigator
    #[must_use]
    pub fn on_drill(mut self, callback_id: CallbackId) -> Self {
        match &mut self.kind {
            GuiElementKind::CubeTable(c) => c.on_drill = Some(callback_id),
            GuiElementKind::HierarchyNavigator(c) => c.on_drill_down = Some(callback_id),
            _ => {}
        }
        self
    }

    /// Set roll-up callback for CubeTable or HierarchyNavigator
    #[must_use]
    pub fn on_roll_up(mut self, callback_id: CallbackId) -> Self {
        match &mut self.kind {
            GuiElementKind::CubeTable(c) => c.on_roll_up = Some(callback_id),
            GuiElementKind::HierarchyNavigator(c) => c.on_roll_up = Some(callback_id),
            _ => {}
        }
        self
    }

    /// Set chart type for CubeChart
    #[must_use]
    pub fn cube_chart_type(mut self, chart_type: CubeChartType) -> Self {
        if let GuiElementKind::CubeChart(c) = &mut self.kind {
            c.chart_type = chart_type;
        }
        self
    }

    /// Set X dimension for CubeChart
    #[must_use]
    pub fn x_dimension(mut self, dim: impl Into<String>) -> Self {
        if let GuiElementKind::CubeChart(c) = &mut self.kind {
            c.x_dimension = Some(dim.into());
        }
        self
    }

    /// Set Y measure for CubeChart
    #[must_use]
    pub fn y_measure(mut self, measure: impl Into<String>) -> Self {
        if let GuiElementKind::CubeChart(c) = &mut self.kind {
            c.y_measure = Some(measure.into());
        }
        self
    }

    /// Set series dimension for CubeChart (for grouped charts)
    #[must_use]
    pub fn series_dimension(mut self, dim: impl Into<String>) -> Self {
        if let GuiElementKind::CubeChart(c) = &mut self.kind {
            c.series_dimension = Some(dim.into());
        }
        self
    }

    /// Set dimension for DimensionFilter
    #[must_use]
    pub fn filter_dimension(mut self, dim: impl Into<String>) -> Self {
        if let GuiElementKind::DimensionFilter(c) = &mut self.kind {
            c.dimension = dim.into();
        }
        self
    }

    /// Set whether to show "All" option in DimensionFilter
    #[must_use]
    pub fn show_all_option(mut self, show: bool) -> Self {
        if let GuiElementKind::DimensionFilter(c) = &mut self.kind {
            c.show_all_option = show;
        }
        self
    }

    /// Set the hierarchy for HierarchyNavigator
    #[must_use]
    pub fn hierarchy(mut self, hierarchy: impl Into<String>) -> Self {
        if let GuiElementKind::HierarchyNavigator(c) = &mut self.kind {
            c.hierarchy = hierarchy.into();
        }
        self
    }

    /// Set the current level for HierarchyNavigator
    #[must_use]
    pub fn current_level(mut self, level: impl Into<String>) -> Self {
        if let GuiElementKind::HierarchyNavigator(c) = &mut self.kind {
            c.current_level = Some(level.into());
        }
        self
    }

    /// Set the level change callback for HierarchyNavigator
    #[must_use]
    pub fn on_level_change(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::HierarchyNavigator(c) = &mut self.kind {
            c.on_level_change = Some(callback_id);
        }
        self
    }

    /// Set label for OLAP widgets
    #[must_use]
    pub fn cube_label(mut self, label: impl Into<String>) -> Self {
        match &mut self.kind {
            GuiElementKind::DimensionFilter(c) => c.label = Some(label.into()),
            GuiElementKind::HierarchyNavigator(c) => c.label = Some(label.into()),
            GuiElementKind::MeasureSelector(c) => c.label = Some(label.into()),
            _ => {}
        }
        self
    }

    // =======================================================================
    // Widget Styling Methods
    // =======================================================================

    /// Set background color using RGBA values
    #[must_use]
    pub fn background(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.style.widget_style.background = Some(StratumColor::rgba(r, g, b, a));
        self
    }

    /// Set background color using RGB values (opaque)
    #[must_use]
    pub fn background_rgb(mut self, r: u8, g: u8, b: u8) -> Self {
        self.style.widget_style.background = Some(StratumColor::rgb(r, g, b));
        self
    }

    /// Set background color from a Color
    #[must_use]
    pub fn background_color(mut self, color: StratumColor) -> Self {
        self.style.widget_style.background = Some(color);
        self
    }

    /// Set foreground/text color using RGBA values
    #[must_use]
    pub fn foreground(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.style.widget_style.foreground = Some(StratumColor::rgba(r, g, b, a));
        self
    }

    /// Set foreground/text color using RGB values (opaque)
    #[must_use]
    pub fn foreground_rgb(mut self, r: u8, g: u8, b: u8) -> Self {
        self.style.widget_style.foreground = Some(StratumColor::rgb(r, g, b));
        self
    }

    /// Set foreground color from a Color
    #[must_use]
    pub fn foreground_color(mut self, color: StratumColor) -> Self {
        self.style.widget_style.foreground = Some(color);
        self
    }

    /// Set border color using RGBA values
    #[must_use]
    pub fn border_color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.style.widget_style.border_color = Some(StratumColor::rgba(r, g, b, a));
        self
    }

    /// Set border color from a Color
    #[must_use]
    pub fn border_color_value(mut self, color: StratumColor) -> Self {
        self.style.widget_style.border_color = Some(color);
        self
    }

    /// Set border width
    #[must_use]
    pub fn border_width(mut self, width: f32) -> Self {
        self.style.widget_style.border_width = Some(width);
        self
    }

    /// Set corner radius for rounded corners
    #[must_use]
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.style.widget_style.corner_radius = Some(radius);
        self
    }

    /// Apply a complete widget style
    #[must_use]
    pub fn widget_style(mut self, style: WidgetStyle) -> Self {
        self.style.widget_style = style;
        self
    }

    // ==================== Interactive builder methods ====================

    /// Set callback for left mouse button press (for Interactive elements)
    #[must_use]
    pub fn on_press(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_press = Some(callback_id);
        }
        self
    }

    /// Set callback for left mouse button release (for Interactive elements)
    #[must_use]
    pub fn on_mouse_release(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_release = Some(callback_id);
        }
        self
    }

    /// Set callback for double-click (for Interactive elements)
    #[must_use]
    pub fn on_double_click(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_double_click = Some(callback_id);
        }
        self
    }

    /// Set callback for right mouse button press (for Interactive elements)
    #[must_use]
    pub fn on_right_press(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_right_press = Some(callback_id);
        }
        self
    }

    /// Set callback for right mouse button release (for Interactive elements)
    #[must_use]
    pub fn on_right_release(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_right_release = Some(callback_id);
        }
        self
    }

    /// Set callback for middle mouse button press (for Interactive elements)
    #[must_use]
    pub fn on_middle_press(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_middle_press = Some(callback_id);
        }
        self
    }

    /// Set callback for middle mouse button release (for Interactive elements)
    #[must_use]
    pub fn on_middle_release(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_middle_release = Some(callback_id);
        }
        self
    }

    /// Set callback for mouse entering the element area (for Interactive elements)
    #[must_use]
    pub fn on_hover_enter(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_enter = Some(callback_id);
        }
        self
    }

    /// Set callback for mouse exiting the element area (for Interactive elements)
    #[must_use]
    pub fn on_hover_exit(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_exit = Some(callback_id);
        }
        self
    }

    /// Set callback for mouse movement within the element area (for Interactive elements)
    #[must_use]
    pub fn on_mouse_move(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_move = Some(callback_id);
        }
        self
    }

    /// Set callback for mouse scroll within the element area (for Interactive elements)
    #[must_use]
    pub fn on_mouse_scroll(mut self, callback_id: CallbackId) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.on_scroll = Some(callback_id);
        }
        self
    }

    /// Set cursor style when hovering (for Interactive elements)
    #[must_use]
    pub fn cursor(mut self, style: CursorStyle) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.cursor_style = Some(style);
        }
        self
    }

    /// Set cursor style by name (for Interactive elements)
    /// Accepts: "pointer", "hand", "text", "crosshair", "move", "grab", "grabbing", "not-allowed"
    #[must_use]
    pub fn cursor_name(mut self, name: &str) -> Self {
        if let GuiElementKind::Interactive(c) = &mut self.kind {
            c.cursor_style = Some(CursorStyle::from_str(name));
        }
        self
    }

    /// Build the final GuiElement
    #[must_use]
    pub fn build(self) -> GuiElement {
        GuiElement {
            kind: self.kind,
            children: self.children,
            style: self.style,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vstack_builder() {
        let element = GuiElement::vstack()
            .spacing(10.0)
            .padding(8.0)
            .child(GuiElement::text("Hello").build())
            .child(GuiElement::text("World").build())
            .build();

        assert!(matches!(element.kind, GuiElementKind::VStack(_)));
        assert_eq!(element.children.len(), 2);
        assert_eq!(element.style.padding, Some(8.0));
    }

    #[test]
    fn test_hstack_builder() {
        let element = GuiElement::hstack_with_spacing(16.0)
            .child(GuiElement::button("OK").build())
            .child(GuiElement::button("Cancel").build())
            .build();

        if let GuiElementKind::HStack(config) = &element.kind {
            assert_eq!(config.spacing, 16.0);
        } else {
            panic!("Expected HStack");
        }
        assert_eq!(element.children.len(), 2);
    }

    #[test]
    fn test_grid_builder() {
        let element = GuiElement::grid(3)
            .spacing(4.0)
            .children([
                GuiElement::text("1").build(),
                GuiElement::text("2").build(),
                GuiElement::text("3").build(),
            ])
            .build();

        if let GuiElementKind::Grid(config) = &element.kind {
            assert_eq!(config.columns, 3);
            assert_eq!(config.spacing, 4.0);
        } else {
            panic!("Expected Grid");
        }
    }

    #[test]
    fn test_text_builder() {
        let element = GuiElement::text("Hello World")
            .text_size(24.0)
            .bold()
            .build();

        if let GuiElementKind::Text(config) = &element.kind {
            assert_eq!(config.content, "Hello World");
            assert_eq!(config.size, Some(24.0));
            assert!(config.bold);
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_text_color() {
        let element = GuiElement::text("Colored")
            .color(255, 0, 0, 255)
            .build();

        if let GuiElementKind::Text(config) = &element.kind {
            assert_eq!(config.color, Some((255, 0, 0, 255)));
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_text_color_rgb() {
        let element = GuiElement::text("Blue Text")
            .color_rgb(0, 0, 255)
            .build();

        if let GuiElementKind::Text(config) = &element.kind {
            assert_eq!(config.color, Some((0, 0, 255, 255)));
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_text_full_styling() {
        let element = GuiElement::text("Styled")
            .text_size(32.0)
            .bold()
            .color_rgb(0, 128, 255)
            .padding(8.0)
            .build();

        if let GuiElementKind::Text(config) = &element.kind {
            assert_eq!(config.content, "Styled");
            assert_eq!(config.size, Some(32.0));
            assert!(config.bold);
            assert_eq!(config.color, Some((0, 128, 255, 255)));
        } else {
            panic!("Expected Text");
        }
        assert_eq!(element.style.padding, Some(8.0));
    }

    #[test]
    fn test_button_builder() {
        let element = GuiElement::button("Click Me")
            .padding(12.0)
            .disabled(true)
            .build();

        if let GuiElementKind::Button(config) = &element.kind {
            assert_eq!(config.label, "Click Me");
            assert!(config.disabled);
        } else {
            panic!("Expected Button");
        }
    }

    #[test]
    fn test_container_builder() {
        let element = GuiElement::container()
            .center()
            .max_width(800.0)
            .child(GuiElement::text("Centered").build())
            .build();

        if let GuiElementKind::Container(config) = &element.kind {
            assert!(config.center_x);
            assert!(config.center_y);
            assert_eq!(config.max_width, Some(800.0));
        } else {
            panic!("Expected Container");
        }
    }

    #[test]
    fn test_spacer_variants() {
        let h_spacer = GuiElement::horizontal_spacer().build();
        let v_spacer = GuiElement::vertical_spacer().build();

        if let GuiElementKind::Spacer(config) = &h_spacer.kind {
            assert_eq!(config.width, Some(Size::Fill));
            assert_eq!(config.height, Some(Size::Fixed(0.0)));
        } else {
            panic!("Expected Spacer");
        }

        if let GuiElementKind::Spacer(config) = &v_spacer.kind {
            assert_eq!(config.width, Some(Size::Fixed(0.0)));
            assert_eq!(config.height, Some(Size::Fill));
        } else {
            panic!("Expected Spacer");
        }
    }

    #[test]
    fn test_nested_elements() {
        let element = GuiElement::vstack()
            .spacing(16.0)
            .child(
                GuiElement::hstack()
                    .spacing(8.0)
                    .child(GuiElement::text("Name:").build())
                    .child(GuiElement::text("John").build())
                    .build(),
            )
            .child(
                GuiElement::hstack()
                    .spacing(8.0)
                    .child(GuiElement::button("Save").build())
                    .child(GuiElement::button("Cancel").build())
                    .build(),
            )
            .build();

        assert_eq!(element.children.len(), 2);
        assert_eq!(element.children[0].children.len(), 2);
        assert_eq!(element.children[1].children.len(), 2);
    }

    #[test]
    fn test_visibility() {
        let hidden = GuiElement::text("Hidden").visible(false).build();
        assert!(!hidden.style.visible);

        let visible = GuiElement::text("Visible").build();
        assert!(visible.style.visible);
    }

    #[test]
    fn test_scroll_view() {
        let element = GuiElement::scroll_view()
            .scroll_direction(ScrollDirection::Both)
            .width(Size::Fill)
            .height(Size::Fixed(400.0))
            .child(GuiElement::vstack().build())
            .build();

        if let GuiElementKind::ScrollView(config) = &element.kind {
            assert_eq!(config.direction, ScrollDirection::Both);
        } else {
            panic!("Expected ScrollView");
        }
        assert_eq!(element.style.width, Some(Size::Fill));
        assert_eq!(element.style.height, Some(Size::Fixed(400.0)));
    }

    #[test]
    fn test_text_field_builder() {
        let element = GuiElement::text_field().build();

        if let GuiElementKind::TextField(config) = &element.kind {
            assert_eq!(config.value, "");
            assert_eq!(config.placeholder, "");
            assert!(!config.secure);
            assert!(config.field_path.is_none());
        } else {
            panic!("Expected TextField");
        }
    }

    #[test]
    fn test_text_field_with_value() {
        let element = GuiElement::text_field_with_value("initial")
            .placeholder("Enter text")
            .build();

        if let GuiElementKind::TextField(config) = &element.kind {
            assert_eq!(config.value, "initial");
            assert_eq!(config.placeholder, "Enter text");
        } else {
            panic!("Expected TextField");
        }
    }

    #[test]
    fn test_text_field_secure() {
        let element = GuiElement::text_field()
            .placeholder("Password")
            .secure(true)
            .build();

        if let GuiElementKind::TextField(config) = &element.kind {
            assert!(config.secure);
            assert_eq!(config.placeholder, "Password");
        } else {
            panic!("Expected TextField");
        }
    }

    #[test]
    fn test_text_field_bind_field() {
        let element = GuiElement::text_field()
            .bind_field("user.name")
            .build();

        if let GuiElementKind::TextField(config) = &element.kind {
            assert_eq!(config.field_path, Some("user.name".to_string()));
        } else {
            panic!("Expected TextField");
        }
    }

    #[test]
    fn test_text_field_full_config() {
        let callback_id = CallbackId::new(42);
        let element = GuiElement::text_field_with_value("hello")
            .placeholder("Type here")
            .secure(false)
            .bind_field("state.text")
            .on_submit(callback_id)
            .padding(8.0)
            .build();

        if let GuiElementKind::TextField(config) = &element.kind {
            assert_eq!(config.value, "hello");
            assert_eq!(config.placeholder, "Type here");
            assert!(!config.secure);
            assert_eq!(config.field_path, Some("state.text".to_string()));
            assert_eq!(config.on_submit, Some(callback_id));
        } else {
            panic!("Expected TextField");
        }
        assert_eq!(element.style.padding, Some(8.0));
    }

    #[test]
    fn test_checkbox_builder() {
        let element = GuiElement::checkbox("I agree").build();

        if let GuiElementKind::Checkbox(config) = &element.kind {
            assert_eq!(config.label, "I agree");
            assert!(!config.checked);
            assert!(config.field_path.is_none());
            assert!(config.on_toggle.is_none());
        } else {
            panic!("Expected Checkbox");
        }
    }

    #[test]
    fn test_checkbox_with_state() {
        let element = GuiElement::checkbox_with_state("Accept terms", true).build();

        if let GuiElementKind::Checkbox(config) = &element.kind {
            assert_eq!(config.label, "Accept terms");
            assert!(config.checked);
        } else {
            panic!("Expected Checkbox");
        }
    }

    #[test]
    fn test_checkbox_checked() {
        let element = GuiElement::checkbox("Option")
            .checked(true)
            .build();

        if let GuiElementKind::Checkbox(config) = &element.kind {
            assert!(config.checked);
        } else {
            panic!("Expected Checkbox");
        }
    }

    #[test]
    fn test_checkbox_on_toggle() {
        let callback_id = CallbackId::new(99);
        let element = GuiElement::checkbox("Toggle me")
            .on_toggle(callback_id)
            .build();

        if let GuiElementKind::Checkbox(config) = &element.kind {
            assert_eq!(config.on_toggle, Some(callback_id));
        } else {
            panic!("Expected Checkbox");
        }
    }

    #[test]
    fn test_checkbox_bind_field() {
        let element = GuiElement::checkbox("Remember me")
            .bind_field("state.remember")
            .build();

        if let GuiElementKind::Checkbox(config) = &element.kind {
            assert_eq!(config.field_path, Some("state.remember".to_string()));
        } else {
            panic!("Expected Checkbox");
        }
    }

    #[test]
    fn test_checkbox_full_config() {
        let callback_id = CallbackId::new(123);
        let element = GuiElement::checkbox_with_state("Full checkbox", true)
            .bind_field("state.agreed")
            .on_toggle(callback_id)
            .padding(8.0)
            .build();

        if let GuiElementKind::Checkbox(config) = &element.kind {
            assert_eq!(config.label, "Full checkbox");
            assert!(config.checked);
            assert_eq!(config.field_path, Some("state.agreed".to_string()));
            assert_eq!(config.on_toggle, Some(callback_id));
        } else {
            panic!("Expected Checkbox");
        }
        assert_eq!(element.style.padding, Some(8.0));
    }

    #[test]
    fn test_radio_button_builder() {
        let element = GuiElement::radio_button("Option A", "a").build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.label, "Option A");
            assert_eq!(config.value, "a");
            assert!(config.selected_value.is_none());
            assert!(config.field_path.is_none());
            assert!(config.on_select.is_none());
        } else {
            panic!("Expected RadioButton");
        }
    }

    #[test]
    fn test_radio_button_with_selection() {
        let element = GuiElement::radio_button_with_selection("Option B", "b", Some("b".to_string())).build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.label, "Option B");
            assert_eq!(config.value, "b");
            assert_eq!(config.selected_value, Some("b".to_string()));
        } else {
            panic!("Expected RadioButton");
        }
    }

    #[test]
    fn test_radio_button_radio_value() {
        let element = GuiElement::radio_button("Label", "old")
            .radio_value("new")
            .build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.value, "new");
        } else {
            panic!("Expected RadioButton");
        }
    }

    #[test]
    fn test_radio_button_selected_value() {
        let element = GuiElement::radio_button("Label", "a")
            .selected_value("b")
            .build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.selected_value, Some("b".to_string()));
        } else {
            panic!("Expected RadioButton");
        }
    }

    #[test]
    fn test_radio_button_on_select() {
        let callback_id = CallbackId::new(77);
        let element = GuiElement::radio_button("Option", "opt")
            .on_select(callback_id)
            .build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.on_select, Some(callback_id));
        } else {
            panic!("Expected RadioButton");
        }
    }

    #[test]
    fn test_radio_button_bind_field() {
        let element = GuiElement::radio_button("Size", "small")
            .bind_field("state.size")
            .build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.field_path, Some("state.size".to_string()));
        } else {
            panic!("Expected RadioButton");
        }
    }

    #[test]
    fn test_radio_button_full_config() {
        let callback_id = CallbackId::new(200);
        let element = GuiElement::radio_button_with_selection("Large", "large", Some("small".to_string()))
            .bind_field("state.size")
            .on_select(callback_id)
            .padding(8.0)
            .build();

        if let GuiElementKind::RadioButton(config) = &element.kind {
            assert_eq!(config.label, "Large");
            assert_eq!(config.value, "large");
            assert_eq!(config.selected_value, Some("small".to_string()));
            assert_eq!(config.field_path, Some("state.size".to_string()));
            assert_eq!(config.on_select, Some(callback_id));
        } else {
            panic!("Expected RadioButton");
        }
        assert_eq!(element.style.padding, Some(8.0));
    }

    // Dropdown tests

    #[test]
    fn test_dropdown_builder() {
        let options = vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()];
        let element = GuiElement::dropdown(options.clone()).build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.options, options);
            assert!(config.selected.is_none());
            assert!(config.placeholder.is_none());
            assert!(config.field_path.is_none());
            assert!(config.on_select.is_none());
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_with_selection() {
        let options = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let element = GuiElement::dropdown_with_selection(options.clone(), Some("B".to_string())).build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.options, options);
            assert_eq!(config.selected, Some("B".to_string()));
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_selected() {
        let element = GuiElement::dropdown(vec!["X".to_string()])
            .selected("X")
            .build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.selected, Some("X".to_string()));
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_placeholder() {
        let element = GuiElement::dropdown(vec!["A".to_string()])
            .dropdown_placeholder("Choose one...")
            .build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.placeholder, Some("Choose one...".to_string()));
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_on_select() {
        let callback_id = CallbackId::new(88);
        let element = GuiElement::dropdown(vec!["A".to_string()])
            .on_select(callback_id)
            .build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.on_select, Some(callback_id));
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_bind_field() {
        let element = GuiElement::dropdown(vec!["A".to_string()])
            .bind_field("state.color")
            .build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.field_path, Some("state.color".to_string()));
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_options() {
        let element = GuiElement::dropdown(vec!["Old".to_string()])
            .options(vec!["New1".to_string(), "New2".to_string()])
            .build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.options, vec!["New1".to_string(), "New2".to_string()]);
        } else {
            panic!("Expected Dropdown");
        }
    }

    #[test]
    fn test_dropdown_full_config() {
        let callback_id = CallbackId::new(300);
        let options = vec!["Small".to_string(), "Medium".to_string(), "Large".to_string()];
        let element = GuiElement::dropdown_with_selection(options.clone(), Some("Medium".to_string()))
            .dropdown_placeholder("Select size")
            .bind_field("state.size")
            .on_select(callback_id)
            .padding(12.0)
            .width(Size::Fixed(200.0))
            .build();

        if let GuiElementKind::Dropdown(config) = &element.kind {
            assert_eq!(config.options, options);
            assert_eq!(config.selected, Some("Medium".to_string()));
            assert_eq!(config.placeholder, Some("Select size".to_string()));
            assert_eq!(config.field_path, Some("state.size".to_string()));
            assert_eq!(config.on_select, Some(callback_id));
        } else {
            panic!("Expected Dropdown");
        }
        assert_eq!(element.style.padding, Some(12.0));
        assert_eq!(element.style.width, Some(Size::Fixed(200.0)));
    }

    // ========== Chart Tests ==========

    #[test]
    fn test_bar_chart_builder() {
        let data = vec![
            DataPoint::new("A", 10.0),
            DataPoint::new("B", 20.0),
            DataPoint::new("C", 15.0),
        ];

        let element = GuiElement::bar_chart_with_data(data)
            .chart_title("Sales")
            .chart_size(500.0, 400.0)
            .show_grid(true)
            .show_values(true)
            .bar_color(66, 133, 244)
            .build();

        if let GuiElementKind::BarChart(config) = &element.kind {
            assert_eq!(config.data.len(), 3);
            assert_eq!(config.title, Some("Sales".to_string()));
            assert!((config.width - 500.0).abs() < f32::EPSILON);
            assert!((config.height - 400.0).abs() < f32::EPSILON);
            assert!(config.show_grid);
            assert!(config.show_values);
            assert_eq!(config.bar_color, Some((66, 133, 244)));
        } else {
            panic!("Expected BarChart");
        }
    }

    #[test]
    fn test_bar_chart_empty() {
        let element = GuiElement::bar_chart().build();

        if let GuiElementKind::BarChart(config) = &element.kind {
            assert!(config.data.is_empty());
            assert!(config.title.is_none());
            assert!(config.show_grid);
        } else {
            panic!("Expected BarChart");
        }
    }

    #[test]
    fn test_line_chart_builder() {
        let labels = vec!["Jan".to_string(), "Feb".to_string(), "Mar".to_string()];
        let series = vec![
            DataSeries::new("Revenue", vec![100.0, 150.0, 120.0]),
            DataSeries::new("Expenses", vec![80.0, 90.0, 85.0]),
        ];

        let element = GuiElement::line_chart_with_data(labels.clone(), series)
            .chart_title("Monthly Report")
            .show_legend(true)
            .show_points(true)
            .fill_area(false)
            .build();

        if let GuiElementKind::LineChart(config) = &element.kind {
            assert_eq!(config.labels, labels);
            assert_eq!(config.series.len(), 2);
            assert_eq!(config.title, Some("Monthly Report".to_string()));
            assert!(config.show_legend);
            assert!(config.show_points);
            assert!(!config.fill_area);
        } else {
            panic!("Expected LineChart");
        }
    }

    #[test]
    fn test_line_chart_add_series() {
        let element = GuiElement::line_chart()
            .line_labels(vec!["Q1".to_string(), "Q2".to_string()])
            .add_series(DataSeries::new("Sales", vec![100.0, 200.0]))
            .add_series(DataSeries::new("Returns", vec![10.0, 15.0]))
            .build();

        if let GuiElementKind::LineChart(config) = &element.kind {
            assert_eq!(config.labels.len(), 2);
            assert_eq!(config.series.len(), 2);
            assert_eq!(config.series[0].name, "Sales");
            assert_eq!(config.series[1].name, "Returns");
        } else {
            panic!("Expected LineChart");
        }
    }

    #[test]
    fn test_pie_chart_builder() {
        let data = vec![
            DataPoint::new("Product A", 45.0),
            DataPoint::new("Product B", 30.0),
            DataPoint::new("Product C", 25.0),
        ];

        let element = GuiElement::pie_chart_with_data(data)
            .chart_title("Market Share")
            .show_percentages(true)
            .show_legend(true)
            .inner_radius(0.0)
            .build();

        if let GuiElementKind::PieChart(config) = &element.kind {
            assert_eq!(config.data.len(), 3);
            assert_eq!(config.title, Some("Market Share".to_string()));
            assert!(config.show_percentages);
            assert!(config.show_legend);
            assert!((config.inner_radius_ratio - 0.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected PieChart");
        }
    }

    #[test]
    fn test_donut_chart() {
        let data = vec![
            DataPoint::new("Yes", 60.0),
            DataPoint::new("No", 40.0),
        ];

        let element = GuiElement::pie_chart_with_data(data)
            .inner_radius(0.5)
            .build();

        if let GuiElementKind::PieChart(config) = &element.kind {
            assert!((config.inner_radius_ratio - 0.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected PieChart");
        }
    }

    #[test]
    fn test_chart_styling() {
        let element = GuiElement::bar_chart()
            .padding(16.0)
            .width(Size::Fixed(600.0))
            .height(Size::Fixed(400.0))
            .build();

        assert_eq!(element.style.padding, Some(16.0));
        assert_eq!(element.style.width, Some(Size::Fixed(600.0)));
        assert_eq!(element.style.height, Some(Size::Fixed(400.0)));
    }

    // OLAP Cube Widget Tests (5.7.7-5.7.11)

    #[test]
    fn test_cube_table_builder() {
        let element = GuiElement::cube_table()
            .row_dimensions(vec!["region".to_string(), "product".to_string()])
            .measures(vec!["revenue".to_string(), "quantity".to_string()])
            .page_size(Some(25))
            .show_drill_controls(true)
            .build();

        if let GuiElementKind::CubeTable(config) = &element.kind {
            assert!(config.cube.is_none()); // No cube set yet
            assert_eq!(config.row_dimensions, vec!["region", "product"]);
            assert_eq!(config.measures, vec!["revenue", "quantity"]);
            assert_eq!(config.page_size, Some(25));
            assert!(config.show_drill_controls);
        } else {
            panic!("Expected CubeTable");
        }
    }

    #[test]
    fn test_cube_table_callbacks() {
        let drill_cb = CallbackId::new(1);
        let rollup_cb = CallbackId::new(2);
        let element = GuiElement::cube_table()
            .on_drill(drill_cb)
            .on_roll_up(rollup_cb)
            .build();

        if let GuiElementKind::CubeTable(config) = &element.kind {
            assert_eq!(config.on_drill, Some(drill_cb));
            assert_eq!(config.on_roll_up, Some(rollup_cb));
        } else {
            panic!("Expected CubeTable");
        }
    }

    #[test]
    fn test_cube_chart_builder() {
        let element = GuiElement::cube_chart()
            .cube_chart_type(CubeChartType::Bar)
            .x_dimension("month")
            .y_measure("sales")
            .series_dimension("region")
            .build();

        if let GuiElementKind::CubeChart(config) = &element.kind {
            assert_eq!(config.chart_type, CubeChartType::Bar);
            assert_eq!(config.x_dimension, Some("month".to_string()));
            assert_eq!(config.y_measure, Some("sales".to_string()));
            assert_eq!(config.series_dimension, Some("region".to_string()));
        } else {
            panic!("Expected CubeChart");
        }
    }

    #[test]
    fn test_cube_chart_types() {
        // Test Line chart type
        let line = GuiElement::cube_chart()
            .cube_chart_type(CubeChartType::Line)
            .build();

        if let GuiElementKind::CubeChart(config) = &line.kind {
            assert_eq!(config.chart_type, CubeChartType::Line);
        } else {
            panic!("Expected CubeChart");
        }

        // Test Pie chart type
        let pie = GuiElement::cube_chart()
            .cube_chart_type(CubeChartType::Pie)
            .build();

        if let GuiElementKind::CubeChart(config) = &pie.kind {
            assert_eq!(config.chart_type, CubeChartType::Pie);
        } else {
            panic!("Expected CubeChart");
        }
    }

    #[test]
    fn test_dimension_filter_builder() {
        let element = GuiElement::dimension_filter("product_category")
            .show_all_option(true)
            .cube_label("Filter by Category")
            .build();

        if let GuiElementKind::DimensionFilter(config) = &element.kind {
            assert_eq!(config.dimension, "product_category");
            assert!(config.show_all_option);
            assert_eq!(config.label, Some("Filter by Category".to_string()));
        } else {
            panic!("Expected DimensionFilter");
        }
    }

    #[test]
    fn test_dimension_filter_callback() {
        let select_cb = CallbackId::new(10);
        let element = GuiElement::dimension_filter("region")
            .on_select(select_cb)
            .build();

        if let GuiElementKind::DimensionFilter(config) = &element.kind {
            assert_eq!(config.on_select, Some(select_cb));
        } else {
            panic!("Expected DimensionFilter");
        }
    }

    #[test]
    fn test_hierarchy_navigator_builder() {
        let element = GuiElement::hierarchy_navigator("time")
            .current_level("Year")
            .cube_label("Time Hierarchy")
            .build();

        if let GuiElementKind::HierarchyNavigator(config) = &element.kind {
            assert_eq!(config.hierarchy, "time");
            assert_eq!(config.current_level, Some("Year".to_string()));
            assert_eq!(config.label, Some("Time Hierarchy".to_string()));
        } else {
            panic!("Expected HierarchyNavigator");
        }
    }

    #[test]
    fn test_hierarchy_navigator_callbacks() {
        let drill_cb = CallbackId::new(20);
        let rollup_cb = CallbackId::new(21);
        let level_cb = CallbackId::new(22);

        let element = GuiElement::hierarchy_navigator("geography")
            .on_drill(drill_cb)
            .on_roll_up(rollup_cb)
            .on_level_change(level_cb)
            .build();

        if let GuiElementKind::HierarchyNavigator(config) = &element.kind {
            assert_eq!(config.on_drill_down, Some(drill_cb));
            assert_eq!(config.on_roll_up, Some(rollup_cb));
            assert_eq!(config.on_level_change, Some(level_cb));
        } else {
            panic!("Expected HierarchyNavigator");
        }
    }

    #[test]
    fn test_measure_selector_builder() {
        let element = GuiElement::measure_selector()
            .measures(vec!["revenue".to_string(), "quantity".to_string()])
            .cube_label("Select Measures")
            .build();

        if let GuiElementKind::MeasureSelector(config) = &element.kind {
            assert_eq!(config.selected_measures, vec!["revenue", "quantity"]);
            assert_eq!(config.label, Some("Select Measures".to_string()));
        } else {
            panic!("Expected MeasureSelector");
        }
    }

    #[test]
    fn test_olap_widget_styling() {
        // Test that OLAP widgets can use common styling
        let element = GuiElement::cube_table()
            .padding(16.0)
            .width(Size::Fill)
            .height(Size::Fixed(500.0))
            .build();

        assert_eq!(element.style.padding, Some(16.0));
        assert_eq!(element.style.width, Some(Size::Fill));
        assert_eq!(element.style.height, Some(Size::Fixed(500.0)));
    }

    // ========================================================================
    // Interactive Element Tests
    // ========================================================================

    #[test]
    fn test_interactive_builder() {
        let element = GuiElement::interactive()
            .child(GuiElement::text("Click me").build())
            .build();

        assert!(matches!(element.kind, GuiElementKind::Interactive(_)));
        assert_eq!(element.children.len(), 1);
    }

    #[test]
    fn test_interactive_with_callbacks() {
        use crate::callback::CallbackId;

        let element = GuiElement::interactive()
            .on_press(CallbackId::new(1))
            .on_mouse_release(CallbackId::new(2))
            .on_hover_enter(CallbackId::new(3))
            .on_hover_exit(CallbackId::new(4))
            .child(GuiElement::text("Interactive").build())
            .build();

        if let GuiElementKind::Interactive(ref config) = element.kind {
            assert_eq!(config.on_press, Some(CallbackId::new(1)));
            assert_eq!(config.on_release, Some(CallbackId::new(2)));
            assert_eq!(config.on_enter, Some(CallbackId::new(3)));
            assert_eq!(config.on_exit, Some(CallbackId::new(4)));
        } else {
            panic!("Expected Interactive element");
        }
    }

    #[test]
    fn test_interactive_double_click() {
        use crate::callback::CallbackId;

        let element = GuiElement::interactive()
            .on_double_click(CallbackId::new(10))
            .build();

        if let GuiElementKind::Interactive(ref config) = element.kind {
            assert_eq!(config.on_double_click, Some(CallbackId::new(10)));
        } else {
            panic!("Expected Interactive element");
        }
    }

    #[test]
    fn test_interactive_right_click() {
        use crate::callback::CallbackId;

        let element = GuiElement::interactive()
            .on_right_press(CallbackId::new(20))
            .on_right_release(CallbackId::new(21))
            .build();

        if let GuiElementKind::Interactive(ref config) = element.kind {
            assert_eq!(config.on_right_press, Some(CallbackId::new(20)));
            assert_eq!(config.on_right_release, Some(CallbackId::new(21)));
        } else {
            panic!("Expected Interactive element");
        }
    }

    #[test]
    fn test_interactive_mouse_move_scroll() {
        use crate::callback::CallbackId;

        let element = GuiElement::interactive()
            .on_mouse_move(CallbackId::new(30))
            .on_mouse_scroll(CallbackId::new(31))
            .build();

        if let GuiElementKind::Interactive(ref config) = element.kind {
            assert_eq!(config.on_move, Some(CallbackId::new(30)));
            assert_eq!(config.on_scroll, Some(CallbackId::new(31)));
        } else {
            panic!("Expected Interactive element");
        }
    }

    #[test]
    fn test_interactive_cursor_style() {
        let element = GuiElement::interactive()
            .cursor(CursorStyle::Pointer)
            .build();

        if let GuiElementKind::Interactive(ref config) = element.kind {
            assert_eq!(config.cursor_style, Some(CursorStyle::Pointer));
        } else {
            panic!("Expected Interactive element");
        }
    }

    #[test]
    fn test_cursor_style_variants() {
        // Test all cursor style variants exist and are distinct
        let styles = [
            CursorStyle::Default,
            CursorStyle::Pointer,
            CursorStyle::Text,
            CursorStyle::Grab,
            CursorStyle::Grabbing,
            CursorStyle::Move,
            CursorStyle::NotAllowed,
            CursorStyle::ResizeHorizontal,
            CursorStyle::ResizeVertical,
            CursorStyle::Crosshair,
        ];

        for (i, style) in styles.iter().enumerate() {
            for (j, other) in styles.iter().enumerate() {
                if i == j {
                    assert_eq!(style, other);
                } else {
                    assert_ne!(style, other);
                }
            }
        }
    }

    #[test]
    fn test_interactive_full_config() {
        use crate::callback::CallbackId;

        let element = GuiElement::interactive()
            .on_press(CallbackId::new(1))
            .on_mouse_release(CallbackId::new(2))
            .on_double_click(CallbackId::new(3))
            .on_right_press(CallbackId::new(4))
            .on_right_release(CallbackId::new(5))
            .on_middle_press(CallbackId::new(6))
            .on_middle_release(CallbackId::new(7))
            .on_hover_enter(CallbackId::new(8))
            .on_hover_exit(CallbackId::new(9))
            .on_mouse_move(CallbackId::new(10))
            .on_mouse_scroll(CallbackId::new(11))
            .cursor(CursorStyle::Pointer)
            .child(GuiElement::text("Full config").build())
            .build();

        if let GuiElementKind::Interactive(ref config) = element.kind {
            assert!(config.on_press.is_some());
            assert!(config.on_release.is_some());
            assert!(config.on_double_click.is_some());
            assert!(config.on_right_press.is_some());
            assert!(config.on_right_release.is_some());
            assert!(config.on_middle_press.is_some());
            assert!(config.on_middle_release.is_some());
            assert!(config.on_enter.is_some());
            assert!(config.on_exit.is_some());
            assert!(config.on_move.is_some());
            assert!(config.on_scroll.is_some());
            assert_eq!(config.cursor_style, Some(CursorStyle::Pointer));
        } else {
            panic!("Expected Interactive element");
        }
    }

    #[test]
    fn test_interactive_with_styling() {
        let element = GuiElement::interactive()
            .padding(10.0)
            .width(Size::Fixed(200.0))
            .height(Size::Fixed(100.0))
            .build();

        assert!(matches!(element.kind, GuiElementKind::Interactive(_)));
        assert_eq!(element.style.padding, Some(10.0));
        assert_eq!(element.style.width, Some(Size::Fixed(200.0)));
        assert_eq!(element.style.height, Some(Size::Fixed(100.0)));
    }
}
