# Cube

OLAP (Online Analytical Processing) cube for multi-dimensional data analysis.

## Overview

The `Cube` namespace provides multi-dimensional analysis capabilities for exploring data across multiple dimensions. Cubes support:

- **Dimensions**: Categorical axes for grouping (e.g., region, product, time)
- **Measures**: Numeric values to aggregate (e.g., revenue, quantity)
- **Hierarchies**: Drill-down paths within dimensions (e.g., Year → Quarter → Month)
- **OLAP Operations**: Slice, dice, drill-down, and roll-up

Cubes are built from DataFrames and provide a powerful abstraction for business intelligence and analytical queries.

---

## Cube Creation

### `Cube.from(df)` / `Cube.from(name, df)`

Creates a CubeBuilder from a DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String?` | Optional cube name for identification |
| `df` | `DataFrame` | Source DataFrame |

**Returns:** `CubeBuilder` - Builder for configuring the cube

**Example:**

```stratum
let sales = Data.read_csv("sales.csv")

// Anonymous cube
let cube = Cube.from(sales)
    |> .dimension("region", "product")
    |> .measure("revenue", "sum")
    |> .build()

// Named cube
let named = Cube.from("SalesCube", sales)
    |> .dimension("region")
    |> .measure("revenue", "sum")
    |> .build()
```

---

### `df.to_cube(name?)`

Alternative way to create a CubeBuilder from a DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String?` | Optional cube name |

**Returns:** `CubeBuilder` - Builder for configuring the cube

**Example:**

```stratum
let cube = sales_df.to_cube("Sales")
    |> .dimension("region", "product", "year")
    |> .measure("revenue", "sum")
    |> .measure("quantity", "sum")
    |> .build()
```

---

## CubeBuilder Methods

### `builder.dimension(columns...)`

Adds one or more dimension columns to the cube.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Column names to use as dimensions |

**Returns:** `CubeBuilder` - The builder for chaining

**Example:**

```stratum
let builder = Cube.from(df)
    |> .dimension("region")           // Single dimension
    |> .dimension("product", "year")  // Multiple dimensions
```

---

### `builder.measure(name, aggregation)`

Adds a measure with an aggregation function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Column name to aggregate |
| `aggregation` | `String` | Aggregation function: `"sum"`, `"count"`, `"mean"`, `"min"`, `"max"` |

**Returns:** `CubeBuilder` - The builder for chaining

**Example:**

```stratum
let builder = Cube.from(df)
    |> .dimension("region")
    |> .measure("revenue", "sum")
    |> .measure("orders", "count")
    |> .measure("avg_order", "mean")
```

---

### `builder.hierarchy(name, levels)`

Defines a drill-down hierarchy within a dimension.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Hierarchy name for reference |
| `levels` | `List<String>` | Column names from coarse to fine granularity |

**Returns:** `CubeBuilder` - The builder for chaining

**Example:**

```stratum
let cube = Cube.from(sales)
    |> .dimension("year", "quarter", "month", "region", "city")
    |> .hierarchy("time", ["year", "quarter", "month"])
    |> .hierarchy("geography", ["region", "city"])
    |> .measure("revenue", "sum")
    |> .build()
```

---

### `builder.build()`

Finalizes the cube definition and creates an immutable Cube.

**Returns:** `Cube` - The constructed OLAP cube

**Throws:** Error if no dimensions or measures are defined

**Example:**

```stratum
let cube = Cube.from(df)
    |> .dimension("region")
    |> .measure("revenue", "sum")
    |> .build()
```

---

## OLAP Operations

### `cube.slice(dimension, value)`

Filters the cube to a single value on one dimension. Like taking a "slice" of the cube.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dimension` | `String` | Dimension to filter |
| `value` | `Value` | Value to select |

**Returns:** `Cube` - Filtered cube

**Example:**

```stratum
// Focus on a specific region
let west_cube = cube.slice("region", "West")

// Get 2024 data only
let current_year = cube.slice("year", 2024)
```

---

### `cube.dice(filters)`

Filters the cube on multiple dimensions simultaneously. Like cutting a "dice" from the cube.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `filters` | `Map` | Map of dimension names to filter values |

**Returns:** `Cube` - Filtered cube

**Example:**

```stratum
// Filter by multiple dimensions
let subset = cube.dice({
    region: "West",
    year: 2024,
    product: "Electronics"
})
```

---

### `cube.drill_down(hierarchy, levels?)`

Navigates to a more detailed level in a hierarchy.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `hierarchy` | `String` | Hierarchy name to drill into |
| `levels` | `Int?` | Number of levels to drill (default: 1) |

**Returns:** `Cube` - Cube at the finer granularity

**Example:**

```stratum
// Start at year level, drill down to quarters
let by_year = cube.slice("year", 2024)
let by_quarter = by_year.drill_down("time")

// Drill down 2 levels at once (year → month, skipping quarter)
let by_month = cube.drill_down("time", 2)
```

---

### `cube.roll_up(hierarchy, levels?)`

Navigates to a less detailed level in a hierarchy (opposite of drill-down).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `hierarchy` | `String` | Hierarchy name to roll up |
| `levels` | `Int?` | Number of levels to roll up (default: 1) |

**Returns:** `Cube` - Cube at the coarser granularity

**Example:**

```stratum
// Go from monthly to quarterly view
let monthly_data = cube.slice("month", "January")
let quarterly = monthly_data.roll_up("time")

// Roll up from city to region
let regional = city_cube.roll_up("geography")
```

---

## Query Interface

For complex queries, use the fluent query builder.

### `cube.query()`

Creates a new query builder for the cube.

**Returns:** `CubeQuery` - Query builder

**Example:**

```stratum
let result = cube.query()
    |> .cube_select("region", "revenue")
    |> .where_("year = 2024")
    |> .cube_group_by("region")
    |> .cube_order_by("-revenue")
    |> .cube_limit(10)
    |> .execute()
```

---

### `query.cube_select(columns...)`

Specifies which dimensions and measures to include in the result.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Column names to select |

**Returns:** `CubeQuery` - Query builder for chaining

---

### `query.where_(expression)`

Filters rows using a SQL-style expression.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `expression` | `String` | SQL WHERE clause expression |

**Returns:** `CubeQuery` - Query builder for chaining

**Example:**

```stratum
query.where_("revenue > 10000 AND region = 'West'")
```

---

### `query.cube_group_by(columns...)`

Groups results by the specified dimensions.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Dimension columns to group by |

**Returns:** `CubeQuery` - Query builder for chaining

---

### `query.cube_order_by(columns...)`

Sorts results by the specified columns. Prefix with `-` for descending order.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Columns to sort by (prefix `-` for DESC) |

**Returns:** `CubeQuery` - Query builder for chaining

**Example:**

```stratum
query.cube_order_by("-revenue", "region")  // DESC revenue, ASC region
```

---

### `query.cube_limit(n)`

Limits the number of result rows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | Maximum rows to return |

**Returns:** `CubeQuery` - Query builder for chaining

---

### `query.execute()`

Executes the query and returns results as a DataFrame.

**Returns:** `DataFrame` - Query results

**Example:**

```stratum
let top_regions = cube.query()
    |> .cube_select("region", "revenue", "orders")
    |> .where_("year = 2024")
    |> .cube_group_by("region")
    |> .cube_order_by("-revenue")
    |> .cube_limit(5)
    |> .execute()
```

---

## Metadata Methods

### `cube.dimensions()`

Returns the list of dimension names.

**Returns:** `List<String>` - Dimension column names

---

### `cube.measures()`

Returns the list of measure names.

**Returns:** `List<String>` - Measure column names

---

### `cube.hierarchies()`

Returns the defined hierarchies.

**Returns:** `Map` - Hierarchy names mapped to their level lists

**Example:**

```stratum
let h = cube.hierarchies()
// {time: ["year", "quarter", "month"], geography: ["region", "city"]}
```

---

### `cube.dimension_values(dimension)`

Returns all unique values for a dimension.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dimension` | `String` | Dimension name |

**Returns:** `List` - Unique values in the dimension

**Example:**

```stratum
let regions = cube.dimension_values("region")
// ["North", "South", "East", "West"]
```

---

### `cube.current_level(hierarchy)`

Returns the current drill level for a hierarchy.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `hierarchy` | `String` | Hierarchy name |

**Returns:** `String` - Current level name

---

## Conversion

### `cube.to_dataframe()` / `query.to_dataframe()`

Converts a cube or query result to a DataFrame.

**Returns:** `DataFrame` - Data as a DataFrame

---

## Complete Example

```stratum
// Load sales data
let sales = Data.read_csv("sales.csv")

// Build an OLAP cube
let cube = sales.to_cube("SalesAnalysis")
    |> .dimension("year", "quarter", "month")
    |> .dimension("region", "city")
    |> .dimension("product_category", "product")
    |> .hierarchy("time", ["year", "quarter", "month"])
    |> .hierarchy("geography", ["region", "city"])
    |> .hierarchy("product", ["product_category", "product"])
    |> .measure("revenue", "sum")
    |> .measure("quantity", "sum")
    |> .measure("orders", "count")
    |> .build()

// High-level overview: revenue by region
let regional = cube.query()
    |> .cube_select("region", "revenue")
    |> .cube_group_by("region")
    |> .cube_order_by("-revenue")
    |> .execute()

println(regional)

// Drill down: West region by quarter
let west_quarterly = cube
    |> .slice("region", "West")
    |> .drill_down("time")  // year → quarter
    |> .query()
    |> .cube_select("quarter", "revenue", "orders")
    |> .cube_group_by("quarter")
    |> .execute()

println(west_quarterly)

// Dice: specific subset analysis
let q4_electronics = cube.dice({
    quarter: "Q4",
    product_category: "Electronics"
})

let detailed = q4_electronics.query()
    |> .cube_select("city", "product", "revenue")
    |> .cube_group_by("city", "product")
    |> .cube_order_by("-revenue")
    |> .cube_limit(20)
    |> .execute()

println(detailed)
```

---

## See Also

- [Data](data.md) - DataFrame operations and SQL queries
- [Agg](agg.md) - Aggregation specifications
