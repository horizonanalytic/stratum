# Db

Database connectivity for SQLite, PostgreSQL, MySQL, and DuckDB.

## Overview

The `Db` namespace provides a unified interface for connecting to and querying relational databases. Stratum supports four database backends:

- **SQLite** - Embedded file-based or in-memory database
- **PostgreSQL** - Full-featured relational database
- **MySQL** - Popular open-source relational database
- **DuckDB** - Embedded analytical database (OLAP-optimized)

All database operations use parameterized queries to prevent SQL injection.

```stratum
// Quick example
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
db.execute("INSERT INTO users (name) VALUES (?)", ["Alice"])
let users = db.query("SELECT * FROM users")
println(users)  // [{"id": 1, "name": "Alice"}]
```

---

## Connection Factory Functions

### `Db.sqlite(path)`

Connects to a SQLite database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to database file, or `":memory:"` for in-memory |

**Returns:** `DbConnection` - A database connection

**Throws:** Error if database cannot be opened

**Example:**

```stratum
// In-memory database (lost when program ends)
let db = Db.sqlite(":memory:")

// File-based database (persists to disk)
let db = Db.sqlite("myapp.db")
let db = Db.sqlite("/path/to/database.sqlite")
```

---

### `Db.postgres(url)`

Connects to a PostgreSQL database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | PostgreSQL connection URL |

**Returns:** `DbConnection` - A database connection

**Throws:** Error if connection fails

**Connection URL format:**
```
postgres://user:password@host:port/database
```

**Example:**

```stratum
let db = Db.postgres("postgres://admin:secret@localhost:5432/myapp")

// With options
let db = Db.postgres("postgres://user:pass@db.example.com/prod?sslmode=require")
```

---

### `Db.mysql(url)`

Connects to a MySQL database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` | MySQL connection URL |

**Returns:** `DbConnection` - A database connection

**Throws:** Error if connection fails

**Connection URL format:**
```
mysql://user:password@host:port/database
```

**Example:**

```stratum
let db = Db.mysql("mysql://root:password@localhost:3306/myapp")

// Remote database
let db = Db.mysql("mysql://user:pass@mysql.example.com/production")
```

---

### `Db.duckdb(path)`

Connects to a DuckDB database. DuckDB is optimized for analytical queries (OLAP) and works well with large datasets.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `path` | `String` | Path to database file, or `":memory:"` for in-memory |

**Returns:** `DbConnection` - A database connection

**Throws:** Error if database cannot be opened

**Example:**

```stratum
// In-memory DuckDB
let db = Db.duckdb(":memory:")

// File-based DuckDB
let db = Db.duckdb("analytics.duckdb")

// DuckDB excels at analytical queries
db.execute("CREATE TABLE sales AS SELECT * FROM read_csv('sales.csv')")
let result = db.query("SELECT region, SUM(amount) FROM sales GROUP BY region")
```

---

## Query Methods

### `connection.query(sql, params?)`

Executes a SQL query and returns results as a list of maps.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `sql` | `String` | SQL query with `?` placeholders |
| `params` | `List?` | Parameter values for placeholders |

**Returns:** `List<Map>` - Each row as a map with column names as keys

**Throws:** Error if SQL syntax is invalid or query fails

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
db.execute("INSERT INTO users (name, age) VALUES ('Alice', 30), ('Bob', 25)")

// Query all rows
let users = db.query("SELECT * FROM users")
// [{"id": 1, "name": "Alice", "age": 30}, {"id": 2, "name": "Bob", "age": 25}]

// Query with parameters (prevents SQL injection)
let adults = db.query("SELECT name FROM users WHERE age >= ?", [18])
// [{"name": "Alice"}, {"name": "Bob"}]

// Multiple parameters
let result = db.query(
    "SELECT * FROM users WHERE age BETWEEN ? AND ?",
    [20, 35]
)
```

---

### `connection.execute(sql, params?)`

Executes a SQL statement and returns the number of affected rows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `sql` | `String` | SQL statement with `?` placeholders |
| `params` | `List?` | Parameter values for placeholders |

**Returns:** `Int` - Number of rows affected

**Throws:** Error if SQL syntax is invalid or execution fails

**Example:**

```stratum
let db = Db.sqlite(":memory:")

// Create table (returns 0)
db.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")

// Insert row (returns 1)
let inserted = db.execute("INSERT INTO items (name) VALUES (?)", ["Widget"])
println(inserted)  // 1

// Update rows (returns count of updated rows)
let updated = db.execute("UPDATE items SET name = ? WHERE id = ?", ["Gadget", 1])
println(updated)  // 1

// Delete rows
let deleted = db.execute("DELETE FROM items WHERE id = ?", [1])
println(deleted)  // 1
```

---

### `connection.close()`

Closes the database connection. Connections are automatically closed when they go out of scope, but you can close explicitly for resource management.

**Returns:** `Null`

**Example:**

```stratum
let db = Db.sqlite("temp.db")
// ... use database ...
db.close()  // Explicit close
```

---

## Transaction Methods

### `connection.begin()`

Begins a database transaction. Changes are not visible to other connections until committed.

**Returns:** `Null`

**Throws:** Error if transaction cannot be started

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE accounts (id INTEGER, balance INTEGER)")
db.execute("INSERT INTO accounts VALUES (1, 100), (2, 50)")

db.begin()
try {
    db.execute("UPDATE accounts SET balance = balance - 30 WHERE id = 1")
    db.execute("UPDATE accounts SET balance = balance + 30 WHERE id = 2")
    db.commit()
    println("Transfer complete")
} catch e {
    db.rollback()
    println("Transfer failed: " + e)
}
```

---

### `connection.commit()`

Commits the current transaction, making all changes permanent.

**Returns:** `Null`

**Throws:** Error if no transaction is active or commit fails

---

### `connection.rollback()`

Rolls back the current transaction, discarding all changes since `begin()`.

**Returns:** `Null`

**Throws:** Error if no transaction is active

---

## Metadata Methods

### `connection.tables()`

Lists all tables in the database.

**Returns:** `List<String>` - Table names

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE users (id INTEGER)")
db.execute("CREATE TABLE posts (id INTEGER)")

let tables = db.tables()
println(tables)  // ["users", "posts"]
```

---

### `connection.columns(table)`

Gets column information for a table.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `table` | `String` | Table name |

**Returns:** `List<Map>` - Column metadata with keys: `name`, `type`, `nullable`, `primary_key`

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")

let cols = db.columns("users")
// [
//   {"name": "id", "type": "INTEGER", "nullable": true, "primary_key": true},
//   {"name": "name", "type": "TEXT", "nullable": false, "primary_key": false}
// ]
```

---

### `connection.table_exists(table)`

Checks if a table exists in the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `table` | `String` | Table name |

**Returns:** `Bool` - True if table exists

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE users (id INTEGER)")

db.table_exists("users")   // true
db.table_exists("orders")  // false
```

---

### `connection.version`

Gets the database version string.

**Returns:** `String` - Version information

**Example:**

```stratum
let db = Db.sqlite(":memory:")
println(db.version)  // "SQLite 3.x.x"

let pg = Db.postgres("postgres://localhost/test")
println(pg.version)  // "PostgreSQL 15.x ..."
```

---

### `connection.db_type`

Gets the database type as a string.

**Returns:** `String` - One of: `"sqlite"`, `"postgres"`, `"mysql"`, `"duckdb"`

**Example:**

```stratum
let db = Db.sqlite(":memory:")
println(db.db_type)  // "sqlite"
```

---

## Working with DataFrames

### `Data.from_query(connection, sql, params?)`

Execute a database query and return the results as a DataFrame. This bridges the `Db` and `Data` namespaces for analytical workflows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `connection` | `DbConnection` | Database connection |
| `sql` | `String` | SQL query |
| `params` | `List?` | Query parameters |

**Returns:** `DataFrame` - Query results as a DataFrame

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE sales (product TEXT, revenue FLOAT)")
db.execute("INSERT INTO sales VALUES ('A', 100.0), ('B', 200.0), ('A', 150.0)")

// Query directly into a DataFrame
let df = Data.from_query(db, "SELECT * FROM sales")

// Now use DataFrame operations
let summary = df
    |> group_by("product")
    |> agg(Agg.sum("revenue").alias("total"))
    |> sort_by("total", "desc")

println(summary)
// product | total
// B       | 200.0
// A       | 250.0
```

---

## Parameter Types

Query parameters support the following Stratum types:

| Stratum Type | SQL Type |
|--------------|----------|
| `Null` | NULL |
| `Bool` | BOOLEAN |
| `Int` | BIGINT/INTEGER |
| `Float` | DOUBLE/REAL |
| `String` | TEXT/VARCHAR |

**Example:**

```stratum
let db = Db.sqlite(":memory:")
db.execute("CREATE TABLE data (flag BOOLEAN, count INTEGER, value REAL, name TEXT)")

// All parameter types in one query
db.execute(
    "INSERT INTO data VALUES (?, ?, ?, ?)",
    [true, 42, 3.14, "hello"]
)

// Null parameters
db.execute(
    "INSERT INTO data VALUES (?, ?, ?, ?)",
    [null, null, null, null]
)
```

---

## Examples

### Basic CRUD Operations

```stratum
let db = Db.sqlite("tasks.db")

// Create table if needed
if !db.table_exists("tasks") {
    db.execute("CREATE TABLE tasks (id INTEGER PRIMARY KEY, title TEXT, done BOOLEAN)")
}

// Create
db.execute("INSERT INTO tasks (title, done) VALUES (?, ?)", ["Buy groceries", false])

// Read
let tasks = db.query("SELECT * FROM tasks WHERE done = ?", [false])
for task in tasks {
    println(task["title"])
}

// Update
db.execute("UPDATE tasks SET done = ? WHERE id = ?", [true, 1])

// Delete
db.execute("DELETE FROM tasks WHERE done = ?", [true])
```

### Transaction Example

```stratum
let db = Db.postgres("postgres://localhost/bank")

fx transfer(from_id: Int, to_id: Int, amount: Float) {
    db.begin()
    try {
        // Check sufficient balance
        let result = db.query(
            "SELECT balance FROM accounts WHERE id = ?",
            [from_id]
        )
        if result[0]["balance"] < amount {
            throw "Insufficient funds"
        }

        // Perform transfer
        db.execute(
            "UPDATE accounts SET balance = balance - ? WHERE id = ?",
            [amount, from_id]
        )
        db.execute(
            "UPDATE accounts SET balance = balance + ? WHERE id = ?",
            [amount, to_id]
        )

        db.commit()
    } catch e {
        db.rollback()
        throw e
    }
}

transfer(1, 2, 100.0)
```

### Using DuckDB for Analytics

```stratum
let db = Db.duckdb(":memory:")

// DuckDB can read files directly
db.execute("CREATE TABLE sales AS SELECT * FROM read_csv('sales.csv')")

// Analytical query
let report = db.query("
    SELECT
        region,
        product,
        SUM(amount) as total_sales,
        AVG(amount) as avg_sale,
        COUNT(*) as num_sales
    FROM sales
    WHERE date >= '2024-01-01'
    GROUP BY region, product
    ORDER BY total_sales DESC
    LIMIT 10
")

for row in report {
    println("${row['region']}: ${row['product']} = ${row['total_sales']}")
}
```

---

## Error Handling

Database operations throw errors on failure:

```stratum
try {
    let db = Db.sqlite("/nonexistent/path/db.sqlite")
} catch e {
    println("Failed to open database: " + e)
}

try {
    let result = db.query("INVALID SQL SYNTAX")
} catch e {
    println("Query failed: " + e)
}

try {
    // Parameter count mismatch
    db.execute("INSERT INTO t VALUES (?, ?)", [1])  // Missing second param
} catch e {
    println("Parameter error: " + e)
}
```

---

## See Also

- [Data](data.md) - DataFrame operations with `Data.from_query()`
- [Json](json.md) - Encoding query results to JSON
- [File](file.md) - Reading/writing database files
