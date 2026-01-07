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
```

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
