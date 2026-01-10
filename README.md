# Stratum

A "Goldilocks" programming language: easier than Rust, more structured than Python.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Build Status](https://github.com/horizonanalytic/stratum/actions/workflows/ci.yml/badge.svg)](https://github.com/horizonanalytic/stratum/actions)

## Overview

Stratum is designed for developers who want:
- **Approachable complexity** - Learn in weeks, not months
- **Hybrid execution** - You control what gets compiled vs interpreted
- **Ultra-fast data** - Built-in Arrow-backed DataFrame with SIMD acceleration
- **Native GUI** - First-class declarative UI components
- **Bundled IDE** - "Stratum Workshop" ships with the language

## Quick Example

```stratum
fx greet(name: String) -> String {
    "Hello, {name}!"
}

fx main() {
    let message = greet("Stratum")
    print(message)
}
```

## Key Features

| Feature | Design |
|---------|--------|
| Functions | `fx` keyword (math-friendly, like f(x)) |
| Types | Static with full inference |
| Nullability | `?` suffix (`User?` for nullable) |
| Errors | Exceptions by default, optional `Result` |
| Memory | Invisible automatic reference counting |
| Data | Pipeline operator `\|>` for data flow |

## File Extensions

- `.strat` - Source files
- `.stratum` - Compiled binaries

## Installation

### Quick Install (Recommended)

```bash
# Interactive installer - choose your tier
curl -fsSL https://get.stratum-lang.dev | sh

# Or non-interactive with defaults
curl -fsSL https://get.stratum-lang.dev | sh -s -- --yes
```

### Homebrew (macOS / Linux)

```bash
# Add the tap
brew tap horizon-analytic/stratum

# Install (default: Data tier with DataFrame support)
brew install stratum

# Install with GUI framework
brew install stratum --with-gui

# Install full (including Workshop IDE)
brew install stratum --with-full
```

### Docker

```bash
# Run a script
docker run --rm -v $(pwd):/app ghcr.io/horizon-analytic/stratum run /app/script.strat

# Interactive REPL
docker run --rm -it ghcr.io/horizon-analytic/stratum repl
```

### Installation Tiers

| Tier | Size | Includes |
|------|------|----------|
| **Core** | ~15 MB | CLI, REPL, compiler, type checker |
| **Data** | ~45 MB | Core + DataFrame, Arrow, SQL (default) |
| **GUI** | ~80 MB | Data + GUI framework (iced) |
| **Full** | ~120 MB | GUI + Workshop IDE, LSP |

For detailed installation options, system requirements, and troubleshooting, see the [Installation Guide](docs/installation.md).

## Project Status

**Status:** In Development

See [planning/00-overview.md](planning/00-overview.md) for development roadmap.

## Building from Source

```bash
# Clone the repository
git clone git@github.com:horizonanalytic/stratum.git
cd stratum

# Build (requires Rust 1.75+)
cargo build --all

# Run tests
cargo test --all
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
