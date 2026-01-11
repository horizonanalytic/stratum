//! Integration tests for the data module (DataFrame and Series)

use std::sync::Arc;

use stratum_core::bytecode::Value;
use stratum_core::data::{DataFrame, Series};

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
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"},
            {"name": "Bob", "age": 25, "city": "LA"}
        ]
        let df = Data.frame(rows)
        let selected = df.select("name", "age")
        selected.columns().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_select_pipeline() {
    // Test that df |> select("col1") works via pipeline
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"},
            {"name": "Bob", "age": 25, "city": "LA"}
        ]
        let df = Data.frame(rows)
        let selected = df |> select("name")
        selected.columns().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1, "Expected 1 column after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_select_with_column_shorthand() {
    // Test that df |> select(.name, .age) works with column shorthands
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"},
            {"name": "Bob", "age": 25, "city": "LA"}
        ]
        let df = Data.frame(rows)
        let selected = df |> select(.name, .age)
        selected.columns().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_select_preserves_data() {
    // Test that select preserves the actual data values
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]
        let df = Data.frame(rows)
        let selected = df |> select(.name)
        selected.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows after select"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_method_select_with_column_shorthand() {
    // Test that df.select(.name) works (method syntax with column shorthand)
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"name": "Alice", "age": 30, "city": "NYC"}
        ]
        let df = Data.frame(rows)
        let selected = df.select(.name, .city)
        selected.columns().len()
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let grouped = df.group_by("region")
        grouped.num_groups()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 groups"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_group_by_with_column_shorthand() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let grouped = df |> group_by(.region)
        grouped.num_groups()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 groups"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_group_by_sum() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> sum("amount", "total")
        result.num_columns()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 columns (region, total)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_group_by_count() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "South", "amount": 200},
            {"region": "North", "amount": 150},
            {"region": "North", "amount": 175}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> count("n")
        result.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows (one per group)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_agg_builder() {
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 columns (region, total, n)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_agg_mean() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "amount": 100},
            {"region": "North", "amount": 200}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> mean("amount")
        result.columns()
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 rows from inner join"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_left() {
    // Test left join - all left rows preserved
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 rows from left join (all left rows)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_method_syntax() {
    // Test join using method syntax
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 columns (id, val, score)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_different_columns() {
    // Test join on different column names
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows from inner join"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_language_join_pipeline_chain() {
    // Test join as part of a pipeline chain
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)");
        db.execute("INSERT INTO users VALUES (1, 'Alice', 30)");
        db.execute("INSERT INTO users VALUES (2, 'Bob', 25)");
        db.execute("INSERT INTO users VALUES (3, 'Charlie', 35)");
        let df = Data.from_query(db, "SELECT * FROM users ORDER BY id");
        df.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 rows from query"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_columns() {
    // Test that Data.from_query returns correct column count
    let result = eval_expr_dynamic(
        r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE products (id INT, name TEXT, price REAL)");
        db.execute("INSERT INTO products VALUES (1, 'Widget', 9.99)");
        let df = Data.from_query(db, "SELECT id, name, price FROM products");
        df.num_columns()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "Expected 3 columns"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_with_params() {
    // Test Data.from_query with parameters
    let result = eval_expr_dynamic(
        r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE users (id INT, name TEXT)");
        db.execute("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')");
        let df = Data.from_query(db, "SELECT * FROM users WHERE id > ?", [1]);
        df.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 rows with id > 1"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_data_from_query_empty_result() {
    // Test Data.from_query with no results
    let result = eval_expr_dynamic(
        r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE users (id INT, name TEXT)");
        let df = Data.from_query(db, "SELECT * FROM users");
        df.num_rows()
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
        let db = Db.sqlite(":memory:");
        db.execute("CREATE TABLE sales (region TEXT, amount INT)");
        db.execute("INSERT INTO sales VALUES ('East', 100), ('West', 200), ('East', 150)");
        let df = Data.from_query(db, "SELECT * FROM sales WHERE amount > 100");
        df.num_rows()
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "revenue": 100.0},
            {"region": "South", "revenue": 200.0}
        ]
        let df = Data.frame(rows)
        let builder = Cube.from(df)
        type_of(builder)
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "CubeBuilder"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_from_with_name() {
    // Test Cube.from("name", df) with name
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let cube = Cube.from("sales_cube", df)
            |> dimension("region")
            |> measure("revenue", "sum")
            |> build()
        cube.name()
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "sales_cube"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_dimension_pipeline() {
    // Test adding dimension via pipeline
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 dimensions"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_measure_pipeline() {
    // Test adding measure via pipeline
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "Expected 2 measures"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_hierarchy_pipeline() {
    // Test adding hierarchy via pipeline
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1, "Expected 1 hierarchy"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_method_syntax() {
    // Test method syntax instead of pipeline
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let builder1 = Cube.from(df)
        let builder2 = builder1.dimension("region")
        let builder3 = builder2.measure("revenue", "sum")
        let cube = builder3.build()
        cube.row_count()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1, "Expected 1 row"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_cube_full_pipeline() {
    // Test complete pipeline with all builder operations
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "revenue": 100.0},
            {"region": "South", "revenue": 200.0}
        ]
        let df = Data.frame(rows)
        let builder = df.to_cube()
        type_of(builder)
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "CubeBuilder"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_to_cube_with_name() {
    // Test df.to_cube("name") method syntax with name
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "North", "revenue": 100.0}
        ]
        let df = Data.frame(rows)
        let cube = df.to_cube("my_cube")
            |> dimension("region")
            |> measure("revenue", "sum")
            |> build()
        cube.name()
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "my_cube"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_to_cube_pipeline() {
    // Test df |> to_cube("name") pipeline syntax
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "sales"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_to_cube_full_pipeline() {
    // Test complete mixed DataFrame -> Cube pipeline
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

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
    let result = eval_expr_dynamic(
        r#"{
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
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "DataFrame"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// Collection Enhancements Tests (11.8)
// ============================================================================

#[test]
fn test_set_creation_and_operations() {
    // Test Set.new() and basic operations
    let result = eval_expr_dynamic(
        r#"{
        let s = Set.new()
        s.add(1).add(2).add(3).len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_set_from_list() {
    // Test Set.from() with duplicates
    let result = eval_expr_dynamic(
        r#"{
        let s = Set.from([1, 2, 2, 3, 3, 3])
        s.len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_set_contains() {
    // Test contains - true case
    let result = eval_expr_dynamic(
        r#"{
        let s = Set.from([1, 2, 3])
        s.contains(2)
    }"#,
    );

    match result {
        Ok(Value::Bool(b)) => assert!(b),
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test contains - false case
    let result = eval_expr_dynamic(
        r#"{
        let s = Set.from([1, 2, 3])
        s.contains(5)
    }"#,
    );

    match result {
        Ok(Value::Bool(b)) => assert!(!b),
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_set_union_intersection() {
    // Test union
    let result = eval_expr_dynamic(
        r#"{
        let a = Set.from([1, 2, 3])
        let b = Set.from([3, 4, 5])
        a.union(b).len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 5),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test intersection
    let result = eval_expr_dynamic(
        r#"{
        let a = Set.from([1, 2, 3])
        let b = Set.from([2, 3, 4])
        a.intersection(b).len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_set_difference() {
    let result = eval_expr_dynamic(
        r#"{
        let a = Set.from([1, 2, 3, 4])
        let b = Set.from([3, 4, 5])
        a.difference(b).len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2), // 1 and 2
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_set_subset_superset() {
    // Test is_subset
    let result = eval_expr_dynamic(
        r#"{
        let a = Set.from([1, 2])
        let b = Set.from([1, 2, 3])
        a.is_subset(b)
    }"#,
    );

    match result {
        Ok(Value::Bool(b)) => assert!(b),
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test is_superset
    let result = eval_expr_dynamic(
        r#"{
        let a = Set.from([1, 2])
        let b = Set.from([1, 2, 3])
        b.is_superset(a)
    }"#,
    );

    match result {
        Ok(Value::Bool(b)) => assert!(b),
        Ok(other) => panic!("Expected Bool, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_list_enumerate() {
    let result = eval_expr_dynamic(
        r#"{
        let items = ["a", "b", "c"]
        let enumerated = items.enumerate()
        enumerated.len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Check that enumerate returns [index, value] pairs
    let result = eval_expr_dynamic(
        r#"{
        let items = ["a", "b", "c"]
        let enumerated = items.enumerate()
        enumerated[1][0]  // Index of second element
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_list_chunk() {
    let result = eval_expr_dynamic(
        r#"{
        let items = [1, 2, 3, 4, 5, 6, 7]
        let chunks = items.chunk(3)
        chunks.len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3), // [1,2,3], [4,5,6], [7]
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Check chunk contents
    let result = eval_expr_dynamic(
        r#"{
        let items = [1, 2, 3, 4, 5]
        let chunks = items.chunk(2)
        chunks[0].len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_list_window() {
    let result = eval_expr_dynamic(
        r#"{
        let items = [1, 2, 3, 4, 5]
        let windows = items.window(3)
        windows.len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3), // [1,2,3], [2,3,4], [3,4,5]
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Empty result for window larger than list
    let result = eval_expr_dynamic(
        r#"{
        let items = [1, 2]
        items.window(5).len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 0),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_list_unique() {
    let result = eval_expr_dynamic(
        r#"{
        let items = [1, 2, 2, 3, 1, 4, 3]
        items.unique().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 4), // 1, 2, 3, 4
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Check order preservation
    let result = eval_expr_dynamic(
        r#"{
        let items = ["c", "a", "b", "a", "c"]
        let unique = items.unique()
        unique[0]
    }"#,
    );

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "c"),
        Ok(other) => panic!("Expected String, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_list_group_by() {
    let result = eval_expr_dynamic(
        r#"{
        let items = [1, 2, 3, 4, 5, 6]
        let groups = items.group_by(|x| { x % 2 })
        groups.keys().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2), // 0 (even) and 1 (odd)
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Check group contents
    let result = eval_expr_dynamic(
        r#"{
        let words = ["apple", "ant", "bear", "ace"]
        let groups = words.group_by(|w| { w.substring(0, 1) })
        groups.get("a").len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3), // apple, ant, ace
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// P0 Statistical Functions Tests (11.1)
// ============================================================================

#[test]
fn test_series_std_variance() {
    // Test standard deviation
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0])
        s.std()
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => assert!((f - 2.0).abs() < 0.1, "Expected std ~2.0, got {}", f),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test variance
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0])
        s.var()
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => assert!((f - 4.0).abs() < 0.1, "Expected var ~4.0, got {}", f),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_median_mode() {
    // Test median
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 3, 5, 7, 9])
        s.median()
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => assert!((f - 5.0).abs() < 0.01),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test mode
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 2, 3, 3, 3, 4])
        s.mode()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_quantile_percentile() {
    // Test quantile (0-1 scale)
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        s.quantile(0.5)
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => assert!((f - 5.5).abs() < 0.1),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test percentile (0-100 scale)
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        s.percentile(25)
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => {
            assert!(f >= 2.0 && f <= 3.5, "Expected percentile ~2.75, got {}", f)
        }
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_skew_kurtosis() {
    // Test skewness (symmetric distribution should have skew ~0)
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5, 6, 7, 8, 9])
        s.skew()
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => assert!(f.abs() < 0.5, "Expected skew near 0, got {}", f),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test kurtosis
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5, 6, 7, 8, 9])
        s.kurtosis()
    }"#,
    );

    match result {
        Ok(Value::Float(_)) => (), // Just verify it returns a float
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_describe() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"a": 1, "b": 10.0},
            {"a": 2, "b": 20.0},
            {"a": 3, "b": 30.0},
            {"a": 4, "b": 40.0},
            {"a": 5, "b": 50.0}
        ]
        let df = Data.frame(rows)
        let desc = df.describe()
        desc.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(
            n, 8,
            "describe() should have 8 rows (count, mean, std, min, 25%, 50%, 75%, max)"
        ),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_corr() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"a": 1, "b": 2},
            {"a": 2, "b": 4},
            {"a": 3, "b": 6},
            {"a": 4, "b": 8}
        ]
        let df = Data.frame(rows)
        let corr = df.corr()
        corr.num_columns()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3, "corr() should have columns (index + a + b)"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_cov() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"x": 1.0, "y": 2.0},
            {"x": 2.0, "y": 4.0},
            {"x": 3.0, "y": 6.0}
        ]
        let df = Data.frame(rows)
        let cov = df.cov()
        cov.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "cov() should have row per numeric column"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_value_counts() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"category": "A"},
            {"category": "B"},
            {"category": "A"},
            {"category": "A"},
            {"category": "B"}
        ]
        let df = Data.frame(rows)
        let counts = df.value_counts("category")
        counts.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2, "value_counts should have 2 unique values"),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// P0 Window Functions Tests (11.2)
// ============================================================================

#[test]
fn test_series_rolling_operations() {
    // Test rolling mean
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1.0, 2.0, 3.0, 4.0, 5.0])
        let r = s.rolling(3)
        r.mean().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 5),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test rolling sum
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1.0, 2.0, 3.0, 4.0, 5.0])
        s.rolling(2).sum().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 5),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_cumulative_operations() {
    // Test cumsum
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5])
        let cs = s.cumsum()
        cs.get(4)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 15), // 1+2+3+4+5 = 15
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test cummax
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [3, 1, 4, 1, 5])
        let cm = s.cummax()
        cm.get(4)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 5),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test cummin
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [5, 3, 4, 1, 2])
        let cm = s.cummin()
        cm.get(4)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test cumprod
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4])
        let cp = s.cumprod()
        cp.get(3)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 24), // 1*2*3*4 = 24
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_shift_lag_lead() {
    // Test shift (positive = lag)
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5])
        let shifted = s.shift(1)
        shifted.get(1)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1), // Value at index 0 shifted to index 1
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test lag
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, 2, 3, 4, 5])
        let lagged = s.lag(1)
        lagged.get(2)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2), // lag(1) at index 2 = value at index 1
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_diff_pct_change() {
    // Test diff
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [10, 15, 25, 30])
        let d = s.diff(1)
        d.get(2)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 10), // 25 - 15 = 10
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test pct_change
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [100.0, 110.0, 121.0])
        let pct = s.pct_change(1)
        pct.get(1)
    }"#,
    );

    match result {
        Ok(Value::Float(f)) => assert!((f - 0.1).abs() < 0.01, "Expected ~0.1, got {}", f),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// P0 Missing Data Handling Tests (11.3)
// ============================================================================

#[test]
fn test_series_dropna() {
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, null, 3, null, 5])
        s.dropna().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 3), // Only 1, 3, 5 remain
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_fillna() {
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, null, 3, null, 5])
        let filled = s.fillna(0)
        filled.get(1)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 0),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_series_fillna_forward_backward() {
    // Test forward fill
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, null, null, 4, 5])
        let filled = s.fillna("forward")
        filled.get(2)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1), // Forward filled from index 0
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    // Test backward fill
    let result = eval_expr_dynamic(
        r#"{
        let s = Data.series("vals", [1, null, null, 4, 5])
        let filled = s.fillna("backward")
        filled.get(1)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 4), // Backward filled from index 3
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_dropna() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"a": 1, "b": 10},
            {"a": null, "b": 20},
            {"a": 3, "b": null},
            {"a": 4, "b": 40}
        ]
        let df = Data.frame(rows)
        df.dropna().num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2), // Only rows without any nulls
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_dataframe_fillna() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"a": 1, "b": null},
            {"a": null, "b": 20}
        ]
        let df = Data.frame(rows)
        let filled = df.fillna(0)
        filled.column("b").get(0)
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 0),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// P0 Aggregation Extensions Tests (11.1.13-17)
// ============================================================================

#[test]
fn test_agg_std_var() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "A", "val": 10},
            {"region": "A", "val": 20},
            {"region": "B", "val": 5},
            {"region": "B", "val": 15}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> agg(Agg.std("val", "std_val"))
        result.columns().len()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2), // region, std_val
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "A", "val": 10},
            {"region": "A", "val": 20}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> agg(Agg.var("val", "var_val"))
        result.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1), // One group
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_agg_median_mode() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"cat": "X", "val": 1},
            {"cat": "X", "val": 3},
            {"cat": "X", "val": 5}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.cat) |> agg(Agg.median("val", "med"))
        result.columns()
    }"#,
    );

    match result {
        Ok(Value::List(cols)) => {
            let cols = cols.borrow();
            assert!(cols.len() == 2);
        }
        Ok(other) => panic!("Expected List, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }

    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"cat": "X", "val": 1},
            {"cat": "X", "val": 2},
            {"cat": "X", "val": 2}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.cat) |> agg(Agg.mode("val", "mode_val"))
        result.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 1),
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

#[test]
fn test_agg_count_distinct() {
    let result = eval_expr_dynamic(
        r#"{
        let rows = [
            {"region": "A", "product": "X"},
            {"region": "A", "product": "X"},
            {"region": "A", "product": "Y"},
            {"region": "B", "product": "Z"}
        ]
        let df = Data.frame(rows)
        let result = df |> group_by(.region) |> agg(Agg.count_distinct("product", "unique_products"))
        result.num_rows()
    }"#,
    );

    match result {
        Ok(Value::Int(n)) => assert_eq!(n, 2), // Two groups: A and B
        Ok(other) => panic!("Expected Int, got {:?}", other),
        Err(e) => panic!("Program failed: {}", e),
    }
}

// ============================================================================
// Phase 8.5: Performance Optimization Tests
// ============================================================================

#[test]
fn test_memory_usage_dataframe() {
    // Test that memory_usage returns valid statistics
    let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
    let ages = Series::from_ints("age", vec![25, 30, 35]);
    let df = DataFrame::from_series(vec![names, ages]).unwrap();

    let stats = df.memory_usage();
    assert_eq!(stats.num_rows, 3);
    assert_eq!(stats.num_columns, 2);
    assert!(stats.data_bytes > 0);
    assert!(stats.total_bytes > 0);
    assert!(stats.bytes_per_row > 0.0);
}

#[test]
fn test_memory_usage_series() {
    let series = Series::from_ints("numbers", vec![1, 2, 3, 4, 5]);
    let stats = series.memory_usage();

    assert_eq!(stats.num_rows, 5);
    assert_eq!(stats.num_columns, 1);
    assert!(stats.data_bytes > 0);
    assert!(stats.total_bytes > 0);
}

#[test]
fn test_parallel_threshold_configuration() {
    use stratum_core::data::{parallel_threshold, set_parallel_threshold};

    // Get the default threshold
    let default = parallel_threshold();
    assert!(default > 0);

    // Set a custom threshold
    set_parallel_threshold(5000);
    assert_eq!(parallel_threshold(), 5000);

    // Reset to default
    set_parallel_threshold(default);
    assert_eq!(parallel_threshold(), default);
}

#[test]
fn test_filter_by_indices_uses_parallel() {
    // Create a large DataFrame to test parallel filtering
    let n = 100;
    let values: Vec<i64> = (0..n).collect();
    let series = Series::from_ints("value", values);
    let df = DataFrame::from_series(vec![series]).unwrap();

    // Set a low threshold to ensure parallel path is used
    use stratum_core::data::{parallel_threshold, set_parallel_threshold};
    let original = parallel_threshold();
    set_parallel_threshold(10);

    // Filter by indices (should use parallel path)
    let indices: Vec<usize> = (0..50).collect();
    let filtered = df.filter_by_indices(&indices).unwrap();

    assert_eq!(filtered.num_rows(), 50);

    // Restore threshold
    set_parallel_threshold(original);
}

#[test]
fn test_lazy_frame_basic() {
    use stratum_core::data::LazyFrame;

    let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
    let ages = Series::from_ints("age", vec![25, 30, 35]);
    let df = DataFrame::from_series(vec![names, ages]).unwrap();

    // Create lazy frame and apply operations
    let result = LazyFrame::new(df)
        .select(["name", "age"])
        .limit(2)
        .collect()
        .unwrap();

    assert_eq!(result.num_rows(), 2);
    assert_eq!(result.num_columns(), 2);
}

#[test]
fn test_lazy_frame_filter() {
    use stratum_core::data::{lazy::FilterPredicate, LazyFrame};

    let ages = Series::from_ints("age", vec![20, 30, 40, 50]);
    let df = DataFrame::from_series(vec![ages]).unwrap();

    // Filter using LazyFrame
    let result = LazyFrame::new(df)
        .filter(FilterPredicate::Gt("age".to_string(), Value::Int(25)))
        .collect()
        .unwrap();

    assert_eq!(result.num_rows(), 3); // 30, 40, 50
}

#[test]
fn test_lazy_frame_query_optimization() {
    use stratum_core::data::LazyFrame;

    let ages = Series::from_ints("age", vec![20, 30, 40, 50]);
    let df = DataFrame::from_series(vec![ages]).unwrap();

    // Chain operations that will be optimized
    let lf = LazyFrame::new(df)
        .select(["age"])
        .select(["age"]) // Duplicate select - should be pruned
        .limit(10)
        .limit(5); // Multiple limits - should be merged to min

    // Get query plan
    let plan = lf.explain();
    assert!(plan.contains("Query Plan"));

    // Execute optimized plan
    let result = lf.collect().unwrap();
    assert_eq!(result.num_rows(), 4); // All 4 rows (limit 5 > row count)
}
