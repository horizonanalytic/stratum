//! File I/O operations for DataFrame
//!
//! Supports reading and writing DataFrames in Parquet, CSV, and JSON formats.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;

use arrow::array::RecordBatch;
use arrow::datatypes::SchemaRef;
use arrow_csv::{ReaderBuilder as CsvReaderBuilder, WriterBuilder as CsvWriterBuilder};
use arrow_json::{LineDelimitedWriter as JsonLineWriter, ReaderBuilder as JsonReaderBuilder};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;

use super::dataframe::DataFrame;
use super::error::{DataError, DataResult};

/// Read a Parquet file into a DataFrame
///
/// # Errors
/// Returns error if file cannot be read or is not valid Parquet
pub fn read_parquet<P: AsRef<Path>>(path: P) -> DataResult<DataFrame> {
    let file = File::open(path.as_ref()).map_err(|e| {
        DataError::Io(format!("failed to open file '{}': {}", path.as_ref().display(), e))
    })?;

    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| DataError::Parquet(format!("failed to read parquet: {e}")))?;

    let schema = builder.schema().clone();
    let reader = builder
        .build()
        .map_err(|e| DataError::Parquet(format!("failed to build reader: {e}")))?;

    let batches: Vec<RecordBatch> = reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DataError::Parquet(format!("failed to read batches: {e}")))?;

    DataFrame::from_batches(schema, batches)
}

/// Write a DataFrame to a Parquet file
///
/// # Errors
/// Returns error if file cannot be written
pub fn write_parquet<P: AsRef<Path>>(df: &DataFrame, path: P) -> DataResult<()> {
    let file = File::create(path.as_ref()).map_err(|e| {
        DataError::Io(format!("failed to create file '{}': {}", path.as_ref().display(), e))
    })?;

    let schema = df.schema().clone();
    let mut writer = ArrowWriter::try_new(file, schema, None)
        .map_err(|e| DataError::Parquet(format!("failed to create writer: {e}")))?;

    for batch in df.batches() {
        writer
            .write(batch)
            .map_err(|e| DataError::Parquet(format!("failed to write batch: {e}")))?;
    }

    writer
        .close()
        .map_err(|e| DataError::Parquet(format!("failed to close writer: {e}")))?;

    Ok(())
}

/// Read a CSV file into a DataFrame
///
/// # Errors
/// Returns error if file cannot be read or is not valid CSV
pub fn read_csv<P: AsRef<Path>>(path: P) -> DataResult<DataFrame> {
    read_csv_with_options(path, true, b',')
}

/// Read a CSV file into a DataFrame with options
///
/// # Arguments
/// * `path` - Path to the CSV file
/// * `has_header` - Whether the first row is a header
/// * `delimiter` - Field delimiter character
///
/// # Errors
/// Returns error if file cannot be read or is not valid CSV
pub fn read_csv_with_options<P: AsRef<Path>>(
    path: P,
    has_header: bool,
    delimiter: u8,
) -> DataResult<DataFrame> {
    let file = File::open(path.as_ref()).map_err(|e| {
        DataError::Io(format!("failed to open file '{}': {}", path.as_ref().display(), e))
    })?;

    let reader = BufReader::new(file);

    // Infer schema from the file
    let (schema, _) = arrow_csv::reader::Format::default()
        .with_header(has_header)
        .with_delimiter(delimiter)
        .infer_schema(
            BufReader::new(File::open(path.as_ref()).map_err(|e| {
                DataError::Io(format!("failed to open file for schema inference: {e}"))
            })?),
            Some(100), // Sample 100 rows for schema inference
        )
        .map_err(|e| DataError::Csv(format!("failed to infer schema: {e}")))?;

    let schema_ref: SchemaRef = Arc::new(schema);

    let csv_reader = CsvReaderBuilder::new(schema_ref.clone())
        .with_header(has_header)
        .with_delimiter(delimiter)
        .build(reader)
        .map_err(|e| DataError::Csv(format!("failed to build CSV reader: {e}")))?;

    let batches: Vec<RecordBatch> = csv_reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DataError::Csv(format!("failed to read CSV batches: {e}")))?;

    DataFrame::from_batches(schema_ref, batches)
}

/// Write a DataFrame to a CSV file
///
/// # Errors
/// Returns error if file cannot be written
pub fn write_csv<P: AsRef<Path>>(df: &DataFrame, path: P) -> DataResult<()> {
    write_csv_with_options(df, path, true, b',')
}

/// Write a DataFrame to a CSV file with options
///
/// # Arguments
/// * `df` - DataFrame to write
/// * `path` - Output file path
/// * `with_header` - Whether to write header row
/// * `delimiter` - Field delimiter character
///
/// # Errors
/// Returns error if file cannot be written
pub fn write_csv_with_options<P: AsRef<Path>>(
    df: &DataFrame,
    path: P,
    with_header: bool,
    delimiter: u8,
) -> DataResult<()> {
    let file = File::create(path.as_ref()).map_err(|e| {
        DataError::Io(format!("failed to create file '{}': {}", path.as_ref().display(), e))
    })?;

    let writer = BufWriter::new(file);
    let mut csv_writer = CsvWriterBuilder::new()
        .with_header(with_header)
        .with_delimiter(delimiter)
        .build(writer);

    for batch in df.batches() {
        csv_writer
            .write(batch)
            .map_err(|e| DataError::Csv(format!("failed to write batch: {e}")))?;
    }

    Ok(())
}

/// Read a JSON file (records format) into a DataFrame
///
/// Expects JSON in newline-delimited JSON (NDJSON) format where each line is a JSON object,
/// or a JSON array of objects.
///
/// # Errors
/// Returns error if file cannot be read or is not valid JSON
pub fn read_json<P: AsRef<Path>>(path: P) -> DataResult<DataFrame> {
    let file = File::open(path.as_ref()).map_err(|e| {
        DataError::Io(format!("failed to open file '{}': {}", path.as_ref().display(), e))
    })?;

    let reader = BufReader::new(file);

    // Infer schema from the file
    let (schema, _) = arrow_json::reader::infer_json_schema(
        BufReader::new(File::open(path.as_ref()).map_err(|e| {
            DataError::Io(format!("failed to open file for schema inference: {e}"))
        })?),
        Some(100), // Sample 100 rows for schema inference
    )
    .map_err(|e| DataError::Json(format!("failed to infer schema: {e}")))?;

    let schema_ref: SchemaRef = Arc::new(schema);

    let json_reader = JsonReaderBuilder::new(schema_ref.clone())
        .build(reader)
        .map_err(|e| DataError::Json(format!("failed to build JSON reader: {e}")))?;

    let batches: Vec<RecordBatch> = json_reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DataError::Json(format!("failed to read JSON batches: {e}")))?;

    DataFrame::from_batches(schema_ref, batches)
}

/// Write a DataFrame to a JSON file (newline-delimited format)
///
/// Writes as newline-delimited JSON (NDJSON) where each line is a JSON object.
///
/// # Errors
/// Returns error if file cannot be written
pub fn write_json<P: AsRef<Path>>(df: &DataFrame, path: P) -> DataResult<()> {
    let file = File::create(path.as_ref()).map_err(|e| {
        DataError::Io(format!("failed to create file '{}': {}", path.as_ref().display(), e))
    })?;

    let writer = BufWriter::new(file);
    let mut json_writer = JsonLineWriter::new(writer);

    for batch in df.batches() {
        json_writer
            .write(batch)
            .map_err(|e| DataError::Json(format!("failed to write batch: {e}")))?;
    }

    json_writer
        .finish()
        .map_err(|e| DataError::Json(format!("failed to finish writing: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Series;
    use tempfile::tempdir;

    fn sample_dataframe() -> DataFrame {
        let names = Series::from_strings("name", vec!["Alice", "Bob", "Charlie"]);
        let ages = Series::from_ints("age", vec![30, 25, 35]);
        let scores = Series::from_floats("score", vec![85.5, 92.0, 78.3]);

        DataFrame::from_series(vec![names, ages, scores]).unwrap()
    }

    #[test]
    fn test_parquet_roundtrip() {
        let df = sample_dataframe();
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.parquet");

        write_parquet(&df, &path).unwrap();
        let loaded = read_parquet(&path).unwrap();

        assert_eq!(loaded.num_rows(), df.num_rows());
        assert_eq!(loaded.num_columns(), df.num_columns());
        assert_eq!(loaded.columns(), df.columns());
    }

    #[test]
    fn test_csv_roundtrip() {
        let df = sample_dataframe();
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.csv");

        write_csv(&df, &path).unwrap();
        let loaded = read_csv(&path).unwrap();

        assert_eq!(loaded.num_rows(), df.num_rows());
        assert_eq!(loaded.num_columns(), df.num_columns());
    }

    #[test]
    fn test_json_roundtrip() {
        let df = sample_dataframe();
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        write_json(&df, &path).unwrap();
        let loaded = read_json(&path).unwrap();

        assert_eq!(loaded.num_rows(), df.num_rows());
        assert_eq!(loaded.num_columns(), df.num_columns());
    }
}
