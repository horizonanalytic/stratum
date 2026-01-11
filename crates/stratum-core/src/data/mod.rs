//! Data operations module for Stratum
//!
//! This module provides:
//! - DataFrame: Apache Arrow-backed columnar data structure
//! - Series: Single column representation
//! - GroupedDataFrame: DataFrame partitioned by key columns for aggregation
//! - JoinSpec: Join specifications for DataFrame operations
//! - Cube: OLAP cube for multi-dimensional analytical processing
//! - Type mapping between Stratum and Arrow types
//! - File I/O for Parquet, CSV, and JSON

mod cube;
mod dataframe;
mod error;
mod grouped;
pub mod io;
mod join;
pub mod lazy;
mod memory;
mod parallel;
mod series;
mod sql;
mod types;

pub use cube::{Cube, CubeBuilder, CubeQuery};
pub use dataframe::DataFrame;
pub use error::{DataError, DataResult};
pub use grouped::{AggOp, AggSpec, GroupedDataFrame};
pub use io::{
    read_csv, read_csv_with_options, read_json, read_parquet, write_csv, write_csv_with_options,
    write_json, write_parquet,
};
pub use join::{JoinSpec, JoinType};
pub use lazy::{LazyFrame, LazyGroupBy};
pub use memory::{
    categories as memory_categories, detect_leaks, disable_profiling, enable_profiling,
    is_profiling_enabled, profiler_summary, record_allocation, record_deallocation, reset_profiler,
    set_profiler_gc_stats, CategoryStats, LeakInfo, MemoryProfiler, MemoryStats,
};
pub use parallel::{parallel_threshold, set_parallel_threshold, ParallelConfig};
pub use series::{Rolling, Series};
pub use sql::{sql_query, sql_query_with_name, SqlContext};
pub use types::{arrow_to_stratum_type, stratum_to_arrow_type};

// Re-export elasticube types for convenience
pub use elasticube_core::AggFunc as CubeAggFunc;
