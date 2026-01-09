# Agg

Builder functions for creating aggregation specifications used with DataFrame `group_by()` and `aggregate()`.

## Overview

The `Agg` namespace provides a fluent API for specifying aggregations when grouping data. Each function returns an `AggSpec` that describes what column to aggregate, how to aggregate it, and optionally what to name the output column.

These specifications are passed to `grouped.aggregate()` after a `group_by()` call.

```stratum
df.group_by("category")
    |> .aggregate(
        Agg.count("num_items"),
        Agg.sum("price", "total_price"),
        Agg.mean("price", "avg_price")
    )
```

---

## Functions

### `Agg.sum(column, output_name?)`

Sums values in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to sum |
| `output_name` | `String?` | Output column name (default: `column_sum`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
let result = df.group_by("region")
    |> .aggregate(Agg.sum("revenue", "total_revenue"))

// Output columns: region, total_revenue
```

---

### `Agg.mean(column, output_name?)`

Calculates the arithmetic mean of values in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to average |
| `output_name` | `String?` | Output column name (default: `column_mean`) |

**Returns:** `AggSpec` - Aggregation specification

**Aliases:** `Agg.avg(column, output_name?)`

**Example:**

```stratum
let result = df.group_by("department")
    |> .aggregate(Agg.mean("salary", "avg_salary"))
```

---

### `Agg.min(column, output_name?)`

Finds the minimum value in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to find minimum |
| `output_name` | `String?` | Output column name (default: `column_min`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
let result = df.group_by("product")
    |> .aggregate(Agg.min("price", "lowest_price"))
```

---

### `Agg.max(column, output_name?)`

Finds the maximum value in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to find maximum |
| `output_name` | `String?` | Output column name (default: `column_max`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
let result = df.group_by("product")
    |> .aggregate(Agg.max("price", "highest_price"))
```

---

### `Agg.count(output_name?)`

Counts the number of rows in each group.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `output_name` | `String?` | Output column name (default: `count`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
let result = df.group_by("status")
    |> .aggregate(Agg.count("num_records"))
```

---

### `Agg.first(column, output_name?)`

Takes the first value in each group.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to take first value from |
| `output_name` | `String?` | Output column name (default: `column_first`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
// Get first order date for each customer
let result = df
    |> .sort_by("order_date")
    |> .group_by("customer_id")
    |> .aggregate(Agg.first("order_date", "first_order"))
```

---

### `Agg.last(column, output_name?)`

Takes the last value in each group.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to take last value from |
| `output_name` | `String?` | Output column name (default: `column_last`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
// Get most recent order for each customer
let result = df
    |> .sort_by("order_date")
    |> .group_by("customer_id")
    |> .aggregate(Agg.last("order_date", "last_order"))
```

---

### `Agg.std(column, output_name?)`

Calculates the standard deviation of values in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to calculate standard deviation |
| `output_name` | `String?` | Output column name (default: `column_std`) |

**Returns:** `AggSpec` - Aggregation specification

**Aliases:** `Agg.stddev(column, output_name?)`

**Example:**

```stratum
let result = df.group_by("category")
    |> .aggregate(Agg.std("price", "price_stddev"))
```

---

### `Agg.var(column, output_name?)`

Calculates the variance of values in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to calculate variance |
| `output_name` | `String?` | Output column name (default: `column_var`) |

**Returns:** `AggSpec` - Aggregation specification

**Aliases:** `Agg.variance(column, output_name?)`

**Example:**

```stratum
let result = df.group_by("region")
    |> .aggregate(Agg.var("sales", "sales_variance"))
```

---

### `Agg.median(column, output_name?)`

Calculates the median of values in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to find median |
| `output_name` | `String?` | Output column name (default: `column_median`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
let result = df.group_by("department")
    |> .aggregate(Agg.median("salary", "median_salary"))
```

---

### `Agg.mode(column, output_name?)`

Finds the most frequent value in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to find mode |
| `output_name` | `String?` | Output column name (default: `column_mode`) |

**Returns:** `AggSpec` - Aggregation specification

**Example:**

```stratum
let result = df.group_by("store")
    |> .aggregate(Agg.mode("product_category", "most_popular"))
```

---

### `Agg.count_distinct(column, output_name?)`

Counts the number of distinct (unique) values in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to count distinct values |
| `output_name` | `String?` | Output column name (default: `column_distinct`) |

**Returns:** `AggSpec` - Aggregation specification

**Aliases:** `Agg.nunique(column, output_name?)`

**Example:**

```stratum
let result = df.group_by("region")
    |> .aggregate(
        Agg.count("total_orders"),
        Agg.count_distinct("customer_id", "unique_customers")
    )
```

---

## Combined Examples

### Multiple Aggregations

```stratum
let sales_summary = sales_df.group_by("region", "year")
    |> .aggregate(
        Agg.count("num_orders"),
        Agg.sum("quantity", "total_units"),
        Agg.sum("revenue", "total_revenue"),
        Agg.mean("revenue", "avg_order_value"),
        Agg.min("revenue", "min_order"),
        Agg.max("revenue", "max_order")
    )
```

### Statistics Dashboard

```stratum
let employee_stats = employees.group_by("department")
    |> .aggregate(
        Agg.count("headcount"),
        Agg.mean("salary", "avg_salary"),
        Agg.min("salary", "min_salary"),
        Agg.max("salary", "max_salary"),
        Agg.sum("salary", "total_payroll")
    )
    |> .sort_by("-headcount")
```

### First/Last Analysis

```stratum
let customer_lifecycle = orders
    |> .sort_by("order_date")
    |> .group_by("customer_id")
    |> .aggregate(
        Agg.count("order_count"),
        Agg.first("order_date", "first_purchase"),
        Agg.last("order_date", "last_purchase"),
        Agg.sum("total", "lifetime_value")
    )
```

---

## See Also

- [Data](data.md) - DataFrame operations including group_by
- [Cube](cube.md) - OLAP cube aggregations
