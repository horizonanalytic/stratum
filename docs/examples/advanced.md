# Advanced Examples

These examples demonstrate real-world use cases and more complex Stratum applications.

## Todo App

A simple console-based todo list application.

```stratum
// Todo Application - Simple console version
// Demonstrates basic state management

fx main() {
    print("Todo App Demo");
    print("=============");

    // Simple todo list stored as strings
    let todos = ["Buy groceries", "Write code", "Read book"];

    print("\nMy Todos:");
    let i = 1;
    for todo in todos {
        print("  {i}. {todo}");
        i = i + 1;
    }
}
```

**Output:**
```
Todo App Demo
=============

My Todos:
  1. Buy groceries
  2. Write code
  3. Read book
```

**Key concepts:**
- List iteration with `for` loops
- Manual index tracking
- String interpolation in output

---

## Data Analysis

Basic statistical operations on numeric data.

```stratum
// Data Analysis - Basic statistical operations
// Demonstrates working with numeric data

fx main() {
    print("Data Analysis Demo");
    print("==================");

    // Sample data
    let values = [4.5, 3.2, 5.1, 4.8, 3.9, 5.5, 4.1];

    print("\nValues:");
    for v in values {
        print("  {v}");
    }

    // Calculate sum using a loop
    let sum = 0.0;
    for v in values {
        sum = sum + v;
    }
    print("\nSum: {sum}");

    // Calculate average
    let count = 7;
    let avg = sum / count;
    print("Average: {avg}")
}
```

**Output:**
```
Data Analysis Demo
==================

Values:
  4.5
  3.2
  5.1
  4.8
  3.9
  5.5
  4.1

Sum: 31.1
Average: 4.442857142857143
```

**Key concepts:**
- Working with float lists
- Accumulator pattern for sum calculation
- Basic statistical computations

---

## CSV Processor

Process and analyze tabular data.

```stratum
// CSV Processor - Data processing demo

fx main() {
    print("Data Processing Example");
    print("=======================");

    // Create simple list data
    let regions = ["North", "South", "East", "West"];
    let sales = [100, 75, 120, 50];

    print("\nRegions:");
    for r in regions {
        print("  - {r}");
    }

    print("\nSales:");
    for s in sales {
        print("  {s}");
    }

    // Calculate total
    let total = 0;
    for s in sales {
        total = total + s;
    }
    print("\nTotal sales: {total}");

    print("\nDone!")
}
```

**Output:**
```
Data Processing Example
=======================

Regions:
  - North
  - South
  - East
  - West

Sales:
  100
  75
  120
  50

Total sales: 345

Done!
```

**Key concepts:**
- Parallel lists for related data
- Multiple iterations over different data sets
- Aggregation calculations

---

## Web Scraper

Fetch and process data from web APIs.

```stratum
// Web Scraper - Fetch data from APIs
// Demonstrates HTTP requests and JSON parsing

fx main() {
    print("Web Data Fetcher");
    print("================");

    // Fetch a user from JSONPlaceholder
    print("\nFetching user data...");
    let response = Http.get("https://jsonplaceholder.typicode.com/users/1");
    let user = Json.parse(response.body);

    print("\nUser Info:");
    let name = user.name;
    let email = user.email;
    let company = user.company.name;
    print("  Name: {name}");
    print("  Email: {email}");
    print("  Company: {company}");

    // Fetch posts for this user (limited to 3)
    print("\nFetching posts...");
    let posts_response = Http.get("https://jsonplaceholder.typicode.com/posts?userId=1&_limit=3");
    let posts = Json.parse(posts_response.body);

    print("\nUser's Posts:");
    for post in posts {
        let title = post.title;
        print("  - {title}");
    }

    print("\nDone!");
}
```

**Output:**
```
Web Data Fetcher
================

Fetching user data...

User Info:
  Name: Leanne Graham
  Email: Sincere@april.biz
  Company: Romaguera-Crona

Fetching posts...

User's Posts:
  - sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  - qui est esse
  - ea molestias quasi exercitationem repellat qui ipsa sit aut

Done!
```

**Key concepts:**
- Multiple HTTP requests in sequence
- Nested JSON property access (`user.company.name`)
- Iterating over JSON arrays with `for` loops
- Query parameters in URLs (`?userId=1&_limit=3`)
- Real-world API integration patterns

---

## Earthquake Analytics

A comprehensive data analytics example that fetches real earthquake data from the USGS API, processes it into a DataFrame, builds an OLAP cube, and runs SQL-based analytical queries.

```stratum
// Earthquake Analytics - Data Analysis Example
// Demonstrates: HTTP API fetch, DataFrame operations, OLAP cube, SQL analytics

// Helper function to categorize magnitude
fx categorize_magnitude(mag: Float) -> String {
    if mag >= 7.0 {
        "Major (7.0+)"
    } else {
        if mag >= 6.0 {
            "Strong (6.0-6.9)"
        } else {
            if mag >= 5.0 {
                "Moderate (5.0-5.9)"
            } else {
                "Light (4.5-4.9)"
            }
        }
    }
}

// Helper function to categorize depth
fx categorize_depth(depth: Float) -> String {
    if depth >= 300.0 {
        "Deep (300+ km)"
    } else {
        if depth >= 70.0 {
            "Intermediate (70-300 km)"
        } else {
            "Shallow (0-70 km)"
        }
    }
}

fx main() {
    print("Earthquake Data Analytics");
    print("=========================");
    print("");

    // Step 1: Download earthquake data from USGS API
    print("Step 1: Fetching earthquake data from USGS API...");
    let api_url = "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/4.5_month.geojson";
    let response = Http.get(api_url);
    let geojson = Json.parse(response.body);
    print("  API Response received!");

    // Step 2: Parse GeoJSON and build DataFrame
    print("");
    print("Step 2: Parsing earthquake data into DataFrame...");

    let features = geojson.features;
    let magnitudes = [];
    let depths = [];
    let places = [];
    let times = [];
    let mag_categories = [];
    let depth_categories = [];

    for feature in features {
        let props = feature.properties;
        let geom = feature.geometry;
        let mag = props.mag;
        let coords = geom.coordinates;
        let depth = coords[2];
        let place = props.place;
        let time_ms = props.time;

        let dt = DateTime.from_timestamp(int(time_ms));
        let date = DateTime.format(dt, "%Y-%m-%d");

        let mag_cat = categorize_magnitude(mag);
        let depth_cat = categorize_depth(depth);

        magnitudes.push(float(mag));
        depths.push(float(depth));
        places.push(place);
        times.push(date);
        mag_categories.push(mag_cat);
        depth_categories.push(depth_cat);
    }

    let earthquakes = Data.from_columns(
        "magnitude", magnitudes,
        "depth_km", depths,
        "place", places,
        "date", times,
        "mag_category", mag_categories,
        "depth_category", depth_categories
    );

    let row_count = earthquakes.rows();
    print("  Created DataFrame with {row_count} earthquakes");

    // Step 3: Build OLAP Cube
    print("");
    print("Step 3: Building OLAP Cube...");

    let event_counts = [];
    let i = 0;
    while i < row_count {
        event_counts.push(1);
        i = i + 1;
    }

    let eq_with_count = Data.from_columns(
        "magnitude", magnitudes,
        "depth_km", depths,
        "date", times,
        "mag_category", mag_categories,
        "depth_category", depth_categories,
        "event_count", event_counts
    );

    let cube = Cube.from("EarthquakeAnalysis", eq_with_count)
        .dimension("mag_category")
        .dimension("depth_category")
        .dimension("date")
        .measure("magnitude", "mean")
        .measure("depth_km", "mean")
        .measure("event_count", "sum")
        .build();

    print("  OLAP Cube built successfully!");
    let dims = cube.dimensions();
    let measures = cube.measures();
    print("  Dimensions: {dims}");
    print("  Measures: {measures}");

    // Step 4: Run Analytical Queries with SQL
    print("");
    print("Step 4: Running Analytical Queries");
    print("-----------------------------------");

    print("");
    print("Query 1: Earthquakes by Magnitude Category");
    let q1 = "SELECT mag_category, COUNT(*) as count, ROUND(AVG(magnitude), 2) as avg_mag FROM df GROUP BY mag_category ORDER BY count DESC";
    let result1 = Data.sql(earthquakes, q1);
    print(result1);

    print("");
    print("Query 2: Earthquakes by Depth Category");
    let q2 = "SELECT depth_category, COUNT(*) as count, ROUND(AVG(depth_km), 1) as avg_depth FROM df GROUP BY depth_category ORDER BY count DESC";
    let result2 = Data.sql(earthquakes, q2);
    print(result2);

    print("");
    print("Query 3: Top 10 Strongest Earthquakes");
    let q3 = "SELECT magnitude, depth_km, place, date FROM df ORDER BY magnitude DESC LIMIT 10";
    let result3 = Data.sql(earthquakes, q3);
    print(result3);

    // Step 5: Save processed data
    print("");
    print("Step 5: Saving processed data...");
    Data.write_csv(earthquakes, "earthquakes_processed.csv");
    print("  Saved to earthquakes_processed.csv");

    let cube_df = cube.to_dataframe();
    Data.write_parquet(cube_df, "earthquake_cube.parquet");
    print("  Saved cube data to earthquake_cube.parquet");

    print("");
    print("Analysis Complete!");
}
```

**Output:**
```
Earthquake Data Analytics
=========================

Step 1: Fetching earthquake data from USGS API...
  API Response received!

Step 2: Parsing earthquake data into DataFrame...
  Created DataFrame with 416 earthquakes

Step 3: Building OLAP Cube...
  OLAP Cube built successfully!
  Dimensions: [mag_category, depth_category, date]
  Measures: [magnitude, depth_km, event_count]

Step 4: Running Analytical Queries
-----------------------------------

Query 1: Earthquakes by Magnitude Category
+--------------------+-------+---------+
| mag_category       | count | avg_mag |
+--------------------+-------+---------+
| Light (4.5-4.9)    | 264   | 4.68    |
| Moderate (5.0-5.9) | 142   | 5.23    |
| Strong (6.0-6.9)   | 10    | 6.33    |
+--------------------+-------+---------+

Query 2: Earthquakes by Depth Category
+--------------------------+-------+-----------+
| depth_category           | count | avg_depth |
+--------------------------+-------+-----------+
| Shallow (0-70 km)        | 294   | 26.4      |
| Intermediate (70-300 km) | 89    | 141.7     |
| Deep (300+ km)           | 33    | 523.9     |
+--------------------------+-------+-----------+

Query 3: Top 10 Strongest Earthquakes
+-----------+----------+---------------------------------------+------------+
| magnitude | depth_km | place                                 | date       |
+-----------+----------+---------------------------------------+------------+
| 6.7       | 19.0     | 111 km NE of Kuji, Japan              | 2025-12-12 |
| 6.6       | 67.534   | 31 km ESE of Yilan, Taiwan            | 2025-12-27 |
| 6.5       | 35.0     | 4 km NNW of Rancho Viejo, Mexico      | 2026-01-02 |
| ...       | ...      | ...                                   | ...        |
+-----------+----------+---------------------------------------+------------+

Step 5: Saving processed data...
  Saved to earthquakes_processed.csv
  Saved cube data to earthquake_cube.parquet

Analysis Complete!
```

**Key concepts:**
- Fetching real-time data from REST APIs (`Http.get`)
- Parsing GeoJSON with nested property access
- Building DataFrames from column arrays (`Data.from_columns`)
- Creating OLAP cubes with dimensions and measures (`Cube.from().dimension().measure().build()`)
- SQL analytics on DataFrames (`Data.sql`)
- Helper functions with explicit return types
- Type conversion (`int()`, `float()`) for JSON values
- DateTime formatting (`DateTime.from_timestamp`, `DateTime.format`)
- Exporting to CSV and Parquet formats
