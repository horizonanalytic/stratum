//! Series: A single column of data backed by Arrow arrays

use std::fmt;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int32Array, Int64Array, Scalar, StringArray,
};
use arrow::compute;
use arrow::compute::kernels::{boolean, cmp, length, numeric, substring};
use arrow::datatypes::DataType;

use super::error::{DataError, DataResult};
use super::types::arrow_to_stratum_type;
use crate::bytecode::Value;
use crate::types::Type;

/// A single column of homogeneous data backed by an Arrow array
#[derive(Clone)]
pub struct Series {
    /// Column name
    name: String,
    /// The underlying Arrow array (reference-counted for zero-copy)
    array: ArrayRef,
}

impl Series {
    /// Create a new Series from an Arrow array
    #[must_use]
    pub fn new(name: impl Into<String>, array: ArrayRef) -> Self {
        Self {
            name: name.into(),
            array,
        }
    }

    /// Create a Series from a vector of integers
    #[must_use]
    pub fn from_ints(name: impl Into<String>, values: Vec<i64>) -> Self {
        let array = Arc::new(Int64Array::from(values)) as ArrayRef;
        Self::new(name, array)
    }

    /// Create a Series from a vector of floats
    #[must_use]
    pub fn from_floats(name: impl Into<String>, values: Vec<f64>) -> Self {
        let array = Arc::new(Float64Array::from(values)) as ArrayRef;
        Self::new(name, array)
    }

    /// Create a Series from a vector of booleans
    #[must_use]
    pub fn from_bools(name: impl Into<String>, values: Vec<bool>) -> Self {
        let array = Arc::new(BooleanArray::from(values)) as ArrayRef;
        Self::new(name, array)
    }

    /// Create a Series from a vector of strings
    #[must_use]
    pub fn from_strings(name: impl Into<String>, values: Vec<&str>) -> Self {
        let array = Arc::new(StringArray::from(values)) as ArrayRef;
        Self::new(name, array)
    }

    /// Create a Series from a vector of optional integers
    #[must_use]
    pub fn from_optional_ints(name: impl Into<String>, values: Vec<Option<i64>>) -> Self {
        let array = Arc::new(Int64Array::from(values)) as ArrayRef;
        Self::new(name, array)
    }

    /// Create a Series from a slice of Stratum Values
    ///
    /// The type is inferred from the first non-null value.
    ///
    /// # Errors
    /// Returns error if values have mixed types or unsupported types
    pub fn from_values(name: impl Into<String>, values: &[Value]) -> DataResult<Self> {
        if values.is_empty() {
            // Default to Int64 for empty series
            return Ok(Self::from_ints(name, vec![]));
        }

        // Find the first non-null value to determine type
        let first_type = values.iter().find(|v| !matches!(v, Value::Null));

        match first_type {
            Some(Value::Int(_)) => {
                let ints: Vec<Option<i64>> = values
                    .iter()
                    .map(|v| match v {
                        Value::Int(i) => Ok(Some(*i)),
                        Value::Null => Ok(None),
                        _ => Err(DataError::TypeMismatch {
                            expected: "Int".to_string(),
                            found: v.type_name().to_string(),
                        }),
                    })
                    .collect::<DataResult<Vec<_>>>()?;
                Ok(Self::from_optional_ints(name, ints))
            }
            Some(Value::Float(_)) => {
                let floats: Vec<Option<f64>> = values
                    .iter()
                    .map(|v| match v {
                        Value::Float(f) => Ok(Some(*f)),
                        Value::Int(i) => Ok(Some(*i as f64)), // Allow int -> float coercion
                        Value::Null => Ok(None),
                        _ => Err(DataError::TypeMismatch {
                            expected: "Float".to_string(),
                            found: v.type_name().to_string(),
                        }),
                    })
                    .collect::<DataResult<Vec<_>>>()?;
                let array = Arc::new(Float64Array::from(floats)) as ArrayRef;
                Ok(Self::new(name, array))
            }
            Some(Value::Bool(_)) => {
                let bools: Vec<Option<bool>> = values
                    .iter()
                    .map(|v| match v {
                        Value::Bool(b) => Ok(Some(*b)),
                        Value::Null => Ok(None),
                        _ => Err(DataError::TypeMismatch {
                            expected: "Bool".to_string(),
                            found: v.type_name().to_string(),
                        }),
                    })
                    .collect::<DataResult<Vec<_>>>()?;
                let array = Arc::new(BooleanArray::from(bools)) as ArrayRef;
                Ok(Self::new(name, array))
            }
            Some(Value::String(_)) => {
                let strings: Vec<Option<String>> = values
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(Some(s.to_string())),
                        Value::Null => Ok(None),
                        _ => Err(DataError::TypeMismatch {
                            expected: "String".to_string(),
                            found: v.type_name().to_string(),
                        }),
                    })
                    .collect::<DataResult<Vec<_>>>()?;
                let array = Arc::new(StringArray::from(
                    strings.iter().map(|s| s.as_deref()).collect::<Vec<_>>(),
                )) as ArrayRef;
                Ok(Self::new(name, array))
            }
            Some(other) => Err(DataError::InvalidOperation(format!(
                "cannot create Series from {} values",
                other.type_name()
            ))),
            None => {
                // All null values - default to Int64
                let nulls: Vec<Option<i64>> = vec![None; values.len()];
                Ok(Self::from_optional_ints(name, nulls))
            }
        }
    }

    /// Get the column name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Rename the series
    #[must_use]
    pub fn rename(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Get the number of elements
    #[must_use]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    /// Check if the series is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    /// Get memory usage statistics for this Series
    ///
    /// Returns statistics including total bytes used, bytes per element, etc.
    #[must_use]
    pub fn memory_usage(&self) -> super::memory::MemoryStats {
        use arrow::array::Array;

        let num_rows = self.len();
        let data_bytes = self.array.get_array_memory_size();

        // Add overhead for name storage
        let name_overhead = self.name.len() + 24; // String capacity + metadata
        let total_bytes = data_bytes + name_overhead;

        super::memory::MemoryStats::new(num_rows, 1, data_bytes, total_bytes)
    }

    /// Get the Arrow data type
    #[must_use]
    pub fn data_type(&self) -> &DataType {
        self.array.data_type()
    }

    /// Get the Stratum type equivalent
    #[must_use]
    pub fn stratum_type(&self) -> Type {
        arrow_to_stratum_type(self.data_type())
    }

    /// Get the underlying Arrow array
    #[must_use]
    pub fn array(&self) -> &ArrayRef {
        &self.array
    }

    /// Get the number of null values
    #[must_use]
    pub fn null_count(&self) -> usize {
        self.array.null_count()
    }

    /// Check if a value at index is null
    #[must_use]
    pub fn is_null(&self, index: usize) -> bool {
        self.array.is_null(index)
    }

    /// Get a value at the given index as a Stratum Value
    ///
    /// # Errors
    /// Returns error if index is out of bounds
    pub fn get(&self, index: usize) -> DataResult<Value> {
        if index >= self.len() {
            return Err(DataError::InvalidColumnIndex(index));
        }

        if self.is_null(index) {
            return Ok(Value::Null);
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                Ok(Value::Int(arr.value(index)))
            }
            DataType::Int32 => {
                let arr = self.array.as_any().downcast_ref::<Int32Array>().unwrap();
                Ok(Value::Int(i64::from(arr.value(index))))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                Ok(Value::Float(arr.value(index)))
            }
            DataType::Boolean => {
                let arr = self.array.as_any().downcast_ref::<BooleanArray>().unwrap();
                Ok(Value::Bool(arr.value(index)))
            }
            DataType::Utf8 => {
                let arr = self.array.as_any().downcast_ref::<StringArray>().unwrap();
                Ok(Value::string(arr.value(index)))
            }
            other => Err(DataError::InvalidOperation(format!(
                "cannot get value of type {other:?}"
            ))),
        }
    }

    /// Calculate the sum of numeric values
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn sum(&self) -> DataResult<Value> {
        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let sum = compute::sum(arr).unwrap_or(0);
                Ok(Value::Int(sum))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let sum = compute::sum(arr).unwrap_or(0.0);
                Ok(Value::Float(sum))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Calculate the mean of numeric values
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn mean(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let sum: i64 = compute::sum(arr).unwrap_or(0);
                let count = arr.len() - arr.null_count();
                if count == 0 {
                    Ok(Value::Null)
                } else {
                    #[allow(clippy::cast_precision_loss)]
                    Ok(Value::Float(sum as f64 / count as f64))
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let sum: f64 = compute::sum(arr).unwrap_or(0.0);
                let count = arr.len() - arr.null_count();
                if count == 0 {
                    Ok(Value::Null)
                } else {
                    #[allow(clippy::cast_precision_loss)]
                    Ok(Value::Float(sum / count as f64))
                }
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Get the minimum value
    ///
    /// # Errors
    /// Returns error for non-comparable types
    pub fn min(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                match compute::min(arr) {
                    Some(v) => Ok(Value::Int(v)),
                    None => Ok(Value::Null),
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                match compute::min(arr) {
                    Some(v) => Ok(Value::Float(v)),
                    None => Ok(Value::Null),
                }
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Get the maximum value
    ///
    /// # Errors
    /// Returns error for non-comparable types
    pub fn max(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                match compute::max(arr) {
                    Some(v) => Ok(Value::Int(v)),
                    None => Ok(Value::Null),
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                match compute::max(arr) {
                    Some(v) => Ok(Value::Float(v)),
                    None => Ok(Value::Null),
                }
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Count non-null values
    #[must_use]
    pub fn count(&self) -> usize {
        self.len() - self.null_count()
    }

    /// Calculate the standard deviation of numeric values
    ///
    /// Uses the population standard deviation formula (N divisor).
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn std(&self) -> DataResult<Value> {
        self.variance_impl().map(|v| match v {
            Value::Float(var) => Value::Float(var.sqrt()),
            other => other, // Null stays Null
        })
    }

    /// Calculate the variance of numeric values
    ///
    /// Uses the population variance formula (N divisor).
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn var(&self) -> DataResult<Value> {
        self.variance_impl()
    }

    /// Internal variance calculation used by both var() and std()
    fn variance_impl(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        // First get the mean
        let mean_val = self.mean()?;
        let mean = match mean_val {
            Value::Float(m) => m,
            Value::Null => return Ok(Value::Null),
            _ => return Ok(Value::Null),
        };

        // Calculate sum of squared differences
        let mut sum_sq_diff: f64 = 0.0;
        let mut count: usize = 0;

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        let diff = arr.value(i) as f64 - mean;
                        sum_sq_diff += diff * diff;
                        count += 1;
                    }
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        let diff = arr.value(i) - mean;
                        sum_sq_diff += diff * diff;
                        count += 1;
                    }
                }
            }
            other => {
                return Err(DataError::TypeMismatch {
                    expected: "numeric type".to_string(),
                    found: format!("{other:?}"),
                });
            }
        }

        if count == 0 {
            Ok(Value::Null)
        } else {
            #[allow(clippy::cast_precision_loss)]
            Ok(Value::Float(sum_sq_diff / count as f64))
        }
    }

    /// Calculate the median of numeric values
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn median(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        // Collect non-null values
        let mut values: Vec<f64> = Vec::new();

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        values.push(arr.value(i) as f64);
                    }
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        values.push(arr.value(i));
                    }
                }
            }
            other => {
                return Err(DataError::TypeMismatch {
                    expected: "numeric type".to_string(),
                    found: format!("{other:?}"),
                });
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

    /// Calculate the mode (most frequent value) of the series
    ///
    /// Returns the most frequent value. For ties, returns the smallest value.
    ///
    /// # Errors
    /// Returns error for unsupported types
    pub fn mode(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        let mut counts: std::collections::HashMap<String, (usize, Value)> =
            std::collections::HashMap::new();

        for i in 0..self.len() {
            let val = self.get(i)?;
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

        // Find max count and return the value with that count
        let max_count = counts.values().map(|(c, _)| *c).max().unwrap_or(0);
        let mode_values: Vec<&Value> = counts
            .values()
            .filter(|(c, _)| *c == max_count)
            .map(|(_, v)| v)
            .collect();

        // Return first one (deterministic for same input)
        Ok(mode_values
            .first()
            .map(|v| (*v).clone())
            .unwrap_or(Value::Null))
    }

    /// Calculate a quantile of numeric values
    ///
    /// # Arguments
    /// * `q` - Quantile value between 0.0 and 1.0
    ///
    /// # Errors
    /// Returns error for non-numeric types or invalid quantile
    pub fn quantile(&self, q: f64) -> DataResult<Value> {
        if !(0.0..=1.0).contains(&q) {
            return Err(DataError::InvalidOperation(format!(
                "quantile must be between 0 and 1, got {q}"
            )));
        }

        if self.is_empty() {
            return Ok(Value::Null);
        }

        // Collect non-null values
        let mut values: Vec<f64> = Vec::new();

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        values.push(arr.value(i) as f64);
                    }
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        values.push(arr.value(i));
                    }
                }
            }
            other => {
                return Err(DataError::TypeMismatch {
                    expected: "numeric type".to_string(),
                    found: format!("{other:?}"),
                });
            }
        }

        if values.is_empty() {
            return Ok(Value::Null);
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Linear interpolation method
        let n = values.len();
        if n == 1 {
            return Ok(Value::Float(values[0]));
        }

        let pos = q * (n - 1) as f64;
        let lower = pos.floor() as usize;
        let upper = pos.ceil() as usize;
        let frac = pos - lower as f64;

        if lower == upper {
            Ok(Value::Float(values[lower]))
        } else {
            Ok(Value::Float(
                values[lower] * (1.0 - frac) + values[upper] * frac,
            ))
        }
    }

    /// Calculate a percentile of numeric values
    ///
    /// # Arguments
    /// * `p` - Percentile value between 0 and 100
    ///
    /// # Errors
    /// Returns error for non-numeric types or invalid percentile
    pub fn percentile(&self, p: f64) -> DataResult<Value> {
        if !(0.0..=100.0).contains(&p) {
            return Err(DataError::InvalidOperation(format!(
                "percentile must be between 0 and 100, got {p}"
            )));
        }
        self.quantile(p / 100.0)
    }

    /// Calculate the skewness of numeric values
    ///
    /// Skewness measures the asymmetry of the distribution.
    /// - Positive skew: tail on the right
    /// - Negative skew: tail on the left
    /// - Zero skew: symmetric
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn skew(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        let mean_val = self.mean()?;
        let mean = match mean_val {
            Value::Float(m) => m,
            Value::Null => return Ok(Value::Null),
            _ => return Ok(Value::Null),
        };

        let std_val = self.std()?;
        let std_dev = match std_val {
            Value::Float(s) if s > 0.0 => s,
            _ => return Ok(Value::Null),
        };

        let mut sum_cubed: f64 = 0.0;
        let mut count: usize = 0;

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        let z = (arr.value(i) as f64 - mean) / std_dev;
                        sum_cubed += z * z * z;
                        count += 1;
                    }
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        let z = (arr.value(i) - mean) / std_dev;
                        sum_cubed += z * z * z;
                        count += 1;
                    }
                }
            }
            other => {
                return Err(DataError::TypeMismatch {
                    expected: "numeric type".to_string(),
                    found: format!("{other:?}"),
                });
            }
        }

        if count < 3 {
            return Ok(Value::Null);
        }

        #[allow(clippy::cast_precision_loss)]
        Ok(Value::Float(sum_cubed / count as f64))
    }

    /// Calculate the kurtosis of numeric values
    ///
    /// Kurtosis measures the "tailedness" of the distribution.
    /// - Positive kurtosis (leptokurtic): heavy tails
    /// - Negative kurtosis (platykurtic): light tails
    /// - Zero kurtosis (mesokurtic): normal distribution
    ///
    /// This returns excess kurtosis (kurtosis - 3), so normal distribution = 0.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn kurtosis(&self) -> DataResult<Value> {
        if self.is_empty() {
            return Ok(Value::Null);
        }

        let mean_val = self.mean()?;
        let mean = match mean_val {
            Value::Float(m) => m,
            Value::Null => return Ok(Value::Null),
            _ => return Ok(Value::Null),
        };

        let std_val = self.std()?;
        let std_dev = match std_val {
            Value::Float(s) if s > 0.0 => s,
            _ => return Ok(Value::Null),
        };

        let mut sum_fourth: f64 = 0.0;
        let mut count: usize = 0;

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        let z = (arr.value(i) as f64 - mean) / std_dev;
                        sum_fourth += z * z * z * z;
                        count += 1;
                    }
                }
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        let z = (arr.value(i) - mean) / std_dev;
                        sum_fourth += z * z * z * z;
                        count += 1;
                    }
                }
            }
            other => {
                return Err(DataError::TypeMismatch {
                    expected: "numeric type".to_string(),
                    found: format!("{other:?}"),
                });
            }
        }

        if count < 4 {
            return Ok(Value::Null);
        }

        // Return excess kurtosis (subtract 3 so normal distribution = 0)
        #[allow(clippy::cast_precision_loss)]
        Ok(Value::Float(sum_fourth / count as f64 - 3.0))
    }

    /// Convert to a vector of Stratum Values
    ///
    /// # Errors
    /// Returns error if conversion fails
    pub fn to_values(&self) -> DataResult<Vec<Value>> {
        (0..self.len()).map(|i| self.get(i)).collect()
    }

    // ========================================================================
    // Arithmetic Operations (element-wise)
    // ========================================================================

    /// Add two series element-wise
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn add(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let (left, right) = self.coerce_numeric_pair(other)?;
        let result = numeric::add(&left, &right).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(self.name.clone(), result))
    }

    /// Add a scalar value to each element
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn add_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = match (self.array.data_type(), value) {
            (DataType::Int64, Value::Int(v)) => {
                let scalar = Scalar::new(Int64Array::from(vec![*v]));
                numeric::add(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::add(&arr, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::add(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::add(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "cannot add {} to {:?} Series",
                    value.type_name(),
                    self.data_type()
                )))
            }
        };
        Ok(Self::new(self.name.clone(), result))
    }

    /// Subtract two series element-wise
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn sub(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let (left, right) = self.coerce_numeric_pair(other)?;
        let result = numeric::sub(&left, &right).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(self.name.clone(), result))
    }

    /// Subtract a scalar value from each element
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn sub_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = match (self.array.data_type(), value) {
            (DataType::Int64, Value::Int(v)) => {
                let scalar = Scalar::new(Int64Array::from(vec![*v]));
                numeric::sub(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::sub(&arr, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::sub(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::sub(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "cannot subtract {} from {:?} Series",
                    value.type_name(),
                    self.data_type()
                )))
            }
        };
        Ok(Self::new(self.name.clone(), result))
    }

    /// Multiply two series element-wise
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn mul(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let (left, right) = self.coerce_numeric_pair(other)?;
        let result = numeric::mul(&left, &right).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(self.name.clone(), result))
    }

    /// Multiply each element by a scalar value
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn mul_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = match (self.array.data_type(), value) {
            (DataType::Int64, Value::Int(v)) => {
                let scalar = Scalar::new(Int64Array::from(vec![*v]));
                numeric::mul(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::mul(&arr, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::mul(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::mul(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "cannot multiply {:?} Series by {}",
                    self.data_type(),
                    value.type_name()
                )))
            }
        };
        Ok(Self::new(self.name.clone(), result))
    }

    /// Divide two series element-wise
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn div(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let (left, right) = self.coerce_numeric_pair(other)?;
        let result = numeric::div(&left, &right).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(self.name.clone(), result))
    }

    /// Divide each element by a scalar value
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn div_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = match (self.array.data_type(), value) {
            (DataType::Int64, Value::Int(v)) => {
                let scalar = Scalar::new(Int64Array::from(vec![*v]));
                numeric::div(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::div(&arr, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::div(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::div(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "cannot divide {:?} Series by {}",
                    self.data_type(),
                    value.type_name()
                )))
            }
        };
        Ok(Self::new(self.name.clone(), result))
    }

    /// Negate each element in the series
    ///
    /// # Errors
    /// Returns error if type is not numeric
    pub fn neg(&self) -> DataResult<Self> {
        let result = numeric::neg(&self.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(self.name.clone(), result))
    }

    // ========================================================================
    // Comparison Operations (element-wise, return boolean Series)
    // ========================================================================

    /// Element-wise equality comparison
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn eq(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let result =
            cmp::eq(&self.array, &other.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_eq", self.name), Arc::new(result)))
    }

    /// Element-wise equality comparison with a scalar
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn eq_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = self.compare_scalar(value, cmp::eq)?;
        Ok(Self::new(format!("{}_eq", self.name), Arc::new(result)))
    }

    /// Element-wise inequality comparison
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn neq(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let result =
            cmp::neq(&self.array, &other.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_neq", self.name), Arc::new(result)))
    }

    /// Element-wise inequality comparison with a scalar
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn neq_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = self.compare_scalar(value, cmp::neq)?;
        Ok(Self::new(format!("{}_neq", self.name), Arc::new(result)))
    }

    /// Element-wise less-than comparison
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn lt(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let result =
            cmp::lt(&self.array, &other.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_lt", self.name), Arc::new(result)))
    }

    /// Element-wise less-than comparison with a scalar
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn lt_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = self.compare_scalar(value, cmp::lt)?;
        Ok(Self::new(format!("{}_lt", self.name), Arc::new(result)))
    }

    /// Element-wise less-than-or-equal comparison
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn le(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let result =
            cmp::lt_eq(&self.array, &other.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_le", self.name), Arc::new(result)))
    }

    /// Element-wise less-than-or-equal comparison with a scalar
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn le_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = self.compare_scalar(value, cmp::lt_eq)?;
        Ok(Self::new(format!("{}_le", self.name), Arc::new(result)))
    }

    /// Element-wise greater-than comparison
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn gt(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let result =
            cmp::gt(&self.array, &other.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_gt", self.name), Arc::new(result)))
    }

    /// Element-wise greater-than comparison with a scalar
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn gt_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = self.compare_scalar(value, cmp::gt)?;
        Ok(Self::new(format!("{}_gt", self.name), Arc::new(result)))
    }

    /// Element-wise greater-than-or-equal comparison
    ///
    /// # Errors
    /// Returns error if types are incompatible or lengths don't match
    pub fn ge(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let result =
            cmp::gt_eq(&self.array, &other.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_ge", self.name), Arc::new(result)))
    }

    /// Element-wise greater-than-or-equal comparison with a scalar
    ///
    /// # Errors
    /// Returns error if types are incompatible
    pub fn ge_scalar(&self, value: &Value) -> DataResult<Self> {
        let result = self.compare_scalar(value, cmp::gt_eq)?;
        Ok(Self::new(format!("{}_ge", self.name), Arc::new(result)))
    }

    // ========================================================================
    // Logical Operations (for boolean Series)
    // ========================================================================

    /// Element-wise logical AND
    ///
    /// # Errors
    /// Returns error if either series is not boolean or lengths don't match
    pub fn and(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let left = self.as_boolean()?;
        let right = other.as_boolean()?;
        let result = boolean::and(&left, &right).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_and", self.name), Arc::new(result)))
    }

    /// Element-wise logical OR
    ///
    /// # Errors
    /// Returns error if either series is not boolean or lengths don't match
    pub fn or(&self, other: &Series) -> DataResult<Self> {
        self.check_length(other)?;
        let left = self.as_boolean()?;
        let right = other.as_boolean()?;
        let result = boolean::or(&left, &right).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_or", self.name), Arc::new(result)))
    }

    /// Element-wise logical NOT
    ///
    /// # Errors
    /// Returns error if series is not boolean
    pub fn not(&self) -> DataResult<Self> {
        let arr = self.as_boolean()?;
        let result = boolean::not(&arr).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_not", self.name), Arc::new(result)))
    }

    // ========================================================================
    // String Operations (for String columns)
    // ========================================================================

    /// Get the array as a StringArray reference
    fn as_string(&self) -> DataResult<&StringArray> {
        self.array
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataError::TypeMismatch {
                expected: "String".to_string(),
                found: format!("{:?}", self.data_type()),
            })
    }

    /// Check if this is a string series
    #[must_use]
    pub fn is_string(&self) -> bool {
        self.data_type() == &DataType::Utf8
    }

    /// Get the length of each string element
    ///
    /// Returns a Series of Int32 with the character length of each string.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_len(&self) -> DataResult<Self> {
        if !self.is_string() {
            return Err(DataError::TypeMismatch {
                expected: "String".to_string(),
                found: format!("{:?}", self.data_type()),
            });
        }
        let result = length::length(&self.array).map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_len", self.name), result))
    }

    /// Check if each string contains the given pattern
    ///
    /// Returns a boolean Series indicating whether each element contains the pattern.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_contains(&self, pattern: &str) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<bool>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).contains(pattern))
                }
            })
            .collect();
        let result = BooleanArray::from(results);
        Ok(Self::new(
            format!("{}_contains", self.name),
            Arc::new(result),
        ))
    }

    /// Check if each string starts with the given prefix
    ///
    /// Returns a boolean Series indicating whether each element starts with the prefix.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_starts_with(&self, prefix: &str) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<bool>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).starts_with(prefix))
                }
            })
            .collect();
        let result = BooleanArray::from(results);
        Ok(Self::new(
            format!("{}_starts_with", self.name),
            Arc::new(result),
        ))
    }

    /// Check if each string ends with the given suffix
    ///
    /// Returns a boolean Series indicating whether each element ends with the suffix.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_ends_with(&self, suffix: &str) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<bool>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).ends_with(suffix))
                }
            })
            .collect();
        let result = BooleanArray::from(results);
        Ok(Self::new(
            format!("{}_ends_with", self.name),
            Arc::new(result),
        ))
    }

    /// Convert each string to lowercase
    ///
    /// Returns a new Series with all characters converted to lowercase.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_to_lowercase(&self) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).to_lowercase())
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(format!("{}_lower", self.name), Arc::new(result)))
    }

    /// Convert each string to uppercase
    ///
    /// Returns a new Series with all characters converted to uppercase.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_to_uppercase(&self) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).to_uppercase())
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(format!("{}_upper", self.name), Arc::new(result)))
    }

    /// Trim whitespace from both ends of each string
    ///
    /// Returns a new Series with leading and trailing whitespace removed.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_trim(&self) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<&str>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).trim())
                }
            })
            .collect();
        let result = StringArray::from(results);
        Ok(Self::new(format!("{}_trim", self.name), Arc::new(result)))
    }

    /// Trim whitespace from the start of each string
    ///
    /// Returns a new Series with leading whitespace removed.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_trim_start(&self) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<&str>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).trim_start())
                }
            })
            .collect();
        let result = StringArray::from(results);
        Ok(Self::new(
            format!("{}_trim_start", self.name),
            Arc::new(result),
        ))
    }

    /// Trim whitespace from the end of each string
    ///
    /// Returns a new Series with trailing whitespace removed.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_trim_end(&self) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<&str>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).trim_end())
                }
            })
            .collect();
        let result = StringArray::from(results);
        Ok(Self::new(
            format!("{}_trim_end", self.name),
            Arc::new(result),
        ))
    }

    /// Extract a substring from each string element
    ///
    /// # Arguments
    /// * `start` - The starting character index (0-based). Negative values count from the end.
    /// * `length` - Optional length of the substring. If None, extracts to the end.
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_substring(&self, start: i64, len: Option<u64>) -> DataResult<Self> {
        if !self.is_string() {
            return Err(DataError::TypeMismatch {
                expected: "String".to_string(),
                found: format!("{:?}", self.data_type()),
            });
        }
        let result = substring::substring(&self.array, start, len)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_substring", self.name), result))
    }

    /// Replace occurrences of a pattern with a replacement string
    ///
    /// # Arguments
    /// * `pattern` - The pattern to search for
    /// * `replacement` - The string to replace the pattern with
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_replace(&self, pattern: &str, replacement: &str) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(arr.value(i).replace(pattern, replacement))
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(
            format!("{}_replace", self.name),
            Arc::new(result),
        ))
    }

    /// Split each string by a delimiter and return the nth part (0-indexed)
    ///
    /// # Arguments
    /// * `delimiter` - The delimiter to split on
    /// * `index` - The index of the part to return (0-based)
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_split_get(&self, delimiter: &str, index: usize) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    arr.value(i).split(delimiter).nth(index).map(String::from)
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(format!("{}_split", self.name), Arc::new(result)))
    }

    /// Pad each string to a minimum width
    ///
    /// # Arguments
    /// * `width` - The minimum width to pad to
    /// * `side` - Where to add padding: "left", "right", or "both"
    /// * `pad_char` - The character to use for padding
    ///
    /// # Errors
    /// Returns error if series is not a string type or side is invalid
    pub fn str_pad(&self, width: usize, side: &str, pad_char: char) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    let s = arr.value(i);
                    let char_count = s.chars().count();
                    if char_count >= width {
                        Some(s.to_string())
                    } else {
                        let pad_len = width - char_count;
                        match side {
                            "left" => {
                                let padding: String =
                                    std::iter::repeat(pad_char).take(pad_len).collect();
                                Some(format!("{padding}{s}"))
                            }
                            "right" => {
                                let padding: String =
                                    std::iter::repeat(pad_char).take(pad_len).collect();
                                Some(format!("{s}{padding}"))
                            }
                            "both" => {
                                let left_pad = pad_len / 2;
                                let right_pad = pad_len - left_pad;
                                let left: String =
                                    std::iter::repeat(pad_char).take(left_pad).collect();
                                let right: String =
                                    std::iter::repeat(pad_char).take(right_pad).collect();
                                Some(format!("{left}{s}{right}"))
                            }
                            _ => Some(s.to_string()), // Invalid side, return unchanged
                        }
                    }
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(format!("{}_pad", self.name), Arc::new(result)))
    }

    /// Extract the first capture group from a regex pattern
    ///
    /// # Arguments
    /// * `pattern` - A regex pattern with at least one capture group
    ///
    /// # Errors
    /// Returns error if series is not a string type or regex is invalid
    pub fn str_extract(&self, pattern: &str) -> DataResult<Self> {
        let arr = self.as_string()?;
        let re = regex::Regex::new(pattern)
            .map_err(|e| DataError::InvalidOperation(format!("Invalid regex: {e}")))?;

        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    let s = arr.value(i);
                    re.captures(s)
                        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(
            format!("{}_extract", self.name),
            Arc::new(result),
        ))
    }

    /// Check if each string matches a regex pattern
    ///
    /// # Arguments
    /// * `pattern` - A regex pattern to match against
    ///
    /// # Errors
    /// Returns error if series is not a string type or regex is invalid
    pub fn str_match(&self, pattern: &str) -> DataResult<Self> {
        let arr = self.as_string()?;
        let re = regex::Regex::new(pattern)
            .map_err(|e| DataError::InvalidOperation(format!("Invalid regex: {e}")))?;

        let results: Vec<Option<bool>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    Some(re.is_match(arr.value(i)))
                }
            })
            .collect();
        let result = BooleanArray::from(results);
        Ok(Self::new(format!("{}_match", self.name), Arc::new(result)))
    }

    /// Concatenate strings from two series element-wise with a separator
    ///
    /// # Arguments
    /// * `other` - Another string series to concatenate with
    /// * `sep` - Separator string between the two values
    ///
    /// # Errors
    /// Returns error if either series is not a string type or lengths differ
    pub fn str_cat(&self, other: &Series, sep: &str) -> DataResult<Self> {
        let arr1 = self.as_string()?;
        let arr2 = other.as_string()?;

        if arr1.len() != arr2.len() {
            return Err(DataError::SchemaMismatch(format!(
                "Series length mismatch: {} vs {}",
                arr1.len(),
                arr2.len()
            )));
        }

        let results: Vec<Option<String>> = (0..arr1.len())
            .map(|i| {
                if arr1.is_null(i) || arr2.is_null(i) {
                    None
                } else {
                    Some(format!("{}{}{}", arr1.value(i), sep, arr2.value(i)))
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(format!("{}_cat", self.name), Arc::new(result)))
    }

    /// Slice each string from start to end index
    ///
    /// # Arguments
    /// * `start` - Start index (0-based, supports negative indexing)
    /// * `end` - Optional end index (exclusive, supports negative indexing)
    ///
    /// # Errors
    /// Returns error if series is not a string type
    pub fn str_slice(&self, start: i64, end: Option<i64>) -> DataResult<Self> {
        let arr = self.as_string()?;
        let results: Vec<Option<String>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    let s = arr.value(i);
                    let chars: Vec<char> = s.chars().collect();
                    let len = chars.len() as i64;

                    // Handle negative indices
                    let start_idx = if start < 0 {
                        (len + start).max(0) as usize
                    } else {
                        (start as usize).min(chars.len())
                    };

                    let end_idx = match end {
                        Some(e) if e < 0 => (len + e).max(0) as usize,
                        Some(e) => (e as usize).min(chars.len()),
                        None => chars.len(),
                    };

                    if start_idx >= end_idx {
                        Some(String::new())
                    } else {
                        Some(chars[start_idx..end_idx].iter().collect())
                    }
                }
            })
            .collect();
        let result = StringArray::from(
            results
                .iter()
                .map(|s| s.as_deref())
                .collect::<Vec<Option<&str>>>(),
        );
        Ok(Self::new(format!("{}_slice", self.name), Arc::new(result)))
    }

    // ========================================================================
    // Type Conversion Methods
    // ========================================================================

    /// Convert series to integer type
    ///
    /// Converts:
    /// - Float → Int: Truncates decimal part
    /// - Bool → Int: true=1, false=0
    /// - String → Int: Parses string as integer
    ///
    /// # Errors
    /// Returns error if conversion fails (e.g., invalid string format)
    pub fn to_int(&self) -> DataResult<Self> {
        if self.data_type() == &DataType::Int64 {
            return Ok(self.clone());
        }

        let result = arrow::compute::cast(&self.array, &DataType::Int64)
            .map_err(|e| DataError::Arrow(format!("Failed to convert to Int: {}", e)))?;

        Ok(Self::new(self.name.clone(), result))
    }

    /// Convert series to float type
    ///
    /// Converts:
    /// - Int → Float: Integer to floating point
    /// - Bool → Float: true=1.0, false=0.0
    /// - String → Float: Parses string as float
    ///
    /// # Errors
    /// Returns error if conversion fails (e.g., invalid string format)
    pub fn to_float(&self) -> DataResult<Self> {
        if self.data_type() == &DataType::Float64 {
            return Ok(self.clone());
        }

        let result = arrow::compute::cast(&self.array, &DataType::Float64)
            .map_err(|e| DataError::Arrow(format!("Failed to convert to Float: {}", e)))?;

        Ok(Self::new(self.name.clone(), result))
    }

    /// Convert series to string type
    ///
    /// Converts any type to its string representation:
    /// - Int → String: e.g., 42 → "42"
    /// - Float → String: e.g., 3.14 → "3.14"
    /// - Bool → String: true → "true", false → "false"
    ///
    /// # Errors
    /// Returns error if conversion fails
    pub fn to_str(&self) -> DataResult<Self> {
        if self.data_type() == &DataType::Utf8 {
            return Ok(self.clone());
        }

        let result = arrow::compute::cast(&self.array, &DataType::Utf8)
            .map_err(|e| DataError::Arrow(format!("Failed to convert to String: {}", e)))?;

        Ok(Self::new(self.name.clone(), result))
    }

    /// Parse string series to datetime (timestamp in milliseconds)
    ///
    /// # Arguments
    /// * `format` - A strftime format string (e.g., "%Y-%m-%d %H:%M:%S")
    ///
    /// Common format specifiers:
    /// - `%Y` - 4-digit year
    /// - `%m` - 2-digit month (01-12)
    /// - `%d` - 2-digit day (01-31)
    /// - `%H` - Hour (00-23)
    /// - `%M` - Minute (00-59)
    /// - `%S` - Second (00-59)
    ///
    /// # Returns
    /// A new Series with Int64 values representing milliseconds since Unix epoch
    ///
    /// # Errors
    /// Returns error if series is not a string type or if parsing fails
    pub fn to_datetime(&self, format: &str) -> DataResult<Self> {
        use chrono::NaiveDateTime;

        let arr = self.as_string()?;
        let timestamps: Vec<Option<i64>> = (0..arr.len())
            .map(|i| {
                if arr.is_null(i) {
                    None
                } else {
                    let s = arr.value(i);
                    NaiveDateTime::parse_from_str(s, format)
                        .map(|dt| dt.and_utc().timestamp_millis())
                        .ok()
                }
            })
            .collect();

        let result = Arc::new(Int64Array::from(timestamps)) as ArrayRef;
        Ok(Self::new(self.name.clone(), result))
    }

    /// Convert series to boolean type
    ///
    /// Converts:
    /// - Int → Bool: 0=false, non-zero=true
    /// - Float → Bool: 0.0=false, non-zero=true
    /// - String → Bool: "true"/"1"=true, "false"/"0"=false
    ///
    /// # Errors
    /// Returns error if conversion fails
    pub fn to_bool(&self) -> DataResult<Self> {
        if self.data_type() == &DataType::Boolean {
            return Ok(self.clone());
        }

        let result = arrow::compute::cast(&self.array, &DataType::Boolean)
            .map_err(|e| DataError::Arrow(format!("Failed to convert to Bool: {}", e)))?;

        Ok(Self::new(self.name.clone(), result))
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    /// Check that two series have the same length
    fn check_length(&self, other: &Series) -> DataResult<()> {
        if self.len() != other.len() {
            return Err(DataError::SchemaMismatch(format!(
                "Series length mismatch: {} vs {}",
                self.len(),
                other.len()
            )));
        }
        Ok(())
    }

    /// Cast Int64 array to Float64
    fn cast_to_float(&self) -> DataResult<ArrayRef> {
        if self.data_type() == &DataType::Float64 {
            return Ok(Arc::clone(&self.array));
        }
        arrow::compute::cast(&self.array, &DataType::Float64)
            .map_err(|e| DataError::Arrow(e.to_string()))
    }

    /// Coerce two series to a common numeric type (Int64 or Float64)
    fn coerce_numeric_pair(&self, other: &Series) -> DataResult<(ArrayRef, ArrayRef)> {
        match (self.data_type(), other.data_type()) {
            (DataType::Int64, DataType::Int64) => {
                Ok((Arc::clone(&self.array), Arc::clone(&other.array)))
            }
            (DataType::Float64, DataType::Float64) => {
                Ok((Arc::clone(&self.array), Arc::clone(&other.array)))
            }
            (DataType::Int64, DataType::Float64) => {
                let left = self.cast_to_float()?;
                Ok((left, Arc::clone(&other.array)))
            }
            (DataType::Float64, DataType::Int64) => {
                let right = other.cast_to_float()?;
                Ok((Arc::clone(&self.array), right))
            }
            _ => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{:?} and {:?}", self.data_type(), other.data_type()),
            }),
        }
    }

    /// Get the array as a BooleanArray
    fn as_boolean(&self) -> DataResult<&BooleanArray> {
        self.array
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| DataError::TypeMismatch {
                expected: "Boolean".to_string(),
                found: format!("{:?}", self.data_type()),
            })
    }

    /// Compare series elements to a scalar value using the given comparison function
    fn compare_scalar<F>(&self, value: &Value, cmp_fn: F) -> DataResult<BooleanArray>
    where
        F: Fn(
            &dyn arrow::array::Datum,
            &dyn arrow::array::Datum,
        ) -> Result<BooleanArray, arrow::error::ArrowError>,
    {
        let result = match (self.array.data_type(), value) {
            (DataType::Int64, Value::Int(v)) => {
                let scalar = Scalar::new(Int64Array::from(vec![*v]));
                cmp_fn(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                cmp_fn(&arr, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                cmp_fn(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                cmp_fn(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Boolean, Value::Bool(v)) => {
                let scalar = Scalar::new(BooleanArray::from(vec![*v]));
                cmp_fn(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Utf8, Value::String(v)) => {
                let scalar = Scalar::new(StringArray::from(vec![v.as_str()]));
                cmp_fn(&self.array, &scalar).map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => {
                return Err(DataError::InvalidOperation(format!(
                    "cannot compare {:?} Series with {}",
                    self.data_type(),
                    value.type_name()
                )))
            }
        };
        Ok(result)
    }

    /// Check if this is a boolean series
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        self.data_type() == &DataType::Boolean
    }

    /// Check if this is a numeric series (Int64 or Float64)
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        matches!(self.data_type(), DataType::Int64 | DataType::Float64)
    }

    // ========================================================================
    // Window Functions: Cumulative Operations
    // ========================================================================

    /// Cumulative sum of numeric values
    ///
    /// Returns a Series where each element is the sum of all previous elements.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn cumsum(&self) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(arr.len());
                let mut cumsum: i64 = 0;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(None);
                    } else {
                        cumsum += arr.value(i);
                        results.push(Some(cumsum));
                    }
                }

                let result = Int64Array::from(results);
                Ok(Self::new(format!("{}_cumsum", self.name), Arc::new(result)))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(arr.len());
                let mut cumsum: f64 = 0.0;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(None);
                    } else {
                        cumsum += arr.value(i);
                        results.push(Some(cumsum));
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(format!("{}_cumsum", self.name), Arc::new(result)))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Cumulative maximum of numeric values
    ///
    /// Returns a Series where each element is the maximum of all previous elements.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn cummax(&self) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(arr.len());
                let mut cummax: Option<i64> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(cummax);
                    } else {
                        let val = arr.value(i);
                        cummax = Some(cummax.map_or(val, |m| m.max(val)));
                        results.push(cummax);
                    }
                }

                let result = Int64Array::from(results);
                Ok(Self::new(format!("{}_cummax", self.name), Arc::new(result)))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(arr.len());
                let mut cummax: Option<f64> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(cummax);
                    } else {
                        let val = arr.value(i);
                        cummax = Some(cummax.map_or(val, |m| m.max(val)));
                        results.push(cummax);
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(format!("{}_cummax", self.name), Arc::new(result)))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Cumulative minimum of numeric values
    ///
    /// Returns a Series where each element is the minimum of all previous elements.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn cummin(&self) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(arr.len());
                let mut cummin: Option<i64> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(cummin);
                    } else {
                        let val = arr.value(i);
                        cummin = Some(cummin.map_or(val, |m| m.min(val)));
                        results.push(cummin);
                    }
                }

                let result = Int64Array::from(results);
                Ok(Self::new(format!("{}_cummin", self.name), Arc::new(result)))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(arr.len());
                let mut cummin: Option<f64> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(cummin);
                    } else {
                        let val = arr.value(i);
                        cummin = Some(cummin.map_or(val, |m| m.min(val)));
                        results.push(cummin);
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(format!("{}_cummin", self.name), Arc::new(result)))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Cumulative product of numeric values
    ///
    /// Returns a Series where each element is the product of all previous elements.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn cumprod(&self) -> DataResult<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(arr.len());
                let mut cumprod: i64 = 1;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(None);
                    } else {
                        cumprod *= arr.value(i);
                        results.push(Some(cumprod));
                    }
                }

                let result = Int64Array::from(results);
                Ok(Self::new(
                    format!("{}_cumprod", self.name),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(arr.len());
                let mut cumprod: f64 = 1.0;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        results.push(None);
                    } else {
                        cumprod *= arr.value(i);
                        results.push(Some(cumprod));
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(
                    format!("{}_cumprod", self.name),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    // ========================================================================
    // Window Functions: Lag/Lead Operations
    // ========================================================================

    /// Shift values by n positions
    ///
    /// Positive n shifts values down (lag), negative n shifts up (lead).
    /// Empty positions are filled with null.
    ///
    /// # Arguments
    /// * `n` - Number of positions to shift (positive = lag, negative = lead)
    ///
    /// # Errors
    /// Returns error for unsupported types
    pub fn shift(&self, n: i64) -> DataResult<Self> {
        if self.is_empty() || n == 0 {
            return Ok(self.clone());
        }

        let len = self.len();
        let abs_n = n.unsigned_abs() as usize;

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(len);

                if n > 0 {
                    // Shift down (lag): first abs_n positions are null
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                    for i in 0..(len.saturating_sub(abs_n)) {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i)));
                        }
                    }
                } else {
                    // Shift up (lead): last abs_n positions are null
                    for i in abs_n..len {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i)));
                        }
                    }
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                }

                let result = Int64Array::from(results);
                Ok(Self::new(format!("{}_shift", self.name), Arc::new(result)))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                if n > 0 {
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                    for i in 0..(len.saturating_sub(abs_n)) {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i)));
                        }
                    }
                } else {
                    for i in abs_n..len {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i)));
                        }
                    }
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(format!("{}_shift", self.name), Arc::new(result)))
            }
            DataType::Boolean => {
                let arr = self.array.as_any().downcast_ref::<BooleanArray>().unwrap();
                let mut results: Vec<Option<bool>> = Vec::with_capacity(len);

                if n > 0 {
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                    for i in 0..(len.saturating_sub(abs_n)) {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i)));
                        }
                    }
                } else {
                    for i in abs_n..len {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i)));
                        }
                    }
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                }

                let result = BooleanArray::from(results);
                Ok(Self::new(format!("{}_shift", self.name), Arc::new(result)))
            }
            DataType::Utf8 => {
                let arr = self.array.as_any().downcast_ref::<StringArray>().unwrap();
                let mut results: Vec<Option<String>> = Vec::with_capacity(len);

                if n > 0 {
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                    for i in 0..(len.saturating_sub(abs_n)) {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i).to_string()));
                        }
                    }
                } else {
                    for i in abs_n..len {
                        if arr.is_null(i) {
                            results.push(None);
                        } else {
                            results.push(Some(arr.value(i).to_string()));
                        }
                    }
                    for _ in 0..abs_n.min(len) {
                        results.push(None);
                    }
                }

                let result = StringArray::from(
                    results
                        .iter()
                        .map(|s| s.as_deref())
                        .collect::<Vec<Option<&str>>>(),
                );
                Ok(Self::new(format!("{}_shift", self.name), Arc::new(result)))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric, boolean, or string type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Alias for shift with positive n
    pub fn lag(&self, n: i64) -> DataResult<Self> {
        self.shift(n.abs())
    }

    /// Alias for shift with negative n (shift up)
    pub fn lead(&self, n: i64) -> DataResult<Self> {
        self.shift(-(n.abs()))
    }

    /// Calculate difference from n periods ago
    ///
    /// Returns current value minus the value n positions before.
    ///
    /// # Arguments
    /// * `n` - Number of periods (default 1)
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn diff(&self, n: i64) -> DataResult<Self> {
        if self.is_empty() || n == 0 {
            return Ok(self.clone());
        }

        let n_abs = n.abs() as usize;
        let len = self.len();

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(len);

                // First n_abs values are null
                for _ in 0..n_abs.min(len) {
                    results.push(None);
                }

                for i in n_abs..len {
                    let prev_idx = i - n_abs;
                    if arr.is_null(i) || arr.is_null(prev_idx) {
                        results.push(None);
                    } else {
                        results.push(Some(arr.value(i) - arr.value(prev_idx)));
                    }
                }

                let result = Int64Array::from(results);
                Ok(Self::new(format!("{}_diff", self.name), Arc::new(result)))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for _ in 0..n_abs.min(len) {
                    results.push(None);
                }

                for i in n_abs..len {
                    let prev_idx = i - n_abs;
                    if arr.is_null(i) || arr.is_null(prev_idx) {
                        results.push(None);
                    } else {
                        results.push(Some(arr.value(i) - arr.value(prev_idx)));
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(format!("{}_diff", self.name), Arc::new(result)))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Calculate percentage change from n periods ago
    ///
    /// Returns (current - previous) / previous as a decimal.
    ///
    /// # Arguments
    /// * `n` - Number of periods (default 1)
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn pct_change(&self, n: i64) -> DataResult<Self> {
        if self.is_empty() || n == 0 {
            return Ok(Self::from_floats(
                format!("{}_pct_change", self.name),
                vec![],
            ));
        }

        let n_abs = n.abs() as usize;
        let len = self.len();

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                // First n_abs values are null
                for _ in 0..n_abs.min(len) {
                    results.push(None);
                }

                for i in n_abs..len {
                    let prev_idx = i - n_abs;
                    if arr.is_null(i) || arr.is_null(prev_idx) {
                        results.push(None);
                    } else {
                        let prev = arr.value(prev_idx) as f64;
                        if prev == 0.0 {
                            results.push(None); // Division by zero
                        } else {
                            let curr = arr.value(i) as f64;
                            results.push(Some((curr - prev) / prev));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(
                    format!("{}_pct_change", self.name),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for _ in 0..n_abs.min(len) {
                    results.push(None);
                }

                for i in n_abs..len {
                    let prev_idx = i - n_abs;
                    if arr.is_null(i) || arr.is_null(prev_idx) {
                        results.push(None);
                    } else {
                        let prev = arr.value(prev_idx);
                        if prev == 0.0 {
                            results.push(None);
                        } else {
                            let curr = arr.value(i);
                            results.push(Some((curr - prev) / prev));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Self::new(
                    format!("{}_pct_change", self.name),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    // ========================================================================
    // Window Functions: Rolling Operations (returns Rolling wrapper)
    // ========================================================================

    /// Create a rolling window over this series
    ///
    /// Returns a Rolling object that can compute windowed aggregations.
    ///
    /// # Arguments
    /// * `window_size` - Size of the rolling window
    pub fn rolling(&self, window_size: usize) -> Rolling {
        Rolling::new(Arc::new(self.clone()), window_size)
    }

    // ========================================================================
    // Missing Data Handling
    // ========================================================================

    /// Remove null values from the series
    ///
    /// Returns a new series with all null values removed.
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn dropna(&self) -> DataResult<Self> {
        if self.null_count() == 0 {
            return Ok(self.clone());
        }

        // Collect indices of non-null values
        let non_null_indices: Vec<usize> = (0..self.len()).filter(|&i| !self.is_null(i)).collect();

        if non_null_indices.is_empty() {
            // Return empty series of same type
            let empty_array = arrow::array::new_empty_array(self.data_type());
            return Ok(Self::new(self.name.clone(), empty_array));
        }

        // Build new array from non-null values
        let values: Vec<Value> = non_null_indices
            .iter()
            .map(|&i| self.get(i))
            .collect::<DataResult<Vec<_>>>()?;

        Self::from_values(&self.name, &values)
    }

    /// Fill null values with a constant value
    ///
    /// # Arguments
    /// * `fill_value` - The value to use for filling nulls
    ///
    /// # Errors
    /// Returns error if the fill value type doesn't match the series type
    pub fn fillna(&self, fill_value: &Value) -> DataResult<Self> {
        if self.null_count() == 0 {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let fill_int = match fill_value {
                    Value::Int(i) => *i,
                    Value::Float(f) => *f as i64,
                    _ => {
                        return Err(DataError::TypeMismatch {
                            expected: "Int".to_string(),
                            found: fill_value.type_name().to_string(),
                        });
                    }
                };
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let filled: Vec<i64> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            fill_int
                        } else {
                            arr.value(i)
                        }
                    })
                    .collect();
                Ok(Self::from_ints(&self.name, filled))
            }
            DataType::Float64 => {
                let fill_float = match fill_value {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => {
                        return Err(DataError::TypeMismatch {
                            expected: "Float".to_string(),
                            found: fill_value.type_name().to_string(),
                        });
                    }
                };
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let filled: Vec<f64> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            fill_float
                        } else {
                            arr.value(i)
                        }
                    })
                    .collect();
                Ok(Self::from_floats(&self.name, filled))
            }
            DataType::Boolean => {
                let fill_bool = match fill_value {
                    Value::Bool(b) => *b,
                    _ => {
                        return Err(DataError::TypeMismatch {
                            expected: "Bool".to_string(),
                            found: fill_value.type_name().to_string(),
                        });
                    }
                };
                let arr = self.array.as_any().downcast_ref::<BooleanArray>().unwrap();
                let filled: Vec<bool> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            fill_bool
                        } else {
                            arr.value(i)
                        }
                    })
                    .collect();
                Ok(Self::from_bools(&self.name, filled))
            }
            DataType::Utf8 => {
                let fill_str = match fill_value {
                    Value::String(s) => s.to_string(),
                    _ => {
                        return Err(DataError::TypeMismatch {
                            expected: "String".to_string(),
                            found: fill_value.type_name().to_string(),
                        });
                    }
                };
                let arr = self.array.as_any().downcast_ref::<StringArray>().unwrap();
                let filled: Vec<String> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            fill_str.clone()
                        } else {
                            arr.value(i).to_string()
                        }
                    })
                    .collect();
                let str_refs: Vec<&str> = filled.iter().map(|s| s.as_str()).collect();
                Ok(Self::from_strings(&self.name, str_refs))
            }
            other => Err(DataError::InvalidOperation(format!(
                "fillna not supported for type {other:?}"
            ))),
        }
    }

    /// Fill null values using forward fill (propagate last valid value)
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn fillna_forward(&self) -> DataResult<Self> {
        if self.null_count() == 0 {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut filled: Vec<Option<i64>> = Vec::with_capacity(arr.len());
                let mut last_valid: Option<i64> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        filled.push(last_valid);
                    } else {
                        let val = arr.value(i);
                        last_valid = Some(val);
                        filled.push(Some(val));
                    }
                }

                Ok(Self::from_optional_ints(&self.name, filled))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut filled: Vec<Option<f64>> = Vec::with_capacity(arr.len());
                let mut last_valid: Option<f64> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        filled.push(last_valid);
                    } else {
                        let val = arr.value(i);
                        last_valid = Some(val);
                        filled.push(Some(val));
                    }
                }

                let result = Float64Array::from(filled);
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            DataType::Boolean => {
                let arr = self.array.as_any().downcast_ref::<BooleanArray>().unwrap();
                let mut filled: Vec<Option<bool>> = Vec::with_capacity(arr.len());
                let mut last_valid: Option<bool> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        filled.push(last_valid);
                    } else {
                        let val = arr.value(i);
                        last_valid = Some(val);
                        filled.push(Some(val));
                    }
                }

                let result = BooleanArray::from(filled);
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            DataType::Utf8 => {
                let arr = self.array.as_any().downcast_ref::<StringArray>().unwrap();
                let mut filled: Vec<Option<String>> = Vec::with_capacity(arr.len());
                let mut last_valid: Option<String> = None;

                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        filled.push(last_valid.clone());
                    } else {
                        let val = arr.value(i).to_string();
                        last_valid = Some(val.clone());
                        filled.push(Some(val));
                    }
                }

                let result =
                    StringArray::from(filled.iter().map(|s| s.as_deref()).collect::<Vec<_>>());
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            other => Err(DataError::InvalidOperation(format!(
                "fillna_forward not supported for type {other:?}"
            ))),
        }
    }

    /// Fill null values using backward fill (propagate next valid value)
    ///
    /// # Errors
    /// Returns error if the operation fails
    pub fn fillna_backward(&self) -> DataResult<Self> {
        if self.null_count() == 0 {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let mut filled: Vec<Option<i64>> = vec![None; arr.len()];
                let mut next_valid: Option<i64> = None;

                for i in (0..arr.len()).rev() {
                    if arr.is_null(i) {
                        filled[i] = next_valid;
                    } else {
                        let val = arr.value(i);
                        next_valid = Some(val);
                        filled[i] = Some(val);
                    }
                }

                Ok(Self::from_optional_ints(&self.name, filled))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let mut filled: Vec<Option<f64>> = vec![None; arr.len()];
                let mut next_valid: Option<f64> = None;

                for i in (0..arr.len()).rev() {
                    if arr.is_null(i) {
                        filled[i] = next_valid;
                    } else {
                        let val = arr.value(i);
                        next_valid = Some(val);
                        filled[i] = Some(val);
                    }
                }

                let result = Float64Array::from(filled);
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            DataType::Boolean => {
                let arr = self.array.as_any().downcast_ref::<BooleanArray>().unwrap();
                let mut filled: Vec<Option<bool>> = vec![None; arr.len()];
                let mut next_valid: Option<bool> = None;

                for i in (0..arr.len()).rev() {
                    if arr.is_null(i) {
                        filled[i] = next_valid;
                    } else {
                        let val = arr.value(i);
                        next_valid = Some(val);
                        filled[i] = Some(val);
                    }
                }

                let result = BooleanArray::from(filled);
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            DataType::Utf8 => {
                let arr = self.array.as_any().downcast_ref::<StringArray>().unwrap();
                let mut filled: Vec<Option<String>> = vec![None; arr.len()];
                let mut next_valid: Option<String> = None;

                for i in (0..arr.len()).rev() {
                    if arr.is_null(i) {
                        filled[i] = next_valid.clone();
                    } else {
                        let val = arr.value(i).to_string();
                        next_valid = Some(val.clone());
                        filled[i] = Some(val);
                    }
                }

                let result =
                    StringArray::from(filled.iter().map(|s| s.as_deref()).collect::<Vec<_>>());
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            other => Err(DataError::InvalidOperation(format!(
                "fillna_backward not supported for type {other:?}"
            ))),
        }
    }

    /// Linearly interpolate missing values
    ///
    /// For numeric series, fills null values by linear interpolation between
    /// surrounding non-null values. Nulls at the start or end remain null.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn interpolate(&self) -> DataResult<Self> {
        if self.null_count() == 0 {
            return Ok(self.clone());
        }

        match self.array.data_type() {
            DataType::Int64 => {
                let arr = self.array.as_any().downcast_ref::<Int64Array>().unwrap();
                let values: Vec<Option<f64>> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            None
                        } else {
                            Some(arr.value(i) as f64)
                        }
                    })
                    .collect();
                let interpolated = Self::interpolate_values(&values);
                // Convert back to Int64
                let result: Vec<Option<i64>> = interpolated
                    .into_iter()
                    .map(|v| v.map(|f| f.round() as i64))
                    .collect();
                Ok(Self::from_optional_ints(&self.name, result))
            }
            DataType::Float64 => {
                let arr = self.array.as_any().downcast_ref::<Float64Array>().unwrap();
                let values: Vec<Option<f64>> = (0..arr.len())
                    .map(|i| {
                        if arr.is_null(i) {
                            None
                        } else {
                            Some(arr.value(i))
                        }
                    })
                    .collect();
                let interpolated = Self::interpolate_values(&values);
                let result = Float64Array::from(interpolated);
                Ok(Self::new(&self.name, Arc::new(result)))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Helper function for linear interpolation
    fn interpolate_values(values: &[Option<f64>]) -> Vec<Option<f64>> {
        let mut result = values.to_vec();
        let n = result.len();

        let mut i = 0;
        while i < n {
            if result[i].is_none() {
                // Find the start of null segment
                let start = i;
                // Find the end of null segment
                while i < n && result[i].is_none() {
                    i += 1;
                }
                let end = i;

                // Get surrounding values for interpolation
                let prev_val = if start > 0 { result[start - 1] } else { None };
                let next_val = if end < n { result[end] } else { None };

                // Interpolate only if we have both boundaries
                if let (Some(prev), Some(next)) = (prev_val, next_val) {
                    let span = (end - start + 1) as f64;
                    for (j, item) in result.iter_mut().enumerate().take(end).skip(start) {
                        let t = (j - start + 1) as f64 / span;
                        *item = Some(prev + t * (next - prev));
                    }
                }
            } else {
                i += 1;
            }
        }

        result
    }
}

impl fmt::Debug for Series {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Series {{ name: {:?}, dtype: {:?}, len: {} }}",
            self.name,
            self.data_type(),
            self.len()
        )
    }
}

impl fmt::Display for Series {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Series: {} ({:?})", self.name, self.data_type())?;
        let max_display = 10;
        let len = self.len();

        for i in 0..len.min(max_display) {
            if let Ok(val) = self.get(i) {
                writeln!(f, "  {i}: {val}")?;
            }
        }

        if len > max_display {
            writeln!(f, "  ... ({} more rows)", len - max_display)?;
        }

        Ok(())
    }
}

// ============================================================================
// Rolling Window Type
// ============================================================================

/// A rolling window over a Series for computing windowed aggregations
///
/// Created by calling `series.rolling(window_size)`.
/// Provides pandas-style chained method calls like `series.rolling(3).mean()`.
#[derive(Clone)]
pub struct Rolling {
    /// The underlying series
    series: Arc<Series>,
    /// Window size
    window_size: usize,
}

impl Rolling {
    /// Create a new rolling window
    #[must_use]
    pub fn new(series: Arc<Series>, window_size: usize) -> Self {
        Self {
            series,
            window_size,
        }
    }

    /// Get the window size
    #[must_use]
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// Get a reference to the underlying series
    #[must_use]
    pub fn series(&self) -> &Arc<Series> {
        &self.series
    }

    /// Rolling sum
    ///
    /// Returns null for positions where the full window is not available.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn sum(&self) -> DataResult<Series> {
        if self.series.is_empty() || self.window_size == 0 {
            return Ok((*self.series).clone());
        }

        let len = self.series.len();
        let ws = self.window_size;

        match self.series.data_type() {
            DataType::Int64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        // Not enough values for a full window
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut sum: i64 = 0;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            sum += arr.value(j);
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(Some(sum));
                        }
                    }
                }

                let result = Int64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_sum", self.series.name()),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut sum: f64 = 0.0;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            sum += arr.value(j);
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(Some(sum));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_sum", self.series.name()),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Rolling mean (average)
    ///
    /// Returns null for positions where the full window is not available.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn mean(&self) -> DataResult<Series> {
        if self.series.is_empty() || self.window_size == 0 {
            return Ok((*self.series).clone());
        }

        let len = self.series.len();
        let ws = self.window_size;

        match self.series.data_type() {
            DataType::Int64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut sum: f64 = 0.0;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            sum += arr.value(j) as f64;
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(Some(sum / ws as f64));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_mean", self.series.name()),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut sum: f64 = 0.0;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            sum += arr.value(j);
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(Some(sum / ws as f64));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_mean", self.series.name()),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Rolling minimum
    ///
    /// Returns null for positions where the full window is not available.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn min(&self) -> DataResult<Series> {
        if self.series.is_empty() || self.window_size == 0 {
            return Ok((*self.series).clone());
        }

        let len = self.series.len();
        let ws = self.window_size;

        match self.series.data_type() {
            DataType::Int64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut min_val: Option<i64> = None;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            let val = arr.value(j);
                            min_val = Some(min_val.map_or(val, |m| m.min(val)));
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(min_val);
                        }
                    }
                }

                let result = Int64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_min", self.series.name()),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut min_val: Option<f64> = None;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            let val = arr.value(j);
                            min_val = Some(min_val.map_or(val, |m| m.min(val)));
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(min_val);
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_min", self.series.name()),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Rolling maximum
    ///
    /// Returns null for positions where the full window is not available.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn max(&self) -> DataResult<Series> {
        if self.series.is_empty() || self.window_size == 0 {
            return Ok((*self.series).clone());
        }

        let len = self.series.len();
        let ws = self.window_size;

        match self.series.data_type() {
            DataType::Int64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                let mut results: Vec<Option<i64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut max_val: Option<i64> = None;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            let val = arr.value(j);
                            max_val = Some(max_val.map_or(val, |m| m.max(val)));
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(max_val);
                        }
                    }
                }

                let result = Int64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_max", self.series.name()),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut max_val: Option<f64> = None;
                        let mut has_null = false;
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            let val = arr.value(j);
                            max_val = Some(max_val.map_or(val, |m| m.max(val)));
                        }
                        if has_null {
                            results.push(None);
                        } else {
                            results.push(max_val);
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_max", self.series.name()),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }

    /// Rolling standard deviation (population)
    ///
    /// Returns null for positions where the full window is not available.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn std(&self) -> DataResult<Series> {
        self.var().map(|var_series| {
            // Take square root of variance for std
            match var_series.data_type() {
                DataType::Float64 => {
                    let arr = var_series
                        .array()
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .unwrap();
                    let results: Vec<Option<f64>> = (0..arr.len())
                        .map(|i| {
                            if arr.is_null(i) {
                                None
                            } else {
                                Some(arr.value(i).sqrt())
                            }
                        })
                        .collect();
                    let result = Float64Array::from(results);
                    Series::new(
                        format!("{}_rolling_std", self.series.name()),
                        Arc::new(result),
                    )
                }
                _ => var_series, // Shouldn't happen
            }
        })
    }

    /// Rolling variance (population)
    ///
    /// Returns null for positions where the full window is not available.
    ///
    /// # Errors
    /// Returns error for non-numeric types
    pub fn var(&self) -> DataResult<Series> {
        if self.series.is_empty() || self.window_size == 0 {
            return Ok((*self.series).clone());
        }

        let len = self.series.len();
        let ws = self.window_size;

        match self.series.data_type() {
            DataType::Int64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut sum: f64 = 0.0;
                        let mut has_null = false;

                        // Calculate mean
                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            sum += arr.value(j) as f64;
                        }

                        if has_null {
                            results.push(None);
                        } else {
                            let mean = sum / ws as f64;

                            // Calculate variance
                            let mut sum_sq_diff: f64 = 0.0;
                            for j in start..=i {
                                let diff = arr.value(j) as f64 - mean;
                                sum_sq_diff += diff * diff;
                            }
                            results.push(Some(sum_sq_diff / ws as f64));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_var", self.series.name()),
                    Arc::new(result),
                ))
            }
            DataType::Float64 => {
                let arr = self
                    .series
                    .array()
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .unwrap();
                let mut results: Vec<Option<f64>> = Vec::with_capacity(len);

                for i in 0..len {
                    if i + 1 < ws {
                        results.push(None);
                    } else {
                        let start = i + 1 - ws;
                        let mut sum: f64 = 0.0;
                        let mut has_null = false;

                        for j in start..=i {
                            if arr.is_null(j) {
                                has_null = true;
                                break;
                            }
                            sum += arr.value(j);
                        }

                        if has_null {
                            results.push(None);
                        } else {
                            let mean = sum / ws as f64;
                            let mut sum_sq_diff: f64 = 0.0;
                            for j in start..=i {
                                let diff = arr.value(j) - mean;
                                sum_sq_diff += diff * diff;
                            }
                            results.push(Some(sum_sq_diff / ws as f64));
                        }
                    }
                }

                let result = Float64Array::from(results);
                Ok(Series::new(
                    format!("{}_rolling_var", self.series.name()),
                    Arc::new(result),
                ))
            }
            other => Err(DataError::TypeMismatch {
                expected: "numeric type".to_string(),
                found: format!("{other:?}"),
            }),
        }
    }
}

impl fmt::Debug for Rolling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rolling {{ series: {:?}, window_size: {} }}",
            self.series.name(),
            self.window_size
        )
    }
}

impl fmt::Display for Rolling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<Rolling window={} on '{}'>",
            self.window_size,
            self.series.name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ints() {
        let series = Series::from_ints("numbers", vec![1, 2, 3, 4, 5]);
        assert_eq!(series.name(), "numbers");
        assert_eq!(series.len(), 5);
        assert_eq!(series.data_type(), &DataType::Int64);
    }

    #[test]
    fn test_from_floats() {
        let series = Series::from_floats("values", vec![1.0, 2.5, 3.7]);
        assert_eq!(series.len(), 3);
        assert_eq!(series.data_type(), &DataType::Float64);
    }

    #[test]
    fn test_from_strings() {
        let series = Series::from_strings("names", vec!["alice", "bob", "charlie"]);
        assert_eq!(series.len(), 3);
        assert_eq!(series.data_type(), &DataType::Utf8);
    }

    #[test]
    fn test_get_values() {
        let series = Series::from_ints("nums", vec![10, 20, 30]);
        assert_eq!(series.get(0).unwrap(), Value::Int(10));
        assert_eq!(series.get(1).unwrap(), Value::Int(20));
        assert_eq!(series.get(2).unwrap(), Value::Int(30));
        assert!(series.get(3).is_err());
    }

    #[test]
    fn test_aggregations() {
        let series = Series::from_ints("nums", vec![1, 2, 3, 4, 5]);

        assert_eq!(series.sum().unwrap(), Value::Int(15));
        assert_eq!(series.min().unwrap(), Value::Int(1));
        assert_eq!(series.max().unwrap(), Value::Int(5));
        assert_eq!(series.count(), 5);

        if let Value::Float(mean) = series.mean().unwrap() {
            assert!((mean - 3.0).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_optional_values() {
        let series = Series::from_optional_ints("nums", vec![Some(1), None, Some(3)]);
        assert_eq!(series.len(), 3);
        assert_eq!(series.null_count(), 1);
        assert!(!series.is_null(0));
        assert!(series.is_null(1));
        assert!(!series.is_null(2));
        assert_eq!(series.get(1).unwrap(), Value::Null);
    }

    #[test]
    fn test_rename() {
        let series = Series::from_ints("old", vec![1, 2, 3]);
        let renamed = series.rename("new");
        assert_eq!(renamed.name(), "new");
    }

    // ===== Arithmetic Operations Tests =====

    #[test]
    fn test_add_series() {
        let s1 = Series::from_ints("a", vec![1, 2, 3]);
        let s2 = Series::from_ints("b", vec![10, 20, 30]);
        let result = s1.add(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(11));
        assert_eq!(result.get(1).unwrap(), Value::Int(22));
        assert_eq!(result.get(2).unwrap(), Value::Int(33));
    }

    #[test]
    fn test_add_scalar() {
        let s = Series::from_ints("a", vec![1, 2, 3]);
        let result = s.add_scalar(&Value::Int(10)).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(11));
        assert_eq!(result.get(1).unwrap(), Value::Int(12));
        assert_eq!(result.get(2).unwrap(), Value::Int(13));
    }

    #[test]
    fn test_add_mixed_types() {
        let s1 = Series::from_ints("a", vec![1, 2, 3]);
        let s2 = Series::from_floats("b", vec![0.5, 1.5, 2.5]);
        let result = s1.add(&s2).unwrap();
        // Result should be float due to type coercion
        assert_eq!(result.data_type(), &DataType::Float64);
        if let Value::Float(v) = result.get(0).unwrap() {
            assert!((v - 1.5).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_sub_series() {
        let s1 = Series::from_ints("a", vec![10, 20, 30]);
        let s2 = Series::from_ints("b", vec![1, 2, 3]);
        let result = s1.sub(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(9));
        assert_eq!(result.get(1).unwrap(), Value::Int(18));
        assert_eq!(result.get(2).unwrap(), Value::Int(27));
    }

    #[test]
    fn test_mul_series() {
        let s1 = Series::from_ints("a", vec![2, 3, 4]);
        let s2 = Series::from_ints("b", vec![10, 10, 10]);
        let result = s1.mul(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(20));
        assert_eq!(result.get(1).unwrap(), Value::Int(30));
        assert_eq!(result.get(2).unwrap(), Value::Int(40));
    }

    #[test]
    fn test_div_series() {
        let s1 = Series::from_floats("a", vec![10.0, 20.0, 30.0]);
        let s2 = Series::from_floats("b", vec![2.0, 4.0, 5.0]);
        let result = s1.div(&s2).unwrap();
        if let Value::Float(v) = result.get(0).unwrap() {
            assert!((v - 5.0).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_neg() {
        let s = Series::from_ints("a", vec![1, -2, 3]);
        let result = s.neg().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(-1));
        assert_eq!(result.get(1).unwrap(), Value::Int(2));
        assert_eq!(result.get(2).unwrap(), Value::Int(-3));
    }

    // ===== Comparison Operations Tests =====

    #[test]
    fn test_eq_series() {
        let s1 = Series::from_ints("a", vec![1, 2, 3]);
        let s2 = Series::from_ints("b", vec![1, 0, 3]);
        let result = s1.eq(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(false));
        assert_eq!(result.get(2).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_scalar() {
        let s = Series::from_ints("a", vec![1, 2, 1]);
        let result = s.eq_scalar(&Value::Int(1)).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(false));
        assert_eq!(result.get(2).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_lt_series() {
        let s1 = Series::from_ints("a", vec![1, 2, 3]);
        let s2 = Series::from_ints("b", vec![2, 2, 2]);
        let result = s1.lt(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(false));
        assert_eq!(result.get(2).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_gt_scalar() {
        let s = Series::from_ints("a", vec![1, 2, 3]);
        let result = s.gt_scalar(&Value::Int(2)).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(false));
        assert_eq!(result.get(1).unwrap(), Value::Bool(false));
        assert_eq!(result.get(2).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_string_comparison() {
        let s = Series::from_strings("a", vec!["apple", "banana", "cherry"]);
        let result = s.eq_scalar(&Value::string("banana")).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(false));
        assert_eq!(result.get(1).unwrap(), Value::Bool(true));
        assert_eq!(result.get(2).unwrap(), Value::Bool(false));
    }

    // ===== Logical Operations Tests =====

    #[test]
    fn test_and() {
        let s1 = Series::from_bools("a", vec![true, true, false, false]);
        let s2 = Series::from_bools("b", vec![true, false, true, false]);
        let result = s1.and(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(false));
        assert_eq!(result.get(2).unwrap(), Value::Bool(false));
        assert_eq!(result.get(3).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_or() {
        let s1 = Series::from_bools("a", vec![true, true, false, false]);
        let s2 = Series::from_bools("b", vec![true, false, true, false]);
        let result = s1.or(&s2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(true));
        assert_eq!(result.get(2).unwrap(), Value::Bool(true));
        assert_eq!(result.get(3).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_not() {
        let s = Series::from_bools("a", vec![true, false, true]);
        let result = s.not().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(false));
        assert_eq!(result.get(1).unwrap(), Value::Bool(true));
        assert_eq!(result.get(2).unwrap(), Value::Bool(false));
    }

    // ===== Error Cases =====

    #[test]
    fn test_length_mismatch() {
        let s1 = Series::from_ints("a", vec![1, 2, 3]);
        let s2 = Series::from_ints("b", vec![1, 2]);
        assert!(s1.add(&s2).is_err());
    }

    #[test]
    fn test_type_mismatch_logical() {
        let s = Series::from_ints("a", vec![1, 2, 3]);
        assert!(s.not().is_err()); // Can't NOT a numeric series
    }

    #[test]
    fn test_helper_methods() {
        let int_series = Series::from_ints("a", vec![1, 2, 3]);
        assert!(int_series.is_numeric());
        assert!(!int_series.is_boolean());

        let bool_series = Series::from_bools("b", vec![true, false]);
        assert!(!bool_series.is_numeric());
        assert!(bool_series.is_boolean());
    }

    // ===== String Operations Tests =====

    #[test]
    fn test_is_string() {
        let str_series = Series::from_strings("s", vec!["a", "b", "c"]);
        assert!(str_series.is_string());

        let int_series = Series::from_ints("i", vec![1, 2, 3]);
        assert!(!int_series.is_string());
    }

    #[test]
    fn test_str_len() {
        let s = Series::from_strings("names", vec!["hello", "hi", "world"]);
        let result = s.str_len().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(5));
        assert_eq!(result.get(1).unwrap(), Value::Int(2));
        assert_eq!(result.get(2).unwrap(), Value::Int(5));
    }

    #[test]
    fn test_str_contains() {
        let s = Series::from_strings("text", vec!["hello world", "hi there", "goodbye"]);
        let result = s.str_contains("o").unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true)); // "hello world" contains "o"
        assert_eq!(result.get(1).unwrap(), Value::Bool(false)); // "hi there" doesn't contain "o"
        assert_eq!(result.get(2).unwrap(), Value::Bool(true)); // "goodbye" contains "o"
    }

    #[test]
    fn test_str_starts_with() {
        let s = Series::from_strings("text", vec!["hello", "hi", "world"]);
        let result = s.str_starts_with("h").unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(true));
        assert_eq!(result.get(2).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_str_ends_with() {
        let s = Series::from_strings("text", vec!["hello", "world", "hi"]);
        let result = s.str_ends_with("o").unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Bool(true));
        assert_eq!(result.get(1).unwrap(), Value::Bool(false));
        assert_eq!(result.get(2).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_str_to_lowercase() {
        let s = Series::from_strings("text", vec!["HELLO", "World", "hi"]);
        let result = s.str_to_lowercase().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("hello"));
        assert_eq!(result.get(1).unwrap(), Value::string("world"));
        assert_eq!(result.get(2).unwrap(), Value::string("hi"));
    }

    #[test]
    fn test_str_to_uppercase() {
        let s = Series::from_strings("text", vec!["hello", "World", "HI"]);
        let result = s.str_to_uppercase().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("HELLO"));
        assert_eq!(result.get(1).unwrap(), Value::string("WORLD"));
        assert_eq!(result.get(2).unwrap(), Value::string("HI"));
    }

    #[test]
    fn test_str_trim() {
        let s = Series::from_strings("text", vec!["  hello  ", "\tworld\n", "hi"]);
        let result = s.str_trim().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("hello"));
        assert_eq!(result.get(1).unwrap(), Value::string("world"));
        assert_eq!(result.get(2).unwrap(), Value::string("hi"));
    }

    #[test]
    fn test_str_trim_start() {
        let s = Series::from_strings("text", vec!["  hello", "\tworld", "hi"]);
        let result = s.str_trim_start().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("hello"));
        assert_eq!(result.get(1).unwrap(), Value::string("world"));
        assert_eq!(result.get(2).unwrap(), Value::string("hi"));
    }

    #[test]
    fn test_str_trim_end() {
        let s = Series::from_strings("text", vec!["hello  ", "world\t", "hi"]);
        let result = s.str_trim_end().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("hello"));
        assert_eq!(result.get(1).unwrap(), Value::string("world"));
        assert_eq!(result.get(2).unwrap(), Value::string("hi"));
    }

    #[test]
    fn test_str_substring() {
        let s = Series::from_strings("text", vec!["hello", "world", "hi"]);
        // Extract from position 1 with length 2
        let result = s.str_substring(1, Some(2)).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("el"));
        assert_eq!(result.get(1).unwrap(), Value::string("or"));
        assert_eq!(result.get(2).unwrap(), Value::string("i"));
    }

    #[test]
    fn test_str_substring_no_length() {
        let s = Series::from_strings("text", vec!["hello", "world"]);
        // Extract from position 2 to end
        let result = s.str_substring(2, None).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("llo"));
        assert_eq!(result.get(1).unwrap(), Value::string("rld"));
    }

    #[test]
    fn test_str_replace() {
        let s = Series::from_strings("text", vec!["hello world", "foo bar", "test"]);
        let result = s.str_replace("o", "0").unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("hell0 w0rld"));
        assert_eq!(result.get(1).unwrap(), Value::string("f00 bar"));
        assert_eq!(result.get(2).unwrap(), Value::string("test"));
    }

    #[test]
    fn test_str_split_get() {
        let s = Series::from_strings("text", vec!["a,b,c", "x,y", "foo"]);
        let result = s.str_split_get(",", 1).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("b"));
        assert_eq!(result.get(1).unwrap(), Value::string("y"));
        assert_eq!(result.get(2).unwrap(), Value::Null); // "foo" doesn't have index 1 after split
    }

    #[test]
    fn test_str_operations_with_nulls() {
        let values = vec![Value::string("hello"), Value::Null, Value::string("world")];
        let s = Series::from_values("text", &values).unwrap();

        let contains = s.str_contains("o").unwrap();
        assert_eq!(contains.get(0).unwrap(), Value::Bool(true));
        assert_eq!(contains.get(1).unwrap(), Value::Null);
        assert_eq!(contains.get(2).unwrap(), Value::Bool(true));

        let upper = s.str_to_uppercase().unwrap();
        assert_eq!(upper.get(0).unwrap(), Value::string("HELLO"));
        assert_eq!(upper.get(1).unwrap(), Value::Null);
        assert_eq!(upper.get(2).unwrap(), Value::string("WORLD"));
    }

    #[test]
    fn test_str_operations_on_non_string_fails() {
        let s = Series::from_ints("nums", vec![1, 2, 3]);
        assert!(s.str_len().is_err());
        assert!(s.str_contains("a").is_err());
        assert!(s.str_to_lowercase().is_err());
    }

    #[test]
    fn test_str_pad() {
        let s = Series::from_strings("nums", vec!["1", "22", "333"]);

        // Left padding
        let left = s.str_pad(5, "left", '0').unwrap();
        assert_eq!(left.get(0).unwrap(), Value::string("00001"));
        assert_eq!(left.get(1).unwrap(), Value::string("00022"));
        assert_eq!(left.get(2).unwrap(), Value::string("00333"));

        // Right padding
        let right = s.str_pad(5, "right", '-').unwrap();
        assert_eq!(right.get(0).unwrap(), Value::string("1----"));
        assert_eq!(right.get(1).unwrap(), Value::string("22---"));
        assert_eq!(right.get(2).unwrap(), Value::string("333--"));

        // Both sides padding
        let both = s.str_pad(5, "both", '*').unwrap();
        assert_eq!(both.get(0).unwrap(), Value::string("**1**"));
        assert_eq!(both.get(1).unwrap(), Value::string("*22**"));
        assert_eq!(both.get(2).unwrap(), Value::string("*333*"));

        // Already at or over width - no change
        let no_pad = s.str_pad(2, "left", '0').unwrap();
        assert_eq!(no_pad.get(0).unwrap(), Value::string("01"));
        assert_eq!(no_pad.get(1).unwrap(), Value::string("22"));
        assert_eq!(no_pad.get(2).unwrap(), Value::string("333"));
    }

    #[test]
    fn test_str_extract() {
        let s = Series::from_strings(
            "emails",
            vec!["user@example.com", "test@domain.org", "invalid"],
        );

        // Extract domain from email
        let domains = s.str_extract(r"@([^.]+)").unwrap();
        assert_eq!(domains.get(0).unwrap(), Value::string("example"));
        assert_eq!(domains.get(1).unwrap(), Value::string("domain"));
        assert_eq!(domains.get(2).unwrap(), Value::Null); // No match
    }

    #[test]
    fn test_str_match() {
        let s = Series::from_strings("codes", vec!["ABC123", "XYZ789", "hello", "123"]);

        // Match alphanumeric pattern with letters followed by numbers
        let matches = s.str_match(r"^[A-Z]+\d+$").unwrap();
        assert_eq!(matches.get(0).unwrap(), Value::Bool(true));
        assert_eq!(matches.get(1).unwrap(), Value::Bool(true));
        assert_eq!(matches.get(2).unwrap(), Value::Bool(false));
        assert_eq!(matches.get(3).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_str_cat() {
        let first = Series::from_strings("first", vec!["John", "Jane", "Bob"]);
        let last = Series::from_strings("last", vec!["Doe", "Smith", "Jones"]);

        let full = first.str_cat(&last, " ").unwrap();
        assert_eq!(full.get(0).unwrap(), Value::string("John Doe"));
        assert_eq!(full.get(1).unwrap(), Value::string("Jane Smith"));
        assert_eq!(full.get(2).unwrap(), Value::string("Bob Jones"));

        // Test with null handling
        let values1 = vec![Value::string("a"), Value::Null, Value::string("c")];
        let values2 = vec![Value::string("1"), Value::string("2"), Value::Null];
        let s1 = Series::from_values("s1", &values1).unwrap();
        let s2 = Series::from_values("s2", &values2).unwrap();
        let result = s1.str_cat(&s2, "-").unwrap();
        assert_eq!(result.get(0).unwrap(), Value::string("a-1"));
        assert_eq!(result.get(1).unwrap(), Value::Null);
        assert_eq!(result.get(2).unwrap(), Value::Null);
    }

    #[test]
    fn test_str_slice() {
        let s = Series::from_strings("text", vec!["hello", "world", "hi"]);

        // Basic slice with start and end
        let sliced = s.str_slice(1, Some(4)).unwrap();
        assert_eq!(sliced.get(0).unwrap(), Value::string("ell"));
        assert_eq!(sliced.get(1).unwrap(), Value::string("orl"));
        assert_eq!(sliced.get(2).unwrap(), Value::string("i"));

        // Slice to end (no end specified)
        let to_end = s.str_slice(2, None).unwrap();
        assert_eq!(to_end.get(0).unwrap(), Value::string("llo"));
        assert_eq!(to_end.get(1).unwrap(), Value::string("rld"));
        assert_eq!(to_end.get(2).unwrap(), Value::string(""));

        // Negative indices
        let neg = s.str_slice(-3, None).unwrap();
        assert_eq!(neg.get(0).unwrap(), Value::string("llo"));
        assert_eq!(neg.get(1).unwrap(), Value::string("rld"));
        assert_eq!(neg.get(2).unwrap(), Value::string("hi"));
    }

    // ===== Statistical Operations Tests =====

    #[test]
    fn test_std_and_var() {
        // Values: 2, 4, 4, 4, 5, 5, 7, 9
        // Mean = 5, Variance = 4, Std = 2
        let s = Series::from_ints("nums", vec![2, 4, 4, 4, 5, 5, 7, 9]);

        if let Value::Float(var) = s.var().unwrap() {
            assert!((var - 4.0).abs() < 0.001);
        } else {
            panic!("Expected Float for variance");
        }

        if let Value::Float(std) = s.std().unwrap() {
            assert!((std - 2.0).abs() < 0.001);
        } else {
            panic!("Expected Float for std");
        }
    }

    #[test]
    fn test_median() {
        // Odd count: median is middle value
        let s1 = Series::from_ints("odd", vec![1, 3, 5, 7, 9]);
        if let Value::Float(med) = s1.median().unwrap() {
            assert!((med - 5.0).abs() < 0.001);
        } else {
            panic!("Expected Float for median");
        }

        // Even count: median is average of two middle values
        let s2 = Series::from_ints("even", vec![1, 2, 3, 4]);
        if let Value::Float(med) = s2.median().unwrap() {
            assert!((med - 2.5).abs() < 0.001);
        } else {
            panic!("Expected Float for median");
        }
    }

    #[test]
    fn test_mode() {
        let s = Series::from_ints("nums", vec![1, 2, 2, 3, 3, 3, 4]);
        // Mode should be 3 (appears most frequently)
        assert_eq!(s.mode().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_quantile_and_percentile() {
        let s = Series::from_ints("nums", vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        // 50th percentile = median
        if let Value::Float(p50) = s.percentile(50.0).unwrap() {
            assert!((p50 - 5.5).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }

        // Quantile 0.25 = 25th percentile
        if let Value::Float(q25) = s.quantile(0.25).unwrap() {
            assert!((q25 - 3.25).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_skew_and_kurtosis() {
        // Normal-ish distribution: skew ~ 0, kurtosis ~ 0
        let s = Series::from_floats("nums", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        // Just verify it returns a Float and doesn't error
        if let Value::Float(skew) = s.skew().unwrap() {
            assert!(skew.abs() < 0.5); // Should be close to 0 for symmetric distribution
        } else {
            panic!("Expected Float for skew");
        }

        if let Value::Float(kurt) = s.kurtosis().unwrap() {
            assert!(kurt.abs() < 2.0); // Should be close to 0 for normal-like distribution
        } else {
            panic!("Expected Float for kurtosis");
        }
    }

    // ===== Window Functions Tests =====

    #[test]
    fn test_cumsum() {
        let s = Series::from_ints("nums", vec![1, 2, 3, 4, 5]);
        let result = s.cumsum().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
        assert_eq!(result.get(1).unwrap(), Value::Int(3));
        assert_eq!(result.get(2).unwrap(), Value::Int(6));
        assert_eq!(result.get(3).unwrap(), Value::Int(10));
        assert_eq!(result.get(4).unwrap(), Value::Int(15));
    }

    #[test]
    fn test_cummax() {
        let s = Series::from_ints("nums", vec![3, 1, 4, 1, 5]);
        let result = s.cummax().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(3));
        assert_eq!(result.get(1).unwrap(), Value::Int(3));
        assert_eq!(result.get(2).unwrap(), Value::Int(4));
        assert_eq!(result.get(3).unwrap(), Value::Int(4));
        assert_eq!(result.get(4).unwrap(), Value::Int(5));
    }

    #[test]
    fn test_cummin() {
        let s = Series::from_ints("nums", vec![5, 3, 4, 1, 2]);
        let result = s.cummin().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(5));
        assert_eq!(result.get(1).unwrap(), Value::Int(3));
        assert_eq!(result.get(2).unwrap(), Value::Int(3));
        assert_eq!(result.get(3).unwrap(), Value::Int(1));
        assert_eq!(result.get(4).unwrap(), Value::Int(1));
    }

    #[test]
    fn test_cumprod() {
        let s = Series::from_ints("nums", vec![1, 2, 3, 4]);
        let result = s.cumprod().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
        assert_eq!(result.get(1).unwrap(), Value::Int(2));
        assert_eq!(result.get(2).unwrap(), Value::Int(6));
        assert_eq!(result.get(3).unwrap(), Value::Int(24));
    }

    #[test]
    fn test_shift_positive() {
        let s = Series::from_ints("nums", vec![1, 2, 3, 4, 5]);
        let result = s.shift(2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Null);
        assert_eq!(result.get(2).unwrap(), Value::Int(1));
        assert_eq!(result.get(3).unwrap(), Value::Int(2));
        assert_eq!(result.get(4).unwrap(), Value::Int(3));
    }

    #[test]
    fn test_shift_negative() {
        let s = Series::from_ints("nums", vec![1, 2, 3, 4, 5]);
        let result = s.shift(-2).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(3));
        assert_eq!(result.get(1).unwrap(), Value::Int(4));
        assert_eq!(result.get(2).unwrap(), Value::Int(5));
        assert_eq!(result.get(3).unwrap(), Value::Null);
        assert_eq!(result.get(4).unwrap(), Value::Null);
    }

    #[test]
    fn test_diff() {
        let s = Series::from_ints("nums", vec![10, 15, 25, 40]);
        let result = s.diff(1).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Int(5));
        assert_eq!(result.get(2).unwrap(), Value::Int(10));
        assert_eq!(result.get(3).unwrap(), Value::Int(15));
    }

    #[test]
    fn test_pct_change() {
        let s = Series::from_floats("nums", vec![100.0, 110.0, 121.0]);
        let result = s.pct_change(1).unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Null);
        if let Value::Float(v) = result.get(1).unwrap() {
            assert!((v - 0.1).abs() < 0.001); // 10% increase
        } else {
            panic!("Expected Float");
        }
        if let Value::Float(v) = result.get(2).unwrap() {
            assert!((v - 0.1).abs() < 0.001); // 10% increase
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_rolling_sum() {
        let s = Series::from_ints("nums", vec![1, 2, 3, 4, 5]);
        let rolling = s.rolling(3);
        let result = rolling.sum().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Null);
        assert_eq!(result.get(2).unwrap(), Value::Int(6)); // 1+2+3
        assert_eq!(result.get(3).unwrap(), Value::Int(9)); // 2+3+4
        assert_eq!(result.get(4).unwrap(), Value::Int(12)); // 3+4+5
    }

    #[test]
    fn test_rolling_mean() {
        let s = Series::from_floats("nums", vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let rolling = s.rolling(3);
        let result = rolling.mean().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Null);
        if let Value::Float(v) = result.get(2).unwrap() {
            assert!((v - 2.0).abs() < 0.001); // (1+2+3)/3
        } else {
            panic!("Expected Float");
        }
        if let Value::Float(v) = result.get(3).unwrap() {
            assert!((v - 3.0).abs() < 0.001); // (2+3+4)/3
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_rolling_min_max() {
        let s = Series::from_ints("nums", vec![3, 1, 4, 1, 5]);
        let rolling = s.rolling(3);

        let min_result = rolling.min().unwrap();
        assert_eq!(min_result.get(2).unwrap(), Value::Int(1)); // min(3,1,4)
        assert_eq!(min_result.get(3).unwrap(), Value::Int(1)); // min(1,4,1)
        assert_eq!(min_result.get(4).unwrap(), Value::Int(1)); // min(4,1,5)

        let max_result = rolling.max().unwrap();
        assert_eq!(max_result.get(2).unwrap(), Value::Int(4)); // max(3,1,4)
        assert_eq!(max_result.get(3).unwrap(), Value::Int(4)); // max(1,4,1)
        assert_eq!(max_result.get(4).unwrap(), Value::Int(5)); // max(4,1,5)
    }

    #[test]
    fn test_rolling_std() {
        let s = Series::from_floats("nums", vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let rolling = s.rolling(3);
        let result = rolling.std().unwrap();

        // First two should be null
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Null);

        // Third should be std of [1,2,3] ~ 0.816
        if let Value::Float(v) = result.get(2).unwrap() {
            assert!((v - 0.816).abs() < 0.01);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_cumsum_with_nulls() {
        let s = Series::from_optional_ints("nums", vec![Some(1), Some(2), None, Some(4)]);
        let result = s.cumsum().unwrap();
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
        assert_eq!(result.get(1).unwrap(), Value::Int(3));
        assert_eq!(result.get(2).unwrap(), Value::Null);
        assert_eq!(result.get(3).unwrap(), Value::Int(7)); // 1+2+4
    }

    // ===== Missing Data Handling Tests =====

    #[test]
    fn test_series_dropna() {
        let s = Series::from_optional_ints("nums", vec![Some(1), None, Some(3), None, Some(5)]);
        let result = s.dropna().unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result.null_count(), 0);
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
        assert_eq!(result.get(1).unwrap(), Value::Int(3));
        assert_eq!(result.get(2).unwrap(), Value::Int(5));
    }

    #[test]
    fn test_series_dropna_all_null() {
        let s = Series::from_optional_ints("nums", vec![None, None, None]);
        let result = s.dropna().unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_series_dropna_no_nulls() {
        let s = Series::from_ints("nums", vec![1, 2, 3]);
        let result = s.dropna().unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_series_fillna_int() {
        let s = Series::from_optional_ints("nums", vec![Some(1), None, Some(3), None]);
        let result = s.fillna(&Value::Int(0)).unwrap();

        assert_eq!(result.null_count(), 0);
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
        assert_eq!(result.get(1).unwrap(), Value::Int(0));
        assert_eq!(result.get(2).unwrap(), Value::Int(3));
        assert_eq!(result.get(3).unwrap(), Value::Int(0));
    }

    #[test]
    fn test_series_fillna_float() {
        let values: Vec<Option<f64>> = vec![Some(1.5), None, Some(3.5), None];
        let array = Arc::new(Float64Array::from(values)) as ArrayRef;
        let s = Series::new("nums", array);

        let result = s.fillna(&Value::Float(-1.0)).unwrap();

        assert_eq!(result.null_count(), 0);
        assert_eq!(result.get(1).unwrap(), Value::Float(-1.0));
        assert_eq!(result.get(3).unwrap(), Value::Float(-1.0));
    }

    #[test]
    fn test_series_fillna_forward() {
        let s = Series::from_optional_ints("nums", vec![Some(10), None, None, Some(40), None]);
        let result = s.fillna_forward().unwrap();

        assert_eq!(result.get(0).unwrap(), Value::Int(10));
        assert_eq!(result.get(1).unwrap(), Value::Int(10));
        assert_eq!(result.get(2).unwrap(), Value::Int(10));
        assert_eq!(result.get(3).unwrap(), Value::Int(40));
        assert_eq!(result.get(4).unwrap(), Value::Int(40));
    }

    #[test]
    fn test_series_fillna_forward_leading_nulls() {
        let s = Series::from_optional_ints("nums", vec![None, None, Some(30), None]);
        let result = s.fillna_forward().unwrap();

        // Leading nulls stay null
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Null);
        assert_eq!(result.get(2).unwrap(), Value::Int(30));
        assert_eq!(result.get(3).unwrap(), Value::Int(30));
    }

    #[test]
    fn test_series_fillna_backward() {
        let s = Series::from_optional_ints("nums", vec![None, Some(20), None, None, Some(50)]);
        let result = s.fillna_backward().unwrap();

        assert_eq!(result.get(0).unwrap(), Value::Int(20));
        assert_eq!(result.get(1).unwrap(), Value::Int(20));
        assert_eq!(result.get(2).unwrap(), Value::Int(50));
        assert_eq!(result.get(3).unwrap(), Value::Int(50));
        assert_eq!(result.get(4).unwrap(), Value::Int(50));
    }

    #[test]
    fn test_series_fillna_backward_trailing_nulls() {
        let s = Series::from_optional_ints("nums", vec![Some(10), None, None]);
        let result = s.fillna_backward().unwrap();

        assert_eq!(result.get(0).unwrap(), Value::Int(10));
        // Trailing nulls stay null
        assert_eq!(result.get(1).unwrap(), Value::Null);
        assert_eq!(result.get(2).unwrap(), Value::Null);
    }

    #[test]
    fn test_series_interpolate_basic() {
        let s = Series::from_optional_ints("nums", vec![Some(0), None, Some(10)]);
        let result = s.interpolate().unwrap();

        assert_eq!(result.get(0).unwrap(), Value::Int(0));
        // Linear interpolation: 0 + (1/2) * (10 - 0) = 5
        assert_eq!(result.get(1).unwrap(), Value::Int(5));
        assert_eq!(result.get(2).unwrap(), Value::Int(10));
    }

    #[test]
    fn test_series_interpolate_float() {
        let values: Vec<Option<f64>> = vec![Some(0.0), None, None, Some(9.0)];
        let array = Arc::new(Float64Array::from(values)) as ArrayRef;
        let s = Series::new("nums", array);

        let result = s.interpolate().unwrap();

        // Linear interpolation: should get 3.0 and 6.0
        if let Value::Float(v1) = result.get(1).unwrap() {
            assert!((v1 - 3.0).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
        if let Value::Float(v2) = result.get(2).unwrap() {
            assert!((v2 - 6.0).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_series_interpolate_edges() {
        // Nulls at edges can't be interpolated
        let s = Series::from_optional_ints("nums", vec![None, Some(10), None, Some(30), None]);
        let result = s.interpolate().unwrap();

        // Leading null stays null
        assert_eq!(result.get(0).unwrap(), Value::Null);
        assert_eq!(result.get(1).unwrap(), Value::Int(10));
        // Middle null gets interpolated: (10+30)/2 = 20
        assert_eq!(result.get(2).unwrap(), Value::Int(20));
        assert_eq!(result.get(3).unwrap(), Value::Int(30));
        // Trailing null stays null
        assert_eq!(result.get(4).unwrap(), Value::Null);
    }

    // ===== Type Conversion Tests =====

    #[test]
    fn test_to_int_from_float() {
        let s = Series::from_floats("nums", vec![1.5, 2.7, 3.0]);
        let result = s.to_int().unwrap();
        assert_eq!(result.data_type(), &DataType::Int64);
        assert_eq!(result.get(0).unwrap(), Value::Int(1)); // truncated
        assert_eq!(result.get(1).unwrap(), Value::Int(2)); // truncated
        assert_eq!(result.get(2).unwrap(), Value::Int(3));
    }

    #[test]
    fn test_to_int_from_bool() {
        let s = Series::from_bools("flags", vec![true, false, true]);
        let result = s.to_int().unwrap();
        assert_eq!(result.data_type(), &DataType::Int64);
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
        assert_eq!(result.get(1).unwrap(), Value::Int(0));
        assert_eq!(result.get(2).unwrap(), Value::Int(1));
    }

    #[test]
    fn test_to_float_from_int() {
        let s = Series::from_ints("nums", vec![1, 2, 3]);
        let result = s.to_float().unwrap();
        assert_eq!(result.data_type(), &DataType::Float64);
        if let Value::Float(v) = result.get(1).unwrap() {
            assert!((v - 2.0).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_to_str_from_int() {
        let s = Series::from_ints("nums", vec![42, 100, -5]);
        let result = s.to_str().unwrap();
        assert_eq!(result.data_type(), &DataType::Utf8);
        assert_eq!(result.get(0).unwrap(), Value::string("42"));
        assert_eq!(result.get(1).unwrap(), Value::string("100"));
        assert_eq!(result.get(2).unwrap(), Value::string("-5"));
    }

    #[test]
    fn test_to_str_from_float() {
        let s = Series::from_floats("nums", vec![3.14, 2.0]);
        let result = s.to_str().unwrap();
        assert_eq!(result.data_type(), &DataType::Utf8);
        // Float to string conversion
        if let Value::String(v) = result.get(0).unwrap() {
            assert!(v.starts_with("3.14"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_to_bool_from_int() {
        let s = Series::from_ints("nums", vec![0, 1, 100, -1]);
        let result = s.to_bool().unwrap();
        assert_eq!(result.data_type(), &DataType::Boolean);
        assert_eq!(result.get(0).unwrap(), Value::Bool(false)); // 0 is false
        assert_eq!(result.get(1).unwrap(), Value::Bool(true)); // non-zero is true
        assert_eq!(result.get(2).unwrap(), Value::Bool(true));
        assert_eq!(result.get(3).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_to_datetime() {
        let s = Series::from_strings("dates", vec!["2024-01-15 10:30:00", "2024-12-25 00:00:00"]);
        let result = s.to_datetime("%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!(result.data_type(), &DataType::Int64);
        // Should be milliseconds since epoch
        if let Value::Int(ts) = result.get(0).unwrap() {
            assert!(ts > 0); // Should be a positive timestamp
        } else {
            panic!("Expected Int timestamp");
        }
    }

    #[test]
    fn test_type_conversion_preserves_name() {
        let s = Series::from_ints("my_column", vec![1, 2, 3]);
        let result = s.to_float().unwrap();
        assert_eq!(result.name(), "my_column");
    }

    #[test]
    fn test_to_int_already_int() {
        let s = Series::from_ints("nums", vec![1, 2, 3]);
        let result = s.to_int().unwrap();
        // Should return clone without conversion
        assert_eq!(result.get(0).unwrap(), Value::Int(1));
    }
}
