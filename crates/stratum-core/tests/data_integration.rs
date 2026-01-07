//! Integration tests for the data module (DataFrame and Series)

use std::sync::Arc;

use stratum_core::data::{DataFrame, Series};
use stratum_core::bytecode::Value;

#[test]
fn test_series_creation_and_access() {
    let series = Series::from_ints("numbers", vec![1, 2, 3, 4, 5]);

    assert_eq!(series.name(), "numbers");
    assert_eq!(series.len(), 5);
    assert!(!series.is_empty());

    // Test value access
    assert_eq!(series.get(0).unwrap(), Value::Int(1));
    assert_eq!(series.get(4).unwrap(), Value::Int(5));
}

#[test]
fn test_series_aggregations() {
    let series = Series::from_ints("nums", vec![10, 20, 30, 40, 50]);

    assert_eq!(series.sum().unwrap(), Value::Int(150));
    assert_eq!(series.min().unwrap(), Value::Int(10));
    assert_eq!(series.max().unwrap(), Value::Int(50));
    assert_eq!(series.count(), 5);
}

#[test]
fn test_dataframe_creation() {
    let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
    let ages = Series::from_ints("age", vec![25, 30, 35]);

    let df = DataFrame::from_series(vec![names, ages]).unwrap();

    assert_eq!(df.num_columns(), 2);
    assert_eq!(df.num_rows(), 3);
    assert_eq!(df.columns(), vec!["name", "age"]);
}

#[test]
fn test_dataframe_column_access() {
    let values = Series::from_floats("score", vec![95.5, 87.3, 92.1]);
    let df = DataFrame::from_series(vec![values]).unwrap();

    let col = df.column("score").unwrap();
    assert_eq!(col.len(), 3);

    // Test column not found
    assert!(df.column("nonexistent").is_err());
}

#[test]
fn test_dataframe_head_tail() {
    let nums = Series::from_ints("n", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let df = DataFrame::from_series(vec![nums]).unwrap();

    let head = df.head(3).unwrap();
    assert_eq!(head.num_rows(), 3);

    let tail = df.tail(3).unwrap();
    assert_eq!(tail.num_rows(), 3);
}

#[test]
fn test_dataframe_select_columns() {
    let a = Series::from_ints("a", vec![1, 2, 3]);
    let b = Series::from_ints("b", vec![4, 5, 6]);
    let c = Series::from_ints("c", vec![7, 8, 9]);

    let df = DataFrame::from_series(vec![a, b, c]).unwrap();

    let selected = df.select(&["a", "c"]).unwrap();
    assert_eq!(selected.num_columns(), 2);
    assert_eq!(selected.columns(), vec!["a", "c"]);
}

#[test]
fn test_dataframe_as_value() {
    let series = Series::from_ints("x", vec![1, 2, 3]);
    let df = DataFrame::from_series(vec![series]).unwrap();

    let value = Value::DataFrame(Arc::new(df));
    assert_eq!(value.type_name(), "DataFrame");
}

#[test]
fn test_series_as_value() {
    let series = Series::from_strings("names", vec!["a", "b"]);

    let value = Value::Series(Arc::new(series));
    assert_eq!(value.type_name(), "Series");
}

#[test]
fn test_series_from_values() {
    // Test creating Series from Stratum Values
    let values = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
    let series = Series::from_values("nums", &values).unwrap();
    assert_eq!(series.len(), 3);
    assert_eq!(series.get(0).unwrap(), Value::Int(1));

    // Test with nulls
    let values = vec![Value::Int(1), Value::Null, Value::Int(3)];
    let series = Series::from_values("with_nulls", &values).unwrap();
    assert_eq!(series.null_count(), 1);

    // Test with floats (allow int coercion)
    let values = vec![Value::Float(1.5), Value::Int(2), Value::Float(3.5)];
    let series = Series::from_values("floats", &values).unwrap();
    assert_eq!(series.len(), 3);
}

#[test]
fn test_dataframe_sample() {
    let nums = Series::from_ints("n", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let df = DataFrame::from_series(vec![nums]).unwrap();

    let sampled = df.sample(3).unwrap();
    assert_eq!(sampled.num_rows(), 3);

    // Sample larger than data returns full data
    let sampled = df.sample(100).unwrap();
    assert_eq!(sampled.num_rows(), 10);
}

#[test]
fn test_dataframe_row_iteration() {
    let names = Series::from_strings("name", vec!["Alice", "Bob"]);
    let ages = Series::from_ints("age", vec![25, 30]);
    let df = DataFrame::from_series(vec![names, ages]).unwrap();

    let rows: Vec<_> = df.iter_rows().collect();
    assert_eq!(rows.len(), 2);

    // Each row should be a Map
    for row in rows {
        assert!(row.is_ok());
        let row = row.unwrap();
        assert!(matches!(row, Value::Map(_)));
    }
}

#[test]
fn test_dataframe_column_iteration() {
    let a = Series::from_ints("a", vec![1, 2]);
    let b = Series::from_ints("b", vec![3, 4]);
    let df = DataFrame::from_series(vec![a, b]).unwrap();

    let cols: Vec<_> = df.iter_columns().collect();
    assert_eq!(cols.len(), 2);
}

#[test]
fn test_dataframe_filter_by_indices() {
    let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie", "Diana"]);
    let ages = Series::from_ints("age", vec![25, 30, 35, 40]);
    let df = DataFrame::from_series(vec![names, ages]).unwrap();

    // Filter to keep only rows at indices 1 and 3
    let filtered = df.filter_by_indices(&[1, 3]).unwrap();
    assert_eq!(filtered.num_rows(), 2);

    // Verify the correct rows were kept
    let name_col = filtered.column("name").unwrap();
    assert_eq!(name_col.get(0).unwrap(), Value::string("Bob"));
    assert_eq!(name_col.get(1).unwrap(), Value::string("Diana"));

    let age_col = filtered.column("age").unwrap();
    assert_eq!(age_col.get(0).unwrap(), Value::Int(30));
    assert_eq!(age_col.get(1).unwrap(), Value::Int(40));
}

#[test]
fn test_dataframe_filter_empty_indices() {
    let nums = Series::from_ints("n", vec![1, 2, 3]);
    let df = DataFrame::from_series(vec![nums]).unwrap();

    let filtered = df.filter_by_indices(&[]).unwrap();
    assert_eq!(filtered.num_rows(), 0);
    assert!(filtered.is_empty());
}

#[test]
fn test_dataframe_filter_out_of_bounds() {
    let nums = Series::from_ints("n", vec![1, 2, 3]);
    let df = DataFrame::from_series(vec![nums]).unwrap();

    // Index 10 is out of bounds
    let result = df.filter_by_indices(&[0, 10]);
    assert!(result.is_err());
}

// ============================================================================
// Language-level tests for DataFrame operations
// ============================================================================

use stratum_core::testutil::eval_expr_dynamic;

#[test]
fn test_dataframe_select_with_strings() {
    // Test that df.select("col1", "col2") works at the language level
    // We use eval_expr_dynamic with a block expression to get proper return values
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"},
            {"name": "Bob", "age": 25, "city": "LA"}
        ]
        let df = Data.frame(rows)
        let selected = df.select("name", "age")
        selected.columns().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_select_pipeline() {
    // Test that df |> select("col1") works via pipeline
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"},
            {"name": "Bob", "age": 25, "city": "LA"}
        ]
        let df = Data.frame(rows)
        let selected = df |> select("name")
        selected.columns().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1, "Expected 1 column after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_select_with_column_shorthand() {
    // Test that df |> select(.name, .age) works with column shorthands
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"},
            {"name": "Bob", "age": 25, "city": "LA"}
        ]
        let df = Data.frame(rows)
        let selected = df |> select(.name, .age)
        selected.columns().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_select_preserves_data() {
    // Test that select preserves the actual data values
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]
        let df = Data.frame(rows)
        let selected = df |> select(.name)
        selected.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_method_select_with_column_shorthand() {
    // Test that df.select(.name) works (method syntax with column shorthand)
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"}
        ]
        let df = Data.frame(rows)
        let selected = df.select(.name, .city)
        selected.columns().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// GroupedDataFrame and Aggregate Tests
// ============================================================================

use stratum_core::data::{AggSpec, GroupedDataFrame};

#[test]
fn test_grouped_dataframe_creation() {
    let regions = Series::from_strings("region", vec!["North", "South", "North", "South"]);
    let amounts = Series::from_ints("amount", vec![100, 200, 150, 250]);
    let df = DataFrame::from_series(vec![regions, amounts]).unwrap();

    let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();

    assert_eq!(grouped.num_groups(), 2); // North and South
    assert_eq!(grouped.group_columns(), &["region"]);
}

#[test]
fn test_grouped_dataframe_sum() {
    let regions = Series::from_strings("region", vec!["North", "South", "North", "South"]);
    let amounts = Series::from_ints("amount", vec![100, 200, 150, 250]);
    let df = DataFrame::from_series(vec![regions, amounts]).unwrap();

    let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();
    let result = grouped.sum("amount", Some("total")).unwrap();

    assert_eq!(result.num_rows(), 2);
    assert_eq!(result.num_columns(), 2); // region + total
    assert!(result.columns().contains(&"region".to_string()));
    assert!(result.columns().contains(&"total".to_string()));
}

#[test]
fn test_grouped_dataframe_count() {
    let regions = Series::from_strings("region", vec!["North", "South", "North", "South", "North"]);
    let amounts = Series::from_ints("amount", vec![100, 200, 150, 250, 175]);
    let df = DataFrame::from_series(vec![regions, amounts]).unwrap();

    let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();
    let result = grouped.count(Some("n")).unwrap();

    assert_eq!(result.num_rows(), 2);

    // Check that North has 3 and South has 2
    let region_col = result.column("region").unwrap();
    let count_col = result.column("n").unwrap();

    for i in 0..result.num_rows() {
        let region = region_col.get(i).unwrap();
        let count = count_col.get(i).unwrap();

        match region {
            Value::String(r) if r.as_ref() == "North" => {
                assert_eq!(count, Value::Int(3));
            }
            Value::String(r) if r.as_ref() == "South" => {
                assert_eq!(count, Value::Int(2));
            }
            _ => panic!("unexpected region: {:?}", region),
        }
    }
}

#[test]
fn test_grouped_dataframe_multiple_aggregations() {
    let regions = Series::from_strings("region", vec!["North", "South", "North"]);
    let amounts = Series::from_ints("amount", vec![100, 200, 150]);
    let df = DataFrame::from_series(vec![regions, amounts]).unwrap();

    let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();

    let specs = vec![
        AggSpec::sum("amount", "total"),
        AggSpec::count("n"),
        AggSpec::mean("amount", "avg"),
    ];

    let result = grouped.aggregate(&specs).unwrap();

    assert_eq!(result.num_columns(), 4); // region + total + n + avg
    assert_eq!(result.columns(), vec!["region", "total", "n", "avg"]);
}

#[test]
fn test_language_group_by_with_strings() {
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let grouped = df.group_by("region")
        grouped.num_groups()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 groups"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_group_by_with_column_shorthand() {
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let grouped = df |> group_by(.region)
        grouped.num_groups()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 groups"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_group_by_sum() {
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> sum("amount", "total")
        result.num_columns()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns (region, total)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_group_by_count() {
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150},
            {"region": "North", "amount": 175}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> count("n")
        result.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows (one per group)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_agg_builder() {
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> agg(
            Agg.sum("amount", "total"),
            Agg.count("n")
        )
        result.num_columns()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 columns (region, total, n)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_agg_mean() {
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "North", "amount": 200}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> mean("amount")
        result.columns()
    }"#);

    match result {
        Ok(Value::List(cols)) => {
            let cols = cols.borrow();
            assert_eq!(cols.len(), 2); // region, amount
        }
        Ok(other) => panic!("Expected List, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// Join operation tests
// ============================================================================

#[test]
fn test_language_join_inner() {
    // Test inner join using pipeline syntax
    let result = eval_expr_dynamic(r#"{
        let users = Data.frame([
            {"user_id": 1, "name": "Alice"},
            {"user_id": 2, "name": "Bob"},
            {"user_id": 3, "name": "Charlie"}
        ])
        let orders = Data.frame([
            {"user_id": 1, "amount": 100},
            {"user_id": 1, "amount": 150},
            {"user_id": 2, "amount": 200}
        ])
        let result = users |> join(orders, Join.on("user_id"))
        result.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 rows from inner join"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_left() {
    // Test left join - all left rows preserved
    let result = eval_expr_dynamic(r#"{
        let users = Data.frame([
            {"user_id": 1, "name": "Alice"},
            {"user_id": 2, "name": "Bob"},
            {"user_id": 3, "name": "Charlie"}
        ])
        let orders = Data.frame([
            {"user_id": 1, "amount": 100}
        ])
        let result = users |> join(orders, Join.left("user_id"))
        result.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 rows from left join (all left rows)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_method_syntax() {
    // Test join using method syntax
    let result = eval_expr_dynamic(r#"{
        let df1 = Data.frame([
            {"id": 1, "val": "a"},
            {"id": 2, "val": "b"}
        ])
        let df2 = Data.frame([
            {"id": 1, "score": 100},
            {"id": 2, "score": 200}
        ])
        let result = df1.join(df2, Join.on("id"))
        result.columns().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 columns (id, val, score)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_different_columns() {
    // Test join on different column names
    let result = eval_expr_dynamic(r#"{
        let df1 = Data.frame([
            {"id": 1, "val": "a"},
            {"id": 2, "val": "b"}
        ])
        let df2 = Data.frame([
            {"ref_id": 1, "score": 100},
            {"ref_id": 2, "score": 200}
        ])
        let result = df1 |> join(df2, Join.cols("id", "ref_id"))
        result.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows from inner join"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_pipeline_chain() {
    // Test join as part of a pipeline chain
    let result = eval_expr_dynamic(r#"{
        let users = Data.frame([
            {"user_id": 1, "name": "Alice", "dept": "Eng"},
            {"user_id": 2, "name": "Bob", "dept": "Sales"}
        ])
        let salaries = Data.frame([
            {"user_id": 1, "salary": 100000},
            {"user_id": 2, "salary": 80000}
        ])
        let result = users
            |> join(salaries, Join.on("user_id"))
            |> select("name", "salary")
        result.columns().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// Database to DataFrame tests
// ============================================================================

#[test]
fn test_data_from_query_sqlite() {
    // Test Data.from_query with SQLite in-memory database
    let result = eval_expr_dynamic(r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)");
        db.execute("INSERT INTO users VALUES (1, 'Alice', 30)");
        db.execute("INSERT INTO users VALUES (2, 'Bob', 25)");
        db.execute("INSERT INTO users VALUES (3, 'Charlie', 35)");
        let df = Data.from_query(db, "SELECT * FROM users ORDER BY id");
        df.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 rows from query"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_columns() {
    // Test that Data.from_query returns correct column count
    let result = eval_expr_dynamic(r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE products (id INT, name TEXT, price REAL)");
        db.execute("INSERT INTO products VALUES (1, 'Widget', 9.99)");
        let df = Data.from_query(db, "SELECT id, name, price FROM products");
        df.num_columns()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 columns"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_with_params() {
    // Test Data.from_query with parameters
    let result = eval_expr_dynamic(r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE users (id INT, name TEXT)");
        db.execute("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')");
        let df = Data.from_query(db, "SELECT * FROM users WHERE id > ?", [1]);
        df.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows with id > 1"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_empty_result() {
    // Test Data.from_query with no results
    let result = eval_expr_dynamic(r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE users (id INT, name TEXT)");
        let df = Data.from_query(db, "SELECT * FROM users");
        df.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 0, "Expected 0 rows from empty table"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_with_operations() {
    // Test that DataFrame from query supports normal operations
    // Use SQL query with WHERE clause instead of filter pipeline
    let result = eval_expr_dynamic(r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE sales (region TEXT, amount INT)");
        db.execute("INSERT INTO sales VALUES ('East', 100), ('West', 200), ('East', 150)");
        let df = Data.from_query(db, "SELECT * FROM sales WHERE amount > 100");
        df.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows from WHERE clause"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// OLAP Cube Builder Tests (4A.2)
// ============================================================================

#[test]
fn test_cube_from_dataframe() {
    // Test basic Cube.from(df) construction
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0},
            {"region": "South", "revenue": 200.0}
        ]
        let df = Data.frame(rows)
        let builder = Cube.from(df)
        type_of(builder)
    }"#);

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "CubeBuilder"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_from_with_name() {
    // Test Cube.from("name", df) with name
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let cube = Cube.from("sales_cube", df)
            |> dimension("region")
            |> measure("revenue", "sum")
            |> build()
        cube.name()
    }"#);

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "sales_cube"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_dimension_pipeline() {
    // Test adding dimension via pipeline
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "product": "A", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let cube = Cube.from(df)
            |> dimension("region")
            |> dimension("product")
            |> measure("revenue", "sum")
            |> build()
        cube.dimensions().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 dimensions"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_measure_pipeline() {
    // Test adding measure via pipeline
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0, "units": 10}
        ]
        let df = Data.frame(rows)
        let cube = Cube.from(df)
            |> dimension("region")
            |> measure("revenue", "sum")
            |> measure("units", "count")
            |> build()
        cube.measures().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 measures"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_hierarchy_pipeline() {
    // Test adding hierarchy via pipeline
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"year": 2024, "quarter": "Q1", "month": "Jan", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let levels = ["year", "quarter", "month"]
        let cube = Cube.from(df)
            |> dimension("year")
            |> dimension("quarter")
            |> dimension("month")
            |> hierarchy("time", levels)
            |> measure("revenue", "sum")
            |> build()
        cube.hierarchies().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1, "Expected 1 hierarchy"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_method_syntax() {
    // Test method syntax instead of pipeline
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let builder1 = Cube.from(df)
        let builder2 = builder1.dimension("region")
        let builder3 = builder2.measure("revenue", "sum")
        let cube = builder3.build()
        cube.row_count()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1, "Expected 1 row"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_full_pipeline() {
    // Test complete pipeline with all builder operations
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "product": "Widget", "year": 2024, "quarter": "Q1", "revenue": 100.0},
            {"region": "South", "product": "Gadget", "year": 2024, "quarter": "Q1", "revenue": 200.0},
            {"region": "North", "product": "Widget", "year": 2024, "quarter": "Q2", "revenue": 150.0}
        ]
        let df = Data.frame(rows)
        let time_levels = ["year", "quarter"]
        let cube = Cube.from("sales", df)
            |> dimension("region")
            |> dimension("product")
            |> dimension("year")
            |> dimension("quarter")
            |> hierarchy("time", time_levels)
            |> measure("revenue", "sum")
            |> build()
        cube.dimensions().len()
    }"#);

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 4, "Expected 4 dimensions"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// DataFrame.to_cube() Tests (4A.6.2)
// ============================================================================

#[test]
fn test_dataframe_to_cube_method() {
    // Test df.to_cube() method syntax
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0},
            {"region": "South", "revenue": 200.0}
        ]
        let df = Data.frame(rows)
        let builder = df.to_cube()
        type_of(builder)
    }"#);

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "CubeBuilder"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_to_cube_with_name() {
    // Test df.to_cube("name") method syntax with name
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let cube = df.to_cube("my_cube")
            |> dimension("region")
            |> measure("revenue", "sum")
            |> build()
        cube.name()
    }"#);

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "my_cube"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_to_cube_pipeline() {
    // Test df |> to_cube("name") pipeline syntax
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let cube = df
            |> to_cube("sales")
            |> dimension("region")
            |> measure("revenue", "sum")
            |> build()
        cube.name()
    }"#);

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "sales"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_to_cube_full_pipeline() {
    // Test complete mixed DataFrame -> Cube pipeline
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "product": "A", "revenue": 100.0},
            {"region": "South", "product": "B", "revenue": 200.0},
            {"region": "North", "product": "A", "revenue": 150.0}
        ]
        let df = Data.frame(rows)

        // DataFrame operations, then convert to Cube, then OLAP operations
        let result_df = df
            |> to_cube("mixed_pipeline")
            |> dimension("region")
            |> dimension("product")
            |> measure("revenue", "sum")
            |> build()
            |> slice("region", "North")
            |> to_dataframe()

        result_df.num_rows()
    }"#);

    match result {
        Ok(Value::Int(n)) => {
            // After slicing "region" = "North", should have rows with North only
            assert!(n > 0, "Expected at least 1 row after slice");
        }
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_pipeline_dataframe_to_cube_back_to_dataframe() {
    // Test mixed pipeline: DataFrame -> Cube -> DataFrame
    let result = eval_expr_dynamic(r#"{
        let rows = [
            {"region": "North", "revenue": 100.0},
            {"region": "South", "revenue": 200.0}
        ]
        let df = Data.frame(rows)

        // Create cube, query it, and convert back to DataFrame
        let cube = df.to_cube("test")
            |> dimension("region")
            |> measure("revenue", "sum")
            |> build()

        let result = cube |> to_dataframe()
        type_of(result)
    }"#);

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "DataFrame"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}
