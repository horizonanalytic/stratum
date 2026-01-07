//! Stratum CLI - Command-line interface for the Stratum programming language

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod add;
mod init;
mod remove;
mod repl;
mod update;

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

    /// Initialize a new Stratum project in the current directory
    Init {
        /// Create a library project instead of a binary
        #[arg(long)]
        lib: bool,

        /// Set the package name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,

        /// Initialize a git repository
        #[arg(long)]
        git: bool,
    },

    /// Add a dependency to stratum.toml
    Add {
        /// Package name (optionally with @version suffix, e.g., "http@1.0")
        package: String,

        /// Add as a development dependency
        #[arg(long)]
        dev: bool,

        /// Add as a build dependency
        #[arg(long)]
        build: bool,

        /// Use a local path dependency
        #[arg(long)]
        path: Option<String>,

        /// Use a git repository dependency
        #[arg(long)]
        git: Option<String>,

        /// Git branch to use (requires --git)
        #[arg(long, requires = "git")]
        branch: Option<String>,

        /// Git tag to use (requires --git)
        #[arg(long, requires = "git")]
        tag: Option<String>,

        /// Git revision/commit to use (requires --git)
        #[arg(long, requires = "git")]
        rev: Option<String>,

        /// Features to enable (comma-separated)
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,

        /// Mark as an optional dependency
        #[arg(long)]
        optional: bool,

        /// Disable default features
        #[arg(long)]
        no_default_features: bool,
    },

    /// Remove a dependency from stratum.toml
    Remove {
        /// Package name to remove
        package: String,

        /// Remove from development dependencies
        #[arg(long)]
        dev: bool,

        /// Remove from build dependencies
        #[arg(long)]
        build: bool,
    },

    /// Update dependencies to latest compatible versions
    Update {
        /// Only update specific packages
        packages: Vec<String>,

        /// Perform a dry run without writing changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Run a Stratum source file
    Run {
        /// Path to the source file
        file: PathBuf,

        /// Force interpret all functions (ignore #[compile] directives)
        #[arg(long, conflicts_with_all = ["compile_all"])]
        interpret_all: bool,

        /// Force compile all functions with JIT (ignore #[interpret] directives)
        #[arg(long, conflicts_with_all = ["interpret_all"])]
        compile_all: bool,

        /// Enable JIT compilation for hot paths (default behavior)
        #[arg(long)]
        jit: bool,
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

    /// Format Stratum source files
    Fmt {
        /// Files to format (if none, formats stdin)
        files: Vec<PathBuf>,

        /// Check if files are formatted without modifying
        #[arg(short, long)]
        check: bool,
    },

    /// Build a Stratum source file into a standalone executable
    Build {
        /// Path to the source file
        file: PathBuf,

        /// Output executable path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Build with optimizations
        #[arg(long)]
        release: bool,
    },

    /// Open Stratum Workshop IDE
    Workshop {
        /// Path to file or folder to open
        path: Option<PathBuf>,
    },

    /// Start the Language Server Protocol (LSP) server
    Lsp,

    /// Generate documentation for a Stratum source file or project
    Doc {
        /// Path to the source file or directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output directory for generated documentation
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (html or markdown)
        #[arg(short, long, default_value = "html")]
        format: String,

        /// Open the documentation in a browser after generation
        #[arg(long)]
        open: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Repl) => {
            let mut repl = repl::Repl::new()?;
            repl.run()?;
        }

        Some(Commands::Init { lib, name, git }) => {
            let options = init::InitOptions {
                lib,
                name,
                git,
            };
            init::init_project(options)?;
        }

        Some(Commands::Add {
            package,
            dev,
            build,
            path,
            git,
            branch,
            tag,
            rev,
            features,
            optional,
            no_default_features,
        }) => {
            let section = if dev {
                add::DependencySection::Dev
            } else if build {
                add::DependencySection::Build
            } else {
                add::DependencySection::Dependencies
            };

            let options = add::AddOptions {
                package,
                section,
                path,
                git,
                branch,
                tag,
                rev,
                features,
                optional,
                no_default_features,
            };
            add::add_dependency(options)?;
        }

        Some(Commands::Remove { package, dev, build }) => {
            let section = if dev {
                Some(add::DependencySection::Dev)
            } else if build {
                Some(add::DependencySection::Build)
            } else {
                None // Search all sections
            };

            let options = remove::RemoveOptions { package, section };
            remove::remove_dependency(options)?;
        }

        Some(Commands::Update { packages, dry_run }) => {
            let options = update::UpdateOptions { packages, dry_run };
            let result = update::update_dependencies(options)?;
            result.print_summary();
        }

        Some(Commands::Run { file, interpret_all, compile_all, jit: _ }) => {
            let mode_override = if interpret_all {
                Some(stratum_core::ExecutionModeOverride::InterpretAll)
            } else if compile_all {
                Some(stratum_core::ExecutionModeOverride::CompileAll)
            } else {
                None // Respect directives
            };
            run_file(&file, mode_override)?;
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

        Some(Commands::Fmt { files, check }) => {
            format_files(&files, check)?;
        }

        Some(Commands::Build { file, output, release }) => {
            build_executable(&file, output, release)?;
        }

        Some(Commands::Workshop { path }) => {
            launch_workshop(path)?;
        }

        Some(Commands::Lsp) => {
            run_lsp_server()?;
        }

        Some(Commands::Doc {
            path,
            output,
            format,
            open,
        }) => {
            generate_documentation(&path, output, &format, open)?;
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
fn run_file(path: &PathBuf, mode_override: Option<stratum_core::ExecutionModeOverride>) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

    // Parse as module
    let module = stratum_core::Parser::parse_module(&source).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
        anyhow::anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
    })?;

    // Type check
    let mut type_checker = stratum_core::TypeChecker::new();
    let type_result = type_checker.check_module(&module);
    if !type_result.errors.is_empty() {
        let error_msgs: Vec<String> = type_result
            .errors
            .iter()
            .map(|e| format!("  {e}"))
            .collect();
        return Err(anyhow::anyhow!("Type errors:\n{}", error_msgs.join("\n")));
    }

    // Compile with execution mode override if specified
    let function = stratum_core::Compiler::with_source(path.display().to_string())
        .with_mode_override(mode_override)
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

    // Type check
    let mut type_checker = stratum_core::TypeChecker::new();
    let type_result = type_checker.check_module(&module);
    if !type_result.errors.is_empty() {
        let error_msgs: Vec<String> = type_result
            .errors
            .iter()
            .map(|e| format!("  {e}"))
            .collect();
        return Err(anyhow::anyhow!("Type errors:\n{}", error_msgs.join("\n")));
    }

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

/// Build a Stratum source file into a standalone executable
fn build_executable(path: &PathBuf, output: Option<PathBuf>, release: bool) -> Result<()> {
    use stratum_core::aot::{AotCompiler, Linker, LinkerConfig};
    use stratum_core::ast::ExecutionMode;

    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

    // Parse as module
    let module = stratum_core::Parser::parse_module(&source).map_err(|errors| {
        let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
        anyhow::anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
    })?;

    // Type check
    let mut type_checker = stratum_core::TypeChecker::new();
    let type_result = type_checker.check_module(&module);
    if !type_result.errors.is_empty() {
        let error_msgs: Vec<String> = type_result
            .errors
            .iter()
            .map(|e| format!("  {e}"))
            .collect();
        return Err(anyhow::anyhow!("Type errors:\n{}", error_msgs.join("\n")));
    }

    // Compile to bytecode
    let bytecode_fn = stratum_core::Compiler::with_source(path.display().to_string())
        .compile_module(&module)
        .map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
            anyhow::anyhow!("Compile errors:\n{}", error_msgs.join("\n"))
        })?;

    // Create AOT compiler
    let mut aot = AotCompiler::new()
        .map_err(|e| anyhow::anyhow!("Failed to create AOT compiler: {e}"))?;

    // Find all functions in the module that should be compiled
    let mut has_main = false;
    for constant in bytecode_fn.chunk.constants() {
        if let stratum_core::bytecode::Value::Function(func) = constant {
            // Only compile functions marked for compilation, or all if building
            let should_compile = matches!(
                func.execution_mode,
                ExecutionMode::Compile | ExecutionMode::CompileHot
            ) || true; // For now, compile all functions in build mode

            if should_compile {
                aot.compile_function(func)
                    .map_err(|e| anyhow::anyhow!("Failed to compile function '{}': {e}", func.name))?;
                if func.name == "main" {
                    has_main = true;
                }
            }
        }
    }

    if !has_main {
        return Err(anyhow::anyhow!("No main function found in module"));
    }

    // Generate entry point
    aot.generate_entry_point()
        .map_err(|e| anyhow::anyhow!("Failed to generate entry point: {e}"))?;

    // Finish compilation
    let product = aot.finish();

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let stem = path.file_stem().unwrap_or_default();
        PathBuf::from(stem)
    });

    // Link into executable
    let linker = Linker::new(LinkerConfig {
        output: output_path.clone(),
        optimize: release,
        extra_flags: Vec::new(),
    });

    linker.link(product)
        .map_err(|e| anyhow::anyhow!("Failed to link: {e}"))?;

    println!("Built: {}", output_path.display());

    Ok(())
}

/// Launch Stratum Workshop IDE
fn launch_workshop(path: Option<PathBuf>) -> Result<()> {
    stratum_workshop::launch(path).map_err(|e| anyhow::anyhow!("Workshop error: {e}"))
}

/// Run the Language Server Protocol (LSP) server
fn run_lsp_server() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(stratum_lsp::run_server())
}

/// Generate documentation for Stratum source files
fn generate_documentation(
    path: &PathBuf,
    output: Option<PathBuf>,
    format: &str,
    open: bool,
) -> Result<()> {
    use stratum_core::doc::{DocExtractor, HtmlGenerator, MarkdownGenerator};

    // Collect source files
    let files = if path.is_file() {
        vec![path.clone()]
    } else if path.is_dir() {
        collect_stratum_files(path)?
    } else {
        return Err(anyhow::anyhow!("Path '{}' does not exist", path.display()));
    };

    if files.is_empty() {
        return Err(anyhow::anyhow!(
            "No .strat files found in '{}'",
            path.display()
        ));
    }

    // Determine output directory
    let output_dir = output.unwrap_or_else(|| {
        if path.is_file() {
            path.parent().unwrap_or(path).join("doc")
        } else {
            path.join("doc")
        }
    });

    // Create output directory
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;

    let extension = if format == "markdown" || format == "md" {
        "md"
    } else {
        "html"
    };

    let mut generated_files = Vec::new();

    // Process each file
    for file in &files {
        let source = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", file.display(), e))?;

        // Parse the module
        let module = match stratum_core::Parser::parse_module(&source) {
            Ok(m) => m,
            Err(errors) => {
                eprintln!("Parse errors in '{}':", file.display());
                for e in &errors {
                    eprintln!("  {}", e);
                }
                continue;
            }
        };

        // Extract documentation
        let module_name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let doc_module = DocExtractor::extract(&module, module_name);

        // Generate output
        let content = if format == "markdown" || format == "md" {
            MarkdownGenerator::generate(&doc_module)
        } else {
            HtmlGenerator::generate(&doc_module)
        };

        // Write output file
        let output_file = output_dir.join(format!("{}.{}", module_name, extension));
        std::fs::write(&output_file, &content)
            .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", output_file.display(), e))?;

        println!("Generated: {}", output_file.display());
        generated_files.push(output_file);
    }

    if generated_files.is_empty() {
        return Err(anyhow::anyhow!("No documentation was generated"));
    }

    // Generate index file if multiple files
    if generated_files.len() > 1 {
        let index_file = output_dir.join(format!("index.{}", extension));
        let index_content = generate_index(&files, format);
        std::fs::write(&index_file, &index_content)
            .map_err(|e| anyhow::anyhow!("Failed to write index: {}", e))?;
        println!("Generated: {}", index_file.display());
        generated_files.insert(0, index_file);
    }

    // Open in browser if requested
    if open {
        let file_to_open = &generated_files[0];
        if let Err(e) = open_in_browser(file_to_open) {
            eprintln!("Warning: Could not open browser: {}", e);
        }
    }

    println!("\nDocumentation generated in: {}", output_dir.display());

    Ok(())
}

/// Collect all .strat files in a directory
fn collect_stratum_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "strat" {
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            // Skip hidden directories and common non-source directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    files.extend(collect_stratum_files(&path)?);
                }
            }
        }
    }

    Ok(files)
}

/// Generate an index page for multiple documented modules
fn generate_index(files: &[PathBuf], format: &str) -> String {
    if format == "markdown" || format == "md" {
        let mut output = String::from("# Documentation\n\n");
        output.push_str("## Modules\n\n");
        for file in files {
            let name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            output.push_str(&format!("- [{}]({}.md)\n", name, name));
        }
        output
    } else {
        let mut output = String::new();
        output.push_str("<!DOCTYPE html>\n");
        output.push_str("<html lang=\"en\">\n");
        output.push_str("<head>\n");
        output.push_str("  <meta charset=\"UTF-8\">\n");
        output.push_str("  <title>Documentation Index</title>\n");
        output.push_str("  <style>\n");
        output.push_str("    body { font-family: sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }\n");
        output.push_str("    h1 { color: #7b68ee; }\n");
        output.push_str("    ul { list-style: none; padding: 0; }\n");
        output.push_str("    li { margin: 0.5rem 0; }\n");
        output.push_str("    a { color: #7b68ee; text-decoration: none; }\n");
        output.push_str("    a:hover { text-decoration: underline; }\n");
        output.push_str("  </style>\n");
        output.push_str("</head>\n");
        output.push_str("<body>\n");
        output.push_str("  <h1>Documentation</h1>\n");
        output.push_str("  <h2>Modules</h2>\n");
        output.push_str("  <ul>\n");
        for file in files {
            let name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            output.push_str(&format!("    <li><a href=\"{}.html\">{}</a></li>\n", name, name));
        }
        output.push_str("  </ul>\n");
        output.push_str("</body>\n");
        output.push_str("</html>\n");
        output
    }
}

/// Open a file in the default browser
fn open_in_browser(path: &PathBuf) -> Result<()> {
    let url = format!("file://{}", path.canonicalize()?.display());

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(&url).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()?;
    }

    Ok(())
}

/// Format Stratum source files
fn format_files(files: &[PathBuf], check: bool) -> Result<()> {
    use std::io::{self, Read, Write};

    // If no files specified, read from stdin and write to stdout
    if files.is_empty() {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| anyhow::anyhow!("Failed to read from stdin: {e}"))?;

        let module = stratum_core::Parser::parse_module(&source).map_err(|errors| {
            let error_msgs: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
            anyhow::anyhow!("Parse errors:\n{}", error_msgs.join("\n"))
        })?;

        let formatted = stratum_core::Formatter::format_module(&module);

        if check {
            if source != formatted {
                return Err(anyhow::anyhow!("stdin is not formatted"));
            }
        } else {
            io::stdout()
                .write_all(formatted.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to write to stdout: {e}"))?;
        }
        return Ok(());
    }

    // Format specified files
    let mut unformatted_files = Vec::new();
    let mut error_files = Vec::new();

    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading '{}': {}", file.display(), e);
                error_files.push(file.clone());
                continue;
            }
        };

        let module = match stratum_core::Parser::parse_module(&source) {
            Ok(m) => m,
            Err(errors) => {
                eprintln!("Parse errors in '{}':", file.display());
                for e in &errors {
                    eprintln!("  {e}");
                }
                error_files.push(file.clone());
                continue;
            }
        };

        let formatted = stratum_core::Formatter::format_module(&module);

        if check {
            if source != formatted {
                println!("Would reformat: {}", file.display());
                unformatted_files.push(file.clone());
            }
        } else if source != formatted {
            match std::fs::write(file, &formatted) {
                Ok(()) => println!("Formatted: {}", file.display()),
                Err(e) => {
                    eprintln!("Error writing '{}': {}", file.display(), e);
                    error_files.push(file.clone());
                }
            }
        }
    }

    // Report results
    if check {
        if !unformatted_files.is_empty() {
            eprintln!(
                "\n{} file(s) would be reformatted",
                unformatted_files.len()
            );
            return Err(anyhow::anyhow!("Some files are not formatted"));
        }
        if !error_files.is_empty() {
            return Err(anyhow::anyhow!("Some files had errors"));
        }
        println!("All files are properly formatted");
    } else if !error_files.is_empty() {
        return Err(anyhow::anyhow!(
            "{} file(s) had errors",
            error_files.len()
        ));
    }

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

    #[test]
    fn test_run_with_interpret_all_flag() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "run", "test.strat", "--interpret-all"]).unwrap();
        match cli.command {
            Some(Commands::Run { interpret_all, compile_all, jit, .. }) => {
                assert!(interpret_all);
                assert!(!compile_all);
                assert!(!jit);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_run_with_compile_all_flag() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "run", "test.strat", "--compile-all"]).unwrap();
        match cli.command {
            Some(Commands::Run { interpret_all, compile_all, jit, .. }) => {
                assert!(!interpret_all);
                assert!(compile_all);
                assert!(!jit);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_run_with_jit_flag() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "run", "test.strat", "--jit"]).unwrap();
        match cli.command {
            Some(Commands::Run { interpret_all, compile_all, jit, .. }) => {
                assert!(!interpret_all);
                assert!(!compile_all);
                assert!(jit);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_run_flags_conflict() {
        use clap::Parser as ClapParser;
        // --interpret-all and --compile-all are mutually exclusive
        let result = Cli::try_parse_from(&["stratum", "run", "test.strat", "--interpret-all", "--compile-all"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_workshop_no_path() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "workshop"]).unwrap();
        match cli.command {
            Some(Commands::Workshop { path }) => {
                assert!(path.is_none());
            }
            _ => panic!("Expected Workshop command"),
        }
    }

    #[test]
    fn test_workshop_with_file() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "workshop", "main.strat"]).unwrap();
        match cli.command {
            Some(Commands::Workshop { path }) => {
                assert_eq!(path, Some(PathBuf::from("main.strat")));
            }
            _ => panic!("Expected Workshop command"),
        }
    }

    #[test]
    fn test_workshop_with_folder() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "workshop", "/path/to/project"]).unwrap();
        match cli.command {
            Some(Commands::Workshop { path }) => {
                assert_eq!(path, Some(PathBuf::from("/path/to/project")));
            }
            _ => panic!("Expected Workshop command"),
        }
    }

    #[test]
    fn test_add_simple() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "add", "http"]).unwrap();
        match cli.command {
            Some(Commands::Add { package, dev, build, .. }) => {
                assert_eq!(package, "http");
                assert!(!dev);
                assert!(!build);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_with_version() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "add", "http@1.0"]).unwrap();
        match cli.command {
            Some(Commands::Add { package, .. }) => {
                assert_eq!(package, "http@1.0");
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_dev_dependency() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "add", "--dev", "test-utils"]).unwrap();
        match cli.command {
            Some(Commands::Add { package, dev, build, .. }) => {
                assert_eq!(package, "test-utils");
                assert!(dev);
                assert!(!build);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_build_dependency() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "add", "--build", "build-helper"]).unwrap();
        match cli.command {
            Some(Commands::Add { package, dev, build, .. }) => {
                assert_eq!(package, "build-helper");
                assert!(!dev);
                assert!(build);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_with_features() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "add", "json", "--features", "pretty,async"]).unwrap();
        match cli.command {
            Some(Commands::Add { package, features, .. }) => {
                assert_eq!(package, "json");
                assert_eq!(features, vec!["pretty", "async"]);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_path_dependency() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "add", "local-lib", "--path", "../local-lib"]).unwrap();
        match cli.command {
            Some(Commands::Add { package, path, .. }) => {
                assert_eq!(package, "local-lib");
                assert_eq!(path, Some("../local-lib".to_string()));
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_git_dependency() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&[
            "stratum", "add", "remote-lib",
            "--git", "https://github.com/example/lib",
            "--branch", "main"
        ]).unwrap();
        match cli.command {
            Some(Commands::Add { package, git, branch, .. }) => {
                assert_eq!(package, "remote-lib");
                assert_eq!(git, Some("https://github.com/example/lib".to_string()));
                assert_eq!(branch, Some("main".to_string()));
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_add_git_branch_requires_git() {
        use clap::Parser as ClapParser;
        // --branch without --git should fail
        let result = Cli::try_parse_from(&["stratum", "add", "pkg", "--branch", "main"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_simple() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "remove", "http"]).unwrap();
        match cli.command {
            Some(Commands::Remove { package, dev, build }) => {
                assert_eq!(package, "http");
                assert!(!dev);
                assert!(!build);
            }
            _ => panic!("Expected Remove command"),
        }
    }

    #[test]
    fn test_remove_dev_dependency() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "remove", "--dev", "test-utils"]).unwrap();
        match cli.command {
            Some(Commands::Remove { package, dev, build }) => {
                assert_eq!(package, "test-utils");
                assert!(dev);
                assert!(!build);
            }
            _ => panic!("Expected Remove command"),
        }
    }

    #[test]
    fn test_remove_build_dependency() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "remove", "--build", "build-helper"]).unwrap();
        match cli.command {
            Some(Commands::Remove { package, dev, build }) => {
                assert_eq!(package, "build-helper");
                assert!(!dev);
                assert!(build);
            }
            _ => panic!("Expected Remove command"),
        }
    }

    #[test]
    fn test_update_simple() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "update"]).unwrap();
        match cli.command {
            Some(Commands::Update { packages, dry_run }) => {
                assert!(packages.is_empty());
                assert!(!dry_run);
            }
            _ => panic!("Expected Update command"),
        }
    }

    #[test]
    fn test_update_specific_packages() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "update", "http", "json"]).unwrap();
        match cli.command {
            Some(Commands::Update { packages, dry_run }) => {
                assert_eq!(packages, vec!["http", "json"]);
                assert!(!dry_run);
            }
            _ => panic!("Expected Update command"),
        }
    }

    #[test]
    fn test_update_dry_run() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "update", "--dry-run"]).unwrap();
        match cli.command {
            Some(Commands::Update { packages, dry_run }) => {
                assert!(packages.is_empty());
                assert!(dry_run);
            }
            _ => panic!("Expected Update command"),
        }
    }

    #[test]
    fn test_lsp_command() {
        use clap::Parser as ClapParser;
        let cli = Cli::try_parse_from(&["stratum", "lsp"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Lsp)));
    }
}
