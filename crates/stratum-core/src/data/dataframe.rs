//! DataFrame: A columnar data structure backed by Apache Arrow

use std::fmt;
use std::sync::Arc;

use std::collections::HashSet;

use arrow::array::{RecordBatch, UInt32Array};
use arrow::compute::{lexsort_to_indices, take, SortColumn, SortOptions};
use arrow::datatypes::{Field, Schema, SchemaRef};

use super::error::{DataError, DataResult};
use super::series::Series;
use crate::bytecode::Value;

/// A DataFrame is a two-dimensional, column-oriented data structure
/// backed by Apache Arrow for high-performance operations.
#[derive(Clone)]
pub struct DataFrame {
    /// The Arrow schema (column names and types)
    schema: SchemaRef,
    /// The data as Arrow RecordBatches
    batches: Vec<RecordBatch>,
}

impl DataFrame {
    /// Create an empty DataFrame with a schema
    #[must_use]
    pub fn empty(schema: SchemaRef) -> Self {
        Self {
            schema,
            batches: Vec::new(),
        }
    }

    /// Create a DataFrame from a single RecordBatch
    #[must_use]
    pub fn from_batch(batch: RecordBatch) -> Self {
        let schema = batch.schema();
        Self {
            schema,
            batches: vec![batch],
        }
    }

    /// Create a DataFrame from multiple RecordBatches
    ///
    /// # Errors
    /// Returns error if batches have incompatible schemas
    pub fn from_batches(schema: SchemaRef, batches: Vec<RecordBatch>) -> DataResult<Self> {
        // Validate that all batches match the schema
        for (i, batch) in batches.iter().enumerate() {
            if batch.schema() != schema {
                return Err(DataError::SchemaMismatch(format!(
                    "batch {i} has incompatible schema"
                )));
            }
        }
        Ok(Self { schema, batches })
    }

    /// Create a DataFrame from a vector of Series
    ///
    /// # Errors
    /// Returns error if series have different lengths
    pub fn from_series(columns: Vec<Series>) -> DataResult<Self> {
        if columns.is_empty() {
            let schema = Arc::new(Schema::empty());
            return Ok(Self::empty(schema));
        }

        // Check all columns have the same length
        let len = columns[0].len();
        for col in &columns {
            if col.len() != len {
                return Err(DataError::SchemaMismatch(format!(
                    "column '{}' has {} rows, expected {}",
                    col.name(),
                    col.len(),
                    len
                )));
            }
        }

        // Build schema and arrays
        let fields: Vec<Field> = columns
            .iter()
            .map(|s| Field::new(s.name(), s.data_type().clone(), true))
            .collect();

        let schema = Arc::new(Schema::new(fields));

        let arrays: Vec<_> = columns.iter().map(|s| s.array().clone()).collect();

        let batch = RecordBatch::try_new(schema.clone(), arrays)?;

        Ok(Self {
            schema,
            batches: vec![batch],
        })
    }

    /// Get the schema
    #[must_use]
    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }

    /// Get column names
    #[must_use]
    pub fn columns(&self) -> Vec<String> {
        self.schema.fields().iter().map(|f| f.name().clone()).collect()
    }

    /// Get the number of columns
    #[must_use]
    pub fn num_columns(&self) -> usize {
        self.schema.fields().len()
    }

    /// Get the number of rows
    #[must_use]
    pub fn num_rows(&self) -> usize {
        self.batches.iter().map(RecordBatch::num_rows).sum()
    }

    /// Check if the DataFrame is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.num_rows() == 0
    }

    /// Get a column by name as a Series
    ///
    /// # Errors
    /// Returns error if column not found
    pub fn column(&self, name: &str) -> DataResult<Series> {
        let idx = self
            .schema
            .index_of(name)
            .map_err(|_| DataError::ColumnNotFound(name.to_string()))?;
        self.column_by_index(idx)
    }

    /// Get a column by index as a Series
    ///
    /// # Errors
    /// Returns error if index is out of bounds
    pub fn column_by_index(&self, index: usize) -> DataResult<Series> {
        if index >= self.num_columns() {
            return Err(DataError::InvalidColumnIndex(index));
        }

        let field = self.schema.field(index);
        let name = field.name().clone();

        // Concatenate arrays from all batches
        if self.batches.is_empty() {
            // Return empty series with correct type
            let array = arrow::array::new_empty_array(field.data_type());
            return Ok(Series::new(name, array));
        }

        if self.batches.len() == 1 {
            let array = self.batches[0].column(index).clone();
            return Ok(Series::new(name, array));
        }

        // Multiple batches - need to concatenate
        let arrays: Vec<_> = self.batches.iter().map(|b| b.column(index).as_ref()).collect();
        let concatenated = arrow::compute::concat(&arrays)?;
        Ok(Series::new(name, concatenated))
    }

    /// Get the first n rows
    ///
    /// # Errors
    /// Returns error if slicing fails
    pub fn head(&self, n: usize) -> DataResult<Self> {
        let total_rows = self.num_rows();
        let take_rows = n.min(total_rows);

        if take_rows == 0 {
            return Ok(Self::empty(self.schema.clone()));
        }

        let mut remaining = take_rows;
        let mut new_batches = Vec::new();

        for batch in &self.batches {
            if remaining == 0 {
                break;
            }

            let batch_rows = batch.num_rows();
            if batch_rows <= remaining {
                new_batches.push(batch.clone());
                remaining -= batch_rows;
            } else {
                // Slice this batch
                let sliced = batch.slice(0, remaining);
                new_batches.push(sliced);
                remaining = 0;
            }
        }

        Ok(Self {
            schema: self.schema.clone(),
            batches: new_batches,
        })
    }

    /// Get the last n rows
    ///
    /// # Errors
    /// Returns error if slicing fails
    pub fn tail(&self, n: usize) -> DataResult<Self> {
        let total_rows = self.num_rows();
        let take_rows = n.min(total_rows);
        let skip_rows = total_rows.saturating_sub(take_rows);

        if take_rows == 0 {
            return Ok(Self::empty(self.schema.clone()));
        }

        let mut skipped = 0;
        let mut new_batches = Vec::new();

        for batch in &self.batches {
            let batch_rows = batch.num_rows();

            if skipped + batch_rows <= skip_rows {
                // Skip this entire batch
                skipped += batch_rows;
                continue;
            }

            if skipped < skip_rows {
                // Partial skip
                let skip_in_batch = skip_rows - skipped;
                let take_in_batch = batch_rows - skip_in_batch;
                let sliced = batch.slice(skip_in_batch, take_in_batch);
                new_batches.push(sliced);
                skipped = skip_rows;
            } else {
                // Take entire batch
                new_batches.push(batch.clone());
            }
        }

        Ok(Self {
            schema: self.schema.clone(),
            batches: new_batches,
        })
    }

    /// Get the underlying RecordBatches
    #[must_use]
    pub fn batches(&self) -> &[RecordBatch] {
        &self.batches
    }

    /// Iterate over rows, returning each row as a Map<String, Value>
    ///
    /// This creates a new Map for each row, so it's not as efficient as
    /// column-oriented operations, but is useful for row-by-row processing.
    pub fn iter_rows(&self) -> impl Iterator<Item = DataResult<Value>> + '_ {
        let col_names = self.columns();
        let num_cols = col_names.len();

        // Pre-fetch all column series
        let columns: Vec<_> = (0..num_cols)
            .map(|i| self.column_by_index(i))
            .collect();

        (0..self.num_rows()).map(move |row_idx| {
            use std::cell::RefCell;
            use std::collections::HashMap;
            use std::rc::Rc;

            let mut row_map = HashMap::new();
            for (col_idx, col_name) in col_names.iter().enumerate() {
                let col = columns[col_idx].as_ref().map_err(|e| e.clone())?;
                let val = col.get(row_idx)?;
                let key = crate::bytecode::HashableValue::String(Rc::new(col_name.clone()));
                row_map.insert(key, val);
            }
            Ok(Value::Map(Rc::new(RefCell::new(row_map))))
        })
    }

    /// Iterate over columns, returning each column as a Series
    pub fn iter_columns(&self) -> impl Iterator<Item = DataResult<Series>> + '_ {
        (0..self.num_columns()).map(move |i| self.column_by_index(i))
    }

    /// Get a random sample of n rows
    ///
    /// # Errors
    /// Returns error if sampling fails
    pub fn sample(&self, n: usize) -> DataResult<Self> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let total_rows = self.num_rows();
        if n >= total_rows {
            return Ok(self.clone());
        }

        // Generate random indices
        let mut indices: Vec<usize> = (0..total_rows).collect();
        indices.shuffle(&mut thread_rng());
        indices.truncate(n);
        indices.sort_unstable(); // Keep rows in original order

        // Build new columns by selecting the sampled rows
        let mut new_columns = Vec::new();
        for col_idx in 0..self.num_columns() {
            let col = self.column_by_index(col_idx)?;
            let values: Vec<Value> = indices
                .iter()
                .map(|&idx| col.get(idx))
                .collect::<DataResult<Vec<_>>>()?;

            // Create new series from sampled values
            let new_series = Series::from_values(col.name(), &values)?;
            new_columns.push(new_series);
        }

        DataFrame::from_series(new_columns)
    }

    /// Select specific columns by name
    ///
    /// # Errors
    /// Returns error if any column is not found
    pub fn select(&self, columns: &[&str]) -> DataResult<Self> {
        let series: Result<Vec<_>, _> = columns.iter().map(|name| self.column(name)).collect();
        DataFrame::from_series(series?)
    }

    /// Drop columns by name
    ///
    /// # Errors
    /// Returns error if resulting DataFrame has no columns
    pub fn drop(&self, columns: &[&str]) -> DataResult<Self> {
        let all_columns = self.columns();
        let keep: Vec<&str> = all_columns
            .iter()
            .filter(|name| !columns.contains(&name.as_str()))
            .map(String::as_str)
            .collect();

        if keep.is_empty() {
            return Err(DataError::EmptyData);
        }

        self.select(&keep)
    }

    /// Rename a column
    ///
    /// # Errors
    /// Returns error if column not found
    pub fn rename_column(&self, old_name: &str, new_name: &str) -> DataResult<Self> {
        let mut series_list = Vec::new();
        for col_name in self.columns() {
            let series = self.column(&col_name)?;
            if col_name == old_name {
                series_list.push(series.rename(new_name));
            } else {
                series_list.push(series);
            }
        }
        DataFrame::from_series(series_list)
    }

    /// Sort the DataFrame by one or more columns
    ///
    /// Each column can have a descending flag. If descending is not provided,
    /// it defaults to ascending (false).
    ///
    /// # Errors
    /// Returns error if any column is not found
    pub fn sort_by(&self, columns: &[(&str, bool)]) -> DataResult<Self> {
        if columns.is_empty() {
            return Ok(self.clone());
        }

        if self.is_empty() {
            return Ok(self.clone());
        }

        // Build sort columns
        let mut sort_columns = Vec::with_capacity(columns.len());
        for (col_name, descending) in columns {
            let series = self.column(col_name)?;
            sort_columns.push(SortColumn {
                values: series.array().clone(),
                options: Some(SortOptions {
                    descending: *descending,
                    nulls_first: false,
                }),
            });
        }

        // Get sorted indices
        let indices: UInt32Array = lexsort_to_indices(&sort_columns, None)?;

        // Use indices to reorder all columns
        let mut new_columns = Vec::with_capacity(self.num_columns());
        for col_idx in 0..self.num_columns() {
            let series = self.column_by_index(col_idx)?;
            let sorted_array = take(series.array(), &indices, None)?;
            new_columns.push(Series::new(series.name(), sorted_array));
        }

        DataFrame::from_series(new_columns)
    }

    /// Take the first n rows (alias for head)
    ///
    /// # Errors
    /// Returns error if slicing fails
    pub fn take_rows(&self, n: usize) -> DataResult<Self> {
        self.head(n)
    }

    /// Get distinct/unique rows based on all columns
    ///
    /// # Errors
    /// Returns error if operation fails
    pub fn distinct(&self) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        // Build a key from all column values for each row
        let num_rows = self.num_rows();
        let columns: Vec<Series> = (0..self.num_columns())
            .map(|i| self.column_by_index(i))
            .collect::<DataResult<Vec<_>>>()?;

        let mut seen: HashSet<String> = HashSet::new();
        let mut unique_indices = Vec::new();

        for row_idx in 0..num_rows {
            // Build key from all column values
            let key: String = columns
                .iter()
                .map(|col| Self::value_to_key_string(&col.get(row_idx).unwrap_or(Value::Null)))
                .collect::<Vec<_>>()
                .join("|");

            if seen.insert(key) {
                unique_indices.push(row_idx);
            }
        }

        self.filter_by_indices(&unique_indices)
    }

    /// Get distinct/unique rows based on specific columns
    ///
    /// # Errors
    /// Returns error if any column is not found
    pub fn distinct_by(&self, column_names: &[&str]) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        // Validate and get column series
        let columns: Vec<Series> = column_names
            .iter()
            .map(|name| self.column(name))
            .collect::<DataResult<Vec<_>>>()?;

        let num_rows = self.num_rows();
        let mut seen: HashSet<String> = HashSet::new();
        let mut unique_indices = Vec::new();

        for row_idx in 0..num_rows {
            // Build key from specified column values only
            let key: String = columns
                .iter()
                .map(|col| Self::value_to_key_string(&col.get(row_idx).unwrap_or(Value::Null)))
                .collect::<Vec<_>>()
                .join("|");

            if seen.insert(key) {
                unique_indices.push(row_idx);
            }
        }

        self.filter_by_indices(&unique_indices)
    }

    /// Convert a Value to a string suitable for use as a hash key
    fn value_to_key_string(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f}"),
            Value::String(s) => format!("s:{s}"),
            Value::List(items) => {
                let borrowed = items.borrow();
                let inner: Vec<String> = borrowed.iter().map(Self::value_to_key_string).collect();
                format!("[{}]", inner.join(","))
            }
            Value::Map(map) => {
                let borrowed = map.borrow();
                let mut pairs: Vec<String> = borrowed
                    .iter()
                    .map(|(k, v)| format!("{:?}:{}", k, Self::value_to_key_string(v)))
                    .collect();
                pairs.sort(); // Ensure consistent ordering
                format!("{{{}}}", pairs.join(","))
            }
            _ => format!("{}", value.type_name()),
        }
    }

    /// Filter the DataFrame by row indices
    ///
    /// Returns a new DataFrame containing only the rows at the specified indices.
    ///
    /// # Errors
    /// Returns error if any index is out of bounds
    pub fn filter_by_indices(&self, indices: &[usize]) -> DataResult<Self> {
        if indices.is_empty() {
            return Ok(Self::empty(self.schema.clone()));
        }

        // Validate indices
        let num_rows = self.num_rows();
        for &idx in indices {
            if idx >= num_rows {
                return Err(DataError::OutOfBounds {
                    index: idx,
                    length: num_rows,
                });
            }
        }

        // Build new columns by selecting the specified rows
        let mut new_columns = Vec::new();
        for col_idx in 0..self.num_columns() {
            let col = self.column_by_index(col_idx)?;
            let values: Vec<Value> = indices
                .iter()
                .map(|&idx| col.get(idx))
                .collect::<DataResult<Vec<_>>>()?;

            let new_series = Series::from_values(col.name(), &values)?;
            new_columns.push(new_series);
        }

        DataFrame::from_series(new_columns)
    }

    /// Pretty print the DataFrame for display
    #[must_use]
    pub fn to_pretty_string(&self, max_rows: usize) -> String {
        use arrow::util::pretty::pretty_format_batches;

        if self.batches.is_empty() {
            return format!(
                "Empty DataFrame with columns: {:?}",
                self.columns()
            );
        }

        // Get limited batches
        let display_df = match self.head(max_rows) {
            Ok(df) => df,
            Err(_) => return "Error formatting DataFrame".to_string(),
        };

        match pretty_format_batches(&display_df.batches) {
            Ok(table) => {
                let total = self.num_rows();
                if total > max_rows {
                    format!("{table}\n... showing {max_rows} of {total} rows")
                } else {
                    table.to_string()
                }
            }
            Err(e) => format!("Error formatting: {e}"),
        }
    }
}

impl fmt::Debug for DataFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataFrame")
            .field("columns", &self.columns())
            .field("rows", &self.num_rows())
            .field("batches", &self.batches.len())
            .finish()
    }
}

impl fmt::Display for DataFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_pretty_string(20))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dataframe() -> DataFrame {
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
        let ages = Series::from_ints("age", vec![30, 25, 35]);
        let scores = Series::from_floats("score", vec![85.5, 92.0, 78.3]);

        DataFrame::from_series(vec![names, ages, scores]).unwrap()
    }

    #[test]
    fn test_from_series() {
        let df = sample_dataframe();
        assert_eq!(df.num_columns(), 3);
        assert_eq!(df.num_rows(), 3);
        assert_eq!(df.columns(), vec!["name", "age", "score"]);
    }

    #[test]
    fn test_column_access() {
        let df = sample_dataframe();

        let age_col = df.column("age").unwrap();
        assert_eq!(age_col.name(), "age");
        assert_eq!(age_col.len(), 3);
    }

    #[test]
    fn test_column_not_found() {
        let df = sample_dataframe();
        assert!(df.column("nonexistent").is_err());
    }

    #[test]
    fn test_head() {
        let df = sample_dataframe();

        let head = df.head(2).unwrap();
        assert_eq!(head.num_rows(), 2);
    }

    #[test]
    fn test_tail() {
        let df = sample_dataframe();

        let tail = df.tail(2).unwrap();
        assert_eq!(tail.num_rows(), 2);
    }

    #[test]
    fn test_select() {
        let df = sample_dataframe();

        let selected = df.select(&["name", "score"]).unwrap();
        assert_eq!(selected.num_columns(), 2);
        assert_eq!(selected.columns(), vec!["name", "score"]);
    }

    #[test]
    fn test_drop() {
        let df = sample_dataframe();

        let dropped = df.drop(&["age"]).unwrap();
        assert_eq!(dropped.num_columns(), 2);
        assert!(!dropped.columns().contains(&"age".to_string()));
    }

    #[test]
    fn test_rename() {
        let df = sample_dataframe();

        let renamed = df.rename_column("age", "years").unwrap();
        assert!(renamed.columns().contains(&"years".to_string()));
        assert!(!renamed.columns().contains(&"age".to_string()));
    }

    #[test]
    fn test_empty_dataframe() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", arrow::datatypes::DataType::Int64, true),
        ]));
        let df = DataFrame::empty(schema);
        assert!(df.is_empty());
        assert_eq!(df.num_columns(), 1);
    }

    #[test]
    fn test_sort_by_ascending() {
        let df = sample_dataframe();

        let sorted = df.sort_by(&[("age", false)]).unwrap();
        assert_eq!(sorted.num_rows(), 3);

        // Bob (25), Alice (30), Charlie (35)
        let age_col = sorted.column("age").unwrap();
        assert_eq!(age_col.get(0).unwrap(), Value::Int(25));
        assert_eq!(age_col.get(1).unwrap(), Value::Int(30));
        assert_eq!(age_col.get(2).unwrap(), Value::Int(35));
    }

    #[test]
    fn test_sort_by_descending() {
        let df = sample_dataframe();

        let sorted = df.sort_by(&[("age", true)]).unwrap();
        assert_eq!(sorted.num_rows(), 3);

        // Charlie (35), Alice (30), Bob (25)
        let age_col = sorted.column("age").unwrap();
        assert_eq!(age_col.get(0).unwrap(), Value::Int(35));
        assert_eq!(age_col.get(1).unwrap(), Value::Int(30));
        assert_eq!(age_col.get(2).unwrap(), Value::Int(25));
    }

    #[test]
    fn test_take_rows() {
        let df = sample_dataframe();

        let taken = df.take_rows(2).unwrap();
        assert_eq!(taken.num_rows(), 2);
    }

    #[test]
    fn test_distinct() {
        // Create DataFrame with duplicates
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Alice", "Charlie", "Bob"]);
        let ages = Series::from_ints("age", vec![30, 25, 30, 35, 25]);
        let df = DataFrame::from_series(vec![names, ages]).unwrap();

        let distinct = df.distinct().unwrap();
        assert_eq!(distinct.num_rows(), 3); // Alice/30, Bob/25, Charlie/35
    }

    #[test]
    fn test_distinct_by() {
        // Create DataFrame with partial duplicates
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Alice", "Charlie"]);
        let ages = Series::from_ints("age", vec![30, 25, 31, 35]);
        let df = DataFrame::from_series(vec![names, ages]).unwrap();

        // Distinct by name only - should get 3 rows (first Alice, Bob, Charlie)
        let distinct = df.distinct_by(&["name"]).unwrap();
        assert_eq!(distinct.num_rows(), 3);

        // Verify first Alice (age 30) is kept, not second (age 31)
        let ages_col = distinct.column("age").unwrap();
        assert_eq!(ages_col.get(0).unwrap(), Value::Int(30));
    }
}
