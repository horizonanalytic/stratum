//! Stratum CLI - Command-line interface for the Stratum programming language

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod repl;

#[derive(Parser)]
#[command(name = "stratum")]
#[command(version = stratum_core::VERSION)]
#[command(about = "The Stratum programming language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the interactive REPL
    Repl,

    /// Run a Stratum source file
    Run {
        /// Path to the source file
        file: PathBuf,
    },

    /// Evaluate a Stratum expression
    Eval {
        /// Expression to evaluate
        expression: String,
    },

    /// Run tests in a Stratum source file
    Test {
        /// Path to the source file containing tests
        file: PathBuf,

        /// Filter tests by name (runs only tests containing this string)
        #[arg(short, long)]
        filter: Option<String>,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Repl) => {
            let mut repl = repl::Repl::new()?;
            repl.run()?;
        }

        Some(Commands::Run { file }) => {
            run_file(&file)?;
        }

        Some(Commands::Eval { expression }) => {
            eval_expression(&expression)?;
        }

        Some(Commands::Test {
            file,
            filter,
            verbose,
        }) => {
            run_tests(&file, filter.as_deref(), verbose)?;
        }

        None => {
            // Default behavior: start REPL
            let mut repl = repl::Repl::new()?;
            repl.run()?;
        }
    }

    Ok(())
}

/// Run a Stratum source file
fn run_file(path: &PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

    // Parse as module
    let module = stratum_core::Parser::parse_module(&source).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
        anyhow::anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
    })?;

    // Compile
    let function = stratum_core::Compiler::with_source(path.display().to_string())
        .compile_module(&module)
        .map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
            anyhow::anyhow!("Compile errors:\n{}", error_msgs.join("\n"))
        })?;

    // Run the module to register functions
    let mut vm = stratum_core::VM::new();
    let _ = vm
        .run(function)
        .map_err(|e| anyhow::anyhow!("Runtime error: {e}"))?;

    // Check if main() exists and call it
    if vm.globals().contains_key("main") {
        // Compile and run a call to main()
        let main_call = stratum_core::Parser::parse_expression("main()").map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
            anyhow::anyhow!("Internal error: {}", error_msgs.join("\n"))
        })?;

        let main_fn = stratum_core::Compiler::new()
            .compile_expression(&main_call)
            .map_err(|errors| {
                let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
                anyhow::anyhow!("Internal error: {}", error_msgs.join("\n"))
            })?;

        let result = vm
            .run(main_fn)
            .map_err(|e| anyhow::anyhow!("Runtime error: {e}"))?;

        // Print result if not null
        if !matches!(result, stratum_core::bytecode::Value::Null) {
            println!("{result}");
        }
    }

    Ok(())
}

/// Run tests in a Stratum source file
fn run_tests(path: &PathBuf, filter: Option<&str>, verbose: bool) -> Result<()> {
    use stratum_core::testing::{self, TestRunner};

    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

    // Parse as module
    let module = stratum_core::Parser::parse_module(&source).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
        anyhow::anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
    })?;

    // Discover and filter tests
    let tests = testing::discover_tests(&module);
    let tests = testing::filter_tests(tests, filter);

    if tests.is_empty() {
        if filter.is_some() {
            println!("No tests matching filter found");
        } else {
            println!("No tests found");
        }
        return Ok(());
    }

    println!("Running {} test(s)...\n", tests.len());

    // Run tests
    let runner = TestRunner::new().verbose(verbose);
    let summary = runner.run_tests(&tests, &path.display().to_string());

    // Print results
    for result in &summary.results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        let duration_ms = result.duration.as_secs_f64() * 1000.0;

        if result.should_panic && result.passed {
            println!("  {} {} (expected panic) [{:.2}ms]", status, result.name, duration_ms);
        } else {
            println!("  {} {} [{:.2}ms]", status, result.name, duration_ms);
        }

        if !result.passed {
            if let Some(ref error) = result.error {
                println!("       Error: {error}");
            }
        }
    }

    // Print summary
    println!();
    println!(
        "Test result: {} passed, {} failed, {} total (in {:.2}ms)",
        summary.passed,
        summary.failed,
        summary.total,
        summary.duration.as_secs_f64() * 1000.0
    );

    if summary.all_passed() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Some tests failed"))
    }
}

/// Evaluate a single expression
fn eval_expression(expression: &str) -> Result<()> {
    // Parse as expression
    let expr = stratum_core::Parser::parse_expression(expression).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
        anyhow::anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
    })?;

    // Compile
    let function = stratum_core::Compiler::new()
        .compile_expression(&expr)
        .map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
            anyhow::anyhow!("Compile errors:\n{}", error_msgs.join("\n"))
        })?;

    // Run
    let mut vm = stratum_core::VM::new();
    let result = vm
        .run(function)
        .map_err(|e| anyhow::anyhow!("Runtime error: {e}"))?;

    // Print result
    println!("{result}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn test_eval_simple_expression() {
        // This tests the parsing/compilation path without running
        let expr = stratum_core::Parser::parse_expression("1 + 2").unwrap();
        let _function = stratum_core::Compiler::new()
            .compile_expression(&expr)
            .unwrap();
    }
}
