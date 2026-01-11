//! OLAP Cube Dashboard example demonstrating all OLAP widgets
//!
//! Run with: cargo run --package stratum-gui --example cube_dashboard

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use stratum_core::bytecode::{StructInstance, Value};
use stratum_core::data::{Cube, CubeAggFunc, DataFrame, Series};
use stratum_gui::element::{CubeChartType, CubeFilterContext, GuiElement};
use stratum_gui::GuiRuntime;

fn main() {
    // Create sample sales data
    let regions = Series::from_strings(
        "region",
        vec![
            "North", "North", "North", "North", "South", "South", "South", "South", "East", "East",
            "East", "East", "West", "West", "West", "West",
        ],
    );

    let products = Series::from_strings(
        "product",
        vec![
            "Widgets", "Gadgets", "Widgets", "Gadgets", "Widgets", "Gadgets", "Widgets", "Gadgets",
            "Widgets", "Gadgets", "Widgets", "Gadgets", "Widgets", "Gadgets", "Widgets", "Gadgets",
        ],
    );

    let quarters = Series::from_strings(
        "quarter",
        vec![
            "Q1", "Q1", "Q2", "Q2", "Q1", "Q1", "Q2", "Q2", "Q1", "Q1", "Q2", "Q2", "Q1", "Q1",
            "Q2", "Q2",
        ],
    );

    let revenue = Series::from_floats(
        "revenue",
        vec![
            1200.0, 800.0, 1500.0, 950.0, 900.0, 650.0, 1100.0, 780.0, 1400.0, 920.0, 1650.0,
            1050.0, 1000.0, 700.0, 1250.0, 850.0,
        ],
    );

    let units = Series::from_ints(
        "units",
        vec![
            120, 80, 150, 95, 90, 65, 110, 78, 140, 92, 165, 105, 100, 70, 125, 85,
        ],
    );

    let df = DataFrame::from_series(vec![regions, products, quarters, revenue, units])
        .expect("Failed to create DataFrame");

    println!("Created DataFrame with {} rows", df.num_rows());

    // Build the OLAP Cube
    let cube = Cube::from_dataframe(&df)
        .expect("Failed to start cube builder")
        .dimension("region")
        .expect("Failed to add region dimension")
        .dimension("product")
        .expect("Failed to add product dimension")
        .dimension("quarter")
        .expect("Failed to add quarter dimension")
        .measure("revenue", CubeAggFunc::Sum)
        .expect("Failed to add revenue measure")
        .measure("units", CubeAggFunc::Sum)
        .expect("Failed to add units measure")
        .build()
        .expect("Failed to build cube");

    println!(
        "Built Cube: {} dimensions, {} measures, {} rows",
        cube.dimension_names().len(),
        cube.measure_names().len(),
        cube.row_count()
    );

    let cube_arc = Arc::new(cube);

    // Create a shared filter context for cross-widget filtering
    let filter_context = CubeFilterContext::new();

    // Create the GUI elements

    // Title
    let title = GuiElement::text("Sales Dashboard")
        .text_size(24.0)
        .bold()
        .build();

    // Sidebar header
    let filters_label = GuiElement::text("Filters").bold().build();

    // Dimension Filter for region - connected to filter context
    let region_filter = GuiElement::dimension_filter_with_cube(cube_arc.clone(), "region")
        .cube_label("Region")
        .show_all_option(true)
        .filter_context(filter_context.clone())
        .build();

    // Dimension Filter for product - connected to filter context
    let product_filter = GuiElement::dimension_filter_with_cube(cube_arc.clone(), "product")
        .cube_label("Product")
        .show_all_option(true)
        .filter_context(filter_context.clone())
        .build();

    // Measure Selector
    let measures_label = GuiElement::text("Measures").bold().build();

    // Measure Selector - connected to filter context
    let measure_selector = GuiElement::measure_selector_with_cube(cube_arc.clone())
        .cube_label("Visible")
        .filter_context(filter_context.clone())
        .build();

    // CubeChart - Bar chart showing revenue by region - connected to filter context
    let bar_chart = GuiElement::cube_chart_with_cube(cube_arc.clone())
        .cube_chart_type(CubeChartType::Bar)
        .x_dimension("region")
        .y_measure("revenue")
        .chart_title("Revenue by Region")
        .chart_size(450.0, 280.0)
        .show_grid(true)
        .filter_context(filter_context.clone())
        .build();

    // CubeChart - Pie chart showing units by product - connected to filter context
    let pie_chart = GuiElement::cube_chart_with_cube(cube_arc.clone())
        .cube_chart_type(CubeChartType::Pie)
        .x_dimension("product")
        .y_measure("units")
        .chart_title("Units by Product")
        .chart_size(320.0, 280.0)
        .filter_context(filter_context.clone())
        .build();

    // CubeChart - Line chart showing revenue by quarter with product series - connected to filter context
    let line_chart = GuiElement::cube_chart_with_cube(cube_arc.clone())
        .cube_chart_type(CubeChartType::Line)
        .x_dimension("quarter")
        .y_measure("revenue")
        .series_dimension("product")
        .chart_title("Revenue Trend by Product")
        .chart_size(780.0, 280.0)
        .show_grid(true)
        .filter_context(filter_context.clone())
        .build();

    // Table header
    let table_label = GuiElement::text("Data Table - Revenue & Units by Region and Product")
        .bold()
        .build();

    // CubeTable showing all data - connected to filter context
    let cube_table = GuiElement::cube_table_with_cube(cube_arc.clone())
        .row_dimensions(vec!["region".to_string(), "product".to_string()])
        .measures(vec!["revenue".to_string(), "units".to_string()])
        .page_size(Some(10))
        .show_drill_controls(true)
        .filter_context(filter_context.clone())
        .build();

    // Layout: Sidebar
    let sidebar = GuiElement::vstack()
        .spacing(12.0)
        .child(filters_label)
        .child(region_filter)
        .child(product_filter)
        .child(GuiElement::spacer().build())
        .child(measures_label)
        .child(measure_selector)
        .build();

    // Layout: Charts row
    let charts_row = GuiElement::hstack()
        .spacing(20.0)
        .child(bar_chart)
        .child(pie_chart)
        .build();

    // Layout: Main content
    let main_content = GuiElement::vstack()
        .spacing(16.0)
        .child(title)
        .child(charts_row)
        .child(line_chart)
        .child(table_label)
        .child(cube_table)
        .build();

    // Root layout
    let root = GuiElement::hstack()
        .spacing(24.0)
        .child(sidebar)
        .child(main_content)
        .build();

    // Create state
    let mut fields = HashMap::new();
    fields.insert("cube".to_string(), Value::Cube(cube_arc));
    fields.insert("selected_region".to_string(), Value::Null);

    let mut instance = StructInstance::new("DashboardState".to_string());
    instance.fields = fields;
    let state = Value::Struct(Rc::new(RefCell::new(instance)));

    // Create and run the GUI
    println!("Launching OLAP Dashboard...");
    println!("- Bar chart: Revenue by Region");
    println!("- Pie chart: Units by Product");
    println!("- Line chart: Revenue Trend by Product (Q1 vs Q2)");
    println!("- Data table: Aggregated view with drill-down controls");

    let runtime = GuiRuntime::new(state)
        .with_window("OLAP Cube Dashboard", (1100, 750))
        .with_root(root)
        .with_spacing(16.0);

    if let Err(e) = runtime.run() {
        eprintln!("GUI error: {e}");
        std::process::exit(1);
    }
}
