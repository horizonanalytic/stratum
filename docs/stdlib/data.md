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
series.mode()     // Most frequent value
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

#### `series.mode()`

Returns the most frequent value in the series. For ties, returns the first value encountered.

**Returns:** `Value` - The most frequent value, or `null` if empty

**Example:**

```stratum
let grades = Data.series("grade", ["A", "B", "A", "C", "A", "B"])
println(grades.mode())  // "A"
```

---

#### `series.quantile(q)`

Returns the value at the given quantile.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `q` | `Float` | Quantile between 0.0 and 1.0 |

**Returns:** `Float` - The value at the specified quantile

**Example:**

```stratum
let values = Data.series("x", [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
println(values.quantile(0.25))  // 3.25 (25th percentile)
println(values.quantile(0.5))   // 5.5 (median)
println(values.quantile(0.75))  // 7.75 (75th percentile)
```

---

#### `series.percentile(p)`

Returns the value at the given percentile.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `p` | `Float` | Percentile between 0 and 100 |

**Returns:** `Float` - The value at the specified percentile

**Example:**

```stratum
let values = Data.series("x", [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
println(values.percentile(25))  // 3.25
println(values.percentile(50))  // 5.5
println(values.percentile(90))  // 9.1
```

---

#### `series.skew()`

Calculates the skewness of numeric values. Skewness measures the asymmetry of the distribution.

**Returns:** `Float` - Skewness value (positive = right tail, negative = left tail, zero = symmetric)

**Example:**

```stratum
let values = Data.series("x", [1, 2, 2, 3, 3, 3, 4, 4, 5])
println(values.skew())  // Measures distribution asymmetry
```

---

#### `series.kurtosis()`

Calculates the kurtosis of numeric values. Kurtosis measures the "tailedness" of the distribution. Returns excess kurtosis (normal distribution = 0).

**Returns:** `Float` - Excess kurtosis value

**Example:**

```stratum
let values = Data.series("x", [1, 2, 3, 4, 5, 6, 7, 8, 9])
println(values.kurtosis())  // Measures tail heaviness
```

---

### Window Functions

Window functions compute values over a sliding window or cumulative range.

#### Rolling Windows

##### `series.rolling(window_size)`

Creates a rolling window object for computing rolling statistics.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `window_size` | `Int` | Number of elements in the window |

**Returns:** `Rolling` - Rolling window object

**Example:**

```stratum
let prices = Data.series("price", [10, 20, 30, 40, 50])

// Rolling 3-period mean
let rolling_avg = prices.rolling(3).mean()
// [null, null, 20.0, 30.0, 40.0]

// Rolling 3-period sum
let rolling_sum = prices.rolling(3).sum()
// [null, null, 60.0, 90.0, 120.0]
```

##### Rolling Methods

After calling `.rolling(n)`, you can call these aggregation methods:

```stratum
rolling.sum()     // Rolling sum
rolling.mean()    // Rolling average
rolling.min()     // Rolling minimum
rolling.max()     // Rolling maximum
rolling.std()     // Rolling standard deviation
rolling.var()     // Rolling variance
```

---

#### Cumulative Operations

##### `series.cumsum()`

Computes the cumulative sum.

**Returns:** `Series` - Cumulative sum at each position

**Example:**

```stratum
let values = Data.series("x", [1, 2, 3, 4, 5])
let cumulative = values.cumsum()
// [1, 3, 6, 10, 15]
```

---

##### `series.cummax()`

Computes the cumulative maximum.

**Returns:** `Series` - Running maximum at each position

---

##### `series.cummin()`

Computes the cumulative minimum.

**Returns:** `Series` - Running minimum at each position

---

##### `series.cumprod()`

Computes the cumulative product.

**Returns:** `Series` - Running product at each position

---

#### Lag/Lead Operations

##### `series.shift(n)` / `series.lag(n)`

Shifts values by n positions. Positive n shifts forward (introduces nulls at start).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | Number of positions to shift |

**Returns:** `Series` - Shifted series with nulls filling gaps

**Example:**

```stratum
let values = Data.series("x", [1, 2, 3, 4, 5])
let shifted = values.shift(1)
// [null, 1, 2, 3, 4]

let lagged = values.lag(2)
// [null, null, 1, 2, 3]
```

---

##### `series.lead(n)`

Shifts values backward by n positions (opposite of lag).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | Number of positions to lead |

**Returns:** `Series` - Series with values shifted backward

**Example:**

```stratum
let values = Data.series("x", [1, 2, 3, 4, 5])
let lead_values = values.lead(1)
// [2, 3, 4, 5, null]
```

---

##### `series.diff(n?)`

Computes the difference from n periods ago.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int?` | Number of periods (default: 1) |

**Returns:** `Series` - Differences from n periods ago

**Example:**

```stratum
let values = Data.series("x", [10, 15, 13, 20, 18])
let changes = values.diff()
// [null, 5, -2, 7, -2]
```

---

##### `series.pct_change(n?)`

Computes the percentage change from n periods ago.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int?` | Number of periods (default: 1) |

**Returns:** `Series` - Percentage changes as decimals

**Example:**

```stratum
let prices = Data.series("price", [100, 110, 105, 120])
let returns = prices.pct_change()
// [null, 0.1, -0.0454..., 0.1428...]
```

---

### Missing Data Handling

#### `series.dropna()`

Removes null values from the series.

**Returns:** `Series` - Series without null values

**Example:**

```stratum
let values = Data.series("x", [1, null, 3, null, 5])
let clean = values.dropna()
// [1, 3, 5]
```

---

#### `series.fillna(value)`

Fills null values with a constant or using a method.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Value \| String` | Fill value or method ("forward", "backward") |

**Returns:** `Series` - Series with nulls filled

**Example:**

```stratum
let values = Data.series("x", [1, null, null, 4, null])

// Fill with constant
values.fillna(0)  // [1, 0, 0, 4, 0]

// Forward fill (last known value)
values.fillna("forward")  // [1, 1, 1, 4, 4]

// Backward fill (next known value)
values.fillna("backward")  // [1, 4, 4, 4, null]
```

---

#### `series.interpolate()`

Linearly interpolates missing values.

**Returns:** `Series` - Series with interpolated values

**Example:**

```stratum
let values = Data.series("x", [1.0, null, null, 4.0, null])
let interpolated = values.interpolate()
// [1.0, 2.0, 3.0, 4.0, null]
```

---

### Type Conversion

#### `series.to_int()`

Converts series values to integers.

**Returns:** `Series<Int>` - Integer series

**Aliases:** `to_integer()`, `as_int()`

---

#### `series.to_float()`

Converts series values to floats.

**Returns:** `Series<Float>` - Float series

**Aliases:** `to_double()`, `as_float()`

---

#### `series.to_string()`

Converts series values to strings.

**Returns:** `Series<String>` - String series

**Aliases:** `to_str()`, `as_string()`

---

#### `series.to_datetime(format)`

Parses string values to datetime using the specified format.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `format` | `String` | Date format string (e.g., "%Y-%m-%d") |

**Returns:** `Series<DateTime>` - DateTime series

**Example:**

```stratum
let dates = Data.series("date", ["2024-01-15", "2024-02-20", "2024-03-25"])
let parsed = dates.to_datetime("%Y-%m-%d")
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

#### `series.str_pad(width, side, char)`

Pads strings to a specified width.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `width` | `Int` | Target width |
| `side` | `String` | "left", "right", or "both" |
| `char` | `String` | Padding character (single char) |

**Returns:** `Series<String>` - Padded strings

**Example:**

```stratum
let codes = Data.series("code", ["1", "12", "123"])
codes.str_pad(5, "left", "0")  // ["00001", "00012", "00123"]
```

---

#### `series.str_extract(pattern)`

Extracts the first regex capture group from each string.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String` | Regex pattern with capture group |

**Returns:** `Series<String>` - Extracted matches

**Example:**

```stratum
let emails = Data.series("email", ["user@domain.com", "admin@site.org"])
let domains = emails.str_extract("@(.+)")
// ["domain.com", "site.org"]
```

---

#### `series.str_match(pattern)`

Tests if each string matches a regex pattern.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pattern` | `String` | Regex pattern |

**Returns:** `Series<Bool>` - True where pattern matches

**Example:**

```stratum
let texts = Data.series("text", ["hello123", "world", "test456"])
texts.str_match("\\d+")  // [true, false, true]
```

---

#### `series.str_cat(other, separator)`

Concatenates two string series element-wise.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `Series` | Other string series |
| `separator` | `String` | Separator between values |

**Returns:** `Series<String>` - Concatenated strings

**Example:**

```stratum
let first = Data.series("first", ["John", "Jane"])
let last = Data.series("last", ["Doe", "Smith"])
first.str_cat(last, " ")  // ["John Doe", "Jane Smith"]
```

---

#### `series.str_slice(start, end?)`

Slices each string from start to end index.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `start` | `Int` | Start index |
| `end` | `Int?` | End index (exclusive, default: end of string) |

**Returns:** `Series<String>` - Sliced strings

**Example:**

```stratum
let codes = Data.series("code", ["ABCD1234", "EFGH5678"])
codes.str_slice(0, 4)  // ["ABCD", "EFGH"]
```

---

## DataFrame Statistical Methods

### `df.describe()`

Generates summary statistics for all numeric columns. Returns a DataFrame with rows for count, mean, std, min, 25%, 50%, 75%, and max.

**Returns:** `DataFrame` - Summary statistics table

**Example:**

```stratum
let df = Data.frame([
    {age: 25, salary: 50000},
    {age: 30, salary: 60000},
    {age: 35, salary: 70000},
    {age: 40, salary: 80000}
])

let stats = df.describe()
// Returns DataFrame with statistics for each numeric column
```

---

### `df.corr()`

Computes the correlation matrix between all numeric columns.

**Returns:** `DataFrame` - Correlation matrix

**Aliases:** `correlation()`

**Example:**

```stratum
let df = Data.frame([
    {height: 170, weight: 70},
    {height: 175, weight: 75},
    {height: 180, weight: 80}
])

let correlations = df.corr()
// Shows correlation between height and weight
```

---

### `df.cov()`

Computes the covariance matrix between all numeric columns.

**Returns:** `DataFrame` - Covariance matrix

**Aliases:** `covariance()`

---

### `df.value_counts(column)`

Counts occurrences of each unique value in a column.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to count values in |

**Returns:** `DataFrame` - Value counts with columns: value, count

**Example:**

```stratum
let df = Data.frame([
    {city: "NYC", status: "active"},
    {city: "LA", status: "active"},
    {city: "NYC", status: "inactive"},
    {city: "NYC", status: "active"}
])

df.value_counts("city")
// {value: ["NYC", "LA"], count: [3, 1]}

df.value_counts("status")
// {value: ["active", "inactive"], count: [3, 1]}
```

---

## DataFrame Missing Data

### `df.dropna(columns...)`

Drops rows containing null values.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...?` | Optional specific columns to check |

**Returns:** `DataFrame` - DataFrame without rows containing nulls

**Example:**

```stratum
let df = Data.frame([
    {a: 1, b: 2},
    {a: null, b: 3},
    {a: 4, b: null}
])

// Drop rows with any null
df.dropna()  // Only keeps {a: 1, b: 2}

// Drop rows with null in specific column
df.dropna("a")  // Keeps rows 1 and 3
```

---

### `df.fillna(value)`

Fills null values in the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `value` | `Value \| Map \| String` | Fill value, column map, or method |

**Returns:** `DataFrame` - DataFrame with nulls filled

**Example:**

```stratum
let df = Data.frame([
    {a: 1, b: null},
    {a: null, b: 3}
])

// Fill all nulls with 0
df.fillna(0)

// Fill with column-specific values
df.fillna({a: -1, b: -2})

// Forward fill
df.fillna("forward")

// Backward fill
df.fillna("backward")
```

---

## DataFrame Reshape Operations

### `df.transpose()` / `df.T`

Transposes rows and columns.

**Returns:** `DataFrame` - Transposed DataFrame

**Example:**

```stratum
let df = Data.frame([
    {a: 1, b: 2, c: 3}
])
df.transpose()
// Converts columns to rows
```

---

### `df.pivot(index, columns, values)`

Creates a pivot table from long to wide format.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `index` | `String` | Column to use as row index |
| `columns` | `String` | Column whose values become new columns |
| `values` | `String` | Column to use for cell values |

**Returns:** `DataFrame` - Pivoted DataFrame

**Example:**

```stratum
let df = Data.frame([
    {date: "2024-01", product: "A", sales: 100},
    {date: "2024-01", product: "B", sales: 150},
    {date: "2024-02", product: "A", sales: 120},
    {date: "2024-02", product: "B", sales: 180}
])

df.pivot("date", "product", "sales")
// Returns:
// date      | A   | B
// 2024-01   | 100 | 150
// 2024-02   | 120 | 180
```

---

### `df.pivot_table(index, columns, values, aggfunc)`

Creates a pivot table with aggregation.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `index` | `String` | Column for row index |
| `columns` | `String` | Column for new columns |
| `values` | `String` | Column to aggregate |
| `aggfunc` | `String` | Aggregation function: "sum", "mean", "count", etc. |

**Returns:** `DataFrame` - Aggregated pivot table

**Example:**

```stratum
let sales = Data.frame([
    {region: "East", product: "A", amount: 100},
    {region: "East", product: "A", amount: 150},
    {region: "West", product: "B", amount: 200}
])

sales.pivot_table("region", "product", "amount", "sum")
// Sums amounts for each region-product combination
```

---

### `df.melt(id_vars...)`

Unpivots a DataFrame from wide to long format.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id_vars` | `String...` | Columns to keep as identifiers |

**Returns:** `DataFrame` - Melted DataFrame with "variable" and "value" columns

**Example:**

```stratum
let df = Data.frame([
    {id: 1, jan: 100, feb: 110, mar: 120},
    {id: 2, jan: 200, feb: 210, mar: 220}
])

df.melt("id")
// Returns:
// id | variable | value
// 1  | jan      | 100
// 1  | feb      | 110
// 1  | mar      | 120
// 2  | jan      | 200
// ...
```

---

### `df.stack(columns...)`

Stacks specified columns into rows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `columns` | `String...` | Columns to stack |

**Returns:** `DataFrame` - Stacked DataFrame

---

### `df.unstack(index_col, column_col, value_col)`

Unstacks rows into columns (opposite of stack).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `index_col` | `String` | Column for row identifiers |
| `column_col` | `String` | Column whose values become columns |
| `value_col` | `String` | Column for cell values |

**Returns:** `DataFrame` - Unstacked DataFrame

---

### `df.explode(column)`

Explodes a list column into multiple rows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column containing lists to explode |

**Returns:** `DataFrame` - DataFrame with exploded rows

**Example:**

```stratum
let df = Data.frame([
    {id: 1, tags: ["a", "b", "c"]},
    {id: 2, tags: ["x", "y"]}
])

df.explode("tags")
// Returns:
// id | tags
// 1  | a
// 1  | b
// 1  | c
// 2  | x
// 2  | y
```

---

## DataFrame Advanced Operations

### Column Operations

#### `df.add_column(name, values_or_closure)`

Adds a new column to the DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` | Name for the new column |
| `values_or_closure` | `List \| Series \| Function` | Values or row transformer |

**Returns:** `DataFrame` - DataFrame with new column

**Example:**

```stratum
let df = Data.frame([
    {a: 1, b: 2},
    {a: 3, b: 4}
])

// Add from list
df.add_column("c", [10, 20])

// Add computed column
df.add_column("sum", |row| row.a + row.b)
// {a: 1, b: 2, sum: 3}, {a: 3, b: 4, sum: 7}
```

---

#### `df.apply(closure)`

Applies a function to each row, returning results as a list.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `closure` | `Function` | Function taking a row and returning a value |

**Returns:** `List` - List of closure results

**Example:**

```stratum
let df = Data.frame([
    {name: "Alice", age: 30},
    {name: "Bob", age: 25}
])

let descriptions = df.apply(|row| {
    row.name + " is " + str(row.age) + " years old"
})
// ["Alice is 30 years old", "Bob is 25 years old"]
```

---

#### `df.transform(column, closure)`

Transforms a single column using a function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to transform |
| `closure` | `Function` | Function to apply to each value |

**Returns:** `DataFrame` - DataFrame with transformed column

**Example:**

```stratum
let df = Data.frame([
    {name: "alice", age: 30},
    {name: "bob", age: 25}
])

df.transform("name", |n| n.to_upper())
// {name: "ALICE", age: 30}, {name: "BOB", age: 25}
```

---

#### `df.cast(column, target_type)`

Casts a column to a different type.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to cast |
| `target_type` | `String` | Target type: "int", "float", "string", "bool" |

**Returns:** `DataFrame` - DataFrame with cast column

**Example:**

```stratum
let df = Data.frame([{value: "123"}, {value: "456"}])
df.cast("value", "int")
```

---

### Concatenation

#### `Data.concat(dfs...)`

Concatenates multiple DataFrames vertically.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dfs` | `DataFrame...` | DataFrames to concatenate |

**Returns:** `DataFrame` - Combined DataFrame

**Example:**

```stratum
let df1 = Data.frame([{a: 1}, {a: 2}])
let df2 = Data.frame([{a: 3}, {a: 4}])

let combined = Data.concat(df1, df2)
// {a: 1}, {a: 2}, {a: 3}, {a: 4}
```

---

#### `df.append(other)`

Appends rows from another DataFrame.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `DataFrame` | DataFrame to append |

**Returns:** `DataFrame` - Combined DataFrame

**Example:**

```stratum
let df1 = Data.frame([{a: 1}])
let df2 = Data.frame([{a: 2}])

df1.append(df2)  // {a: 1}, {a: 2}
```

---

### Merge Operations

#### `df.merge(other, on, how, left_suffix?, right_suffix?)`

SQL-style merge/join with suffix handling for duplicate columns.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `DataFrame` | DataFrame to merge with |
| `on` | `String \| List` | Column(s) to join on |
| `how` | `String` | Join type: "inner", "left", "right", "outer" |
| `left_suffix` | `String?` | Suffix for left duplicates (default: "_x") |
| `right_suffix` | `String?` | Suffix for right duplicates (default: "_y") |

**Returns:** `DataFrame` - Merged DataFrame

**Example:**

```stratum
let users = Data.frame([
    {id: 1, name: "Alice"},
    {id: 2, name: "Bob"}
])

let orders = Data.frame([
    {user_id: 1, amount: 100},
    {user_id: 1, amount: 200}
])

users.merge(orders, "id", "left")
// Joins on users.id = orders.user_id
```

---

#### `df.cross_join(other)`

Creates the Cartesian product of two DataFrames.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `DataFrame` | DataFrame for cross join |

**Returns:** `DataFrame` - Cartesian product

**Example:**

```stratum
let sizes = Data.frame([{size: "S"}, {size: "M"}, {size: "L"}])
let colors = Data.frame([{color: "Red"}, {color: "Blue"}])

sizes.cross_join(colors)
// 6 rows: all combinations of size and color
```

---

### Index Operations

#### `df.reset_index()`

Resets the DataFrame index to default sequential integers.

**Returns:** `DataFrame` - DataFrame with reset index

---

#### `df.set_index(column)`

Sets a column as the DataFrame index.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column to use as index |

**Returns:** `DataFrame` - DataFrame with new index

**Example:**

```stratum
let df = Data.frame([
    {id: "A", value: 1},
    {id: "B", value: 2}
])

df.set_index("id")
// Uses "id" as the row index
```

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
