//! Parallel processing configuration and utilities for DataFrame operations
//!
//! This module provides automatic parallelization for large DataFrame operations
//! using Rayon. Operations are parallelized when the row count exceeds a
//! configurable threshold.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Default threshold for automatic parallelization (10,000 rows)
pub const DEFAULT_PARALLEL_THRESHOLD: usize = 10_000;

/// Global parallel threshold - operations with more rows than this will be parallelized
static PARALLEL_THRESHOLD: AtomicUsize = AtomicUsize::new(DEFAULT_PARALLEL_THRESHOLD);

/// Get the current parallel threshold
#[must_use]
pub fn parallel_threshold() -> usize {
    PARALLEL_THRESHOLD.load(Ordering::Relaxed)
}

/// Set the parallel threshold
///
/// Operations with more rows than this threshold will be automatically parallelized.
/// Set to 0 to always parallelize, or `usize::MAX` to disable parallelization.
pub fn set_parallel_threshold(threshold: usize) {
    PARALLEL_THRESHOLD.store(threshold, Ordering::Relaxed);
}

/// Check if the given row count should trigger parallel execution
#[must_use]
pub fn should_parallelize(num_rows: usize) -> bool {
    num_rows > parallel_threshold()
}

/// Configuration builder for parallel operations
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Minimum rows to trigger parallelization
    pub threshold: usize,
    /// Number of threads to use (None = use Rayon default)
    pub num_threads: Option<usize>,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_PARALLEL_THRESHOLD,
            num_threads: None,
        }
    }
}

impl ParallelConfig {
    /// Create a new parallel configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the parallelization threshold
    #[must_use]
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the number of threads
    #[must_use]
    pub fn with_num_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }

    /// Apply this configuration globally
    pub fn apply(&self) {
        set_parallel_threshold(self.threshold);
        if let Some(threads) = self.num_threads {
            // Configure Rayon thread pool if custom thread count specified
            let _ = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build_global();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_threshold() {
        assert_eq!(parallel_threshold(), DEFAULT_PARALLEL_THRESHOLD);
    }

    #[test]
    fn test_should_parallelize() {
        set_parallel_threshold(1000);
        assert!(!should_parallelize(500));
        assert!(!should_parallelize(1000));
        assert!(should_parallelize(1001));
        // Reset to default
        set_parallel_threshold(DEFAULT_PARALLEL_THRESHOLD);
    }
}
