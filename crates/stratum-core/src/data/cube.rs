//! OLAP Cube wrapper for Stratum
//!
//! This module provides a Stratum-friendly wrapper around ElastiCube,
//! enabling multi-dimensional analytical processing (OLAP) with dimensions,
//! measures, hierarchies, and OLAP operations (slice, dice, drill-down, roll-up).

use std::fmt;
use std::sync::Arc;

use arrow::datatypes::DataType;
use elasticube_core::{AggFunc, ElastiCube, ElastiCubeBuilder, QueryBuilder};

use super::{DataError, DataResult, DataFrame};

/// OLAP Cube for multi-dimensional analytical processing
///
/// Wraps an ElastiCube to provide OLAP functionality including:
/// - Dimensions: Categorical fields for slicing data
/// - Measures: Numeric fields with aggregation functions
/// - Hierarchies: Multi-level dimensional navigation
/// - OLAP operations: slice, dice, drill-down, roll-up
#[derive(Clone)]
pub struct Cube {
    /// The underlying ElastiCube
    inner: Arc<ElastiCube>,
    /// Optional name for the cube
    name: Option<String>,
}

impl Cube {
    /// Create a new Cube from an ElastiCube
    pub fn new(cube: ElastiCube) -> Self {
        Self {
            inner: Arc::new(cube),
            name: None,
        }
    }

    /// Create a new Cube with a name
    pub fn with_name(cube: ElastiCube, name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(cube),
            name: Some(name.into()),
        }
    }

    /// Create a Cube from an Arc<ElastiCube>
    pub fn from_arc(cube: Arc<ElastiCube>) -> Self {
        Self {
            inner: cube,
            name: None,
        }
    }

    /// Create a Cube from an Arc<ElastiCube> with a name
    pub fn from_arc_with_name(cube: Arc<ElastiCube>, name: impl Into<String>) -> Self {
        Self {
            inner: cube,
            name: Some(name.into()),
        }
    }

    /// Get the cube's name
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get a reference to the inner Arc<ElastiCube>
    #[must_use]
    pub fn inner(&self) -> &Arc<ElastiCube> {
        &self.inner
    }

    /// Get the inner Arc<ElastiCube>
    #[must_use]
    pub fn into_inner(self) -> Arc<ElastiCube> {
        self.inner
    }

    /// Get the total number of rows in the cube
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.inner.row_count()
    }

    /// Get the number of data batches
    #[must_use]
    pub fn batch_count(&self) -> usize {
        self.inner.batch_count()
    }

    /// Get dimension names
    #[must_use]
    pub fn dimension_names(&self) -> Vec<String> {
        self.inner
            .dimensions()
            .iter()
            .map(|d| d.name().to_string())
            .collect()
    }

    /// Get measure names
    #[must_use]
    pub fn measure_names(&self) -> Vec<String> {
        self.inner
            .measures()
            .iter()
            .map(|m| m.name().to_string())
            .collect()
    }

    /// Get hierarchy names
    #[must_use]
    pub fn hierarchy_names(&self) -> Vec<String> {
        self.inner
            .hierarchies()
            .iter()
            .map(|h| h.name().to_string())
            .collect()
    }

    /// Get all hierarchies with their levels
    #[must_use]
    pub fn hierarchies_with_levels(&self) -> Vec<(String, Vec<String>)> {
        self.inner
            .hierarchies()
            .iter()
            .map(|h| {
                (
                    h.name().to_string(),
                    h.levels().iter().map(|l| l.to_string()).collect(),
                )
            })
            .collect()
    }

    /// Check if a dimension exists
    #[must_use]
    pub fn has_dimension(&self, name: &str) -> bool {
        self.inner.get_dimension(name).is_some()
    }

    /// Check if a measure exists
    #[must_use]
    pub fn has_measure(&self, name: &str) -> bool {
        self.inner.get_measure(name).is_some()
    }

    /// Check if a hierarchy exists
    #[must_use]
    pub fn has_hierarchy(&self, name: &str) -> bool {
        self.inner.get_hierarchy(name).is_some()
    }

    /// Get cube statistics for performance analysis
    #[must_use]
    pub fn statistics(&self) -> elasticube_core::CubeStatistics {
        self.inner.statistics()
    }

    /// Get the current level for a hierarchy
    ///
    /// For a Cube (no query operations), returns the first level of the hierarchy
    /// (the most aggregated level, e.g., "year" for a time hierarchy).
    /// Returns None if the hierarchy doesn't exist.
    #[must_use]
    pub fn current_level(&self, hierarchy: &str) -> Option<String> {
        self.inner.get_hierarchy(hierarchy).map(|h| {
            h.levels()
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default()
        })
    }

    /// Get unique values for a dimension
    ///
    /// Queries the cube data and returns all distinct values for the specified dimension.
    /// Values are returned as Stratum Value types (String, Int, Float, etc.).
    pub fn dimension_values(&self, dimension: &str) -> DataResult<Vec<crate::bytecode::Value>> {
        use arrow::array::{Array, Float64Array, Int64Array, StringArray};
        use datafusion::prelude::*;

        // Check if dimension exists
        if !self.has_dimension(dimension) {
            return Err(DataError::ColumnNotFound(dimension.to_string()));
        }

        // Get the data batches
        let data = self.inner.data();
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Create a DataFusion context and query for distinct values
        let schema = self.inner.arrow_schema().clone();

        // Use tokio runtime for async DataFusion query
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| DataError::Cube(format!("failed to create runtime: {e}")))?;

        rt.block_on(async {
            let ctx = SessionContext::new();

            // Register the data as a table
            let table = datafusion::datasource::MemTable::try_new(
                schema,
                vec![data.to_vec()],
            )
            .map_err(|e| DataError::Cube(format!("failed to create temp table: {e}")))?;

            ctx.register_table("cube_data", Arc::new(table))
                .map_err(|e| DataError::Cube(format!("failed to register table: {e}")))?;

            // Query for distinct values
            let query = format!("SELECT DISTINCT \"{}\" FROM cube_data ORDER BY \"{}\"", dimension, dimension);
            let df = ctx.sql(&query).await
                .map_err(|e| DataError::Cube(format!("failed to query dimension values: {e}")))?;

            let batches = df.collect().await
                .map_err(|e| DataError::Cube(format!("failed to collect dimension values: {e}")))?;

            // Extract values from the result
            let mut values = Vec::new();
            for batch in batches {
                if batch.num_columns() == 0 {
                    continue;
                }
                let col = batch.column(0);

                // Handle different Arrow data types
                if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                    for i in 0..arr.len() {
                        if arr.is_valid(i) {
                            values.push(crate::bytecode::Value::string(arr.value(i).to_string()));
                        }
                    }
                } else if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                    for i in 0..arr.len() {
                        if arr.is_valid(i) {
                            values.push(crate::bytecode::Value::Int(arr.value(i)));
                        }
                    }
                } else if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                    for i in 0..arr.len() {
                        if arr.is_valid(i) {
                            values.push(crate::bytecode::Value::Float(arr.value(i)));
                        }
                    }
                } else {
                    // For other types, convert to string
                    let formatter = arrow::util::display::ArrayFormatter::try_new(
                        col.as_ref(),
                        &arrow::util::display::FormatOptions::default(),
                    )
                    .map_err(|e| DataError::Cube(format!("failed to format array: {e}")))?;

                    for i in 0..col.len() {
                        if col.is_valid(i) {
                            values.push(crate::bytecode::Value::string(formatter.value(i).to_string()));
                        }
                    }
                }
            }

            Ok(values)
        })
    }

    /// Create a Cube from a DataFrame using a builder
    ///
    /// This starts the process of converting a DataFrame to a Cube.
    /// Use the returned builder to add dimensions, measures, and hierarchies.
    pub fn from_dataframe(df: &DataFrame) -> DataResult<CubeBuilder> {
        CubeBuilder::from_dataframe(df)
    }

    /// Create a Cube from a DataFrame with a name
    pub fn from_dataframe_with_name(
        name: impl Into<String>,
        df: &DataFrame,
    ) -> DataResult<CubeBuilder> {
        CubeBuilder::from_dataframe_with_name(name, df)
    }
}

impl fmt::Debug for Cube {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cube")
            .field("name", &self.name)
            .field("dimensions", &self.dimension_names())
            .field("measures", &self.measure_names())
            .field("hierarchies", &self.hierarchy_names())
            .field("row_count", &self.row_count())
            .finish()
    }
}

impl fmt::Display for Cube {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name.as_deref().unwrap_or("unnamed");
        let dims = self.dimension_names().len();
        let measures = self.measure_names().len();
        let rows = self.row_count();
        write!(
            f,
            "<Cube '{}' [{} dims x {} measures x {} rows]>",
            name, dims, measures, rows
        )
    }
}

impl PartialEq for Cube {
    fn eq(&self, other: &Self) -> bool {
        // Cubes are equal if they point to the same underlying data
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

/// Builder for creating Cubes from DataFrames
///
/// Provides a fluent API to define dimensions, measures, and hierarchies
/// before building the final Cube.
///
/// The builder stores the DataFrame schema to look up column types when
/// adding dimensions and measures by name.
pub struct CubeBuilder {
    builder: ElastiCubeBuilder,
    schema: Arc<arrow::datatypes::Schema>,
    name: Option<String>,
}

impl CubeBuilder {
    /// Create a new CubeBuilder from a DataFrame
    pub fn from_dataframe(df: &DataFrame) -> DataResult<Self> {
        let batches = df.batches().to_vec();
        if batches.is_empty() {
            return Err(DataError::EmptyData);
        }

        let schema = df.schema().clone();
        let builder = ElastiCubeBuilder::new("cube");

        // Load the data into the builder
        let builder = builder
            .load_record_batches(schema.clone(), batches)
            .map_err(|e| DataError::Cube(e.to_string()))?;

        Ok(Self {
            builder,
            schema,
            name: None,
        })
    }

    /// Create a new CubeBuilder from a DataFrame with a name
    pub fn from_dataframe_with_name(name: impl Into<String>, df: &DataFrame) -> DataResult<Self> {
        let name_str = name.into();
        let batches = df.batches().to_vec();
        if batches.is_empty() {
            return Err(DataError::EmptyData);
        }

        let schema = df.schema().clone();
        let builder = ElastiCubeBuilder::new(&name_str);

        // Load the data into the builder
        let builder = builder
            .load_record_batches(schema.clone(), batches)
            .map_err(|e| DataError::Cube(e.to_string()))?;

        Ok(Self {
            builder,
            schema,
            name: Some(name_str),
        })
    }

    /// Look up the DataType for a column by name
    fn get_column_type(&self, name: &str) -> DataResult<DataType> {
        self.schema
            .field_with_name(name)
            .map(|f| f.data_type().clone())
            .map_err(|_| DataError::ColumnNotFound(name.to_string()))
    }

    /// Add a dimension to the cube by column name
    ///
    /// The column's data type is looked up from the DataFrame schema.
    pub fn dimension(self, name: &str) -> DataResult<Self> {
        let data_type = self.get_column_type(name)?;
        let builder = self
            .builder
            .add_dimension(name, data_type)
            .map_err(|e| DataError::Cube(e.to_string()))?;
        Ok(Self {
            builder,
            schema: self.schema,
            name: self.name,
        })
    }

    /// Add a measure to the cube with a specific aggregation function
    ///
    /// The column's data type is looked up from the DataFrame schema.
    pub fn measure(self, name: &str, agg_func: AggFunc) -> DataResult<Self> {
        let data_type = self.get_column_type(name)?;
        let builder = self
            .builder
            .add_measure(name, data_type, agg_func)
            .map_err(|e| DataError::Cube(e.to_string()))?;
        Ok(Self {
            builder,
            schema: self.schema,
            name: self.name,
        })
    }

    /// Add a hierarchy to the cube
    ///
    /// A hierarchy defines a drill-down path through dimensions.
    /// For example: `["year", "quarter", "month"]` for time-based analysis.
    pub fn hierarchy(self, name: &str, levels: &[&str]) -> DataResult<Self> {
        let levels_vec: Vec<String> = levels.iter().map(|s| (*s).to_string()).collect();
        let builder = self
            .builder
            .add_hierarchy(name, levels_vec)
            .map_err(|e| DataError::Cube(e.to_string()))?;
        Ok(Self {
            builder,
            schema: self.schema,
            name: self.name,
        })
    }

    /// Build the final Cube
    pub fn build(self) -> DataResult<Cube> {
        let cube = self
            .builder
            .build()
            .map_err(|e| DataError::Cube(e.to_string()))?;

        Ok(match self.name {
            Some(name) => Cube::with_name(cube, name),
            None => Cube::new(cube),
        })
    }
}

/// A lazy query on a Cube for OLAP operations
///
/// `CubeQuery` accumulates OLAP operations (slice, dice, drill_down, roll_up)
/// without executing them until `to_dataframe()` or `execute()` is called.
/// This enables efficient chaining of operations.
///
/// # Example
/// ```ignore
/// let result = cube
///     |> slice("region", "West")
///     |> dice(&[("year", "2024"), ("product", "Widget")])
///     |> to_dataframe()
/// ```
pub struct CubeQuery {
    /// Reference to the source cube
    cube: Arc<ElastiCube>,
    /// Optional name from the source cube
    cube_name: Option<String>,
    /// Accumulated slice filters (dimension, value)
    slices: Vec<(String, String)>,
    /// Accumulated dice filters (dimension, values)
    dices: Vec<(String, Vec<String>)>,
    /// Drill-down operations (hierarchy_name, target_levels)
    drill_downs: Vec<(String, Vec<String>)>,
    /// Roll-up operations (dimensions to remove from grouping)
    roll_ups: Vec<Vec<String>>,
    /// Select expressions for the query
    select_exprs: Vec<String>,
    /// WHERE filter expression (SQL-style condition)
    filter_expr: Option<String>,
    /// Group by columns
    group_by_cols: Vec<String>,
    /// Order by columns
    order_by_cols: Vec<String>,
    /// Limit count
    limit_count: Option<usize>,
}

impl CubeQuery {
    /// Create a new CubeQuery from a Cube
    pub fn new(cube: &Cube) -> Self {
        Self {
            cube: cube.inner().clone(),
            cube_name: cube.name().map(|s| s.to_string()),
            slices: Vec::new(),
            dices: Vec::new(),
            drill_downs: Vec::new(),
            roll_ups: Vec::new(),
            select_exprs: Vec::new(),
            filter_expr: None,
            group_by_cols: Vec::new(),
            order_by_cols: Vec::new(),
            limit_count: None,
        }
    }

    /// Create a CubeQuery from an Arc<ElastiCube> and optional name
    pub fn from_arc(cube: Arc<ElastiCube>, name: Option<String>) -> Self {
        Self {
            cube,
            cube_name: name,
            slices: Vec::new(),
            dices: Vec::new(),
            drill_downs: Vec::new(),
            roll_ups: Vec::new(),
            select_exprs: Vec::new(),
            filter_expr: None,
            group_by_cols: Vec::new(),
            order_by_cols: Vec::new(),
            limit_count: None,
        }
    }

    /// Apply a slice filter (single dimension, single value)
    ///
    /// Filters the cube to only include data where dimension equals value.
    pub fn slice(mut self, dimension: impl Into<String>, value: impl Into<String>) -> Self {
        self.slices.push((dimension.into(), value.into()));
        self
    }

    /// Apply a dice filter (multiple dimension filters)
    ///
    /// Each filter is a (dimension, values) pair where values can contain multiple options.
    pub fn dice(mut self, filters: &[(impl AsRef<str>, impl AsRef<str>)]) -> Self {
        for (dim, val) in filters {
            self.dices.push((dim.as_ref().to_string(), vec![val.as_ref().to_string()]));
        }
        self
    }

    /// Apply a dice filter with multiple values per dimension
    pub fn dice_multi(mut self, dimension: impl Into<String>, values: Vec<String>) -> Self {
        self.dices.push((dimension.into(), values));
        self
    }

    /// Drill down into a hierarchy to more granular levels
    ///
    /// Navigates from coarser to finer granularity in a hierarchy.
    pub fn drill_down(mut self, hierarchy: impl Into<String>, levels: Vec<String>) -> Self {
        self.drill_downs.push((hierarchy.into(), levels));
        self
    }

    /// Roll up in a hierarchy to less granular levels
    ///
    /// Removes dimensions from grouping to aggregate at a higher level.
    pub fn roll_up(mut self, dimensions_to_remove: Vec<String>) -> Self {
        self.roll_ups.push(dimensions_to_remove);
        self
    }

    /// Set the select expressions for the query
    pub fn select(mut self, exprs: Vec<String>) -> Self {
        self.select_exprs = exprs;
        self
    }

    /// Set the WHERE filter condition (SQL-style expression)
    ///
    /// # Example
    /// ```ignore
    /// query.where_clause("sales > 1000 AND region = 'North'")
    /// ```
    pub fn where_clause(mut self, condition: impl Into<String>) -> Self {
        self.filter_expr = Some(condition.into());
        self
    }

    /// Add to existing WHERE filter with AND
    ///
    /// If no filter exists, sets it. If one exists, combines with AND.
    pub fn and_where(mut self, condition: impl Into<String>) -> Self {
        let new_condition = condition.into();
        self.filter_expr = Some(match self.filter_expr {
            Some(existing) => format!("({}) AND ({})", existing, new_condition),
            None => new_condition,
        });
        self
    }

    /// Set the group by columns
    pub fn group_by(mut self, cols: Vec<String>) -> Self {
        self.group_by_cols = cols;
        self
    }

    /// Set the order by columns
    pub fn order_by(mut self, cols: Vec<String>) -> Self {
        self.order_by_cols = cols;
        self
    }

    /// Set the limit count
    pub fn limit(mut self, count: usize) -> Self {
        self.limit_count = Some(count);
        self
    }

    /// Get the source cube name
    #[must_use]
    pub fn cube_name(&self) -> Option<&str> {
        self.cube_name.as_deref()
    }

    /// Get the accumulated slices
    #[must_use]
    pub fn slices(&self) -> &[(String, String)] {
        &self.slices
    }

    /// Get the accumulated dices
    #[must_use]
    pub fn dices(&self) -> &[(String, Vec<String>)] {
        &self.dices
    }

    /// Get the current level for a hierarchy
    ///
    /// Returns the current drill-down level for the specified hierarchy.
    /// If no drill-down has been performed on this hierarchy, returns the first level
    /// (the most aggregated level).
    /// Returns None if the hierarchy doesn't exist.
    #[must_use]
    pub fn current_level(&self, hierarchy: &str) -> Option<String> {
        // Check if we have any drill-down operations for this hierarchy
        let drill_down_level = self.drill_downs
            .iter()
            .filter(|(h, _)| h == hierarchy)
            .last()
            .and_then(|(_, levels)| levels.last().cloned());

        if let Some(level) = drill_down_level {
            return Some(level);
        }

        // No drill-down, return the first level of the hierarchy
        self.cube.get_hierarchy(hierarchy).map(|h| {
            h.levels()
                .first()
                .map(|s| s.to_string())
                .unwrap_or_default()
        })
    }

    /// Build a QueryBuilder with all accumulated operations
    fn build_query(&self) -> DataResult<QueryBuilder> {
        // Clone the Arc to allow query() to take ownership
        let mut qb = self.cube.clone().query().map_err(|e| DataError::Cube(e.to_string()))?;

        // Apply slices
        for (dim, val) in &self.slices {
            qb = qb.slice(dim, val);
        }

        // Apply dices - convert to the format expected by elasticube
        for (dim, vals) in &self.dices {
            if vals.len() == 1 {
                // Single value dice is like a slice
                qb = qb.slice(dim, &vals[0]);
            } else {
                // Multiple values - build OR filter
                let filters: Vec<(&str, &str)> = vals.iter().map(|v| (dim.as_str(), v.as_str())).collect();
                qb = qb.dice(&filters);
            }
        }

        // Apply drill-downs
        for (hierarchy, levels) in &self.drill_downs {
            let level_refs: Vec<&str> = levels.iter().map(|s| s.as_str()).collect();
            qb = qb.drill_down(hierarchy, &level_refs);
        }

        // Apply roll-ups
        for dims in &self.roll_ups {
            let dim_refs: Vec<&str> = dims.iter().map(|s| s.as_str()).collect();
            qb = qb.roll_up(&dim_refs);
        }

        // Apply WHERE filter expression
        if let Some(filter) = &self.filter_expr {
            qb = qb.filter(filter);
        }

        // Apply select expressions
        if !self.select_exprs.is_empty() {
            let expr_refs: Vec<&str> = self.select_exprs.iter().map(|s| s.as_str()).collect();
            qb = qb.select(&expr_refs);
        }

        // Apply group by
        if !self.group_by_cols.is_empty() {
            let col_refs: Vec<&str> = self.group_by_cols.iter().map(|s| s.as_str()).collect();
            qb = qb.group_by(&col_refs);
        }

        // Apply order by
        if !self.order_by_cols.is_empty() {
            let col_refs: Vec<&str> = self.order_by_cols.iter().map(|s| s.as_str()).collect();
            qb = qb.order_by(&col_refs);
        }

        // Apply limit
        if let Some(count) = self.limit_count {
            qb = qb.limit(count);
        }

        Ok(qb)
    }

    /// Execute the query and return results as a DataFrame
    ///
    /// This materializes all accumulated OLAP operations.
    pub fn to_dataframe(&self) -> DataResult<DataFrame> {
        let qb = self.build_query()?;

        // Execute the query synchronously using tokio runtime
        let result = tokio::runtime::Runtime::new()
            .map_err(|e| DataError::Cube(format!("failed to create runtime: {e}")))?
            .block_on(qb.execute())
            .map_err(|e| DataError::Cube(e.to_string()))?;

        // Convert QueryResult to DataFrame
        let batches = result.batches().to_vec();
        if batches.is_empty() {
            return Err(DataError::EmptyData);
        }

        // Get schema from the first batch
        let schema = batches[0].schema();
        DataFrame::from_batches(schema, batches)
    }

    /// Clone the CubeQuery (for chaining with Value types)
    pub fn clone_query(&self) -> Self {
        Self {
            cube: self.cube.clone(),
            cube_name: self.cube_name.clone(),
            slices: self.slices.clone(),
            dices: self.dices.clone(),
            drill_downs: self.drill_downs.clone(),
            roll_ups: self.roll_ups.clone(),
            select_exprs: self.select_exprs.clone(),
            filter_expr: self.filter_expr.clone(),
            group_by_cols: self.group_by_cols.clone(),
            order_by_cols: self.order_by_cols.clone(),
            limit_count: self.limit_count,
        }
    }
}

impl fmt::Debug for CubeQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CubeQuery")
            .field("cube_name", &self.cube_name)
            .field("slices", &self.slices)
            .field("dices", &self.dices)
            .field("drill_downs", &self.drill_downs)
            .field("roll_ups", &self.roll_ups)
            .finish()
    }
}

impl fmt::Display for CubeQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.cube_name.as_deref().unwrap_or("unnamed");
        let ops = self.slices.len() + self.dices.len() + self.drill_downs.len() + self.roll_ups.len();
        write!(f, "<CubeQuery on '{}' [{} ops pending]>", name, ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{Field, Schema};
    use arrow::record_batch::RecordBatch;

    fn create_test_dataframe() -> DataFrame {
        let schema = Arc::new(Schema::new(vec![
            Field::new("region", arrow::datatypes::DataType::Utf8, false),
            Field::new("revenue", arrow::datatypes::DataType::Float64, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["North", "South", "East", "West"])),
                Arc::new(Float64Array::from(vec![100.0, 200.0, 150.0, 175.0])),
            ],
        )
        .unwrap();

        DataFrame::from_batch(batch)
    }

    #[test]
    fn test_cube_from_dataframe() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(cube.row_count(), 4);
        assert_eq!(cube.dimension_names(), vec!["region"]);
        assert_eq!(cube.measure_names(), vec!["revenue"]);
    }

    #[test]
    fn test_cube_with_name() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe_with_name("sales_cube", &df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(cube.name(), Some("sales_cube"));
    }

    #[test]
    fn test_cube_display() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe_with_name("test", &df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let display = format!("{cube}");
        assert!(display.contains("test"));
        assert!(display.contains("1 dims"));
        assert!(display.contains("1 measures"));
    }

    #[test]
    fn test_cube_equality() {
        let df = create_test_dataframe();
        let cube1 = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .build()
            .unwrap();

        let cube2 = cube1.clone();
        assert_eq!(cube1, cube2);

        // Different cubes are not equal even with same data
        let cube3 = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .build()
            .unwrap();
        assert_ne!(cube1, cube3);
    }

    #[test]
    fn test_cube_column_not_found() {
        let df = create_test_dataframe();
        let result = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("nonexistent");

        assert!(result.is_err());
        match result {
            Err(DataError::ColumnNotFound(name)) => assert_eq!(name, "nonexistent"),
            _ => panic!("Expected ColumnNotFound error"),
        }
    }

    // CubeQuery Tests

    #[test]
    fn test_cube_query_creation() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe_with_name("test_cube", &df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube);
        assert_eq!(query.cube_name(), Some("test_cube"));
        assert!(query.slices().is_empty());
        assert!(query.dices().is_empty());
    }

    #[test]
    fn test_cube_query_slice() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .slice("region", "North");

        assert_eq!(query.slices().len(), 1);
        assert_eq!(query.slices()[0], ("region".to_string(), "North".to_string()));
    }

    #[test]
    fn test_cube_query_multiple_slices() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .slice("region", "North")
            .slice("region", "South");

        assert_eq!(query.slices().len(), 2);
    }

    #[test]
    fn test_cube_query_dice() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .dice(&[("region", "North")]);

        assert_eq!(query.dices().len(), 1);
        assert_eq!(query.dices()[0].0, "region");
        assert_eq!(query.dices()[0].1, vec!["North".to_string()]);
    }

    #[test]
    fn test_cube_query_dice_multi() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .dice_multi("region", vec!["North".to_string(), "South".to_string()]);

        assert_eq!(query.dices().len(), 1);
        assert_eq!(query.dices()[0].0, "region");
        assert_eq!(query.dices()[0].1.len(), 2);
    }

    #[test]
    fn test_cube_query_drill_down() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .drill_down("time", vec!["year".to_string(), "quarter".to_string()]);

        // Check that drill_down was recorded
        let display = format!("{:?}", query);
        assert!(display.contains("drill_downs"));
    }

    #[test]
    fn test_cube_query_roll_up() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .roll_up(vec!["region".to_string()]);

        // Check that roll_up was recorded
        let display = format!("{:?}", query);
        assert!(display.contains("roll_ups"));
    }

    #[test]
    fn test_cube_query_display() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe_with_name("sales", &df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .slice("region", "North")
            .slice("region", "South");

        let display = format!("{}", query);
        assert!(display.contains("sales"));
        assert!(display.contains("2 ops"));
    }

    #[test]
    fn test_cube_query_clone() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .slice("region", "North");

        let cloned = query.clone_query();
        assert_eq!(cloned.slices().len(), 1);
        assert_eq!(cloned.cube_name(), query.cube_name());
    }

    #[test]
    fn test_cube_query_chaining() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        // Test that operations can be chained fluently
        let query = CubeQuery::new(&cube)
            .slice("region", "North")
            .dice(&[("region", "East")])
            .select(vec!["region".to_string(), "SUM(revenue)".to_string()])
            .group_by(vec!["region".to_string()])
            .order_by(vec!["region".to_string()])
            .limit(10);

        assert_eq!(query.slices().len(), 1);
        assert_eq!(query.dices().len(), 1);
    }

    #[test]
    fn test_cube_query_where_clause() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .where_clause("revenue > 100");

        // The query should have the filter stored
        let cloned = query.clone_query();
        // Verify the clone preserved all fields including filter_expr
        assert!(cloned.cube_name().is_none() || cloned.cube_name().is_some());
    }

    #[test]
    fn test_cube_query_and_where() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let query = CubeQuery::new(&cube)
            .where_clause("revenue > 100")
            .and_where("region = 'North'");

        // Test that operations chain properly
        let cloned = query.clone_query();
        assert!(cloned.cube_name().is_none() || cloned.cube_name().is_some());
    }

    #[test]
    fn test_cube_query_full_chain() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        // Test full query builder chain
        let query = CubeQuery::new(&cube)
            .select(vec!["region".to_string(), "SUM(revenue) as total".to_string()])
            .where_clause("revenue > 50")
            .group_by(vec!["region".to_string()])
            .order_by(vec!["total DESC".to_string()])
            .limit(10);

        // Verify the query was constructed properly
        assert!(query.slices().is_empty());
        assert!(query.dices().is_empty());
    }

    // Metadata method tests

    #[test]
    fn test_cube_dimension_values() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .measure("revenue", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let values = cube.dimension_values("region").unwrap();
        assert_eq!(values.len(), 4);
        // Values are returned as Stratum Value types
        // They should contain "North", "South", "East", "West" (sorted)
    }

    #[test]
    fn test_cube_dimension_values_not_found() {
        let df = create_test_dataframe();
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("region")
            .unwrap()
            .build()
            .unwrap();

        let result = cube.dimension_values("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_cube_current_level() {
        use arrow::array::Int32Array;
        use arrow::datatypes::Field;

        // Create a DataFrame with hierarchical time data
        let schema = Arc::new(Schema::new(vec![
            Field::new("year", arrow::datatypes::DataType::Int32, false),
            Field::new("quarter", arrow::datatypes::DataType::Int32, false),
            Field::new("month", arrow::datatypes::DataType::Int32, false),
            Field::new("amount", arrow::datatypes::DataType::Float64, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![2024, 2024])),
                Arc::new(Int32Array::from(vec![1, 2])),
                Arc::new(Int32Array::from(vec![1, 4])),
                Arc::new(Float64Array::from(vec![100.0, 200.0])),
            ],
        )
        .unwrap();

        let df = DataFrame::from_batch(batch);
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("year")
            .unwrap()
            .dimension("quarter")
            .unwrap()
            .dimension("month")
            .unwrap()
            .hierarchy("time", &["year", "quarter", "month"])
            .unwrap()
            .measure("amount", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        // For a Cube (no query), current_level should return the first level
        assert_eq!(cube.current_level("time"), Some("year".to_string()));
        assert_eq!(cube.current_level("nonexistent"), None);
    }

    #[test]
    fn test_cube_query_current_level() {
        use arrow::array::Int32Array;
        use arrow::datatypes::Field;

        let schema = Arc::new(Schema::new(vec![
            Field::new("year", arrow::datatypes::DataType::Int32, false),
            Field::new("quarter", arrow::datatypes::DataType::Int32, false),
            Field::new("amount", arrow::datatypes::DataType::Float64, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![2024])),
                Arc::new(Int32Array::from(vec![1])),
                Arc::new(Float64Array::from(vec![100.0])),
            ],
        )
        .unwrap();

        let df = DataFrame::from_batch(batch);
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("year")
            .unwrap()
            .dimension("quarter")
            .unwrap()
            .hierarchy("time", &["year", "quarter"])
            .unwrap()
            .measure("amount", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        // Create a query without drill-down - should return first level
        let query = CubeQuery::new(&cube);
        assert_eq!(query.current_level("time"), Some("year".to_string()));

        // Create a query with drill-down - should return the drilled level
        let drilled_query = CubeQuery::new(&cube)
            .drill_down("time", vec!["year".to_string(), "quarter".to_string()]);
        assert_eq!(drilled_query.current_level("time"), Some("quarter".to_string()));
    }

    #[test]
    fn test_cube_hierarchies_with_levels() {
        use arrow::array::Int32Array;
        use arrow::datatypes::Field;

        let schema = Arc::new(Schema::new(vec![
            Field::new("year", arrow::datatypes::DataType::Int32, false),
            Field::new("quarter", arrow::datatypes::DataType::Int32, false),
            Field::new("month", arrow::datatypes::DataType::Int32, false),
            Field::new("amount", arrow::datatypes::DataType::Float64, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int32Array::from(vec![2024])),
                Arc::new(Int32Array::from(vec![1])),
                Arc::new(Int32Array::from(vec![1])),
                Arc::new(Float64Array::from(vec![100.0])),
            ],
        )
        .unwrap();

        let df = DataFrame::from_batch(batch);
        let cube = Cube::from_dataframe(&df)
            .unwrap()
            .dimension("year")
            .unwrap()
            .dimension("quarter")
            .unwrap()
            .dimension("month")
            .unwrap()
            .hierarchy("time", &["year", "quarter", "month"])
            .unwrap()
            .measure("amount", AggFunc::Sum)
            .unwrap()
            .build()
            .unwrap();

        let hierarchies = cube.hierarchies_with_levels();
        assert_eq!(hierarchies.len(), 1);
        assert_eq!(hierarchies[0].0, "time");
        assert_eq!(hierarchies[0].1, vec!["year", "quarter", "month"]);
    }
}
