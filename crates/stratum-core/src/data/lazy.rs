//! Lazy evaluation for DataFrame operations
//!
//! This module provides `LazyFrame`, a lazy evaluation wrapper that builds an
//! execution plan before materializing results. This enables query optimization
//! such as predicate pushdown and projection pruning.

use std::sync::Arc;

use super::dataframe::DataFrame;
use super::error::DataResult;
use super::grouped::AggSpec;
use super::join::JoinSpec;
use super::series::Series;
use crate::bytecode::Value;

/// A lazy operation that can be applied to a DataFrame
#[derive(Debug, Clone)]
pub enum LazyOp {
    /// Select specific columns
    Select(Vec<String>),
    /// Drop columns
    Drop(Vec<String>),
    /// Filter rows based on column predicates
    Filter(FilterPredicate),
    /// Sort by columns
    Sort {
        columns: Vec<String>,
        ascending: Vec<bool>,
    },
    /// Limit to first n rows
    Limit(usize),
    /// Skip first n rows
    Offset(usize),
    /// Rename columns
    Rename(Vec<(String, String)>),
    /// Distinct/unique rows
    Distinct,
    /// Distinct by specific columns
    DistinctBy(Vec<String>),
    /// Join with another LazyFrame
    Join {
        right: Box<LazyFrame>,
        spec: JoinSpec,
    },
    /// Fill null values
    FillNa(Value),
    /// Drop rows with null values
    DropNa,
    /// Apply window function
    Window(WindowSpec),
    /// Add a computed column
    WithColumn {
        name: String,
        expr: ColumnExpr,
    },
    /// Explode a list column into multiple rows
    Explode(String),
}

/// Filter predicate for lazy filtering
#[derive(Debug, Clone)]
pub enum FilterPredicate {
    /// Column equals value
    Eq(String, Value),
    /// Column not equals value
    Ne(String, Value),
    /// Column less than value
    Lt(String, Value),
    /// Column less than or equal to value
    Le(String, Value),
    /// Column greater than value
    Gt(String, Value),
    /// Column greater than or equal to value
    Ge(String, Value),
    /// Column is null
    IsNull(String),
    /// Column is not null
    IsNotNull(String),
    /// Column value is in list
    In(String, Vec<Value>),
    /// Column value is not in list
    NotIn(String, Vec<Value>),
    /// Column string contains substring
    Contains(String, String),
    /// Column string starts with prefix
    StartsWith(String, String),
    /// Column string ends with suffix
    EndsWith(String, String),
    /// Column value between two values (inclusive)
    Between(String, Value, Value),
    /// Logical AND of predicates
    And(Box<FilterPredicate>, Box<FilterPredicate>),
    /// Logical OR of predicates
    Or(Box<FilterPredicate>, Box<FilterPredicate>),
    /// Logical NOT of predicate
    Not(Box<FilterPredicate>),
}

/// Window function specification
#[derive(Debug, Clone)]
pub struct WindowSpec {
    /// The column to apply the window function to
    pub column: String,
    /// The window function to apply
    pub func: WindowFunc,
    /// Partition by columns (optional)
    pub partition_by: Vec<String>,
    /// Order by columns (optional)
    pub order_by: Vec<String>,
    /// Output column name
    pub output_name: String,
}

/// Window functions
#[derive(Debug, Clone)]
pub enum WindowFunc {
    /// Row number within partition
    RowNumber,
    /// Rank with gaps
    Rank,
    /// Dense rank without gaps
    DenseRank,
    /// Lead (next value)
    Lead(usize),
    /// Lag (previous value)
    Lag(usize),
    /// First value in window
    First,
    /// Last value in window
    Last,
    /// Running sum
    CumSum,
    /// Running mean
    CumMean,
    /// Running min
    CumMin,
    /// Running max
    CumMax,
    /// Running count
    CumCount,
    /// Percent rank
    PercentRank,
    /// N-tile
    Ntile(usize),
}

/// Column expression for computed columns
#[derive(Debug, Clone)]
pub enum ColumnExpr {
    /// Reference to an existing column
    Column(String),
    /// Literal value
    Literal(Value),
    /// Add two expressions
    Add(Box<ColumnExpr>, Box<ColumnExpr>),
    /// Subtract two expressions
    Sub(Box<ColumnExpr>, Box<ColumnExpr>),
    /// Multiply two expressions
    Mul(Box<ColumnExpr>, Box<ColumnExpr>),
    /// Divide two expressions
    Div(Box<ColumnExpr>, Box<ColumnExpr>),
    /// Modulo
    Mod(Box<ColumnExpr>, Box<ColumnExpr>),
    /// Negate
    Neg(Box<ColumnExpr>),
    /// Absolute value
    Abs(Box<ColumnExpr>),
    /// String concatenation
    Concat(Vec<ColumnExpr>),
    /// Cast to type
    Cast(Box<ColumnExpr>, DataType),
    /// Coalesce (first non-null)
    Coalesce(Vec<ColumnExpr>),
    /// Case expression
    Case {
        when_then: Vec<(FilterPredicate, ColumnExpr)>,
        otherwise: Box<ColumnExpr>,
    },
}

/// Data types for casting
#[derive(Debug, Clone)]
pub enum DataType {
    Int,
    Float,
    String,
    Bool,
}

/// A lazy DataFrame that builds an execution plan
#[derive(Debug, Clone)]
pub struct LazyFrame {
    /// Source data (or None if chained from another LazyFrame)
    source: LazySource,
    /// Operations to apply in order
    ops: Vec<LazyOp>,
}

/// Source of data for a LazyFrame
#[derive(Debug, Clone)]
enum LazySource {
    /// DataFrame already loaded
    DataFrame(Arc<DataFrame>),
    /// Read from Parquet file
    Parquet(String),
    /// Read from CSV file
    Csv(String),
    /// Read from JSON file
    Json(String),
}

impl LazyFrame {
    /// Create a LazyFrame from a DataFrame
    #[must_use]
    pub fn new(df: DataFrame) -> Self {
        Self {
            source: LazySource::DataFrame(Arc::new(df)),
            ops: Vec::new(),
        }
    }

    /// Scan a Parquet file lazily
    #[must_use]
    pub fn scan_parquet(path: impl Into<String>) -> Self {
        Self {
            source: LazySource::Parquet(path.into()),
            ops: Vec::new(),
        }
    }

    /// Scan a CSV file lazily
    #[must_use]
    pub fn scan_csv(path: impl Into<String>) -> Self {
        Self {
            source: LazySource::Csv(path.into()),
            ops: Vec::new(),
        }
    }

    /// Scan a JSON file lazily
    #[must_use]
    pub fn scan_json(path: impl Into<String>) -> Self {
        Self {
            source: LazySource::Json(path.into()),
            ops: Vec::new(),
        }
    }

    /// Add an operation to the plan
    fn push_op(&mut self, op: LazyOp) {
        self.ops.push(op);
    }

    /// Select specific columns
    #[must_use]
    pub fn select(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let cols: Vec<String> = columns.into_iter().map(Into::into).collect();
        self.push_op(LazyOp::Select(cols));
        self
    }

    /// Drop specific columns
    #[must_use]
    pub fn drop(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let cols: Vec<String> = columns.into_iter().map(Into::into).collect();
        self.push_op(LazyOp::Drop(cols));
        self
    }

    /// Filter rows based on a predicate
    #[must_use]
    pub fn filter(mut self, predicate: FilterPredicate) -> Self {
        self.push_op(LazyOp::Filter(predicate));
        self
    }

    /// Filter where column equals value
    #[must_use]
    pub fn filter_eq(self, column: impl Into<String>, value: Value) -> Self {
        self.filter(FilterPredicate::Eq(column.into(), value))
    }

    /// Filter where column is greater than value
    #[must_use]
    pub fn filter_gt(self, column: impl Into<String>, value: Value) -> Self {
        self.filter(FilterPredicate::Gt(column.into(), value))
    }

    /// Filter where column is less than value
    #[must_use]
    pub fn filter_lt(self, column: impl Into<String>, value: Value) -> Self {
        self.filter(FilterPredicate::Lt(column.into(), value))
    }

    /// Filter where column is null
    #[must_use]
    pub fn filter_null(self, column: impl Into<String>) -> Self {
        self.filter(FilterPredicate::IsNull(column.into()))
    }

    /// Filter where column is not null
    #[must_use]
    pub fn filter_not_null(self, column: impl Into<String>) -> Self {
        self.filter(FilterPredicate::IsNotNull(column.into()))
    }

    /// Filter where column is in list
    #[must_use]
    pub fn filter_in(self, column: impl Into<String>, values: Vec<Value>) -> Self {
        self.filter(FilterPredicate::In(column.into(), values))
    }

    /// Filter where column is between two values
    #[must_use]
    pub fn filter_between(
        self,
        column: impl Into<String>,
        low: Value,
        high: Value,
    ) -> Self {
        self.filter(FilterPredicate::Between(column.into(), low, high))
    }

    /// Sort by columns
    #[must_use]
    pub fn sort(mut self, columns: impl IntoIterator<Item = impl Into<String>>, ascending: bool) -> Self {
        let cols: Vec<String> = columns.into_iter().map(Into::into).collect();
        let asc = vec![ascending; cols.len()];
        self.push_op(LazyOp::Sort {
            columns: cols,
            ascending: asc,
        });
        self
    }

    /// Sort by columns with individual directions
    #[must_use]
    pub fn sort_by(
        mut self,
        columns: impl IntoIterator<Item = (impl Into<String>, bool)>,
    ) -> Self {
        let (cols, asc): (Vec<_>, Vec<_>) = columns
            .into_iter()
            .map(|(c, a)| (c.into(), a))
            .unzip();
        self.push_op(LazyOp::Sort {
            columns: cols,
            ascending: asc,
        });
        self
    }

    /// Limit to first n rows
    #[must_use]
    pub fn limit(mut self, n: usize) -> Self {
        self.push_op(LazyOp::Limit(n));
        self
    }

    /// Alias for limit
    #[must_use]
    pub fn head(self, n: usize) -> Self {
        self.limit(n)
    }

    /// Skip first n rows
    #[must_use]
    pub fn offset(mut self, n: usize) -> Self {
        self.push_op(LazyOp::Offset(n));
        self
    }

    /// Alias for offset
    #[must_use]
    pub fn skip(self, n: usize) -> Self {
        self.offset(n)
    }

    /// Get last n rows (skip all but last n)
    #[must_use]
    pub fn tail(self, n: usize) -> Self {
        // This is a hint - actual implementation needs row count
        // We'll handle this specially in optimization
        self.limit(n) // Placeholder - need special handling
    }

    /// Rename columns
    #[must_use]
    pub fn rename(mut self, renames: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        let pairs: Vec<(String, String)> = renames
            .into_iter()
            .map(|(old, new)| (old.into(), new.into()))
            .collect();
        self.push_op(LazyOp::Rename(pairs));
        self
    }

    /// Get distinct rows
    #[must_use]
    pub fn distinct(mut self) -> Self {
        self.push_op(LazyOp::Distinct);
        self
    }

    /// Alias for distinct
    #[must_use]
    pub fn unique(self) -> Self {
        self.distinct()
    }

    /// Get distinct rows by specific columns
    #[must_use]
    pub fn distinct_by(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let cols: Vec<String> = columns.into_iter().map(Into::into).collect();
        self.push_op(LazyOp::DistinctBy(cols));
        self
    }

    /// Join with another LazyFrame
    #[must_use]
    pub fn join(mut self, right: LazyFrame, spec: JoinSpec) -> Self {
        self.push_op(LazyOp::Join {
            right: Box::new(right),
            spec,
        });
        self
    }

    /// Inner join on a single column
    #[must_use]
    pub fn inner_join(
        self,
        right: LazyFrame,
        on: impl Into<String>,
    ) -> Self {
        let col = on.into();
        let spec = JoinSpec::inner(&col);
        self.join(right, spec)
    }

    /// Inner join on different columns
    #[must_use]
    pub fn inner_join_cols(
        self,
        right: LazyFrame,
        left_on: impl Into<String>,
        right_on: impl Into<String>,
    ) -> Self {
        let spec = JoinSpec::inner_cols(&left_on.into(), &right_on.into());
        self.join(right, spec)
    }

    /// Left join on a single column
    #[must_use]
    pub fn left_join(
        self,
        right: LazyFrame,
        on: impl Into<String>,
    ) -> Self {
        let col = on.into();
        let spec = JoinSpec::left(&col);
        self.join(right, spec)
    }

    /// Left join on different columns
    #[must_use]
    pub fn left_join_cols(
        self,
        right: LazyFrame,
        left_on: impl Into<String>,
        right_on: impl Into<String>,
    ) -> Self {
        let spec = JoinSpec::left_cols(&left_on.into(), &right_on.into());
        self.join(right, spec)
    }

    /// Right join on a single column
    #[must_use]
    pub fn right_join(
        self,
        right: LazyFrame,
        on: impl Into<String>,
    ) -> Self {
        let col = on.into();
        let spec = JoinSpec::right(&col);
        self.join(right, spec)
    }

    /// Outer join on a single column
    #[must_use]
    pub fn outer_join(
        self,
        right: LazyFrame,
        on: impl Into<String>,
    ) -> Self {
        let col = on.into();
        let spec = JoinSpec::outer(&col);
        self.join(right, spec)
    }

    /// Fill null values
    #[must_use]
    pub fn fill_null(mut self, value: Value) -> Self {
        self.push_op(LazyOp::FillNa(value));
        self
    }

    /// Drop rows with null values
    #[must_use]
    pub fn drop_nulls(mut self) -> Self {
        self.push_op(LazyOp::DropNa);
        self
    }

    /// Add a window function column
    #[must_use]
    pub fn with_window(mut self, spec: WindowSpec) -> Self {
        self.push_op(LazyOp::Window(spec));
        self
    }

    /// Add row number column
    #[must_use]
    pub fn with_row_number(self, name: impl Into<String>) -> Self {
        self.with_window(WindowSpec {
            column: String::new(),
            func: WindowFunc::RowNumber,
            partition_by: Vec::new(),
            order_by: Vec::new(),
            output_name: name.into(),
        })
    }

    /// Add a computed column
    #[must_use]
    pub fn with_column(mut self, name: impl Into<String>, expr: ColumnExpr) -> Self {
        self.push_op(LazyOp::WithColumn {
            name: name.into(),
            expr,
        });
        self
    }

    /// Explode a list column
    #[must_use]
    pub fn explode(mut self, column: impl Into<String>) -> Self {
        self.push_op(LazyOp::Explode(column.into()));
        self
    }

    /// Group by columns for aggregation
    #[must_use]
    pub fn group_by(self, columns: impl IntoIterator<Item = impl Into<String>>) -> LazyGroupBy {
        let cols: Vec<String> = columns.into_iter().map(Into::into).collect();
        LazyGroupBy {
            source: self,
            group_columns: cols,
            agg_specs: Vec::new(),
        }
    }

    /// Optimize the query plan
    #[must_use]
    pub fn optimize(self) -> Self {
        let mut optimized = self;
        optimized = Self::push_down_predicates(optimized);
        optimized = Self::prune_projections(optimized);
        optimized = Self::merge_limits(optimized);
        optimized
    }

    /// Push filter predicates as early as possible
    fn push_down_predicates(mut lf: Self) -> Self {
        // Simple predicate pushdown: move filters before selects/renames
        let mut filters = Vec::new();
        let mut other_ops = Vec::new();

        for op in lf.ops {
            match op {
                LazyOp::Filter(pred) => filters.push(pred),
                other => other_ops.push(other),
            }
        }

        // Rebuild ops with filters first (when safe)
        let mut new_ops = Vec::new();

        // Push filters before any operation that doesn't affect filtered columns
        for filter in filters {
            new_ops.push(LazyOp::Filter(filter));
        }
        new_ops.extend(other_ops);

        lf.ops = new_ops;
        lf
    }

    /// Remove unused column selections
    fn prune_projections(mut lf: Self) -> Self {
        // Find the last Select and remove earlier selects that are supersets
        let mut last_select_idx = None;
        for (i, op) in lf.ops.iter().enumerate() {
            if matches!(op, LazyOp::Select(_)) {
                last_select_idx = Some(i);
            }
        }

        if let Some(idx) = last_select_idx {
            // Remove earlier selects (simplified - could be smarter)
            let mut new_ops = Vec::new();
            for (i, op) in lf.ops.into_iter().enumerate() {
                if i < idx && matches!(op, LazyOp::Select(_)) {
                    continue; // Skip earlier selects
                }
                new_ops.push(op);
            }
            lf.ops = new_ops;
        }

        lf
    }

    /// Merge consecutive limits
    fn merge_limits(mut lf: Self) -> Self {
        let mut new_ops = Vec::new();
        let mut pending_limit: Option<usize> = None;

        for op in lf.ops {
            match op {
                LazyOp::Limit(n) => {
                    pending_limit = Some(match pending_limit {
                        Some(existing) => existing.min(n),
                        None => n,
                    });
                }
                other => {
                    if let Some(limit) = pending_limit.take() {
                        new_ops.push(LazyOp::Limit(limit));
                    }
                    new_ops.push(other);
                }
            }
        }

        if let Some(limit) = pending_limit {
            new_ops.push(LazyOp::Limit(limit));
        }

        lf.ops = new_ops;
        lf
    }

    /// Execute the query plan and return the result
    ///
    /// # Errors
    /// Returns error if any operation fails
    pub fn collect(self) -> DataResult<DataFrame> {
        // Optimize before executing
        let optimized = self.optimize();

        // Load source data
        let mut df = match optimized.source {
            LazySource::DataFrame(arc_df) => (*arc_df).clone(),
            LazySource::Parquet(path) => super::io::read_parquet(&path)?,
            LazySource::Csv(path) => super::io::read_csv(&path)?,
            LazySource::Json(path) => super::io::read_json(&path)?,
        };

        // Apply operations in order
        for op in optimized.ops {
            df = Self::apply_op(df, op)?;
        }

        Ok(df)
    }

    /// Apply a single operation to a DataFrame
    fn apply_op(df: DataFrame, op: LazyOp) -> DataResult<DataFrame> {
        match op {
            LazyOp::Select(cols) => {
                let col_refs: Vec<&str> = cols.iter().map(String::as_str).collect();
                df.select(&col_refs)
            }
            LazyOp::Drop(cols) => {
                let col_refs: Vec<&str> = cols.iter().map(String::as_str).collect();
                df.drop(&col_refs)
            }
            LazyOp::Filter(pred) => Self::apply_filter(df, pred),
            LazyOp::Sort { columns, ascending } => {
                let sort_spec: Vec<(&str, bool)> = columns
                    .iter()
                    .zip(ascending.iter())
                    .map(|(c, a)| (c.as_str(), *a))
                    .collect();
                df.sort_by(&sort_spec)
            }
            LazyOp::Limit(n) => df.head(n),
            LazyOp::Offset(n) => df.tail(df.num_rows().saturating_sub(n)),
            LazyOp::Rename(pairs) => {
                let mut result = df;
                for (old, new) in &pairs {
                    result = result.rename_column(old, new)?;
                }
                Ok(result)
            }
            LazyOp::Distinct => df.distinct(),
            LazyOp::DistinctBy(cols) => {
                let col_refs: Vec<&str> = cols.iter().map(String::as_str).collect();
                df.distinct_by(&col_refs)
            }
            LazyOp::Join { right, spec } => {
                let right_df = right.collect()?;
                df.join(&right_df, &spec)
            }
            LazyOp::FillNa(value) => df.fillna(&value),
            LazyOp::DropNa => df.dropna(),
            LazyOp::Window(spec) => Self::apply_window(df, spec),
            LazyOp::WithColumn { name, expr } => Self::apply_with_column(df, &name, expr),
            LazyOp::Explode(col) => df.explode(&col),
        }
    }

    /// Apply a filter predicate
    fn apply_filter(df: DataFrame, pred: FilterPredicate) -> DataResult<DataFrame> {
        let indices = Self::evaluate_predicate(&df, &pred)?;
        df.filter_by_indices(&indices)
    }

    /// Evaluate a predicate and return matching row indices
    fn evaluate_predicate(df: &DataFrame, pred: &FilterPredicate) -> DataResult<Vec<usize>> {
        match pred {
            FilterPredicate::Eq(col, val) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| series.get(i).ok().as_ref() == Some(val))
                    .collect())
            }
            FilterPredicate::Ne(col, val) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| series.get(i).ok().as_ref() != Some(val))
                    .collect())
            }
            FilterPredicate::Lt(col, val) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| Self::compare_lt(&v, val))
                    })
                    .collect())
            }
            FilterPredicate::Le(col, val) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| Self::compare_le(&v, val))
                    })
                    .collect())
            }
            FilterPredicate::Gt(col, val) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| Self::compare_gt(&v, val))
                    })
                    .collect())
            }
            FilterPredicate::Ge(col, val) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| Self::compare_ge(&v, val))
                    })
                    .collect())
            }
            FilterPredicate::IsNull(col) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| series.get(i).ok() == Some(Value::Null))
                    .collect())
            }
            FilterPredicate::IsNotNull(col) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| series.get(i).ok() != Some(Value::Null))
                    .collect())
            }
            FilterPredicate::In(col, vals) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| vals.contains(&v))
                    })
                    .collect())
            }
            FilterPredicate::NotIn(col, vals) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| !vals.contains(&v))
                    })
                    .collect())
            }
            FilterPredicate::Contains(col, substr) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| {
                            if let Value::String(s) = v {
                                s.contains(substr.as_str())
                            } else {
                                false
                            }
                        })
                    })
                    .collect())
            }
            FilterPredicate::StartsWith(col, prefix) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| {
                            if let Value::String(s) = v {
                                s.starts_with(prefix.as_str())
                            } else {
                                false
                            }
                        })
                    })
                    .collect())
            }
            FilterPredicate::EndsWith(col, suffix) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| {
                            if let Value::String(s) = v {
                                s.ends_with(suffix.as_str())
                            } else {
                                false
                            }
                        })
                    })
                    .collect())
            }
            FilterPredicate::Between(col, low, high) => {
                let series = df.column(col)?;
                Ok((0..series.len())
                    .filter(|&i| {
                        series.get(i).ok().map_or(false, |v| {
                            Self::compare_ge(&v, low) && Self::compare_le(&v, high)
                        })
                    })
                    .collect())
            }
            FilterPredicate::And(left, right) => {
                let left_indices: std::collections::HashSet<usize> =
                    Self::evaluate_predicate(df, left)?.into_iter().collect();
                let right_indices = Self::evaluate_predicate(df, right)?;
                Ok(right_indices
                    .into_iter()
                    .filter(|i| left_indices.contains(i))
                    .collect())
            }
            FilterPredicate::Or(left, right) => {
                let mut indices: std::collections::HashSet<usize> =
                    Self::evaluate_predicate(df, left)?.into_iter().collect();
                indices.extend(Self::evaluate_predicate(df, right)?);
                let mut result: Vec<usize> = indices.into_iter().collect();
                result.sort_unstable();
                Ok(result)
            }
            FilterPredicate::Not(inner) => {
                let exclude: std::collections::HashSet<usize> =
                    Self::evaluate_predicate(df, inner)?.into_iter().collect();
                Ok((0..df.num_rows())
                    .filter(|i| !exclude.contains(i))
                    .collect())
            }
        }
    }

    // Comparison helpers
    fn compare_lt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x < y,
            (Value::Float(x), Value::Float(y)) => x < y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) < *y,
            (Value::Float(x), Value::Int(y)) => *x < (*y as f64),
            (Value::String(x), Value::String(y)) => x < y,
            _ => false,
        }
    }

    fn compare_le(a: &Value, b: &Value) -> bool {
        a == b || Self::compare_lt(a, b)
    }

    fn compare_gt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x > y,
            (Value::Float(x), Value::Float(y)) => x > y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) > *y,
            (Value::Float(x), Value::Int(y)) => *x > (*y as f64),
            (Value::String(x), Value::String(y)) => x > y,
            _ => false,
        }
    }

    fn compare_ge(a: &Value, b: &Value) -> bool {
        a == b || Self::compare_gt(a, b)
    }

    /// Apply a window function
    fn apply_window(df: DataFrame, spec: WindowSpec) -> DataResult<DataFrame> {
        let num_rows = df.num_rows();
        if num_rows == 0 {
            return Ok(df);
        }

        let values: Vec<Value> = match spec.func {
            WindowFunc::RowNumber => {
                if spec.partition_by.is_empty() {
                    (1..=num_rows).map(|i| Value::Int(i as i64)).collect()
                } else {
                    // Partitioned row number
                    let mut result = vec![Value::Null; num_rows];
                    let partitions = Self::compute_partitions(&df, &spec.partition_by)?;
                    for indices in partitions.values() {
                        for (rank, &idx) in indices.iter().enumerate() {
                            result[idx] = Value::Int((rank + 1) as i64);
                        }
                    }
                    result
                }
            }
            WindowFunc::Rank | WindowFunc::DenseRank => {
                // Simplified - would need proper ordering
                (1..=num_rows).map(|i| Value::Int(i as i64)).collect()
            }
            WindowFunc::CumSum => {
                let col = df.column(&spec.column)?;
                let mut sum = 0.0;
                (0..num_rows)
                    .map(|i| {
                        if let Ok(v) = col.get(i) {
                            match v {
                                Value::Int(n) => sum += n as f64,
                                Value::Float(f) => sum += f,
                                _ => {}
                            }
                        }
                        Value::Float(sum)
                    })
                    .collect()
            }
            WindowFunc::CumCount => {
                (1..=num_rows).map(|i| Value::Int(i as i64)).collect()
            }
            WindowFunc::CumMin => {
                let col = df.column(&spec.column)?;
                let mut min: Option<f64> = None;
                (0..num_rows)
                    .map(|i| {
                        if let Ok(v) = col.get(i) {
                            let val = match v {
                                Value::Int(n) => Some(n as f64),
                                Value::Float(f) => Some(f),
                                _ => None,
                            };
                            if let Some(v) = val {
                                min = Some(min.map_or(v, |m| m.min(v)));
                            }
                        }
                        min.map_or(Value::Null, Value::Float)
                    })
                    .collect()
            }
            WindowFunc::CumMax => {
                let col = df.column(&spec.column)?;
                let mut max: Option<f64> = None;
                (0..num_rows)
                    .map(|i| {
                        if let Ok(v) = col.get(i) {
                            let val = match v {
                                Value::Int(n) => Some(n as f64),
                                Value::Float(f) => Some(f),
                                _ => None,
                            };
                            if let Some(v) = val {
                                max = Some(max.map_or(v, |m| m.max(v)));
                            }
                        }
                        max.map_or(Value::Null, Value::Float)
                    })
                    .collect()
            }
            WindowFunc::CumMean => {
                let col = df.column(&spec.column)?;
                let mut sum = 0.0;
                let mut count = 0;
                (0..num_rows)
                    .map(|i| {
                        if let Ok(v) = col.get(i) {
                            match v {
                                Value::Int(n) => {
                                    sum += n as f64;
                                    count += 1;
                                }
                                Value::Float(f) => {
                                    sum += f;
                                    count += 1;
                                }
                                _ => {}
                            }
                        }
                        if count > 0 {
                            Value::Float(sum / count as f64)
                        } else {
                            Value::Null
                        }
                    })
                    .collect()
            }
            WindowFunc::Lead(offset) => {
                let col = df.column(&spec.column)?;
                (0..num_rows)
                    .map(|i| col.get(i + offset).unwrap_or(Value::Null))
                    .collect()
            }
            WindowFunc::Lag(offset) => {
                let col = df.column(&spec.column)?;
                (0..num_rows)
                    .map(|i| {
                        if i >= offset {
                            col.get(i - offset).unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    })
                    .collect()
            }
            WindowFunc::First => {
                let col = df.column(&spec.column)?;
                let first = col.get(0).unwrap_or(Value::Null);
                vec![first; num_rows]
            }
            WindowFunc::Last => {
                let col = df.column(&spec.column)?;
                let last = col.get(num_rows - 1).unwrap_or(Value::Null);
                vec![last; num_rows]
            }
            WindowFunc::PercentRank => {
                (0..num_rows)
                    .map(|i| {
                        if num_rows <= 1 {
                            Value::Float(0.0)
                        } else {
                            Value::Float(i as f64 / (num_rows - 1) as f64)
                        }
                    })
                    .collect()
            }
            WindowFunc::Ntile(n) => {
                (0..num_rows)
                    .map(|i| {
                        let tile = (i * n / num_rows) + 1;
                        Value::Int(tile as i64)
                    })
                    .collect()
            }
        };

        // Add new column to DataFrame
        let new_series = Series::from_values(&spec.output_name, &values)?;
        df.with_column(new_series)
    }

    /// Compute partition groups
    fn compute_partitions(
        df: &DataFrame,
        partition_by: &[String],
    ) -> DataResult<std::collections::HashMap<String, Vec<usize>>> {
        let mut partitions: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();

        let cols: Vec<Series> = partition_by
            .iter()
            .map(|c| df.column(c))
            .collect::<DataResult<_>>()?;

        for i in 0..df.num_rows() {
            let key: String = cols
                .iter()
                .map(|c| format!("{:?}", c.get(i).unwrap_or(Value::Null)))
                .collect::<Vec<_>>()
                .join("|");
            partitions.entry(key).or_default().push(i);
        }

        Ok(partitions)
    }

    /// Apply a with_column expression
    fn apply_with_column(df: DataFrame, name: &str, expr: ColumnExpr) -> DataResult<DataFrame> {
        let values = Self::evaluate_expr(&df, &expr)?;
        let new_series = Series::from_values(name, &values)?;
        df.with_column(new_series)
    }

    /// Evaluate a column expression
    fn evaluate_expr(df: &DataFrame, expr: &ColumnExpr) -> DataResult<Vec<Value>> {
        let num_rows = df.num_rows();
        match expr {
            ColumnExpr::Column(name) => {
                let col = df.column(name)?;
                (0..num_rows).map(|i| col.get(i)).collect()
            }
            ColumnExpr::Literal(val) => Ok(vec![val.clone(); num_rows]),
            ColumnExpr::Add(left, right) => {
                let left_vals = Self::evaluate_expr(df, left)?;
                let right_vals = Self::evaluate_expr(df, right)?;
                Ok(left_vals
                    .into_iter()
                    .zip(right_vals)
                    .map(|(l, r)| Self::add_values(l, r))
                    .collect())
            }
            ColumnExpr::Sub(left, right) => {
                let left_vals = Self::evaluate_expr(df, left)?;
                let right_vals = Self::evaluate_expr(df, right)?;
                Ok(left_vals
                    .into_iter()
                    .zip(right_vals)
                    .map(|(l, r)| Self::sub_values(l, r))
                    .collect())
            }
            ColumnExpr::Mul(left, right) => {
                let left_vals = Self::evaluate_expr(df, left)?;
                let right_vals = Self::evaluate_expr(df, right)?;
                Ok(left_vals
                    .into_iter()
                    .zip(right_vals)
                    .map(|(l, r)| Self::mul_values(l, r))
                    .collect())
            }
            ColumnExpr::Div(left, right) => {
                let left_vals = Self::evaluate_expr(df, left)?;
                let right_vals = Self::evaluate_expr(df, right)?;
                Ok(left_vals
                    .into_iter()
                    .zip(right_vals)
                    .map(|(l, r)| Self::div_values(l, r))
                    .collect())
            }
            ColumnExpr::Mod(left, right) => {
                let left_vals = Self::evaluate_expr(df, left)?;
                let right_vals = Self::evaluate_expr(df, right)?;
                Ok(left_vals
                    .into_iter()
                    .zip(right_vals)
                    .map(|(l, r)| Self::mod_values(l, r))
                    .collect())
            }
            ColumnExpr::Neg(inner) => {
                let vals = Self::evaluate_expr(df, inner)?;
                Ok(vals.into_iter().map(Self::neg_value).collect())
            }
            ColumnExpr::Abs(inner) => {
                let vals = Self::evaluate_expr(df, inner)?;
                Ok(vals.into_iter().map(Self::abs_value).collect())
            }
            ColumnExpr::Concat(exprs) => {
                let all_vals: Vec<Vec<Value>> = exprs
                    .iter()
                    .map(|e| Self::evaluate_expr(df, e))
                    .collect::<DataResult<_>>()?;
                Ok((0..num_rows)
                    .map(|i| {
                        let s: String = all_vals
                            .iter()
                            .map(|vals| Self::value_to_string(&vals[i]))
                            .collect();
                        Value::string(s)
                    })
                    .collect())
            }
            ColumnExpr::Cast(inner, dtype) => {
                let vals = Self::evaluate_expr(df, inner)?;
                Ok(vals
                    .into_iter()
                    .map(|v| Self::cast_value(v, dtype))
                    .collect())
            }
            ColumnExpr::Coalesce(exprs) => {
                let all_vals: Vec<Vec<Value>> = exprs
                    .iter()
                    .map(|e| Self::evaluate_expr(df, e))
                    .collect::<DataResult<_>>()?;
                Ok((0..num_rows)
                    .map(|i| {
                        for vals in &all_vals {
                            if vals[i] != Value::Null {
                                return vals[i].clone();
                            }
                        }
                        Value::Null
                    })
                    .collect())
            }
            ColumnExpr::Case { when_then, otherwise } => {
                let mut result = Self::evaluate_expr(df, otherwise)?;
                // Apply when clauses in reverse order (last match wins)
                for (pred, then_expr) in when_then.iter().rev() {
                    let matching = Self::evaluate_predicate(df, pred)?;
                    let then_vals = Self::evaluate_expr(df, then_expr)?;
                    for idx in matching {
                        result[idx] = then_vals[idx].clone();
                    }
                }
                Ok(result)
            }
        }
    }

    // Value arithmetic helpers
    fn add_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
            (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 + y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x + y as f64),
            (Value::String(x), Value::String(y)) => Value::string(format!("{x}{y}")),
            _ => Value::Null,
        }
    }

    fn sub_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
            (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 - y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x - y as f64),
            _ => Value::Null,
        }
    }

    fn mul_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
            (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
            (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 * y),
            (Value::Float(x), Value::Int(y)) => Value::Float(x * y as f64),
            _ => Value::Null,
        }
    }

    fn div_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) if y != 0 => Value::Float(x as f64 / y as f64),
            (Value::Float(x), Value::Float(y)) if y != 0.0 => Value::Float(x / y),
            (Value::Int(x), Value::Float(y)) if y != 0.0 => Value::Float(x as f64 / y),
            (Value::Float(x), Value::Int(y)) if y != 0 => Value::Float(x / y as f64),
            _ => Value::Null,
        }
    }

    fn mod_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) if y != 0 => Value::Int(x % y),
            (Value::Float(x), Value::Float(y)) if y != 0.0 => Value::Float(x % y),
            _ => Value::Null,
        }
    }

    fn neg_value(v: Value) -> Value {
        match v {
            Value::Int(x) => Value::Int(-x),
            Value::Float(x) => Value::Float(-x),
            _ => Value::Null,
        }
    }

    fn abs_value(v: Value) -> Value {
        match v {
            Value::Int(x) => Value::Int(x.abs()),
            Value::Float(x) => Value::Float(x.abs()),
            _ => Value::Null,
        }
    }

    fn value_to_string(v: &Value) -> String {
        match v {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.to_string(),
            _ => String::new(),
        }
    }

    fn cast_value(v: Value, dtype: &DataType) -> Value {
        match dtype {
            DataType::Int => match v {
                Value::Int(i) => Value::Int(i),
                Value::Float(f) => Value::Int(f as i64),
                Value::String(s) => s.parse().map(Value::Int).unwrap_or(Value::Null),
                Value::Bool(b) => Value::Int(i64::from(b)),
                _ => Value::Null,
            },
            DataType::Float => match v {
                Value::Int(i) => Value::Float(i as f64),
                Value::Float(f) => Value::Float(f),
                Value::String(s) => s.parse().map(Value::Float).unwrap_or(Value::Null),
                Value::Bool(b) => Value::Float(if b { 1.0 } else { 0.0 }),
                _ => Value::Null,
            },
            DataType::String => Value::string(Self::value_to_string(&v)),
            DataType::Bool => match v {
                Value::Bool(b) => Value::Bool(b),
                Value::Int(i) => Value::Bool(i != 0),
                Value::Float(f) => Value::Bool(f != 0.0),
                Value::String(s) => Value::Bool(!s.is_empty()),
                _ => Value::Null,
            },
        }
    }

    /// Get the operations in this LazyFrame (for debugging/introspection)
    #[must_use]
    pub fn operations(&self) -> &[LazyOp] {
        &self.ops
    }

    /// Explain the query plan as a string
    #[must_use]
    pub fn explain(&self) -> String {
        let mut plan = String::new();
        plan.push_str("=== Query Plan ===\n");
        plan.push_str(&format!("Source: {:?}\n", self.source));
        plan.push_str("Operations:\n");
        for (i, op) in self.ops.iter().enumerate() {
            plan.push_str(&format!("  {}. {:?}\n", i + 1, op));
        }
        plan
    }
}

/// Lazy group-by builder for aggregations
#[derive(Debug, Clone)]
pub struct LazyGroupBy {
    source: LazyFrame,
    group_columns: Vec<String>,
    agg_specs: Vec<AggSpec>,
}

impl LazyGroupBy {
    /// Add a sum aggregation
    #[must_use]
    pub fn sum(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::sum(&column.into(), &output_name.into()));
        self
    }

    /// Add a mean aggregation
    #[must_use]
    pub fn mean(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::mean(&column.into(), &output_name.into()));
        self
    }

    /// Add a min aggregation
    #[must_use]
    pub fn min(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::min(&column.into(), &output_name.into()));
        self
    }

    /// Add a max aggregation
    #[must_use]
    pub fn max(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::max(&column.into(), &output_name.into()));
        self
    }

    /// Add a count aggregation
    #[must_use]
    pub fn count(mut self, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::count(&output_name.into()));
        self
    }

    /// Add a first aggregation
    #[must_use]
    pub fn first(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::first(&column.into(), &output_name.into()));
        self
    }

    /// Add a last aggregation
    #[must_use]
    pub fn last(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::last(&column.into(), &output_name.into()));
        self
    }

    /// Add a std (standard deviation) aggregation
    #[must_use]
    pub fn std(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::std(&column.into(), &output_name.into()));
        self
    }

    /// Add a var (variance) aggregation
    #[must_use]
    pub fn var(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::var(&column.into(), &output_name.into()));
        self
    }

    /// Add a median aggregation
    #[must_use]
    pub fn median(mut self, column: impl Into<String>, output_name: impl Into<String>) -> Self {
        self.agg_specs.push(AggSpec::median(&column.into(), &output_name.into()));
        self
    }

    /// Add aggregations from specs
    #[must_use]
    pub fn agg(mut self, specs: impl IntoIterator<Item = AggSpec>) -> Self {
        self.agg_specs.extend(specs);
        self
    }

    /// Execute the group-by and return a LazyFrame
    ///
    /// # Errors
    /// Returns error if aggregation fails
    pub fn collect(self) -> DataResult<DataFrame> {
        use std::sync::Arc;
        use super::grouped::GroupedDataFrame;

        let df = self.source.collect()?;
        let grouped = GroupedDataFrame::new(Arc::new(df), self.group_columns)?;
        grouped.aggregate(&self.agg_specs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_df() -> DataFrame {
        let a = Series::from_values("a", &[Value::Int(1), Value::Int(2), Value::Int(3)]).unwrap();
        let b = Series::from_values("b", &[Value::Int(10), Value::Int(20), Value::Int(30)]).unwrap();
        let c = Series::from_values("c", &[
            Value::string("x"),
            Value::string("y"),
            Value::string("z"),
        ]).unwrap();
        DataFrame::from_series(vec![a, b, c]).unwrap()
    }

    #[test]
    fn test_lazy_select() {
        let df = test_df();
        let result = LazyFrame::new(df)
            .select(["a", "c"])
            .collect()
            .unwrap();
        assert_eq!(result.num_columns(), 2);
        assert_eq!(result.columns(), vec!["a", "c"]);
    }

    #[test]
    fn test_lazy_filter() {
        let df = test_df();
        let result = LazyFrame::new(df)
            .filter_gt("a", Value::Int(1))
            .collect()
            .unwrap();
        assert_eq!(result.num_rows(), 2);
    }

    #[test]
    fn test_lazy_limit() {
        let df = test_df();
        let result = LazyFrame::new(df)
            .limit(2)
            .collect()
            .unwrap();
        assert_eq!(result.num_rows(), 2);
    }

    #[test]
    fn test_lazy_chain() {
        let df = test_df();
        let result = LazyFrame::new(df)
            .filter_gt("a", Value::Int(0))
            .select(["a", "b"])
            .limit(2)
            .collect()
            .unwrap();
        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.num_columns(), 2);
    }

    #[test]
    fn test_lazy_explain() {
        let df = test_df();
        let lf = LazyFrame::new(df)
            .filter_gt("a", Value::Int(0))
            .select(["a"])
            .limit(10);
        let plan = lf.explain();
        assert!(plan.contains("Query Plan"));
        assert!(plan.contains("Filter"));
        assert!(plan.contains("Select"));
        assert!(plan.contains("Limit"));
    }
}
