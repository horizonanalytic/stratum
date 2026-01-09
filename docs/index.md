# Stratum Documentation

Welcome to the official documentation for **Stratum**, the "Goldilocks" programming language - easier than Rust, more structured than Python.

## Quick Start

```stratum
// Hello, World!
println("Hello, Stratum!")

// Variables with type inference
let name = "Stratum"
let version = 1.0

// Functions use the `fx` keyword
fx greet(who: String) -> String {
    return "Hello, " + who + "!"
}

println(greet(name))
```

## Documentation Sections

### [Standard Library Reference](stdlib/index.md)

Complete reference for Stratum's built-in functions and namespaces:

- **Core** - `print`, `println`, `assert`, `type_of`, `len`, `range`
- **Math** - Mathematical constants and functions
- **Collections** - List, Map, and Set operations
- **Strings** - String manipulation and regex
- **File System** - File, Dir, and Path operations
- **Data** - DataFrame and OLAP Cube operations
- **Networking** - HTTP, TCP, UDP, WebSocket
- **And more...**

### Language Reference

*(Coming soon)* - Syntax, types, control flow, and language semantics.

### Getting Started Guide

*(Coming soon)* - Step-by-step introduction to Stratum programming.

## Key Features

| Feature | Description |
|---------|-------------|
| **Simple Syntax** | `fx` keyword for functions, clean expression syntax |
| **Type Inference** | Full static typing without the boilerplate |
| **Nullable Safety** | `?` suffix for nullable types (`String?`) |
| **Pipeline Operator** | `\|>` for fluent data transformations |
| **Built-in DataFrames** | Arrow-backed, SIMD-accelerated data operations |
| **Native GUI** | Declarative UI components with reactive state |

## Running Stratum

```bash
# Run a script
stratum run script.strat

# Compile to binary
stratum build app.strat

# Start the REPL
stratum repl

# Open the Workshop IDE
stratum workshop

# Format source files
stratum fmt *.strat

# Run tests
stratum test tests.strat

# Generate documentation
stratum doc src/
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `stratum run <file>` | Execute a Stratum source file |
| `stratum build <file>` | Compile to standalone executable |
| `stratum repl` | Start interactive REPL |
| `stratum workshop [path]` | Open the Workshop IDE |
| `stratum test <file>` | Run tests in a source file |
| `stratum fmt <files>` | Format source files |
| `stratum doc <path>` | Generate documentation |
| `stratum lsp` | Start language server (for editors) |
| `stratum dap` | Start debug adapter (for editors) |
| `stratum init` | Initialize a new project |
| `stratum add <pkg>` | Add a dependency |
| `stratum remove <pkg>` | Remove a dependency |
| `stratum update` | Update dependencies |
| `stratum publish` | Publish package to GitHub Releases |
| `stratum extension install` | Install VS Code extension |

## VS Code Extension

The Stratum VS Code extension provides full IDE support:

- **IntelliSense** - Completions, hover info, signature help
- **Navigation** - Go to definition, find references, rename
- **Diagnostics** - Real-time error checking and quick fixes
- **Debugging** - Breakpoints, stepping, variable inspection
- **Tasks** - Auto-detected build, run, and test tasks
- **Formatting** - Format on save

### Installation

```bash
# Install via Stratum CLI (recommended)
stratum extension install

# Or install from VSIX file
stratum extension install --vsix /path/to/stratum.vsix

# Check installation status
stratum extension list

# Uninstall
stratum extension uninstall
```

The extension requires the Stratum CLI to be installed and available in your PATH.

## File Extensions

| Extension | Purpose |
|-----------|---------|
| `.strat` | Stratum source files |
| `.stratum` | Compiled binaries |

## Community & Support

- [GitHub Repository](https://github.com/horizon-analytic-studios/stratum)
- [Report Issues](https://github.com/horizon-analytic-studios/stratum/issues)

---

*Stratum is developed by Horizon Analytic Studios, LLC*
