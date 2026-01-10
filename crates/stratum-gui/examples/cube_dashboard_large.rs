//! OLAP Cube Dashboard with large dataset (loaded from parquet)
//!
//! First generate the data:
//!   cargo run --package stratum-gui --example generate_sales_data
//!
//! Then run the dashboard:
//!   cargo run --release --package stratum-gui --example cube_dashboard_large

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use stratum_core::bytecode::{StructInstance, Value};
use stratum_core::data::{read_parquet, Cube, CubeAggFunc};
use stratum_gui::element::{CubeChartType, CubeFilterContext, GuiElement};
use stratum_gui::GuiRuntime;

fn main() {
    let parquet_path = "sales_data_2m.parquet";

    if !Path::new(parquet_path).exists() {
        eprintln!("Error: {} not found!", parquet_path);
        eprintln!("Please generate the data first:");
        eprintln!("  cargo run --package stratum-gui --example generate_sales_data");
        std::process::exit(1);
    }

    println!("Loading data from {}...", parquet_path);
    let start = Instant::now();

    let df = read_parquet(parquet_path).expect("Failed to read parquet file");
    let load_time = start.elapsed();

    println!("Loaded DataFrame with {} rows in {:.2?}", df.num_rows(), load_time);

    // Build the OLAP Cube
    println!("Building OLAP Cube...");
    let cube_start = Instant::now();

    let cube = Cube::from_dataframe(&df)
        .expect("Failed to start cube builder")
        .dimension("region")
        .expect("Failed to add region dimension")
        .dimension("product")
        .expect("Failed to add product dimension")
        .dimension("quarter")
        .expect("Failed to add quarter dimension")
        .dimension("year")
        .expect("Failed to add year dimension")
        .dimension("channel")
        .expect("Failed to add channel dimension")
        .dimension("category")
        .expect("Failed to add category dimension")
        .measure("revenue", CubeAggFunc::Sum)
        .expect("Failed to add revenue measure")
        .measure("units", CubeAggFunc::Sum)
        .expect("Failed to add units measure")
        .measure("cost", CubeAggFunc::Sum)
        .expect("Failed to add cost measure")
        .build()
        .expect("Failed to build cube");

    let cube_time = cube_start.elapsed();

    println!(
        "Built Cube: {} dimensions, {} measures, {} rows in {:.2?}",
        cube.dimension_names().len(),
        cube.measure_names().len(),
        cube.row_count(),
        cube_time
    );

    let cube_arc = Arc::new(cube);

    // Create a shared filter context for cross-widget filtering
    let filter_context = CubeFilterContext::new();

    // Create the GUI elements

    // Title
    let title = GuiElement::text("Sales Dashboard (2M rows)")
        .text_size(24.0)
        .bold()
        .build();

    // Sidebar header
    let filters_label = GuiElement::text("Filters")
        .bold()
        .build();

    // Dimension Filters
    let region_filter = GuiElement::dimension_filter_with_cube(cube_arc.clone(), "region")
        .cube_label("Region")
        .show_all_option(true)
        .filter_context(filter_context.clone())
        .build();

    let product_filter = GuiElement::dimension_filter_with_cube(cube_arc.clone(), "product")
        .cube_label("Product")
        .show_all_option(true)
        .filter_context(filter_context.clone())
        .build();

    let year_filter = GuiElement::dimension_filter_with_cube(cube_arc.clone(), "year")
        .cube_label("Year")
        .show_all_option(true)
        .filter_context(filter_context.clone())
        .build();

    let channel_filter = GuiElement::dimension_filter_with_cube(cube_arc.clone(), "channel")
        .cube_label("Channel")
        .show_all_option(true)
        .filter_context(filter_context.clone())
        .build();

    // Measure Selector
    let measures_label = GuiElement::text("Measures")
        .bold()
        .build();

    let measure_selector = GuiElement::measure_selector_with_cube(cube_arc.clone())
        .cube_label("Visible")
        .filter_context(filter_context.clone())
        .build();

    // CubeChart - Bar chart showing revenue by region
    let bar_chart = GuiElement::cube_chart_with_cube(cube_arc.clone())
        .cube_chart_type(CubeChartType::Bar)
        .x_dimension("region")
        .y_measure("revenue")
        .chart_title("Revenue by Region")
        .chart_size(450.0, 280.0)
        .show_grid(true)
        .filter_context(filter_context.clone())
        .build();

    // CubeChart - Pie chart showing units by product
    let pie_chart = GuiElement::cube_chart_with_cube(cube_arc.clone())
        .cube_chart_type(CubeChartType::Pie)
        .x_dimension("product")
        .y_measure("units")
        .chart_title("Units by Product")
        .chart_size(320.0, 280.0)
        .filter_context(filter_context.clone())
        .build();

    // CubeChart - Line chart showing revenue by quarter with year series
    let line_chart = GuiElement::cube_chart_with_cube(cube_arc.clone())
        .cube_chart_type(CubeChartType::Line)
        .x_dimension("quarter")
        .y_measure("revenue")
        .series_dimension("year")
        .chart_title("Revenue Trend by Year")
        .chart_size(780.0, 280.0)
        .show_grid(true)
        .filter_context(filter_context.clone())
        .build();

    // Table header
    let table_label = GuiElement::text("Data Table - Aggregated by Region and Product")
        .bold()
        .build();

    // CubeTable showing aggregated data
    let cube_table = GuiElement::cube_table_with_cube(cube_arc.clone())
        .row_dimensions(vec!["region".to_string(), "product".to_string()])
        .measures(vec!["revenue".to_string(), "units".to_string(), "cost".to_string()])
        .page_size(Some(15))
        .show_drill_controls(true)
        .filter_context(filter_context.clone())
        .build();

    // Layout: Sidebar
    let sidebar = GuiElement::vstack()
        .spacing(12.0)
        .child(filters_label)
        .child(region_filter)
        .child(product_filter)
        .child(year_filter)
        .child(channel_filter)
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

    let mut instance = StructInstance::new("DashboardState".to_string());
    instance.fields = fields;
    let state = Value::Struct(Rc::new(RefCell::new(instance)));

    // Create and run the GUI
    println!("\nLaunching OLAP Dashboard...");
    println!("- Data: {} rows from {}", df.num_rows(), parquet_path);
    println!("- Bar chart: Revenue by Region");
    println!("- Pie chart: Units by Product");
    println!("- Line chart: Revenue Trend by Year");
    println!("- Data table: Aggregated view with drill-down controls");
    println!("\nTip: Run with --release for better performance!");

    let runtime = GuiRuntime::new(state)
        .with_window("OLAP Cube Dashboard (Large Dataset)", (1200, 800))
        .with_root(root)
        .with_spacing(16.0);

    if let Err(e) = runtime.run() {
        eprintln!("GUI error: {e}");
        std::process::exit(1);
    }
}
