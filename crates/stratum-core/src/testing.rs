//! Testing framework for the Stratum programming language
//!
//! This module provides functionality for discovering and running test functions
//! marked with the `#[test]` attribute.

use crate::ast::{Function, Module};
use crate::bytecode::Compiler;
use crate::coverage::CoverageCollector;
use crate::vm::VM;
use std::time::{Duration, Instant};

/// Result of running a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Name of the test function
    pub name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Duration of the test
    pub duration: Duration,
    /// Error message if the test failed
    pub error: Option<String>,
    /// Whether this test was expected to panic
    pub should_panic: bool,
}

impl TestResult {
    /// Create a passing test result
    #[must_use]
    pub fn passed(name: String, duration: Duration) -> Self {
        Self {
            name,
            passed: true,
            duration,
            error: None,
            should_panic: false,
        }
    }

    /// Create a failing test result
    #[must_use]
    pub fn failed(name: String, duration: Duration, error: String) -> Self {
        Self {
            name,
            passed: false,
            duration,
            error: Some(error),
            should_panic: false,
        }
    }
}

/// Summary of running multiple tests
#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    /// Total number of tests
    pub total: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Total duration of all tests
    pub duration: Duration,
    /// Individual test results
    pub results: Vec<TestResult>,
    /// Aggregated coverage data (if coverage was enabled)
    pub coverage: Option<CoverageCollector>,
}

impl TestSummary {
    /// Create a new empty summary
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a test result to the summary
    pub fn add(&mut self, result: TestResult) {
        self.total += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.duration += result.duration;
        self.results.push(result);
    }

    /// Check if all tests passed
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

/// A discovered test function
#[derive(Debug, Clone)]
pub struct TestCase {
    /// The function name
    pub name: String,
    /// The function AST
    pub function: Function,
    /// Whether the test should expect a panic
    pub should_panic: bool,
}

/// Discovers test functions from a module
pub fn discover_tests(module: &Module) -> Vec<TestCase> {
    let mut tests = Vec::new();

    for item in module.items() {
        if let crate::ast::ItemKind::Function(func) = &item.kind {
            if func.is_test() {
                tests.push(TestCase {
                    name: func.name.name.clone(),
                    function: func.clone(),
                    should_panic: func.should_panic(),
                });
            }
        }
    }

    tests
}

/// Filters tests by a pattern
pub fn filter_tests(tests: Vec<TestCase>, filter: Option<&str>) -> Vec<TestCase> {
    match filter {
        Some(pattern) => tests
            .into_iter()
            .filter(|t| t.name.contains(pattern))
            .collect(),
        None => tests,
    }
}

/// Run a single test function
pub fn run_test(test: &TestCase, source_name: &str, vm: &mut VM) -> TestResult {
    let start = Instant::now();

    // Compile the test function
    let compile_result =
        Compiler::with_source(source_name.to_string()).compile_test_function(&test.function);

    let function = match compile_result {
        Ok(f) => f,
        Err(errors) => {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("{e}")).collect();
            return TestResult::failed(
                test.name.clone(),
                start.elapsed(),
                format!("Compile error: {}", error_msgs.join(", ")),
            );
        }
    };

    // Run the test
    let result = vm.run(function);
    let duration = start.elapsed();

    match (result, test.should_panic) {
        (Ok(_), false) => TestResult::passed(test.name.clone(), duration),
        (Ok(_), true) => TestResult::failed(
            test.name.clone(),
            duration,
            "Test was expected to panic but did not".to_string(),
        ),
        (Err(_), true) => {
            // Test panicked as expected
            let mut result = TestResult::passed(test.name.clone(), duration);
            result.should_panic = true;
            result
        }
        (Err(e), false) => TestResult::failed(test.name.clone(), duration, e.to_string()),
    }
}

/// Test runner that executes tests and collects results
pub struct TestRunner {
    /// Filter pattern for test names
    filter: Option<String>,
    /// Whether to show verbose output
    verbose: bool,
    /// Whether to collect coverage data
    coverage: bool,
}

impl TestRunner {
    /// Create a new test runner
    #[must_use]
    pub fn new() -> Self {
        Self {
            filter: None,
            verbose: false,
            coverage: false,
        }
    }

    /// Set a filter pattern for test names
    #[must_use]
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Enable verbose output
    #[must_use]
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Enable coverage collection
    #[must_use]
    pub fn with_coverage(mut self, coverage: bool) -> Self {
        self.coverage = coverage;
        self
    }

    /// Run all tests in a module
    pub fn run_module(&self, module: &Module, source_name: &str) -> TestSummary {
        let tests = discover_tests(module);
        let tests = filter_tests(tests, self.filter.as_deref());
        self.run_tests(&tests, source_name)
    }

    /// Run a list of test cases
    pub fn run_tests(&self, tests: &[TestCase], source_name: &str) -> TestSummary {
        let mut summary = TestSummary::new();
        let mut aggregated_coverage = if self.coverage {
            Some(CoverageCollector::new())
        } else {
            None
        };

        // First compile and run the module to register all functions
        // This is needed so test functions can call helper functions

        for test in tests {
            // Create a fresh VM for each test
            let mut vm = VM::new();

            // Enable coverage if requested
            if self.coverage {
                vm.enable_coverage();
            }

            let result = run_test(test, source_name, &mut vm);
            summary.add(result);

            // Collect coverage data from this test
            if self.coverage {
                if let Some(test_coverage) = vm.take_coverage() {
                    if let Some(ref mut agg) = aggregated_coverage {
                        agg.merge(&test_coverage);
                    }
                }
            }
        }

        summary.coverage = aggregated_coverage;
        summary
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_discover_tests() {
        let source = r#"
            #[test]
            fx test_one() {
                assert(true)
            }

            fx helper() {
                42
            }

            #[test]
            fx test_two() {
                assert_eq(1, 1)
            }
        "#;

        let module = Parser::parse_module(source).unwrap();
        let tests = discover_tests(&module);

        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "test_one");
        assert_eq!(tests[1].name, "test_two");
    }

    #[test]
    fn test_filter_tests() {
        let source = r#"
            #[test]
            fx test_add() { }

            #[test]
            fx test_subtract() { }

            #[test]
            fx check_multiply() { }
        "#;

        let module = Parser::parse_module(source).unwrap();
        let tests = discover_tests(&module);
        let filtered = filter_tests(tests, Some("test_"));

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "test_add");
        assert_eq!(filtered[1].name, "test_subtract");
    }

    #[test]
    fn test_should_panic_detection() {
        let source = r#"
            #[test(should_panic)]
            fx test_panic() {
                assert(false)
            }
        "#;

        let module = Parser::parse_module(source).unwrap();
        let tests = discover_tests(&module);

        assert_eq!(tests.len(), 1);
        assert!(tests[0].should_panic);
    }
}
