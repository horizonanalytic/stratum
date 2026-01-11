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
        self.schema
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect()
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

    /// Get memory usage statistics for this DataFrame
    ///
    /// Returns statistics including total bytes used, bytes per row, etc.
    #[must_use]
    pub fn memory_usage(&self) -> super::memory::MemoryStats {
        use arrow::array::Array;

        let num_rows = self.num_rows();
        let num_columns = self.num_columns();

        // Calculate data bytes from Arrow arrays
        let data_bytes: usize = self
            .batches
            .iter()
            .map(|batch| {
                batch
                    .columns()
                    .iter()
                    .map(|col| col.get_array_memory_size())
                    .sum::<usize>()
            })
            .sum();

        // Estimate total bytes including schema overhead
        let schema_overhead = self.schema.fields().len() * 64; // Rough estimate per field
        let batch_overhead = self.batches.len() * 48; // Vec overhead per batch
        let total_bytes = data_bytes + schema_overhead + batch_overhead;

        super::memory::MemoryStats::new(num_rows, num_columns, data_bytes, total_bytes)
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
        let arrays: Vec<_> = self
            .batches
            .iter()
            .map(|b| b.column(index).as_ref())
            .collect();
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
        let columns: Vec<_> = (0..num_cols).map(|i| self.column_by_index(i)).collect();

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
    /// Automatically uses parallel processing for large DataFrames.
    ///
    /// # Errors
    /// Returns error if any index is out of bounds
    pub fn filter_by_indices(&self, indices: &[usize]) -> DataResult<Self> {
        use super::parallel::should_parallelize;

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

        // Use parallel processing if beneficial
        if should_parallelize(indices.len() * self.num_columns()) {
            self.filter_by_indices_parallel(indices)
        } else {
            self.filter_by_indices_sequential(indices)
        }
    }

    /// Sequential filter implementation
    fn filter_by_indices_sequential(&self, indices: &[usize]) -> DataResult<Self> {
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

    /// Parallel filter implementation using Rayon
    fn filter_by_indices_parallel(&self, indices: &[usize]) -> DataResult<Self> {
        use rayon::prelude::*;

        // Get all columns first
        let columns: Vec<Series> = (0..self.num_columns())
            .map(|i| self.column_by_index(i))
            .collect::<DataResult<Vec<_>>>()?;

        // Process columns in parallel
        let new_columns: Result<Vec<Series>, DataError> = columns
            .into_par_iter()
            .map(|col| {
                let values: Vec<Value> = indices
                    .iter()
                    .map(|&idx| col.get(idx))
                    .collect::<DataResult<Vec<_>>>()?;
                Series::from_values(col.name(), &values)
            })
            .collect();

        DataFrame::from_series(new_columns?)
    }

    /// Generate summary statistics for numeric columns
    ///
    /// Returns a DataFrame with statistics: count, mean, std, min, 25%, 50%, 75%, max
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn describe(&self) -> DataResult<Self> {
        // Find numeric columns
        let numeric_cols: Vec<(String, Series)> = (0..self.num_columns())
            .filter_map(|i| {
                let series = self.column_by_index(i).ok()?;
                if series.is_numeric() {
                    Some((series.name().to_string(), series))
                } else {
                    None
                }
            })
            .collect();

        if numeric_cols.is_empty() {
            // Return empty DataFrame with statistic column
            let stat_series = Series::from_strings(
                "statistic",
                vec!["count", "mean", "std", "min", "25%", "50%", "75%", "max"],
            );
            return DataFrame::from_series(vec![stat_series]);
        }

        // Build result columns
        let stat_names = vec!["count", "mean", "std", "min", "25%", "50%", "75%", "max"];
        let stat_series = Series::from_strings("statistic", stat_names.clone());

        let mut result_columns = vec![stat_series];

        for (col_name, series) in &numeric_cols {
            let count = series.count() as f64;
            let mean = match series.mean()? {
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };
            let std = match series.std()? {
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };
            let min_val = match series.min()? {
                Value::Int(i) => i as f64,
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };
            let q25 = match series.quantile(0.25)? {
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };
            let q50 = match series.quantile(0.50)? {
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };
            let q75 = match series.quantile(0.75)? {
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };
            let max_val = match series.max()? {
                Value::Int(i) => i as f64,
                Value::Float(f) => f,
                Value::Null => f64::NAN,
                _ => f64::NAN,
            };

            let values = vec![count, mean, std, min_val, q25, q50, q75, max_val];
            let col_series = Series::from_floats(col_name, values);
            result_columns.push(col_series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Calculate correlation matrix for numeric columns
    ///
    /// Returns a DataFrame with Pearson correlation coefficients between all pairs
    /// of numeric columns.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn corr(&self) -> DataResult<Self> {
        // Find numeric columns
        let numeric_cols: Vec<(String, Series)> = (0..self.num_columns())
            .filter_map(|i| {
                let series = self.column_by_index(i).ok()?;
                if series.is_numeric() {
                    Some((series.name().to_string(), series))
                } else {
                    None
                }
            })
            .collect();

        if numeric_cols.is_empty() {
            return Ok(Self::empty(self.schema.clone()));
        }

        let col_names: Vec<&str> = numeric_cols.iter().map(|(n, _)| n.as_str()).collect();
        let n = numeric_cols.len();

        // First column is the column names
        let name_series = Series::from_strings("column", col_names.clone());
        let mut result_columns = vec![name_series];

        // Calculate correlations
        for i in 0..n {
            let mut corr_values: Vec<f64> = Vec::with_capacity(n);

            for j in 0..n {
                let corr = Self::pearson_correlation(&numeric_cols[i].1, &numeric_cols[j].1)?;
                corr_values.push(corr);
            }

            let corr_series = Series::from_floats(&numeric_cols[i].0, corr_values);
            result_columns.push(corr_series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Calculate Pearson correlation between two series
    fn pearson_correlation(s1: &Series, s2: &Series) -> DataResult<f64> {
        let mean1 = match s1.mean()? {
            Value::Float(f) => f,
            _ => return Ok(f64::NAN),
        };
        let mean2 = match s2.mean()? {
            Value::Float(f) => f,
            _ => return Ok(f64::NAN),
        };

        let std1 = match s1.std()? {
            Value::Float(f) if f > 0.0 => f,
            _ => return Ok(f64::NAN),
        };
        let std2 = match s2.std()? {
            Value::Float(f) if f > 0.0 => f,
            _ => return Ok(f64::NAN),
        };

        let n = s1.len().min(s2.len());
        let mut sum_product: f64 = 0.0;
        let mut count: usize = 0;

        for i in 0..n {
            let v1 = s1.get(i)?;
            let v2 = s2.get(i)?;

            let f1 = match v1 {
                Value::Int(x) => x as f64,
                Value::Float(x) => x,
                Value::Null => continue,
                _ => continue,
            };
            let f2 = match v2 {
                Value::Int(x) => x as f64,
                Value::Float(x) => x,
                Value::Null => continue,
                _ => continue,
            };

            sum_product += (f1 - mean1) * (f2 - mean2);
            count += 1;
        }

        if count == 0 {
            return Ok(f64::NAN);
        }

        #[allow(clippy::cast_precision_loss)]
        let covariance = sum_product / count as f64;
        Ok(covariance / (std1 * std2))
    }

    /// Calculate covariance matrix for numeric columns
    ///
    /// Returns a DataFrame with covariance values between all pairs
    /// of numeric columns.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn cov(&self) -> DataResult<Self> {
        // Find numeric columns
        let numeric_cols: Vec<(String, Series)> = (0..self.num_columns())
            .filter_map(|i| {
                let series = self.column_by_index(i).ok()?;
                if series.is_numeric() {
                    Some((series.name().to_string(), series))
                } else {
                    None
                }
            })
            .collect();

        if numeric_cols.is_empty() {
            return Ok(Self::empty(self.schema.clone()));
        }

        let col_names: Vec<&str> = numeric_cols.iter().map(|(n, _)| n.as_str()).collect();
        let n = numeric_cols.len();

        // First column is the column names
        let name_series = Series::from_strings("column", col_names.clone());
        let mut result_columns = vec![name_series];

        // Calculate covariances
        for i in 0..n {
            let mut cov_values: Vec<f64> = Vec::with_capacity(n);

            for j in 0..n {
                let cov_val = Self::covariance(&numeric_cols[i].1, &numeric_cols[j].1)?;
                cov_values.push(cov_val);
            }

            let cov_series = Series::from_floats(&numeric_cols[i].0, cov_values);
            result_columns.push(cov_series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Calculate covariance between two series
    fn covariance(s1: &Series, s2: &Series) -> DataResult<f64> {
        let mean1 = match s1.mean()? {
            Value::Float(f) => f,
            _ => return Ok(f64::NAN),
        };
        let mean2 = match s2.mean()? {
            Value::Float(f) => f,
            _ => return Ok(f64::NAN),
        };

        let n = s1.len().min(s2.len());
        let mut sum_product: f64 = 0.0;
        let mut count: usize = 0;

        for i in 0..n {
            let v1 = s1.get(i)?;
            let v2 = s2.get(i)?;

            let f1 = match v1 {
                Value::Int(x) => x as f64,
                Value::Float(x) => x,
                Value::Null => continue,
                _ => continue,
            };
            let f2 = match v2 {
                Value::Int(x) => x as f64,
                Value::Float(x) => x,
                Value::Null => continue,
                _ => continue,
            };

            sum_product += (f1 - mean1) * (f2 - mean2);
            count += 1;
        }

        if count == 0 {
            return Ok(f64::NAN);
        }

        #[allow(clippy::cast_precision_loss)]
        Ok(sum_product / count as f64)
    }

    /// Count the frequency of values in a column
    ///
    /// Returns a DataFrame with two columns: the unique values and their counts,
    /// sorted by count in descending order.
    ///
    /// # Errors
    /// Returns error if the column is not found
    pub fn value_counts(&self, column: &str) -> DataResult<Self> {
        use std::collections::HashMap;

        let series = self.column(column)?;
        let mut counts: HashMap<String, (Value, i64)> = HashMap::new();

        for i in 0..series.len() {
            let val = series.get(i)?;
            let key = Self::value_to_key_string(&val);
            counts
                .entry(key)
                .and_modify(|(_, c)| *c += 1)
                .or_insert((val, 1));
        }

        // Sort by count descending
        let mut sorted: Vec<_> = counts.into_values().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        // Build result columns
        let values: Vec<Value> = sorted.iter().map(|(v, _)| v.clone()).collect();
        let count_values: Vec<i64> = sorted.iter().map(|(_, c)| *c).collect();

        let value_series = Series::from_values(column, &values)?;
        let count_series = Series::from_ints("count", count_values);

        DataFrame::from_series(vec![value_series, count_series])
    }

    /// Pretty print the DataFrame for display
    #[must_use]
    pub fn to_pretty_string(&self, max_rows: usize) -> String {
        use arrow::util::pretty::pretty_format_batches;

        if self.batches.is_empty() {
            return format!("Empty DataFrame with columns: {:?}", self.columns());
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

    // ========================================================================
    // Missing Data Handling
    // ========================================================================

    /// Drop rows containing any null values
    ///
    /// Returns a new DataFrame with all rows that contain at least one null
    /// value removed.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn dropna(&self) -> DataResult<Self> {
        self.dropna_columns(&self.columns())
    }

    /// Drop rows containing null values in specified columns
    ///
    /// Only considers the specified columns when determining which rows to drop.
    ///
    /// # Arguments
    /// * `columns` - Column names to check for null values
    ///
    /// # Errors
    /// Returns error if any column is not found or operation fails
    pub fn dropna_columns(&self, columns: &[String]) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        // Get series for specified columns
        let check_series: Vec<Series> = columns
            .iter()
            .map(|name| self.column(name))
            .collect::<DataResult<Vec<_>>>()?;

        // If no nulls in any column, return self
        let has_any_nulls = check_series.iter().any(|s| s.null_count() > 0);
        if !has_any_nulls {
            return Ok(self.clone());
        }

        // Find indices of rows that have no nulls in the specified columns
        let num_rows = self.num_rows();
        let non_null_indices: Vec<usize> = (0..num_rows)
            .filter(|&row_idx| check_series.iter().all(|series| !series.is_null(row_idx)))
            .collect();

        if non_null_indices.is_empty() {
            return Ok(Self::empty(self.schema.clone()));
        }

        self.filter_by_indices(&non_null_indices)
    }

    /// Fill null values with a constant value across all columns
    ///
    /// The fill value is applied to all columns where the type is compatible.
    /// Columns with incompatible types are left unchanged.
    ///
    /// # Arguments
    /// * `fill_value` - The value to use for filling nulls
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn fillna(&self, fill_value: &Value) -> DataResult<Self> {
        let mut new_columns = Vec::with_capacity(self.num_columns());

        for col_idx in 0..self.num_columns() {
            let series = self.column_by_index(col_idx)?;
            // Try to fill with the value; if type doesn't match, keep original
            let filled = series.fillna(fill_value).unwrap_or_else(|_| series);
            new_columns.push(filled);
        }

        DataFrame::from_series(new_columns)
    }

    /// Fill null values with column-specific values
    ///
    /// Each entry in the map specifies a fill value for a specific column.
    /// Columns not in the map are left unchanged.
    ///
    /// # Arguments
    /// * `column_values` - A map of column names to fill values
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn fillna_map(
        &self,
        column_values: &std::collections::HashMap<String, Value>,
    ) -> DataResult<Self> {
        let mut new_columns = Vec::with_capacity(self.num_columns());

        for col_name in self.columns() {
            let series = self.column(&col_name)?;
            let filled = if let Some(fill_value) = column_values.get(&col_name) {
                series.fillna(fill_value).unwrap_or_else(|_| series)
            } else {
                series
            };
            new_columns.push(filled);
        }

        DataFrame::from_series(new_columns)
    }

    /// Fill null values using forward fill (propagate last valid value)
    ///
    /// For each column, null values are replaced with the last non-null value
    /// in that column. Nulls at the start of a column remain null.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn fillna_forward(&self) -> DataResult<Self> {
        let mut new_columns = Vec::with_capacity(self.num_columns());

        for col_idx in 0..self.num_columns() {
            let series = self.column_by_index(col_idx)?;
            let filled = series.fillna_forward().unwrap_or_else(|_| series);
            new_columns.push(filled);
        }

        DataFrame::from_series(new_columns)
    }

    /// Fill null values using backward fill (propagate next valid value)
    ///
    /// For each column, null values are replaced with the next non-null value
    /// in that column. Nulls at the end of a column remain null.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn fillna_backward(&self) -> DataResult<Self> {
        let mut new_columns = Vec::with_capacity(self.num_columns());

        for col_idx in 0..self.num_columns() {
            let series = self.column_by_index(col_idx)?;
            let filled = series.fillna_backward().unwrap_or_else(|_| series);
            new_columns.push(filled);
        }

        DataFrame::from_series(new_columns)
    }

    // ========================================================================
    // Reshape Operations
    // ========================================================================

    /// Transpose the DataFrame (swap rows and columns)
    ///
    /// The first column becomes the new column names, and each row becomes a column.
    /// If no column can be used as names, generates default column names (col_0, col_1, ...).
    /// All values are converted to strings to handle mixed types from different columns.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn transpose(&self) -> DataResult<Self> {
        if self.is_empty() || self.num_columns() == 0 {
            return Ok(self.clone());
        }

        let num_rows = self.num_rows();
        let col_names = self.columns();

        // Create the "column" column with original column names
        let column_name_series =
            Series::from_strings("column", col_names.iter().map(|s| s.as_str()).collect());

        let mut result_columns = vec![column_name_series];

        // Each original row becomes a new column
        // Convert all values to strings since columns may have different types
        for row_idx in 0..num_rows {
            let col_name = format!("row_{}", row_idx);
            let mut row_values: Vec<String> = Vec::with_capacity(self.num_columns());

            for col_idx in 0..self.num_columns() {
                let series = self.column_by_index(col_idx)?;
                let val = series.get(row_idx)?;
                row_values.push(Self::value_to_display_string(&val));
            }

            let row_series =
                Series::from_strings(&col_name, row_values.iter().map(|s| s.as_str()).collect());
            result_columns.push(row_series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Convert a Value to a display string
    fn value_to_display_string(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f}"),
            Value::String(s) => s.to_string(),
            _ => format!("{}", value.type_name()),
        }
    }

    /// Explode a list column into multiple rows
    ///
    /// Each element in the list column becomes a separate row, with other columns
    /// duplicated for each element.
    ///
    /// # Arguments
    /// * `column` - The name of the column containing lists to explode
    ///
    /// # Errors
    /// Returns error if column not found or column doesn't contain lists
    pub fn explode(&self, column: &str) -> DataResult<Self> {
        let list_col = self.column(column)?;
        let num_rows = self.num_rows();
        let other_columns = self.columns();

        // Collect all columns except the explode column
        let other_series: Vec<Series> = other_columns
            .iter()
            .filter(|name| name.as_str() != column)
            .map(|name| self.column(name))
            .collect::<DataResult<Vec<_>>>()?;

        // Build new row indices and exploded values
        let mut exploded_values: Vec<Value> = Vec::new();
        let mut row_indices: Vec<usize> = Vec::new();

        for row_idx in 0..num_rows {
            let val = list_col.get(row_idx)?;
            match val {
                Value::List(items) => {
                    let borrowed = items.borrow();
                    if borrowed.is_empty() {
                        // Empty list - include one row with null
                        exploded_values.push(Value::Null);
                        row_indices.push(row_idx);
                    } else {
                        for item in borrowed.iter() {
                            exploded_values.push(item.clone());
                            row_indices.push(row_idx);
                        }
                    }
                }
                Value::Null => {
                    // Null value - include one row with null
                    exploded_values.push(Value::Null);
                    row_indices.push(row_idx);
                }
                _ => {
                    // Non-list value - keep as single row
                    exploded_values.push(val);
                    row_indices.push(row_idx);
                }
            }
        }

        // Build result columns
        let mut result_columns: Vec<Series> = Vec::new();

        // Add other columns (duplicated based on row_indices)
        for series in &other_series {
            let values: Vec<Value> = row_indices
                .iter()
                .map(|&idx| series.get(idx))
                .collect::<DataResult<Vec<_>>>()?;
            let new_series = Series::from_values(series.name(), &values)?;
            result_columns.push(new_series);
        }

        // Add the exploded column
        let exploded_series = Series::from_values(column, &exploded_values)?;
        result_columns.push(exploded_series);

        DataFrame::from_series(result_columns)
    }

    /// Melt (unpivot) a DataFrame from wide to long format
    ///
    /// Transforms the DataFrame so that value columns become rows.
    ///
    /// # Arguments
    /// * `id_vars` - Columns to use as identifier variables (kept as-is)
    /// * `value_vars` - Columns to unpivot (if empty, uses all non-id columns)
    /// * `var_name` - Name for the variable column (default: "variable")
    /// * `value_name` - Name for the value column (default: "value")
    ///
    /// # Errors
    /// Returns error if any column is not found
    pub fn melt(
        &self,
        id_vars: &[&str],
        value_vars: &[&str],
        var_name: Option<&str>,
        value_name: Option<&str>,
    ) -> DataResult<Self> {
        let var_col_name = var_name.unwrap_or("variable");
        let val_col_name = value_name.unwrap_or("value");

        // If value_vars is empty, use all columns not in id_vars
        let value_columns: Vec<&str> = if value_vars.is_empty() {
            self.columns()
                .into_iter()
                .filter(|c| !id_vars.contains(&c.as_str()))
                .map(|c| {
                    // We need to leak the string to get a &'static str
                    // This is safe because we only use it within this function
                    Box::leak(c.into_boxed_str()) as &str
                })
                .collect()
        } else {
            value_vars.to_vec()
        };

        if value_columns.is_empty() {
            return Err(DataError::EmptyData);
        }

        // Validate columns exist
        for col in id_vars {
            if self.column(col).is_err() {
                return Err(DataError::ColumnNotFound(col.to_string()));
            }
        }
        for col in &value_columns {
            if self.column(col).is_err() {
                return Err(DataError::ColumnNotFound(col.to_string()));
            }
        }

        let num_rows = self.num_rows();
        let num_value_cols = value_columns.len();
        let total_rows = num_rows * num_value_cols;

        // Get id column series
        let id_series: Vec<Series> = id_vars
            .iter()
            .map(|name| self.column(name))
            .collect::<DataResult<Vec<_>>>()?;

        // Build result columns
        let mut result_columns: Vec<Series> = Vec::new();

        // Replicate id columns for each value column
        for series in &id_series {
            let mut values: Vec<Value> = Vec::with_capacity(total_rows);
            for _ in 0..num_value_cols {
                for row_idx in 0..num_rows {
                    values.push(series.get(row_idx)?);
                }
            }
            let new_series = Series::from_values(series.name(), &values)?;
            result_columns.push(new_series);
        }

        // Build the variable column (names of value columns repeated)
        let mut var_values: Vec<Value> = Vec::with_capacity(total_rows);
        for col_name in &value_columns {
            for _ in 0..num_rows {
                var_values.push(Value::string(*col_name));
            }
        }
        let var_series = Series::from_values(var_col_name, &var_values)?;
        result_columns.push(var_series);

        // Build the value column (actual values from each value column)
        let mut val_values: Vec<Value> = Vec::with_capacity(total_rows);
        for col_name in &value_columns {
            let col = self.column(col_name)?;
            for row_idx in 0..num_rows {
                val_values.push(col.get(row_idx)?);
            }
        }
        let val_series = Series::from_values(val_col_name, &val_values)?;
        result_columns.push(val_series);

        DataFrame::from_series(result_columns)
    }

    /// Stack columns into rows (similar to melt but creates a multi-index style result)
    ///
    /// Converts columns into rows, creating a "column" column with the original column
    /// names and a "value" column with the values. Values are converted to strings
    /// to handle mixed types from different columns.
    ///
    /// # Arguments
    /// * `columns` - Columns to stack (if empty, stacks all columns)
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn stack(&self, columns: &[&str]) -> DataResult<Self> {
        let cols_to_stack: Vec<String> = if columns.is_empty() {
            self.columns()
        } else {
            columns.iter().map(|s| s.to_string()).collect()
        };

        if cols_to_stack.is_empty() {
            return Err(DataError::EmptyData);
        }

        let num_rows = self.num_rows();
        let total_rows = num_rows * cols_to_stack.len();

        // Build row index column
        let mut row_indices: Vec<i64> = Vec::with_capacity(total_rows);
        for col_idx in 0..cols_to_stack.len() {
            for row_idx in 0..num_rows {
                let _ = col_idx; // Used for iteration count only
                row_indices.push(row_idx as i64);
            }
        }
        let row_index_series = Series::from_ints("row", row_indices);

        // Build column name column
        let mut col_names: Vec<&str> = Vec::with_capacity(total_rows);
        for col_name in &cols_to_stack {
            for _ in 0..num_rows {
                col_names.push(col_name.as_str());
            }
        }
        let col_name_series = Series::from_strings("column", col_names);

        // Build value column - convert to strings to handle mixed types
        let mut values: Vec<String> = Vec::with_capacity(total_rows);
        for col_name in &cols_to_stack {
            let col = self.column(col_name)?;
            for row_idx in 0..num_rows {
                let val = col.get(row_idx)?;
                values.push(Self::value_to_display_string(&val));
            }
        }
        let value_series =
            Series::from_strings("value", values.iter().map(|s| s.as_str()).collect());

        DataFrame::from_series(vec![row_index_series, col_name_series, value_series])
    }

    /// Unstack rows into columns (inverse of stack)
    ///
    /// Pivots unique values from a column into new columns.
    ///
    /// # Arguments
    /// * `index_col` - Column to use as the row index
    /// * `column_col` - Column whose unique values become new column names
    /// * `value_col` - Column containing the values to spread
    ///
    /// # Errors
    /// Returns error if columns not found or operation fails
    pub fn unstack(&self, index_col: &str, column_col: &str, value_col: &str) -> DataResult<Self> {
        use std::collections::HashMap;

        // Validate columns exist
        let index_series = self.column(index_col)?;
        let column_series = self.column(column_col)?;
        let value_series = self.column(value_col)?;

        // Get unique values for index and columns
        let mut index_values: Vec<Value> = Vec::new();
        let mut index_map: HashMap<String, usize> = HashMap::new();

        for i in 0..self.num_rows() {
            let val = index_series.get(i)?;
            let key = Self::value_to_key_string(&val);
            if !index_map.contains_key(&key) {
                index_map.insert(key, index_values.len());
                index_values.push(val);
            }
        }

        let mut col_values: Vec<String> = Vec::new();
        let mut col_map: HashMap<String, usize> = HashMap::new();

        for i in 0..self.num_rows() {
            let val = column_series.get(i)?;
            let key = Self::value_to_key_string(&val);
            if !col_map.contains_key(&key) {
                col_map.insert(key.clone(), col_values.len());
                // Use the string representation as column name
                col_values.push(match val {
                    Value::String(s) => s.to_string(),
                    _ => key,
                });
            }
        }

        // Build result matrix (index_values.len() x col_values.len())
        let num_rows = index_values.len();
        let num_cols = col_values.len();
        let mut matrix: Vec<Vec<Value>> = vec![vec![Value::Null; num_cols]; num_rows];

        // Fill in values
        for i in 0..self.num_rows() {
            let idx_val = index_series.get(i)?;
            let col_val = column_series.get(i)?;
            let val = value_series.get(i)?;

            let idx_key = Self::value_to_key_string(&idx_val);
            let col_key = Self::value_to_key_string(&col_val);

            if let (Some(&row_idx), Some(&col_idx)) =
                (index_map.get(&idx_key), col_map.get(&col_key))
            {
                matrix[row_idx][col_idx] = val;
            }
        }

        // Build result DataFrame
        let mut result_columns: Vec<Series> = Vec::new();

        // Index column
        let index_result = Series::from_values(index_col, &index_values)?;
        result_columns.push(index_result);

        // Value columns
        for (col_idx, col_name) in col_values.iter().enumerate() {
            let values: Vec<Value> = matrix.iter().map(|row| row[col_idx].clone()).collect();
            let series = Series::from_values(col_name, &values)?;
            result_columns.push(series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Create a pivot table from the DataFrame
    ///
    /// Reshapes data by using unique values from index columns as rows and
    /// unique values from column columns as new columns, with values from
    /// the values column filling the cells.
    ///
    /// # Arguments
    /// * `index` - Column to use as row index
    /// * `columns` - Column whose unique values become column headers
    /// * `values` - Column containing the values
    ///
    /// # Errors
    /// Returns error if columns not found or operation fails
    pub fn pivot(&self, index: &str, columns: &str, values: &str) -> DataResult<Self> {
        // pivot is essentially unstack with specific column names
        self.unstack(index, columns, values)
    }

    /// Create a pivot table with aggregation
    ///
    /// Similar to pivot, but aggregates values when there are duplicates.
    ///
    /// # Arguments
    /// * `index` - Column to use as row index
    /// * `columns` - Column whose unique values become column headers
    /// * `values` - Column containing the values to aggregate
    /// * `aggfunc` - Aggregation function: "sum", "mean", "min", "max", "count", "first", "last"
    ///
    /// # Errors
    /// Returns error if columns not found or operation fails
    pub fn pivot_table(
        &self,
        index: &str,
        columns: &str,
        values: &str,
        aggfunc: &str,
    ) -> DataResult<Self> {
        use std::collections::HashMap;

        // Validate columns exist
        let index_series = self.column(index)?;
        let column_series = self.column(columns)?;
        let value_series = self.column(values)?;

        // Get unique values for index and columns
        let mut index_values: Vec<Value> = Vec::new();
        let mut index_map: HashMap<String, usize> = HashMap::new();

        for i in 0..self.num_rows() {
            let val = index_series.get(i)?;
            let key = Self::value_to_key_string(&val);
            if !index_map.contains_key(&key) {
                index_map.insert(key, index_values.len());
                index_values.push(val);
            }
        }

        let mut col_values: Vec<String> = Vec::new();
        let mut col_map: HashMap<String, usize> = HashMap::new();

        for i in 0..self.num_rows() {
            let val = column_series.get(i)?;
            let key = Self::value_to_key_string(&val);
            if !col_map.contains_key(&key) {
                col_map.insert(key.clone(), col_values.len());
                col_values.push(match val {
                    Value::String(s) => s.to_string(),
                    _ => key,
                });
            }
        }

        // Build aggregation matrix - collect all values for each cell
        let num_rows = index_values.len();
        let num_cols = col_values.len();
        let mut cell_values: Vec<Vec<Vec<Value>>> = vec![vec![Vec::new(); num_cols]; num_rows];

        // Collect values for each cell
        for i in 0..self.num_rows() {
            let idx_val = index_series.get(i)?;
            let col_val = column_series.get(i)?;
            let val = value_series.get(i)?;

            let idx_key = Self::value_to_key_string(&idx_val);
            let col_key = Self::value_to_key_string(&col_val);

            if let (Some(&row_idx), Some(&col_idx)) =
                (index_map.get(&idx_key), col_map.get(&col_key))
            {
                cell_values[row_idx][col_idx].push(val);
            }
        }

        // Apply aggregation function
        let aggregate = |values: &[Value]| -> Value {
            if values.is_empty() {
                return Value::Null;
            }

            match aggfunc {
                "count" => Value::Int(values.len() as i64),
                "first" => values.first().cloned().unwrap_or(Value::Null),
                "last" => values.last().cloned().unwrap_or(Value::Null),
                "sum" => {
                    let mut sum = 0.0;
                    for v in values {
                        match v {
                            Value::Int(i) => sum += *i as f64,
                            Value::Float(f) => sum += f,
                            _ => {}
                        }
                    }
                    Value::Float(sum)
                }
                "mean" => {
                    let mut sum = 0.0;
                    let mut count = 0;
                    for v in values {
                        match v {
                            Value::Int(i) => {
                                sum += *i as f64;
                                count += 1;
                            }
                            Value::Float(f) => {
                                sum += f;
                                count += 1;
                            }
                            _ => {}
                        }
                    }
                    if count == 0 {
                        Value::Null
                    } else {
                        Value::Float(sum / count as f64)
                    }
                }
                "min" => values
                    .iter()
                    .filter_map(|v| match v {
                        Value::Int(i) => Some(*i as f64),
                        Value::Float(f) => Some(*f),
                        _ => None,
                    })
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map_or(Value::Null, Value::Float),
                "max" => values
                    .iter()
                    .filter_map(|v| match v {
                        Value::Int(i) => Some(*i as f64),
                        Value::Float(f) => Some(*f),
                        _ => None,
                    })
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map_or(Value::Null, Value::Float),
                _ => values.first().cloned().unwrap_or(Value::Null),
            }
        };

        // Build result matrix with aggregated values
        let mut matrix: Vec<Vec<Value>> = Vec::with_capacity(num_rows);
        for row_cells in &cell_values {
            let row: Vec<Value> = row_cells.iter().map(|cell| aggregate(cell)).collect();
            matrix.push(row);
        }

        // Build result DataFrame
        let mut result_columns: Vec<Series> = Vec::new();

        // Index column
        let index_result = Series::from_values(index, &index_values)?;
        result_columns.push(index_result);

        // Value columns
        for (col_idx, col_name) in col_values.iter().enumerate() {
            let values: Vec<Value> = matrix.iter().map(|row| row[col_idx].clone()).collect();
            let series = Series::from_values(col_name, &values)?;
            result_columns.push(series);
        }

        DataFrame::from_series(result_columns)
    }

    // ========================================================================
    // Column Operations (11.5.1, 11.5.2)
    // ========================================================================

    /// Add a new column from a Series
    ///
    /// The Series must have the same length as the DataFrame.
    ///
    /// # Arguments
    /// * `series` - The Series to add as a new column
    ///
    /// # Errors
    /// Returns error if the Series has a different length than the DataFrame
    pub fn add_column(&self, series: Series) -> DataResult<Self> {
        // Validate length matches
        if series.len() != self.num_rows() && !self.is_empty() {
            return Err(DataError::SchemaMismatch(format!(
                "cannot add column '{}' with {} rows to DataFrame with {} rows",
                series.name(),
                series.len(),
                self.num_rows()
            )));
        }

        // Check for duplicate column name
        if self.columns().contains(&series.name().to_string()) {
            return Err(DataError::SchemaMismatch(format!(
                "column '{}' already exists in DataFrame",
                series.name()
            )));
        }

        // Build new DataFrame with the additional column
        let mut columns: Vec<Series> = self.iter_columns().collect::<DataResult<Vec<_>>>()?;
        columns.push(series);

        DataFrame::from_series(columns)
    }

    /// Add a new column from values
    ///
    /// # Arguments
    /// * `name` - Name for the new column
    /// * `values` - Values for the new column (must match DataFrame row count)
    ///
    /// # Errors
    /// Returns error if values length doesn't match DataFrame row count
    pub fn add_column_from_values(&self, name: &str, values: &[Value]) -> DataResult<Self> {
        let series = Series::from_values(name, values)?;
        self.add_column(series)
    }

    /// Add or replace a column with a Series
    ///
    /// If a column with the same name exists, it is replaced.
    /// Otherwise, the column is added.
    ///
    /// # Arguments
    /// * `series` - The Series to add or replace
    ///
    /// # Errors
    /// Returns error if the Series has a different length than the DataFrame
    pub fn with_column(&self, series: Series) -> DataResult<Self> {
        // Validate length matches
        if series.len() != self.num_rows() && !self.is_empty() {
            return Err(DataError::SchemaMismatch(format!(
                "cannot add column '{}' with {} rows to DataFrame with {} rows",
                series.name(),
                series.len(),
                self.num_rows()
            )));
        }

        let col_name = series.name().to_string();

        // Check if column already exists
        if self.columns().contains(&col_name) {
            // Replace existing column
            let mut columns: Vec<Series> = Vec::with_capacity(self.num_columns());
            for col_idx in 0..self.num_columns() {
                let col = self.column_by_index(col_idx)?;
                if col.name() == col_name {
                    columns.push(series.clone());
                } else {
                    columns.push(col);
                }
            }
            DataFrame::from_series(columns)
        } else {
            // Add new column
            self.add_column(series)
        }
    }

    /// Cast a column to a specified type
    ///
    /// # Arguments
    /// * `column` - Name of the column to cast
    /// * `target_type` - Target type: "int", "float", "string", or "bool"
    ///
    /// # Errors
    /// Returns error if column doesn't exist or cast fails
    pub fn cast(&self, column: &str, target_type: &str) -> DataResult<Self> {
        // Validate column exists
        if !self.columns().contains(&column.to_string()) {
            return Err(DataError::ColumnNotFound(column.to_string()));
        }

        // Get and convert the column
        let series = self.column(column)?;
        let converted = match target_type.to_lowercase().as_str() {
            "int" | "integer" | "i64" => series.to_int()?,
            "float" | "double" | "f64" => series.to_float()?,
            "string" | "str" | "utf8" => series.to_str()?,
            "bool" | "boolean" => series.to_bool()?,
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "unknown type '{}', expected: int, float, string, or bool",
                    target_type
                )));
            }
        };

        // Rebuild DataFrame with converted column
        let columns: Vec<Series> = self
            .iter_columns()
            .map(|col| {
                let col = col?;
                if col.name() == column {
                    Ok(converted.clone())
                } else {
                    Ok(col)
                }
            })
            .collect::<DataResult<Vec<_>>>()?;

        DataFrame::from_series(columns)
    }

    // ========================================================================
    // Concatenation (11.5.5, 11.5.6)
    // ========================================================================

    /// Concatenate multiple DataFrames vertically (stack rows)
    ///
    /// All DataFrames must have the same columns (names and compatible types).
    ///
    /// # Arguments
    /// * `dataframes` - DataFrames to concatenate
    ///
    /// # Errors
    /// Returns error if schemas don't match
    pub fn concat(dataframes: &[&DataFrame]) -> DataResult<Self> {
        if dataframes.is_empty() {
            return Err(DataError::EmptyData);
        }

        if dataframes.len() == 1 {
            return Ok(dataframes[0].clone());
        }

        // Use first DataFrame as reference schema
        let first = dataframes[0];
        let col_names = first.columns();

        // Validate all DataFrames have the same columns
        for (i, df) in dataframes.iter().enumerate().skip(1) {
            let df_cols = df.columns();
            if df_cols != col_names {
                return Err(DataError::SchemaMismatch(format!(
                    "DataFrame at index {} has different columns: expected {:?}, got {:?}",
                    i, col_names, df_cols
                )));
            }
        }

        // Concatenate each column across all DataFrames
        let mut result_columns: Vec<Series> = Vec::with_capacity(col_names.len());

        for col_name in &col_names {
            let mut all_values: Vec<Value> = Vec::new();

            for df in dataframes {
                let col = df.column(col_name)?;
                for i in 0..col.len() {
                    all_values.push(col.get(i)?);
                }
            }

            let series = Series::from_values(col_name, &all_values)?;
            result_columns.push(series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Append rows from another DataFrame
    ///
    /// The other DataFrame must have the same columns.
    ///
    /// # Arguments
    /// * `other` - DataFrame to append
    ///
    /// # Errors
    /// Returns error if schemas don't match
    pub fn append(&self, other: &DataFrame) -> DataResult<Self> {
        DataFrame::concat(&[self, other])
    }

    // ========================================================================
    // Merge/Join Operations (11.5.7, 11.5.8)
    // ========================================================================

    /// Merge two DataFrames with SQL-style semantics and suffix handling
    ///
    /// # Arguments
    /// * `other` - Right DataFrame to merge with
    /// * `on` - Column names to join on (must exist in both DataFrames)
    /// * `how` - Join type: "inner", "left", "right", "outer"
    /// * `suffixes` - Tuple of suffixes to add to overlapping column names (left_suffix, right_suffix)
    ///
    /// # Errors
    /// Returns error if columns not found or operation fails
    pub fn merge(
        &self,
        other: &DataFrame,
        on: &[&str],
        how: &str,
        suffixes: (&str, &str),
    ) -> DataResult<Self> {
        use std::collections::{HashMap, HashSet};

        // Validate join columns exist in both DataFrames
        for col in on {
            if self.column(col).is_err() {
                return Err(DataError::ColumnNotFound(format!(
                    "join column '{}' not found in left DataFrame",
                    col
                )));
            }
            if other.column(col).is_err() {
                return Err(DataError::ColumnNotFound(format!(
                    "join column '{}' not found in right DataFrame",
                    col
                )));
            }
        }

        // Build key for each row based on join columns
        let left_keys: Vec<String> = (0..self.num_rows())
            .map(|i| {
                on.iter()
                    .map(|col| {
                        let series = self.column(col).unwrap();
                        Self::value_to_key_string(&series.get(i).unwrap_or(Value::Null))
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect();

        let right_keys: Vec<String> = (0..other.num_rows())
            .map(|i| {
                on.iter()
                    .map(|col| {
                        let series = other.column(col).unwrap();
                        Self::value_to_key_string(&series.get(i).unwrap_or(Value::Null))
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect();

        // Build index maps
        let mut right_index: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, key) in right_keys.iter().enumerate() {
            right_index.entry(key.clone()).or_default().push(i);
        }

        let mut left_index: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, key) in left_keys.iter().enumerate() {
            left_index.entry(key.clone()).or_default().push(i);
        }

        // Determine which rows to include based on join type
        let mut result_pairs: Vec<(Option<usize>, Option<usize>)> = Vec::new();

        match how {
            "inner" => {
                for (left_i, left_key) in left_keys.iter().enumerate() {
                    if let Some(right_indices) = right_index.get(left_key) {
                        for &right_i in right_indices {
                            result_pairs.push((Some(left_i), Some(right_i)));
                        }
                    }
                }
            }
            "left" => {
                for (left_i, left_key) in left_keys.iter().enumerate() {
                    if let Some(right_indices) = right_index.get(left_key) {
                        for &right_i in right_indices {
                            result_pairs.push((Some(left_i), Some(right_i)));
                        }
                    } else {
                        result_pairs.push((Some(left_i), None));
                    }
                }
            }
            "right" => {
                for (right_i, right_key) in right_keys.iter().enumerate() {
                    if let Some(left_indices) = left_index.get(right_key) {
                        for &left_i in left_indices {
                            result_pairs.push((Some(left_i), Some(right_i)));
                        }
                    } else {
                        result_pairs.push((None, Some(right_i)));
                    }
                }
            }
            "outer" | "full" => {
                let mut matched_right: HashSet<usize> = HashSet::new();
                for (left_i, left_key) in left_keys.iter().enumerate() {
                    if let Some(right_indices) = right_index.get(left_key) {
                        for &right_i in right_indices {
                            result_pairs.push((Some(left_i), Some(right_i)));
                            matched_right.insert(right_i);
                        }
                    } else {
                        result_pairs.push((Some(left_i), None));
                    }
                }
                // Add unmatched right rows
                for right_i in 0..other.num_rows() {
                    if !matched_right.contains(&right_i) {
                        result_pairs.push((None, Some(right_i)));
                    }
                }
            }
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "unsupported join type '{}': use 'inner', 'left', 'right', or 'outer'",
                    how
                )));
            }
        }

        // Determine column names for result
        let left_cols: HashSet<String> = self.columns().into_iter().collect();
        let right_cols: HashSet<String> = other.columns().into_iter().collect();
        let join_cols: HashSet<&str> = on.iter().copied().collect();

        // Build result columns
        let mut result_columns: Vec<Series> = Vec::new();

        // Add left columns (with suffix if overlapping and not a join key)
        for col_name in self.columns() {
            let series = self.column(&col_name)?;
            let final_name =
                if !join_cols.contains(col_name.as_str()) && right_cols.contains(&col_name) {
                    format!("{}{}", col_name, suffixes.0)
                } else {
                    col_name.clone()
                };

            let values: Vec<Value> = result_pairs
                .iter()
                .map(|(left_opt, _)| match left_opt {
                    Some(i) => series.get(*i).unwrap_or(Value::Null),
                    None => Value::Null,
                })
                .collect();

            let result_series = Series::from_values(&final_name, &values)?;
            result_columns.push(result_series);
        }

        // Add right columns (skip join keys, add suffix if overlapping)
        for col_name in other.columns() {
            if join_cols.contains(col_name.as_str()) {
                continue; // Skip join keys (already included from left)
            }

            let series = other.column(&col_name)?;
            let final_name = if left_cols.contains(&col_name) {
                format!("{}{}", col_name, suffixes.1)
            } else {
                col_name.clone()
            };

            let values: Vec<Value> = result_pairs
                .iter()
                .map(|(_, right_opt)| match right_opt {
                    Some(i) => series.get(*i).unwrap_or(Value::Null),
                    None => Value::Null,
                })
                .collect();

            let result_series = Series::from_values(&final_name, &values)?;
            result_columns.push(result_series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Cross join (Cartesian product) of two DataFrames
    ///
    /// Returns a DataFrame with every combination of rows from both DataFrames.
    ///
    /// # Arguments
    /// * `other` - Right DataFrame to cross join with
    ///
    /// # Errors
    /// Returns error if operation fails
    pub fn cross_join(&self, other: &DataFrame) -> DataResult<Self> {
        let left_rows = self.num_rows();
        let right_rows = other.num_rows();
        let total_rows = left_rows * right_rows;

        if total_rows == 0 {
            // Return empty DataFrame with combined columns
            let mut all_columns: Vec<Series> =
                self.iter_columns().collect::<DataResult<Vec<_>>>()?;
            let right_cols: Vec<Series> = other.iter_columns().collect::<DataResult<Vec<_>>>()?;
            all_columns.extend(right_cols);
            return DataFrame::from_series(all_columns);
        }

        let mut result_columns: Vec<Series> = Vec::new();

        // Handle overlapping column names
        let left_cols: std::collections::HashSet<String> = self.columns().into_iter().collect();

        // Add left columns (repeated for each right row)
        for col_name in self.columns() {
            let series = self.column(&col_name)?;
            let mut values: Vec<Value> = Vec::with_capacity(total_rows);

            for left_i in 0..left_rows {
                let val = series.get(left_i)?;
                for _ in 0..right_rows {
                    values.push(val.clone());
                }
            }

            let result_series = Series::from_values(&col_name, &values)?;
            result_columns.push(result_series);
        }

        // Add right columns (repeated for each left row)
        for col_name in other.columns() {
            let series = other.column(&col_name)?;
            let final_name = if left_cols.contains(&col_name) {
                format!("{}_right", col_name)
            } else {
                col_name.clone()
            };

            let mut values: Vec<Value> = Vec::with_capacity(total_rows);

            for _ in 0..left_rows {
                for right_i in 0..right_rows {
                    values.push(series.get(right_i)?);
                }
            }

            let result_series = Series::from_values(&final_name, &values)?;
            result_columns.push(result_series);
        }

        DataFrame::from_series(result_columns)
    }

    // ========================================================================
    // Index Operations (11.5.9, 11.5.10)
    // ========================================================================

    /// Reset the index to default sequential integers (0, 1, 2, ...)
    ///
    /// Adds an "index" column at the beginning with row numbers.
    ///
    /// # Errors
    /// Returns error if operation fails
    pub fn reset_index(&self) -> DataResult<Self> {
        let num_rows = self.num_rows();

        // Create index column
        let index_values: Vec<i64> = (0..num_rows as i64).collect();
        let index_series = Series::from_ints("index", index_values);

        // Build new columns list with index first
        let mut result_columns = vec![index_series];
        for col_name in self.columns() {
            let series = self.column(&col_name)?;
            result_columns.push(series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Set a column as the index (move it to the first position)
    ///
    /// # Arguments
    /// * `column` - Name of the column to set as index
    ///
    /// # Errors
    /// Returns error if column not found
    pub fn set_index(&self, column: &str) -> DataResult<Self> {
        // Validate column exists
        let index_series = self.column(column)?;

        // Build new columns list with index first
        let mut result_columns = vec![index_series];
        for col_name in self.columns() {
            if col_name != column {
                let series = self.column(&col_name)?;
                result_columns.push(series);
            }
        }

        DataFrame::from_series(result_columns)
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
        let schema = Arc::new(Schema::new(vec![Field::new(
            "a",
            arrow::datatypes::DataType::Int64,
            true,
        )]));
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

    // ===== Statistical Operations Tests =====

    #[test]
    fn test_describe() {
        let df = sample_dataframe();
        let desc = df.describe().unwrap();

        // Should have statistic column + 2 numeric columns (age, score)
        assert_eq!(desc.num_columns(), 3);
        assert!(desc.columns().contains(&"statistic".to_string()));
        assert!(desc.columns().contains(&"age".to_string()));
        assert!(desc.columns().contains(&"score".to_string()));

        // Should have 8 rows: count, mean, std, min, 25%, 50%, 75%, max
        assert_eq!(desc.num_rows(), 8);
    }

    #[test]
    fn test_corr() {
        let ages = Series::from_ints("age", vec![20, 30, 40, 50, 60]);
        let scores = Series::from_floats("score", vec![50.0, 60.0, 70.0, 80.0, 90.0]);
        let df = DataFrame::from_series(vec![ages, scores]).unwrap();

        let corr = df.corr().unwrap();

        // Should have column names column + 2 correlation columns
        assert_eq!(corr.num_columns(), 3);
        // Should have 2 rows (one for each variable)
        assert_eq!(corr.num_rows(), 2);

        // Diagonal should be 1.0 (perfect correlation with self)
        let age_col = corr.column("age").unwrap();
        if let Value::Float(self_corr) = age_col.get(0).unwrap() {
            assert!((self_corr - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_cov() {
        let x = Series::from_floats("x", vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let y = Series::from_floats("y", vec![2.0, 4.0, 6.0, 8.0, 10.0]);
        let df = DataFrame::from_series(vec![x, y]).unwrap();

        let cov = df.cov().unwrap();

        // Should have column names column + 2 covariance columns
        assert_eq!(cov.num_columns(), 3);
        assert_eq!(cov.num_rows(), 2);
    }

    #[test]
    fn test_value_counts() {
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Alice", "Charlie", "Alice"]);
        let df = DataFrame::from_series(vec![names]).unwrap();

        let counts = df.value_counts("name").unwrap();

        // Should have value column and count column
        assert_eq!(counts.num_columns(), 2);
        // Should have 3 unique values
        assert_eq!(counts.num_rows(), 3);

        // First row should be Alice (most frequent)
        let name_col = counts.column("name").unwrap();
        assert_eq!(name_col.get(0).unwrap(), Value::string("Alice"));

        // Alice should appear 3 times
        let count_col = counts.column("count").unwrap();
        assert_eq!(count_col.get(0).unwrap(), Value::Int(3));
    }

    // ===== Missing Data Handling Tests =====

    fn sample_dataframe_with_nulls() -> DataFrame {
        // Create DataFrame with some null values
        let names = Series::from_values(
            "name",
            &[
                Value::string("Alice"),
                Value::Null,
                Value::string("Charlie"),
                Value::string("Dave"),
            ],
        )
        .unwrap();
        let ages = Series::from_optional_ints("age", vec![Some(30), Some(25), None, Some(40)]);
        let scores = Series::from_values(
            "score",
            &[
                Value::Float(85.5),
                Value::Float(92.0),
                Value::Float(78.3),
                Value::Null,
            ],
        )
        .unwrap();

        DataFrame::from_series(vec![names, ages, scores]).unwrap()
    }

    #[test]
    fn test_dropna() {
        let df = sample_dataframe_with_nulls();
        assert_eq!(df.num_rows(), 4);

        let dropped = df.dropna().unwrap();
        // Only Alice (row 0) has no nulls
        assert_eq!(dropped.num_rows(), 1);

        let name_col = dropped.column("name").unwrap();
        assert_eq!(name_col.get(0).unwrap(), Value::string("Alice"));
    }

    #[test]
    fn test_dropna_columns() {
        let df = sample_dataframe_with_nulls();

        // Only check "name" column for nulls
        let dropped = df.dropna_columns(&["name".to_string()]).unwrap();
        // Row 1 (Bob) has null name, so should have 3 rows
        assert_eq!(dropped.num_rows(), 3);
    }

    #[test]
    fn test_fillna_constant() {
        let df = sample_dataframe_with_nulls();

        // Fill nulls with 0
        let filled = df.fillna(&Value::Int(0)).unwrap();
        assert_eq!(filled.num_rows(), 4);

        // Check age column - row 2 should now be 0 instead of null
        let age_col = filled.column("age").unwrap();
        assert_eq!(age_col.get(2).unwrap(), Value::Int(0));
    }

    #[test]
    fn test_fillna_forward() {
        let ages = Series::from_optional_ints("age", vec![Some(10), None, None, Some(40), None]);
        let df = DataFrame::from_series(vec![ages]).unwrap();

        let filled = df.fillna_forward().unwrap();
        let age_col = filled.column("age").unwrap();

        // First value stays 10, next two get filled with 10, then 40, then 40
        assert_eq!(age_col.get(0).unwrap(), Value::Int(10));
        assert_eq!(age_col.get(1).unwrap(), Value::Int(10));
        assert_eq!(age_col.get(2).unwrap(), Value::Int(10));
        assert_eq!(age_col.get(3).unwrap(), Value::Int(40));
        assert_eq!(age_col.get(4).unwrap(), Value::Int(40));
    }

    #[test]
    fn test_fillna_backward() {
        let ages = Series::from_optional_ints("age", vec![None, Some(20), None, None, Some(50)]);
        let df = DataFrame::from_series(vec![ages]).unwrap();

        let filled = df.fillna_backward().unwrap();
        let age_col = filled.column("age").unwrap();

        // First null gets filled with 20, then 20, then next nulls get 50
        assert_eq!(age_col.get(0).unwrap(), Value::Int(20));
        assert_eq!(age_col.get(1).unwrap(), Value::Int(20));
        assert_eq!(age_col.get(2).unwrap(), Value::Int(50));
        assert_eq!(age_col.get(3).unwrap(), Value::Int(50));
        assert_eq!(age_col.get(4).unwrap(), Value::Int(50));
    }

    #[test]
    fn test_fillna_map() {
        let df = sample_dataframe_with_nulls();

        let mut fill_map = std::collections::HashMap::new();
        fill_map.insert("age".to_string(), Value::Int(99));
        fill_map.insert("name".to_string(), Value::string("Unknown"));

        let filled = df.fillna_map(&fill_map).unwrap();

        // Check that name null was filled
        let name_col = filled.column("name").unwrap();
        assert_eq!(name_col.get(1).unwrap(), Value::string("Unknown"));

        // Check that age null was filled
        let age_col = filled.column("age").unwrap();
        assert_eq!(age_col.get(2).unwrap(), Value::Int(99));
    }

    // ===== Reshape Operations Tests =====

    #[test]
    fn test_transpose() {
        let df = sample_dataframe();
        let transposed = df.transpose().unwrap();

        // Original: 3 columns x 3 rows
        // Transposed: 4 columns (column + row_0 + row_1 + row_2) x 3 rows
        assert_eq!(transposed.num_columns(), 4);
        assert_eq!(transposed.num_rows(), 3);

        // Check column names are preserved in first column
        let col_names = transposed.column("column").unwrap();
        assert_eq!(col_names.get(0).unwrap(), Value::string("name"));
        assert_eq!(col_names.get(1).unwrap(), Value::string("age"));
        assert_eq!(col_names.get(2).unwrap(), Value::string("score"));

        // Check that values are converted to strings
        let row_0 = transposed.column("row_0").unwrap();
        assert_eq!(row_0.get(0).unwrap(), Value::string("Alice")); // name
        assert_eq!(row_0.get(1).unwrap(), Value::string("30")); // age as string
    }

    #[test]
    fn test_melt() {
        // Create a wide-format DataFrame
        let names = Series::from_strings("name", vec!["Alice", "Bob"]);
        let q1 = Series::from_ints("Q1", vec![100, 200]);
        let q2 = Series::from_ints("Q2", vec![150, 250]);
        let df = DataFrame::from_series(vec![names, q1, q2]).unwrap();

        // Melt: keep "name" as id, melt Q1 and Q2
        let melted = df.melt(&["name"], &["Q1", "Q2"], None, None).unwrap();

        // Should have: name, variable, value columns
        assert_eq!(melted.num_columns(), 3);
        // Should have 4 rows (2 names x 2 quarters)
        assert_eq!(melted.num_rows(), 4);

        assert!(melted.columns().contains(&"variable".to_string()));
        assert!(melted.columns().contains(&"value".to_string()));
    }

    #[test]
    fn test_stack() {
        let df = sample_dataframe();
        let stacked = df.stack(&["age", "score"]).unwrap();

        // Should have: row, column, value
        assert_eq!(stacked.num_columns(), 3);
        // 3 rows x 2 columns = 6 rows
        assert_eq!(stacked.num_rows(), 6);

        assert!(stacked.columns().contains(&"row".to_string()));
        assert!(stacked.columns().contains(&"column".to_string()));
        assert!(stacked.columns().contains(&"value".to_string()));

        // Check that values are converted to strings
        let value_col = stacked.column("value").unwrap();
        assert_eq!(value_col.get(0).unwrap(), Value::string("30")); // first age value as string
    }

    #[test]
    fn test_unstack() {
        // Create a long-format DataFrame (like the result of stack)
        let index = Series::from_strings("person", vec!["Alice", "Alice", "Bob", "Bob"]);
        let variable = Series::from_strings("metric", vec!["age", "score", "age", "score"]);
        let values = Series::from_ints("value", vec![30, 85, 25, 92]);
        let df = DataFrame::from_series(vec![index, variable, values]).unwrap();

        // Unstack: person as index, metric values become columns
        let unstacked = df.unstack("person", "metric", "value").unwrap();

        // Should have: person, age, score columns
        assert_eq!(unstacked.num_columns(), 3);
        // Should have 2 rows (Alice and Bob)
        assert_eq!(unstacked.num_rows(), 2);

        assert!(unstacked.columns().contains(&"person".to_string()));
        assert!(unstacked.columns().contains(&"age".to_string()));
        assert!(unstacked.columns().contains(&"score".to_string()));
    }

    #[test]
    fn test_pivot() {
        // Create sales data
        let product = Series::from_strings("product", vec!["A", "A", "B", "B"]);
        let quarter = Series::from_strings("quarter", vec!["Q1", "Q2", "Q1", "Q2"]);
        let sales = Series::from_ints("sales", vec![100, 150, 200, 250]);
        let df = DataFrame::from_series(vec![product, quarter, sales]).unwrap();

        let pivoted = df.pivot("product", "quarter", "sales").unwrap();

        // Should have: product, Q1, Q2 columns
        assert_eq!(pivoted.num_columns(), 3);
        // Should have 2 rows (products A and B)
        assert_eq!(pivoted.num_rows(), 2);

        assert!(pivoted.columns().contains(&"product".to_string()));
        assert!(pivoted.columns().contains(&"Q1".to_string()));
        assert!(pivoted.columns().contains(&"Q2".to_string()));
    }

    #[test]
    fn test_pivot_table_sum() {
        // Create data with duplicates (need aggregation)
        let product = Series::from_strings("product", vec!["A", "A", "A", "B", "B"]);
        let quarter = Series::from_strings("quarter", vec!["Q1", "Q1", "Q2", "Q1", "Q2"]);
        let sales = Series::from_ints("sales", vec![100, 50, 150, 200, 250]);
        let df = DataFrame::from_series(vec![product, quarter, sales]).unwrap();

        let pivoted = df
            .pivot_table("product", "quarter", "sales", "sum")
            .unwrap();

        // Should have: product, Q1, Q2 columns
        assert_eq!(pivoted.num_columns(), 3);
        assert_eq!(pivoted.num_rows(), 2);

        // Check A's Q1 sales (should be 100 + 50 = 150)
        let q1_col = pivoted.column("Q1").unwrap();
        let product_col = pivoted.column("product").unwrap();

        // Find A's row
        for i in 0..pivoted.num_rows() {
            if product_col.get(i).unwrap() == Value::string("A") {
                if let Value::Float(sum) = q1_col.get(i).unwrap() {
                    assert!((sum - 150.0).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn test_explode_non_list() {
        // Test explode with non-list column (should keep values as-is)
        let names = Series::from_strings("name", vec!["Alice", "Bob"]);
        let scores = Series::from_ints("score", vec![100, 200]);
        let df = DataFrame::from_series(vec![names, scores]).unwrap();

        // Exploding a non-list column should pass through unchanged
        let exploded = df.explode("score").unwrap();
        assert_eq!(exploded.num_rows(), 2);
        assert_eq!(exploded.num_columns(), 2);

        // Values should be preserved
        let score_col = exploded.column("score").unwrap();
        assert_eq!(score_col.get(0).unwrap(), Value::Int(100));
        assert_eq!(score_col.get(1).unwrap(), Value::Int(200));
    }

    #[test]
    fn test_explode_with_nulls() {
        // Test explode handles null values
        let names = Series::from_strings("name", vec!["Alice", "Bob"]);
        let scores = Series::from_optional_ints("score", vec![Some(100), None]);
        let df = DataFrame::from_series(vec![names, scores]).unwrap();

        let exploded = df.explode("score").unwrap();
        assert_eq!(exploded.num_rows(), 2);

        // Check null is preserved
        let score_col = exploded.column("score").unwrap();
        assert_eq!(score_col.get(0).unwrap(), Value::Int(100));
        assert_eq!(score_col.get(1).unwrap(), Value::Null);
    }

    // ===== Column Operations Tests (11.5.1, 11.5.2) =====

    #[test]
    fn test_add_column_from_values() {
        let df = sample_dataframe();
        let new_values = vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)];

        let result = df.add_column_from_values("is_active", &new_values).unwrap();

        assert_eq!(result.num_columns(), 4);
        assert!(result.columns().contains(&"is_active".to_string()));

        let new_col = result.column("is_active").unwrap();
        assert_eq!(new_col.get(0).unwrap(), Value::Bool(true));
        assert_eq!(new_col.get(1).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_add_column_from_series() {
        let df = sample_dataframe();
        let new_series = Series::from_ints("bonus", vec![100, 200, 300]);

        let result = df.add_column(new_series).unwrap();

        assert_eq!(result.num_columns(), 4);
        assert!(result.columns().contains(&"bonus".to_string()));
    }

    #[test]
    fn test_add_column_length_mismatch() {
        let df = sample_dataframe();
        let wrong_values = vec![Value::Int(1), Value::Int(2)]; // Only 2 values, df has 3 rows

        let result = df.add_column_from_values("bad", &wrong_values);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_column_duplicate_name() {
        let df = sample_dataframe();
        let values = vec![Value::Int(1), Value::Int(2), Value::Int(3)];

        let result = df.add_column_from_values("name", &values); // "name" already exists
        assert!(result.is_err());
    }

    // ===== Concatenation Tests (11.5.5, 11.5.6) =====

    #[test]
    fn test_concat_two_dataframes() {
        let df1 = sample_dataframe();
        let df2 = sample_dataframe();

        let result = DataFrame::concat(&[&df1, &df2]).unwrap();

        assert_eq!(result.num_rows(), 6); // 3 + 3
        assert_eq!(result.num_columns(), 3);
    }

    #[test]
    fn test_concat_mismatched_columns() {
        let names = Series::from_strings("name", vec!["Alice"]);
        let ages = Series::from_ints("age", vec![30]);
        let df1 = DataFrame::from_series(vec![names, ages]).unwrap();

        let other_names = Series::from_strings("name", vec!["Bob"]);
        let scores = Series::from_floats("score", vec![85.0]); // Different column
        let df2 = DataFrame::from_series(vec![other_names, scores]).unwrap();

        let result = DataFrame::concat(&[&df1, &df2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_append() {
        let df1 = sample_dataframe();
        let df2 = sample_dataframe();

        let result = df1.append(&df2).unwrap();

        assert_eq!(result.num_rows(), 6);
        assert_eq!(result.num_columns(), 3);
    }

    // ===== Merge Tests (11.5.7) =====

    #[test]
    fn test_merge_inner() {
        let id1 = Series::from_ints("id", vec![1, 2, 3]);
        let name1 = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
        let left = DataFrame::from_series(vec![id1, name1]).unwrap();

        let id2 = Series::from_ints("id", vec![2, 3, 4]);
        let score2 = Series::from_ints("score", vec![85, 90, 95]);
        let right = DataFrame::from_series(vec![id2, score2]).unwrap();

        let result = left.merge(&right, &["id"], "inner", ("_x", "_y")).unwrap();

        // Inner join: only ids 2 and 3 match
        assert_eq!(result.num_rows(), 2);
        assert!(result.columns().contains(&"id".to_string()));
        assert!(result.columns().contains(&"name".to_string()));
        assert!(result.columns().contains(&"score".to_string()));
    }

    #[test]
    fn test_merge_left() {
        let id1 = Series::from_ints("id", vec![1, 2, 3]);
        let name1 = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
        let left = DataFrame::from_series(vec![id1, name1]).unwrap();

        let id2 = Series::from_ints("id", vec![2, 3, 4]);
        let score2 = Series::from_ints("score", vec![85, 90, 95]);
        let right = DataFrame::from_series(vec![id2, score2]).unwrap();

        let result = left.merge(&right, &["id"], "left", ("_x", "_y")).unwrap();

        // Left join: all 3 rows from left, with nulls for id=1
        assert_eq!(result.num_rows(), 3);
    }

    #[test]
    fn test_merge_with_suffixes() {
        let id = Series::from_ints("id", vec![1, 2]);
        let value = Series::from_ints("value", vec![10, 20]);
        let left = DataFrame::from_series(vec![id.clone(), value]).unwrap();

        let id2 = Series::from_ints("id", vec![1, 2]);
        let value2 = Series::from_ints("value", vec![100, 200]);
        let right = DataFrame::from_series(vec![id2, value2]).unwrap();

        let result = left
            .merge(&right, &["id"], "inner", ("_left", "_right"))
            .unwrap();

        // Both have "value" column, should get suffixes
        assert!(result.columns().contains(&"value_left".to_string()));
        assert!(result.columns().contains(&"value_right".to_string()));
    }

    // ===== Cross Join Tests (11.5.8) =====

    #[test]
    fn test_cross_join() {
        let letters = Series::from_strings("letter", vec!["A", "B"]);
        let left = DataFrame::from_series(vec![letters]).unwrap();

        let numbers = Series::from_ints("number", vec![1, 2, 3]);
        let right = DataFrame::from_series(vec![numbers]).unwrap();

        let result = left.cross_join(&right).unwrap();

        // Cartesian product: 2 x 3 = 6 rows
        assert_eq!(result.num_rows(), 6);
        assert_eq!(result.num_columns(), 2);

        // Check some values
        let letter_col = result.column("letter").unwrap();
        let number_col = result.column("number").unwrap();

        // First row should be (A, 1)
        assert_eq!(letter_col.get(0).unwrap(), Value::string("A"));
        assert_eq!(number_col.get(0).unwrap(), Value::Int(1));
    }

    // ===== Index Operations Tests (11.5.9, 11.5.10) =====

    #[test]
    fn test_reset_index() {
        let df = sample_dataframe();
        let result = df.reset_index().unwrap();

        assert_eq!(result.num_columns(), 4); // Original 3 + index
        assert_eq!(result.columns()[0], "index");

        let index_col = result.column("index").unwrap();
        assert_eq!(index_col.get(0).unwrap(), Value::Int(0));
        assert_eq!(index_col.get(1).unwrap(), Value::Int(1));
        assert_eq!(index_col.get(2).unwrap(), Value::Int(2));
    }

    #[test]
    fn test_set_index() {
        let df = sample_dataframe();
        let result = df.set_index("age").unwrap();

        // "age" should now be the first column
        assert_eq!(result.columns()[0], "age");
        assert_eq!(result.num_columns(), 3); // Same number of columns

        // Other columns follow
        assert!(result.columns().contains(&"name".to_string()));
        assert!(result.columns().contains(&"score".to_string()));
    }

    #[test]
    fn test_set_index_not_found() {
        let df = sample_dataframe();
        let result = df.set_index("nonexistent");
        assert!(result.is_err());
    }

    // ===== Type Conversion Tests =====

    #[test]
    fn test_cast_int_to_float() {
        let df = sample_dataframe(); // has "age" as Int column
        let result = df.cast("age", "float").unwrap();

        let age_col = result.column("age").unwrap();
        assert_eq!(age_col.data_type(), &arrow::datatypes::DataType::Float64);
    }

    #[test]
    fn test_cast_int_to_string() {
        let df = sample_dataframe();
        let result = df.cast("age", "string").unwrap();

        let age_col = result.column("age").unwrap();
        assert_eq!(age_col.data_type(), &arrow::datatypes::DataType::Utf8);
        assert_eq!(age_col.get(0).unwrap(), Value::string("30")); // First age is 30
    }

    #[test]
    fn test_cast_preserves_other_columns() {
        let df = sample_dataframe();
        let result = df.cast("age", "float").unwrap();

        // Other columns should remain unchanged
        assert_eq!(result.num_columns(), df.num_columns());
        let name_col = result.column("name").unwrap();
        assert_eq!(name_col.data_type(), &arrow::datatypes::DataType::Utf8);
    }

    #[test]
    fn test_cast_invalid_column() {
        let df = sample_dataframe();
        let result = df.cast("nonexistent", "int");
        assert!(result.is_err());
    }

    #[test]
    fn test_cast_invalid_type() {
        let df = sample_dataframe();
        let result = df.cast("age", "invalid_type");
        assert!(result.is_err());
    }
}
