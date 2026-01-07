//! SQL query support for DataFrames using DataFusion
//!
//! This module provides SQL query capabilities for Stratum DataFrames
//! by leveraging Apache DataFusion as the query engine.

use std::sync::Arc;

use arrow::datatypes::Schema;
use datafusion::datasource::MemTable;
use datafusion::prelude::*;

use super::dataframe::DataFrame;
use super::error::{DataError, DataResult};

/// Create a tokio runtime for blocking SQL operations
fn create_runtime() -> DataResult<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DataError::Sql(e.to_string()))
}

/// Execute a SQL query against a single DataFrame
///
/// The DataFrame is registered as a table named "df" by default.
///
/// # Arguments
/// * `df` - The DataFrame to query
/// * `query` - SQL query string
///
/// # Example
/// ```ignore
/// let result = sql_query(&df, "SELECT name, age FROM df WHERE age > 25")?;
/// ```
pub fn sql_query(df: &DataFrame, query: &str) -> DataResult<DataFrame> {
    sql_query_with_name(df, "df", query)
}

/// Execute a SQL query against a single DataFrame with a custom table name
///
/// # Arguments
/// * `df` - The DataFrame to query
/// * `table_name` - Name to use for the table in SQL
/// * `query` - SQL query string
pub fn sql_query_with_name(df: &DataFrame, table_name: &str, query: &str) -> DataResult<DataFrame> {
    let rt = create_runtime()?;

    rt.block_on(async {
        let ctx = SessionContext::new();

        // Register the DataFrame as a table
        register_dataframe(&ctx, table_name, df).await?;

        // Execute the query
        let df_result = ctx.sql(query).await?;

        // Get schema before collecting (collect consumes the DataFrame)
        let df_schema = df_result.schema().clone();

        // Collect results
        let batches = df_result.collect().await?;

        if batches.is_empty() {
            // Convert DFSchema to Arrow Schema for empty result
            let arrow_schema: &Schema = df_schema.as_ref();
            return Ok(DataFrame::empty(Arc::new(arrow_schema.clone())));
        }

        let schema = batches[0].schema();
        DataFrame::from_batches(schema, batches)
    })
}

/// A SQL context for executing queries against multiple DataFrames
///
/// # Example
/// ```ignore
/// let mut ctx = SqlContext::new();
/// ctx.register("users", &users_df)?;
/// ctx.register("orders", &orders_df)?;
/// let result = ctx.query("SELECT u.name, COUNT(*) FROM users u JOIN orders o ON u.id = o.user_id GROUP BY u.name")?;
/// ```
pub struct SqlContext {
    session: SessionContext,
    runtime: tokio::runtime::Runtime,
}

impl SqlContext {
    /// Create a new SQL context
    pub fn new() -> DataResult<Self> {
        let runtime = create_runtime()?;
        let session = SessionContext::new();
        Ok(Self { session, runtime })
    }

    /// Register a DataFrame as a table with the given name
    pub fn register(&self, table_name: &str, df: &DataFrame) -> DataResult<()> {
        self.runtime.block_on(async {
            register_dataframe(&self.session, table_name, df).await
        })
    }

    /// Execute a SQL query and return the result as a DataFrame
    pub fn query(&self, sql: &str) -> DataResult<DataFrame> {
        self.runtime.block_on(async {
            let df_result = self.session.sql(sql).await?;

            // Get schema before collecting (collect consumes the DataFrame)
            let df_schema = df_result.schema().clone();

            let batches = df_result.collect().await?;

            if batches.is_empty() {
                let arrow_schema: &Schema = df_schema.as_ref();
                return Ok(DataFrame::empty(Arc::new(arrow_schema.clone())));
            }

            let schema = batches[0].schema();
            DataFrame::from_batches(schema, batches)
        })
    }

    /// Get the list of registered table names
    pub fn tables(&self) -> Vec<String> {
        self.runtime.block_on(async {
            self.session
                .catalog("datafusion")
                .and_then(|cat| cat.schema("public"))
                .map(|schema| schema.table_names())
                .unwrap_or_default()
        })
    }
}

impl Default for SqlContext {
    fn default() -> Self {
        Self::new().expect("Failed to create SqlContext")
    }
}

/// Register a Stratum DataFrame as a DataFusion table
async fn register_dataframe(
    ctx: &SessionContext,
    table_name: &str,
    df: &DataFrame,
) -> DataResult<()> {
    let schema = df.schema().clone();
    let batches = df.batches().to_vec();

    // Create a MemTable from the batches
    let mem_table = MemTable::try_new(schema, vec![batches])?;

    // Register the table
    ctx.register_table(table_name, Arc::new(mem_table))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Series;

    fn sample_dataframe() -> DataFrame {
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie", "Diana"]);
        let ages = Series::from_ints("age", vec![30, 25, 35, 28]);
        let scores = Series::from_floats("score", vec![85.5, 92.0, 78.3, 88.5]);
        DataFrame::from_series(vec![names, ages, scores]).unwrap()
    }

    #[test]
    fn test_sql_select_all() {
        let df = sample_dataframe();
        let result = sql_query(&df, "SELECT * FROM df").unwrap();
        assert_eq!(result.num_rows(), 4);
        assert_eq!(result.num_columns(), 3);
    }

    #[test]
    fn test_sql_filter() {
        let df = sample_dataframe();
        let result = sql_query(&df, "SELECT name, age FROM df WHERE age > 27").unwrap();
        assert_eq!(result.num_rows(), 3); // Alice (30), Charlie (35), Diana (28)
        assert_eq!(result.num_columns(), 2);
    }

    #[test]
    fn test_sql_aggregation() {
        let df = sample_dataframe();
        let result = sql_query(&df, "SELECT COUNT(*) as cnt, AVG(age) as avg_age FROM df").unwrap();
        assert_eq!(result.num_rows(), 1);
    }

    #[test]
    fn test_sql_order_by() {
        let df = sample_dataframe();
        let result = sql_query(&df, "SELECT name FROM df ORDER BY age DESC").unwrap();
        assert_eq!(result.num_rows(), 4);

        let name_col = result.column("name").unwrap();
        assert_eq!(name_col.get(0).unwrap().to_string(), "Charlie");
    }

    #[test]
    fn test_sql_context_multiple_tables() {
        let users = {
            let ids = Series::from_ints("id", vec![1, 2, 3]);
            let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
            DataFrame::from_series(vec![ids, names]).unwrap()
        };

        let orders = {
            let order_ids = Series::from_ints("order_id", vec![101, 102, 103, 104]);
            let user_ids = Series::from_ints("user_id", vec![1, 2, 1, 3]);
            let amounts = Series::from_floats("amount", vec![100.0, 200.0, 150.0, 300.0]);
            DataFrame::from_series(vec![order_ids, user_ids, amounts]).unwrap()
        };

        let ctx = SqlContext::new().unwrap();
        ctx.register("users", &users).unwrap();
        ctx.register("orders", &orders).unwrap();

        let result = ctx.query(
            "SELECT u.name, SUM(o.amount) as total
             FROM users u
             JOIN orders o ON u.id = o.user_id
             GROUP BY u.name
             ORDER BY total DESC"
        ).unwrap();

        assert_eq!(result.num_rows(), 3);
        assert_eq!(result.columns(), vec!["name", "total"]);
    }

    #[test]
    fn test_sql_empty_result() {
        let df = sample_dataframe();
        let result = sql_query(&df, "SELECT * FROM df WHERE age > 100").unwrap();
        assert!(result.is_empty());
        assert_eq!(result.num_columns(), 3);
    }
}
