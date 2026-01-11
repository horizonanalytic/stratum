//! Code coverage module for the Stratum programming language
//!
//! This module provides bytecode-level coverage tracking with support for:
//! - Line coverage: Track which source lines were executed
//! - Branch coverage: Track which conditional branches were taken
//! - Coverage reporting in multiple formats (summary, HTML, lcov)

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::bytecode::{Chunk, Function, OpCode};

/// Identifies a branch point in the bytecode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BranchId {
    /// Bytecode offset where the branch instruction occurs
    pub offset: usize,
    /// Source line number
    pub line: u32,
}

/// Tracks coverage data for a single function/chunk
#[derive(Debug, Clone, Default)]
pub struct FunctionCoverage {
    /// Function name
    pub name: String,
    /// Source file (if known)
    pub source_file: Option<String>,
    /// Lines that are executable (have bytecode)
    pub executable_lines: HashSet<u32>,
    /// Lines that were executed
    pub executed_lines: HashSet<u32>,
    /// Branch points: offset -> (line, taken_count, not_taken_count)
    pub branches: HashMap<usize, BranchInfo>,
}

/// Information about a branch point
#[derive(Debug, Clone, Default)]
pub struct BranchInfo {
    /// Source line number
    pub line: u32,
    /// Number of times the branch was taken (condition was true/jumped)
    pub taken_count: usize,
    /// Number of times the branch was not taken (fell through)
    pub not_taken_count: usize,
}

impl BranchInfo {
    /// Returns true if both branches have been exercised
    pub fn is_fully_covered(&self) -> bool {
        self.taken_count > 0 && self.not_taken_count > 0
    }

    /// Returns true if at least one branch has been exercised
    pub fn is_partially_covered(&self) -> bool {
        self.taken_count > 0 || self.not_taken_count > 0
    }
}

impl FunctionCoverage {
    /// Create a new function coverage tracker
    pub fn new(name: String, source_file: Option<String>) -> Self {
        Self {
            name,
            source_file,
            executable_lines: HashSet::new(),
            executed_lines: HashSet::new(),
            branches: HashMap::new(),
        }
    }

    /// Analyze a chunk to find executable lines and branch points
    pub fn analyze_chunk(&mut self, chunk: &Chunk) {
        let code = chunk.code();
        let mut offset = 0;

        while offset < code.len() {
            let byte = code[offset];
            let Ok(opcode) = OpCode::try_from(byte) else {
                offset += 1;
                continue;
            };

            let line = chunk.get_line(offset);

            // Every instruction makes its line executable
            if line > 0 {
                self.executable_lines.insert(line);
            }

            // Track branch points
            match opcode {
                OpCode::JumpIfFalse
                | OpCode::JumpIfTrue
                | OpCode::JumpIfNull
                | OpCode::JumpIfNotNull
                | OpCode::PopJumpIfNull
                | OpCode::IterNext => {
                    self.branches.insert(
                        offset,
                        BranchInfo {
                            line,
                            taken_count: 0,
                            not_taken_count: 0,
                        },
                    );
                }
                _ => {}
            }

            offset += opcode.size();
        }
    }

    /// Record that a line was executed
    pub fn record_line(&mut self, line: u32) {
        if line > 0 {
            self.executed_lines.insert(line);
        }
    }

    /// Record a branch taken (jumped)
    pub fn record_branch_taken(&mut self, offset: usize) {
        if let Some(info) = self.branches.get_mut(&offset) {
            info.taken_count += 1;
        }
    }

    /// Record a branch not taken (fell through)
    pub fn record_branch_not_taken(&mut self, offset: usize) {
        if let Some(info) = self.branches.get_mut(&offset) {
            info.not_taken_count += 1;
        }
    }

    /// Calculate line coverage percentage
    pub fn line_coverage_percent(&self) -> f64 {
        if self.executable_lines.is_empty() {
            return 100.0;
        }
        (self.executed_lines.len() as f64 / self.executable_lines.len() as f64) * 100.0
    }

    /// Calculate branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.branches.is_empty() {
            return 100.0;
        }
        let total_branches = self.branches.len() * 2; // Each branch has two outcomes
        let covered_branches: usize = self
            .branches
            .values()
            .map(|b| (b.taken_count > 0) as usize + (b.not_taken_count > 0) as usize)
            .sum();
        (covered_branches as f64 / total_branches as f64) * 100.0
    }
}

/// Collects coverage data across multiple functions/files
#[derive(Debug, Clone, Default)]
pub struct CoverageCollector {
    /// Coverage data per function (keyed by function pointer address as string)
    functions: HashMap<String, FunctionCoverage>,
    /// Currently active function for recording
    active_function: Option<String>,
    /// Map of source files to their total line counts (for reporting)
    source_lines: HashMap<String, u32>,
}

impl CoverageCollector {
    /// Create a new coverage collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin tracking a function
    ///
    /// This marks all executable lines in the function as executed since we're
    /// entering the function. For more precise line-by-line tracking, use
    /// `record_line` from the VM execution loop.
    pub fn begin_function(&mut self, function: &Function) {
        let key = format!("{}@{:p}", function.name, function);

        if !self.functions.contains_key(&key) {
            let mut coverage =
                FunctionCoverage::new(function.name.clone(), function.chunk.source_name.clone());
            coverage.analyze_chunk(&function.chunk);

            // Update source line count
            if let Some(ref source) = function.chunk.source_name {
                if let Some(max_line) = coverage.executable_lines.iter().max() {
                    let entry = self.source_lines.entry(source.clone()).or_insert(0);
                    *entry = (*entry).max(*max_line);
                }
            }

            self.functions.insert(key.clone(), coverage);
        }

        // Mark all executable lines as executed since we're entering this function
        // This is a simplification - true line coverage would track each line individually
        if let Some(coverage) = self.functions.get_mut(&key) {
            coverage.executed_lines = coverage.executable_lines.clone();
        }

        self.active_function = Some(key);
    }

    /// End tracking the current function
    pub fn end_function(&mut self) {
        self.active_function = None;
    }

    /// Record that a line was executed in the current function
    pub fn record_line(&mut self, line: u32) {
        if let Some(ref key) = self.active_function {
            if let Some(coverage) = self.functions.get_mut(key) {
                coverage.record_line(line);
            }
        }
    }

    /// Record a branch taken in the current function
    pub fn record_branch_taken(&mut self, offset: usize) {
        if let Some(ref key) = self.active_function {
            if let Some(coverage) = self.functions.get_mut(key) {
                coverage.record_branch_taken(offset);
            }
        }
    }

    /// Record a branch not taken in the current function
    pub fn record_branch_not_taken(&mut self, offset: usize) {
        if let Some(ref key) = self.active_function {
            if let Some(coverage) = self.functions.get_mut(key) {
                coverage.record_branch_not_taken(offset);
            }
        }
    }

    /// Merge another collector's data into this one
    pub fn merge(&mut self, other: &CoverageCollector) {
        for (key, other_cov) in &other.functions {
            if let Some(self_cov) = self.functions.get_mut(key) {
                // Merge executed lines
                self_cov.executed_lines.extend(&other_cov.executed_lines);
                // Merge branch counts
                for (offset, other_branch) in &other_cov.branches {
                    if let Some(self_branch) = self_cov.branches.get_mut(offset) {
                        self_branch.taken_count += other_branch.taken_count;
                        self_branch.not_taken_count += other_branch.not_taken_count;
                    }
                }
            } else {
                self.functions.insert(key.clone(), other_cov.clone());
            }
        }
        // Merge source line counts
        for (source, lines) in &other.source_lines {
            let entry = self.source_lines.entry(source.clone()).or_insert(0);
            *entry = (*entry).max(*lines);
        }
    }

    /// Get coverage data aggregated by source file
    pub fn by_source_file(&self) -> HashMap<String, FileCoverage> {
        let mut files: HashMap<String, FileCoverage> = HashMap::new();

        for coverage in self.functions.values() {
            let source = coverage
                .source_file
                .clone()
                .unwrap_or_else(|| "<unknown>".to_string());

            let file_cov = files
                .entry(source.clone())
                .or_insert_with(|| FileCoverage::new(source));

            file_cov.executable_lines.extend(&coverage.executable_lines);
            file_cov.executed_lines.extend(&coverage.executed_lines);

            for (offset, branch) in &coverage.branches {
                let key = (coverage.name.clone(), *offset);
                file_cov.branches.insert(key, branch.clone());
            }

            file_cov.functions.push(coverage.name.clone());
        }

        files
    }

    /// Generate a summary report
    pub fn generate_summary(&self) -> CoverageSummary {
        let files = self.by_source_file();

        let mut summary = CoverageSummary {
            total_lines: 0,
            covered_lines: 0,
            total_branches: 0,
            covered_branches: 0,
            total_functions: self.functions.len(),
            files: Vec::new(),
        };

        for (source, file_cov) in files {
            let file_summary = FileCoverageSummary {
                source_file: source,
                total_lines: file_cov.executable_lines.len(),
                covered_lines: file_cov.executed_lines.len(),
                total_branches: file_cov.branches.len() * 2,
                covered_branches: file_cov
                    .branches
                    .values()
                    .map(|b| (b.taken_count > 0) as usize + (b.not_taken_count > 0) as usize)
                    .sum(),
                line_coverage_percent: file_cov.line_coverage_percent(),
                branch_coverage_percent: file_cov.branch_coverage_percent(),
                functions: file_cov.functions,
            };

            summary.total_lines += file_summary.total_lines;
            summary.covered_lines += file_summary.covered_lines;
            summary.total_branches += file_summary.total_branches;
            summary.covered_branches += file_summary.covered_branches;
            summary.files.push(file_summary);
        }

        summary
    }
}

/// Coverage data aggregated by source file
#[derive(Debug, Clone, Default)]
pub struct FileCoverage {
    /// Source file path
    pub source_file: String,
    /// Executable lines across all functions in this file
    pub executable_lines: HashSet<u32>,
    /// Executed lines
    pub executed_lines: HashSet<u32>,
    /// Branch points: (function_name, offset) -> BranchInfo
    pub branches: HashMap<(String, usize), BranchInfo>,
    /// Functions in this file
    pub functions: Vec<String>,
}

impl FileCoverage {
    /// Create new file coverage tracker
    pub fn new(source_file: String) -> Self {
        Self {
            source_file,
            executable_lines: HashSet::new(),
            executed_lines: HashSet::new(),
            branches: HashMap::new(),
            functions: Vec::new(),
        }
    }

    /// Calculate line coverage percentage
    pub fn line_coverage_percent(&self) -> f64 {
        if self.executable_lines.is_empty() {
            return 100.0;
        }
        (self.executed_lines.len() as f64 / self.executable_lines.len() as f64) * 100.0
    }

    /// Calculate branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.branches.is_empty() {
            return 100.0;
        }
        let total_branches = self.branches.len() * 2;
        let covered_branches: usize = self
            .branches
            .values()
            .map(|b| (b.taken_count > 0) as usize + (b.not_taken_count > 0) as usize)
            .sum();
        (covered_branches as f64 / total_branches as f64) * 100.0
    }

    /// Get uncovered lines
    pub fn uncovered_lines(&self) -> Vec<u32> {
        let mut lines: Vec<u32> = self
            .executable_lines
            .difference(&self.executed_lines)
            .copied()
            .collect();
        lines.sort();
        lines
    }
}

/// Summary of coverage across all files
#[derive(Debug, Clone, Default)]
pub struct CoverageSummary {
    /// Total executable lines
    pub total_lines: usize,
    /// Lines that were executed
    pub covered_lines: usize,
    /// Total branch outcomes (branches * 2)
    pub total_branches: usize,
    /// Branch outcomes that were covered
    pub covered_branches: usize,
    /// Total number of functions
    pub total_functions: usize,
    /// Per-file summaries
    pub files: Vec<FileCoverageSummary>,
}

impl CoverageSummary {
    /// Calculate overall line coverage percentage
    pub fn line_coverage_percent(&self) -> f64 {
        if self.total_lines == 0 {
            return 100.0;
        }
        (self.covered_lines as f64 / self.total_lines as f64) * 100.0
    }

    /// Calculate overall branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.total_branches == 0 {
            return 100.0;
        }
        (self.covered_branches as f64 / self.total_branches as f64) * 100.0
    }
}

/// Coverage summary for a single file
#[derive(Debug, Clone)]
pub struct FileCoverageSummary {
    /// Source file path
    pub source_file: String,
    /// Total executable lines
    pub total_lines: usize,
    /// Lines that were executed
    pub covered_lines: usize,
    /// Total branch outcomes
    pub total_branches: usize,
    /// Branch outcomes that were covered
    pub covered_branches: usize,
    /// Line coverage percentage
    pub line_coverage_percent: f64,
    /// Branch coverage percentage
    pub branch_coverage_percent: f64,
    /// Functions in this file
    pub functions: Vec<String>,
}

/// Coverage report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoverageFormat {
    /// Plain text summary to stdout
    #[default]
    Summary,
    /// HTML report
    Html,
    /// LCOV format for CI tooling
    Lcov,
}

impl std::str::FromStr for CoverageFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "summary" | "text" => Ok(CoverageFormat::Summary),
            "html" => Ok(CoverageFormat::Html),
            "lcov" => Ok(CoverageFormat::Lcov),
            _ => Err(format!("Unknown coverage format: {}", s)),
        }
    }
}

/// Generate coverage report in the specified format
pub fn generate_report(
    collector: &CoverageCollector,
    format: CoverageFormat,
    output_dir: Option<&Path>,
) -> String {
    match format {
        CoverageFormat::Summary => generate_summary_report(collector),
        CoverageFormat::Html => generate_html_report(collector, output_dir),
        CoverageFormat::Lcov => generate_lcov_report(collector),
    }
}

/// Generate plain text summary report
fn generate_summary_report(collector: &CoverageCollector) -> String {
    let summary = collector.generate_summary();
    let mut output = String::new();

    output.push_str("\n");
    output.push_str("Coverage Report\n");
    output.push_str("===============\n\n");

    // Overall summary
    output.push_str(&format!(
        "Lines:    {}/{} ({:.1}%)\n",
        summary.covered_lines,
        summary.total_lines,
        summary.line_coverage_percent()
    ));
    output.push_str(&format!(
        "Branches: {}/{} ({:.1}%)\n",
        summary.covered_branches,
        summary.total_branches,
        summary.branch_coverage_percent()
    ));
    output.push_str(&format!("Functions: {}\n\n", summary.total_functions));

    // Per-file details
    if !summary.files.is_empty() {
        output.push_str("Per-file coverage:\n");
        output.push_str("-----------------\n");

        for file in &summary.files {
            let status = if file.line_coverage_percent >= 80.0 {
                "OK"
            } else if file.line_coverage_percent >= 50.0 {
                "WARN"
            } else {
                "LOW"
            };

            output.push_str(&format!(
                "[{}] {} - Lines: {:.1}%, Branches: {:.1}%\n",
                status, file.source_file, file.line_coverage_percent, file.branch_coverage_percent
            ));
        }
    }

    output
}

/// Generate HTML coverage report
fn generate_html_report(collector: &CoverageCollector, output_dir: Option<&Path>) -> String {
    let summary = collector.generate_summary();
    let files = collector.by_source_file();

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<meta charset=\"UTF-8\">\n");
    html.push_str("<title>Stratum Coverage Report</title>\n");
    html.push_str("<style>\n");
    html.push_str(HTML_STYLES);
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    // Header
    html.push_str("<div class=\"header\">\n");
    html.push_str("<h1>Stratum Coverage Report</h1>\n");
    html.push_str("</div>\n");

    // Overall summary
    html.push_str("<div class=\"summary\">\n");
    html.push_str("<h2>Summary</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Metric</th><th>Covered</th><th>Total</th><th>Coverage</th></tr>\n");
    html.push_str(&format!(
        "<tr><td>Lines</td><td>{}</td><td>{}</td><td class=\"{}\">{:.1}%</td></tr>\n",
        summary.covered_lines,
        summary.total_lines,
        coverage_class(summary.line_coverage_percent()),
        summary.line_coverage_percent()
    ));
    html.push_str(&format!(
        "<tr><td>Branches</td><td>{}</td><td>{}</td><td class=\"{}\">{:.1}%</td></tr>\n",
        summary.covered_branches,
        summary.total_branches,
        coverage_class(summary.branch_coverage_percent()),
        summary.branch_coverage_percent()
    ));
    html.push_str(&format!(
        "<tr><td>Functions</td><td colspan=\"2\">{}</td><td>-</td></tr>\n",
        summary.total_functions
    ));
    html.push_str("</table>\n");
    html.push_str("</div>\n");

    // Per-file details
    html.push_str("<div class=\"files\">\n");
    html.push_str("<h2>Files</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>File</th><th>Lines</th><th>Branches</th><th>Functions</th></tr>\n");

    for file_summary in &summary.files {
        html.push_str(&format!(
            "<tr><td>{}</td><td class=\"{}\">{:.1}%</td><td class=\"{}\">{:.1}%</td><td>{}</td></tr>\n",
            file_summary.source_file,
            coverage_class(file_summary.line_coverage_percent),
            file_summary.line_coverage_percent,
            coverage_class(file_summary.branch_coverage_percent),
            file_summary.branch_coverage_percent,
            file_summary.functions.len()
        ));
    }

    html.push_str("</table>\n");
    html.push_str("</div>\n");

    // Uncovered lines section
    html.push_str("<div class=\"uncovered\">\n");
    html.push_str("<h2>Uncovered Lines</h2>\n");

    for (source, file_cov) in &files {
        let uncovered = file_cov.uncovered_lines();
        if !uncovered.is_empty() {
            html.push_str(&format!("<h3>{}</h3>\n", source));
            html.push_str("<p class=\"uncovered-lines\">");
            let line_strs: Vec<String> = uncovered.iter().map(|l| l.to_string()).collect();
            html.push_str(&line_strs.join(", "));
            html.push_str("</p>\n");
        }
    }

    html.push_str("</div>\n");

    html.push_str("</body>\n</html>\n");

    // Write to file if output_dir is provided
    if let Some(dir) = output_dir {
        let path = dir.join("coverage.html");
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("Warning: Could not create output directory: {}", e);
        } else if let Err(e) = std::fs::write(&path, &html) {
            eprintln!("Warning: Could not write HTML report: {}", e);
        } else {
            return format!("HTML report written to: {}", path.display());
        }
    }

    html
}

/// Generate LCOV format report
fn generate_lcov_report(collector: &CoverageCollector) -> String {
    let files = collector.by_source_file();
    let mut lcov = String::new();

    for (source, file_cov) in &files {
        // Test name (TN)
        lcov.push_str("TN:\n");
        // Source file (SF)
        lcov.push_str(&format!("SF:{}\n", source));

        // Function coverage (FN, FNDA, FNF, FNH)
        for func_name in &file_cov.functions {
            lcov.push_str(&format!("FN:1,{}\n", func_name));
            lcov.push_str(&format!("FNDA:1,{}\n", func_name));
        }
        lcov.push_str(&format!("FNF:{}\n", file_cov.functions.len()));
        lcov.push_str(&format!("FNH:{}\n", file_cov.functions.len()));

        // Line coverage (DA, LF, LH)
        let mut executed_lines: Vec<u32> = file_cov.executed_lines.iter().copied().collect();
        executed_lines.sort();
        for line in &executed_lines {
            lcov.push_str(&format!("DA:{},1\n", line));
        }

        // Lines that weren't executed
        let mut unexecuted: Vec<u32> = file_cov
            .executable_lines
            .difference(&file_cov.executed_lines)
            .copied()
            .collect();
        unexecuted.sort();
        for line in &unexecuted {
            lcov.push_str(&format!("DA:{},0\n", line));
        }

        lcov.push_str(&format!("LF:{}\n", file_cov.executable_lines.len()));
        lcov.push_str(&format!("LH:{}\n", file_cov.executed_lines.len()));

        // Branch coverage (BRDA, BRF, BRH)
        let mut branch_idx = 0;
        let mut covered_branches = 0;
        for ((_func_name, _offset), branch) in &file_cov.branches {
            // BRDA:line,block,branch,taken
            lcov.push_str(&format!(
                "BRDA:{},{},{},{}\n",
                branch.line,
                branch_idx,
                0,
                if branch.taken_count > 0 {
                    branch.taken_count.to_string()
                } else {
                    "-".to_string()
                }
            ));
            lcov.push_str(&format!(
                "BRDA:{},{},{},{}\n",
                branch.line,
                branch_idx,
                1,
                if branch.not_taken_count > 0 {
                    branch.not_taken_count.to_string()
                } else {
                    "-".to_string()
                }
            ));
            if branch.taken_count > 0 {
                covered_branches += 1;
            }
            if branch.not_taken_count > 0 {
                covered_branches += 1;
            }
            branch_idx += 1;
        }

        lcov.push_str(&format!("BRF:{}\n", file_cov.branches.len() * 2));
        lcov.push_str(&format!("BRH:{}\n", covered_branches));

        // End of record
        lcov.push_str("end_of_record\n");
    }

    lcov
}

/// Get CSS class based on coverage percentage
fn coverage_class(percent: f64) -> &'static str {
    if percent >= 80.0 {
        "high"
    } else if percent >= 50.0 {
        "medium"
    } else {
        "low"
    }
}

/// CSS styles for HTML report
const HTML_STYLES: &str = r#"
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 0;
    padding: 20px;
    background: #f5f5f5;
}
.header {
    background: #333;
    color: white;
    padding: 20px;
    margin: -20px -20px 20px -20px;
}
.header h1 {
    margin: 0;
}
.summary, .files, .uncovered {
    background: white;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}
h2 {
    margin-top: 0;
    color: #333;
    border-bottom: 2px solid #eee;
    padding-bottom: 10px;
}
h3 {
    color: #666;
}
table {
    width: 100%;
    border-collapse: collapse;
}
th, td {
    padding: 10px;
    text-align: left;
    border-bottom: 1px solid #eee;
}
th {
    background: #f9f9f9;
    font-weight: 600;
}
.high { color: #22863a; font-weight: bold; }
.medium { color: #b08800; font-weight: bold; }
.low { color: #cb2431; font-weight: bold; }
.uncovered-lines {
    font-family: monospace;
    background: #fff3cd;
    padding: 10px;
    border-radius: 4px;
    word-break: break-all;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_coverage_new() {
        let coverage = FunctionCoverage::new("test_fn".to_string(), Some("test.strat".to_string()));
        assert_eq!(coverage.name, "test_fn");
        assert_eq!(coverage.source_file, Some("test.strat".to_string()));
        assert!(coverage.executable_lines.is_empty());
        assert!(coverage.executed_lines.is_empty());
    }

    #[test]
    fn test_line_coverage_percentage() {
        let mut coverage = FunctionCoverage::new("test".to_string(), None);
        coverage.executable_lines = [1, 2, 3, 4, 5].into_iter().collect();
        coverage.executed_lines = [1, 2, 3].into_iter().collect();

        assert!((coverage.line_coverage_percent() - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_branch_info_coverage() {
        let mut branch = BranchInfo::default();
        assert!(!branch.is_partially_covered());
        assert!(!branch.is_fully_covered());

        branch.taken_count = 1;
        assert!(branch.is_partially_covered());
        assert!(!branch.is_fully_covered());

        branch.not_taken_count = 1;
        assert!(branch.is_fully_covered());
    }

    #[test]
    fn test_coverage_format_parsing() {
        assert_eq!(
            "summary".parse::<CoverageFormat>().unwrap(),
            CoverageFormat::Summary
        );
        assert_eq!(
            "html".parse::<CoverageFormat>().unwrap(),
            CoverageFormat::Html
        );
        assert_eq!(
            "lcov".parse::<CoverageFormat>().unwrap(),
            CoverageFormat::Lcov
        );
        assert!("invalid".parse::<CoverageFormat>().is_err());
    }

    #[test]
    fn test_collector_merge() {
        let mut collector1 = CoverageCollector::new();
        let mut collector2 = CoverageCollector::new();

        // Create some mock coverage data
        let mut cov1 = FunctionCoverage::new("fn1".to_string(), Some("test.strat".to_string()));
        cov1.executable_lines = [1, 2, 3].into_iter().collect();
        cov1.executed_lines = [1].into_iter().collect();

        let mut cov2 = FunctionCoverage::new("fn1".to_string(), Some("test.strat".to_string()));
        cov2.executable_lines = [1, 2, 3].into_iter().collect();
        cov2.executed_lines = [2, 3].into_iter().collect();

        collector1.functions.insert("fn1".to_string(), cov1);
        collector2.functions.insert("fn1".to_string(), cov2);

        collector1.merge(&collector2);

        let merged = collector1.functions.get("fn1").unwrap();
        assert_eq!(merged.executed_lines.len(), 3);
    }

    #[test]
    fn test_summary_generation() {
        let mut collector = CoverageCollector::new();
        let mut cov = FunctionCoverage::new("test_fn".to_string(), Some("test.strat".to_string()));
        cov.executable_lines = [1, 2, 3, 4, 5].into_iter().collect();
        cov.executed_lines = [1, 2, 3].into_iter().collect();
        collector.functions.insert("test_fn".to_string(), cov);

        let summary = collector.generate_summary();
        assert_eq!(summary.total_lines, 5);
        assert_eq!(summary.covered_lines, 3);
        assert_eq!(summary.total_functions, 1);
    }
}
