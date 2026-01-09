//! Memory profiling for Stratum runtime
//!
//! This module provides memory usage tracking and profiling capabilities
//! for the VM, DataFrames, Series, and other runtime allocations.
//!
//! # Usage
//!
//! Enable profiling before running code:
//! ```ignore
//! use stratum_core::data::memory::{enable_profiling, profiler_summary, reset_profiler};
//!
//! reset_profiler();
//! enable_profiling();
//! // ... run VM code ...
//! println!("{}", profiler_summary());
//! ```

use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use crate::gc::GcStats;

/// Memory statistics for a single DataFrame or Series
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Number of rows
    pub num_rows: usize,
    /// Number of columns
    pub num_columns: usize,
    /// Total bytes used by data (not including Arrow overhead)
    pub data_bytes: usize,
    /// Estimated total memory including Arrow structures
    pub total_bytes: usize,
    /// Bytes per row (average)
    pub bytes_per_row: f64,
}

impl MemoryStats {
    /// Create new memory stats
    #[must_use]
    pub fn new(num_rows: usize, num_columns: usize, data_bytes: usize, total_bytes: usize) -> Self {
        let bytes_per_row = if num_rows > 0 {
            total_bytes as f64 / num_rows as f64
        } else {
            0.0
        };
        Self {
            num_rows,
            num_columns,
            data_bytes,
            total_bytes,
            bytes_per_row,
        }
    }

    /// Format bytes in human-readable form
    #[must_use]
    pub fn format_bytes(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;
        const GB: usize = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{bytes} B")
        }
    }

    /// Get human-readable total memory
    #[must_use]
    pub fn total_formatted(&self) -> String {
        Self::format_bytes(self.total_bytes)
    }

    /// Get human-readable data memory
    #[must_use]
    pub fn data_formatted(&self) -> String {
        Self::format_bytes(self.data_bytes)
    }
}

/// Standard allocation categories for VM values
pub mod categories {
    pub const LIST: &str = "List";
    pub const MAP: &str = "Map";
    pub const SET: &str = "Set";
    pub const STRUCT: &str = "Struct";
    pub const CLOSURE: &str = "Closure";
    pub const STRING: &str = "String";
    pub const DATAFRAME: &str = "DataFrame";
    pub const SERIES: &str = "Series";
    pub const FUTURE: &str = "Future";
    pub const COROUTINE: &str = "Coroutine";
    pub const OTHER: &str = "Other";
}

/// A tracked memory allocation event
#[derive(Debug, Clone)]
pub struct AllocationEvent {
    /// Timestamp of the allocation
    pub timestamp: Instant,
    /// Size in bytes
    pub bytes: usize,
    /// Description of the allocation
    pub description: String,
    /// Whether this is an allocation (true) or deallocation (false)
    pub is_allocation: bool,
}

/// Statistics for a single allocation category
#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    /// Number of allocations
    pub allocation_count: usize,
    /// Number of deallocations
    pub deallocation_count: usize,
    /// Total bytes allocated
    pub total_allocated: usize,
    /// Total bytes deallocated
    pub total_deallocated: usize,
    /// Current bytes (allocated - deallocated)
    pub current_bytes: usize,
    /// Peak bytes for this category
    pub peak_bytes: usize,
}

/// Potential memory leak information
#[derive(Debug, Clone)]
pub struct LeakInfo {
    /// Category of the leak
    pub category: String,
    /// Number of unmatched allocations
    pub unmatched_allocations: usize,
    /// Bytes potentially leaked
    pub bytes: usize,
}

/// Global memory profiler for tracking runtime allocations
#[derive(Debug)]
pub struct MemoryProfiler {
    /// Whether profiling is enabled
    enabled: bool,
    /// Recorded allocation events (limited to last N events to prevent unbounded growth)
    events: Vec<AllocationEvent>,
    /// Current total allocations by description
    current_allocations: HashMap<String, usize>,
    /// Statistics per category
    category_stats: HashMap<String, CategoryStats>,
    /// Start time for profiling session
    start_time: Instant,
    /// Peak memory usage
    peak_bytes: usize,
    /// Current memory usage
    current_bytes: usize,
    /// GC stats snapshot (captured at report time)
    gc_stats: Option<GcStats>,
    /// Maximum events to keep (for memory bounds)
    max_events: usize,
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Default maximum events to keep
const DEFAULT_MAX_EVENTS: usize = 10_000;

impl MemoryProfiler {
    /// Create a new memory profiler
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: false,
            events: Vec::new(),
            current_allocations: HashMap::new(),
            category_stats: HashMap::new(),
            start_time: Instant::now(),
            peak_bytes: 0,
            current_bytes: 0,
            gc_stats: None,
            max_events: DEFAULT_MAX_EVENTS,
        }
    }

    /// Enable profiling
    pub fn enable(&mut self) {
        self.enabled = true;
        self.start_time = Instant::now();
    }

    /// Disable profiling
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if profiling is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set the maximum number of events to keep
    pub fn set_max_events(&mut self, max: usize) {
        self.max_events = max;
    }

    /// Record an allocation
    pub fn record_allocation(&mut self, bytes: usize, description: &str) {
        if !self.enabled {
            return;
        }

        // Keep events bounded
        if self.events.len() >= self.max_events {
            // Remove oldest 10% when at capacity
            let remove_count = self.max_events / 10;
            self.events.drain(0..remove_count);
        }

        self.events.push(AllocationEvent {
            timestamp: Instant::now(),
            bytes,
            description: description.to_string(),
            is_allocation: true,
        });

        self.current_bytes += bytes;
        if self.current_bytes > self.peak_bytes {
            self.peak_bytes = self.current_bytes;
        }

        *self
            .current_allocations
            .entry(description.to_string())
            .or_insert(0) += bytes;

        // Update category stats
        let stats = self
            .category_stats
            .entry(description.to_string())
            .or_default();
        stats.allocation_count += 1;
        stats.total_allocated += bytes;
        stats.current_bytes += bytes;
        if stats.current_bytes > stats.peak_bytes {
            stats.peak_bytes = stats.current_bytes;
        }
    }

    /// Record a deallocation
    pub fn record_deallocation(&mut self, bytes: usize, description: &str) {
        if !self.enabled {
            return;
        }

        // Keep events bounded
        if self.events.len() >= self.max_events {
            let remove_count = self.max_events / 10;
            self.events.drain(0..remove_count);
        }

        self.events.push(AllocationEvent {
            timestamp: Instant::now(),
            bytes,
            description: description.to_string(),
            is_allocation: false,
        });

        self.current_bytes = self.current_bytes.saturating_sub(bytes);

        if let Some(current) = self.current_allocations.get_mut(description) {
            *current = current.saturating_sub(bytes);
        }

        // Update category stats
        let stats = self
            .category_stats
            .entry(description.to_string())
            .or_default();
        stats.deallocation_count += 1;
        stats.total_deallocated += bytes;
        stats.current_bytes = stats.current_bytes.saturating_sub(bytes);
    }

    /// Get current memory usage
    #[must_use]
    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    /// Get peak memory usage
    #[must_use]
    pub fn peak_bytes(&self) -> usize {
        self.peak_bytes
    }

    /// Get elapsed time since profiling started
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get allocation events
    #[must_use]
    pub fn events(&self) -> &[AllocationEvent] {
        &self.events
    }

    /// Get current allocations by description
    #[must_use]
    pub fn current_allocations(&self) -> &HashMap<String, usize> {
        &self.current_allocations
    }

    /// Get category statistics
    #[must_use]
    pub fn category_stats(&self) -> &HashMap<String, CategoryStats> {
        &self.category_stats
    }

    /// Set GC stats snapshot (called at report time)
    pub fn set_gc_stats(&mut self, stats: GcStats) {
        self.gc_stats = Some(stats);
    }

    /// Get GC stats if available
    #[must_use]
    pub fn gc_stats(&self) -> Option<&GcStats> {
        self.gc_stats.as_ref()
    }

    /// Reset the profiler
    pub fn reset(&mut self) {
        self.events.clear();
        self.current_allocations.clear();
        self.category_stats.clear();
        self.peak_bytes = 0;
        self.current_bytes = 0;
        self.gc_stats = None;
        self.start_time = Instant::now();
    }

    /// Detect potential memory leaks
    ///
    /// Returns a list of categories where allocations significantly exceed deallocations.
    /// This is a heuristic - not all "leaks" are actual leaks (could be long-lived objects).
    #[must_use]
    pub fn detect_leaks(&self) -> Vec<LeakInfo> {
        let mut leaks = Vec::new();

        for (category, stats) in &self.category_stats {
            // Consider it a potential leak if:
            // 1. There are unmatched allocations (alloc_count > dealloc_count)
            // 2. Current bytes is > 0
            let unmatched = stats.allocation_count.saturating_sub(stats.deallocation_count);
            if unmatched > 0 && stats.current_bytes > 0 {
                leaks.push(LeakInfo {
                    category: category.clone(),
                    unmatched_allocations: unmatched,
                    bytes: stats.current_bytes,
                });
            }
        }

        // Sort by bytes descending
        leaks.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        leaks
    }

    /// Generate a summary report
    #[must_use]
    pub fn summary(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Memory Profile Report ===\n\n");

        // Timing
        report.push_str(&format!(
            "Execution Time: {:.3}s\n",
            self.elapsed().as_secs_f64()
        ));

        // Memory overview
        report.push_str(&format!(
            "Peak Memory:    {}\n",
            MemoryStats::format_bytes(self.peak_bytes)
        ));
        report.push_str(&format!(
            "Final Memory:   {}\n",
            MemoryStats::format_bytes(self.current_bytes)
        ));

        // GC stats if available
        if let Some(gc) = &self.gc_stats {
            report.push_str("\n--- GC Statistics ---\n");
            report.push_str(&format!("Collections:     {}\n", gc.collections));
            report.push_str(&format!("Cycles Broken:   {}\n", gc.cycles_broken));
            report.push_str(&format!("Objects Tracked: {}\n", gc.tracked_objects));
            report.push_str(&format!("Threshold:       {}\n", gc.threshold));
        }

        // Allocation breakdown by category
        if !self.category_stats.is_empty() {
            report.push_str("\n--- Allocation Breakdown ---\n");
            let mut sorted: Vec<_> = self.category_stats.iter().collect();
            sorted.sort_by(|a, b| b.1.total_allocated.cmp(&a.1.total_allocated));

            report.push_str(&format!(
                "{:<12} {:>10} {:>10} {:>12} {:>12}\n",
                "Category", "Allocs", "Deallocs", "Total", "Current"
            ));
            report.push_str(&format!("{}\n", "-".repeat(58)));

            for (category, stats) in sorted {
                report.push_str(&format!(
                    "{:<12} {:>10} {:>10} {:>12} {:>12}\n",
                    truncate_str(category, 12),
                    stats.allocation_count,
                    stats.deallocation_count,
                    MemoryStats::format_bytes(stats.total_allocated),
                    MemoryStats::format_bytes(stats.current_bytes),
                ));
            }
        }

        // Leak detection
        let leaks = self.detect_leaks();
        if !leaks.is_empty() {
            report.push_str("\n--- Potential Leaks ---\n");
            report.push_str("(Objects allocated but not deallocated - may be intentional)\n");
            for leak in &leaks {
                report.push_str(&format!(
                    "  {}: {} unmatched ({} bytes)\n",
                    leak.category,
                    leak.unmatched_allocations,
                    MemoryStats::format_bytes(leak.bytes)
                ));
            }
        } else if self.current_bytes == 0 {
            report.push_str("\nNo memory leaks detected.\n");
        }

        report
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

// Global profiler instance using OnceLock for thread-safe lazy initialization
fn global_profiler_instance() -> &'static RwLock<MemoryProfiler> {
    use std::sync::OnceLock;
    static INSTANCE: OnceLock<RwLock<MemoryProfiler>> = OnceLock::new();
    INSTANCE.get_or_init(|| RwLock::new(MemoryProfiler::new()))
}

/// Get a reference to the global memory profiler (for reading)
pub fn global_profiler() -> RwLockReadGuard<'static, MemoryProfiler> {
    global_profiler_instance().read().unwrap()
}

/// Get a mutable reference to the global memory profiler
pub fn global_profiler_mut() -> RwLockWriteGuard<'static, MemoryProfiler> {
    global_profiler_instance().write().unwrap()
}

/// Enable global memory profiling
pub fn enable_profiling() {
    global_profiler_mut().enable();
}

/// Disable global memory profiling
pub fn disable_profiling() {
    global_profiler_mut().disable();
}

/// Check if global profiling is enabled
pub fn is_profiling_enabled() -> bool {
    global_profiler().is_enabled()
}

/// Record a global allocation
pub fn record_allocation(bytes: usize, description: &str) {
    global_profiler_mut().record_allocation(bytes, description);
}

/// Record a global deallocation
pub fn record_deallocation(bytes: usize, description: &str) {
    global_profiler_mut().record_deallocation(bytes, description);
}

/// Get the global profiler summary
pub fn profiler_summary() -> String {
    global_profiler().summary()
}

/// Reset the global profiler
pub fn reset_profiler() {
    global_profiler_mut().reset();
}

/// Set GC stats on the global profiler
pub fn set_profiler_gc_stats(stats: GcStats) {
    global_profiler_mut().set_gc_stats(stats);
}

/// Detect potential leaks from the global profiler
pub fn detect_leaks() -> Vec<LeakInfo> {
    global_profiler().detect_leaks()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats::new(1000, 5, 40000, 50000);
        assert_eq!(stats.num_rows, 1000);
        assert_eq!(stats.num_columns, 5);
        assert_eq!(stats.bytes_per_row, 50.0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(MemoryStats::format_bytes(500), "500 B");
        assert_eq!(MemoryStats::format_bytes(1024), "1.00 KB");
        assert_eq!(MemoryStats::format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(MemoryStats::format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_profiler() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        profiler.record_allocation(1000, "test");
        assert_eq!(profiler.current_bytes(), 1000);
        assert_eq!(profiler.peak_bytes(), 1000);

        profiler.record_allocation(500, "test");
        assert_eq!(profiler.current_bytes(), 1500);
        assert_eq!(profiler.peak_bytes(), 1500);

        profiler.record_deallocation(700, "test");
        assert_eq!(profiler.current_bytes(), 800);
        assert_eq!(profiler.peak_bytes(), 1500);
    }

    #[test]
    fn test_category_stats() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        profiler.record_allocation(1000, categories::LIST);
        profiler.record_allocation(2000, categories::MAP);
        profiler.record_allocation(500, categories::LIST);

        let stats = profiler.category_stats();
        assert_eq!(stats.len(), 2);

        let list_stats = stats.get(categories::LIST).unwrap();
        assert_eq!(list_stats.allocation_count, 2);
        assert_eq!(list_stats.total_allocated, 1500);
        assert_eq!(list_stats.current_bytes, 1500);

        let map_stats = stats.get(categories::MAP).unwrap();
        assert_eq!(map_stats.allocation_count, 1);
        assert_eq!(map_stats.total_allocated, 2000);
    }

    #[test]
    fn test_leak_detection() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        // Allocate some memory
        profiler.record_allocation(1000, categories::LIST);
        profiler.record_allocation(2000, categories::MAP);

        // Only deallocate some
        profiler.record_deallocation(1000, categories::LIST);

        let leaks = profiler.detect_leaks();
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0].category, categories::MAP);
        assert_eq!(leaks[0].bytes, 2000);
    }

    #[test]
    fn test_no_leaks_when_balanced() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        profiler.record_allocation(1000, categories::LIST);
        profiler.record_deallocation(1000, categories::LIST);

        let leaks = profiler.detect_leaks();
        assert!(leaks.is_empty());
    }

    #[test]
    fn test_profiler_reset() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        profiler.record_allocation(1000, "test");
        assert_eq!(profiler.current_bytes(), 1000);

        profiler.reset();
        assert_eq!(profiler.current_bytes(), 0);
        assert_eq!(profiler.peak_bytes(), 0);
        assert!(profiler.category_stats().is_empty());
    }

    #[test]
    fn test_gc_stats_integration() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        let gc_stats = GcStats {
            collections: 5,
            cycles_broken: 10,
            tracked_objects: 100,
            allocation_count: 50,
            threshold: 10000,
        };
        profiler.set_gc_stats(gc_stats.clone());

        let stored = profiler.gc_stats().unwrap();
        assert_eq!(stored.collections, 5);
        assert_eq!(stored.cycles_broken, 10);
    }

    #[test]
    fn test_summary_contains_gc_stats() {
        let mut profiler = MemoryProfiler::new();
        profiler.enable();

        let gc_stats = GcStats {
            collections: 3,
            cycles_broken: 7,
            tracked_objects: 50,
            allocation_count: 25,
            threshold: 10000,
        };
        profiler.set_gc_stats(gc_stats);

        let summary = profiler.summary();
        assert!(summary.contains("GC Statistics"));
        assert!(summary.contains("Collections:"));
        assert!(summary.contains("Cycles Broken:"));
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("hi", 2), "hi");
    }

    #[test]
    fn test_event_limit() {
        let mut profiler = MemoryProfiler::new();
        profiler.set_max_events(100);
        profiler.enable();

        // Add more events than the limit
        for i in 0..150 {
            profiler.record_allocation(10, &format!("test{i}"));
        }

        // Should have trimmed to stay under limit
        assert!(profiler.events().len() <= 100);
    }
}
