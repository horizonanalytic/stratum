# Join

Builder functions for creating join specifications used with DataFrame `join()`.

## Overview

The `Join` namespace provides a fluent API for specifying how two DataFrames should be joined. Each function returns a `JoinSpec` that describes the join type and which columns to match on.

There are two patterns:
- **Same column name**: Use `Join.on()`, `Join.inner()`, `Join.left()`, etc. when both DataFrames have the same column name to join on
- **Different column names**: Use `Join.cols()`, `Join.inner_cols()`, `Join.left_cols()`, etc. when the join columns have different names

The default join type is INNER.

---

## Same Column Name Functions

### `Join.on(column)`

Creates an INNER join specification on a column that exists in both DataFrames.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column name present in both DataFrames |

**Returns:** `JoinSpec` - Inner join specification

**Example:**

```stratum
let users = Data.frame([
    {user_id: 1, name: "Alice"},
    {user_id: 2, name: "Bob"}
])

let orders = Data.frame([
    {user_id: 1, product: "Widget"},
    {user_id: 1, product: "Gadget"},
    {user_id: 3, product: "Thing"}
])

let joined = users.join(orders, Join.on("user_id"))
// Result: Only user_id 1 (Alice) with her 2 orders
```

---

### `Join.inner(column)`

Creates an INNER join specification. Only rows with matching values in both DataFrames are included.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column name present in both DataFrames |

**Returns:** `JoinSpec` - Inner join specification

**Example:**

```stratum
let result = users.join(orders, Join.inner("user_id"))
// Equivalent to Join.on("user_id")
```

---

### `Join.left(column)`

Creates a LEFT join specification. All rows from the left DataFrame are included, with nulls for non-matching right rows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column name present in both DataFrames |

**Returns:** `JoinSpec` - Left join specification

**Example:**

```stratum
let result = users.join(orders, Join.left("user_id"))
// Result: All users, with orders where they exist, null otherwise
// Alice appears with her orders
// Bob appears with null order fields
```

---

### `Join.right(column)`

Creates a RIGHT join specification. All rows from the right DataFrame are included, with nulls for non-matching left rows.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column name present in both DataFrames |

**Returns:** `JoinSpec` - Right join specification

**Example:**

```stratum
let result = users.join(orders, Join.right("user_id"))
// Result: All orders, with user info where it exists
// Alice's orders appear with her name
// user_id 3's order appears with null name
```

---

### `Join.outer(column)`

Creates a FULL OUTER join specification. All rows from both DataFrames are included, with nulls where there's no match.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `column` | `String` | Column name present in both DataFrames |

**Returns:** `JoinSpec` - Outer join specification

**Example:**

```stratum
let result = users.join(orders, Join.outer("user_id"))
// Result: All users and all orders
// Matching rows are combined
// Non-matching rows have nulls for the other side
```

---

## Different Column Names Functions

Use these when the join columns have different names in each DataFrame.

### `Join.cols(left_col, right_col)`

Creates an INNER join specification on columns with different names.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `left_col` | `String` | Column name in the left DataFrame |
| `right_col` | `String` | Column name in the right DataFrame |

**Returns:** `JoinSpec` - Inner join specification

**Example:**

```stratum
let users = Data.frame([
    {id: 1, name: "Alice"},
    {id: 2, name: "Bob"}
])

let orders = Data.frame([
    {customer_id: 1, product: "Widget"},
    {customer_id: 1, product: "Gadget"}
])

let joined = users.join(orders, Join.cols("id", "customer_id"))
```

---

### `Join.inner_cols(left_col, right_col)`

Creates an INNER join on columns with different names.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `left_col` | `String` | Column name in the left DataFrame |
| `right_col` | `String` | Column name in the right DataFrame |

**Returns:** `JoinSpec` - Inner join specification

---

### `Join.left_cols(left_col, right_col)`

Creates a LEFT join on columns with different names.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `left_col` | `String` | Column name in the left DataFrame |
| `right_col` | `String` | Column name in the right DataFrame |

**Returns:** `JoinSpec` - Left join specification

**Example:**

```stratum
let result = users.join(orders, Join.left_cols("id", "customer_id"))
// All users included, orders matched by id = customer_id
```

---

### `Join.right_cols(left_col, right_col)`

Creates a RIGHT join on columns with different names.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `left_col` | `String` | Column name in the left DataFrame |
| `right_col` | `String` | Column name in the right DataFrame |

**Returns:** `JoinSpec` - Right join specification

---

### `Join.outer_cols(left_col, right_col)`

Creates a FULL OUTER join on columns with different names.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `left_col` | `String` | Column name in the left DataFrame |
| `right_col` | `String` | Column name in the right DataFrame |

**Returns:** `JoinSpec` - Outer join specification

---

## Join Type Reference

| Join Type | Description | Null Behavior |
|-----------|-------------|---------------|
| INNER | Only matching rows | No nulls from join |
| LEFT | All left rows + matching right | Right columns null when no match |
| RIGHT | All right rows + matching left | Left columns null when no match |
| OUTER | All rows from both | Either side can have nulls |

---

## Examples

### Multi-Table Analysis

```stratum
let users = Data.read_csv("users.csv")
let orders = Data.read_csv("orders.csv")
let products = Data.read_csv("products.csv")

// Build a complete order report
let report = orders
    |> .join(users, Join.left_cols("user_id", "id"))
    |> .join(products, Join.left_cols("product_id", "id"))
    |> .select("user_name", "product_name", "quantity", "price")
    |> .sort_by("-quantity")
```

### Finding Unmatched Records

```stratum
// Find users who have never ordered
let all_users = users.join(orders, Join.left("user_id"))
let inactive = all_users.filter(|row| row.order_id == null)
```

### Combining Data Sources

```stratum
// Merge data from different systems
let crm_data = Data.read_csv("crm_customers.csv")
let billing_data = Data.read_csv("billing_accounts.csv")

let combined = crm_data.join(
    billing_data,
    Join.outer_cols("customer_email", "email")
)
```

---

## See Also

- [Data](data.md) - DataFrame operations including join
- [Agg](agg.md) - Aggregation specifications
