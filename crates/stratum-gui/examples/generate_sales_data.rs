//! Generate fake sales data for performance testing
//!
//! Run with: cargo run --package stratum-gui --example generate_sales_data
//!
//! This generates a parquet file with 1M+ rows of fake sales data.

use stratum_core::data::{write_parquet, DataFrame, Series};

fn main() {
    let num_rows: usize = 2_000_000; // 2 million rows

    println!("Generating {} rows of fake sales data...", num_rows);

    // Define our dimension values
    let regions = [
        "North",
        "South",
        "East",
        "West",
        "Central",
        "Northwest",
        "Southwest",
        "Northeast",
        "Southeast",
        "Midwest",
    ];
    let products = [
        "Widgets",
        "Gadgets",
        "Gizmos",
        "Doohickeys",
        "Thingamajigs",
        "Whatchamacallits",
        "Doodads",
        "Contraptions",
    ];
    let quarters = ["Q1", "Q2", "Q3", "Q4"];
    let years = ["2021", "2022", "2023", "2024"];
    let channels = ["Online", "Retail", "Wholesale", "Direct"];
    let categories = ["Electronics", "Home", "Office", "Outdoor", "Sports"];

    // Pre-allocate vectors
    let mut region_data: Vec<&str> = Vec::with_capacity(num_rows);
    let mut product_data: Vec<&str> = Vec::with_capacity(num_rows);
    let mut quarter_data: Vec<&str> = Vec::with_capacity(num_rows);
    let mut year_data: Vec<&str> = Vec::with_capacity(num_rows);
    let mut channel_data: Vec<&str> = Vec::with_capacity(num_rows);
    let mut category_data: Vec<&str> = Vec::with_capacity(num_rows);
    let mut revenue_data: Vec<f64> = Vec::with_capacity(num_rows);
    let mut units_data: Vec<i64> = Vec::with_capacity(num_rows);
    let mut cost_data: Vec<f64> = Vec::with_capacity(num_rows);

    // Simple pseudo-random number generator (LCG)
    let mut seed: u64 = 12345;
    let lcg_next = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    println!("Generating data...");
    for i in 0..num_rows {
        if i % 500_000 == 0 && i > 0 {
            println!("  Generated {} rows...", i);
        }

        // Use LCG for pseudo-random selection
        let r = lcg_next(&mut seed);

        region_data.push(regions[(r as usize) % regions.len()]);
        product_data.push(products[((r >> 8) as usize) % products.len()]);
        quarter_data.push(quarters[((r >> 16) as usize) % quarters.len()]);
        year_data.push(years[((r >> 24) as usize) % years.len()]);
        channel_data.push(channels[((r >> 32) as usize) % channels.len()]);
        category_data.push(categories[((r >> 40) as usize) % categories.len()]);

        // Generate revenue between 100 and 10000
        let revenue = 100.0 + ((r >> 48) as f64 % 9900.0);
        revenue_data.push(revenue);

        // Generate units between 1 and 100
        let units = 1 + ((r >> 56) as i64 % 100);
        units_data.push(units);

        // Cost is 40-70% of revenue
        let cost_ratio = 0.4 + ((r >> 4) as f64 % 30.0) / 100.0;
        cost_data.push(revenue * cost_ratio);
    }

    println!("Creating Series...");

    // Create Series using stratum-core API
    let region_series = Series::from_strings("region", region_data);
    let product_series = Series::from_strings("product", product_data);
    let quarter_series = Series::from_strings("quarter", quarter_data);
    let year_series = Series::from_strings("year", year_data);
    let channel_series = Series::from_strings("channel", channel_data);
    let category_series = Series::from_strings("category", category_data);
    let revenue_series = Series::from_floats("revenue", revenue_data);
    let units_series = Series::from_ints("units", units_data);
    let cost_series = Series::from_floats("cost", cost_data);

    println!("Creating DataFrame...");

    // Create DataFrame
    let df = DataFrame::from_series(vec![
        region_series,
        product_series,
        quarter_series,
        year_series,
        channel_series,
        category_series,
        revenue_series,
        units_series,
        cost_series,
    ])
    .expect("Failed to create DataFrame");

    println!("DataFrame created with {} rows", df.num_rows());

    // Write to parquet
    let output_path = "sales_data_2m.parquet";
    println!("Writing to {}...", output_path);

    write_parquet(&df, output_path).expect("Failed to write parquet file");

    println!("Done! Created {} with {} rows", output_path, num_rows);
    println!("\nTo test with the dashboard, run:");
    println!("  cargo run --release --package stratum-gui --example cube_dashboard_large");
}
