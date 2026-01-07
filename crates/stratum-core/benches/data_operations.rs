//! Benchmark suite for Stratum data operations
//!
//! Tests performance against targets from planning/04-data-operations.md:
//! - read_parquet: < 100ms for 1M rows
//! - filter: < 50ms for 1M rows
//! - group_by + agg: < 100ms for 1M rows
//! - join: < 200ms for 1M rows
//! - write_parquet: < 100ms for 1M rows

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;

use stratum_core::data::{DataFrame, GroupedDataFrame, Series, AggSpec};

/// Generate a DataFrame with the specified number of rows
fn generate_test_dataframe(num_rows: usize) -> DataFrame {
    // Create sample data
    let mut ids = Vec::with_capacity(num_rows);
    let mut regions = Vec::with_capacity(num_rows);
    let mut amounts = Vec::with_capacity(num_rows);
    let mut scores = Vec::with_capacity(num_rows);

    let region_options = ["North", "South", "East", "West"];

    for i in 0..num_rows {
        ids.push(i as i64);
        regions.push(region_options[i % 4]);
        amounts.push((i % 1000) as i64);
        scores.push((i % 100) as f64 + 0.5);
    }

    let id_series = Series::from_ints("id", ids);
    let region_series = Series::from_strings("region", regions);
    let amount_series = Series::from_ints("amount", amounts);
    let score_series = Series::from_floats("score", scores);

    DataFrame::from_series(vec![id_series, region_series, amount_series, score_series])
        .expect("Failed to create DataFrame")
}

/// Benchmark DataFrame creation
fn bench_dataframe_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dataframe_creation");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| black_box(generate_test_dataframe(size)));
        });
    }

    group.finish();
}

/// Benchmark filter operation
fn bench_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        let df = generate_test_dataframe(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            // Filter to keep rows where amount > 500
            b.iter(|| {
                let indices: Vec<usize> = (0..df.num_rows())
                    .filter(|&i| {
                        let col = df.column("amount").unwrap();
                        match col.get(i) {
                            Ok(stratum_core::bytecode::Value::Int(v)) => v > 500,
                            _ => false,
                        }
                    })
                    .collect();
                black_box(df.filter_by_indices(&indices).unwrap())
            });
        });
    }

    group.finish();
}

/// Benchmark group_by + aggregation
fn bench_group_by_aggregate(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_by_agg");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        let df = generate_test_dataframe(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let grouped = GroupedDataFrame::new(
                    Arc::new(df.clone()),
                    vec!["region".to_string()]
                ).unwrap();
                let specs = vec![
                    AggSpec::sum("amount", "total"),
                    AggSpec::count("n"),
                    AggSpec::mean("score", "avg_score"),
                ];
                black_box(grouped.aggregate(&specs).unwrap())
            });
        });
    }

    group.finish();
}

/// Benchmark column access
fn bench_column_access(c: &mut Criterion) {
    let df = generate_test_dataframe(1_000_000);

    c.bench_function("column_access_1M", |b| {
        b.iter(|| {
            let col = black_box(df.column("amount").unwrap());
            black_box(col.len())
        });
    });
}

/// Benchmark Series aggregations
fn bench_series_aggregations(c: &mut Criterion) {
    let mut group = c.benchmark_group("series_agg");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        let values: Vec<i64> = (0..*size).map(|i| i % 1000).collect();
        let series = Series::from_ints("values", values);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("sum", size), size, |b, _| {
            b.iter(|| black_box(series.sum().unwrap()));
        });
        group.bench_with_input(BenchmarkId::new("min", size), size, |b, _| {
            b.iter(|| black_box(series.min().unwrap()));
        });
        group.bench_with_input(BenchmarkId::new("max", size), size, |b, _| {
            b.iter(|| black_box(series.max().unwrap()));
        });
    }

    group.finish();
}

/// Benchmark head/tail operations
fn bench_head_tail(c: &mut Criterion) {
    let df = generate_test_dataframe(1_000_000);

    c.bench_function("head_10_from_1M", |b| {
        b.iter(|| black_box(df.head(10).unwrap()));
    });

    c.bench_function("tail_10_from_1M", |b| {
        b.iter(|| black_box(df.tail(10).unwrap()));
    });
}

/// Benchmark select columns
fn bench_select(c: &mut Criterion) {
    let df = generate_test_dataframe(1_000_000);

    c.bench_function("select_2_columns_from_1M", |b| {
        b.iter(|| black_box(df.select(&["id", "amount"]).unwrap()));
    });
}

/// Benchmark DataFrame iteration
fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("iteration");

    for size in [1_000, 10_000, 100_000].iter() {
        let df = generate_test_dataframe(*size);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("row_iter", size), size, |b, _| {
            b.iter(|| {
                let mut count = 0;
                for row in df.iter_rows() {
                    black_box(row.unwrap());
                    count += 1;
                }
                black_box(count)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dataframe_creation,
    bench_filter,
    bench_group_by_aggregate,
    bench_column_access,
    bench_series_aggregations,
    bench_head_tail,
    bench_select,
    bench_iteration,
);

criterion_main!(benches);
