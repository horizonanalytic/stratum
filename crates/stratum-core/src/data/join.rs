//! Join specifications for DataFrame operations

use std::collections::HashMap;

use super::dataframe::DataFrame;
use super::error::{DataError, DataResult};
use super::series::Series;
use crate::bytecode::Value;

/// Type of join operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    /// Inner join - only matching rows from both DataFrames
    Inner,
    /// Left join - all rows from left, matching from right
    Left,
    /// Right join - matching from left, all rows from right
    Right,
    /// Outer join - all rows from both DataFrames
    Outer,
}

impl JoinType {
    /// Get the join type name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            JoinType::Inner => "inner",
            JoinType::Left => "left",
            JoinType::Right => "right",
            JoinType::Outer => "outer",
        }
    }
}

/// Join specification - describes how to join two DataFrames
#[derive(Debug, Clone, PartialEq)]
pub struct JoinSpec {
    /// The type of join to perform
    pub join_type: JoinType,
    /// The column name in the left DataFrame
    pub left_column: String,
    /// The column name in the right DataFrame
    pub right_column: String,
}

impl JoinSpec {
    /// Create a new join spec with the same column name in both DataFrames
    #[must_use]
    pub fn on(column: &str) -> Self {
        Self {
            join_type: JoinType::Inner,
            left_column: column.to_string(),
            right_column: column.to_string(),
        }
    }

    /// Create a new join spec with different column names
    #[must_use]
    pub fn cols(left: &str, right: &str) -> Self {
        Self {
            join_type: JoinType::Inner,
            left_column: left.to_string(),
            right_column: right.to_string(),
        }
    }

    /// Create an inner join on the same column name
    #[must_use]
    pub fn inner(column: &str) -> Self {
        Self {
            join_type: JoinType::Inner,
            left_column: column.to_string(),
            right_column: column.to_string(),
        }
    }

    /// Create an inner join with different column names
    #[must_use]
    pub fn inner_cols(left: &str, right: &str) -> Self {
        Self {
            join_type: JoinType::Inner,
            left_column: left.to_string(),
            right_column: right.to_string(),
        }
    }

    /// Create a left join on the same column name
    #[must_use]
    pub fn left(column: &str) -> Self {
        Self {
            join_type: JoinType::Left,
            left_column: column.to_string(),
            right_column: column.to_string(),
        }
    }

    /// Create a left join with different column names
    #[must_use]
    pub fn left_cols(left: &str, right: &str) -> Self {
        Self {
            join_type: JoinType::Left,
            left_column: left.to_string(),
            right_column: right.to_string(),
        }
    }

    /// Create a right join on the same column name
    #[must_use]
    pub fn right(column: &str) -> Self {
        Self {
            join_type: JoinType::Right,
            left_column: column.to_string(),
            right_column: column.to_string(),
        }
    }

    /// Create a right join with different column names
    #[must_use]
    pub fn right_cols(left: &str, right: &str) -> Self {
        Self {
            join_type: JoinType::Right,
            left_column: left.to_string(),
            right_column: right.to_string(),
        }
    }

    /// Create an outer join on the same column name
    #[must_use]
    pub fn outer(column: &str) -> Self {
        Self {
            join_type: JoinType::Outer,
            left_column: column.to_string(),
            right_column: column.to_string(),
        }
    }

    /// Create an outer join with different column names
    #[must_use]
    pub fn outer_cols(left: &str, right: &str) -> Self {
        Self {
            join_type: JoinType::Outer,
            left_column: left.to_string(),
            right_column: right.to_string(),
        }
    }
}

/// A value that can be used as a join key (for building hash maps)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum JoinKey {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
}

impl JoinKey {
    fn from_value(value: &Value) -> DataResult<Self> {
        match value {
            Value::Null => Ok(JoinKey::Null),
            Value::Bool(b) => Ok(JoinKey::Bool(*b)),
            Value::Int(i) => Ok(JoinKey::Int(*i)),
            Value::String(s) => Ok(JoinKey::String(s.as_ref().clone())),
            _ => Err(DataError::InvalidOperation(format!(
                "cannot use {} as a join key",
                value.type_name()
            ))),
        }
    }
}

impl DataFrame {
    /// Join this DataFrame with another using the given specification
    ///
    /// # Errors
    /// Returns error if join columns don't exist or types are incompatible
    pub fn join(&self, other: &DataFrame, spec: &JoinSpec) -> DataResult<DataFrame> {
        // Validate that join columns exist
        let left_col = self.column(&spec.left_column)?;
        let right_col = other.column(&spec.right_column)?;

        // Build hash map from right DataFrame
        let mut right_map: HashMap<JoinKey, Vec<usize>> = HashMap::new();
        for idx in 0..other.num_rows() {
            let val = right_col.get(idx)?;
            let key = JoinKey::from_value(&val)?;
            right_map.entry(key).or_default().push(idx);
        }

        // Track which right rows were matched (for right/outer joins)
        let mut right_matched = vec![false; other.num_rows()];

        // Collect matching row pairs
        let mut left_indices: Vec<Option<usize>> = Vec::new();
        let mut right_indices: Vec<Option<usize>> = Vec::new();

        // Process left rows
        for left_idx in 0..self.num_rows() {
            let val = left_col.get(left_idx)?;
            let key = JoinKey::from_value(&val)?;

            if let Some(matching_right_indices) = right_map.get(&key) {
                // Found matches
                for &right_idx in matching_right_indices {
                    left_indices.push(Some(left_idx));
                    right_indices.push(Some(right_idx));
                    right_matched[right_idx] = true;
                }
            } else if spec.join_type == JoinType::Left || spec.join_type == JoinType::Outer {
                // No match, but include left row with null right
                left_indices.push(Some(left_idx));
                right_indices.push(None);
            }
            // For inner/right joins, unmatched left rows are dropped
        }

        // For right/outer joins, add unmatched right rows
        if spec.join_type == JoinType::Right || spec.join_type == JoinType::Outer {
            for (right_idx, matched) in right_matched.iter().enumerate() {
                if !*matched {
                    left_indices.push(None);
                    right_indices.push(Some(right_idx));
                }
            }
        }

        // Build result columns
        let mut result_columns: Vec<Series> = Vec::new();

        // Add all columns from left DataFrame
        // Special handling for the join column: when left is null, use right's value
        for col_idx in 0..self.num_columns() {
            let col = self.column_by_index(col_idx)?;
            let col_name = col.name();
            let is_join_column = col_name == spec.left_column;

            let values: Vec<Value> = left_indices
                .iter()
                .zip(right_indices.iter())
                .map(|(left_opt, right_opt)| {
                    if let Some(left_idx) = left_opt {
                        col.get(*left_idx)
                    } else if is_join_column && spec.left_column == spec.right_column {
                        // For unmatched right rows, use right's join column value
                        if let Some(right_idx) = right_opt {
                            right_col.get(*right_idx)
                        } else {
                            Ok(Value::Null)
                        }
                    } else {
                        Ok(Value::Null)
                    }
                })
                .collect::<DataResult<Vec<_>>>()?;
            let new_series = Series::from_values(col_name, &values)?;
            result_columns.push(new_series);
        }

        // Add columns from right DataFrame (excluding the join column if same name)
        let left_columns: Vec<String> = self.columns();
        for col_idx in 0..other.num_columns() {
            let col = other.column_by_index(col_idx)?;
            let col_name = col.name();

            // Skip the right join column if it has the same name as the left
            if col_name == spec.right_column && spec.left_column == spec.right_column {
                continue;
            }

            // Handle column name conflicts by adding suffix
            let output_name = if left_columns.contains(&col_name.to_string()) {
                format!("{col_name}_right")
            } else {
                col_name.to_string()
            };

            let values: Vec<Value> = right_indices
                .iter()
                .map(|opt_idx| opt_idx.map_or(Ok(Value::Null), |idx| col.get(idx)))
                .collect::<DataResult<Vec<_>>>()?;
            let new_series = Series::from_values(&output_name, &values)?;
            result_columns.push(new_series);
        }

        DataFrame::from_series(result_columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn users_df() -> DataFrame {
        let ids = Series::from_ints("user_id", vec![1, 2, 3, 4]);
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie", "Diana"]);
        DataFrame::from_series(vec![ids, names]).unwrap()
    }

    fn orders_df() -> DataFrame {
        let user_ids = Series::from_ints("user_id", vec![1, 1, 2, 5]);
        let amounts = Series::from_ints("amount", vec![100, 150, 200, 50]);
        DataFrame::from_series(vec![user_ids, amounts]).unwrap()
    }

    #[test]
    fn test_inner_join() {
        let users = users_df();
        let orders = orders_df();
        let spec = JoinSpec::on("user_id");

        let result = users.join(&orders, &spec).unwrap();

        // Inner join: only users 1 and 2 have orders
        // User 1 has 2 orders, user 2 has 1 order = 3 rows
        assert_eq!(result.num_rows(), 3);
        assert_eq!(result.columns(), vec!["user_id", "name", "amount"]);
    }

    #[test]
    fn test_left_join() {
        let users = users_df();
        let orders = orders_df();
        let spec = JoinSpec::left("user_id");

        let result = users.join(&orders, &spec).unwrap();

        // Left join: all 4 users, users 3 and 4 have null amounts
        // User 1 appears twice (2 orders), users 2,3,4 appear once each = 5 rows
        assert_eq!(result.num_rows(), 5);

        // Check that user 3 and 4 have null amounts
        let name_col = result.column("name").unwrap();
        let amount_col = result.column("amount").unwrap();

        let mut found_charlie = false;
        for i in 0..result.num_rows() {
            let name = name_col.get(i).unwrap();
            if let Value::String(s) = name {
                if s.as_ref() == "Charlie" || s.as_ref() == "Diana" {
                    let amount = amount_col.get(i).unwrap();
                    assert!(matches!(amount, Value::Null));
                    found_charlie = true;
                }
            }
        }
        assert!(found_charlie);
    }

    #[test]
    fn test_right_join() {
        let users = users_df();
        let orders = orders_df();
        let spec = JoinSpec::right("user_id");

        let result = users.join(&orders, &spec).unwrap();

        // Right join: all 4 orders, order for user_id=5 has null name
        assert_eq!(result.num_rows(), 4);

        // Check that user_id 5 has null name
        let user_id_col = result.column("user_id").unwrap();
        let name_col = result.column("name").unwrap();

        let mut found_user_5 = false;
        for i in 0..result.num_rows() {
            let user_id = user_id_col.get(i).unwrap();
            if let Value::Int(5) = user_id {
                let name = name_col.get(i).unwrap();
                assert!(matches!(name, Value::Null));
                found_user_5 = true;
            }
        }
        assert!(found_user_5);
    }

    #[test]
    fn test_outer_join() {
        let users = users_df();
        let orders = orders_df();
        let spec = JoinSpec::outer("user_id");

        let result = users.join(&orders, &spec).unwrap();

        // Outer join: 3 matched + 2 unmatched left (users 3,4) + 1 unmatched right (user 5) = 6 rows
        assert_eq!(result.num_rows(), 6);
    }

    #[test]
    fn test_join_different_column_names() {
        let left = {
            let ids = Series::from_ints("id", vec![1, 2, 3]);
            let values = Series::from_strings("val", vec!["a", "b", "c"]);
            DataFrame::from_series(vec![ids, values]).unwrap()
        };

        let right = {
            let ids = Series::from_ints("ref_id", vec![1, 2, 4]);
            let scores = Series::from_ints("score", vec![10, 20, 40]);
            DataFrame::from_series(vec![ids, scores]).unwrap()
        };

        let spec = JoinSpec::cols("id", "ref_id");
        let result = left.join(&right, &spec).unwrap();

        // Inner join: only ids 1 and 2 match
        assert_eq!(result.num_rows(), 2);
        // Both columns kept since different names
        assert_eq!(result.columns(), vec!["id", "val", "ref_id", "score"]);
    }

    #[test]
    fn test_join_column_name_conflict() {
        let left = {
            let ids = Series::from_ints("id", vec![1, 2]);
            let values = Series::from_strings("value", vec!["a", "b"]);
            DataFrame::from_series(vec![ids, values]).unwrap()
        };

        let right = {
            let ids = Series::from_ints("id", vec![1, 2]);
            let values = Series::from_strings("value", vec!["x", "y"]);
            DataFrame::from_series(vec![ids, values]).unwrap()
        };

        let spec = JoinSpec::on("id");
        let result = left.join(&right, &spec).unwrap();

        // "value" from right should be renamed to "value_right"
        assert_eq!(result.columns(), vec!["id", "value", "value_right"]);
    }
}
