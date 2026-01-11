//! GroupedDataFrame: A DataFrame partitioned by key columns for aggregation

use std::collections::HashMap;
use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

use super::dataframe::DataFrame;
use super::error::{DataError, DataResult};
use super::series::Series;
use crate::bytecode::Value;

/// Aggregation operation type
#[derive(Debug, Clone, PartialEq)]
pub enum AggOp {
    /// Sum of values
    Sum,
    /// Mean/average of values
    Mean,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Count of non-null values
    Count,
    /// First value in each group
    First,
    /// Last value in each group
    Last,
    /// Standard deviation
    Std,
    /// Variance
    Var,
    /// Median value
    Median,
    /// Mode (most frequent value)
    Mode,
    /// Count of distinct values
    CountDistinct,
}

impl AggOp {
    /// Get the operation name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            AggOp::Sum => "sum",
            AggOp::Mean => "mean",
            AggOp::Min => "min",
            AggOp::Max => "max",
            AggOp::Count => "count",
            AggOp::First => "first",
            AggOp::Last => "last",
            AggOp::Std => "std",
            AggOp::Var => "var",
            AggOp::Median => "median",
            AggOp::Mode => "mode",
            AggOp::CountDistinct => "count_distinct",
        }
    }
}

/// Aggregation specification - describes one aggregation to perform
#[derive(Debug, Clone, PartialEq)]
pub struct AggSpec {
    /// The aggregation operation
    pub op: AggOp,
    /// The source column name (None for count which doesn't need a column)
    pub column: Option<String>,
    /// The output column name
    pub output_name: String,
}

impl AggSpec {
    /// Create a new aggregation spec
    #[must_use]
    pub fn new(op: AggOp, column: Option<String>, output_name: String) -> Self {
        Self {
            op,
            column,
            output_name,
        }
    }

    /// Create a sum aggregation
    #[must_use]
    pub fn sum(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Sum,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a mean aggregation
    #[must_use]
    pub fn mean(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Mean,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a min aggregation
    #[must_use]
    pub fn min(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Min,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a max aggregation
    #[must_use]
    pub fn max(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Max,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a count aggregation
    #[must_use]
    pub fn count(output_name: &str) -> Self {
        Self::new(AggOp::Count, None, output_name.to_string())
    }

    /// Create a first aggregation
    #[must_use]
    pub fn first(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::First,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a last aggregation
    #[must_use]
    pub fn last(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Last,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a std (standard deviation) aggregation
    #[must_use]
    pub fn std(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Std,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a var (variance) aggregation
    #[must_use]
    pub fn var(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Var,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a median aggregation
    #[must_use]
    pub fn median(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Median,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a mode aggregation
    #[must_use]
    pub fn mode(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::Mode,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }

    /// Create a count_distinct aggregation
    #[must_use]
    pub fn count_distinct(column: &str, output_name: &str) -> Self {
        Self::new(
            AggOp::CountDistinct,
            Some(column.to_string()),
            output_name.to_string(),
        )
    }
}

/// A grouped DataFrame - the result of calling group_by on a DataFrame
#[derive(Clone)]
pub struct GroupedDataFrame {
    /// The underlying DataFrame
    source: Arc<DataFrame>,
    /// The columns to group by
    group_columns: Vec<String>,
    /// Map from group key to row indices belonging to that group
    groups: HashMap<Vec<GroupKey>, Vec<usize>>,
}

/// A value that can be used as a group key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GroupKey {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
}

impl GroupKey {
    /// Convert to a Stratum Value
    #[must_use]
    pub fn to_value(&self) -> Value {
        match self {
            GroupKey::Null => Value::Null,
            GroupKey::Bool(b) => Value::Bool(*b),
            GroupKey::Int(i) => Value::Int(*i),
            GroupKey::String(s) => Value::string(s.clone()),
        }
    }
}

impl GroupedDataFrame {
    /// Create a new grouped DataFrame
    ///
    /// # Errors
    /// Returns error if group columns don't exist
    pub fn new(source: Arc<DataFrame>, group_columns: Vec<String>) -> DataResult<Self> {
        // Validate that all group columns exist
        for col in &group_columns {
            if source.column(col).is_err() {
                return Err(DataError::ColumnNotFound(col.clone()));
            }
        }

        // Build the group map
        let groups = Self::build_groups(&source, &group_columns)?;

        Ok(Self {
            source,
            group_columns,
            groups,
        })
    }

    /// Build the mapping from group keys to row indices
    fn build_groups(
        df: &DataFrame,
        group_columns: &[String],
    ) -> DataResult<HashMap<Vec<GroupKey>, Vec<usize>>> {
        let mut groups: HashMap<Vec<GroupKey>, Vec<usize>> = HashMap::new();

        // Get the group column series
        let group_series: Vec<Series> = group_columns
            .iter()
            .map(|name| df.column(name))
            .collect::<DataResult<Vec<_>>>()?;

        // Iterate over all rows and assign to groups
        for row_idx in 0..df.num_rows() {
            let mut key = Vec::with_capacity(group_columns.len());

            for series in &group_series {
                let val = series.get(row_idx)?;
                let group_key = value_to_group_key(&val)?;
                key.push(group_key);
            }

            groups.entry(key).or_default().push(row_idx);
        }

        Ok(groups)
    }

    /// Get the source DataFrame
    #[must_use]
    pub fn source(&self) -> &Arc<DataFrame> {
        &self.source
    }

    /// Get the group columns
    #[must_use]
    pub fn group_columns(&self) -> &[String] {
        &self.group_columns
    }

    /// Get the number of groups
    #[must_use]
    pub fn num_groups(&self) -> usize {
        self.groups.len()
    }

    /// Apply aggregations and return a DataFrame
    ///
    /// # Errors
    /// Returns error if aggregation fails
    pub fn aggregate(&self, specs: &[AggSpec]) -> DataResult<DataFrame> {
        if self.groups.is_empty() {
            return self.empty_aggregate_result(specs);
        }

        // Collect group keys and their indices in sorted order for deterministic output
        let mut sorted_groups: Vec<_> = self.groups.iter().collect();
        sorted_groups.sort_by(|a, b| {
            for (ka, kb) in a.0.iter().zip(b.0.iter()) {
                match (ka, kb) {
                    (GroupKey::Int(a), GroupKey::Int(b)) => match a.cmp(b) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    },
                    (GroupKey::String(a), GroupKey::String(b)) => match a.cmp(b) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    },
                    _ => continue,
                }
            }
            std::cmp::Ordering::Equal
        });

        // Build result columns
        let mut result_columns: Vec<Series> = Vec::new();

        // First, add the group key columns
        for (col_idx, col_name) in self.group_columns.iter().enumerate() {
            let values: Vec<Value> = sorted_groups
                .iter()
                .map(|(key, _)| key[col_idx].to_value())
                .collect();
            let series = Series::from_values(col_name, &values)?;
            result_columns.push(series);
        }

        // Then, compute each aggregation
        // Note: Parallel aggregation is not used here because Value is not Send
        // (it uses Rc internally). Parallelization is applied at the column level
        // in DataFrame operations instead.
        for spec in specs {
            let values = self.compute_aggregation(spec, &sorted_groups)?;
            let series = Series::from_values(&spec.output_name, &values)?;
            result_columns.push(series);
        }

        DataFrame::from_series(result_columns)
    }

    /// Compute a single aggregation across all groups
    fn compute_aggregation(
        &self,
        spec: &AggSpec,
        sorted_groups: &[(&Vec<GroupKey>, &Vec<usize>)],
    ) -> DataResult<Vec<Value>> {
        let mut results = Vec::with_capacity(sorted_groups.len());

        // Get the source column if needed
        let source_col = if let Some(col_name) = &spec.column {
            Some(self.source.column(col_name)?)
        } else {
            None
        };

        for (_, indices) in sorted_groups {
            let value = match spec.op {
                AggOp::Sum => self.compute_sum(&source_col, indices)?,
                AggOp::Mean => self.compute_mean(&source_col, indices)?,
                AggOp::Min => self.compute_min(&source_col, indices)?,
                AggOp::Max => self.compute_max(&source_col, indices)?,
                AggOp::Count => Value::Int(indices.len() as i64),
                AggOp::First => self.compute_first(&source_col, indices)?,
                AggOp::Last => self.compute_last(&source_col, indices)?,
                AggOp::Std => self.compute_std(&source_col, indices)?,
                AggOp::Var => self.compute_var(&source_col, indices)?,
                AggOp::Median => self.compute_median(&source_col, indices)?,
                AggOp::Mode => self.compute_mode(&source_col, indices)?,
                AggOp::CountDistinct => self.compute_count_distinct(&source_col, indices)?,
            };
            results.push(value);
        }

        Ok(results)
    }

    fn compute_sum(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("sum requires a column".to_string()))?;

        let mut sum_int: i64 = 0;
        let mut sum_float: f64 = 0.0;
        let mut is_float = false;
        let mut has_value = false;

        for &idx in indices {
            let val = col.get(idx)?;
            match val {
                Value::Int(i) => {
                    sum_int += i;
                    has_value = true;
                }
                Value::Float(f) => {
                    sum_float += f;
                    if !is_float {
                        sum_float += sum_int as f64;
                        is_float = true;
                    }
                    has_value = true;
                }
                Value::Null => {}
                _ => {
                    return Err(DataError::InvalidOperation(format!(
                        "cannot sum non-numeric value: {}",
                        val.type_name()
                    )));
                }
            }
        }

        if !has_value {
            return Ok(Value::Null);
        }

        if is_float {
            Ok(Value::Float(sum_float))
        } else {
            Ok(Value::Int(sum_int))
        }
    }

    fn compute_mean(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("mean requires a column".to_string()))?;

        let mut sum: f64 = 0.0;
        let mut count: usize = 0;

        for &idx in indices {
            let val = col.get(idx)?;
            match val {
                Value::Int(i) => {
                    sum += i as f64;
                    count += 1;
                }
                Value::Float(f) => {
                    sum += f;
                    count += 1;
                }
                Value::Null => {}
                _ => {
                    return Err(DataError::InvalidOperation(format!(
                        "cannot compute mean of non-numeric value: {}",
                        val.type_name()
                    )));
                }
            }
        }

        if count == 0 {
            Ok(Value::Null)
        } else {
            Ok(Value::Float(sum / count as f64))
        }
    }

    fn compute_min(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("min requires a column".to_string()))?;

        let mut min_val: Option<Value> = None;

        for &idx in indices {
            let val = col.get(idx)?;
            if matches!(val, Value::Null) {
                continue;
            }

            min_val = Some(match min_val {
                None => val,
                Some(current) => {
                    if Self::value_lt(&val, &current)? {
                        val
                    } else {
                        current
                    }
                }
            });
        }

        Ok(min_val.unwrap_or(Value::Null))
    }

    fn compute_max(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("max requires a column".to_string()))?;

        let mut max_val: Option<Value> = None;

        for &idx in indices {
            let val = col.get(idx)?;
            if matches!(val, Value::Null) {
                continue;
            }

            max_val = Some(match max_val {
                None => val,
                Some(current) => {
                    if Self::value_lt(&current, &val)? {
                        val
                    } else {
                        current
                    }
                }
            });
        }

        Ok(max_val.unwrap_or(Value::Null))
    }

    fn compute_first(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("first requires a column".to_string()))?;

        if let Some(&first_idx) = indices.first() {
            col.get(first_idx)
        } else {
            Ok(Value::Null)
        }
    }

    fn compute_last(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("last requires a column".to_string()))?;

        if let Some(&last_idx) = indices.last() {
            col.get(last_idx)
        } else {
            Ok(Value::Null)
        }
    }

    fn compute_std(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        // Std = sqrt(variance)
        let var_result = self.compute_var(source_col, indices)?;
        match var_result {
            Value::Float(v) => Ok(Value::Float(v.sqrt())),
            other => Ok(other),
        }
    }

    fn compute_var(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("var requires a column".to_string()))?;

        // First compute the mean
        let mean_result = self.compute_mean(source_col, indices)?;
        let mean = match mean_result {
            Value::Float(m) => m,
            Value::Null => return Ok(Value::Null),
            _ => return Ok(Value::Null),
        };

        let mut sum_sq_diff: f64 = 0.0;
        let mut count: usize = 0;

        for &idx in indices {
            let val = col.get(idx)?;
            let f_val = match val {
                Value::Int(i) => i as f64,
                Value::Float(f) => f,
                Value::Null => continue,
                _ => {
                    return Err(DataError::InvalidOperation(format!(
                        "cannot compute variance of non-numeric value: {}",
                        val.type_name()
                    )));
                }
            };
            let diff = f_val - mean;
            sum_sq_diff += diff * diff;
            count += 1;
        }

        if count == 0 {
            Ok(Value::Null)
        } else {
            #[allow(clippy::cast_precision_loss)]
            Ok(Value::Float(sum_sq_diff / count as f64))
        }
    }

    fn compute_median(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("median requires a column".to_string()))?;

        let mut values: Vec<f64> = Vec::new();

        for &idx in indices {
            let val = col.get(idx)?;
            match val {
                Value::Int(i) => values.push(i as f64),
                Value::Float(f) => values.push(f),
                Value::Null => continue,
                _ => {
                    return Err(DataError::InvalidOperation(format!(
                        "cannot compute median of non-numeric value: {}",
                        val.type_name()
                    )));
                }
            }
        }

        if values.is_empty() {
            return Ok(Value::Null);
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = values.len() / 2;

        if values.len() % 2 == 0 {
            Ok(Value::Float((values[mid - 1] + values[mid]) / 2.0))
        } else {
            Ok(Value::Float(values[mid]))
        }
    }

    fn compute_mode(&self, source_col: &Option<Series>, indices: &[usize]) -> DataResult<Value> {
        let col = source_col
            .as_ref()
            .ok_or_else(|| DataError::InvalidOperation("mode requires a column".to_string()))?;

        let mut counts: std::collections::HashMap<String, (usize, Value)> =
            std::collections::HashMap::new();

        for &idx in indices {
            let val = col.get(idx)?;
            if matches!(val, Value::Null) {
                continue;
            }
            let key = format!("{val:?}");
            counts
                .entry(key)
                .and_modify(|(c, _)| *c += 1)
                .or_insert((1, val));
        }

        if counts.is_empty() {
            return Ok(Value::Null);
        }

        let max_count = counts.values().map(|(c, _)| *c).max().unwrap_or(0);
        let mode_values: Vec<&Value> = counts
            .values()
            .filter(|(c, _)| *c == max_count)
            .map(|(_, v)| v)
            .collect();

        Ok(mode_values
            .first()
            .map(|v| (*v).clone())
            .unwrap_or(Value::Null))
    }

    fn compute_count_distinct(
        &self,
        source_col: &Option<Series>,
        indices: &[usize],
    ) -> DataResult<Value> {
        let col = source_col.as_ref().ok_or_else(|| {
            DataError::InvalidOperation("count_distinct requires a column".to_string())
        })?;

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for &idx in indices {
            let val = col.get(idx)?;
            if matches!(val, Value::Null) {
                continue;
            }
            let key = format!("{val:?}");
            seen.insert(key);
        }

        Ok(Value::Int(seen.len() as i64))
    }

    /// Compare two values for less-than
    fn value_lt(a: &Value, b: &Value) -> DataResult<bool> {
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(a < b),
            (Value::Float(a), Value::Float(b)) => Ok(a < b),
            (Value::Int(a), Value::Float(b)) => Ok((*a as f64) < *b),
            (Value::Float(a), Value::Int(b)) => Ok(*a < (*b as f64)),
            (Value::String(a), Value::String(b)) => Ok(a < b),
            _ => Err(DataError::InvalidOperation(format!(
                "cannot compare {} and {}",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    /// Create an empty result DataFrame with the correct schema
    fn empty_aggregate_result(&self, specs: &[AggSpec]) -> DataResult<DataFrame> {
        let mut fields = Vec::new();

        // Add group column fields
        for col_name in &self.group_columns {
            let col_type = self.source.column(col_name)?.data_type().clone();
            fields.push(Field::new(col_name, col_type, true));
        }

        // Add aggregation result fields
        for spec in specs {
            let data_type = match spec.op {
                AggOp::Count => DataType::Int64,
                AggOp::Mean => DataType::Float64,
                _ => {
                    if let Some(col_name) = &spec.column {
                        self.source.column(col_name)?.data_type().clone()
                    } else {
                        DataType::Int64
                    }
                }
            };
            fields.push(Field::new(&spec.output_name, data_type, true));
        }

        let schema = Arc::new(Schema::new(fields));
        Ok(DataFrame::empty(schema))
    }

    /// Simple aggregation: sum a column
    pub fn sum(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::sum(column, out_name)])
    }

    /// Simple aggregation: mean of a column
    pub fn mean(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::mean(column, out_name)])
    }

    /// Simple aggregation: min of a column
    pub fn min(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::min(column, out_name)])
    }

    /// Simple aggregation: max of a column
    pub fn max(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::max(column, out_name)])
    }

    /// Simple aggregation: count rows in each group
    pub fn count(&self, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or("count");
        self.aggregate(&[AggSpec::count(out_name)])
    }

    /// Simple aggregation: first value in each group
    pub fn first(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::first(column, out_name)])
    }

    /// Simple aggregation: last value in each group
    pub fn last(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::last(column, out_name)])
    }

    /// Simple aggregation: standard deviation of a column
    pub fn std(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::std(column, out_name)])
    }

    /// Simple aggregation: variance of a column
    pub fn var(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::var(column, out_name)])
    }

    /// Simple aggregation: median of a column
    pub fn median(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::median(column, out_name)])
    }

    /// Simple aggregation: mode of a column
    pub fn mode(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::mode(column, out_name)])
    }

    /// Simple aggregation: count distinct values in a column
    pub fn count_distinct(&self, column: &str, output_name: Option<&str>) -> DataResult<DataFrame> {
        let out_name = output_name.unwrap_or(column);
        self.aggregate(&[AggSpec::count_distinct(column, out_name)])
    }
}

impl std::fmt::Debug for GroupedDataFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedDataFrame")
            .field("group_columns", &self.group_columns)
            .field("num_groups", &self.groups.len())
            .field("source_rows", &self.source.num_rows())
            .finish()
    }
}

/// Convert a Value to a GroupKey for hashing
fn value_to_group_key(value: &Value) -> DataResult<GroupKey> {
    match value {
        Value::Null => Ok(GroupKey::Null),
        Value::Bool(b) => Ok(GroupKey::Bool(*b)),
        Value::Int(i) => Ok(GroupKey::Int(*i)),
        Value::String(s) => Ok(GroupKey::String(s.as_ref().clone())),
        _ => Err(DataError::InvalidOperation(format!(
            "cannot use {} as a group key",
            value.type_name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sales_data() -> DataFrame {
        let regions =
            Series::from_strings("region", vec!["North", "South", "North", "South", "North"]);
        let amounts = Series::from_ints("amount", vec![100, 200, 150, 250, 175]);
        let quantities = Series::from_ints("quantity", vec![10, 20, 15, 25, 18]);

        DataFrame::from_series(vec![regions, amounts, quantities]).unwrap()
    }

    #[test]
    fn test_group_by_creation() {
        let df = sample_sales_data();
        let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();

        assert_eq!(grouped.num_groups(), 2); // North and South
        assert_eq!(grouped.group_columns(), &["region"]);
    }

    #[test]
    fn test_group_by_sum() {
        let df = sample_sales_data();
        let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();

        let result = grouped.sum("amount", Some("total")).unwrap();

        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.num_columns(), 2); // region + total

        // Verify the sums are correct
        let region_col = result.column("region").unwrap();
        let total_col = result.column("total").unwrap();

        for i in 0..result.num_rows() {
            let region = region_col.get(i).unwrap();
            let total = total_col.get(i).unwrap();

            match region {
                Value::String(r) if r.as_ref() == "North" => {
                    assert_eq!(total, Value::Int(425)); // 100 + 150 + 175
                }
                Value::String(r) if r.as_ref() == "South" => {
                    assert_eq!(total, Value::Int(450)); // 200 + 250
                }
                _ => panic!("unexpected region: {:?}", region),
            }
        }
    }

    #[test]
    fn test_group_by_count() {
        let df = sample_sales_data();
        let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();

        let result = grouped.count(Some("n")).unwrap();

        assert_eq!(result.num_rows(), 2);

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
    fn test_group_by_mean() {
        let df = sample_sales_data();
        let grouped = GroupedDataFrame::new(Arc::new(df), vec!["region".to_string()]).unwrap();

        let result = grouped.mean("amount", None).unwrap();

        let region_col = result.column("region").unwrap();
        let mean_col = result.column("amount").unwrap();

        for i in 0..result.num_rows() {
            let region = region_col.get(i).unwrap();
            let mean = mean_col.get(i).unwrap();

            match region {
                Value::String(r) if r.as_ref() == "North" => {
                    // (100 + 150 + 175) / 3 = 141.666...
                    if let Value::Float(m) = mean {
                        assert!((m - 141.666666666).abs() < 0.001);
                    } else {
                        panic!("expected float");
                    }
                }
                Value::String(r) if r.as_ref() == "South" => {
                    // (200 + 250) / 2 = 225
                    if let Value::Float(m) = mean {
                        assert!((m - 225.0).abs() < 0.001);
                    } else {
                        panic!("expected float");
                    }
                }
                _ => panic!("unexpected region: {:?}", region),
            }
        }
    }

    #[test]
    fn test_multiple_aggregations() {
        let df = sample_sales_data();
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
}
