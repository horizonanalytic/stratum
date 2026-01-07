# Data

High-performance columnar data operations with Arrow-backed DataFrames and Series.

## Overview

The `Data` namespace provides Stratum's data analysis capabilities built on Apache Arrow for SIMD-accelerated columnar operations. DataFrames store tabular data with typed columns, while Series represent single columns of homogeneous data.

Key features include:
- **Lazy evaluation** with query optimization
- **Pipeline operator** (`|>`) integration for fluent data transformations
- **Built-in SQL** support for familiar query syntax
- **Parquet, CSV, and JSON** file I/O
- **Database integration** via `from_query()`

DataFrames are immutable—transformation methods return new DataFrames rather than modifying in place.

---

## DataFrame Creation

### `Data.frame(rows)`

Creates a DataFrame from a list of row objects.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `rows` | `List<Map>` | List of maps where keys are column names |

**Returns:** `DataFrame` - A new DataFrame with inferred column types

**Example:**

```stratum
let df = Data.frame([
    {name: "Alice", age: 30, city: "NYC"},
    {name: "Bob", age: 25, city: "LA"},
    {name: "Carol", age: 35, city: "NYC"}
])

println(df.columns())  // ["name", "age", "city"]
println(df.rows())     // 3
```

---

### `Data.series(name, values)`

Creates a named Series from a list of values.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | The column name |
| `values` | `List` | List of values (type is inferred) |

**Returns:** `Series` - A new Series with the given name and values

**Example:**

```stratum
let ages = Data.series("age", [30, 25, 35, 28])
println(ages.name())   // "age"
println(ages.len())    // 4
println(ages.mean())   // 29.5
```

---

### `Data.from_columns(name1, values1, name2, values2, ...)`

Creates a DataFrame from alternating column names and value lists.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name1` | `String` | First column name |
| `values1` | `List` | First column values |
| `...` | `String, List` | Additional name/values pairs |

**Returns:** `DataFrame` - A new DataFrame with the specified columns

**Throws:** Error if column lengths don't match

**Example:**

```stratum
let df = Data.from_columns(
    "name", ["Alice", "Bob", "Carol"],
    "age", [30, 25, 35],
    "active", [true, true, false]
)
```

---

## File I/O

### `Data.read_parquet(path)`

Reads a Parquet file into a DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the Parquet file |

**Returns:** `DataFrame` - DataFrame containing the file data

**Throws:** Error if file doesn't exist or is invalid

**Example:**

```stratum
let df = Data.read_parquet("data/sales.parquet")
println(df.schema())
```

---

### `Data.read_csv(path, has_header?, delimiter?)`

Reads a CSV file into a DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the CSV file |
| `has_header` | `Bool?` | Whether first row is header (default: `true`) |
| `delimiter` | `String?` | Field delimiter (default: `","`) |

**Returns:** `DataFrame` - DataFrame containing the file data

**Throws:** Error if file doesn't exist or is malformed

**Example:**

```stratum
// Standard CSV with header
let df = Data.read_csv("data/users.csv")

// Tab-separated without header
let tsv = Data.read_csv("data/raw.tsv", false, "\t")
```

---

### `Data.read_json(path)`

Reads a JSON file into a DataFrame. Expects an array of objects.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to the JSON file |

**Returns:** `DataFrame` - DataFrame containing the file data

**Throws:** Error if file doesn't exist or isn't valid JSON array

**Example:**

```stratum
// File: [{"name": "Alice", "score": 95}, {"name": "Bob", "score": 87}]
let df = Data.read_json("data/scores.json")
```

---

### `Data.write_parquet(df, path)`

Writes a DataFrame to a Parquet file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `df` | `DataFrame` | The DataFrame to write |
| `path` | `String` | Output file path |

**Returns:** `Null`

**Example:**

```stratum
let df = Data.frame([{x: 1, y: 2}, {x: 3, y: 4}])
Data.write_parquet(df, "output/data.parquet")
```

---

### `Data.write_csv(df, path)`

Writes a DataFrame to a CSV file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `df` | `DataFrame` | The DataFrame to write |
| `path` | `String` | Output file path |

**Returns:** `Null`

**Example:**

```stratum
df.to_csv("output/data.csv")  // Method form
Data.write_csv(df, "output/data.csv")  // Function form
```

---

### `Data.write_json(df, path)`

Writes a DataFrame to a JSON file as an array of objects.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `df` | `DataFrame` | The DataFrame to write |
| `path` | `String` | Output file path |

**Returns:** `Null`

**Example:**

```stratum
df.to_json("output/data.json")
```

---

## SQL Operations

### `Data.sql(df, query)`

Executes a SQL query against a single DataFrame. The DataFrame is available as `df` in the query.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `df` | `DataFrame` | The DataFrame to query |
| `query` | `String` | SQL query string |

**Returns:** `DataFrame` - Query result as a new DataFrame

**Example:**

```stratum
let sales = Data.read_csv("sales.csv")

// Filter and aggregate with SQL
let result = Data.sql(sales, "
    SELECT region, SUM(amount) as total
    FROM df
    WHERE year = 2024
    GROUP BY region
    ORDER BY total DESC
")
```

---

### `Data.sql_context()`

Creates a SQL context for multi-table queries. Use `register()` to add tables and `query()` to execute.

**Returns:** `SqlContext` - A new SQL context builder

**Example:**

```stratum
let users = Data.read_csv("users.csv")
let orders = Data.read_csv("orders.csv")

let result = Data.sql_context()
    |> .register("users", users)
    |> .register("orders", orders)
    |> .query("
        SELECT u.name, COUNT(o.id) as order_count
        FROM users u
        LEFT JOIN orders o ON u.id = o.user_id
        GROUP BY u.name
    ")
```

---

### `Data.from_query(db, sql, params?)`

Executes a SQL query against a database connection and returns the result as a DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `db` | `DbConnection` | Database connection from `Db.connect()` |
| `sql` | `String` | SQL query string |
| `params` | `List?` | Optional query parameters |

**Returns:** `DataFrame` - Query result as a DataFrame

**Example:**

```stratum
let db = Db.connect("postgresql://localhost/mydb")
let df = Data.from_query(db, "SELECT * FROM users WHERE active = $1", [true])
```

---

## DataFrame Methods

### Inspection

#### `df.columns()`

Returns the list of column names.

**Returns:** `List<String>` - Column names in order

**Example:**

```stratum
let cols = df.columns()  // ["name", "age", "city"]
```

---

#### `df.rows()`

Returns the number of rows in the DataFrame.

**Returns:** `Int` - Row count

**Example:**

```stratum
let count = df.rows()  // 1000
```

---

#### `df.schema()`

Returns schema information about the DataFrame's columns and types.

**Returns:** `Map` - Column names mapped to type information

**Example:**

```stratum
let schema = df.schema()
// {name: "String", age: "Int64", active: "Bool"}
```

---

### Row Access

#### `df.head(n?)`

Returns the first n rows of the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int?` | Number of rows (default: 5) |

**Returns:** `DataFrame` - First n rows

**Aliases:** `df.take(n)`, `df.limit(n)`

**Example:**

```stratum
let top5 = df.head()
let top10 = df.head(10)
```

---

#### `df.tail(n?)`

Returns the last n rows of the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int?` | Number of rows (default: 5) |

**Returns:** `DataFrame` - Last n rows

**Example:**

```stratum
let bottom5 = df.tail()
```

---

#### `df.sample(n)`

Returns n randomly sampled rows from the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | Number of rows to sample |

**Returns:** `DataFrame` - Random sample of rows

**Example:**

```stratum
let sample = df.sample(100)
```

---

### Transformation

#### `df.select(columns...)`

Selects specific columns from the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Column names to select |

**Returns:** `DataFrame` - DataFrame with only the specified columns

**Aliases:** `df.map(columns...)`

**Example:**

```stratum
let subset = df.select("name", "age")

// With pipeline operator
df |> .select("name", "email")
```

---

#### `df.filter(predicate)`

Filters rows based on a predicate function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `predicate` | `Function` | Function that takes a row and returns Bool |

**Returns:** `DataFrame` - DataFrame containing only rows where predicate is true

**Example:**

```stratum
let adults = df.filter(|row| row.age >= 18)

let nyc_users = df.filter(|row| row.city == "NYC")

// Pipeline style
df |> .filter(|row| row.active)
```

---

#### `df.rename(old_name, new_name)`

Renames a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `old_name` | `String` | Current column name |
| `new_name` | `String` | New column name |

**Returns:** `DataFrame` - DataFrame with the renamed column

**Example:**

```stratum
let renamed = df.rename("user_id", "id")
```

---

#### `df.drop(column)`

Removes a column from the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column name to remove |

**Returns:** `DataFrame` - DataFrame without the specified column

**Example:**

```stratum
let cleaned = df.drop("internal_id")
```

---

#### `df.sort_by(columns...)`

Sorts the DataFrame by one or more columns. Prefix column name with `-` for descending order.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Column names (prefix with `-` for DESC) |

**Returns:** `DataFrame` - Sorted DataFrame

**Example:**

```stratum
// Ascending by name
let sorted = df.sort_by("name")

// Descending by age, then ascending by name
let multi = df.sort_by("-age", "name")
```

---

#### `df.distinct()`

Returns unique rows (removes duplicates).

**Returns:** `DataFrame` - DataFrame with duplicate rows removed

**Aliases:** `df.unique()`

**Example:**

```stratum
let unique_cities = df.select("city").distinct()
```

---

### Aggregation

#### `df.group_by(columns...)`

Groups the DataFrame by one or more columns. Returns a GroupedDataFrame that supports aggregation.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Columns to group by |

**Returns:** `GroupedDataFrame` - Grouped data ready for aggregation

**Example:**

```stratum
let by_city = df.group_by("city")
    |> .aggregate(Agg.count(), Agg.mean("age", "avg_age"))
```

---

#### `grouped.aggregate(specs...)`

Applies aggregation functions to a grouped DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `specs` | `AggSpec...` | Aggregation specifications from `Agg` namespace |

**Returns:** `DataFrame` - Aggregated result

**Example:**

```stratum
let stats = df.group_by("department")
    |> .aggregate(
        Agg.count("employee_count"),
        Agg.sum("salary", "total_salary"),
        Agg.mean("salary", "avg_salary"),
        Agg.max("salary", "max_salary")
    )
```

---

#### Direct Aggregations

DataFrames also support direct aggregation methods that operate on all numeric columns:

```stratum
df.sum()      // Sum of each numeric column
df.mean()     // Mean of each numeric column
df.min()      // Minimum of each column
df.max()      // Maximum of each column
df.count()    // Row count
df.first()    // First row
df.last()     // Last row
```

---

### Joins

#### `df.join(other, spec)`

Joins two DataFrames based on a join specification.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `DataFrame` | DataFrame to join with |
| `spec` | `JoinSpec` | Join specification from `Join` namespace |

**Returns:** `DataFrame` - Joined DataFrame

**Example:**

```stratum
let users = Data.read_csv("users.csv")
let orders = Data.read_csv("orders.csv")

// Inner join on same column name
let joined = users.join(orders, Join.on("user_id"))

// Left join with different column names
let result = users.join(orders, Join.left_cols("id", "user_id"))
```

See [Join](join.md) namespace for all join types and options.

---

### Conversion

#### `df.to_parquet(path)`

Writes the DataFrame to a Parquet file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Output file path |

**Returns:** `Null`

---

#### `df.to_csv(path)`

Writes the DataFrame to a CSV file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Output file path |

**Returns:** `Null`

---

#### `df.to_json(path)`

Writes the DataFrame to a JSON file.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Output file path |

**Returns:** `Null`

---

#### `df.to_cube(name?)`

Converts the DataFrame to a CubeBuilder for OLAP operations.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String?` | Optional cube name |

**Returns:** `CubeBuilder` - Builder for constructing an OLAP cube

**Example:**

```stratum
let cube = sales_df.to_cube("sales")
    |> .dimension("region", "product", "year")
    |> .measure("revenue", "sum")
    |> .build()
```

See [Cube](cube.md) namespace for OLAP operations.

---

## Series Methods

Series represent a single column of typed data. They support element access, aggregations, and string operations.

### Information

#### `series.name()`

Returns the series name.

**Returns:** `String` - The column name

---

#### `series.len()`

Returns the number of elements.

**Returns:** `Int` - Element count

**Aliases:** `series.length()`

---

#### `series.dtype()`

Returns the data type of the series.

**Returns:** `String` - Type name (e.g., "Int64", "Utf8", "Float64")

---

#### `series.null_count()`

Returns the count of null values.

**Returns:** `Int` - Number of nulls

---

#### `series.is_empty()`

Checks if the series has no elements.

**Returns:** `Bool` - True if length is 0

---

### Access

#### `series.get(index)`

Gets the element at the specified index.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `index` | `Int` | Zero-based index |

**Returns:** `Value?` - The value at the index, or null if out of bounds

**Example:**

```stratum
let first = ages.get(0)
let last = ages.get(ages.len() - 1)
```

---

#### `series.is_null(index)`

Checks if the value at an index is null.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `index` | `Int` | Zero-based index |

**Returns:** `Bool` - True if the value is null

---

### Aggregation

All aggregation methods return a single value computed over the entire series.

```stratum
series.sum()      // Sum of all values
series.mean()     // Arithmetic mean
series.min()      // Minimum value
series.max()      // Maximum value
series.std()      // Standard deviation
series.var()      // Variance
series.median()   // Median value
series.count()    // Count of non-null values
```

**Example:**

```stratum
let prices = Data.series("price", [10.5, 20.0, 15.5, 30.0, 25.0])

println(prices.sum())     // 101.0
println(prices.mean())    // 20.2
println(prices.min())     // 10.5
println(prices.max())     // 30.0
println(prices.median())  // 20.0
```

---

### String Operations

For String-typed series, additional string methods are available:

#### `series.str_len()`

Returns a new series with the length of each string.

**Returns:** `Series<Int>` - String lengths

---

#### `series.str_contains(pattern)`

Returns a boolean series indicating pattern matches.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String` | Substring to search for |

**Returns:** `Series<Bool>` - True where pattern is found

---

#### `series.str_starts_with(prefix)`

Checks if strings start with the given prefix.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `prefix` | `String` | Prefix to check |

**Returns:** `Series<Bool>` - True where prefix matches

---

#### `series.str_ends_with(suffix)`

Checks if strings end with the given suffix.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `suffix` | `String` | Suffix to check |

**Returns:** `Series<Bool>` - True where suffix matches

---

#### `series.str_to_uppercase()` / `series.upper()`

Converts all strings to uppercase.

**Returns:** `Series<String>` - Uppercase strings

---

#### `series.str_to_lowercase()` / `series.lower()`

Converts all strings to lowercase.

**Returns:** `Series<String>` - Lowercase strings

---

#### `series.str_trim()`

Removes leading and trailing whitespace.

**Returns:** `Series<String>` - Trimmed strings

---

#### `series.str_replace(pattern, replacement)`

Replaces occurrences of a pattern.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String` | Pattern to find |
| `replacement` | `String` | Replacement text |

**Returns:** `Series<String>` - Series with replacements made

---

#### `series.str_substring(start, length?)`

Extracts substrings.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `start` | `Int` | Starting index |
| `length` | `Int?` | Length (default: rest of string) |

**Returns:** `Series<String>` - Extracted substrings

---

## Pipeline Examples

DataFrames integrate naturally with Stratum's pipeline operator:

```stratum
// Load, transform, and analyze
let result = Data.read_csv("sales.csv")
    |> .filter(|row| row.year == 2024)
    |> .select("region", "product", "revenue")
    |> .group_by("region")
    |> .aggregate(
        Agg.sum("revenue", "total_revenue"),
        Agg.count("num_sales")
    )
    |> .sort_by("-total_revenue")
    |> .head(10)

result.to_csv("top_regions.csv")
```

```stratum
// Join multiple data sources
let users = Data.read_parquet("users.parquet")
let orders = Data.read_parquet("orders.parquet")
let products = Data.read_parquet("products.parquet")

let report = orders
    |> .join(users, Join.left_cols("user_id", "id"))
    |> .join(products, Join.left_cols("product_id", "id"))
    |> .select("user_name", "product_name", "quantity", "total")
    |> .sort_by("-total")
```

---

## See Also

- [Agg](agg.md) - Aggregation specifications for group_by
- [Join](join.md) - Join specifications for combining DataFrames
- [Cube](cube.md) - OLAP cube operations for multi-dimensional analysis
- [Json](json.md) - JSON encoding/decoding
- [File](file.md) - General file operations
