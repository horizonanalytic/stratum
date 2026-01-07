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
        let result = numeric::add(&left, &right)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
                numeric::add(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::add(&arr, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::add(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::add(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => return Err(DataError::InvalidOperation(format!(
                "cannot add {} to {:?} Series",
                value.type_name(),
                self.data_type()
            ))),
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
        let result = numeric::sub(&left, &right)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
                numeric::sub(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::sub(&arr, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::sub(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::sub(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => return Err(DataError::InvalidOperation(format!(
                "cannot subtract {} from {:?} Series",
                value.type_name(),
                self.data_type()
            ))),
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
        let result = numeric::mul(&left, &right)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
                numeric::mul(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::mul(&arr, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::mul(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::mul(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => return Err(DataError::InvalidOperation(format!(
                "cannot multiply {:?} Series by {}",
                self.data_type(),
                value.type_name()
            ))),
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
        let result = numeric::div(&left, &right)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
                numeric::div(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::div(&arr, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                numeric::div(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                numeric::div(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => return Err(DataError::InvalidOperation(format!(
                "cannot divide {:?} Series by {}",
                self.data_type(),
                value.type_name()
            ))),
        };
        Ok(Self::new(self.name.clone(), result))
    }

    /// Negate each element in the series
    ///
    /// # Errors
    /// Returns error if type is not numeric
    pub fn neg(&self) -> DataResult<Self> {
        let result = numeric::neg(&self.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = cmp::eq(&self.array, &other.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = cmp::neq(&self.array, &other.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = cmp::lt(&self.array, &other.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = cmp::lt_eq(&self.array, &other.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = cmp::gt(&self.array, &other.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = cmp::gt_eq(&self.array, &other.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = boolean::and(&left, &right)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = boolean::or(&left, &right)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
        Ok(Self::new(format!("{}_or", self.name), Arc::new(result)))
    }

    /// Element-wise logical NOT
    ///
    /// # Errors
    /// Returns error if series is not boolean
    pub fn not(&self) -> DataResult<Self> {
        let arr = self.as_boolean()?;
        let result = boolean::not(&arr)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        let result = length::length(&self.array)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
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
        Ok(Self::new(format!("{}_contains", self.name), Arc::new(result)))
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
                    arr.value(i)
                        .split(delimiter)
                        .nth(index)
                        .map(String::from)
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
        F: Fn(&dyn arrow::array::Datum, &dyn arrow::array::Datum) -> Result<BooleanArray, arrow::error::ArrowError>,
    {
        let result = match (self.array.data_type(), value) {
            (DataType::Int64, Value::Int(v)) => {
                let scalar = Scalar::new(Int64Array::from(vec![*v]));
                cmp_fn(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Int64, Value::Float(v)) => {
                let arr = self.cast_to_float()?;
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                cmp_fn(&arr, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Int(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v as f64]));
                cmp_fn(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Float64, Value::Float(v)) => {
                let scalar = Scalar::new(Float64Array::from(vec![*v]));
                cmp_fn(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Boolean, Value::Bool(v)) => {
                let scalar = Scalar::new(BooleanArray::from(vec![*v]));
                cmp_fn(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            (DataType::Utf8, Value::String(v)) => {
                let scalar = Scalar::new(StringArray::from(vec![v.as_str()]));
                cmp_fn(&self.array, &scalar)
                    .map_err(|e| DataError::Arrow(e.to_string()))?
            }
            _ => return Err(DataError::InvalidOperation(format!(
                "cannot compare {:?} Series with {}",
                self.data_type(),
                value.type_name()
            ))),
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
        let values = vec![
            Value::string("hello"),
            Value::Null,
            Value::string("world"),
        ];
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
}
