# Gui

Declarative GUI framework for building interactive desktop applications with layouts, widgets, charts, and OLAP visualizations.

## Overview

The `Gui` namespace provides Stratum's declarative GUI capabilities for creating desktop applications. It features a reactive state model where the UI automatically updates when state changes, making it easy to build responsive interfaces.

Key features include:
- **Reactive state management** with automatic UI updates
- **Flexible layouts** (VStack, HStack, Grid, ZStack)
- **Rich widgets** (buttons, text fields, checkboxes, sliders, dropdowns)
- **Data visualization** (DataTable, bar charts, line charts, pie charts)
- **OLAP integration** (CubeTable, CubeChart with drill-down)
- **Theming system** with 20+ built-in themes

GUI elements are immutable—configuration methods return new elements rather than modifying in place.

---

## Application Lifecycle

### `Gui.app(title, initial_state, view_fn, width?, height?)`

Creates and runs a reactive GUI application with state management.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `title` | `String` | Window title |
| `initial_state` | `Any` | Initial application state (typically a struct) |
| `view_fn` | `Closure` | Function that takes state and returns a GuiElement |
| `width` | `Int?` | Window width in pixels (default: 800) |
| `height` | `Int?` | Window height in pixels (default: 600) |

**Returns:** `Null` - Blocks until window is closed

**Example:**

```stratum
struct CounterState {
    count: Int
}

fx build_ui(state: CounterState) {
    let text = Gui.text("Count: " + state.count.to_string())
    Gui.vstack(10.0, [text])
}

fx main() {
    let state = CounterState { count: 0 }
    Gui.app("My Counter", state, build_ui, 400, 300)
}
```

---

### `Gui.run(element, title?, width?, height?)`

Runs a standalone GUI element without reactive state management.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | The root element to render |
| `title` | `String?` | Window title (default: "Stratum App") |
| `width` | `Int?` | Window width in pixels (default: 800) |
| `height` | `Int?` | Window height in pixels (default: 600) |

**Returns:** `Null` - Blocks until window is closed

**Example:**

```stratum
let greeting = Gui.text("Hello, Stratum!")
Gui.run(greeting, "Simple App", 300, 200)
```

---

### `Gui.quit()`

Requests application shutdown from within a callback.

**Returns:** `Null`

**Example:**

```stratum
let quit_id = Gui.register_callback(|s| {
    println("Goodbye!")
    Gui.quit()
})
let quit_btn = Gui.button("Quit", quit_id)
```

---

### `Gui.register_callback(closure)`

Registers a closure for later invocation by UI events.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `closure` | `Closure` | The callback function to register |

**Returns:** `Int` - Callback ID to pass to event handlers

**Example:**

```stratum
let click_id = Gui.register_callback(|state| {
    println("Button clicked!")
})
let btn = Gui.button("Click Me", click_id)
```

---

## Layout Functions

### `Gui.vstack(spacing?, children?)`

Creates a vertical stack layout that arranges children top-to-bottom.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `spacing` | `Float?` | Space between children in pixels (default: 0) |
| `children` | `List<GuiElement>?` | Child elements |

**Returns:** `GuiElement` - A VStack element

**Example:**

```stratum
// Empty VStack, add children later
let stack = Gui.vstack()

// VStack with spacing
let stack = Gui.vstack(16.0)

// VStack with spacing and children
let stack = Gui.vstack(16.0, [
    Gui.text("Line 1"),
    Gui.text("Line 2"),
    Gui.text("Line 3")
])
```

---

### `Gui.hstack(spacing?, children?)`

Creates a horizontal stack layout that arranges children left-to-right.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `spacing` | `Float?` | Space between children in pixels (default: 0) |
| `children` | `List<GuiElement>?` | Child elements |

**Returns:** `GuiElement` - An HStack element

**Example:**

```stratum
let row = Gui.hstack(8.0, [
    Gui.button("-", dec_id),
    Gui.text("Count: 0"),
    Gui.button("+", inc_id)
])
```

---

### `Gui.zstack(children?)`

Creates a z-axis stack that overlays children on top of each other.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `children` | `List<GuiElement>?` | Child elements (first is bottom, last is top) |

**Returns:** `GuiElement` - A ZStack element

**Example:**

```stratum
let overlay = Gui.zstack([
    Gui.image("background.png"),
    Gui.text("Overlay Text")
])
```

---

### `Gui.grid(columns, spacing?, children?)`

Creates a grid layout with a fixed number of columns.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `Int` | Number of columns |
| `spacing` | `Float?` | Space between cells in pixels |
| `children` | `List<GuiElement>?` | Child elements |

**Returns:** `GuiElement` - A Grid element

**Example:**

```stratum
let grid = Gui.grid(3, 8.0, [
    Gui.text("A"), Gui.text("B"), Gui.text("C"),
    Gui.text("D"), Gui.text("E"), Gui.text("F")
])
```

---

### `Gui.scroll_view(direction?, children?)`

Creates a scrollable container.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `direction` | `String?` | Scroll direction: "vertical", "horizontal", or "both" (default: "vertical") |
| `children` | `List<GuiElement>?` | Child elements |

**Returns:** `GuiElement` - A ScrollView element

**Example:**

```stratum
let scrollable = Gui.scroll_view("vertical", [
    // Many child elements...
])
```

---

### `Gui.container(children?)`

Creates a generic container for grouping and styling elements.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `children` | `List<GuiElement>?` | Child elements |

**Returns:** `GuiElement` - A Container element

**Example:**

```stratum
let box = Gui.container([Gui.text("Content")])
let styled = Gui.set_background(box, 240, 240, 240, 255)
```

---

### `Gui.spacer()`

Creates a flexible spacer that fills available space.

**Returns:** `GuiElement` - A Spacer element

**Example:**

```stratum
let header = Gui.hstack(0.0, [
    Gui.text("Title"),
    Gui.spacer(),
    Gui.button("Close", close_id)
])
```

---

## Layout Configuration

### `Gui.add_child(element, child)`

Adds a child element to a layout container.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | The parent layout element |
| `child` | `GuiElement` | The child element to add |

**Returns:** `GuiElement` - Updated element with the child added

**Example:**

```stratum
let layout = Gui.vstack()
let layout = Gui.add_child(layout, Gui.text("First"))
let layout = Gui.add_child(layout, Gui.text("Second"))
```

---

### `Gui.set_spacing(element, value)`

Sets the spacing between children in a layout.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A layout element (VStack, HStack, Grid) |
| `value` | `Float` | Spacing in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_padding(element, value)`

Sets the padding around an element's content.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `value` | `Float` | Padding in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_width(element, value)`

Sets the width of an element.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `value` | `Float` | Width in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_height(element, value)`

Sets the height of an element.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `value` | `Float` | Height in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_alignment(element, horizontal, vertical)`

Sets the content alignment within a layout.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A layout element |
| `horizontal` | `String` | "start", "center", or "end" |
| `vertical` | `String` | "start", "center", or "end" |

**Returns:** `GuiElement` - Updated element

---

## Core Widgets

### `Gui.text(content)`

Creates a text display element.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `content` | `String` | The text to display |

**Returns:** `GuiElement` - A Text element

**Example:**

```stratum
let label = Gui.text("Hello, World!")
let styled = Gui.set_text_size(label, 24.0)
let bold = Gui.set_text_bold(styled)
```

---

### `Gui.button(label, callback_id?)`

Creates a clickable button.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `label` | `String` | Button text |
| `callback_id` | `Int?` | Callback ID to invoke on click |

**Returns:** `GuiElement` - A Button element

**Example:**

```stratum
let click_id = Gui.register_callback(|s| println("Clicked!"))
let btn = Gui.button("Click Me", click_id)
```

---

### `Gui.text_field(initial_value?)`

Creates a text input field.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `initial_value` | `String?` | Initial text value (default: "") |

**Returns:** `GuiElement` - A TextField element

**Example:**

```stratum
let input = Gui.text_field()
let with_placeholder = Gui.set_placeholder(input, "Enter name...")
let password = Gui.set_secure(input, true)
```

---

### `Gui.checkbox(label, checked?)`

Creates a checkbox with a label.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `label` | `String` | Checkbox label |
| `checked` | `Bool?` | Initial checked state (default: false) |

**Returns:** `GuiElement` - A Checkbox element

**Example:**

```stratum
let agree = Gui.checkbox("I agree to the terms", false)
```

---

### `Gui.radio_button(label, value, selected)`

Creates a radio button for single-selection groups.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `label` | `String` | Radio button label |
| `value` | `String` | Value when selected |
| `selected` | `String` | Currently selected value in the group |

**Returns:** `GuiElement` - A RadioButton element

**Example:**

```stratum
let opt1 = Gui.radio_button("Option A", "a", "a")
let opt2 = Gui.radio_button("Option B", "b", "a")
```

---

### `Gui.dropdown(options, selected?)`

Creates a dropdown selection menu.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `options` | `List<String>` | Available options |
| `selected` | `Int?` | Index of selected option (default: 0) |

**Returns:** `GuiElement` - A Dropdown element

**Example:**

```stratum
let colors = Gui.dropdown(["Red", "Green", "Blue"], 0)
let with_placeholder = Gui.set_dropdown_placeholder(colors, "Select color...")
```

---

### `Gui.slider(value?, min?, max?)`

Creates a slider for numeric input.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Float?` | Initial value (default: 0) |
| `min` | `Float?` | Minimum value (default: 0) |
| `max` | `Float?` | Maximum value (default: 100) |

**Returns:** `GuiElement` - A Slider element

**Example:**

```stratum
let volume = Gui.slider(50.0, 0.0, 100.0)
let with_step = Gui.set_slider_step(volume, 5.0)
```

---

### `Gui.toggle(label, on?)`

Creates a toggle switch.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `label` | `String` | Toggle label |
| `on` | `Bool?` | Initial state (default: false) |

**Returns:** `GuiElement` - A Toggle element

**Example:**

```stratum
let dark_mode = Gui.toggle("Dark Mode", false)
```

---

### `Gui.progress_bar(value?)`

Creates a progress indicator.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Float?` | Progress value 0.0-1.0 (default: 0) |

**Returns:** `GuiElement` - A ProgressBar element

**Example:**

```stratum
let progress = Gui.progress_bar(0.75)  // 75% complete
```

---

### `Gui.image(path)`

Creates an image display element.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the image file |

**Returns:** `GuiElement` - An Image element

**Example:**

```stratum
let logo = Gui.image("assets/logo.png")
let fitted = Gui.set_content_fit(logo, "contain")
let faded = Gui.set_opacity(logo, 0.8)
```

---

## Text Styling

### `Gui.set_text_bold(element)`

Makes text bold.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A Text element |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_text_size(element, size)`

Sets the font size.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A Text element |
| `size` | `Float` | Font size in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_text_color(element, r, g, b, a?)`

Sets the text color.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A Text element |
| `r` | `Int` | Red component (0-255) |
| `g` | `Int` | Green component (0-255) |
| `b` | `Int` | Blue component (0-255) |
| `a` | `Int?` | Alpha component (0-255, default: 255) |

**Returns:** `GuiElement` - Updated element

---

## Widget Styling

### `Gui.set_disabled(element, disabled)`

Disables or enables a widget.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A Button or input element |
| `disabled` | `Bool` | True to disable |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_placeholder(element, text)`

Sets placeholder text for input fields.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A TextField element |
| `text` | `String` | Placeholder text |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_secure(element, secure)`

Enables password mode (masked input).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A TextField element |
| `secure` | `Bool` | True for password mode |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_background(element, r, g, b, a?)`

Sets the background color.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `r` | `Int` | Red component (0-255) |
| `g` | `Int` | Green component (0-255) |
| `b` | `Int` | Blue component (0-255) |
| `a` | `Int?` | Alpha component (0-255, default: 255) |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_foreground(element, r, g, b, a?)`

Sets the foreground/text color.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `r` | `Int` | Red component (0-255) |
| `g` | `Int` | Green component (0-255) |
| `b` | `Int` | Blue component (0-255) |
| `a` | `Int?` | Alpha component (0-255, default: 255) |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_border_color(element, r, g, b, a?)`

Sets the border color.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `r` | `Int` | Red component (0-255) |
| `g` | `Int` | Green component (0-255) |
| `b` | `Int` | Blue component (0-255) |
| `a` | `Int?` | Alpha component (0-255, default: 255) |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_border_width(element, width)`

Sets the border width.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `width` | `Float` | Border width in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_corner_radius(element, radius)`

Sets the corner radius for rounded corners.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `radius` | `Float` | Corner radius in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_content_fit(element, mode)`

Sets the image scaling mode.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | An Image element |
| `mode` | `String` | "fill", "contain", "cover", or "none" |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_opacity(element, value)`

Sets the element opacity.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `value` | `Float` | Opacity 0.0-1.0 |

**Returns:** `GuiElement` - Updated element

---

## Data Widgets

### `Gui.data_table(dataframe, page_size?)`

Creates a data table from a DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dataframe` | `DataFrame` | The data to display |
| `page_size` | `Int?` | Rows per page (default: 10) |

**Returns:** `GuiElement` - A DataTable element

**Example:**

```stratum
let df = Data.from_columns(
    "name", ["Alice", "Bob", "Carol"],
    "age", [30, 25, 35]
)
let table = Gui.data_table(df, 10)
let sortable = Gui.set_sortable(table, true)
```

---

### `Gui.set_table_columns(element, columns)`

Sets which columns to display.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable element |
| `columns` | `List<String>` | Column names to show |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_page_size(element, size)`

Sets the number of rows per page.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable or CubeTable element |
| `size` | `Int` | Rows per page |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_sortable(element, sortable)`

Enables or disables column sorting.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable element |
| `sortable` | `Bool` | True to enable sorting |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_selectable(element, selectable)`

Enables or disables row selection.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable element |
| `selectable` | `Bool` | True to enable selection |

**Returns:** `GuiElement` - Updated element

---

## Charts

### `Gui.bar_chart()`

Creates an empty bar chart.

**Returns:** `GuiElement` - A BarChart element

**Example:**

```stratum
let chart = Gui.bar_chart()
let with_data = Gui.set_chart_data_arrays(chart,
    ["North", "South", "East", "West"],
    [1500.0, 1200.0, 1800.0, 1400.0]
)
let titled = Gui.set_chart_title(with_data, "Sales by Region")
let sized = Gui.set_chart_size(titled, 400.0, 300.0)
```

---

### `Gui.line_chart(labels, series_name1, values1, ...)`

Creates a line chart with multiple series.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `labels` | `List<String>` | X-axis labels |
| `series_name` | `String` | Name for the first series |
| `values` | `List<Float>` | Values for the first series |
| `...` | `String, List<Float>` | Additional series name/values pairs |

**Returns:** `GuiElement` - A LineChart element

**Example:**

```stratum
let chart = Gui.line_chart(
    ["Q1", "Q2", "Q3", "Q4"],
    "North", [100.0, 150.0, 200.0, 250.0],
    "South", [80.0, 120.0, 160.0, 200.0]
)
let with_legend = Gui.set_show_legend(chart, true)
```

---

### `Gui.pie_chart()`

Creates an empty pie chart.

**Returns:** `GuiElement` - A PieChart element

**Example:**

```stratum
let chart = Gui.pie_chart()
let with_data = Gui.set_chart_data_arrays(chart,
    ["Widgets", "Gadgets", "Gizmos"],
    [45.0, 35.0, 20.0]
)
```

---

### `Gui.set_chart_title(element, title)`

Sets the chart title.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A chart element |
| `title` | `String` | Chart title |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_chart_size(element, width, height)`

Sets the chart dimensions.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A chart element |
| `width` | `Float` | Width in pixels |
| `height` | `Float` | Height in pixels |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_chart_data_arrays(element, labels, values)`

Sets chart data using separate label and value arrays.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A BarChart or PieChart element |
| `labels` | `List<String>` | Data labels |
| `values` | `List<Float>` | Data values |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_show_legend(element, show)`

Shows or hides the chart legend.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A chart element |
| `show` | `Bool` | True to show legend |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_show_grid(element, show)`

Shows or hides the chart grid lines.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A chart element |
| `show` | `Bool` | True to show grid |

**Returns:** `GuiElement` - Updated element

---

## OLAP Widgets

### `Gui.cube_chart(cube, chart_type)`

Creates a chart powered by an OLAP cube.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cube` | `Cube` | The OLAP cube |
| `chart_type` | `String` | "bar", "line", or "pie" |

**Returns:** `GuiElement` - A CubeChart element

**Example:**

```stratum
let chart = Gui.cube_chart(cube, "bar")
let with_x = Gui.set_x_dimension(chart, "region")
let with_y = Gui.set_y_measure(with_x, "revenue")
let titled = Gui.set_chart_title(with_y, "Revenue by Region")
```

---

### `Gui.cube_table(cube)`

Creates a table with OLAP drill-down capabilities.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cube` | `Cube` | The OLAP cube |

**Returns:** `GuiElement` - A CubeTable element

**Example:**

```stratum
let table = Gui.cube_table(cube)
let with_dims = Gui.set_row_dimensions(table, ["region", "product"])
let with_measures = Gui.set_measures(with_dims, ["revenue", "units"])
```

---

### `Gui.dimension_filter(cube, dimension)`

Creates a dropdown filter for a cube dimension.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cube` | `Cube` | The OLAP cube |
| `dimension` | `String` | Dimension name to filter |

**Returns:** `GuiElement` - A DimensionFilter element

**Example:**

```stratum
let region_filter = Gui.dimension_filter(cube, "region")
```

---

### `Gui.measure_selector(cube)`

Creates checkboxes to select visible measures.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cube` | `Cube` | The OLAP cube |

**Returns:** `GuiElement` - A MeasureSelector element

---

### `Gui.hierarchy_navigator(cube, hierarchy)`

Creates a breadcrumb navigator for hierarchy drill-down.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `cube` | `Cube` | The OLAP cube |
| `hierarchy` | `String` | Hierarchy name |

**Returns:** `GuiElement` - A HierarchyNavigator element

---

### `Gui.set_x_dimension(element, dimension)`

Sets the X-axis dimension for a CubeChart.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeChart element |
| `dimension` | `String` | Dimension name |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_y_measure(element, measure)`

Sets the Y-axis measure for a CubeChart.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeChart element |
| `measure` | `String` | Measure name |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_series_dimension(element, dimension)`

Sets the series grouping dimension for multi-series charts.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeChart element |
| `dimension` | `String` | Dimension name for series |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_row_dimensions(element, dimensions)`

Sets the row grouping dimensions for a CubeTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeTable element |
| `dimensions` | `List<String>` | Dimension names |

**Returns:** `GuiElement` - Updated element

---

### `Gui.set_measures(element, measures)`

Sets the visible measures for a CubeTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeTable element |
| `measures` | `List<String>` | Measure names |

**Returns:** `GuiElement` - Updated element

---

## Event Handling

### `Gui.on_press(element, callback_id)`

Attaches a click/press handler to an element.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `callback_id` | `Int` | Callback ID from register_callback |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_double_click(element, callback_id)`

Attaches a double-click handler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_hover_enter(element, callback_id)`

Attaches a mouse enter handler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_hover_exit(element, callback_id)`

Attaches a mouse leave handler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | Any element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_change(element, callback_id)`

Attaches a value change handler (for TextField, Slider, etc.).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A TextField, Slider, or MeasureSelector |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_submit(element, callback_id)`

Attaches a submit handler (triggered on Enter key in TextField).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A TextField element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_row_click(element, callback_id)`

Attaches a row click handler to a DataTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_sort(element, callback_id)`

Attaches a sort handler to a DataTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_page_change(element, callback_id)`

Attaches a pagination handler to a DataTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A DataTable element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_drill(element, callback_id)`

Attaches a drill-down handler to a CubeTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeTable element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

### `Gui.on_roll_up(element, callback_id)`

Attaches a roll-up handler to a CubeTable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `element` | `GuiElement` | A CubeTable element |
| `callback_id` | `Int` | Callback ID |

**Returns:** `GuiElement` - Updated element

---

## Theming

### `Gui.theme_presets()`

Returns a list of available theme preset names.

**Returns:** `List<String>` - Theme preset names

**Example:**

```stratum
let themes = Gui.theme_presets()
// ["light", "dark", "nord", "dracula", "solarized_light", ...]
```

---

### `Gui.set_theme(preset_name)`

Sets the application theme to a preset.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `preset_name` | `String` | Name of the theme preset |

**Returns:** `Null`

**Example:**

```stratum
Gui.set_theme("dracula")
```

Available presets include: light, dark, nord, dracula, solarized_light, solarized_dark, monokai, gruvbox_light, gruvbox_dark, one_dark, tokyo_night, catppuccin_latte, catppuccin_mocha, material_light, material_dark, github_light, github_dark, ayu_light, ayu_dark, everforest_light, everforest_dark.

---

### `Gui.custom_theme(name, palette)`

Creates and applies a custom theme.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Theme name |
| `palette` | `Struct` | Palette with color fields |

**Returns:** `Null`

The palette struct should have these fields:
- `background`: Background color as `{r, g, b}`
- `text`: Text color
- `primary`: Primary accent color
- `success`: Success indicator color
- `warning`: Warning indicator color
- `danger`: Error/danger color

**Example:**

```stratum
Gui.custom_theme("corporate", {
    background: {r: 250, g: 250, b: 255},
    text: {r: 30, g: 30, b: 50},
    primary: {r: 0, g: 100, b: 180},
    success: {r: 40, g: 160, b: 80},
    warning: {r: 220, g: 160, b: 40},
    danger: {r: 200, g: 60, b: 60}
})
```

---

## Complete Examples

### Counter Application

```stratum
struct CounterState {
    count: Int
}

fx build_ui(state: CounterState) {
    let dec_id = Gui.register_callback(|s| println("Decrement"))
    let inc_id = Gui.register_callback(|s| println("Increment"))

    let title = Gui.set_text_size(Gui.set_text_bold(Gui.text("Counter")), 24.0)
    let count_display = Gui.set_text_size(Gui.text("Count: " + state.count.to_string()), 32.0)

    let btn_row = Gui.set_spacing(Gui.hstack(0.0, [
        Gui.button("-", dec_id),
        Gui.button("+", inc_id)
    ]), 16.0)

    Gui.set_padding(Gui.set_spacing(Gui.vstack(0.0, [
        title, count_display, btn_row
    ]), 20.0), 40.0)
}

fx main() {
    Gui.app("Counter", CounterState { count: 0 }, build_ui, 400, 300)
}
```

### Data Dashboard

```stratum
fx build_dashboard(state) {
    let df = Data.from_columns(
        "region", ["North", "South", "East", "West"],
        "sales", [1200, 980, 1450, 1100]
    )

    let table = Gui.set_sortable(Gui.data_table(df, 10), true)

    let chart = Gui.set_chart_size(
        Gui.set_chart_title(
            Gui.set_chart_data_arrays(Gui.bar_chart(),
                ["North", "South", "East", "West"],
                [1200.0, 980.0, 1450.0, 1100.0]
            ),
            "Sales by Region"
        ),
        400.0, 300.0
    )

    Gui.set_padding(Gui.set_spacing(Gui.vstack(0.0, [table, chart]), 20.0), 24.0)
}
```

---

## See Also

- [Data (DataFrame)](data.md) - Data manipulation for table sources
- [Cube (OLAP)](cube.md) - OLAP cube creation for CubeTable and CubeChart
