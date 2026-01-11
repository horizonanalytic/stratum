//! Error types for data operations

use std::fmt;

/// Result type for data operations
pub type DataResult<T> = Result<T, DataError>;

/// Errors that can occur during data operations
#[derive(Debug, Clone)]
pub enum DataError {
    /// Arrow error (from arrow-rs)
    Arrow(String),
    /// Column not found in DataFrame
    ColumnNotFound(String),
    /// Invalid column index
    InvalidColumnIndex(usize),
    /// Type mismatch during operation
    TypeMismatch { expected: String, found: String },
    /// Invalid operation for the data type
    InvalidOperation(String),
    /// I/O error (file read/write)
    Io(String),
    /// Schema mismatch
    SchemaMismatch(String),
    /// Empty DataFrame where data was expected
    EmptyData,
    /// Parquet error
    Parquet(String),
    /// CSV error
    Csv(String),
    /// JSON error
    Json(String),
    /// Index out of bounds
    OutOfBounds { index: usize, length: usize },
    /// SQL/DataFusion error
    Sql(String),
    /// OLAP Cube error
    Cube(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Arrow(msg) => write!(f, "Arrow error: {msg}"),
            DataError::ColumnNotFound(name) => write!(f, "column not found: {name}"),
            DataError::InvalidColumnIndex(idx) => write!(f, "invalid column index: {idx}"),
            DataError::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            DataError::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
            DataError::Io(msg) => write!(f, "I/O error: {msg}"),
            DataError::SchemaMismatch(msg) => write!(f, "schema mismatch: {msg}"),
            DataError::EmptyData => write!(f, "empty DataFrame"),
            DataError::Parquet(msg) => write!(f, "Parquet error: {msg}"),
            DataError::Csv(msg) => write!(f, "CSV error: {msg}"),
            DataError::Json(msg) => write!(f, "JSON error: {msg}"),
            DataError::OutOfBounds { index, length } => {
                write!(f, "index {index} out of bounds for length {length}")
            }
            DataError::Sql(msg) => write!(f, "SQL error: {msg}"),
            DataError::Cube(msg) => write!(f, "Cube error: {msg}"),
        }
    }
}

impl std::error::Error for DataError {}

impl From<arrow::error::ArrowError> for DataError {
    fn from(err: arrow::error::ArrowError) -> Self {
        DataError::Arrow(err.to_string())
    }
}

impl From<std::io::Error> for DataError {
    fn from(err: std::io::Error) -> Self {
        DataError::Io(err.to_string())
    }
}

impl From<parquet::errors::ParquetError> for DataError {
    fn from(err: parquet::errors::ParquetError) -> Self {
        DataError::Parquet(err.to_string())
    }
}

impl From<datafusion::error::DataFusionError> for DataError {
    fn from(err: datafusion::error::DataFusionError) -> Self {
        DataError::Sql(err.to_string())
    }
}

impl From<elasticube_core::Error> for DataError {
    fn from(err: elasticube_core::Error) -> Self {
        DataError::Cube(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DataError::ColumnNotFound("age".to_string());
        assert_eq!(err.to_string(), "column not found: age");

        let err = DataError::TypeMismatch {
            expected: "Int".to_string(),
            found: "String".to_string(),
        };
        assert_eq!(err.to_string(), "type mismatch: expected Int, found String");
    }
}
